/**
 * What the cloud-sync UI says, as pure functions so the wording is in one place and testable.
 * Nothing here touches the DOM or the network.
 */

import type { ConflictChoice, SyncConflict, SyncError, SyncStatus } from './types'

/** "just now", "5 min ago", "3 h ago", "2 days ago", or a date after a week. */
export function lastSyncLabel(at: number | null, now: number): string {
  if (at === null) return 'Not synced yet'
  const seconds = Math.max(0, Math.round((now - at) / 1000))
  if (seconds < 45) return 'just now'
  const minutes = Math.round(seconds / 60)
  if (minutes < 60) return `${minutes} min ago`
  const hours = Math.round(minutes / 60)
  if (hours < 24) return `${hours} h ago`
  const days = Math.round(hours / 24)
  if (days < 7) return `${days} ${days === 1 ? 'day' : 'days'} ago`
  return new Date(at).toISOString().slice(0, 10)
}

/** The slice of the sync view the status line reads. */
export interface StatusInput {
  available: boolean
  signedIn: boolean
  enabled: boolean
  status: SyncStatus
  lastSyncAt: number | null
}

/** A short status line for the sidebar and the settings card. Empty where sync does not exist. */
export function statusLabel(v: StatusInput, now: number): string {
  if (!v.available) return ''
  if (!v.signedIn) return 'Sign in to back up'
  if (!v.enabled || v.status === 'off') return 'Cloud sync is off'
  switch (v.status) {
    case 'syncing':
      return 'Syncing…'
    case 'synced':
      return `Synced ${lastSyncLabel(v.lastSyncAt, now)}`
    case 'conflict':
      return 'Choose which copy to keep'
    case 'error':
      return 'Sync problem'
    case 'not-owner':
      return 'Open in another tab'
    case 'signed-out':
      return 'Sign in to back up'
    case 'idle':
      return v.lastSyncAt === null ? 'Waiting to sync' : `Synced ${lastSyncLabel(v.lastSyncAt, now)}`
    default:
      return ''
  }
}

/** A tone for the indicator dot. */
export type StatusTone = 'ok' | 'busy' | 'attention' | 'muted'

export function statusTone(v: StatusInput): StatusTone {
  if (!v.available || !v.signedIn || !v.enabled || v.status === 'off') return 'muted'
  switch (v.status) {
    case 'synced':
      return 'ok'
    case 'syncing':
      return 'busy'
    case 'conflict':
    case 'error':
      return 'attention'
    default:
      return 'muted'
  }
}

/** One button of the conflict dialog. */
export interface ChoiceCopy {
  choice: ConflictChoice
  label: string
  /** One line saying what it does to which copy. */
  description: string
  tone: 'primary' | 'secondary' | 'ghost'
}

export interface ConflictCopy {
  title: string
  body: string
  choices: ChoiceCopy[]
}

const USE_CLOUD: ChoiceCopy = {
  choice: 'use-cloud',
  label: 'Use the cloud copy',
  description: "Replaces the data in this browser with your cloud copy. This browser's changes are lost.",
  tone: 'secondary',
}

const REPLACE_CLOUD: ChoiceCopy = {
  choice: 'replace-cloud',
  label: "Keep this browser's data",
  description: 'Replaces your cloud copy with the data in this browser. Your other devices get it the next time they sync.',
  tone: 'secondary',
}

const TURN_OFF: ChoiceCopy = {
  choice: 'turn-off',
  label: 'Turn cloud sync off here',
  description: 'Leaves both copies as they are. This browser keeps working on its own.',
  tone: 'ghost',
}

/**
 * The text of the conflict dialog for `conflict`. `use-cloud` is only offered when there is a cloud
 * copy to use. Every variant ends with turning sync off, and the dialog adds "export first".
 */
export function conflictCopy(conflict: SyncConflict): ConflictCopy {
  const cloudExists = conflict.remote.kind === 'exists'
  const choices = (...list: ChoiceCopy[]) => list.filter((c) => c.choice !== 'use-cloud' || cloudExists)

  switch (conflict.reason) {
    case 'unbound':
      return {
        title: 'Your account already has cloud data',
        body:
          "This browser has data of its own, and your account already has a cloud copy. They can't be merged, so one has to replace the other. If you might want this browser's data back, export it first.",
        choices: choices(USE_CLOUD, REPLACE_CLOUD, TURN_OFF),
      }
    case 'diverged':
      return {
        title: 'Your data changed in two places',
        body:
          "The cloud copy changed since this browser last synced, and this browser's data changed too. They can't be merged, so one has to replace the other. If you might want this browser's changes back, export first.",
        choices: choices(USE_CLOUD, REPLACE_CLOUD, TURN_OFF),
      }
    case 'account-switch':
      return {
        title: 'This browser belongs to another account',
        body: cloudExists
          ? "This browser's data was synced with a different account. Syncing it with this one means choosing which data to keep. Export first if you might want this browser's data."
          : "This browser's data was synced with a different account, and this account has no cloud data yet. Upload this browser's data to it, or leave this browser out of sync.",
        choices: choices(
          USE_CLOUD,
          cloudExists
            ? REPLACE_CLOUD
            : {
                ...REPLACE_CLOUD,
                label: "Upload this browser's data",
                description: "Creates this account's cloud copy from the data in this browser.",
              },
          TURN_OFF,
        ),
      }
    case 'cloud-deleted':
      return {
        title: 'Your cloud copy was deleted',
        body:
          'Your cloud copy no longer exists. It was deleted here or on another device. This browser still has your data, and nothing is uploaded unless you say so.',
        choices: [
          {
            choice: 'replace-cloud',
            label: "Upload this browser's data again",
            description: 'Creates a new cloud copy from the data in this browser.',
            tone: 'primary',
          },
          { ...TURN_OFF, label: 'Turn cloud sync off' },
        ],
      }
  }
}

/** A short sentence for a toast or the settings card. The engine's own message covers the rest. */
export function errorMessage(error: SyncError): string {
  switch (error.code) {
    case 'network':
      return "Can't reach the sync service. Your data is safe in this browser, and sync will try again."
    case 'server':
      return 'The sync service had a problem. Trying again shortly.'
    case 'rate_limited':
      return 'Syncing too often, so it is slowing down.'
    case 'too_large':
      return "Your database is larger than cloud sync accepts, so it isn't being backed up."
    case 'runtime_unavailable':
      return "The database isn't open yet. Sync will start when it is."
    case 'schema_too_new':
    case 'import_failed':
    case 'conflict_loop':
      return error.message
    default:
      return "Sync hit a problem and stopped. Your data in this browser isn't affected."
  }
}
