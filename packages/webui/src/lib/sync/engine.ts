/**
 * The snapshot sync engine: keeps this browser's database and the cloud copy in step.
 *
 * A state machine over injected dependencies (network, database runtime, storage, timers), so
 * it has no browser in it. The behaviour is the "Client behaviour" section of the snapshot API
 * contract; `decide()` is its table. Wiring it to the session, the runtime and the UI is the
 * sync store's job, not this file's.
 *
 * Rules the engine holds to:
 * - Only the tab that owns the database syncs (`isOwner()`).
 * - Sync state lives in the injected `storage` (`forge.sync.v1`), never in the database, so it
 *   never travels inside a snapshot.
 * - It never uploads bytes that are not a rollback-journal SQLite file (header bytes 18-19 are
 *   1,1: the service rejects anything else), and never imports a snapshot from a newer bundle.
 * - A 401 stops it (no retries); 413, 422, 428 and the like are not retried either; 429 waits
 *   for `Retry-After`; network and server errors back off, capped.
 * - A 412 on a push re-enters the decision table.
 * - With sync turned off (`record.syncEnabled === false`) it makes no request at all. Deleting the
 *   cloud copy turns sync off here, and another device that finds the copy gone asks before it uploads.
 */

import { decide } from './decide'
import { emptySyncRecord } from './storage'
import {
  DEFAULT_SYNC_OPTIONS,
  type ConflictChoice,
  type ExportResult,
  type RemoteSnapshot,
  type RemoteState,
  type SyncAction,
  type SyncConflict,
  type SyncDeps,
  type SyncEngine,
  type SyncError,
  type SyncErrorCode,
  type SyncEvent,
  type SyncRecord,
  type SyncState,
  type SyncStatus,
  type Decision,
} from './types'

const SNAPSHOT_PATH = '/v1/snapshot'
const SQLITE_HEADER = 'SQLite format 3\0'

/** A failure the engine knows how to answer, and whether (and when) to try again. */
class SyncFailure extends Error {
  constructor(
    readonly error: SyncError,
    readonly retry: 'none' | 'backoff' | { afterMs: number } = 'none',
  ) {
    super(error.message)
  }
}

/** This tab stopped owning the database part-way through an operation. */
class LostOwnership extends Error {}

function fail(
  code: SyncErrorCode,
  message: string,
  extra: Partial<SyncError> = {},
  retry: SyncFailure['retry'] = 'none',
): SyncFailure {
  return new SyncFailure({ code, message, ...extra }, retry)
}

function hasSqliteHeader(bytes: Uint8Array): boolean {
  if (bytes.length < SQLITE_HEADER.length) return false
  for (let i = 0; i < SQLITE_HEADER.length; i++) if (bytes[i] !== SQLITE_HEADER.charCodeAt(i)) return false
  return true
}

/** `Retry-After` as milliseconds: delta-seconds or an HTTP date. Null when absent or unusable. */
export function parseRetryAfter(value: string | null, now: number): number | null {
  if (!value) return null
  const trimmed = value.trim()
  const ms = /^\d+$/.test(trimmed) ? Number(trimmed) * 1000 : Date.parse(trimmed) - now
  return Number.isFinite(ms) ? Math.max(1000, ms) : null
}

const message = (e: unknown) => (e instanceof Error ? e.message : String(e))

export function createSyncEngine(deps: SyncDeps): SyncEngine {
  const opts = { ...DEFAULT_SYNC_OPTIONS, ...deps.options }
  const url = deps.baseUrl.replace(/\/+$/, '') + SNAPSHOT_PATH

  let account: string | null = null
  let status: SyncStatus = 'idle'
  let conflict: SyncConflict | null = null
  let error: SyncError | null = null
  /** A failure retrying cannot fix: no automatic pushes until `sync()` or `setAccount()`. */
  let blocked = false
  /** Counts changes, so a push can tell whether the database moved while it ran. */
  let changeSeq = 0
  /** True while our own import runs, so the change it announces is not taken for a user edit. */
  let importing = false
  let disposed = false
  let debounceTimer: unknown
  let retryTimer: unknown
  let retryCount = 0
  let queue: Promise<unknown> = Promise.resolve()

  // ── Small helpers ─────────────────────────────────────────────────

  const read = (): SyncRecord => deps.storage.get() ?? emptySyncRecord()
  const write = (patch: Partial<SyncRecord>): SyncRecord => {
    const next = { ...read(), ...patch }
    deps.storage.set(next)
    return next
  }
  /** Sync is on unless the record says it was turned off. */
  const enabled = (): boolean => read().syncEnabled !== false

  function emit(event: SyncEvent) {
    try {
      deps.onEvent(event)
    } catch {
      // a listener's failure must not break the sync that already happened
    }
  }

  function setStatus(next: SyncStatus) {
    if (status === next) return
    status = next
    emit({ type: 'status', status })
  }

  /** Run `fn` after everything queued before it, so two operations never overlap. */
  function exclusive<T>(fn: () => Promise<T>): Promise<T> {
    const run = queue.then(fn, fn)
    queue = run.catch(() => undefined)
    return run
  }

  function cancelDebounce() {
    if (debounceTimer !== undefined) deps.clock.clearTimeout(debounceTimer)
    debounceTimer = undefined
  }

  function cancelRetry() {
    if (retryTimer !== undefined) deps.clock.clearTimeout(retryTimer)
    retryTimer = undefined
  }

  /** (Re)start the quiet period: the push runs `debounceMs` after the last change. */
  function scheduleDebounce() {
    cancelDebounce()
    if (disposed || account === null || conflict || blocked) return
    debounceTimer = deps.clock.setTimeout(() => {
      debounceTimer = undefined
      void exclusive(pushOp)
    }, opts.debounceMs)
  }

  /**
   * Turn sync off here: unbind this browser and stop everything. Local data is untouched. Used by
   * `disable()`, by "turn off" at a conflict and after the cloud copy is deleted.
   */
  function turnOff() {
    cancelDebounce()
    cancelRetry()
    retryCount = 0
    conflict = null
    error = null
    blocked = false
    write({ boundStorageId: null, lastSyncedEtag: null, dirty: false, syncEnabled: false })
    setStatus('off')
  }

  /** Whether an operation may run now; sets the status when it may not. */
  function ready(): boolean {
    if (disposed || account === null) return false
    if (!enabled()) {
      setStatus('off')
      return false
    }
    if (!deps.isOwner()) {
      setStatus('not-owner')
      return false
    }
    return true
  }

  function ensureOwner() {
    if (!deps.isOwner()) throw new LostOwnership()
  }

  function succeed(action: Exclude<SyncAction, 'prompt'>, etag: string | null) {
    cancelRetry()
    retryCount = 0
    error = null
    conflict = null
    setStatus('synced')
    emit({ type: 'synced', action, etag, at: deps.clock.now() })
  }

  // ── The snapshot API ──────────────────────────────────────────────

  function failForStatus(res: Response): SyncFailure {
    const code = res.status
    switch (code) {
      case 413:
        return fail('too_large', 'The database is larger than the sync service accepts.', { status: code })
      case 422:
        return fail('invalid_snapshot', 'The sync service rejected the database file as invalid.', { status: code })
      case 428:
        return fail('precondition_required', 'The sync request was missing its precondition.', { status: code })
      case 429: {
        const waitMs = parseRetryAfter(res.headers.get('Retry-After'), deps.clock.now())
        return fail(
          'rate_limited',
          'The sync service is rate limiting this account; trying again shortly.',
          { status: code },
          waitMs === null ? 'backoff' : { afterMs: waitMs },
        )
      }
      default:
        if (code >= 500) return fail('server', `The sync service answered ${code}.`, { status: code }, 'backoff')
        return fail('unexpected_status', `The sync service answered ${code}.`, { status: code })
    }
  }

  async function request(method: 'HEAD' | 'GET' | 'PUT' | 'DELETE', headers: Record<string, string> = {}, body?: Uint8Array) {
    let res: Response
    try {
      res = await deps.fetch(url, {
        method,
        headers,
        body: body as BodyInit | undefined,
        credentials: 'include',
        cache: 'no-store',
      })
    } catch {
      throw fail('network', 'Could not reach the sync service.', {}, 'backoff')
    }
    if (res.status === 401) throw fail('unauthorized', 'Signed out.', { status: 401 })
    return res
  }

  async function headRemote(): Promise<RemoteState> {
    const res = await request('HEAD')
    if (res.status === 404) return { kind: 'none' }
    if (res.status !== 200) throw failForStatus(res)
    const etag = res.headers.get('ETag')
    if (!etag) throw fail('unexpected_status', 'The cloud copy came back without an ETag.', { status: 200 })
    return {
      kind: 'exists',
      etag,
      schemaHead: res.headers.get('X-Forge-Schema-Head'),
      bundleVersion: res.headers.get('X-Forge-Bundle-Version'),
      lastModified: res.headers.get('Last-Modified'),
    }
  }

  // ── Pull and push ─────────────────────────────────────────────────

  /** Refuse a cloud copy whose schema is past what this bundle knows. */
  function checkImportable(schemaHead: string | null) {
    const known = deps.runtime.knownHead()
    if (!known) throw fail('runtime_unavailable', 'The database is not open yet.')
    if (schemaHead && schemaHead > known) {
      throw fail(
        'schema_too_new',
        `The cloud copy was written by a newer version of Forge (schema ${schemaHead}; this version knows up to ${known}). Reload the page to update, then sync again.`,
      )
    }
  }

  /**
   * Replace this browser's database with the cloud copy and bind to `storageId`. `force` is the
   * user's "use the cloud copy": it replaces local changes made since the check; without it, an
   * edit made while the copy downloads sends the sync back through the table instead.
   */
  async function pull(remote: RemoteSnapshot, storageId: string, force: boolean): Promise<'done' | 'reenter'> {
    checkImportable(remote.schemaHead)
    const seq = changeSeq
    const res = await request('GET')
    if (res.status === 404) return 'reenter'
    if (res.status !== 200) throw failForStatus(res)
    // The ETag of the bytes in hand, which can be newer than the HEAD that led here.
    const etag = res.headers.get('ETag') ?? remote.etag
    checkImportable(res.headers.get('X-Forge-Schema-Head') ?? remote.schemaHead)
    const bytes = new Uint8Array(await res.arrayBuffer())

    ensureOwner()
    if (!force && changeSeq !== seq) return 'reenter'
    importing = true
    try {
      await deps.runtime.importDatabase(bytes)
    } catch (e) {
      if ((e as { code?: unknown } | null)?.code === 'schema_too_new') {
        throw fail('schema_too_new', message(e))
      }
      throw fail('import_failed', `Could not import the cloud copy: ${message(e)}`)
    } finally {
      importing = false
    }

    cancelDebounce()
    write({ boundStorageId: storageId, lastSyncedEtag: etag, dirty: false, lastSyncAt: deps.clock.now() })
    succeed('pull', etag)
    emit({ type: 'reload' })
    return 'done'
  }

  /**
   * Upload this browser's database and bind to `storageId`. `ifMatch` is the ETag of the cloud
   * copy to replace, or null to create it (`If-None-Match: *`). A 412 means the cloud copy is
   * not what the caller thought: the sync goes back through the table.
   */
  async function push(ifMatch: string | null, storageId: string): Promise<'done' | 'reenter'> {
    const seq = changeSeq
    let exported: ExportResult
    try {
      exported = await deps.runtime.exportDatabase()
    } catch (e) {
      throw fail('runtime_unavailable', `Could not export the database: ${message(e)}`)
    }
    const { bytes, schema_head: schemaHead, bundle_version: bundleVersion } = exported

    // Check the file before it leaves the browser: the service would reject it, and a file that
    // is not what it should be is worth knowing about here.
    if (!hasSqliteHeader(bytes) || bytes.length < 20) {
      throw fail('invalid_export', 'The exported database is not a SQLite file, so nothing was uploaded.')
    }
    if (bytes[18] !== 1 || bytes[19] !== 1) {
      throw fail(
        'journal_mode',
        `The exported database is not in rollback-journal mode (header bytes 18-19 are ${bytes[18]},${bytes[19]}, not 1,1), so nothing was uploaded.`,
      )
    }
    if (!schemaHead) {
      throw fail('invalid_export', 'The exported database does not say which schema it has, so nothing was uploaded.')
    }

    ensureOwner()
    const res = await request(
      'PUT',
      {
        'Content-Type': 'application/vnd.sqlite3',
        'X-Forge-Schema-Head': schemaHead,
        'X-Forge-Bundle-Version': bundleVersion,
        ...(ifMatch === null ? { 'If-None-Match': '*' } : { 'If-Match': ifMatch }),
      },
      bytes,
    )
    if (res.status === 412) return 'reenter'
    if (res.status !== 200) throw failForStatus(res)

    let etag = res.headers.get('ETag')
    if (!etag) {
      const body = (await res.json().catch(() => null)) as { etag?: unknown } | null
      etag = typeof body?.etag === 'string' ? body.etag : null
    }
    if (!etag) throw fail('unexpected_status', 'The sync service stored the database but sent no ETag.', { status: 200 })

    // Changes made while this ran may or may not be in the file: stay dirty and push again.
    const movedOn = changeSeq !== seq
    write({ boundStorageId: storageId, lastSyncedEtag: etag, dirty: movedOn, lastSyncAt: deps.clock.now() })
    succeed(ifMatch === null ? 'first-upload' : 'push', etag)
    if (movedOn) scheduleDebounce()
    return 'done'
  }

  // ── The decision table, applied ───────────────────────────────────

  async function execute(
    decision: Decision,
    remote: RemoteState,
    storageId: string,
    record: SyncRecord,
  ): Promise<'done' | 'reenter'> {
    switch (decision.action) {
      case 'first-upload':
        return push(null, storageId)
      case 'bind':
        write({ boundStorageId: storageId, lastSyncedEtag: null, lastSyncAt: deps.clock.now() })
        succeed('bind', null)
        return 'done'
      case 'pull':
        return pull(remote as RemoteSnapshot, storageId, false)
      case 'push':
        return push((remote as RemoteSnapshot).etag, storageId)
      case 'noop':
        write({ lastSyncAt: deps.clock.now() })
        succeed('noop', record.lastSyncedEtag)
        return 'done'
      case 'prompt':
        conflict = { reason: decision.reason, storageId, boundStorageId: record.boundStorageId, remote }
        cancelDebounce()
        setStatus('conflict')
        emit({ type: 'conflict', conflict })
        return 'done'
    }
  }

  /** Check the cloud copy and apply the table, again if a 412 or a vanished object says to. */
  async function runSync(): Promise<void> {
    if (!ready()) return
    const storageId = account as string
    conflict = null
    for (let pass = 0; pass <= opts.maxReentries; pass++) {
      setStatus('syncing')
      const remote = await headRemote()
      const record = read()
      // Only a missing cloud copy makes the answer depend on what is here.
      const localHasData = remote.kind === 'none' && (await deps.localHasData())
      const decision = decide({ remote, storageId, record, localHasData })
      if ((await execute(decision, remote, storageId, record)) === 'done') return
    }
    throw fail('conflict_loop', 'The cloud copy kept changing while syncing. Try again in a moment.')
  }

  /** The debounced push (and `flush`): push if there is something to push, else nothing. */
  async function runPush(): Promise<void> {
    if (!ready() || conflict || blocked) return
    const record = read()
    if (!record.dirty) return
    const storageId = account as string
    if (record.boundStorageId === storageId && record.lastSyncedEtag !== null) {
      setStatus('syncing')
      if ((await push(record.lastSyncedEtag, storageId)) === 'done') return
    }
    // Not bound yet, or the cloud copy is not what we last saw (a 412): the table decides.
    await runSync()
  }

  /**
   * Delete everything stored for the account, then turn sync off here. It runs even when sync is
   * already off (someone who turned it off can still want the cloud copy gone) and does not need to
   * be the owner tab: it touches the network and the sync record, not the database.
   */
  async function runDeleteCloudCopy(): Promise<void> {
    if (disposed || account === null) return
    cancelDebounce()
    cancelRetry()
    setStatus('syncing')
    const res = await request('DELETE')
    // 204 is the contract; the service answers it whether or not anything existed.
    if (res.status !== 204 && res.status !== 200) throw failForStatus(res)
    turnOff()
    emit({ type: 'cloud-deleted' })
  }

  // ── Failures ──────────────────────────────────────────────────────

  function backoffMs(attempt: number): number {
    return Math.min(opts.backoffCapMs, opts.backoffBaseMs * 2 ** (attempt - 1))
  }

  function handleFailure(failure: SyncFailure, retryOp: (() => Promise<void>) | null) {
    const info = failure.error

    if (info.code === 'unauthorized') {
      // The session is gone. Stop everything; the sign-in flow starts the engine again.
      account = null
      conflict = null
      error = null
      blocked = false
      retryCount = 0
      cancelDebounce()
      cancelRetry()
      setStatus('signed-out')
      emit({ type: 'unauthorized' })
      return
    }

    let retryAt: number | undefined
    if (failure.retry === 'none') {
      blocked = true
    } else if (retryOp) {
      retryCount++
      if (retryCount <= opts.maxRetries) {
        const delay =
          failure.retry === 'backoff' ? backoffMs(retryCount) : Math.min(failure.retry.afterMs, opts.backoffCapMs)
        retryAt = deps.clock.now() + delay
        cancelRetry()
        retryTimer = deps.clock.setTimeout(() => {
          retryTimer = undefined
          void exclusive(retryOp)
        }, delay)
      } else {
        // Out of retries: wait for the next change, a manual sync or a flush.
        retryCount = 0
      }
    }

    error = retryAt === undefined ? info : { ...info, retryAt }
    setStatus('error')
    emit({ type: 'error', error })
  }

  /** Run `op`, turning its failures into state, events and (when worth it) a scheduled retry. */
  async function guarded(op: () => Promise<void>, retryOp: (() => Promise<void>) | null): Promise<void> {
    if (disposed) return
    try {
      await op()
    } catch (e) {
      if (e instanceof LostOwnership) {
        setStatus('not-owner')
        return
      }
      const failure =
        e instanceof SyncFailure ? e : new SyncFailure({ code: 'unexpected', message: message(e) })
      handleFailure(failure, retryOp)
    }
  }

  const syncOp = (): Promise<void> => guarded(runSync, syncOp)
  const pushOp = (): Promise<void> => guarded(runPush, pushOp)

  // ── The public surface ────────────────────────────────────────────

  return {
    state(): SyncState {
      return { status, conflict, error }
    },

    setAccount(storageId) {
      return exclusive(async () => {
        if (disposed) return
        cancelDebounce()
        cancelRetry()
        retryCount = 0
        conflict = null
        error = null
        blocked = false
        account = storageId
        if (storageId === null) {
          setStatus('signed-out')
          return
        }
        await syncOp()
      })
    },

    sync() {
      return exclusive(async () => {
        if (disposed) return
        blocked = false
        cancelRetry()
        retryCount = 0
        await syncOp()
      })
    },

    notifyChange() {
      // The change an import announces is the import itself, not an edit. With sync off there is
      // nothing to push, so nothing is tracked.
      if (disposed || importing || !enabled()) return
      changeSeq++
      if (!read().dirty) write({ dirty: true })
      scheduleDebounce()
    },

    flush() {
      cancelDebounce()
      return exclusive(async () => {
        cancelRetry()
        await pushOp()
      })
    },

    resolveConflict(choice: ConflictChoice) {
      return exclusive(async () => {
        const pending = conflict
        if (disposed || !pending || account === null) return
        if (choice === 'turn-off') {
          turnOff()
          return
        }
        if (choice === 'use-cloud' && pending.remote.kind === 'none') {
          throw new Error('There is no cloud copy to use; only "replace-cloud" applies to this conflict.')
        }
        blocked = false
        cancelRetry()
        await guarded(async () => {
          if (!ready()) return
          setStatus('syncing')
          const remote = await headRemote()
          let outcome: 'done' | 'reenter'
          if (choice === 'use-cloud') {
            outcome = remote.kind === 'exists' ? await pull(remote, pending.storageId, true) : 'reenter'
          } else if (remote.kind === 'exists' && !(pending.remote.kind === 'exists' && pending.remote.etag === remote.etag)) {
            // The cloud copy is not the one the user was shown: do not replace what they never saw.
            outcome = 'reenter'
          } else {
            outcome = await push(remote.kind === 'exists' ? remote.etag : null, pending.storageId)
          }
          if (outcome === 'reenter') await runSync()
        }, null)
      })
    },

    deleteCloudCopy() {
      return exclusive(async () => {
        // A failure keeps sync on and is reported; a delete that did not happen is not retried on
        // its own, the person asks again.
        await guarded(runDeleteCloudCopy, null)
      })
    },

    disable() {
      return exclusive(async () => {
        if (disposed) return
        turnOff()
      })
    },

    enable() {
      return exclusive(async () => {
        if (disposed) return
        write({ syncEnabled: true })
        blocked = false
        cancelRetry()
        retryCount = 0
        // Signed out: nothing to check yet. The next `setAccount` finds sync on and runs the table.
        if (account === null) return
        await syncOp()
      })
    },

    dispose() {
      disposed = true
      cancelDebounce()
      cancelRetry()
    },
  }
}
