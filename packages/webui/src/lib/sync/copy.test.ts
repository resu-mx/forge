import { describe, expect, test } from 'bun:test'
import { conflictCopy, errorMessage, lastSyncLabel, statusLabel, statusTone, type StatusInput } from './copy'
import type { ConflictReason, RemoteState, SyncConflict, SyncError, SyncErrorCode } from './types'

const NOW = 1_700_000_000_000
const none: RemoteState = { kind: 'none' }
const exists: RemoteState = { kind: 'exists', etag: '"v1"', schemaHead: '055', bundleVersion: '0.1.0', lastModified: null }

const conflict = (reason: ConflictReason, remote: RemoteState): SyncConflict => ({
  reason,
  storageId: 'a',
  boundStorageId: reason === 'unbound' ? null : 'b',
  remote,
})

describe('lastSyncLabel', () => {
  test('reads naturally at each scale', () => {
    expect(lastSyncLabel(null, NOW)).toBe('Not synced yet')
    expect(lastSyncLabel(NOW - 5_000, NOW)).toBe('just now')
    expect(lastSyncLabel(NOW - 120_000, NOW)).toBe('2 min ago')
    expect(lastSyncLabel(NOW - 3 * 3_600_000, NOW)).toBe('3 h ago')
    expect(lastSyncLabel(NOW - 86_400_000, NOW)).toBe('1 day ago')
    expect(lastSyncLabel(NOW - 3 * 86_400_000, NOW)).toBe('3 days ago')
  })

  test('after a week it gives the date, and a clock that ran backwards is "just now"', () => {
    expect(lastSyncLabel(NOW - 30 * 86_400_000, NOW)).toMatch(/^\d{4}-\d{2}-\d{2}$/)
    expect(lastSyncLabel(NOW + 60_000, NOW)).toBe('just now')
  })
})

const input = (over: Partial<StatusInput> = {}): StatusInput => ({
  available: true,
  signedIn: true,
  enabled: true,
  status: 'synced',
  lastSyncAt: NOW - 120_000,
  ...over,
})

describe('statusLabel and statusTone', () => {
  test('nothing where sync does not exist', () => {
    expect(statusLabel(input({ available: false }), NOW)).toBe('')
    expect(statusTone(input({ available: false }))).toBe('muted')
  })

  test('signed out invites a sign-in', () => {
    expect(statusLabel(input({ signedIn: false }), NOW)).toBe('Sign in to back up')
  })

  test('off wins over whatever status the engine last had', () => {
    for (const status of ['synced', 'syncing', 'error', 'conflict'] as const) {
      expect(statusLabel(input({ enabled: false, status }), NOW)).toBe('Cloud sync is off')
      expect(statusTone(input({ enabled: false, status }))).toBe('muted')
    }
    expect(statusLabel(input({ status: 'off' }), NOW)).toBe('Cloud sync is off')
  })

  test('each live status has a label and a tone', () => {
    expect(statusLabel(input({ status: 'syncing' }), NOW)).toBe('Syncing…')
    expect(statusLabel(input({ status: 'synced' }), NOW)).toBe('Synced 2 min ago')
    expect(statusLabel(input({ status: 'conflict' }), NOW)).toBe('Choose which copy to keep')
    expect(statusLabel(input({ status: 'error' }), NOW)).toBe('Sync problem')
    expect(statusLabel(input({ status: 'not-owner' }), NOW)).toBe('Open in another tab')
    expect(statusTone(input({ status: 'synced' }))).toBe('ok')
    expect(statusTone(input({ status: 'syncing' }))).toBe('busy')
    expect(statusTone(input({ status: 'conflict' }))).toBe('attention')
    expect(statusTone(input({ status: 'error' }))).toBe('attention')
  })
})

const labels = (c: SyncConflict) => conflictCopy(c).choices.map((x) => x.choice)

describe('conflictCopy', () => {
  test('every variant ends with turning sync off', () => {
    const cases: SyncConflict[] = [
      conflict('unbound', exists),
      conflict('diverged', exists),
      conflict('account-switch', exists),
      conflict('account-switch', none),
      conflict('cloud-deleted', none),
    ]
    for (const c of cases) expect(labels(c).at(-1)).toBe('turn-off')
  })

  test('use-cloud is only offered when there is a cloud copy to use', () => {
    expect(labels(conflict('unbound', exists))).toEqual(['use-cloud', 'replace-cloud', 'turn-off'])
    expect(labels(conflict('diverged', exists))).toEqual(['use-cloud', 'replace-cloud', 'turn-off'])
    expect(labels(conflict('account-switch', exists))).toEqual(['use-cloud', 'replace-cloud', 'turn-off'])
    expect(labels(conflict('account-switch', none))).toEqual(['replace-cloud', 'turn-off'])
    expect(labels(conflict('cloud-deleted', none))).toEqual(['replace-cloud', 'turn-off'])
  })

  test('a deleted cloud copy: upload again is the primary action and nothing is uploaded unasked', () => {
    const c = conflictCopy(conflict('cloud-deleted', none))
    expect(c.title).toBe('Your cloud copy was deleted')
    expect(c.body).toContain('nothing is uploaded unless you say so')
    expect(c.choices[0]).toMatchObject({ choice: 'replace-cloud', label: "Upload this browser's data again", tone: 'primary' })
  })

  test('with no cloud copy to replace, the wording does not claim to replace one', () => {
    const upload = conflictCopy(conflict('account-switch', none)).choices.find((x) => x.choice === 'replace-cloud')!
    expect(upload.label).toBe("Upload this browser's data")
    expect(upload.description).not.toContain('Replaces your cloud copy')
  })

  test('the descriptions say which copy is lost, and do not overstate what other devices do', () => {
    const c = conflictCopy(conflict('diverged', exists))
    expect(c.choices[0].description).toContain('changes are lost')
    const replace = c.choices[1].description
    expect(replace).toContain('Your other devices get it the next time they sync')
    expect(replace).not.toContain('ask')
  })

  test('the two-copy dialogs tell people to export first', () => {
    for (const reason of ['unbound', 'diverged', 'account-switch'] as const) {
      expect(conflictCopy(conflict(reason, exists)).body).toContain('xport')
    }
  })
})

describe('errorMessage', () => {
  const err = (code: SyncErrorCode, message = 'engine message'): SyncError => ({ code, message })

  test('a network error says the data is safe', () => {
    expect(errorMessage(err('network'))).toContain('safe in this browser')
  })

  test('errors that explain themselves use the engine message', () => {
    for (const code of ['schema_too_new', 'import_failed', 'conflict_loop'] as const) {
      expect(errorMessage(err(code, 'specific words'))).toBe('specific words')
    }
  })

  test('everything else is reassuring and does not leak internals', () => {
    for (const code of ['invalid_snapshot', 'journal_mode', 'unexpected', 'precondition_required'] as const) {
      const text = errorMessage(err(code, 'header bytes 18-19 are 2,2'))
      expect(text).toContain("isn't affected")
      expect(text).not.toContain('header bytes')
    }
  })

  test('too large says it is not being backed up', () => {
    expect(errorMessage(err('too_large'))).toContain("isn't being backed up")
  })
})
