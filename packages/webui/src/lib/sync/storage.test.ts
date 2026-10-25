import { describe, expect, test } from 'bun:test'
import { createLocalStorageSyncStorage, emptySyncRecord, parseSyncRecord, type KeyValueStorage } from './storage'
import { SYNC_STORAGE_KEY } from './types'

function memoryStorage(initial: Record<string, string> = {}): KeyValueStorage & { data: Record<string, string> } {
  const data = { ...initial }
  return {
    data,
    getItem: (k) => (k in data ? data[k] : null),
    setItem: (k, v) => {
      data[k] = v
    },
  }
}

describe('parseSyncRecord', () => {
  test('anything that is not an object is "never synced"', () => {
    for (const raw of [null, undefined, 'x', 42, true]) expect(parseSyncRecord(raw)).toEqual(emptySyncRecord())
  })

  test('keeps a valid record as it is', () => {
    const record = { boundStorageId: 'a', lastSyncedEtag: '"v1"', dirty: true, lastSyncAt: 1_700_000_000_000 }
    expect(parseSyncRecord(record)).toEqual(record)
  })

  test('fills in missing or wrongly typed fields', () => {
    expect(parseSyncRecord({ boundStorageId: 7, lastSyncedEtag: '', dirty: 'yes', lastSyncAt: 'now' })).toEqual(
      emptySyncRecord(),
    )
    expect(parseSyncRecord({ boundStorageId: 'a' })).toEqual({ ...emptySyncRecord(), boundStorageId: 'a' })
  })

  test('drops fields it does not know', () => {
    expect(Object.keys(parseSyncRecord({ boundStorageId: 'a', extra: 1 })).sort()).toEqual([
      'boundStorageId',
      'dirty',
      'lastSyncAt',
      'lastSyncedEtag',
    ])
  })

  test('sync is on unless the record says false: absent and malformed values leave no field', () => {
    // Records written before the field existed stay valid, and a hand edit cannot switch sync off by accident.
    for (const syncEnabled of [undefined, true, null, 0, 'false', 'no']) {
      expect('syncEnabled' in parseSyncRecord({ boundStorageId: 'a', syncEnabled })).toBe(false)
    }
  })

  test('syncEnabled: false is kept: that is how "off" survives a reload', () => {
    expect(parseSyncRecord({ boundStorageId: null, syncEnabled: false })).toEqual({
      ...emptySyncRecord(),
      syncEnabled: false,
    })
  })
})

describe('createLocalStorageSyncStorage', () => {
  test('the key is forge.sync.v1', () => {
    expect(SYNC_STORAGE_KEY).toBe('forge.sync.v1')
  })

  test('writes and reads the record as JSON under that key', () => {
    const backing = memoryStorage()
    const storage = createLocalStorageSyncStorage(backing)
    expect(storage.get()).toBeNull()
    const record = { boundStorageId: 'a', lastSyncedEtag: '"v1"', dirty: false, lastSyncAt: 5 }
    storage.set(record)
    expect(JSON.parse(backing.data[SYNC_STORAGE_KEY])).toEqual(record)
    expect(storage.get()).toEqual(record)
  })

  test('an off record round-trips through storage', () => {
    const storage = createLocalStorageSyncStorage(memoryStorage())
    storage.set({ boundStorageId: null, lastSyncedEtag: null, dirty: false, lastSyncAt: null, syncEnabled: false })
    expect(storage.get()).toEqual({
      boundStorageId: null,
      lastSyncedEtag: null,
      dirty: false,
      lastSyncAt: null,
      syncEnabled: false,
    })
  })

  test('a corrupt stored value reads as nothing', () => {
    const storage = createLocalStorageSyncStorage(memoryStorage({ [SYNC_STORAGE_KEY]: '{not json' }))
    expect(storage.get()).toBeNull()
  })

  test('missing, blocked or full storage does not throw', () => {
    expect(createLocalStorageSyncStorage(undefined).get()).toBeNull()
    createLocalStorageSyncStorage(undefined).set(emptySyncRecord())
    const blocked: KeyValueStorage = {
      getItem: () => {
        throw new Error('SecurityError')
      },
      setItem: () => {
        throw new Error('QuotaExceededError')
      },
    }
    const storage = createLocalStorageSyncStorage(blocked)
    expect(storage.get()).toBeNull()
    expect(() => storage.set(emptySyncRecord())).not.toThrow()
  })
})
