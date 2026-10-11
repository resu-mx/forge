/**
 * Status overlay: the statuses no API route can set, applied with checked SQL.
 *
 * - Sources: no route changes a source's status (`UpdateSource` has no `status`), so
 *   `draft → in_review | approved | rejected | archived` is set here, after derivation.
 * - Bullets and perspectives: there is no archive route, so `approved → archived` is set here.
 *
 * Each step must match the row's current status and be one of the allowed transitions; an
 * archived row may not be referenced by a resume entry. Run inside the caller's transaction.
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
