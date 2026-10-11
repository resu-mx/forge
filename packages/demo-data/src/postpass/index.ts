/**
 * The post-pass runs on the database file after `forge-server` has exited:
 *
 * 1. one transaction: status overlay (and refused note references), timestamp backdating,
 *    `dataset_meta`;
 * 2. invariants;
 * 3. compaction (checkpoint, DELETE journal mode, VACUUM) and its byte-level checks;
 * 4. invariants again on the read-only result, then counts, coverage and the fingerprint.
 */

import { Database } from 'bun:sqlite'
import type { PersonaCorpus } from '../corpus/types'
import type { LedgerEntry } from '../ledger'
import { type BackdateResult, backdate } from './backdate'
import { compact, verifyCompacted } from './compact'
import { contentFingerprint } from './fingerprint'
import { assertInvariants } from './invariants'
import { coverageReport, tableCounts } from './manifest'
import { type DatasetMeta, schemaHead, writeDatasetMeta } from './meta'
import { type NoteReferenceStep, type OverlayLogEntry, type OverlayStep, applyNoteReferences, applyOverlay } from './overlay'

export interface PostPassInput {
  dbPath: string
  corpus: PersonaCorpus
  asOf: Date
  /** ISO instant for seeded and system rows. */
  epoch: string
  entries: readonly LedgerEntry[]
  overlay: readonly OverlayStep[]
  noteReferences: readonly NoteReferenceStep[]
  meta: Omit<DatasetMeta, 'schema_head'>
}

export interface PostPassResult {
  schemaHead: string
  overlay: OverlayLogEntry[]
  backdate: BackdateResult
  contentFingerprint: string
  counts: Record<string, number>
  coverage: Record<string, Record<string, number>>
}

export function runPostPass(input: PostPassInput): PostPassResult {
  const db = new Database(input.dbPath)
  let head: string
  let overlay: OverlayLogEntry[]
  let bd: BackdateResult
  try {
    db.run('PRAGMA foreign_keys = ON')
    head = schemaHead(db)
    const apply = db.transaction(() => {
      const log = [...applyOverlay(db, input.overlay), ...applyNoteReferences(db, input.noteReferences)]
      const result = backdate(db, input.entries, input.epoch)
      writeDatasetMeta(db, { ...input.meta, schema_head: head })
      return { log, result }
    })
    const done = apply()
    overlay = done.log
    bd = done.result
    assertInvariants(db, { asOf: input.asOf, corpus: input.corpus })
  } finally {
    db.close()
  }

  compact(input.dbPath)
  const problems = verifyCompacted(input.dbPath)
  if (problems.length > 0) throw new Error(`compaction checks failed:\n  ${problems.join('\n  ')}`)

  const ro = new Database(input.dbPath, { readonly: true })
  try {
    assertInvariants(ro, { asOf: input.asOf, corpus: input.corpus })
    return {
      schemaHead: head,
      overlay,
      backdate: bd,
      contentFingerprint: contentFingerprint(ro),
      counts: tableCounts(ro),
      coverage: coverageReport(ro),
    }
  } finally {
    ro.close()
  }
}
