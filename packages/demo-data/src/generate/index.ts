/**
 * Generate one persona's dataset: spawn `forge-server` on a fresh file, drive every phase
 * through the API, stop the server, run the post-pass, and write
 * `<out>/user/<uuid>/{data.sqlite,manifest.json}` plus `<out>/index.json`.
 */

import { ForgeClient } from '@forge/sdk'
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { rawClient } from '../api'
import type { PersonaCorpus } from '../corpus/types'
import { personaUuid } from '../ids'
import { Ledger } from '../ledger'
import { runPostPass } from '../postpass'
import { type Manifest, datasetKey, sha256File } from '../postpass/manifest'
import { startServer } from '../server'
import { Clock, isoZ, parseAsOf } from '../time'
import { phaseResumes, phaseContacts, phaseJobDescriptions, phaseSummaries } from './applications'
import { phaseBullets, phasePerspectives, phaseSources } from './career'
import { type ApiCheckReport, phaseApiChecks } from './checks'
import type { GenContext } from './context'
import { phaseNotes } from './notes'
import { phaseOrganizations, phaseQualifications } from './orgs'
import { loadSeeded, phaseProfile, phaseReference } from './reference'

export const GENERATOR = '@forge/demo-data'
export const GENERATOR_VERSION: string = (JSON.parse(readFileSync(join(import.meta.dir, '../../package.json'), 'utf8')) as { version: string }).version

export interface GenerateOptions {
  corpus: PersonaCorpus
  seed: string
  asOf: string
  /** Output root, e.g. `<repo>/data/demo`. */
  outDir: string
  serverBin: string
  log?: (message: string) => void
  /** Keep the temporary working directory (for debugging). */
  keepTemp?: boolean
}

export interface GenerateResult {
  manifest: Manifest
  dbPath: string
  manifestPath: string
  checks: ApiCheckReport
}

export async function generatePersona(opts: GenerateOptions): Promise<GenerateResult> {
  const { corpus } = opts
  const log = opts.log ?? (() => {})
  const asOf = parseAsOf(opts.asOf)
  const uuid = personaUuid(corpus.slug)
  const work = mkdtempSync(join(tmpdir(), `forge-demo-${corpus.slug}-`))
  const workDb = join(work, 'data.sqlite')
  log(`${corpus.slug} (${uuid}) as of ${opts.asOf}, seed ${opts.seed}`)

  try {
    const server = await startServer({ bin: opts.serverBin, dbPath: workDb })
    const ledger = new Ledger()
    const clock = new Clock(asOf, opts.seed, corpus.slug)
    const ctx: GenContext = {
      corpus,
      forge: new ForgeClient({ baseUrl: server.baseUrl, debug: false }),
      raw: rawClient(server.baseUrl),
      baseUrl: server.baseUrl,
      ledger,
      clock,
      seed: opts.seed,
      overlay: [],
      noteReferences: [],
      seeded: { archetypes: new Map(), domains: new Map(), templates: new Map() },
      log,
    }

    let checks: ApiCheckReport
    try {
      await loadSeeded(ctx)
      await phaseReference(ctx)
      await phaseProfile(ctx)
      await phaseOrganizations(ctx)
      await phaseQualifications(ctx)
      await phaseSources(ctx)
      await phaseBullets(ctx)
      await phasePerspectives(ctx)
      // Source statuses are overlaid after derivation (a derivation refuses archived sources).
      for (const s of corpus.sources) {
        if (s.status !== 'draft') ctx.overlay.push({ table: 'sources', ref: `source:${s.key}`, id: ledger.id('source', s.key), from: 'draft', to: s.status })
      }
      await phaseSummaries(ctx)
      await phaseJobDescriptions(ctx)
      await phaseContacts(ctx)
      await phaseResumes(ctx)
      await phaseNotes(ctx, GENERATOR_VERSION)
      checks = await phaseApiChecks(ctx)
    } finally {
      await server.stop()
    }

    log('  · post-pass: overlay, backdate, dataset_meta, invariants, compaction')
    const generatedAt = isoZ(new Date())
    const post = runPostPass({
      dbPath: workDb,
      corpus,
      asOf,
      epoch: clock.epoch(corpus.accountAgeDays),
      entries: ledger.entries(),
      overlay: ctx.overlay,
      noteReferences: ctx.noteReferences,
      meta: {
        kind: 'generated',
        generator: GENERATOR,
        generator_version: GENERATOR_VERSION,
        seed: opts.seed,
        persona: corpus.slug,
        dataset_uuid: uuid,
        as_of: opts.asOf,
        generated_at: generatedAt,
      },
    })

    const destDir = join(opts.outDir, 'user', uuid)
    mkdirSync(destDir, { recursive: true })
    const dbPath = join(destDir, 'data.sqlite')
    for (const suffix of ['', '-wal', '-shm', '-journal']) rmSync(dbPath + suffix, { force: true })
    copyFileSync(workDb, dbPath)

    const manifest: Manifest = {
      uuid,
      persona: corpus.slug,
      seed: opts.seed,
      as_of: opts.asOf,
      generator: GENERATOR,
      generator_version: GENERATOR_VERSION,
      schema_head: post.schemaHead,
      generated_at: generatedAt,
      r2_key: datasetKey(uuid),
      bytes: Bun.file(dbPath).size,
      sha256: sha256File(dbPath),
      content_fingerprint: post.contentFingerprint,
      counts: post.counts,
      coverage: post.coverage,
      overlay: post.overlay,
    }
    const manifestPath = join(destDir, 'manifest.json')
    writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`)
    updateIndex(opts.outDir, manifest)
    log(`  ✓ ${dbPath} (${manifest.bytes} bytes, fingerprint ${manifest.content_fingerprint.slice(0, 12)}…)`)
    return { manifest, dbPath, manifestPath, checks }
  } finally {
    if (!opts.keepTemp) rmSync(work, { recursive: true, force: true })
  }
}

export interface IndexFile {
  generator: string
  personas: Record<string, { uuid: string; key: string; sha256: string; content_fingerprint: string }>
}

/** `<out>/index.json`: persona slug → dataset uuid (merged with existing entries). */
export function updateIndex(outDir: string, manifest: Manifest): void {
  const path = join(outDir, 'index.json')
  const index: IndexFile = existsSync(path) ? (JSON.parse(readFileSync(path, 'utf8')) as IndexFile) : { generator: GENERATOR, personas: {} }
  index.personas[manifest.persona] = {
    uuid: manifest.uuid,
    key: manifest.r2_key,
    sha256: manifest.sha256,
    content_fingerprint: manifest.content_fingerprint,
  }
  index.personas = Object.fromEntries(Object.entries(index.personas).sort(([a], [b]) => a.localeCompare(b)))
  writeFileSync(path, `${JSON.stringify(index, null, 2)}\n`)
}
