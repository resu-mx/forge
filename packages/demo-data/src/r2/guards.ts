/**
 * Push guards: the checks a file must pass before it may be uploaded. They exist so that
 * nothing but a generated, compacted, correctly keyed demo dataset can ever reach a bucket,
 * and above all never a real user's database.
 *
 * The checks run on the exact bytes that would be uploaded (the database is opened from
 * memory with `Database.deserialize`), so the file cannot change between check and upload.
 */

import { Database } from 'bun:sqlite'
import { readdirSync } from 'fs'
import { resolve } from 'path'
import { personaUuid } from '../ids'
import { PERSONAS } from '../personas'
import type { Manifest } from '../postpass/manifest'
import { readDatasetMeta, schemaHead } from '../postpass/meta'
import { REPO_ROOT } from '../server'
import { sha256Hex, uuidFromUserKey } from './objects'

export const MIGRATIONS_DIR = resolve(REPO_ROOT, 'packages/core/src/db/migrations')

/** The repo's newest migration (e.g. `055_dataset_meta`), from the files on disk. */
export function repoSchemaHead(dir: string = MIGRATIONS_DIR): string {
  const files = readdirSync(dir)
    .filter((f) => /^\d{3}_.+\.sql$/.test(f))
    .sort()
  const last = files.at(-1)
  if (!last) throw new Error(`no migrations in ${dir}`)
  return last.replace(/\.sql$/, '')
}

/** Every registered demo dataset uuid → its persona slug. */
export function registeredDemoUuids(): Map<string, string> {
  return new Map(Object.keys(PERSONAS).map((slug) => [personaUuid(slug), slug]))
}

export interface GuardInput {
  /** The bytes that would be uploaded. */
  bytes: Uint8Array
  manifest: Manifest
  /** The destination key, `user/<uuid>/data.sqlite`. */
  key: string
  repoSchemaHead: string
  /** Accept a dataset whose schema head is not the repo's. */
  allowStale?: boolean
  /** uuid → persona slug; defaults to `registeredDemoUuids()`. */
  registered?: ReadonlyMap<string, string>
}

export interface GuardResult {
  /** Empty when the upload may proceed. */
  problems: string[]
  sha256: string | null
  datasetMeta: Record<string, string>
}

const SQLITE_MAGIC = 'SQLite format 3\u0000'

export function checkPushGuards(input: GuardInput): GuardResult {
  const { bytes, manifest, key } = input
  const registered = input.registered ?? registeredDemoUuids()
  const result: GuardResult = { problems: [], sha256: null, datasetMeta: {} }
  const fail = (p: string) => result.problems.push(p)

  const keyUuid = uuidFromUserKey(key)
  if (!keyUuid) {
    fail(`key ${key} is not user/<canonical uuid>/data.sqlite`)
    return result
  }

  // 1. A SQLite file, in rollback (DELETE) journal mode. Anything else stops here: the
  //    browser import would drop a WAL file's unflushed pages, and we do not open it.
  if (bytes.byteLength < 100 || Buffer.from(bytes.subarray(0, 16)).toString('latin1') !== SQLITE_MAGIC) {
    fail('not a SQLite database (missing the "SQLite format 3" header)')
    return result
  }
  if (bytes[18] !== 1 || bytes[19] !== 1) {
    fail(`header bytes 18-19 are ${bytes[18]},${bytes[19]}; expected 1,1 (DELETE journal mode). A WAL-mode file is not a finished dataset`)
    return result
  }

  // 2. The bytes are the ones the manifest describes.
  result.sha256 = sha256Hex(bytes)
  if (result.sha256 !== manifest.sha256) fail(`sha256 ${result.sha256} does not match manifest ${manifest.sha256} (file changed after generation?)`)
  if (bytes.byteLength !== manifest.bytes) fail(`size ${bytes.byteLength} does not match manifest ${manifest.bytes}`)

  // 3. The key's uuid is a registered demo uuid, and the manifest agrees with the key.
  const persona = registered.get(keyUuid)
  if (!persona) fail(`uuid ${keyUuid} is not a registered demo dataset uuid (personaUuid of a known persona)`)
  if (manifest.uuid !== keyUuid) fail(`manifest uuid ${manifest.uuid} does not match key uuid ${keyUuid}`)
  if (manifest.r2_key !== key) fail(`manifest r2_key ${manifest.r2_key} does not match ${key}`)

  // 4. The marker inside the database.
  let db: Database
  try {
    db = Database.deserialize(bytes, { readonly: true })
  } catch (e) {
    fail(`cannot open the database: ${(e as Error).message}`)
    return result
  }
  try {
    const hasMeta = db.query("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'dataset_meta'").get()
    if (!hasMeta) {
      fail('no dataset_meta table: not a generated dataset')
      return result
    }
    const meta = readDatasetMeta(db)
    result.datasetMeta = meta
    if (meta.kind !== 'generated') fail(`dataset_meta.kind is ${meta.kind === undefined ? 'missing' : JSON.stringify(meta.kind)}; expected "generated"`)
    if (meta.dataset_uuid !== keyUuid) fail(`dataset_meta.dataset_uuid ${meta.dataset_uuid ?? '(missing)'} does not match key uuid ${keyUuid}`)
    if (persona && meta.persona !== persona) fail(`dataset_meta.persona ${meta.persona ?? '(missing)'} is not ${persona}, the persona of ${keyUuid}`)

    // 5. Schema head: recorded, matches the file's migrations, and is the repo's head.
    let fileHead: string | null = null
    try {
      fileHead = schemaHead(db)
    } catch (e) {
      fail((e as Error).message)
    }
    if (fileHead !== null && meta.schema_head !== fileHead) fail(`dataset_meta.schema_head ${meta.schema_head ?? '(missing)'} does not match the newest applied migration ${fileHead}`)
    if (manifest.schema_head !== meta.schema_head) fail(`manifest schema_head ${manifest.schema_head} does not match dataset_meta ${meta.schema_head ?? '(missing)'}`)
    if (!input.allowStale && meta.schema_head !== input.repoSchemaHead) {
      fail(`schema head ${meta.schema_head ?? '(missing)'} is not the repo's ${input.repoSchemaHead}: regenerate, or pass --allow-stale`)
    }

    const quick = db.query('PRAGMA quick_check').all() as { quick_check: string }[]
    if (quick.length !== 1 || quick[0]?.quick_check !== 'ok') fail(`quick_check: ${quick.map((r) => r.quick_check).join('; ')}`)
  } finally {
    db.close()
  }
  return result
}
