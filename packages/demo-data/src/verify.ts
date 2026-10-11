/**
 * Verify a generated dataset on disk against its manifest, without regenerating it:
 * hashes, compaction, `dataset_meta`, invariants, row counts and the content fingerprint.
 */

import { Database } from 'bun:sqlite'
import { existsSync, readFileSync } from 'fs'
import { join } from 'path'
import type { PersonaCorpus } from './corpus/types'
import { personaUuid } from './ids'
import { verifyCompacted } from './postpass/compact'
import { contentFingerprint } from './postpass/fingerprint'
import { checkInvariants } from './postpass/invariants'
import { type Manifest, sha256File, tableCounts } from './postpass/manifest'
import { META_KEYS, readDatasetMeta } from './postpass/meta'
import { parseAsOf } from './time'

export interface VerifyReport {
  persona: string
  dbPath: string
  problems: string[]
  manifest?: Manifest
}

export function verifyDataset(outDir: string, corpus: PersonaCorpus): VerifyReport {
  const uuid = personaUuid(corpus.slug)
  const dir = join(outDir, 'user', uuid)
  const dbPath = join(dir, 'data.sqlite')
  const manifestPath = join(dir, 'manifest.json')
  const report: VerifyReport = { persona: corpus.slug, dbPath, problems: [] }
  const fail = (p: string) => report.problems.push(p)

  if (!existsSync(dbPath)) {
    fail(`missing ${dbPath} (run: just demo-data generate ${corpus.slug})`)
    return report
  }
  if (!existsSync(manifestPath)) {
    fail(`missing ${manifestPath}`)
    return report
  }
  const manifest = JSON.parse(readFileSync(manifestPath, 'utf8')) as Manifest
  report.manifest = manifest

  if (manifest.uuid !== uuid) fail(`manifest uuid ${manifest.uuid} != ${uuid}`)
  if (manifest.persona !== corpus.slug) fail(`manifest persona ${manifest.persona} != ${corpus.slug}`)
  if (manifest.r2_key !== `user/${uuid}/data.sqlite`) fail(`manifest r2_key ${manifest.r2_key} is not user/${uuid}/data.sqlite`)
  const bytes = Bun.file(dbPath).size
  if (bytes !== manifest.bytes) fail(`size ${bytes} != manifest ${manifest.bytes}`)
  const sha = sha256File(dbPath)
  if (sha !== manifest.sha256) fail(`sha256 ${sha} != manifest ${manifest.sha256}`)

  for (const p of verifyCompacted(dbPath)) fail(`compaction: ${p}`)

  const db = new Database(dbPath, { readonly: true })
  try {
    const meta = readDatasetMeta(db)
    for (const k of META_KEYS) if (!meta[k]) fail(`dataset_meta.${k} missing`)
    const expected: Record<string, string> = {
      kind: 'generated',
      generator: manifest.generator,
      generator_version: manifest.generator_version,
      seed: manifest.seed,
      persona: manifest.persona,
      dataset_uuid: manifest.uuid,
      as_of: manifest.as_of,
      generated_at: manifest.generated_at,
      schema_head: manifest.schema_head,
    }
    for (const [k, v] of Object.entries(expected)) {
      if (meta[k] !== v) fail(`dataset_meta.${k} = ${meta[k]}, manifest says ${v}`)
    }

    const asOf = parseAsOf(meta.as_of ?? manifest.as_of)
    for (const p of checkInvariants(db, { asOf, corpus })) fail(p)

    const counts = tableCounts(db)
    for (const [t, n] of Object.entries(manifest.counts)) {
      if (counts[t] !== n) fail(`count ${t} = ${counts[t]}, manifest says ${n}`)
    }
    for (const t of Object.keys(counts)) if (!(t in manifest.counts)) fail(`table ${t} is not in the manifest counts`)

    const fp = contentFingerprint(db)
    if (fp !== manifest.content_fingerprint) fail(`content_fingerprint ${fp} != manifest ${manifest.content_fingerprint}`)
  } catch (e) {
    fail(`verification error: ${(e as Error).message}`)
  } finally {
    db.close()
  }
  return report
}
