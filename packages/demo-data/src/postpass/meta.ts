/**
 * The `dataset_meta` marker (migration 055): key/value rows that say a database was
 * generated, by what, and from which inputs. Real user databases leave the table empty.
 */

import type { Database } from 'bun:sqlite'

export const META_KEYS = [
  'kind',
  'generator',
  'generator_version',
  'seed',
  'persona',
  'dataset_uuid',
  'as_of',
  'generated_at',
  'schema_head',
] as const

export type MetaKey = (typeof META_KEYS)[number]
export type DatasetMeta = Record<MetaKey, string>

/** Keys whose values legitimately differ between two otherwise identical runs. */
export const VOLATILE_META_KEYS: readonly MetaKey[] = ['generated_at']

/** The newest applied migration, e.g. `055_dataset_meta`. */
export function schemaHead(db: Database): string {
  const row = db.query('SELECT name FROM _migrations ORDER BY name DESC LIMIT 1').get() as { name: string } | null
  if (!row) throw new Error('no _migrations rows: not a Forge database')
  return row.name
}

export function writeDatasetMeta(db: Database, meta: DatasetMeta): void {
  const existing = (db.query('SELECT COUNT(*) AS n FROM dataset_meta').get() as { n: number }).n
  if (existing > 0) throw new Error(`dataset_meta already has ${existing} row(s)`)
  for (const key of META_KEYS) {
    const value = meta[key]
    if (typeof value !== 'string' || value === '') throw new Error(`dataset_meta.${key} is empty`)
    db.run('INSERT INTO dataset_meta (key, value) VALUES (?, ?)', [key, value])
  }
}

export function readDatasetMeta(db: Database): Partial<DatasetMeta> & Record<string, string> {
  const rows = db.query('SELECT key, value FROM dataset_meta').all() as { key: string; value: string }[]
  return Object.fromEntries(rows.map((r) => [r.key, r.value]))
}
