/**
 * Status overlay: the statuses no API route can set, applied with checked SQL.
 *
 * - Sources: no route changes a source's status (`UpdateSource` has no `status`), so
 *   `draft → in_review | approved | rejected | archived` is set here, after derivation.
 * - Bullets and perspectives: there is no archive route, so `approved → archived` is set here.
 *
 * Each step must match the row's current status and be one of the allowed transitions; an
 * archived row may not be referenced by a resume entry.
 *
 * Note references to credentials and certifications are inserted here too, when the API
 * refuses them (see `NoteReferenceStep`). Run inside the caller's transaction.
 */

import type { Database } from 'bun:sqlite'

export interface OverlayStep {
  table: 'sources' | 'bullets' | 'perspectives'
  /** Ledger reference, for the log. */
  ref: string
  id: string
  from: string
  to: string
}

export interface OverlayLogEntry {
  table: string
  ref: string
  from: string
  to: string
}

export const ALLOWED_OVERLAYS: Record<string, Record<string, readonly string[]>> = {
  sources: { draft: ['in_review', 'approved', 'rejected', 'archived'] },
  bullets: { approved: ['archived'] },
  perspectives: { approved: ['archived'] },
}

/**
 * A note reference the Rust API refuses: its `NoteReferenceEntityType` lacks `credential` and
 * `certification`, although the schema (migration 048) and the TS API accept them.
 */
export interface NoteReferenceStep {
  /** Ledger reference of the note, for the log. */
  ref: string
  noteId: string
  entityType: 'credential' | 'certification'
  /** Ledger reference of the target, for the log. */
  entityRef: string
  entityId: string
}

const REFERENCE_TABLES: Record<NoteReferenceStep['entityType'], string> = {
  credential: 'credentials',
  certification: 'certifications',
}

/** Insert note references the API cannot create, checking both ends exist. */
export function applyNoteReferences(db: Database, steps: readonly NoteReferenceStep[]): OverlayLogEntry[] {
  return steps.map((step) => {
    const table = REFERENCE_TABLES[step.entityType]
    if (!table) throw new Error(`note reference ${step.ref}: entity type ${step.entityType} is not overlaid`)
    if (!db.query('SELECT 1 FROM user_notes WHERE id = ?').get(step.noteId)) throw new Error(`note reference ${step.ref}: no note ${step.noteId}`)
    if (!db.query(`SELECT 1 FROM ${table} WHERE id = ?`).get(step.entityId)) {
      throw new Error(`note reference ${step.ref}: no ${table} row ${step.entityId}`)
    }
    db.run('INSERT INTO note_references (note_id, entity_type, entity_id) VALUES (?, ?, ?)', [step.noteId, step.entityType, step.entityId])
    return { table: 'note_references', ref: step.ref, from: '(none)', to: `${step.entityType} ${step.entityRef}` }
  })
}

export function applyOverlay(db: Database, steps: readonly OverlayStep[]): OverlayLogEntry[] {
  const log: OverlayLogEntry[] = []
  for (const step of steps) {
    const allowed = ALLOWED_OVERLAYS[step.table]?.[step.from] ?? []
    if (!allowed.includes(step.to)) {
      throw new Error(`overlay ${step.ref}: ${step.table} ${step.from} → ${step.to} is not an allowed overlay`)
    }
    const row = db.query(`SELECT status FROM ${step.table} WHERE id = ?`).get(step.id) as { status: string } | null
    if (!row) throw new Error(`overlay ${step.ref}: no ${step.table} row ${step.id}`)
    if (row.status !== step.from) {
      throw new Error(`overlay ${step.ref}: expected status ${step.from}, found ${row.status}`)
    }
    if (step.to === 'archived') {
      const column = step.table === 'sources' ? 'source_id' : step.table === 'perspectives' ? 'perspective_id' : null
      const used = column
        ? (db.query(`SELECT COUNT(*) AS n FROM resume_entries WHERE ${column} = ?`).get(step.id) as { n: number }).n
        : (db.query('SELECT COUNT(*) AS n FROM resume_entries e JOIN perspectives p ON p.id = e.perspective_id WHERE p.bullet_id = ?').get(step.id) as { n: number }).n
      if (used > 0) throw new Error(`overlay ${step.ref}: cannot archive, ${used} resume entr${used === 1 ? 'y' : 'ies'} reference it`)
    }
    const res = db.run(`UPDATE ${step.table} SET status = ? WHERE id = ? AND status = ?`, [step.to, step.id, step.from])
    if (res.changes !== 1) throw new Error(`overlay ${step.ref}: update changed ${res.changes} rows`)
    log.push({ table: step.table, ref: step.ref, from: step.from, to: step.to })
  }
  return log
}
