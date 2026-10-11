/**
 * Invariants a generated dataset must satisfy. Each check returns a list of problems; the
 * post-pass and `verify` fail on any. They read the database only (plus the corpus, for the
 * coverage the persona promised), so they work on a finished file as well as mid-pass.
 */

import type { Database } from 'bun:sqlite'
import { scanText } from '../conventions'
import {
  ANSWER_OPTIONS,
  JD_STATUSES,
  NOTE_ENTITY_TYPES,
  ORG_STATUSES,
  type PersonaCorpus,
  REVIEW_STATUSES,
  SOURCE_TYPES,
} from '../corpus/types'
import { parseTimestamp, timestampFormat } from '../time'
import { META_KEYS } from './meta'

export interface InvariantContext {
  asOf: Date
  corpus: PersonaCorpus
  /** Skip the checks that need `dataset_meta` (the post-pass runs some before writing it). */
  skipMeta?: boolean
}

export interface Invariant {
  name: string
  check(db: Database, ctx: InvariantContext): string[]
}

type Row = Record<string, unknown>

function rows(db: Database, sql: string, ...params: (string | number)[]): Row[] {
  return db.query(sql).all(...params) as Row[]
}

/** A check that fails with one problem per returned row. */
function sqlCheck(name: string, sql: string, describe: (r: Row) => string): Invariant {
  return { name, check: (db) => rows(db, sql).map(describe) }
}

function textTables(db: Database): string[] {
  return (db.query("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name").all() as { name: string }[]).map((r) => r.name)
}

/** Hosts app migrations seed into config rows (not generated content). */
const SEEDED_HOSTS: Record<string, readonly string[]> = { extension_config: ['localhost', '127.0.0.1'] }

function coverage(db: Database, table: string, column: string, want: readonly (string | null)[]): string[] {
  const have = new Set(rows(db, `SELECT DISTINCT ${column} AS v FROM ${table}`).map((r) => r.v as string | null))
  return want.filter((w) => !have.has(w)).map((w) => `${table}: no row with ${column} = ${w === null ? 'NULL' : w}`)
}

export const INVARIANTS: readonly Invariant[] = [
  sqlCheck(
    'one primary source per bullet',
    `SELECT b.id, (SELECT COUNT(*) FROM bullet_sources bs WHERE bs.bullet_id = b.id AND bs.is_primary = 1) AS n
       FROM bullets b WHERE n != 1`,
    (r) => `bullet ${r.id} has ${r.n} primary sources`,
  ),
  sqlCheck(
    'bullet snapshot matches its primary source',
    `SELECT b.id FROM bullets b JOIN bullet_sources bs ON bs.bullet_id = b.id AND bs.is_primary = 1
       JOIN sources s ON s.id = bs.source_id WHERE b.source_content_snapshot != s.description`,
    (r) => `bullet ${r.id}: source_content_snapshot differs from its primary source`,
  ),
  sqlCheck(
    'perspective snapshot matches its bullet',
    'SELECT p.id FROM perspectives p JOIN bullets b ON b.id = p.bullet_id WHERE p.bullet_content_snapshot != b.content',
    (r) => `perspective ${r.id}: bullet_content_snapshot differs from its bullet`,
  ),
  sqlCheck(
    'resume entry snapshot matches its perspective',
    `SELECT e.id FROM resume_entries e JOIN perspectives p ON p.id = e.perspective_id
       WHERE e.perspective_content_snapshot IS NULL OR e.perspective_content_snapshot != p.content`,
    (r) => `resume entry ${r.id}: perspective_content_snapshot differs from its perspective`,
  ),
  sqlCheck(
    'resume entries point only at approved perspectives',
    "SELECT e.id, p.status FROM resume_entries e JOIN perspectives p ON p.id = e.perspective_id WHERE p.status != 'approved'",
    (r) => `resume entry ${r.id} uses a ${r.status} perspective`,
  ),
  sqlCheck(
    'resume entries do not use archived sources or bullets',
    `SELECT e.id FROM resume_entries e LEFT JOIN sources s ON s.id = e.source_id
       LEFT JOIN perspectives p ON p.id = e.perspective_id LEFT JOIN bullets b ON b.id = p.bullet_id
       WHERE s.status = 'archived' OR b.status = 'archived'`,
    (r) => `resume entry ${r.id} uses an archived source or bullet`,
  ),
  sqlCheck(
    'each resume entry has exactly one driver',
    `SELECT id FROM resume_entries
       WHERE (perspective_id IS NOT NULL) + (source_id IS NOT NULL) + (content IS NOT NULL AND perspective_id IS NULL AND source_id IS NULL) != 1`,
    (r) => `resume entry ${r.id} has no single perspective, source or content`,
  ),
  sqlCheck(
    'archetype and domain names exist',
    `SELECT 'perspective ' || id || ' archetype ' || target_archetype AS what FROM perspectives
       WHERE target_archetype IS NOT NULL AND target_archetype NOT IN (SELECT name FROM archetypes)
     UNION ALL SELECT 'perspective ' || id || ' domain ' || domain FROM perspectives
       WHERE domain IS NOT NULL AND domain NOT IN (SELECT name FROM domains)
     UNION ALL SELECT 'bullet ' || id || ' domain ' || domain FROM bullets
       WHERE domain IS NOT NULL AND domain NOT IN (SELECT name FROM domains)
     UNION ALL SELECT 'resume ' || id || ' archetype ' || archetype FROM resumes
       WHERE archetype NOT IN (SELECT name FROM archetypes)`,
    (r) => `unknown name: ${r.what}`,
  ),
  sqlCheck(
    'section typing',
    `SELECT 'resume_skill ' || rs.id AS what FROM resume_skills rs JOIN resume_sections s ON s.id = rs.section_id WHERE s.entry_type != 'skills'
     UNION ALL SELECT 'resume_certification ' || rc.id FROM resume_certifications rc JOIN resume_sections s ON s.id = rc.section_id
       WHERE s.entry_type != 'certifications' OR s.resume_id != rc.resume_id
     UNION ALL SELECT 'resume_entry ' || e.id FROM resume_entries e JOIN resume_sections s ON s.id = e.section_id
       WHERE s.resume_id != e.resume_id OR s.entry_type IN ('skills', 'certifications')`,
    (r) => `mistyped section use: ${r.what}`,
  ),
  {
    name: 'exactly one profile, salaries in order',
    check(db) {
      const profiles = rows(db, 'SELECT salary_minimum AS lo, salary_target AS mid, salary_stretch AS hi FROM user_profile')
      if (profiles.length !== 1) return [`user_profile has ${profiles.length} rows`]
      const p = profiles[0] as { lo: number | null; mid: number | null; hi: number | null }
      if (p.lo === null || p.mid === null || p.hi === null) return ['profile salaries are not all set']
      return p.lo <= p.mid && p.mid <= p.hi ? [] : [`profile salaries out of order: ${p.lo} / ${p.mid} / ${p.hi}`]
    },
  },
  sqlCheck(
    'skill names unique ignoring case',
    'SELECT lower(name) AS n, COUNT(*) AS c FROM skills GROUP BY lower(name) HAVING c > 1',
    (r) => `skill name "${r.n}" appears ${r.c} times`,
  ),
  {
    name: 'skills are exactly the persona catalog',
    check(db, ctx) {
      const have = new Set(rows(db, 'SELECT name FROM skills').map((r) => r.name as string))
      const want = new Set(ctx.corpus.skills.map((s) => s.name))
      return [
        ...[...have].filter((n) => !want.has(n)).map((n) => `skill "${n}" is not in the catalog (created by a derivation?)`),
        ...[...want].filter((n) => !have.has(n)).map((n) => `catalog skill "${n}" is missing`),
      ]
    },
  },
  {
    name: 'credential details have a valid shape',
    check(db) {
      const problems: string[] = []
      for (const r of rows(db, 'SELECT id, credential_type AS t, details FROM credentials')) {
        let d: Record<string, unknown>
        try {
          d = JSON.parse(r.details as string) as Record<string, unknown>
        } catch {
          problems.push(`credential ${r.id}: details is not JSON`)
          continue
        }
        if (r.t === 'clearance') {
          const ok = typeof d.level === 'string' && (d.polygraph === null || typeof d.polygraph === 'string') &&
            typeof d.clearance_type === 'string' && Array.isArray(d.access_programs)
          if (!ok) problems.push(`credential ${r.id}: clearance details need {level, polygraph, clearance_type, access_programs[]}`)
        } else if (r.t === 'drivers_license') {
          const ok = typeof d.class === 'string' && typeof d.state === 'string' && Array.isArray(d.endorsements)
          if (!ok) problems.push(`credential ${r.id}: drivers_license details need {class, state, endorsements[]}`)
        }
      }
      return problems
    },
  },
  {
    name: 'answer bank values are valid',
    check(db) {
      return rows(db, 'SELECT field_kind AS k, value AS v FROM answer_bank').flatMap((r) => {
        const option = ANSWER_OPTIONS[r.k as string]
        if (!option) return [`answer_bank: unknown field_kind ${r.k}`]
        return option.values.includes(r.v as string) ? [] : [`answer_bank ${r.k}: "${r.v}" is not an offered value`]
      })
    },
  },
  sqlCheck(
    'rejection reasons match status',
    `SELECT 'bullet ' || id AS what, status FROM bullets
       WHERE (status = 'rejected') != (rejection_reason IS NOT NULL AND trim(rejection_reason) != '')
     UNION ALL SELECT 'perspective ' || id, status FROM perspectives
       WHERE (status = 'rejected') != (rejection_reason IS NOT NULL AND trim(rejection_reason) != '')`,
    (r) => `${r.what} (${r.status}): rejection_reason present iff rejected`,
  ),
  sqlCheck(
    'approved items carry approved_at',
    `SELECT 'bullet ' || id AS what FROM bullets WHERE status IN ('approved', 'archived') AND approved_at IS NULL
     UNION ALL SELECT 'perspective ' || id FROM perspectives WHERE status IN ('approved', 'archived') AND approved_at IS NULL`,
    (r) => `${r.what} is approved without approved_at`,
  ),
  sqlCheck('no pending derivations', 'SELECT id FROM pending_derivations', (r) => `pending derivation ${r.id} left behind`),
  sqlCheck("no source left 'deriving'", "SELECT id FROM sources WHERE status = 'deriving'", (r) => `source ${r.id} is still deriving`),
  sqlCheck(
    'every resume has a generated tagline',
    "SELECT id FROM resumes WHERE generated_tagline IS NULL OR trim(generated_tagline) = ''",
    (r) => `resume ${r.id} has no generated tagline`,
  ),
  {
    name: 'timestamps are well-formed and not after as_of',
    check(db, ctx) {
      const problems: string[] = []
      for (const t of textTables(db)) {
        const cols = (db.query(`PRAGMA table_info("${t}")`).all() as { name: string }[]).map((c) => c.name).filter((c) => c.endsWith('_at'))
        for (const c of cols) {
          for (const r of rows(db, `SELECT "${c}" AS v FROM "${t}" WHERE "${c}" IS NOT NULL`)) {
            const v = r.v
            if (typeof v !== 'string' || timestampFormat(v) === null) {
              problems.push(`${t}.${c}: malformed timestamp ${String(v)}`)
            } else if (parseTimestamp(v).getTime() > ctx.asOf.getTime()) {
              problems.push(`${t}.${c}: ${v} is after as_of`)
            }
          }
        }
      }
      return problems
    },
  },
  {
    name: 'content conventions (every TEXT value)',
    check(db) {
      const problems: string[] = []
      for (const t of textTables(db)) {
        const cols = (db.query(`PRAGMA table_info("${t}")`).all() as { name: string }[]).map((c) => c.name)
        for (const r of rows(db, `SELECT * FROM "${t}"`)) {
          for (const c of cols) {
            const v = r[c]
            if (typeof v !== 'string') continue
            for (const p of scanText(v, { extraHosts: SEEDED_HOSTS[t] })) problems.push(`${t}.${c}: ${p}`)
          }
        }
      }
      return problems
    },
  },
  {
    name: 'board coverage',
    check(db) {
      return [
        ...coverage(db, 'sources', 'status', REVIEW_STATUSES),
        ...coverage(db, 'bullets', 'status', REVIEW_STATUSES),
        ...coverage(db, 'perspectives', 'status', REVIEW_STATUSES),
        ...coverage(db, 'resumes', 'status', REVIEW_STATUSES),
        ...coverage(db, 'job_descriptions', 'status', JD_STATUSES),
        ...coverage(db, 'organizations', 'status', [...ORG_STATUSES, null]),
        ...coverage(db, 'sources', 'source_type', SOURCE_TYPES),
        ...coverage(db, 'note_references', 'entity_type', NOTE_ENTITY_TYPES),
      ]
    },
  },
  {
    name: 'statuses match the corpus plan',
    check(db, ctx) {
      const problems: string[] = []
      const tally = (table: string) =>
        Object.fromEntries(rows(db, `SELECT status AS s, COUNT(*) AS n FROM ${table} GROUP BY status`).map((r) => [r.s as string, r.n as number]))
      const plan = (items: { status: string }[]) =>
        items.reduce<Record<string, number>>((acc, x) => ({ ...acc, [x.status]: (acc[x.status] ?? 0) + 1 }), {})
      const pairs: [string, { status: string }[]][] = [
        ['sources', ctx.corpus.sources],
        ['bullets', ctx.corpus.bullets],
        ['perspectives', ctx.corpus.perspectives],
        ['resumes', ctx.corpus.resumes],
        ['job_descriptions', ctx.corpus.jds],
      ]
      for (const [table, items] of pairs) {
        const have = JSON.stringify(Object.entries(tally(table)).sort())
        const want = JSON.stringify(Object.entries(plan(items)).sort())
        if (have !== want) problems.push(`${table} statuses ${have}, planned ${want}`)
      }
      return problems
    },
  },
  {
    name: 'dataset_meta is complete',
    check(db, ctx) {
      if (ctx.skipMeta) return []
      const have = new Map(rows(db, 'SELECT key, value FROM dataset_meta').map((r) => [r.key as string, r.value as string]))
      const problems = META_KEYS.filter((k) => !have.get(k)).map((k) => `dataset_meta.${k} missing`)
      if (have.get('kind') !== 'generated') problems.push(`dataset_meta.kind is ${have.get('kind')}, expected generated`)
      if (have.get('persona') && have.get('persona') !== ctx.corpus.slug) problems.push(`dataset_meta.persona is ${have.get('persona')}`)
      return problems
    },
  },
  {
    name: 'integrity_check and foreign_key_check',
    check(db) {
      const integrity = rows(db, 'PRAGMA integrity_check').map((r) => Object.values(r)[0] as string)
      const fk = rows(db, 'PRAGMA foreign_key_check')
      return [
        ...(integrity.length === 1 && integrity[0] === 'ok' ? [] : integrity.map((i) => `integrity_check: ${i}`)),
        ...fk.map((r) => `foreign_key_check: ${JSON.stringify(r)}`),
      ]
    },
  },
]

/** Run every invariant; returns problems prefixed by the invariant's name. */
export function checkInvariants(db: Database, ctx: InvariantContext): string[] {
  return INVARIANTS.flatMap((inv) => inv.check(db, ctx).map((p) => `[${inv.name}] ${p}`))
}

/** Throw if any invariant fails. */
export function assertInvariants(db: Database, ctx: InvariantContext): void {
  const problems = checkInvariants(db, ctx)
  if (problems.length > 0) {
    const shown = problems.slice(0, 40).join('\n  ')
    throw new Error(`${problems.length} invariant violation(s):\n  ${shown}${problems.length > 40 ? '\n  …' : ''}`)
  }
}
