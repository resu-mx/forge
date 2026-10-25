/**
 * Types for the snapshot sync engine (`engine.ts`).
 *
 * The behaviour they describe is the "Client behaviour" section of the snapshot API contract:
 * what the client does at start and sign-in, when it pushes, and what it refuses to send or
 * import. Everything the engine touches (network, database runtime, storage, timers) is injected
 * through `SyncDeps`, so it runs and is tested without a browser.
 */

/** `localStorage` key of the per-browser sync record. It never lives inside the database. */
export const SYNC_STORAGE_KEY = 'forge.sync.v1'

/** The per-browser sync state, stored under `SYNC_STORAGE_KEY`. */
export interface SyncRecord {
  /** The account (`storageId`) this browser's database is synced with; null until first bound. */
  boundStorageId: string | null
  /** The `ETag` of the cloud copy this browser last pushed or pulled; null if it has none. */
  lastSyncedEtag: string | null
  /** The database changed since the last push or pull. */
  dirty: boolean
  /** When the last successful sync check finished, in ms since the epoch. */
  lastSyncAt: number | null
}

/** Get and set the record. `localStorage` is synchronous, so this is too. */
export interface SyncStorage {
  get(): SyncRecord | null
  set(record: SyncRecord): void
}

// ── The database runtime ────────────────────────────────────────────

/** What `ForgeRuntime.exportDatabase()` resolves to (`@forge/runtime`'s `ExportInfo`). */
export interface ExportResult {
  bytes: Uint8Array
  /** Newest migration recorded in `bytes`; null if they could not be read as a database. */
  schema_head: string | null
  known_head: string
  bundle_version: string
}

/** What `ForgeRuntime.importDatabase()` resolves to (`@forge/runtime`'s `ImportInfo`). */
export interface ImportResult {
  imported_bytes: number
  migrations: number
  schema_head: string | null
  known_head: string
  bundle_version: string
}

/** The slice of `ForgeRuntime` the engine uses. */
export interface SyncRuntime {
  exportDatabase(): Promise<ExportResult>
  /** Throws an error with `code: 'schema_too_new'` for a snapshot this bundle must not open. */
  importDatabase(bytes: Uint8Array): Promise<ImportResult>
  /**
   * The newest migration this bundle knows (`StartInfo.known_head`), or undefined while the
   * runtime has not started. The engine refuses to import a newer snapshot.
   */
  knownHead(): string | undefined
}

// ── Injected plumbing ───────────────────────────────────────────────

/** The `fetch` the engine calls, always with `credentials: 'include'`. */
export type SyncFetch = (url: string, init: RequestInit) => Promise<Response>

export interface SyncClock {
  /** Milliseconds since the epoch. */
  now(): number
  setTimeout(fn: () => void, ms: number): unknown
  clearTimeout(handle: unknown): void
}

export interface SyncOptions {
  /** Quiet time after the last change before a push. The contract says 30 s. */
  debounceMs: number
  /** First retry delay after a network or server error; it doubles each time. */
  backoffBaseMs: number
  /** The longest retry delay, for backoff and for `Retry-After`. */
  backoffCapMs: number
  /** Retries after one failure before the engine stops and waits for the next change or sync. */
  maxRetries: number
  /** Times one sync may re-enter the decision table (a 412, a vanished object) before it fails. */
  maxReentries: number
}

export const DEFAULT_SYNC_OPTIONS: SyncOptions = {
  debounceMs: 30_000,
  backoffBaseMs: 2_000,
  backoffCapMs: 300_000,
  maxRetries: 6,
  maxReentries: 3,
}

export interface SyncDeps {
  /** Origin of the snapshot API, for example `https://data.resu.mx` (no trailing slash needed). */
  baseUrl: string
  fetch: SyncFetch
  runtime: SyncRuntime
  storage: SyncStorage
  clock: SyncClock
  /** True in the tab that owns the database (the `forge-database-owner` lock holder, ready). */
  isOwner(): boolean
  /** Whether this browser's database holds user data worth keeping (not just the seeded schema). */
  localHasData(): Promise<boolean>
  onEvent(event: SyncEvent): void
  options?: Partial<SyncOptions>
}

// ── The snapshot API, as the engine sees it ─────────────────────────

/** What a `HEAD /v1/snapshot` says about the cloud copy. */
export interface RemoteSnapshot {
  kind: 'exists'
  /** Opaque: sent back verbatim in `If-Match`. */
  etag: string
  schemaHead: string | null
  bundleVersion: string | null
  lastModified: string | null
}

export type RemoteState = { kind: 'none' } | RemoteSnapshot

// ── The decision table ──────────────────────────────────────────────

export type SyncAction = 'first-upload' | 'bind' | 'pull' | 'push' | 'prompt' | 'noop'

/** Why the user has to choose. */
export type ConflictReason =
  /** A cloud copy exists and this browser has never been bound. */
  | 'unbound'
  /** Signed in as a different account than the one this browser is bound to. */
  | 'account-switch'
  /** The cloud copy changed and so did this browser. */
  | 'diverged'

export type Decision =
  | { action: 'first-upload' | 'bind' | 'pull' | 'push' | 'noop' }
  | { action: 'prompt'; reason: ConflictReason }

export interface DecideInput {
  remote: RemoteState
  /** The signed-in account. */
  storageId: string
  record: Pick<SyncRecord, 'boundStorageId' | 'lastSyncedEtag' | 'dirty'>
  /** Whether this browser's database holds user data. Only matters when there is no cloud copy. */
  localHasData: boolean
}

/** What the user picks at a conflict. */
export type ConflictChoice = 'use-cloud' | 'replace-cloud'

export interface SyncConflict {
  reason: ConflictReason
  /** The signed-in account. */
  storageId: string
  boundStorageId: string | null
  /**
   * The cloud copy. When it is `none` (an account switch to an account with nothing stored) only
   * `replace-cloud` applies: there is no cloud copy to use.
   */
  remote: RemoteState
}

// ── Status, events, errors ──────────────────────────────────────────

export type SyncStatus =
  /** No account has been set. */
  | 'idle'
  | 'signed-out'
  /** Another tab owns the database; nothing syncs from this one. */
  | 'not-owner'
  | 'syncing'
  | 'synced'
  | 'conflict'
  | 'error'

export type SyncErrorCode =
  | 'unauthorized'
  | 'network'
  | 'server'
  | 'rate_limited'
  | 'too_large'
  | 'invalid_snapshot'
  | 'precondition_required'
  | 'unexpected_status'
  /** The exported file is not a rollback-journal database (header bytes 18-19 are not 1,1). */
  | 'journal_mode'
  /** The export is not a database the service would accept. */
  | 'invalid_export'
  /** The cloud copy was written by a newer bundle than this one. */
  | 'schema_too_new'
  | 'import_failed'
  | 'runtime_unavailable'
  /** Re-entering the decision table did not settle. */
  | 'conflict_loop'
  /** `resolveConflict` was called with a choice that does not apply. */
  | 'invalid_choice'
  | 'unexpected'

export interface SyncError {
  code: SyncErrorCode
  message: string
  /** The HTTP status, when one caused this. */
  status?: number
  /** When the next automatic retry runs (ms since the epoch); absent if none is scheduled. */
  retryAt?: number
}

export type SyncEvent =
  | { type: 'status'; status: SyncStatus }
  | { type: 'conflict'; conflict: SyncConflict }
  | { type: 'synced'; action: Exclude<SyncAction, 'prompt'>; etag: string | null; at: number }
  /** A pull replaced the database: the page must reload to drop what it holds from the old one. */
  | { type: 'reload' }
  | { type: 'error'; error: SyncError }
  /** The session is gone (a 401). The engine stopped; call `setAccount` again after sign-in. */
  | { type: 'unauthorized' }

export interface SyncState {
  status: SyncStatus
  conflict: SyncConflict | null
  error: SyncError | null
}

export interface SyncEngine {
  state(): SyncState
  /**
   * Set the signed-in account (its `storageId`), or null on sign-out. Sets the engine going: the
   * cloud copy is checked and the decision table applied. Call it on app start once the session is
   * known, and again on sign-in.
   */
  setAccount(storageId: string | null): Promise<void>
  /** Check the cloud copy and apply the decision table now ("Sync now", ownership gained). */
  sync(): Promise<void>
  /** The database changed (`forge:changed`). Marks the record dirty and arms the debounced push. */
  notifyChange(): void
  /** Push now if there is anything to push (the page is being hidden). */
  flush(): Promise<void>
  /** Settle a conflict. `use-cloud` replaces this browser's data; `replace-cloud` replaces the cloud copy. */
  resolveConflict(choice: ConflictChoice): Promise<void>
  /** Stop timers; the engine does nothing afterwards. */
  dispose(): void
}
