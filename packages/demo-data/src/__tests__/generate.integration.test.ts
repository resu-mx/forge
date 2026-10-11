/**
 * End to end: generate every persona through a real `forge-server`, twice, and check the
 * result. Needs the server binary (`FORGE_SERVER_BIN`, or `target/debug/forge-server` from
 * `cargo build -p forge-server`); skipped with a message when it is missing.
 */

import { Database } from 'bun:sqlite'
import { afterAll, describe, expect, test } from 'bun:test'
import { copyFileSync } from 'fs'
import { join } from 'path'
import { fetchBytes, rawClient } from '../api'
import { type GenerateResult, generatePersona } from '../generate'
import { checkInvariants } from '../postpass/invariants'
import { META_KEYS, readDatasetMeta } from '../postpass/meta'
import { PERSONAS } from '../personas'
import { resolveServerBin, startServer } from '../server'
import { DEFAULT_AS_OF, parseAsOf } from '../time'
import { verifyDataset } from '../verify'
import { tempDir } from './helpers'

/** Tables every persona must fill. */
const PLANNED = [
  'addresses', 'answer_bank', 'bullet_skills', 'bullet_sources', 'bullets', 'certification_skills', 'certifications',
  'contact_job_descriptions', 'contact_organizations', 'contact_resumes', 'contacts', 'credentials', 'dataset_meta',
  'industries', 'job_description_resumes', 'job_description_skills', 'job_descriptions', 'note_references', 'org_aliases',
  'org_locations', 'org_tags', 'organizations', 'perspectives', 'profile_urls', 'prompt_logs', 'resume_certifications',
  'resume_entries', 'resume_sections', 'resume_skills', 'resumes', 'role_types', 'skill_domains', 'skills',
  'source_education', 'source_presentations', 'source_projects', 'source_roles', 'source_skills', 'sources', 'summaries',
  'summary_skills', 'user_notes', 'user_profile',
]
/** Tables a generated dataset must leave empty. */
const EXPECTED_EMPTY = ['pending_derivations', 'embeddings', 'alignment_results', 'v1_import_map']
/** Seeded by migrations, or not reachable through the API (nothing writes them yet). */
const UNPLANNED = [
  '_migrations', 'archetype_domains', 'archetypes', 'domains', 'extension_config', 'extension_logs', 'perspective_skills',
  'resume_templates', 'skill_categories', 'skill_graph_edges', 'skill_graph_nodes', 'sqlite_sequence',
]

const bin = resolveServerBin()
const SEED = 'integration-test'
const work = tempDir('integration')
afterAll(() => work.cleanup())

if (!bin) {
  describe('generate (integration)', () => {
    test.skip('needs forge-server: run `cargo build -p forge-server` or set FORGE_SERVER_BIN', () => {})
  })
} else {
  for (const corpus of Object.values(PERSONAS)) {
    describe(`generate ${corpus.slug} (integration)`, () => {
      let first: GenerateResult
      let second: GenerateResult

      test(
        'generates twice with the same content fingerprint',
        async () => {
          first = await generatePersona({ corpus, seed: SEED, asOf: DEFAULT_AS_OF, outDir: join(work.dir, 'a'), serverBin: bin })
          second = await generatePersona({ corpus, seed: SEED, asOf: DEFAULT_AS_OF, outDir: join(work.dir, 'b'), serverBin: bin })
          expect(second.manifest.content_fingerprint).toBe(first.manifest.content_fingerprint)
          // The ids differ, so the bytes do; the content does not.
          expect(second.manifest.sha256).not.toBe(first.manifest.sha256)
          expect(first.checks.drift).toBe(0)
        },
        180_000,
      )

      test('every table is classified; planned tables are filled, the rest empty where expected', () => {
        const counts = first.manifest.counts
        const unclassified = Object.keys(counts).filter((t) => ![...PLANNED, ...EXPECTED_EMPTY, ...UNPLANNED].includes(t))
        expect(unclassified).toEqual([])
        expect(PLANNED.filter((t) => !(counts[t] && counts[t] > 0))).toEqual([])
        expect(EXPECTED_EMPTY.filter((t) => counts[t] !== 0)).toEqual([])
      })

      test('invariants, integrity_check, foreign_key_check and dataset_meta', () => {
        const db = new Database(first.dbPath, { readonly: true })
        try {
          expect(checkInvariants(db, { asOf: parseAsOf(DEFAULT_AS_OF), corpus })).toEqual([])
          expect(db.query('PRAGMA integrity_check').get()).toEqual({ integrity_check: 'ok' })
          expect(db.query('PRAGMA foreign_key_check').all()).toEqual([])
          const meta = readDatasetMeta(db)
          expect(Object.keys(meta).sort()).toEqual([...META_KEYS].sort())
          expect(meta).toMatchObject({ kind: 'generated', persona: corpus.slug, seed: SEED, as_of: DEFAULT_AS_OF, dataset_uuid: first.manifest.uuid })
        } finally {
          db.close()
        }
      })

      test('verify passes on the written files', () => {
        expect(verifyDataset(join(work.dir, 'a'), corpus).problems).toEqual([])
      })

      test(
        'the file reopens in forge-server: no pending migrations, no drift, every resume renders a PDF',
        async () => {
          const copy = join(work.dir, `${corpus.slug}-reopen.sqlite`)
          copyFileSync(first.dbPath, copy)
          const server = await startServer({ bin, dbPath: copy })
          try {
            const raw = rawClient(server.baseUrl)
            expect(await raw<unknown[]>('GET', '/integrity/drift')).toEqual([])
            const resumes = await raw<{ id: string }[]>('GET', '/resumes?limit=200')
            expect(resumes.length).toBe(corpus.resumes.length)
            for (const r of resumes) {
              const pdf = await fetchBytes(server.baseUrl, 'POST', `/resumes/${r.id}/pdf`)
              expect(new TextDecoder().decode(pdf.slice(0, 4))).toBe('%PDF')
            }
          } finally {
            await server.stop()
          }
          // The server switched the copy to WAL; a read-only open could not create the -shm.
          const db = new Database(copy)
          try {
            expect((db.query('SELECT COUNT(*) AS n FROM _migrations').get() as { n: number }).n).toBe(first.manifest.counts._migrations as number)
          } finally {
            db.close()
          }
        },
        60_000,
      )
    })
  }
}
