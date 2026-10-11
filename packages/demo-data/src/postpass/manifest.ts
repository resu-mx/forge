/**
 * `manifest.json`, written next to each `data.sqlite`: what the file is, how it was made,
 * what it holds, and the hashes that identify it.
 */

import type { Database } from 'bun:sqlite'
import { readFileSync } from 'fs'
import type { OverlayLogEntry } from './overlay'

export interface Manifest {
  uuid: string
  persona: string
  seed: string
  as_of: string
  generator: string
  generator_version: string
  schema_head: string
  generated_at: string
  /** The object key this file is published under. */
  r2_key: string
  bytes: number
  sha256: string
  content_fingerprint: string
  counts: Record<string, number>
  coverage: Record<string, Record<string, number>>
  overlay: OverlayLogEntry[]
}

/** `user/<uuid>/data.sqlite`. */
export function datasetKey(uuid: string): string {
  return `user/${uuid}/data.sqlite`
}

export function sha256File(path: string): string {
  const hasher = new Bun.CryptoHasher('sha256')
  hasher.update(readFileSync(path))
  return hasher.digest('hex')
}

/** Row counts for every table. */
export function tableCounts(db: Database): Record<string, number> {
  const names = (db.query("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name").all() as { name: string }[]).map((r) => r.name)
  return Object.fromEntries(names.map((t) => [t, (db.query(`SELECT COUNT(*) AS n FROM "${t}"`).get() as { n: number }).n]))
}

function tally(db: Database, sql: string): Record<string, number> {
  const rows = db.query(sql).all() as { k: string | null; n: number }[]
  return Object.fromEntries(rows.map((r) => [r.k ?? 'NULL', r.n]))
}

/** Distribution of the values the boards and pickers group by. */
export function coverageReport(db: Database): Record<string, Record<string, number>> {
  return {
    sources: tally(db, 'SELECT status AS k, COUNT(*) AS n FROM sources GROUP BY status ORDER BY status'),
    bullets: tally(db, 'SELECT status AS k, COUNT(*) AS n FROM bullets GROUP BY status ORDER BY status'),
    perspectives: tally(db, 'SELECT status AS k, COUNT(*) AS n FROM perspectives GROUP BY status ORDER BY status'),
    resumes: tally(db, 'SELECT status AS k, COUNT(*) AS n FROM resumes GROUP BY status ORDER BY status'),
    job_descriptions: tally(db, 'SELECT status AS k, COUNT(*) AS n FROM job_descriptions GROUP BY status ORDER BY status'),
    organizations: tally(db, 'SELECT status AS k, COUNT(*) AS n FROM organizations GROUP BY status ORDER BY status'),
    source_types: tally(db, 'SELECT source_type AS k, COUNT(*) AS n FROM sources GROUP BY source_type ORDER BY source_type'),
    education_types: tally(db, 'SELECT education_type AS k, COUNT(*) AS n FROM source_education GROUP BY education_type ORDER BY education_type'),
    presentation_types: tally(db, 'SELECT presentation_type AS k, COUNT(*) AS n FROM source_presentations GROUP BY presentation_type ORDER BY presentation_type'),
    perspective_archetypes: tally(db, 'SELECT target_archetype AS k, COUNT(*) AS n FROM perspectives GROUP BY target_archetype ORDER BY target_archetype'),
    skill_categories: tally(db, 'SELECT category AS k, COUNT(*) AS n FROM skills GROUP BY category ORDER BY category'),
    note_reference_types: tally(db, 'SELECT entity_type AS k, COUNT(*) AS n FROM note_references GROUP BY entity_type ORDER BY entity_type'),
  }
}
