/**
 * Tests for Migration 055: dataset_meta
 *
 * The table marks generated demo datasets and must stay EMPTY in real
 * user databases.
 */

import { describe, test, expect } from 'bun:test'
import { createTestDb } from './helpers'

describe('Migration 055: dataset_meta', () => {
  test('table exists and is empty on a fresh database', () => {
    const db = createTestDb()
    try {
      const t = db
        .query("SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'dataset_meta'")
        .get()
      expect(t).not.toBeNull()
      const row = db.query('SELECT COUNT(*) AS n FROM dataset_meta').get() as { n: number }
      expect(row.n).toBe(0)
    } finally {
      db.close()
    }
  })

  test('rejects empty and over-long keys', () => {
    const db = createTestDb()
    try {
      const ins = db.prepare('INSERT INTO dataset_meta (key, value) VALUES (?, ?)')
      expect(() => ins.run('', 'x')).toThrow()
      expect(() => ins.run('k'.repeat(65), 'x')).toThrow()
      ins.run('k'.repeat(64), 'x')
      ins.run('kind', 'generated')
    } finally {
      db.close()
    }
  })
})
