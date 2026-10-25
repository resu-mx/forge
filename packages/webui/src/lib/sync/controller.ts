/**
 * Connects the sync engine to everything around it: the session (which account, if any), the
 * runtime (is this tab the owner of the database), data changes, the page being hidden, toasts and
 * the reload a pull asks for. It exposes one `view` for the UI.
 *
 * It has no Svelte and no browser globals in it: everything it touches is injected through
 * `SyncControllerDeps`, so `bun test` drives it with fakes. `stores/sync.svelte.ts` holds the view
 * in `$state` and attaches the DOM listeners.
 *
 * Where cloud sync cannot run (`available` is false: a preview host, API mode) it is inert: no
 * engine exists, nothing is requested, and every method does nothing.
 */

import { errorMessage } from './copy'
import { createSyncEngine } from './engine'
import type {
  ConflictChoice,
  SyncClock,
  SyncConflict,
  SyncEngine,
  SyncError,
  SyncEvent,
  SyncFetch,
  SyncOptions,
  SyncRuntime,
  SyncStatus,
  SyncStorage,
} from './types'

/** What the UI renders from. */
export interface SyncViewState {
  /** Cloud sync exists on this origin at all. When false, render nothing. */
  available: boolean
  /** An account is set, so there is something to sync with. */
  signedIn: boolean
  /** Sync is not turned off in this browser. */
  enabled: boolean
  status: SyncStatus
  conflict: SyncConflict | null
  error: SyncError | null
  /** When the last successful sync check finished, in ms since the epoch. */
  lastSyncAt: number | null
  /** A user-started action (sync now, delete, a conflict choice) is in flight. */
  busy: boolean
}

export interface SyncControllerDeps {
  /** Cloud sync can run on this origin (and in this mode). */
  available: boolean
  baseUrl: string
  fetch: SyncFetch
  runtime: SyncRuntime
  storage: SyncStorage
  clock: SyncClock
  /** True in the tab that owns the database and has it open. */
  isOwner(): boolean
  localHasData(): Promise<boolean>
  notify(opts: { message: string; type?: 'error' | 'info' | 'success'; duration?: number }): void
  /** Reload the page: a pull replaced the database underneath it. */
  reload(): void
  /** The session ended: ask the account service who is signed in now. */
  refreshSession(): void | Promise<void>
  /** Download this browser's database (offered before anything is replaced). */
  exportBackup(): Promise<unknown>
  /** Called with the new view after every change to it. */
  onChange(view: SyncViewState): void
  options?: Partial<SyncOptions>
}

export interface SyncController {
  get(): SyncViewState
  /** Set the signed-in account's `storageId`, or null when signed out. Repeat calls with the same value do nothing. */
  setAccount(storageId: string | null): void
  /** The runtime's state changed: this tab may have just become the database owner. */
  runtimeChanged(): void
  /** The database changed (`forge:changed`). */
  notifyChange(): void
  /** The page is being hidden: push now if there is anything to push. */
  pageHidden(): void
  syncNow(): Promise<void>
  turnOn(): Promise<void>
  turnOff(): Promise<void>
  deleteCloudCopy(): Promise<void>
  resolve(choice: ConflictChoice): Promise<void>
  exportBackup(): Promise<void>
  dispose(): void
}

/** The same error is toasted at most this often. The settings card and the sidebar keep showing it. */
const ERROR_TOAST_QUIET_MS = 60_000

export function createSyncController(deps: SyncControllerDeps): SyncController {
  let account: string | null | undefined // undefined until the session has answered
  let busy = 0
  let lastErrorToast: { code: string; at: number } | null = null

  const engine: SyncEngine | null = deps.available
    ? createSyncEngine({
        baseUrl: deps.baseUrl,
        fetch: deps.fetch,
        runtime: deps.runtime,
        storage: deps.storage,
        clock: deps.clock,
        isOwner: deps.isOwner,
        localHasData: deps.localHasData,
        onEvent,
        options: deps.options,
      })
    : null

  const enabled = () => deps.storage.get()?.syncEnabled !== false
  const lastSyncAt = () => deps.storage.get()?.lastSyncAt ?? null

  let view: SyncViewState = {
    available: deps.available,
    signedIn: false,
    enabled: enabled(),
    status: 'idle',
    conflict: null,
    error: null,
    lastSyncAt: lastSyncAt(),
    busy: false,
  }

  function publish() {
    const state = engine?.state()
    view = {
      available: deps.available,
      signedIn: typeof account === 'string',
      enabled: enabled(),
      status: state?.status ?? 'idle',
      conflict: state?.conflict ?? null,
      error: state?.error ?? null,
      lastSyncAt: lastSyncAt(),
      busy: busy > 0,
    }
    deps.onChange(view)
  }

  function toastError(error: SyncError) {
    const now = deps.clock.now()
    if (lastErrorToast && lastErrorToast.code === error.code && now - lastErrorToast.at < ERROR_TOAST_QUIET_MS) return
    lastErrorToast = { code: error.code, at: now }
    deps.notify({ message: errorMessage(error), type: 'error' })
  }

  function onEvent(event: SyncEvent) {
    switch (event.type) {
      case 'reload':
        deps.reload()
        break
      case 'cloud-deleted':
        deps.notify({
          message: 'Your cloud copy was deleted. Cloud sync is off in this browser, and the data here is untouched.',
          type: 'success',
        })
        break
      case 'unauthorized':
        // The engine stopped and forgot its account. Forget ours too, so the session's next answer
        // ("still signed in", or a new account) starts it again instead of being skipped as a repeat.
        account = undefined
        void deps.refreshSession()
        break
      case 'error':
        toastError(event.error)
        break
      default:
        break
    }
    publish()
  }

  /** Run a user-started action: the view says `busy` until it finishes. */
  function track(run: Promise<unknown>): Promise<void> {
    busy++
    publish()
    return run
      .then(() => undefined)
      .finally(() => {
        busy--
        publish()
      })
  }

  return {
    get: () => view,

    setAccount(storageId) {
      if (!engine || storageId === account) return
      account = storageId
      publish()
      void engine.setAccount(storageId).then(publish)
    },

    runtimeChanged() {
      if (!engine || typeof account !== 'string') return
      // Another tab held the database and has let go: start with what we skipped.
      if (view.status === 'not-owner' && deps.isOwner()) void engine.sync().then(publish)
    },

    notifyChange() {
      engine?.notifyChange()
    },

    pageHidden() {
      if (engine) void engine.flush().then(publish)
    },

    syncNow() {
      return engine ? track(engine.sync()) : Promise.resolve()
    },

    turnOn() {
      return engine ? track(engine.enable()) : Promise.resolve()
    },

    turnOff() {
      return engine ? track(engine.disable()) : Promise.resolve()
    },

    deleteCloudCopy() {
      return engine ? track(engine.deleteCloudCopy()) : Promise.resolve()
    },

    resolve(choice) {
      if (!engine) return Promise.resolve()
      return track(
        engine.resolveConflict(choice).catch((e: unknown) => {
          deps.notify({ message: e instanceof Error ? e.message : String(e), type: 'error' })
        }),
      )
    },

    async exportBackup() {
      try {
        await deps.exportBackup()
      } catch (e) {
        deps.notify({ message: `Export failed: ${e instanceof Error ? e.message : String(e)}`, type: 'error' })
      }
    },

    dispose() {
      engine?.dispose()
    },
  }
}
