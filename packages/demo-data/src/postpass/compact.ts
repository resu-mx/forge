/**
 * Compaction: the browser runtime's OPFS (sahpool) import forces a WAL-mode file into legacy
 * mode without checkpointing, which would drop anything still in the WAL. So the output is
 * checkpointed, switched to DELETE journal mode, and vacuumed, and then checked from the bytes
 * up: header bytes 18–19 (file format write/read versions) are 1/1 (legacy, not WAL), the size
 * is a whole number of pages, and no `-wal`/`-shm` sidecar is left.
 */

import { Database } from 'bun:sqlite'
import { existsSync, readFileSync, rmSync, statSync } from 'fs'

export function compact(path: string): void {
  const db = new Database(path)
  try {
    const cp = db.query('PRAGMA wal_checkpoint(TRUNCATE)').get() as { busy: number; log: number; checkpointed: number }
    if (cp.busy !== 0) throw new Error(`wal_checkpoint(TRUNCATE) reported busy=${cp.busy}: another connection holds the database`)
    const mode = db.query('PRAGMA journal_mode = DELETE').get() as { journal_mode: string }
    if (mode.journal_mode !== 'delete') throw new Error(`could not switch to DELETE journal mode (got ${mode.journal_mode})`)
    db.run('VACUUM')
  } finally {
    db.close()
  }
  // Leaving WAL mode deletes the -wal file, but some SQLite builds (the macOS system library
  // among them) leave the -shm index behind. With no -wal and a rollback-mode header it holds
  // nothing; remove it so the file ships alone.
  const facts = fileFacts(path)
  if (!existsSync(`${path}-wal`) && facts.writeVersion === 1 && facts.readVersion === 1) {
    rmSync(`${path}-shm`, { force: true })
  }
}

export interface FileFacts {
  bytes: number
  pageSize: number
  writeVersion: number
  readVersion: number
}

export function fileFacts(path: string): FileFacts {
  const bytes = statSync(path).size
  const header = readFileSync(path).subarray(0, 100)
  if (header.subarray(0, 16).toString('latin1') !== 'SQLite format 3\u0000') throw new Error(`${path} is not a SQLite database`)
  const raw = header.readUInt16BE(16)
  return { bytes, pageSize: raw === 1 ? 65536 : raw, writeVersion: header[18] ?? -1, readVersion: header[19] ?? -1 }
}

/** Problems with a compacted file (empty when it is ready to ship). */
export function verifyCompacted(path: string): string[] {
  const problems: string[] = []
  for (const suffix of ['-wal', '-shm', '-journal']) {
    if (existsSync(path + suffix)) problems.push(`sidecar ${suffix} exists`)
  }
  const facts = fileFacts(path)
  if (facts.writeVersion !== 1 || facts.readVersion !== 1) {
    problems.push(`header bytes 18-19 are ${facts.writeVersion},${facts.readVersion}; expected 1,1 (DELETE journal mode)`)
  }
  if (facts.bytes % facts.pageSize !== 0) problems.push(`size ${facts.bytes} is not a multiple of page size ${facts.pageSize}`)

  const db = new Database(path, { readonly: true })
  try {
    const mode = db.query('PRAGMA journal_mode').get() as { journal_mode: string }
    if (mode.journal_mode !== 'delete') problems.push(`journal_mode is ${mode.journal_mode}`)
    const pageCount = (db.query('PRAGMA page_count').get() as { page_count: number }).page_count
    if (pageCount * facts.pageSize !== facts.bytes) problems.push(`page_count × page_size (${pageCount * facts.pageSize}) != file size (${facts.bytes})`)
  } finally {
    db.close()
  }
  for (const suffix of ['-wal', '-shm']) {
    if (existsSync(path + suffix)) problems.push(`opening read-only created ${suffix}`)
  }
  return problems
}
