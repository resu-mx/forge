import type { DecideInput, Decision } from './types'

/**
 * The "Client behaviour" table of the snapshot API contract, as one pure function.
 *
 * | Remote | This browser | Action |
 * |---|---|---|
 * | none | never bound, local has data | first upload with `If-None-Match: *`, then bind |
 * | none | never bound, local empty | bind; nothing to do |
 * | exists | never bound, or bound to a different `storageId` | prompt |
 * | ETag equals last synced | dirty | push with `If-Match` |
 * | ETag differs from last synced | clean | pull, `importDatabase`, reload |
 * | ETag differs from last synced | dirty | prompt |
 *
 * A different account always prompts, whatever the cloud holds, so one account's data is never
 * merged into another's without a choice (the local database belongs to the origin, not the user).
 *
 * Two cells the table leaves open:
 * - Equal ETag and clean is a `noop`.
 * - No cloud copy while this browser is bound to this account (it was deleted, from here or from
 *   another device) is decided like "never bound": upload what is here, or bind if there is nothing.
 */
export function decide({ remote, storageId, record, localHasData }: DecideInput): Decision {
  const { boundStorageId, lastSyncedEtag, dirty } = record

  if (boundStorageId !== null && boundStorageId !== storageId) {
    return { action: 'prompt', reason: 'account-switch' }
  }

  if (remote.kind === 'none') {
    return { action: localHasData ? 'first-upload' : 'bind' }
  }

  if (boundStorageId === null) {
    return { action: 'prompt', reason: 'unbound' }
  }

  if (remote.etag === lastSyncedEtag) {
    return { action: dirty ? 'push' : 'noop' }
  }
  return dirty ? { action: 'prompt', reason: 'diverged' } : { action: 'pull' }
}
