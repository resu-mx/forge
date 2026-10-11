/**
 * The R2 operator commands behind `cli.ts`: push (preview) / publish (prod), pull, head, ls.
 *
 * They take an `ObjectStore` (an `R2Objects` in real use, an in-memory fake in tests) so the
 * guard and skip logic is testable without a network.
 */

import { Database } from 'bun:sqlite'
import { existsSync, mkdirSync, readFileSync, renameSync, rmSync, writeFileSync } from 'fs'
import { dirname, join } from 'path'
import type { PersonaCorpus } from '../corpus/types'
import { isCanonicalUuid, personaUuid } from '../ids'
import { PERSONAS } from '../personas'
import type { Manifest } from '../postpass/manifest'
import { readDatasetMeta } from '../postpass/meta'
import { checkPushGuards, registeredDemoUuids, repoSchemaHead } from './guards'
import { type R2Meta, buildR2Meta } from './meta'
import { type ListEntry, type ObjectHead, type PutOptions, sha256Hex, userKey } from './objects'

/** The subset of `R2Objects` the commands use. */
export interface ObjectStore {
  readonly bucket: string
  put(key: string, bytes: Uint8Array, meta: Record<string, string>, opts?: PutOptions): Promise<{ etag: string | null }>
  head(key: string): Promise<ObjectHead | null>
  get(key: string): Promise<{ bytes: Uint8Array; head: ObjectHead } | null>
  delete(key: string): Promise<void>
  list(prefix: string): Promise<ListEntry[]>
}

/** Bad command-line input (an unknown id): printed without a stack trace. */
export class UsageError extends Error {
  override name = 'UsageError'
}

export class PushRefusedError extends Error {
  override name = 'PushRefusedError'
  constructor(
    readonly persona: string,
    readonly problems: string[],
  ) {
    super(`refusing to push ${persona}:\n${problems.map((p) => `  - ${p}`).join('\n')}`)
  }
}

export class PullVerificationError extends Error {
  override name = 'PullVerificationError'
  constructor(
    readonly key: string,
    readonly problems: string[],
  ) {
    super(`${key} failed verification; nothing was written:\n${problems.map((p) => `  - ${p}`).join('\n')}`)
  }
}

// ─── Datasets on disk ──────────────────────────────────────

export interface DatasetFiles {
  slug: string
  uuid: string
  key: string
  dbPath: string
  manifestPath: string
}

export function datasetFiles(outDir: string, slug: string): DatasetFiles {
  const uuid = personaUuid(slug)
  const dir = join(outDir, 'user', uuid)
  return { slug, uuid, key: userKey(uuid), dbPath: join(dir, 'data.sqlite'), manifestPath: join(dir, 'manifest.json') }
}

/** A dataset id given as a canonical uuid or a persona slug → its uuid and slug (if known). */
export function resolveDatasetId(id: string | undefined): { uuid: string; slug: string | null } {
  if (!id) throw new UsageError('give a dataset uuid or persona slug')
  if (isCanonicalUuid(id)) return { uuid: id, slug: registeredDemoUuids().get(id) ?? null }
  if (Object.hasOwn(PERSONAS, id)) return { uuid: personaUuid(id), slug: id }
  throw new UsageError(`${JSON.stringify(id)} is neither a canonical lowercase uuid nor a persona (${Object.keys(PERSONAS).join(', ')})`)
}

// ─── push / publish ────────────────────────────────────────

export interface PushOptions {
  personas: PersonaCorpus[]
  /** Dataset root (`data/demo`). */
  outDir: string
  allowStale?: boolean
  /** Upload even when the remote fingerprint and schema head already match. */
  force?: boolean
  /** Defaults to the repo's newest migration on disk. */
  repoSchemaHead?: string
  log?: (message: string) => void
}

export interface PushOutcome {
  persona: string
  key: string
  action: 'uploaded' | 'skipped'
  sha256: string
  bytes: number
  /** Why it was skipped. */
  reason?: string
}

/**
 * Push each persona's dataset. Every guard runs before any network call; a refusal throws
 * `PushRefusedError` and nothing is uploaded for that persona.
 */
export async function pushDatasets(store: ObjectStore, opts: PushOptions): Promise<PushOutcome[]> {
  const head = opts.repoSchemaHead ?? repoSchemaHead()
  const outcomes: PushOutcome[] = []
  for (const corpus of opts.personas) {
    outcomes.push(await pushDataset(store, datasetFiles(opts.outDir, corpus.slug), { ...opts, repoSchemaHead: head }))
  }
  return outcomes
}

export async function pushDataset(
  store: ObjectStore,
  files: DatasetFiles,
  opts: Pick<PushOptions, 'allowStale' | 'force' | 'repoSchemaHead' | 'log'>,
): Promise<PushOutcome> {
  const log = opts.log ?? (() => {})
  if (!existsSync(files.dbPath) || !existsSync(files.manifestPath)) {
    throw new PushRefusedError(files.slug, [`no dataset at ${dirname(files.dbPath)} (run: just demo-data generate ${files.slug})`])
  }
  const manifest = JSON.parse(readFileSync(files.manifestPath, 'utf8')) as Manifest
  // Read once: the guards check these bytes and these bytes are what gets uploaded.
  const bytes = new Uint8Array(readFileSync(files.dbPath))
  const guards = checkPushGuards({
    bytes,
    manifest,
    key: files.key,
    repoSchemaHead: opts.repoSchemaHead ?? repoSchemaHead(),
    allowStale: opts.allowStale,
  })
  if (guards.problems.length > 0) throw new PushRefusedError(files.slug, guards.problems)
  const meta = buildR2Meta(manifest, guards.datasetMeta)
  const sha256 = guards.sha256 as string

  const remote = await store.head(files.key)
  if (remote) {
    if (remote.meta['forge-kind'] !== 'generated' && !opts.force) {
      throw new PushRefusedError(files.slug, [
        `${files.key} already exists in ${store.bucket} without forge-kind=generated metadata; refusing to overwrite it (pass --force if you are sure)`,
      ])
    }
    const same =
      remote.meta['forge-content-fingerprint'] === meta['forge-content-fingerprint'] && remote.meta['forge-schema-head'] === meta['forge-schema-head']
    if (same && !opts.force) {
      const reason = `remote content fingerprint and schema head already match (pass --force to re-upload)`
      log(`= ${files.slug}: ${files.key} up to date in ${store.bucket}; ${reason}`)
      return { persona: files.slug, key: files.key, action: 'skipped', sha256, bytes: bytes.byteLength, reason }
    }
  }

  // Create-only when the key is new, so two concurrent pushes cannot silently clobber.
  await store.put(files.key, bytes, meta, { ifNoneMatch: remote === null })
  const after = await store.head(files.key)
  const problems = verifyUploaded(after, bytes.byteLength, meta)
  if (problems.length > 0) throw new Error(`uploaded ${files.key} but the object does not read back correctly:\n  - ${problems.join('\n  - ')}`)
  log(`↑ ${files.slug}: ${files.key} → ${store.bucket} (${bytes.byteLength} bytes, sha256 ${sha256.slice(0, 12)}…)`)
  return { persona: files.slug, key: files.key, action: 'uploaded', sha256, bytes: bytes.byteLength }
}

function verifyUploaded(head: ObjectHead | null, size: number, meta: R2Meta): string[] {
  if (!head) return ['HEAD after PUT found no object']
  const problems: string[] = []
  if (head.size !== size) problems.push(`size ${head.size} != ${size}`)
  for (const [k, v] of Object.entries(meta)) if (head.meta[k] !== v) problems.push(`metadata ${k} is ${head.meta[k] ?? '(missing)'}, expected ${v}`)
  return problems
}

// ─── pull ──────────────────────────────────────────────────

/** Problems with downloaded dataset bytes (empty when the file is a generated dataset for `uuid`). */
export function inspectDatasetBytes(bytes: Uint8Array, uuid: string): string[] {
  const problems: string[] = []
  if (bytes.byteLength < 100 || Buffer.from(bytes.subarray(0, 16)).toString('latin1') !== 'SQLite format 3\u0000') {
    return ['not a SQLite database']
  }
  if (bytes[18] !== 1 || bytes[19] !== 1) return [`header bytes 18-19 are ${bytes[18]},${bytes[19]}; expected 1,1 (DELETE journal mode)`]
  let db: Database
  try {
    db = Database.deserialize(bytes, { readonly: true })
  } catch (e) {
    return [`cannot open the database: ${(e as Error).message}`]
  }
  try {
    if (!db.query("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'dataset_meta'").get()) return ['no dataset_meta table']
    const meta = readDatasetMeta(db)
    if (meta.kind !== 'generated') problems.push(`dataset_meta.kind is ${meta.kind ?? '(missing)'}, expected generated`)
    if (meta.dataset_uuid !== uuid) problems.push(`dataset_meta.dataset_uuid ${meta.dataset_uuid ?? '(missing)'} != ${uuid}`)
    const check = db.query('PRAGMA integrity_check').all() as { integrity_check: string }[]
    if (check.length !== 1 || check[0]?.integrity_check !== 'ok') problems.push(`integrity_check: ${check.map((r) => r.integrity_check).join('; ')}`)
  } finally {
    db.close()
  }
  return problems
}

export interface PullOptions {
  id: string
  /** Destination file. */
  outPath: string
  /** Overwrite an existing file at `outPath`. */
  force?: boolean
}

export interface PullResult {
  key: string
  uuid: string
  slug: string | null
  path: string
  bytes: number
  sha256: string
  meta: Record<string, string>
}

export async function pullDataset(store: ObjectStore, opts: PullOptions): Promise<PullResult> {
  const { uuid, slug } = resolveDatasetId(opts.id)
  const key = userKey(uuid)
  if (existsSync(opts.outPath) && !opts.force) throw new Error(`${opts.outPath} exists; pass --force to overwrite it`)
  const obj = await store.get(key)
  if (!obj) throw new Error(`no object at ${key} in ${store.bucket}`)

  const { bytes, head } = obj
  const sha256 = sha256Hex(bytes)
  const problems: string[] = []
  if (head.meta['forge-kind'] !== 'generated') problems.push(`object metadata forge-kind is ${head.meta['forge-kind'] ?? '(missing)'}, expected generated`)
  if (!head.meta['forge-sha256']) problems.push('object has no forge-sha256 metadata')
  else if (head.meta['forge-sha256'] !== sha256) problems.push(`sha256 ${sha256} does not match object metadata ${head.meta['forge-sha256']}`)
  if (head.meta['forge-dataset-uuid'] !== uuid) problems.push(`object metadata forge-dataset-uuid ${head.meta['forge-dataset-uuid'] ?? '(missing)'} != ${uuid}`)
  problems.push(...inspectDatasetBytes(bytes, uuid))
  if (problems.length > 0) throw new PullVerificationError(key, problems)

  mkdirSync(dirname(opts.outPath), { recursive: true })
  const partial = `${opts.outPath}.partial-${process.pid}`
  try {
    writeFileSync(partial, bytes)
    renameSync(partial, opts.outPath)
  } finally {
    rmSync(partial, { force: true })
  }
  return { key, uuid, slug, path: opts.outPath, bytes: bytes.byteLength, sha256, meta: head.meta }
}

/** How to load a pulled file into the browser app. */
export function importInstructions(path: string): string {
  return [
    'Import it into Forge (browser runtime):',
    '  1. Open the app, then Settings → Storage.',
    '  2. Optional: Backup → "Export database" to keep a copy of the current data.',
    `  3. Restore or move in → choose ${path}`,
    '  4. Confirm "Replace data". The app migrates the file if it is older than the app, then reloads.',
    'The import replaces everything in that browser profile; use a separate profile for demos.',
  ].join('\n')
}

// ─── head / ls ─────────────────────────────────────────────

export async function headDataset(store: ObjectStore, id: string): Promise<{ key: string; slug: string | null; head: ObjectHead | null }> {
  const { uuid, slug } = resolveDatasetId(id)
  const key = userKey(uuid)
  return { key, slug, head: await store.head(key) }
}

export function formatHead(key: string, slug: string | null, head: ObjectHead): string {
  const lines = [
    `${key}${slug ? ` (${slug})` : ''}`,
    `  size          ${head.size}`,
    `  etag          ${head.etag ?? '-'}`,
    `  last-modified ${head.lastModified ?? '-'}`,
    `  content-type  ${head.contentType ?? '-'}`,
  ]
  for (const [k, v] of Object.entries(head.meta)) lines.push(`  ${k.padEnd(26)} ${v}`)
  return lines.join('\n')
}

export interface ListedDataset extends ListEntry {
  slug: string | null
}

export async function listDatasets(store: ObjectStore, prefix = 'user/'): Promise<ListedDataset[]> {
  const registered = registeredDemoUuids()
  const entries = await store.list(prefix)
  return entries.map((e) => {
    const m = /^user\/([^/]+)\//.exec(e.key)
    return { ...e, slug: m ? (registered.get(m[1] as string) ?? null) : null }
  })
}
