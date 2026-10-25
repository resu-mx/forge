import { describe, expect, test } from 'bun:test'
import { decide } from './decide'
import type { DecideInput, RemoteState } from './types'

const A = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa'
const B = 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb'

const none: RemoteState = { kind: 'none' }
const exists = (etag: string): RemoteState => ({
  kind: 'exists',
  etag,
  schemaHead: '055_dataset_meta',
  bundleVersion: '0.1.0',
  lastModified: null,
})

function input(over: Partial<Omit<DecideInput, 'record'>> & { record?: Partial<DecideInput['record']> }): DecideInput {
  return {
    remote: none,
    storageId: A,
    localHasData: false,
    ...over,
    record: { boundStorageId: null, lastSyncedEtag: null, dirty: false, ...over.record },
  }
}

describe('decide: the contract table, row by row', () => {
  test('none + never bound + local has data → first upload', () => {
    expect(decide(input({ remote: none, localHasData: true }))).toEqual({ action: 'first-upload' })
  })

  test('none + never bound + local empty → bind', () => {
    expect(decide(input({ remote: none, localHasData: false }))).toEqual({ action: 'bind' })
  })

  test('exists + never bound → prompt, whatever is local', () => {
    for (const localHasData of [true, false]) {
      for (const dirty of [true, false]) {
        expect(decide(input({ remote: exists('"v1"'), localHasData, record: { dirty } }))).toEqual({
          action: 'prompt',
          reason: 'unbound',
        })
      }
    }
  })

  test('exists + bound to a different account → prompt, whatever the ETags say', () => {
    const record = { boundStorageId: B, lastSyncedEtag: '"v1"' }
    for (const dirty of [true, false]) {
      expect(decide(input({ remote: exists('"v1"'), record: { ...record, dirty } }))).toEqual({
        action: 'prompt',
        reason: 'account-switch',
      })
      expect(decide(input({ remote: exists('"v2"'), record: { ...record, dirty } }))).toEqual({
        action: 'prompt',
        reason: 'account-switch',
      })
    }
  })

  test('ETag equals last synced + dirty → push', () => {
    const record = { boundStorageId: A, lastSyncedEtag: '"v1"', dirty: true }
    expect(decide(input({ remote: exists('"v1"'), record }))).toEqual({ action: 'push' })
  })

  test('ETag differs from last synced + clean → pull', () => {
    const record = { boundStorageId: A, lastSyncedEtag: '"v1"', dirty: false }
    expect(decide(input({ remote: exists('"v2"'), record }))).toEqual({ action: 'pull' })
  })

  test('ETag differs from last synced + dirty → prompt', () => {
    const record = { boundStorageId: A, lastSyncedEtag: '"v1"', dirty: true }
    expect(decide(input({ remote: exists('"v2"'), record }))).toEqual({ action: 'prompt', reason: 'diverged' })
  })
})

describe('decide: the cells the table leaves open', () => {
  test('ETag equals last synced + clean → nothing to do', () => {
    const record = { boundStorageId: A, lastSyncedEtag: '"v1"', dirty: false }
    expect(decide(input({ remote: exists('"v1"'), record }))).toEqual({ action: 'noop' })
  })

  test('a different account always prompts, even when the cloud holds nothing', () => {
    const record = { boundStorageId: B, lastSyncedEtag: '"v1"', dirty: false }
    for (const localHasData of [true, false]) {
      expect(decide(input({ remote: none, localHasData, record }))).toEqual({
        action: 'prompt',
        reason: 'account-switch',
      })
    }
  })

  test('the cloud copy is gone while bound to this account and this browser holds data → prompt, never a silent upload', () => {
    for (const dirty of [true, false]) {
      const record = { boundStorageId: A, lastSyncedEtag: '"v1"', dirty }
      expect(decide(input({ remote: none, localHasData: true, record }))).toEqual({
        action: 'prompt',
        reason: 'cloud-deleted',
      })
    }
  })

  test('the cloud copy is gone while bound and nothing is held here → bind, nothing to ask about', () => {
    const record = { boundStorageId: A, lastSyncedEtag: '"v1"', dirty: false }
    expect(decide(input({ remote: none, localHasData: false, record }))).toEqual({ action: 'bind' })
  })

  test('bound to an empty cloud (no copy was ever synced) is the ordinary first upload, not a deletion', () => {
    const record = { boundStorageId: A, lastSyncedEtag: null, dirty: true }
    expect(decide(input({ remote: none, localHasData: true, record }))).toEqual({ action: 'first-upload' })
    expect(decide(input({ remote: none, localHasData: false, record }))).toEqual({ action: 'bind' })
  })

  test('a different account wins over a deleted copy: the account switch is asked first', () => {
    const record = { boundStorageId: B, lastSyncedEtag: '"v1"', dirty: false }
    expect(decide(input({ remote: none, localHasData: true, record }))).toEqual({
      action: 'prompt',
      reason: 'account-switch',
    })
  })

  test('bound without an ETag (bound to an empty cloud) pulls once a cloud copy appears', () => {
    const record = { boundStorageId: A, lastSyncedEtag: null, dirty: false }
    expect(decide(input({ remote: exists('"v1"'), record }))).toEqual({ action: 'pull' })
    expect(decide(input({ remote: exists('"v1"'), record: { ...record, dirty: true } }))).toEqual({
      action: 'prompt',
      reason: 'diverged',
    })
  })
})
