/**
 * Resume IR Compiler — assembles a ResumeDocument from database state.
 *
 * Reads resume_sections from the database and dispatches to section-type
 * builders. Each builder queries entries via section_id instead of
 * hardcoded section strings.
 */

import type { Database } from 'bun:sqlite'
import { buildDefaultElm } from '../storage/build-elm'
import { nonBlank } from './tagline-service'
import type { EntityLifecycleManager } from '../storage/lifecycle-manager'
import type {
  ResumeDocument,
  ResumeHeader,
  ResumeSummary,
  IRSection,
  IRSectionType,
  IRSectionItem,
  ExperienceGroup,
  ExperienceSubheading,
  ExperienceBullet,
  SkillGroup,
  EducationItem,
  ProjectItem,
  CertificationGroup,
  ClearanceItem,
  PresentationItem,
  SummaryItem,
  UserProfile,
} from '../types'

/** Truncate text to `len` characters, appending '...' if truncated. */
function truncate(text: string | null, len = 60): string {
  if (!text) return ''
  if (text.length <= len) return text
  return text.slice(0, len) + '...'
}

// ── Row types for query results ─────────────────────────────────────

interface ResumeRow {
  id: string
  name: string
  target_role: string
  header: string | null  // JSON blob
  summary_id: string | null
  generated_tagline: string | null
  tagline_override: string | null
  summary_override: string | null
  show_clearance_in_header: number
}

interface ResumeSectionRow {
  id: string
  resume_id: string
  title: string
  entry_type: string
  position: number
}

interface ExperienceEntryRow {
  entry_id: string
  entry_content: string | null
  // Nullable after the compiler LEFT JOINs perspectives to support entries
  // that link to a source directly (re.source_id) without going through
  // the perspective → bullet chain. The row-mapper below coalesces
  // content across entry_content → perspective_content → source.description.
  perspective_id: string | null
  perspective_content: string | null
  bullet_id: string | null
  bullet_content: string | null
  source_id: string | null
  source_title: string | null
  organization_id: string | null
  org_name: string | null
  org_city: string | null
  org_state: string | null
  work_arrangement: string | null
  employment_type: string | null
  start_date: string | null
  end_date: string | null
  is_current: number | null
  position: number
}

interface SkillRow {
  category: string | null
  skill_name: string
}

// ── Compiler ─────────────────────────────────────────────────────────

/**
 * Compile a resume into the IR format.
 *
 * Returns null if the resume does not exist.
 *
 * Phase 1.4: optional elm parameter for ELM compatibility. Internal
 * builder queries stay as raw SQL (complex JOINs with 6-8 tables that
 * would be extremely verbose as multi-step ELM fetches). These will
 * become named queries in Phase 2 when adapter portability matters.
 */
export function compileResumeIR(db: Database, resumeId: string, elm?: EntityLifecycleManager): ResumeDocument | null {
  // 1. Fetch resume base data (including Phase 92 tagline columns)
  const resume = db
    .query(
      `SELECT id, name, target_role, header, summary_id,
              generated_tagline, tagline_override,
              summary_override, show_clearance_in_header
       FROM resumes WHERE id = ?`,
    )
    .get(resumeId) as ResumeRow | null

  if (!resume) return null

  // 2. Fetch user profile for contact fields, including address and URLs
  const profileRow = db
    .query('SELECT * FROM user_profile LIMIT 1')
    .get() as Record<string, unknown> | null

  let profile: UserProfile | null = null
  if (profileRow) {
    const address = profileRow.address_id
      ? db.query('SELECT * FROM addresses WHERE id = ?').get(profileRow.address_id as string) as UserProfile['address']
      : null
    const urls = db
      .query('SELECT * FROM profile_urls WHERE profile_id = ? ORDER BY position')
      .all(profileRow.id as string) as UserProfile['urls']
    profile = { ...profileRow, address, urls } as unknown as UserProfile
  }

  // 4. Parse header — contact fields from profile, tagline from resume-level
  // `tagline_override ?? generated_tagline` (Phase 92). Summary is no longer
  // consulted for tagline.
  const header = parseHeader(db, resume, profile)
  const summary = buildSummary(db, resume)

  // 4. Fetch sections from resume_sections table
  const sectionRows = db
    .query('SELECT * FROM resume_sections WHERE resume_id = ? ORDER BY position')
    .all(resumeId) as ResumeSectionRow[]

  // 5. Build IR sections from DB sections
  const sections: IRSection[] = sectionRows
    // Filter out user-created 'summary' sections — the summary is
    // rendered from `ir.summary` via a synthetic section below.
    .filter(s => s.entry_type !== 'summary')
    .map(section => {
      const items = buildSectionItems(db, section)
      return {
        id: section.id,  // real UUID from DB, not synthetic
        type: section.entry_type as IRSectionType,
        title: section.title,
        display_order: section.position,
        items,
      }
    })

  // 6. Inject a synthetic summary section from ir.summary so
  //    PDF/LaTeX/Markdown renderers always pick it up. Placed at
  //    display_order -1 so it sorts before all user sections.
  if (summary) {
    sections.unshift({
      id: syntheticUUID(resumeId, 'summary'),
      type: 'summary',
      title: 'Summary',
      display_order: -1,
      items: [{
        kind: 'summary',
        content: summary.content,
        entry_id: null,
      }],
    })
  }

  return { resume_id: resumeId, header, summary, sections }
}

/**
 * Build the ResumeSummary IR entry from resume.summary_id and
 * resume.summary_override. Resolution order:
 *   1. summary_override (local edit, highest priority)
 *   2. summaries.description via summary_id (template reference)
 *   3. null (empty state — card shows "Select summary" button)
 */
function buildSummary(db: Database, resume: ResumeRow): ResumeSummary | null {
  const summaryId = resume.summary_id
  const override = resume.summary_override
  if (!summaryId && !override) return null

  let title: string | null = null
  let templateDescription: string | null = null
  if (summaryId) {
    const row = db
      .query('SELECT id, title, description FROM summaries WHERE id = ?')
      .get(summaryId) as { id: string; title: string; description: string | null } | null
    if (row) {
      title = row.title
      templateDescription = row.description
    }
  }

  const content = override ?? templateDescription ?? ''
  if (!content) return null

  return {
    summary_id: summaryId,
    title,
    content,
    is_override: override !== null,
  }
}

/**
 * Dispatch to the correct builder based on entry_type.
 */
function buildSectionItems(db: Database, section: ResumeSectionRow): IRSectionItem[] {
  switch (section.entry_type) {
    case 'experience': return buildExperienceItems(db, section.id)
    case 'skills': return buildSkillItems(db, section.id)
    case 'education': return buildEducationItems(db, section.id)
    case 'projects': return buildProjectItems(db, section.id)
    case 'clearance': return buildClearanceItems(db, section.id)
    case 'presentations': return buildPresentationItems(db, section.id)
    case 'freeform': return buildFreeformItems(db, section.id)
    case 'awards': return buildFreeformItems(db, section.id)
    case 'certifications': return buildCertificationItems(db, section.id)
    default: return []
  }
}

// ── Section builders ─────────────────────────────────────────────────

/**
 * Clearance level ordering: highest first. Used to pick the "most
 * impressive" active clearance for the resume header one-liner.
 */
const CLEARANCE_LEVEL_ORDER: Record<string, number> = {
  top_secret: 6,
  q: 5,
  secret: 4,
  l: 3,
  confidential: 2,
  public: 1,
}

/**
 * Build a one-liner clearance string from the user's highest active
 * clearance credential for the resume header.
 *
 * Format: `Active {level} Clearance[ with {polygraph}]`
 * Returns null if no active clearance credentials exist.
 */
function buildHeaderClearanceLine(db: Database): string | null {
  type ClearanceRow = {
    details: string
  }

  const rows = db
    .query(
      `SELECT details FROM credentials
       WHERE credential_type = 'clearance' AND status = 'active'`,
    )
    .all() as ClearanceRow[]

  if (rows.length === 0) return null

  // Parse all active clearance details and pick the highest level
  let bestLevel = ''
  let bestOrder = -1
  let bestPolygraph: string | null = null

  for (const row of rows) {
    try {
      const d = JSON.parse(row.details) as { level?: string; polygraph?: string | null }
      const order = CLEARANCE_LEVEL_ORDER[d.level ?? ''] ?? 0
      if (order > bestOrder) {
        bestOrder = order
        bestLevel = d.level ?? ''
        bestPolygraph = d.polygraph ?? null
      }
    } catch {
      // Skip malformed details
    }
  }

  if (!bestLevel) return null

  let line = `Active ${formatClearanceLevel(bestLevel)} Clearance`
  const polyLabel = formatPolygraph(bestPolygraph)
  if (polyLabel) line += ` with ${polyLabel}`

  return line
}

function parseHeader(db: Database, resume: ResumeRow, profile: UserProfile | null): ResumeHeader {
  // Tagline resolution order (Phase 92):
  //   1. resume.tagline_override (user-authored, highest priority)
  //   2. resume.generated_tagline (TF-IDF extracted from linked JDs)
  //   3. header JSON tagline (legacy resume-specific header)
  //   4. resume.target_role (fallback)
  // A blank string (empty or whitespace-only) counts as absent at every step, the same rule as
  // `resolved` and `has_override` (resu-mx/forge#72).
  let tagline: string | null = resume.target_role
  if (resume.header) {
    try {
      const parsed = JSON.parse(resume.header)
      const legacy = typeof parsed.tagline === 'string' ? nonBlank(parsed.tagline) : null
      if (legacy) tagline = legacy
    } catch {
      // Fall through
    }
  }
  const generated = nonBlank(resume.generated_tagline)
  if (generated) tagline = generated
  const override = nonBlank(resume.tagline_override)
  if (override) tagline = override

  // Warn on placeholder profile data
  const name = profile?.name ?? resume.name
  if (!profile) {
    console.warn('[resume-compiler] No user_profile row found. Header name falls back to resume.name:', JSON.stringify(resume.name))
  } else if (profile.name === 'User') {
    console.warn('[resume-compiler] user_profile.name is "User" (seed default). Update via Settings > Profile.')
  }

  // Check for missing contact fields
  const hasContactUrl = profile?.urls?.some(u => ['linkedin', 'github'].includes(u.key))
  if (profile && !profile.email && !profile.phone && !hasContactUrl) {
    console.warn('[resume-compiler] user_profile has no contact fields populated. Header will have no contact info.')
  }

  // Clearance one-liner: per-resume toggle. When enabled and the user has
  // an active clearance credential, show the highest-level one as a header
  // line between tagline and contact info.
  const clearance = resume.show_clearance_in_header
    ? buildHeaderClearanceLine(db)
    : null

  // Contact fields from profile (single source of truth).
  return {
    name,
    tagline,
    location: profile?.address?.name ?? null,
    email: profile?.email ?? null,
    phone: profile?.phone ?? null,
    linkedin: ensureProtocol(profile?.urls?.find(u => u.key === 'linkedin')?.url ?? null),
    github: ensureProtocol(profile?.urls?.find(u => u.key === 'github')?.url ?? null),
    website: ensureProtocol(profile?.urls?.find(u => u.key === 'blog')?.url ?? profile?.urls?.find(u => u.key === 'portfolio')?.url ?? null),
    clearance,
  }
}

function buildFreeformItems(db: Database, sectionId: string): SummaryItem[] {
  const rows = db
    .query(
      `SELECT re.id AS entry_id, re.content
       FROM resume_entries re
       WHERE re.section_id = ?
       ORDER BY re.position ASC`
    )
    .all(sectionId) as Array<{ entry_id: string; content: string | null }>

  return rows.filter(r => r.content).map(r => ({
    kind: 'summary' as const,
    content: r.content!,
    entry_id: r.entry_id,
  }))
}

function buildExperienceItems(db: Database, sectionId: string): ExperienceGroup[] {
  // Dual-path source resolution via COALESCE(re.source_id, bs.source_id).
  // Experience entries almost always come through the perspective chain
  // (bullets are the main derivation output for role sources), but the
  // LEFT JOIN keeps the query robust against direct-source entries too.
  const rows = db
    .query(
      `SELECT
        re.id AS entry_id,
        re.content AS entry_content,
        re.perspective_id,
        re.position,
        p.content AS perspective_content,
        p.bullet_id,
        b.content AS bullet_content,
        COALESCE(re.source_id, bs.source_id) AS source_id,
        s.title AS source_title,
        sr.organization_id,
        sr.start_date,
        sr.end_date,
        sr.is_current,
        sr.work_arrangement,
        o.name AS org_name,
        o.employment_type,
        addr.city AS org_city,
        addr.state AS org_state
      FROM resume_entries re
      LEFT JOIN perspectives p ON p.id = re.perspective_id
      LEFT JOIN bullets b ON b.id = p.bullet_id
      LEFT JOIN bullet_sources bs ON bs.bullet_id = p.bullet_id AND bs.is_primary = 1
      LEFT JOIN sources s ON s.id = COALESCE(re.source_id, bs.source_id)
      LEFT JOIN source_roles sr ON sr.source_id = s.id
      LEFT JOIN organizations o ON o.id = sr.organization_id
      LEFT JOIN org_locations ol ON ol.id = (
        SELECT id FROM org_locations
        WHERE organization_id = o.id
        ORDER BY is_headquarters DESC, id ASC
        LIMIT 1
      )
      LEFT JOIN addresses addr ON addr.id = ol.address_id
      WHERE re.section_id = ?
      ORDER BY sr.is_current DESC, sr.start_date DESC, re.position ASC`
    )
    .all(sectionId) as ExperienceEntryRow[]

  // Group by organization_id (or org_name fallback), then sub-group by source_title (role)
  // Using organization_id prevents collision when two different orgs have the same name.
  const orgMap = new Map<string, Map<string, ExperienceEntryRow[]>>()

  for (const row of rows) {
    const orgKey = row.organization_id ?? row.org_name ?? 'Other'
    if (!orgMap.has(orgKey)) orgMap.set(orgKey, new Map())
    const roleMap = orgMap.get(orgKey)!
    // Role key: prefer the source title; fall back to a stable "untitled"
    // token so direct-source entries without a title still group per-source
    // instead of colliding across unrelated sources.
    const roleKey = row.source_title ?? `untitled:${row.source_id ?? row.entry_id}`
    if (!roleMap.has(roleKey)) roleMap.set(roleKey, [])
    roleMap.get(roleKey)!.push(row)
  }

  const groups: ExperienceGroup[] = []

  for (const [orgKey, roleMap] of orgMap) {
    const subheadings: ExperienceSubheading[] = []

    // Use first entry in the group to build the display string.
    // Defense-in-depth: when all roles in this group are unlinked
    // (no organization_id and no org_name), use a distinct label that
    // makes the data problem visible instead of silently degrading to
    // "Other (Remote)". The user can fix this by linking their sources
    // to organizations on the /data/sources page.
    const firstEntry = roleMap.values().next().value![0]
    const isUnlinked = orgKey === 'Other' && !firstEntry.organization_id && !firstEntry.org_name
    const orgDisplayName = isUnlinked
      ? '⚠ Unlinked Sources'
      : buildOrgDisplayString(
          firstEntry.org_name,
          firstEntry.org_city,
          firstEntry.org_state,
          firstEntry.work_arrangement,
        )

    if (isUnlinked) {
      const roleNames = Array.from(roleMap.keys()).join(', ')
      console.warn(
        `[resume-compiler] Experience group has ${roleMap.size} role(s) without organization_id: ${roleNames}. ` +
        `Link them in /data/sources for proper resume grouping.`
      )
    }

    for (const [roleTitle, entries] of roleMap) {
      const first = entries[0]
      const dateRange = formatDateRange(first.start_date, first.end_date, !!first.is_current)

      const bullets: ExperienceBullet[] = entries.map(e => {
        // Only attach a source_chain when the full perspective-chain is
        // present. Direct-source entries (re.source_id set, perspective
        // null) omit source_chain since there's no bullet/perspective
        // provenance to point to — the entry itself is the only record.
        const hasChain =
          e.perspective_id !== null &&
          e.bullet_id !== null &&
          e.source_id !== null &&
          e.source_title !== null
        return {
          content: e.entry_content ?? e.perspective_content ?? e.bullet_content ?? '',
          entry_id: e.entry_id,
          source_chain: hasChain
            ? {
                source_id: e.source_id!,
                source_title: truncate(e.source_title!, 60),
                bullet_id: e.bullet_id!,
                bullet_preview: truncate(e.bullet_content ?? '', 60),
                perspective_id: e.perspective_id!,
                perspective_preview: truncate(e.perspective_content ?? '', 60),
              }
            : undefined,
          is_cloned: e.entry_content !== null,
        }
      })

      subheadings.push({
        id: syntheticUUID('subheading', `${orgKey}-${roleTitle}`),
        title: roleTitle.startsWith('untitled:') ? 'Untitled Role' : roleTitle,
        location: buildRoleLocationString(first.org_city, first.org_state, first.work_arrangement),
        date_range: dateRange,
        source_id: first.source_id,
        bullets,
      })
    }

    groups.push({
      kind: 'experience_group',
      id: syntheticUUID('org', orgKey),
      organization: orgDisplayName,
      subheadings,
    })
  }

  return groups
}

function buildSkillItems(db: Database, sectionId: string): SkillGroup[] {
  // Check for orphaned resume_skills entries (skill_id points to nonexistent skill)
  const orphanCount = db
    .query(
      `SELECT COUNT(*) AS cnt
       FROM resume_skills rs
       LEFT JOIN skills s ON s.id = rs.skill_id
       WHERE rs.section_id = ? AND s.id IS NULL`
    )
    .get(sectionId) as { cnt: number }

  if (orphanCount.cnt > 0) {
    console.warn(`[resume-compiler] ${orphanCount.cnt} orphaned resume_skills entries in section ${sectionId} (skill_id FK missing). These are skipped.`)
  }

  const rows = db
    .query(
      `SELECT s.name AS skill_name, COALESCE(sc.display_name, s.category) AS category_display
       FROM resume_skills rs
       JOIN skills s ON s.id = rs.skill_id
       LEFT JOIN skill_categories sc ON sc.slug = s.category
       WHERE rs.section_id = ?
       ORDER BY COALESCE(sc.position, 999) ASC, rs.position ASC`
    )
    .all(sectionId) as Array<{ skill_name: string; category_display: string | null }>

  if (rows.length === 0) return []

  // Group by category display name (ordered by position due to ORDER BY above)
  const catMap = new Map<string, string[]>()
  for (const row of rows) {
    const cat = row.category_display ?? 'Other'
    if (!catMap.has(cat)) catMap.set(cat, [])
    catMap.get(cat)!.push(row.skill_name)
  }

  return [{
    kind: 'skill_group',
    categories: Array.from(catMap.entries()).map(([label, skills]) => ({
      label,
      skills,
    })),
  }]
}

function buildEducationItems(db: Database, sectionId: string): EducationItem[] {
  // Entries may reach their source via two paths:
  //   1. perspective-chain:  re → perspective → bullet → bullet_sources → source
  //   2. direct:             re.source_id → source
  // We LEFT JOIN the perspective chain and use COALESCE to pick whichever
  // source id is present, preferring the perspective-chain source when both
  // are set. This lets education entries added from sources without
  // derived perspectives still render with structured data.
  //
  // The display text (`degree`) falls back in this order:
  //   entry_content → perspective_content → source.title → ''
  // We include source.title so direct-source entries (no perspective,
  // no clone content) render the source's canonical title (e.g.
  // "Cloud Security") instead of showing as empty.
  const rows = db
    .query(
      `SELECT
        re.id AS entry_id,
        re.content AS entry_content,
        p.content AS perspective_content,
        s.title AS source_title,
        COALESCE(re.source_id, bs.source_id) AS source_id,
        se.education_type,
        o.name AS institution,
        se.field,
        se.end_date,
        se.degree_level,
        se.degree_type,
        se.gpa,
        COALESCE(se.location,
          CASE
            WHEN addr.city IS NOT NULL AND addr.state IS NOT NULL THEN addr.city || ', ' || addr.state
            WHEN addr.city IS NOT NULL THEN addr.city
            WHEN addr.state IS NOT NULL THEN addr.state
            ELSE NULL
          END
        ) AS location,
        se.credential_id,
        o.name AS issuing_body,
        se.certificate_subtype,
        se.edu_description,
        se.organization_id,
        ol.name AS campus_name,
        addr.city AS campus_city,
        addr.state AS campus_state
      FROM resume_entries re
      LEFT JOIN perspectives p ON p.id = re.perspective_id
      LEFT JOIN bullet_sources bs ON bs.bullet_id = p.bullet_id AND bs.is_primary = 1
      LEFT JOIN sources s ON s.id = COALESCE(re.source_id, bs.source_id)
      LEFT JOIN source_education se ON se.source_id = s.id
      LEFT JOIN organizations o ON o.id = se.organization_id
      LEFT JOIN org_locations ol ON ol.id = se.campus_id
      LEFT JOIN addresses addr ON addr.id = ol.address_id
      WHERE re.section_id = ?
      -- Sort most recent first, then by user-defined position as a tiebreaker.
      -- NULL end_date (in-progress degrees) sorts first since they're implicitly "current".
      ORDER BY CASE WHEN se.end_date IS NULL THEN 1 ELSE 0 END DESC,
               se.end_date DESC,
               re.position ASC`
    )
    .all(sectionId) as Array<{
      entry_id: string
      entry_content: string | null
      perspective_content: string | null
      source_title: string | null
      source_id: string | null
      education_type: string | null
      institution: string | null
      field: string | null
      end_date: string | null
      degree_level: string | null
      degree_type: string | null
      gpa: string | null
      location: string | null
      credential_id: string | null
      issuing_body: string | null
      certificate_subtype: string | null
      edu_description: string | null
      organization_id: string | null
      campus_name: string | null
      campus_city: string | null
      campus_state: string | null
    }>

  return rows.map(row => ({
    kind: 'education' as const,
    institution: row.institution ?? 'Unknown',
    degree: row.entry_content ?? row.perspective_content ?? row.source_title ?? '',
    date: row.end_date ? new Date(row.end_date).getFullYear().toString() : '',
    entry_id: row.entry_id,
    source_id: row.source_id,
    education_type: row.education_type ?? undefined,
    degree_level: row.degree_level,
    degree_type: row.degree_type,
    field: row.field ?? null,
    gpa: row.gpa,
    location: row.location,
    credential_id: row.credential_id,
    issuing_body: row.issuing_body,
    certificate_subtype: row.certificate_subtype,
    edu_description: row.edu_description,
    campus_name: row.campus_name ?? null,
    campus_city: row.campus_city ?? null,
    campus_state: row.campus_state ?? null,
  }))
}

function buildProjectItems(db: Database, sectionId: string): ProjectItem[] {
  // Dual-path source resolution: see buildEducationItems for rationale.
  // source_description carries the project description for display when
  // there are no derived bullets (direct-source project entries).
  const rows = db
    .query(
      `SELECT
        re.id AS entry_id,
        re.content AS entry_content,
        re.perspective_id,
        p.content AS perspective_content,
        p.bullet_id,
        b.content AS bullet_content,
        COALESCE(re.source_id, bs.source_id) AS source_id,
        s.title AS source_title,
        s.description AS source_description,
        sp.start_date,
        sp.end_date
      FROM resume_entries re
      LEFT JOIN perspectives p ON p.id = re.perspective_id
      LEFT JOIN bullets b ON b.id = p.bullet_id
      -- For projects section: prefer project-type sources over role sources
      -- when a bullet is linked to both. Falls back to is_primary if no
      -- project source exists.
      LEFT JOIN bullet_sources bs ON bs.bullet_id = p.bullet_id AND bs.source_id = (
        SELECT bs2.source_id FROM bullet_sources bs2
        JOIN sources s2 ON s2.id = bs2.source_id
        WHERE bs2.bullet_id = p.bullet_id
        ORDER BY CASE WHEN s2.source_type = 'project' THEN 0 ELSE 1 END,
                 bs2.is_primary DESC
        LIMIT 1
      )
      LEFT JOIN sources s ON s.id = COALESCE(re.source_id, bs.source_id)
      LEFT JOIN source_projects sp ON sp.source_id = s.id
      WHERE re.section_id = ?
      ORDER BY re.position ASC`
    )
    .all(sectionId) as Array<{
      entry_id: string
      entry_content: string | null
      perspective_id: string | null
      perspective_content: string | null
      bullet_id: string | null
      bullet_content: string | null
      source_id: string | null
      source_title: string | null
      source_description: string | null
      start_date: string | null
      end_date: string | null
    }>

  // Group by source_title (project name). Direct-source entries without a
  // linked source get bucketed into a synthetic untitled group per entry so
  // they don't collide with each other.
  const projectMap = new Map<string, typeof rows>()
  for (const row of rows) {
    const key = row.source_title ?? `untitled:${row.source_id ?? row.entry_id}`
    if (!projectMap.has(key)) projectMap.set(key, [])
    projectMap.get(key)!.push(row)
  }

  return Array.from(projectMap.entries()).map(([name, entries]) => {
    const realBullets = entries.map(e => {
      const hasChain =
        e.perspective_id !== null &&
        e.bullet_id !== null &&
        e.source_id !== null &&
        e.source_title !== null
      const content = e.entry_content ?? e.perspective_content ?? e.bullet_content ?? ''
      return {
        content,
        entry_id: e.entry_id,
        source_chain: hasChain
          ? {
              source_id: e.source_id!,
              source_title: truncate(e.source_title!, 60),
              bullet_id: e.bullet_id!,
              bullet_preview: truncate(e.bullet_content ?? '', 60),
              perspective_id: e.perspective_id!,
              perspective_preview: truncate(e.perspective_content ?? '', 60),
            }
          : undefined,
        is_cloned: e.entry_content !== null,
      }
    }).filter(b => b.content !== '')

    // When no real bullets exist, split description paragraphs into
    // individual bullet items so they render as separate lines in the
    // DnD editor and PDF output.
    const bullets = realBullets.length > 0
      ? realBullets
      : splitDescriptionIntoBullets(entries[0].source_description, entries[0].entry_id)

    return {
      kind: 'project' as const,
      name: name.startsWith('untitled:') ? 'Untitled Project' : name,
      description: realBullets.length > 0 ? null : entries[0].source_description,
      date: entries[0].end_date ? new Date(entries[0].end_date).getFullYear().toString() : null,
      entry_id: entries[0].entry_id,
      source_id: entries[0].source_id,
      bullets,
    }
  })
}

/**
 * Format a credential details `level` value for resume display.
 *
 * The schema stores normalized DB-friendly tokens ('top_secret', 'secret',
 * etc.), but resumes use industry-standard abbreviations. `top_secret`
 * renders as "TS/SCI" because in IC/DoD contexts Top Secret clearances
 * carry SCI compartment access by default; users who want a different
 * display can set a custom `label` on the credential row.
 */
export function formatClearanceLevel(level: string | null): string {
  if (!level) return ''
  const labels: Record<string, string> = {
    public: 'Public Trust',
    confidential: 'Confidential',
    secret: 'Secret',
    top_secret: 'TS/SCI',
    q: 'DOE Q',
    l: 'DOE L',
  }
  return labels[level] ?? level
}

/**
 * Format a credential details `polygraph` value for resume display.
 * Returns null when the polygraph should be omitted entirely (none/null).
 */
export function formatPolygraph(polygraph: string | null): string | null {
  if (!polygraph || polygraph === 'none') return null
  const labels: Record<string, string> = {
    ci: 'CI Poly',
    full_scope: 'Full-Scope Poly',
  }
  return labels[polygraph] ?? polygraph
}

function buildClearanceItems(db: Database, _sectionId: string): ClearanceItem[] {
  // As of migration 037 (Phase 84, Qualifications track), clearance is no
  // longer a source_type. It lives in the `credentials` table with
  // `credential_type = 'clearance'`. The clearance resume section now pulls
  // from credentials directly — there's no resume_entries row tying a
  // resume to a specific credential; every clearance credential renders
  // automatically into any resume that contains a clearance section.
  //
  // Phase 88 will refine the UX around credential ↔ resume association
  // (e.g., letting users hide specific clearances per-resume). For now,
  // this reads ALL clearance credentials and renders them in
  // status/label order.
  type CredentialRow = {
    id: string
    label: string
    status: string
    details: string
  }

  const rows = db
    .query(
      `SELECT id, label, status, details
       FROM credentials
       WHERE credential_type = 'clearance'
       ORDER BY
         CASE status WHEN 'active' THEN 0 WHEN 'inactive' THEN 1 ELSE 2 END,
         label ASC`,
    )
    .all() as CredentialRow[]

  return rows.map((row): ClearanceItem => {
    let details: { level?: string; polygraph?: string | null; access_programs?: string[] } = {}
    try {
      details = JSON.parse(row.details)
    } catch {
      // Malformed details JSON — fall back to label only.
    }

    // Prefer the user-provided label (e.g. "Top Secret / SCI") if set;
    // otherwise synthesize from the structured details fields using the
    // same human-readable formatters as pre-037 resumes (TS/SCI, CI Poly,
    // etc.).
    //
    // Display conventions:
    //   level: 'top_secret' → 'TS/SCI' (IC default), 'secret' → 'Secret', etc.
    //   polygraph: 'ci' → 'CI Poly', 'full_scope' → 'Full-Scope Poly', 'none' → omitted
    //   status: 'active' is the default so it's not displayed; 'inactive' gets an "(Inactive)" suffix.
    let content = row.label
    if (!content && details.level) {
      content = formatClearanceLevel(details.level)
      const polyLabel = formatPolygraph(details.polygraph ?? null)
      if (polyLabel) content += ` with ${polyLabel}`
    }
    if (row.status === 'inactive') content += ' (Inactive)'

    return {
      kind: 'clearance' as const,
      content,
      entry_id: row.id, // credential id — lets the renderer trace back
      source_id: null, // no source chain — credentials aren't sources
    }
  })
}

function buildPresentationItems(db: Database, sectionId: string): PresentationItem[] {
  // Dual-path source resolution: see buildEducationItems for rationale.
  const rows = db
    .query(
      `SELECT
        re.id AS entry_id,
        re.content AS entry_content,
        re.perspective_id,
        p.content AS perspective_content,
        p.bullet_id,
        b.content AS bullet_content,
        COALESCE(re.source_id, bs.source_id) AS source_id,
        s.title AS source_title,
        s.description AS source_description,
        s.end_date,
        sp.venue,
        sp.presentation_type,
        sp.url AS presentation_url,
        sp.coauthors
      FROM resume_entries re
      LEFT JOIN perspectives p ON p.id = re.perspective_id
      LEFT JOIN bullets b ON b.id = p.bullet_id
      LEFT JOIN bullet_sources bs ON bs.bullet_id = p.bullet_id AND bs.is_primary = 1
      LEFT JOIN sources s ON s.id = COALESCE(re.source_id, bs.source_id)
      LEFT JOIN source_presentations sp ON sp.source_id = s.id
      WHERE re.section_id = ?
      ORDER BY re.position ASC`
    )
    .all(sectionId) as Array<{
      entry_id: string
      entry_content: string | null
      perspective_id: string | null
      perspective_content: string | null
      bullet_id: string | null
      bullet_content: string | null
      source_id: string | null
      source_title: string | null
      source_description: string | null
      end_date: string | null
      venue: string | null
      presentation_type: string | null
      presentation_url: string | null
      coauthors: string | null
    }>

  // Group by source_title (presentation name). Direct-source entries
  // without a linked source fall back to per-entry untitled keys.
  const presMap = new Map<string, typeof rows>()
  for (const row of rows) {
    const key = row.source_title ?? `untitled:${row.source_id ?? row.entry_id}`
    if (!presMap.has(key)) presMap.set(key, [])
    presMap.get(key)!.push(row)
  }

  return Array.from(presMap.entries()).map(([title, entries]) => {
    const realBullets = entries.map(e => {
      const hasChain =
        e.perspective_id !== null &&
        e.bullet_id !== null &&
        e.source_id !== null &&
        e.source_title !== null
      const content = e.entry_content ?? e.perspective_content ?? e.bullet_content ?? ''
      return {
        content,
        entry_id: e.entry_id,
        source_chain: hasChain
          ? {
              source_id: e.source_id!,
              source_title: truncate(e.source_title!, 60),
              bullet_id: e.bullet_id!,
              bullet_preview: truncate(e.bullet_content ?? '', 60),
              perspective_id: e.perspective_id!,
              perspective_preview: truncate(e.perspective_content ?? '', 60),
            }
          : undefined,
        is_cloned: e.entry_content !== null,
      }
    }).filter(b => b.content !== '')

    const bullets = realBullets.length > 0
      ? realBullets
      : splitDescriptionIntoBullets(entries[0].source_description, entries[0].entry_id)

    return {
      kind: 'presentation' as const,
      title: title.startsWith('untitled:') ? 'Untitled Presentation' : title,
      description: realBullets.length > 0 ? null : (entries[0].source_description ?? null),
      venue: entries[0].venue ?? null,
      presentation_type: entries[0].presentation_type ?? null,
      url: entries[0].presentation_url ?? null,
      coauthors: entries[0].coauthors ?? null,
      date: entries[0].end_date ? new Date(entries[0].end_date).getFullYear().toString() : null,
      entry_id: entries[0].entry_id,
      source_id: entries[0].source_id,
      bullets,
    }
  })
}

function buildCertificationItems(db: Database, sectionId: string): CertificationGroup[] {
  // As of the cert-rework (resume_certifications junction), certifications
  // are per-resume: only certs explicitly pinned to this resume section
  // via resume_certifications appear here. Grouped by issuer org.

  const rows = db
    .query(
      `SELECT rc.id AS entry_id, c.short_name, c.cert_id,
              o.name AS issuer_name, c.date_earned, c.in_progress
       FROM resume_certifications rc
       JOIN certifications c ON c.id = rc.certification_id
       LEFT JOIN organizations o ON o.id = c.issuer_id
       WHERE rc.section_id = ?
       ORDER BY rc.position ASC`,
    )
    .all(sectionId) as Array<{
      entry_id: string
      short_name: string
      cert_id: string | null
      issuer_name: string | null
      date_earned: string | null
      in_progress: number
    }>

  if (rows.length === 0) return []

  // Group by issuer
  const catMap = new Map<
    string,
    Array<{ name: string; entry_id: string | null; source_id: string | null; in_progress: boolean }>
  >()

  for (const row of rows) {
    const label = row.issuer_name ?? 'Other'
    if (!catMap.has(label)) catMap.set(label, [])

    catMap.get(label)!.push({
      name: row.short_name,
      entry_id: row.entry_id,
      source_id: null,
      in_progress: row.in_progress === 1,
    })
  }

  return [{
    kind: 'certification_group',
    categories: Array.from(catMap.entries()).map(([label, certs]) => ({
      label,
      certs,
    })),
  }]
}

// ── Helpers ───────────────────────────────────────────────────────────

/**
 * Split a source description into individual bullet items.
 *
 * Paragraphs are delimited by blank lines (`\n\n`). Each paragraph
 * becomes a separate bullet so projects and presentations render as
 * individual lines in both the DnD editor and PDF/LaTeX output.
 *
 * Bullets get synthetic entry_ids (deterministic from the parent
 * entry_id + index) so the DnD view can key them stably.
 */
function splitDescriptionIntoBullets(
  description: string | null,
  parentEntryId: string | null,
): ExperienceBullet[] {
  if (!description) return []
  return description
    .split(/\n\n+/)
    .map(p => p.trim())
    .filter(p => p.length > 0)
    .map((content, i) => ({
      content,
      entry_id: parentEntryId ? syntheticUUID(parentEntryId, `desc-${i}`) : null,
      source_chain: undefined,
      is_cloned: false,
    }))
}

/**
 * Generate a deterministic synthetic UUID from a namespace and key.
 * Uses a simple hash to produce a stable ID for DnD addressing.
 */
export function syntheticUUID(namespace: string, key: string): string {
  const input = `${namespace}:${key}`
  let hash = 0
  for (let i = 0; i < input.length; i++) {
    const char = input.charCodeAt(i)
    hash = ((hash << 5) - hash) + char
    hash = hash & hash // Convert to 32-bit int
  }
  const hex = Math.abs(hash).toString(16).padStart(8, '0')
  return `${hex.slice(0, 8)}-${hex.slice(0, 4)}-4${hex.slice(1, 4)}-a${hex.slice(1, 4)}-${hex.slice(0, 12).padEnd(12, '0')}`
}

/**
 * Format a date range for display.
 * Handles: "Mar 2024 - Jul 2025", "Sep 2018 - Present", "2023"
 */
export function formatDateRange(
  startDate: string | null,
  endDate: string | null,
  isCurrent: boolean,
): string {
  const fmt = (d: string) => {
    const date = new Date(d)
    return date.toLocaleDateString('en-US', { month: 'short', year: 'numeric' })
  }

  if (!startDate && !endDate) return ''
  if (!startDate && endDate) return fmt(endDate)
  if (startDate && isCurrent) return `${fmt(startDate)} - Present`
  if (startDate && endDate) return `${fmt(startDate)} - ${fmt(endDate)}`
  if (startDate) return fmt(startDate)
  return ''
}

/**
 * Build the organization display string for experience sections.
 *
 * Prioritizes location over work arrangement:
 *   1. city + state → "Org (City, ST)"
 *   2. city only    → "Org (City)"
 *   3. state only   → "Org (State)"
 *   4. work_arrangement → "Org (Remote)" / "Org (Hybrid)"
 *   5. nothing      → "Org"
 *
 * Examples:
 *   - "Raytheon Intelligence & Space (Arlington, VA)" — HQ location
 *   - "Cisco (Remote)"                               — no location, arrangement
 *   - "Raytheon Intelligence & Space"                 — no location or arrangement
 *   - "Other" (null org name)
 */

export function buildOrgDisplayString(
  orgName: string | null,
  city: string | null,
  state: string | null,
  workArrangement: string | null,
): string {
  const name = orgName ?? 'Other'

  // Location takes precedence over work arrangement
  if (city && state) return `${name} (${city}, ${state})`
  if (city) return `${name} (${city})`
  if (state) return `${name} (${state})`
  if (workArrangement) {
    const label = workArrangement.charAt(0).toUpperCase() + workArrangement.slice(1)
    return `${name} (${label})`
  }
  return name
}

/**
 * Build a location string for a role subheading.
 * Prefers org location city/state, falls back to work arrangement.
 *
 * Examples:
 *   - "Arlington, VA"
 *   - "Remote"
 *   - null (no location or arrangement)
 */
/** Prepend https:// to URLs that lack a protocol prefix. */
function ensureProtocol(url: string | null): string | null {
  if (!url) return null
  if (url.startsWith('http://') || url.startsWith('https://')) return url
  return `https://${url}`
}

export function buildRoleLocationString(
  city: string | null,
  state: string | null,
  workArrangement: string | null,
): string | null {
  if (city && state) return `${city}, ${state}`
  if (city) return city
  if (state) return state
  if (workArrangement) return workArrangement.charAt(0).toUpperCase() + workArrangement.slice(1)
  return null
}
