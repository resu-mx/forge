/**
 * Content fingerprint: a sha256 over a canonical dump of every table, with every UUID
 * replaced by the row's natural key. Ids are random (the server and the migrations mint
 * v4 UUIDs), so two runs never produce the same bytes; they must produce the same content.
 *
 * Natural keys come from the database itself (names, titles, contents, and the keys of
 * parent rows), so `verify` can recompute the fingerprint from a finished file. The corpus
 * tests keep those values unique within a persona. A UUID that is no row's id, a table with
 * an `id` column and rows but no natural-key rule, or two rows with the same key all fail.
 */

import type { Database } from 'bun:sqlite'
import { UUID_PATTERN } from '../ids'
import { VOLATILE_META_KEYS } from './meta'

/** `table` → SQL yielding `(id, nk)` for every row. */
export const NATURAL_KEYS: Record<string, string> = {
  addresses: "SELECT id, name || ' / ' || coalesce(street_1, '-') AS nk FROM addresses",
  answer_bank: 'SELECT id, field_kind AS nk FROM answer_bank',
  archetypes: 'SELECT id, name AS nk FROM archetypes',
  bullets: 'SELECT id, content AS nk FROM bullets',
  certifications: 'SELECT id, short_name AS nk FROM certifications',
  contacts: 'SELECT id, name AS nk FROM contacts',
  credentials: 'SELECT id, label AS nk FROM credentials',
  domains: 'SELECT id, name AS nk FROM domains',
  industries: 'SELECT id, name AS nk FROM industries',
  job_descriptions:
    "SELECT j.id, j.title || ' @ ' || coalesce(o.name, '-') AS nk FROM job_descriptions j LEFT JOIN organizations o ON o.id = j.organization_id",
  org_aliases: "SELECT a.id, o.name || ' / ' || a.alias AS nk FROM org_aliases a JOIN organizations o ON o.id = a.organization_id",
  org_locations: "SELECT l.id, o.name || ' / ' || l.name AS nk FROM org_locations l JOIN organizations o ON o.id = l.organization_id",
  organizations: 'SELECT id, name AS nk FROM organizations',
  perspectives: 'SELECT id, content AS nk FROM perspectives',
  profile_urls: 'SELECT id, key AS nk FROM profile_urls',
  prompt_logs: `SELECT pl.id, pl.entity_type || ' / ' || coalesce(b.content, p.content, pl.entity_id) AS nk
                  FROM prompt_logs pl LEFT JOIN bullets b ON b.id = pl.entity_id LEFT JOIN perspectives p ON p.id = pl.entity_id`,
  resume_certifications: `SELECT rc.id, r.name || ' / ' || c.short_name AS nk FROM resume_certifications rc
                            JOIN resumes r ON r.id = rc.resume_id JOIN certifications c ON c.id = rc.certification_id`,
  resume_entries: `SELECT e.id, r.name || ' / ' || s.title || ' / ' || e.position AS nk FROM resume_entries e
                     JOIN resumes r ON r.id = e.resume_id JOIN resume_sections s ON s.id = e.section_id`,
  resume_sections: "SELECT s.id, r.name || ' / ' || s.title AS nk FROM resume_sections s JOIN resumes r ON r.id = s.resume_id",
  resume_skills: `SELECT rs.id, r.name || ' / ' || s.title || ' / ' || k.name AS nk FROM resume_skills rs
                    JOIN resume_sections s ON s.id = rs.section_id JOIN resumes r ON r.id = s.resume_id JOIN skills k ON k.id = rs.skill_id`,
  resume_templates: 'SELECT id, name AS nk FROM resume_templates',
  resumes: 'SELECT id, name AS nk FROM resumes',
  role_types: 'SELECT id, name AS nk FROM role_types',
  skill_categories: 'SELECT id, slug AS nk FROM skill_categories',
  skill_graph_nodes: 'SELECT id, canonical_name AS nk FROM skill_graph_nodes',
  skills: 'SELECT id, name AS nk FROM skills',
  sources: 'SELECT id, title AS nk FROM sources',
  summaries: 'SELECT id, title AS nk FROM summaries',
  user_notes: 'SELECT id, coalesce(title, content) AS nk FROM user_notes',
  user_profile: "SELECT id, 'profile' AS nk FROM user_profile",
}

function tables(db: Database): string[] {
  return (db.query("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name").all() as { name: string }[]).map((r) => r.name)
}

function hasTextId(db: Database, table: string): boolean {
  return (db.query(`PRAGMA table_info("${table}")`).all() as { name: string; type: string }[]).some(
    (c) => c.name === 'id' && c.type.toUpperCase() === 'TEXT',
  )
}

/**
 * id → `table:natural key`, for every row of every table with a TEXT `id`. The dataset's own
 * uuid (from `dataset_meta`, also quoted in the "About" note) maps to `dataset`.
 */
export function naturalKeyMap(db: Database): Map<string, string> {
  const map = new Map<string, string>()
  const seen = new Map<string, string>()
  const datasetUuid = tables(db).includes('dataset_meta')
    ? (db.query("SELECT value FROM dataset_meta WHERE key = 'dataset_uuid'").get() as { value: string } | null)?.value
    : undefined
  if (datasetUuid) map.set(datasetUuid.toLowerCase(), 'dataset')
  for (const t of tables(db)) {
    if (!hasTextId(db, t)) continue
    const count = (db.query(`SELECT COUNT(*) AS n FROM "${t}"`).get() as { n: number }).n
    if (count === 0) continue
    const sql = NATURAL_KEYS[t]
    if (!sql) throw new Error(`fingerprint: table ${t} has ${count} row(s) but no natural-key rule`)
    const rows = db.query(sql).all() as { id: string; nk: string | null }[]
    if (rows.length !== count) throw new Error(`fingerprint: natural-key rule for ${t} covers ${rows.length} of ${count} rows`)
    for (const r of rows) {
      if (r.nk === null) throw new Error(`fingerprint: ${t} row ${r.id} has a NULL natural key`)
      const key = `${t}:${r.nk}`
      const clash = seen.get(key)
      if (clash) throw new Error(`fingerprint: two ${t} rows share the natural key "${r.nk}"`)
      seen.set(key, r.id)
      map.set(r.id.toLowerCase(), key)
    }
  }
  return map
}

function canonicalValue(v: unknown, map: Map<string, string>, where: string): unknown {
  if (typeof v === 'string') {
    return v.replace(UUID_PATTERN, (u) => {
      const key = map.get(u.toLowerCase())
      if (!key) throw new Error(`fingerprint: unknown UUID ${u} in ${where}`)
      return `{${key}}`
    })
  }
  if (v instanceof Uint8Array) return `x'${Buffer.from(v).toString('hex')}'`
  return v
}

/** The canonical dump: table → sorted rows (each a JSON object with sorted keys). */
export function canonicalDump(db: Database): Record<string, string[]> {
  const map = naturalKeyMap(db)
  const out: Record<string, string[]> = {}
  for (const t of tables(db)) {
    const cols = (db.query(`PRAGMA table_info("${t}")`).all() as { name: string }[]).map((c) => c.name).sort()
    const rows = db.query(`SELECT * FROM "${t}"`).all() as Record<string, unknown>[]
    out[t] = rows
      .filter((r) => !(t === 'dataset_meta' && VOLATILE_META_KEYS.includes(r.key as never)))
      .map((r) => JSON.stringify(cols.map((c) => [c, canonicalValue(r[c], map, `${t}.${c}`)])))
      .sort()
  }
  return out
}

export function contentFingerprint(db: Database): string {
  const hasher = new Bun.CryptoHasher('sha256')
  hasher.update(JSON.stringify(canonicalDump(db)))
  return hasher.digest('hex')
}
