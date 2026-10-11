/**
 * Post-pass units against a small database built from the real migrations.
 */

import type { Database } from 'bun:sqlite'
import { afterEach, describe, expect, test } from 'bun:test'
import { existsSync, readdirSync } from 'fs'
import { join } from 'path'
import type { LedgerEntry } from '../ledger'
import { backdate } from '../postpass/backdate'
import { compact, fileFacts, verifyCompacted } from '../postpass/compact'
import { contentFingerprint, naturalKeyMap } from '../postpass/fingerprint'
import { INVARIANTS, type InvariantContext } from '../postpass/invariants'
import { META_KEYS, type DatasetMeta, readDatasetMeta, schemaHead, writeDatasetMeta } from '../postpass/meta'
import { applyNoteReferences, applyOverlay } from '../postpass/overlay'
import { PERSONAS } from '../personas'
import { MIGRATIONS_DIR, migratedDb, tempDir, uuid } from './helpers'

const EPOCH = '2025-08-01T12:00:00Z'
const cleanups: (() => void)[] = []
afterEach(() => {
  while (cleanups.length) cleanups.pop()?.()
})

interface Fixture {
  path: string
  db: Database
  profileId: string
  sourceId: string
  jdId: string
  /** Ledger entries covering every non-rule timestamp in the fixture. */
  entries: LedgerEntry[]
}

function fixture(): Fixture {
  const t = tempDir('postpass')
  const path = join(t.dir, 'data.sqlite')
  const db = migratedDb(path)
  cleanups.push(() => {
    try {
      db.close()
    } catch {
      // already closed
    }
    t.cleanup()
  })
  const profileId = (db.query('SELECT id FROM user_profile').get() as { id: string }).id
  const sourceId = uuid()
  const jdId = uuid()
  db.run("INSERT INTO sources (id, title, description) VALUES (?, 'A source', 'What happened.')", [sourceId])
  db.run("INSERT INTO job_descriptions (id, title, raw_text) VALUES (?, 'A job', 'Job text.')", [jdId])
  const entries: LedgerEntry[] = [
    { kind: 'profile', key: 'profile', table: 'user_profile', id: profileId, times: { created_at: EPOCH, updated_at: EPOCH } },
    { kind: 'source', key: 's', table: 'sources', id: sourceId, times: { created_at: '2025-09-01T15:00:00Z', updated_at: '2025-09-02T15:00:00Z' } },
    { kind: 'jd', key: 'j', table: 'job_descriptions', id: jdId, times: { created_at: '2026-01-01T15:00:00Z', updated_at: '2026-02-01T15:00:00Z' } },
  ]
  return { path, db, profileId, sourceId, jdId, entries }
}

const get = <T>(db: Database, sql: string, ...params: string[]) => db.query(sql).get(...params) as T

describe('backdate', () => {
  test('writes ledger times and rule times, keeping each column format', () => {
    const f = fixture()
    f.db.transaction(() => backdate(f.db, f.entries, EPOCH))()
    expect(get<{ c: string; u: string }>(f.db, 'SELECT created_at c, updated_at u FROM sources WHERE id = ?', f.sourceId)).toEqual({
      c: '2025-09-01T15:00:00Z',
      u: '2025-09-02T15:00:00Z',
    })
    // The trigger would have overwritten this with the wall clock had it not been dropped.
    expect(get<{ u: string }>(f.db, 'SELECT updated_at u FROM job_descriptions WHERE id = ?', f.jdId).u).toBe('2026-02-01T15:00:00Z')
    expect(get<{ n: number }>(f.db, 'SELECT COUNT(*) n FROM archetypes WHERE created_at != ?', EPOCH).n).toBe(0)
    expect(get<{ n: number }>(f.db, 'SELECT COUNT(*) n FROM _migrations WHERE applied_at != ?', EPOCH).n).toBe(0)
    // extension_config uses datetime('now') format; it must stay in that format.
    expect(get<{ n: number }>(f.db, "SELECT COUNT(*) n FROM extension_config WHERE updated_at != '2025-08-01 12:00:00'").n).toBe(0)
  })

  test('recreates jd_updated_at byte-identically, and it still fires', () => {
    const f = fixture()
    const before = get<{ sql: string }>(f.db, "SELECT sql FROM sqlite_master WHERE type = 'trigger' AND name = 'jd_updated_at'").sql
    const result = f.db.transaction(() => backdate(f.db, f.entries, EPOCH))()
    const after = get<{ sql: string }>(f.db, "SELECT sql FROM sqlite_master WHERE type = 'trigger' AND name = 'jd_updated_at'").sql
    expect(after).toBe(before)
    expect(result.trigger).toBe(before)
    f.db.run("UPDATE job_descriptions SET title = 'Renamed' WHERE id = ?", [f.jdId])
    const updated = get<{ u: string }>(f.db, 'SELECT updated_at u FROM job_descriptions WHERE id = ?', f.jdId).u
    expect(updated).not.toBe('2026-02-01T15:00:00Z')
    expect(Math.abs(new Date(updated).getTime() - Date.now())).toBeLessThan(120_000)
  })

  test('fails, and rolls back, when an unknown trigger exists', () => {
    const f = fixture()
    f.db.run("CREATE TRIGGER sneaky AFTER UPDATE ON sources BEGIN UPDATE sources SET updated_at = '2099-01-01T00:00:00Z' WHERE id = NEW.id; END")
    expect(() => f.db.transaction(() => backdate(f.db, f.entries, EPOCH))()).toThrow(/unexpected trigger.*sneaky/)
    expect(get<{ n: number }>(f.db, "SELECT COUNT(*) n FROM sqlite_master WHERE type = 'trigger' AND name = 'jd_updated_at'").n).toBe(1)
  })

  test('fails when the ledger misses a row of a ledger-timed table', () => {
    const f = fixture()
    f.db.run("INSERT INTO sources (id, title, description) VALUES (?, 'Unplanned', 'Nobody recorded this one.')", [uuid()])
    expect(() => f.db.transaction(() => backdate(f.db, f.entries, EPOCH))()).toThrow(/sources.created_at has 1 row\(s\) the ledger does not cover/)
  })

  test('fails when a timestamp column has neither ledger times nor a rule', () => {
    const f = fixture()
    f.db.run("INSERT INTO extension_logs (error_code, message, layer) VALUES ('E', 'm', 'l')")
    expect(() => f.db.transaction(() => backdate(f.db, f.entries, EPOCH))()).toThrow(/extension_logs.created_at .* no ledger times or rule/)
  })
})

describe('overlay', () => {
  function withBullet(f: Fixture, status: string): string {
    const id = uuid()
    f.db.run('INSERT INTO bullets (id, content, source_content_snapshot, status) VALUES (?, ?, ?, ?)', [id, `Bullet ${id}`, 'What happened.', status])
    f.db.run('INSERT INTO bullet_sources (bullet_id, source_id, is_primary) VALUES (?, ?, 1)', [id, f.sourceId])
    return id
  }

  test('applies allowed transitions and logs them', () => {
    const f = fixture()
    const bullet = withBullet(f, 'approved')
    const log = applyOverlay(f.db, [
      { table: 'sources', ref: 'source:s', id: f.sourceId, from: 'draft', to: 'approved' },
      { table: 'bullets', ref: 'bullet:b', id: bullet, from: 'approved', to: 'archived' },
    ])
    expect(log.map((l) => `${l.table} ${l.from}->${l.to}`)).toEqual(['sources draft->approved', 'bullets approved->archived'])
    expect(get<{ s: string }>(f.db, 'SELECT status s FROM sources WHERE id = ?', f.sourceId).s).toBe('approved')
  })

  test('rejects a transition that is not allowed', () => {
    const f = fixture()
    const bullet = withBullet(f, 'in_review')
    expect(() => applyOverlay(f.db, [{ table: 'bullets', ref: 'bullet:b', id: bullet, from: 'in_review', to: 'archived' }])).toThrow(/not an allowed overlay/)
    expect(() => applyOverlay(f.db, [{ table: 'sources', ref: 'source:s', id: f.sourceId, from: 'approved', to: 'archived' }])).toThrow(/not an allowed overlay/)
  })

  test('rejects a step whose from-status does not match the row', () => {
    const f = fixture()
    const bullet = withBullet(f, 'in_review')
    expect(() => applyOverlay(f.db, [{ table: 'bullets', ref: 'bullet:b', id: bullet, from: 'approved', to: 'archived' }])).toThrow(/expected status approved, found in_review/)
  })

  test('refuses to archive a perspective a resume entry uses', () => {
    const f = fixture()
    const bullet = withBullet(f, 'approved')
    const persp = uuid()
    f.db.run(
      "INSERT INTO perspectives (id, bullet_id, content, bullet_content_snapshot, framing, status) VALUES (?, ?, 'P', ?, 'accomplishment', 'approved')",
      [persp, bullet, `Bullet ${bullet}`],
    )
    const resume = uuid()
    const section = uuid()
    f.db.run("INSERT INTO resumes (id, name, target_role, target_employer, archetype) VALUES (?, 'R', 'Dev', 'Co', 'infrastructure')", [resume])
    f.db.run("INSERT INTO resume_sections (id, resume_id, title, entry_type) VALUES (?, ?, 'Experience', 'experience')", [section, resume])
    f.db.run("INSERT INTO resume_entries (id, resume_id, section_id, perspective_id, perspective_content_snapshot) VALUES (?, ?, ?, ?, 'P')", [uuid(), resume, section, persp])
    expect(() => applyOverlay(f.db, [{ table: 'perspectives', ref: 'perspective:p', id: persp, from: 'approved', to: 'archived' }])).toThrow(/resume entr/)
  })

  test('inserts note references the API refuses, checking both ends', () => {
    const f = fixture()
    const note = uuid()
    const cred = uuid()
    f.db.run("INSERT INTO user_notes (id, content) VALUES (?, 'n')", [note])
    f.db.run("INSERT INTO credentials (id, credential_type, label) VALUES (?, 'drivers_license', 'DL')", [cred])
    applyNoteReferences(f.db, [{ ref: 'note:n', noteId: note, entityType: 'credential', entityRef: 'credential:c', entityId: cred }])
    expect(get<{ n: number }>(f.db, "SELECT COUNT(*) n FROM note_references WHERE entity_type = 'credential'").n).toBe(1)
    expect(() => applyNoteReferences(f.db, [{ ref: 'note:n', noteId: note, entityType: 'certification', entityRef: 'certification:x', entityId: uuid() }])).toThrow(
      /no certifications row/,
    )
  })
})

describe('dataset_meta', () => {
  const meta: DatasetMeta = {
    kind: 'generated',
    generator: '@forge/demo-data',
    generator_version: '0.0.0',
    seed: 's',
    persona: 'p',
    dataset_uuid: '5d660672-665c-512b-a0be-0b6a31d99e17',
    as_of: '2026-09-30T17:00:00Z',
    generated_at: '2026-10-01T00:00:00Z',
    schema_head: 'x',
  }

  test('schema_head is the newest migration file', () => {
    const f = fixture()
    const newest = readdirSync(MIGRATIONS_DIR).filter((n) => n.endsWith('.sql')).sort().at(-1)?.replace(/\.sql$/, '')
    expect(schemaHead(f.db)).toBe(newest as string)
  })

  test('writes every key once, and refuses empties and rewrites', () => {
    const f = fixture()
    expect(() => writeDatasetMeta(f.db, { ...meta, seed: '' })).toThrow(/seed is empty/)
    f.db.run('DELETE FROM dataset_meta')
    writeDatasetMeta(f.db, meta)
    expect(Object.keys(readDatasetMeta(f.db)).sort()).toEqual([...META_KEYS].sort())
    expect(() => writeDatasetMeta(f.db, meta)).toThrow(/already has/)
  })
})

describe('compaction', () => {
  test('a WAL-mode file fails the checks; after compaction it passes them', () => {
    const f = fixture()
    f.db.close()
    expect(fileFacts(f.path).writeVersion).toBe(2) // WAL
    expect(verifyCompacted(f.path).join('\n')).toMatch(/header bytes 18-19 are 2,2/)
    compact(f.path)
    expect(verifyCompacted(f.path)).toEqual([])
    const facts = fileFacts(f.path)
    expect([facts.writeVersion, facts.readVersion]).toEqual([1, 1])
    expect(facts.bytes % facts.pageSize).toBe(0)
    expect(existsSync(`${f.path}-wal`) || existsSync(`${f.path}-shm`)).toBe(false)
  })
})

describe('content fingerprint', () => {
  function seeded(): Fixture {
    const f = fixture()
    f.db.transaction(() => backdate(f.db, f.entries, EPOCH))()
    return f
  }

  test('equal content with different ids gives the same fingerprint', () => {
    const a = seeded()
    const b = seeded()
    expect(a.sourceId).not.toBe(b.sourceId)
    expect(contentFingerprint(a.db)).toBe(contentFingerprint(b.db))
    b.db.run("UPDATE sources SET description = 'Something else.' WHERE id = ?", [b.sourceId])
    expect(contentFingerprint(a.db)).not.toBe(contentFingerprint(b.db))
  })

  test('references are rendered as the target natural key', () => {
    const f = seeded()
    expect(naturalKeyMap(f.db).get(f.sourceId)).toBe('sources:A source')
  })

  test('an unknown UUID fails', () => {
    const f = seeded()
    f.db.run('UPDATE sources SET description = ? WHERE id = ?', [`See ${uuid()}.`, f.sourceId])
    expect(() => contentFingerprint(f.db)).toThrow(/unknown UUID .* in sources.description/)
  })

  test('a table with ids and rows but no natural-key rule fails', () => {
    const f = seeded()
    f.db.run('CREATE TABLE widgets (id TEXT PRIMARY KEY, name TEXT)')
    f.db.run("INSERT INTO widgets VALUES (?, 'w')", [uuid()])
    expect(() => contentFingerprint(f.db)).toThrow(/widgets has 1 row\(s\) but no natural-key rule/)
  })

  test('two rows with the same natural key fail', () => {
    const f = seeded()
    f.db.run("INSERT INTO sources (id, title, description, created_at, updated_at) VALUES (?, 'A source', 'dup', ?, ?)", [uuid(), EPOCH, EPOCH])
    expect(() => contentFingerprint(f.db)).toThrow(/share the natural key/)
  })
})

describe('invariants', () => {
  const corpus = PERSONAS['early-career-developer']
  if (!corpus) throw new Error('missing persona')
  const ctx: InvariantContext = { asOf: new Date('2026-09-30T17:00:00Z'), corpus }
  const run = (db: Database, name: string) => {
    const inv = INVARIANTS.find((i) => i.name === name)
    if (!inv) throw new Error(`no invariant ${name}`)
    return inv.check(db, ctx)
  }

  test('skill names must be unique ignoring case', () => {
    const f = fixture()
    expect(run(f.db, 'skill names unique ignoring case')).toEqual([])
    f.db.run("INSERT INTO skills (id, name) VALUES (?, 'Python'), (?, 'python')", [uuid(), uuid()])
    expect(run(f.db, 'skill names unique ignoring case')).toHaveLength(1)
  })

  test('a bullet needs exactly one primary source', () => {
    const f = fixture()
    const b = uuid()
    f.db.run("INSERT INTO bullets (id, content, source_content_snapshot) VALUES (?, 'B', 'What happened.')", [b])
    expect(run(f.db, 'one primary source per bullet')).toHaveLength(1)
    f.db.run('INSERT INTO bullet_sources (bullet_id, source_id, is_primary) VALUES (?, ?, 1)', [b, f.sourceId])
    expect(run(f.db, 'one primary source per bullet')).toEqual([])
    expect(run(f.db, 'bullet snapshot matches its primary source')).toEqual([])
    f.db.run("UPDATE sources SET description = 'Changed.' WHERE id = ?", [f.sourceId])
    expect(run(f.db, 'bullet snapshot matches its primary source')).toHaveLength(1)
  })

  test('timestamps after as_of are reported', () => {
    const f = fixture()
    f.db.transaction(() => backdate(f.db, f.entries, EPOCH))()
    expect(run(f.db, 'timestamps are well-formed and not after as_of')).toEqual([])
    f.db.run("UPDATE sources SET updated_at = '2027-01-01T00:00:00Z' WHERE id = ?", [f.sourceId])
    expect(run(f.db, 'timestamps are well-formed and not after as_of')).toEqual(['sources.updated_at: 2027-01-01T00:00:00Z is after as_of'])
  })

  test('the content scan covers every TEXT column', () => {
    const f = fixture()
    expect(run(f.db, 'content conventions (every TEXT value)')).toEqual([])
    f.db.run("UPDATE sources SET description = 'Reach me at someone@realmail.net' WHERE id = ?", [f.sourceId])
    expect(run(f.db, 'content conventions (every TEXT value)').join('\n')).toMatch(/sources.description: email not @example.com/)
  })
})
