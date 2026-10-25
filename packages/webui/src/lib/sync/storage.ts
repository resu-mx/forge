import { SYNC_STORAGE_KEY, type SyncRecord, type SyncStorage } from './types'

/** A browser that has never synced. */
export function emptySyncRecord(): SyncRecord {
  return { boundStorageId: null, lastSyncedEtag: null, dirty: false, lastSyncAt: null }
}

const stringOrNull = (v: unknown): string | null => (typeof v === 'string' && v !== '' ? v : null)

/**
 * Read a stored record defensively: whatever is there (nothing, another version's shape, hand
 * edits) becomes a valid record, with the fields it lacks at their "never synced" values.
 */
export function parseSyncRecord(raw: unknown): SyncRecord {
  if (!raw || typeof raw !== 'object') return emptySyncRecord()
  const r = raw as Record<string, unknown>
  return {
    boundStorageId: stringOrNull(r.boundStorageId),
    lastSyncedEtag: stringOrNull(r.lastSyncedEtag),
    dirty: r.dirty === true,
    lastSyncAt: typeof r.lastSyncAt === 'number' && Number.isFinite(r.lastSyncAt) ? r.lastSyncAt : null,
  }
}

/** The slice of `Storage` the adapter uses. */
export interface KeyValueStorage {
  getItem(key: string): string | null
  setItem(key: string, value: string): void
}

/**
 * The record in `localStorage` (or anything shaped like it) under `forge.sync.v1`. Storage can be
 * missing, full or blocked (a private window); a failed read is "never synced" and a failed write
 * is dropped, so sync falls back to checking the cloud copy at the next start.
 */
export function createLocalStorageSyncStorage(
  storage: KeyValueStorage | undefined,
  key: string = SYNC_STORAGE_KEY,
): SyncStorage {
  return {
    get() {
      try {
        const text = storage?.getItem(key)
        return text ? parseSyncRecord(JSON.parse(text)) : null
      } catch {
        return null
      }
    },
    set(record) {
      try {
        storage?.setItem(key, JSON.stringify(record))
      } catch {
        // quota or blocked storage: the record is a cache of what the cloud already says
      }
    },
  }
}
