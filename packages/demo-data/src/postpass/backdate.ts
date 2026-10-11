/**
 * Backdate every timestamp column (`*_at`) to the instants the ledger planned, keeping each
 * column's format. The server stamped rows with the wall clock; a dataset must read as if the
 * persona built it up over months before `--as-of`.
 *
 * Each `(table, column)` gets its values from exactly one place:
 * - the ledger, for entity rows the generator created (every non-null row must be covered);
 * - a rule, for seeded, system and child rows (SQL derived from already-backdated parents,
 *   or the dataset epoch).
 * A non-null timestamp column with neither fails the run, so a new migration that adds one
 * cannot slip through unbackdated.
 *
 * `jd_updated_at` is an unconditional AFTER UPDATE trigger that would re-stamp
 * `job_descriptions.updated_at` with the wall clock. It is saved from `sqlite_master`,
 * dropped, and recreated verbatim afterwards; the recreated SQL must be byte-identical.
 * Any other trigger fails the run. Run inside the caller's transaction.
 */

import type { Database } from 'bun:sqlite'
import type { LedgerEntry } from '../ledger'
import { inFormatOf, sqlTimestamp, timestampFormat } from '../time'

export const ALLOWED_TRIGGERS: readonly string[] = ['jd_updated_at']

type Rule = { table: string; column: string } & ({ epoch: true } | { sql: string })

const resumeCreated = (alias: string, fk: string) => `(SELECT r.created_at FROM resumes r WHERE r.id = ${alias}.${fk})`

/** Rules, applied in order after the ledger (later rules may read earlier results). */
export const TIMESTAMP_RULES: readonly Rule[] = [
  { table: '_migrations', column: 'applied_at', epoch: true },
  { table: 'archetypes', column: 'created_at', epoch: true },
  { table: 'domains', column: 'created_at', epoch: true },
  { table: 'archetype_domains', column: 'created_at', epoch: true },
  { table: 'resume_templates', column: 'created_at', epoch: true },
  { table: 'resume_templates', column: 'updated_at', epoch: true },
  { table: 'skill_graph_nodes', column: 'created_at', epoch: true },
  { table: 'skill_graph_nodes', column: 'updated_at', epoch: true },
  { table: 'skill_graph_edges', column: 'created_at', epoch: true },
  { table: 'skill_graph_edges', column: 'updated_at', epoch: true },
  { table: 'extension_config', column: 'updated_at', epoch: true },
  { table: 'skill_domains', column: 'created_at', sql: '(SELECT s.created_at FROM skills s WHERE s.id = skill_domains.skill_id)' },
  { table: 'summary_skills', column: 'created_at', sql: '(SELECT s.created_at FROM summaries s WHERE s.id = summary_skills.summary_id)' },
  { table: 'certification_skills', column: 'created_at', sql: '(SELECT c.created_at FROM certifications c WHERE c.id = certification_skills.certification_id)' },
  { table: 'profile_urls', column: 'created_at', sql: '(SELECT p.created_at FROM user_profile p WHERE p.id = profile_urls.profile_id)' },
  { table: 'org_locations', column: 'created_at', sql: '(SELECT o.created_at FROM organizations o WHERE o.id = org_locations.organization_id)' },
  {
    table: 'job_description_resumes',
    column: 'created_at',
    sql: `max((SELECT j.created_at FROM job_descriptions j WHERE j.id = job_description_resumes.job_description_id), ${resumeCreated('job_description_resumes', 'resume_id')})`,
  },
  { table: 'resume_sections', column: 'created_at', sql: resumeCreated('resume_sections', 'resume_id') },
  { table: 'resume_sections', column: 'updated_at', sql: resumeCreated('resume_sections', 'resume_id') },
  { table: 'resume_entries', column: 'created_at', sql: resumeCreated('resume_entries', 'resume_id') },
  { table: 'resume_entries', column: 'updated_at', sql: resumeCreated('resume_entries', 'resume_id') },
  { table: 'resume_certifications', column: 'created_at', sql: resumeCreated('resume_certifications', 'resume_id') },
  {
    table: 'resume_skills',
    column: 'created_at',
    sql: '(SELECT r.created_at FROM resume_sections s JOIN resumes r ON r.id = s.resume_id WHERE s.id = resume_skills.section_id)',
  },
  {
    table: 'prompt_logs',
    column: 'created_at',
    sql: `CASE prompt_logs.entity_type
            WHEN 'bullet' THEN (SELECT b.created_at FROM bullets b WHERE b.id = prompt_logs.entity_id)
            WHEN 'perspective' THEN (SELECT p.created_at FROM perspectives p WHERE p.id = prompt_logs.entity_id)
          END`,
  },
  { table: 'resumes', column: 'markdown_override_updated_at', sql: 'resumes.updated_at' },
  { table: 'resumes', column: 'latex_override_updated_at', sql: 'resumes.updated_at' },
  { table: 'resumes', column: 'summary_override_updated_at', sql: 'resumes.updated_at' },
]

export interface BackdateResult {
  ledgerUpdates: number
  ruleUpdates: number
  /** The trigger SQL that was dropped and recreated (null if absent). */
  trigger: string | null
}

function tables(db: Database): string[] {
  return (db.query("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name").all() as { name: string }[]).map((r) => r.name)
}

/** `*_at` columns per table. */
export function timestampColumns(db: Database): Map<string, string[]> {
  const out = new Map<string, string[]>()
  for (const t of tables(db)) {
    const cols = (db.query(`PRAGMA table_info("${t}")`).all() as { name: string }[]).map((c) => c.name).filter((c) => c.endsWith('_at'))
    if (cols.length > 0) out.set(t, cols)
  }
  return out
}

/** Save, drop, and return the JD trigger; throw on any trigger not in the allow-list. */
export function dropTriggers(db: Database): { name: string; sql: string } | null {
  const triggers = db.query("SELECT name, sql FROM sqlite_master WHERE type = 'trigger' ORDER BY name").all() as { name: string; sql: string }[]
  const unexpected = triggers.filter((t) => !ALLOWED_TRIGGERS.includes(t.name))
  if (unexpected.length > 0) {
    throw new Error(`unexpected trigger(s): ${unexpected.map((t) => t.name).join(', ')}; teach the post-pass how to backdate around them`)
  }
  const jd = triggers.find((t) => t.name === 'jd_updated_at') ?? null
  if (jd) db.run(`DROP TRIGGER "${jd.name}"`)
  return jd
}

/** Recreate a dropped trigger verbatim and assert the stored SQL is byte-identical. */
export function recreateTrigger(db: Database, trigger: { name: string; sql: string }): void {
  db.run(trigger.sql)
  const row = db.query("SELECT sql FROM sqlite_master WHERE type = 'trigger' AND name = ?").get(trigger.name) as { sql: string } | null
  if (!row) throw new Error(`trigger ${trigger.name} was not recreated`)
  if (row.sql !== trigger.sql) throw new Error(`trigger ${trigger.name} was recreated with different SQL`)
}

export function backdate(db: Database, entries: readonly LedgerEntry[], epochIso: string): BackdateResult {
  const columns = timestampColumns(db)
  const formats = new Map<string, 'iso' | 'sql'>()
  for (const [t, cols] of columns) {
    for (const c of cols) {
      const sample = db.query(`SELECT "${c}" AS v FROM "${t}" WHERE "${c}" IS NOT NULL LIMIT 1`).get() as { v: unknown } | null
      if (sample && typeof sample.v === 'string') {
        const fmt = timestampFormat(sample.v)
        if (!fmt) throw new Error(`${t}.${c}: unrecognised timestamp format "${sample.v}"`)
        formats.set(`${t}.${c}`, fmt)
      }
    }
  }

  const trigger = dropTriggers(db)

  // 1. Ledger.
  const ledgerSourced = new Map<string, Set<string>>() // "table.col" -> ids
  let ledgerUpdates = 0
  for (const e of entries) {
    for (const [col, iso] of Object.entries(e.times)) {
      const row = db.query(`SELECT "${col}" AS v FROM "${e.table}" WHERE id = ?`).get(e.id) as { v: unknown } | null
      if (!row) throw new Error(`backdate: ${e.kind}:${e.key} has no row in ${e.table}`)
      if (typeof row.v !== 'string') throw new Error(`backdate: ${e.kind}:${e.key} ${e.table}.${col} is ${row.v === null ? 'NULL' : typeof row.v}, but the ledger has a time for it`)
      db.run(`UPDATE "${e.table}" SET "${col}" = ? WHERE id = ?`, [inFormatOf(row.v, iso), e.id])
      const k = `${e.table}.${col}`
      if (!ledgerSourced.has(k)) ledgerSourced.set(k, new Set())
      ledgerSourced.get(k)?.add(e.id)
      ledgerUpdates++
    }
  }

  // 2. Rules.
  let ruleUpdates = 0
  const ruled = new Set<string>()
  for (const rule of TIMESTAMP_RULES) {
    const k = `${rule.table}.${rule.column}`
    if (!columns.get(rule.table)?.includes(rule.column)) continue
    if (ledgerSourced.has(k)) throw new Error(`backdate: ${k} has both ledger times and a rule`)
    ruled.add(k)
    const fmt = formats.get(k)
    if (!fmt) continue // no non-null values
    if ('epoch' in rule) {
      const value = fmt === 'iso' ? epochIso : sqlTimestamp(new Date(epochIso))
      ruleUpdates += db.run(`UPDATE "${rule.table}" SET "${rule.column}" = ? WHERE "${rule.column}" IS NOT NULL`, [value]).changes
    } else {
      ruleUpdates += db.run(`UPDATE "${rule.table}" SET "${rule.column}" = ${rule.sql} WHERE "${rule.column}" IS NOT NULL`).changes
    }
  }

  // 3. Coverage: every non-null timestamp came from the ledger (all rows) or a rule.
  for (const [t, cols] of columns) {
    for (const c of cols) {
      const k = `${t}.${c}`
      const nonNull = (db.query(`SELECT COUNT(*) AS n FROM "${t}" WHERE "${c}" IS NOT NULL`).get() as { n: number }).n
      if (nonNull === 0) continue
      if (!ruled.has(k)) {
        const covered = ledgerSourced.get(k)
        if (!covered) throw new Error(`backdate: ${k} has ${nonNull} non-null value(s) and no ledger times or rule`)
        if (covered.size !== nonNull) {
          const missing = (db.query(`SELECT id FROM "${t}" WHERE "${c}" IS NOT NULL`).all() as { id: string }[]).filter((r) => !covered.has(r.id))
          throw new Error(`backdate: ${k} has ${missing.length} row(s) the ledger does not cover: ${missing.slice(0, 5).map((r) => r.id).join(', ')}`)
        }
      }
      const fmt = formats.get(k)
      const bad = (db.query(`SELECT "${c}" AS v FROM "${t}" WHERE "${c}" IS NOT NULL`).all() as { v: string }[]).filter((r) => timestampFormat(r.v) !== fmt)
      if (bad.length > 0) throw new Error(`backdate: ${k} changed format (${bad[0]?.v})`)
    }
  }

  if (trigger) recreateTrigger(db, trigger)
  return { ledgerUpdates, ruleUpdates, trigger: trigger?.sql ?? null }
}
