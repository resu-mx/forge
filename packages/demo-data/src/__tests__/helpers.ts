/**
 * Test helpers. `migratedDb` applies the shared SQL migrations (read from disk, the same files
 * both runtimes embed) to a fresh file, in WAL mode like `forge-server` leaves it. It does not
 * import core's runner: packages may not depend on core internals.
 */

import { Database } from 'bun:sqlite'
import { mkdtempSync, readdirSync, readFileSync, rmSync } from 'fs'
import { tmpdir } from 'os'
import { join, resolve } from 'path'
import { REPO_ROOT } from '../server'

export const MIGRATIONS_DIR = resolve(REPO_ROOT, 'packages/core/src/db/migrations')

export function tempDir(prefix: string): { dir: string; cleanup(): void } {
  const dir = mkdtempSync(join(tmpdir(), `forge-demo-test-${prefix}-`))
  return { dir, cleanup: () => rmSync(dir, { recursive: true, force: true }) }
}

export function migratedDb(path: string): Database {
  const db = new Database(path)
  db.run('PRAGMA journal_mode = WAL')
  db.run('PRAGMA foreign_keys = ON')
  for (const file of readdirSync(MIGRATIONS_DIR).filter((f) => f.endsWith('.sql')).sort()) {
    const sql = readFileSync(join(MIGRATIONS_DIR, file), 'utf8')
    const fkOff = sql.includes('PRAGMA foreign_keys = OFF')
    if (fkOff) db.run('PRAGMA foreign_keys = OFF')
    db.run('BEGIN')
    db.exec(sql)
    db.run('INSERT OR IGNORE INTO _migrations (name) VALUES (?)', [file.replace(/\.sql$/, '')])
    db.run('COMMIT')
    if (fkOff) db.run('PRAGMA foreign_keys = ON')
  }
  return db
}

export function uuid(): string {
  return crypto.randomUUID()
}
