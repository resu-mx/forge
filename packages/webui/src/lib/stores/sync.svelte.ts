import { FORGE_CHANGED_EVENT } from '@forge/runtime'
import { downloadBackup } from '$lib/backup'
import { cloudSyncOn, dataUrl, syncAvailable } from '$lib/config'
import { forge, forgeMode, runtime } from '$lib/sdk'
import { createSyncController, type SyncViewState } from '$lib/sync/controller'
import { entityCounts, hasLocalData } from '$lib/sync/local-data'
import { createLocalStorageSyncStorage } from '$lib/sync/storage'
import type { ConflictChoice } from '$lib/sync/types'
import { refreshSession } from '$lib/stores/session.svelte'
import { addToast } from '$lib/stores/toast.svelte'

// Cloud sync for a signed-in person: their database, backed up to the snapshot service and restored
// on another device. The logic is `$lib/sync/controller` (no Svelte, tested with fakes); this file
// holds its view in `$state` and attaches the page-level listeners.
//
// It exists only when the build has it switched on (`VITE_CLOUD_SYNC=on`), in `wasm` mode, on a host
// that can carry the `.resu.mx` session cookie (see `$lib/config`). Anywhere else `available` is
// false, no engine is created, nothing is requested, and the UI renders nothing.
const available =
  typeof window !== 'undefined' && cloudSyncOn && forgeMode === 'wasm' && runtime !== null && syncAvailable()

// `$state.raw`: the whole view is replaced on every change, and `view.conflict` must keep its
// identity across replacements so "was this conflict dismissed?" can compare it. A deep `$state`
// would wrap it in a new proxy each time.
let view = $state.raw<SyncViewState>({
  available,
  signedIn: false,
  enabled: true,
  status: 'idle',
  conflict: null,
  error: null,
  lastSyncAt: null,
  busy: false,
})

const controller = createSyncController({
  available,
  baseUrl: dataUrl,
  // Looked up per call, so a test double installed on `window` (or Playwright's routing) is honoured.
  fetch: (url, init) => window.fetch(url, init),
  runtime: {
    exportDatabase: () => runtime!.exportDatabase(),
    importDatabase: (bytes) => runtime!.importDatabase(bytes),
    knownHead: () => {
      const status = runtime!.status()
      return status.state === 'ready' ? status.info.known_head : undefined
    },
  },
  storage: createLocalStorageSyncStorage(typeof localStorage === 'undefined' ? undefined : localStorage),
  clock: {
    now: () => Date.now(),
    setTimeout: (fn, ms) => window.setTimeout(fn, ms),
    clearTimeout: (handle) => window.clearTimeout(handle as number),
  },
  // Only the tab that owns the database (the Web Lock holder, with it open) syncs.
  isOwner: () => runtime!.status().state === 'ready',
  localHasData: () => hasLocalData(entityCounts(forge)),
  notify: addToast,
  reload: () => window.location.reload(),
  refreshSession,
  exportBackup: () => downloadBackup(runtime!),
  onChange: (next) => {
    view = next
  },
})

if (available) {
  // A data change (including one an agent made through `window.forge`). The engine debounces.
  window.addEventListener(FORGE_CHANGED_EVENT, () => controller.notifyChange())
  // Push before the page goes away. `visibilitychange` is the reliable one on mobile; `pagehide` covers the rest.
  document.addEventListener('visibilitychange', () => {
    if (document.visibilityState === 'hidden') controller.pageHidden()
  })
  window.addEventListener('pagehide', () => controller.pageHidden())
  // Another tab let go of the database, or this one just opened it.
  runtime!.onStatus(() => controller.runtimeChanged())
}

/** The current sync view: whether it exists here, the status, any conflict or error. */
export function getSyncView(): SyncViewState {
  return view
}

// The conflict dialog can be put away ("Decide later") without answering. The conflict stays
// pending and the sidebar says so; this remembers which one was put away, so a different conflict
// (a new prompt) opens the dialog again.
let dismissed = $state.raw<SyncViewState['conflict']>(null)

/** Whether the person put away the conflict that is pending now. */
export function isConflictDismissed(): boolean {
  return view.conflict !== null && dismissed === view.conflict
}

export function dismissConflict(): void {
  dismissed = view.conflict
}

export function reopenConflict(): void {
  dismissed = null
}

/**
 * Tell sync who is signed in: the account's `storageId`, or null when signed out. Call it whenever
 * the session settles; repeats with the same value do nothing.
 */
export function setSyncAccount(storageId: string | null): void {
  controller.setAccount(storageId)
}

export const syncNow = () => controller.syncNow()
export const turnSyncOn = () => controller.turnOn()
export const turnSyncOff = () => controller.turnOff()
export const deleteCloudCopy = () => controller.deleteCloudCopy()
export const resolveSyncConflict = (choice: ConflictChoice) => controller.resolve(choice)
export const exportBeforeResolving = () => controller.exportBackup()
