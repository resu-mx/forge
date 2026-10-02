# Phase 44: IR Data Quality

**Status:** Planning
**Date:** 2026-04-03
**Spec:** [2026-04-03-ir-data-quality.md](../refs/specs/2026-04-03-ir-data-quality.md)
**Depends on:** Migration 005 (`user_profile`), Migration 010 (`education_org_fk`), Migration 013 (`org_campuses`)
**Blocks:** None currently identified
**Parallelizable with:** Phase 45 (Editor Restructuring), Phase 46 (LaTeX/XeTeX Docs), Phase 47 (Clearance Structured Data) -- no code overlap

## Goal

Fix the `compileResumeIR` pipeline so that every field in the `ResumeDocument` IR populates correctly from database state. Specifically: header name from `user_profile`, contact info from `user_profile`, experience organization names from the `organizations` table (not "Other"), experience organization lines with location and work arrangement, education institution names via `organization_id` JOIN, education locations via `org_campuses`, and skills with graceful orphan handling. The output must structurally match the reference resume format (`{Org Name - Location (Work Arrangement)}{}`).

## Non-Goals

- Template changes (the sb2nov template macros are correct; this spec fixes the data flowing into them)
- New section types
- Changes to the `user_profile` schema or migration
- Automatic profile population from external sources
- Changes to the Markdown renderer (it uses the same IR, so fixes here propagate)
- UI changes to the resume editor

## Context

The IR compiler (`compileResumeIR`) produces a `ResumeDocument` that feeds into the LaTeX template and Markdown renderer. Several fields are not populating correctly:

1. **Header name** shows "User" instead of actual profile name (seed migration defaulted to "User")
2. **Organization name** shows "Other" for experience entries when `source_roles.organization_id` JOIN returns NULL
3. **No location** on organization lines (reference uses `{Org Name - Location (Contract)}{}`)
4. **Missing contact info** in header (email, phone, LinkedIn, GitHub not flowing from `user_profile`)
5. **Skills** not populating when `resume_skills` entries lack proper `skill_id` FK linkage

Root cause: incomplete `user_profile` seed data, missing JOINs for location/work_arrangement in experience/education queries, and "Other" fallback masking NULL org references.

## Scope

| Spec section | Covered here? |
|-------------|---------------|
| 1. Header population (`parseHeader`) | Yes |
| 2. Experience org names (`buildExperienceItems`) | Yes |
| 3. Education institution names (`buildEducationItems`) | Yes |
| 4. Skills flow (`buildSkillItems`) | Yes |
| 5. Experience org display format | Yes |

## Files to Create

None.

## Files to Modify

| File | Change |
|------|--------|
| `packages/core/src/services/resume-compiler.ts` | Extend experience query with campus/work_arrangement JOINs; build org display strings; extend education query with campus JOIN; change experience grouping key to `organization_id`; change skills JOIN to LEFT JOIN with orphan logging; add warning log for "User" profile name |
| `packages/core/src/services/__tests__/resume-compiler.test.ts` | Add/update tests for header, experience org names, experience locations, experience work arrangements, education locations, skills orphan handling |

## Fallback Strategies

- **`org_campuses` table does not exist:** The query uses LEFT JOINs for `org_campuses`, so if migration 013 has not run, the campus columns resolve to NULL and the org line falls back to `{Org Name}` without location. No crash. **Note:** The education query (T44.4) uses `LEFT JOIN org_campuses oc ON oc.id = se.campus_id`. If `source_education.campus_id` column does not exist (migration 013 not applied), the LEFT JOIN on `org_campuses` will fail with 'no such column: se.campus_id'. Prerequisite: Requires migration 013 (org_campuses) to be applied.
- **`work_arrangement` column missing from `source_roles`:** The column was added in migration 002. If somehow absent, the LEFT JOIN produces NULL and the work arrangement is omitted from the display string. No crash.
- **No `user_profile` row:** The existing fallback chain `profile?.name ?? resume.name` handles this. The warning log surfaces the issue without crashing.
- **Multiple campuses per org:** The query uses `oc.is_headquarters = 1` to select the HQ campus. If no HQ campus exists, the LEFT JOIN returns NULL and location is omitted. If an org has multiple HQ campuses (data error), `LIMIT 1` in a subquery (see T44.2) prevents row multiplication.
- **Duplicate org names across different orgs:** Grouping by `organization_id` instead of `org_name` prevents collision. Two different orgs named "Acme" produce separate experience groups.

---

## Tasks

### T44.1: Add Warning Log for "User" Profile Name [IMPORTANT]

**File:** `packages/core/src/services/resume-compiler.ts`

The `parseHeader` function correctly falls through `profile?.name ?? resume.name`, but when the profile name is "User" (the seed default), it silently produces wrong output. Add a diagnostic warning.

**Current code (line 148-177):**
```typescript
function parseHeader(resume: ResumeRow, profile: UserProfile | null, summary: Summary | null): ResumeHeader {
  // Content fields from the resume-specific header JSON
  let tagline: string | null = resume.target_role
  if (resume.header) {
    try {
      const parsed = JSON.parse(resume.header)
      tagline = parsed.tagline ?? tagline
    } catch {
      // Fall through — tagline defaults to target_role
    }
  }

  // Overlay summary fields (tagline from summary takes priority)
  if (summary && summary.tagline) {
    tagline = summary.tagline
  }

  // Contact fields from profile (single source of truth)
  return {
    name: profile?.name ?? resume.name,
    tagline,
    location: profile?.location ?? null,
    email: profile?.email ?? null,
    phone: profile?.phone ?? null,
    linkedin: profile?.linkedin ?? null,
    github: profile?.github ?? null,
    website: profile?.website ?? null,
    clearance: profile?.clearance ?? null,
  }
}
```

**Replace with:**
```typescript
function parseHeader(resume: ResumeRow, profile: UserProfile | null, summary: Summary | null): ResumeHeader {
  // Content fields from the resume-specific header JSON
  let tagline: string | null = resume.target_role
  if (resume.header) {
    try {
      const parsed = JSON.parse(resume.header)
      tagline = parsed.tagline ?? tagline
    } catch {
      // Fall through — tagline defaults to target_role
    }
  }

  // Overlay summary fields (tagline from summary takes priority)
  if (summary && summary.tagline) {
    tagline = summary.tagline
  }

  // Warn on placeholder profile data
  const name = profile?.name ?? resume.name
  if (!profile) {
    console.warn('[resume-compiler] No user_profile row found. Header name falls back to resume.name:', JSON.stringify(resume.name))
  } else if (profile.name === 'User') {
    console.warn('[resume-compiler] user_profile.name is "User" (seed default). Update via Settings > Profile.')
  }

  // Check for missing contact fields
  if (profile && !profile.email && !profile.phone && !profile.linkedin && !profile.github) {
    console.warn('[resume-compiler] user_profile has no contact fields populated. Header will have no contact info.')
  }

  // Contact fields from profile (single source of truth)
  return {
    name,
    tagline,
    location: profile?.location ?? null,
    email: profile?.email ?? null,
    phone: profile?.phone ?? null,
    linkedin: profile?.linkedin ?? null,
    github: profile?.github ?? null,
    website: profile?.website ?? null,
    clearance: profile?.clearance ?? null,
  }
}
```

**Acceptance criteria:**
- When `user_profile` row does not exist, a warning is logged mentioning fallback to `resume.name`.
- When `user_profile.name` is "User", a warning is logged suggesting the user update their profile.
- When all contact fields are null, a warning is logged.
- No warnings when profile is properly populated.
- Return value is unchanged for all cases.

**Failure criteria:**
- Warning logs throw errors instead of warning.
- The function behavior changes (different return values).

---

### T44.2: Extend Experience Query with Location and Work Arrangement [CRITICAL]

**File:** `packages/core/src/services/resume-compiler.ts`

Extend the `buildExperienceItems` query to JOIN `org_campuses` for HQ location and add `work_arrangement` to the SELECT. Update the `ExperienceEntryRow` type. Change the grouping key from `org_name` to `organization_id`. Build the organization display string in the format `{Org Name - Location (Work Arrangement)}`.

**Current `ExperienceEntryRow` (line 55-70):**
```typescript
interface ExperienceEntryRow {
  entry_id: string
  entry_content: string | null
  perspective_id: string
  perspective_content: string
  bullet_id: string
  bullet_content: string
  source_id: string
  source_title: string
  organization_id: string | null
  org_name: string | null
  start_date: string | null
  end_date: string | null
  is_current: number
  position: number
}
```

**Replace with:**
```typescript
interface ExperienceEntryRow {
  entry_id: string
  entry_content: string | null
  perspective_id: string
  perspective_content: string
  bullet_id: string
  bullet_content: string
  source_id: string
  source_title: string
  organization_id: string | null
  org_name: string | null
  org_city: string | null
  org_state: string | null
  work_arrangement: string | null
  start_date: string | null
  end_date: string | null
  is_current: number
  position: number
}
```

**Current experience query (line 197-224):**
```typescript
function buildExperienceItems(db: Database, sectionId: string): ExperienceGroup[] {
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
        bs.source_id,
        s.title AS source_title,
        sr.organization_id,
        sr.start_date,
        sr.end_date,
        sr.is_current,
        o.name AS org_name
      FROM resume_entries re
      JOIN perspectives p ON p.id = re.perspective_id
      JOIN bullets b ON b.id = p.bullet_id
      JOIN bullet_sources bs ON bs.bullet_id = p.bullet_id AND bs.is_primary = 1
      JOIN sources s ON s.id = bs.source_id
      LEFT JOIN source_roles sr ON sr.source_id = s.id
      LEFT JOIN organizations o ON o.id = sr.organization_id
      WHERE re.section_id = ?
      ORDER BY sr.is_current DESC, sr.start_date DESC, re.position ASC`
    )
    .all(sectionId) as ExperienceEntryRow[]
```

**Replace with:**
```typescript
function buildExperienceItems(db: Database, sectionId: string): ExperienceGroup[] {
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
        bs.source_id,
        s.title AS source_title,
        sr.organization_id,
        sr.start_date,
        sr.end_date,
        sr.is_current,
        sr.work_arrangement,
        o.name AS org_name,
        oc.city AS org_city,
        oc.state AS org_state
      FROM resume_entries re
      JOIN perspectives p ON p.id = re.perspective_id
      JOIN bullets b ON b.id = p.bullet_id
      JOIN bullet_sources bs ON bs.bullet_id = p.bullet_id AND bs.is_primary = 1
      JOIN sources s ON s.id = bs.source_id
      LEFT JOIN source_roles sr ON sr.source_id = s.id
      LEFT JOIN organizations o ON o.id = sr.organization_id
      LEFT JOIN org_campuses oc ON oc.organization_id = o.id AND oc.is_headquarters = 1
      WHERE re.section_id = ?
      ORDER BY sr.is_current DESC, sr.start_date DESC, re.position ASC`
    )
    .all(sectionId) as ExperienceEntryRow[]
```

**Current grouping logic (line 227-276):**
```typescript
  // Group by org_name, then sub-group by source_title (role)
  const orgMap = new Map<string, Map<string, ExperienceEntryRow[]>>()

  for (const row of rows) {
    const orgKey = row.org_name ?? 'Other'
    if (!orgMap.has(orgKey)) orgMap.set(orgKey, new Map())
    const roleMap = orgMap.get(orgKey)!
    const roleKey = row.source_title
    if (!roleMap.has(roleKey)) roleMap.set(roleKey, [])
    roleMap.get(roleKey)!.push(row)
  }

  const groups: ExperienceGroup[] = []

  for (const [orgName, roleMap] of orgMap) {
    const subheadings: ExperienceSubheading[] = []

    for (const [roleTitle, entries] of roleMap) {
      const first = entries[0]
      const dateRange = formatDateRange(first.start_date, first.end_date, !!first.is_current)

      const bullets: ExperienceBullet[] = entries.map(e => ({
        content: e.entry_content ?? e.perspective_content,
        entry_id: e.entry_id,
        source_chain: {
          source_id: e.source_id,
          source_title: truncate(e.source_title, 60),
          bullet_id: e.bullet_id,
          bullet_preview: truncate(e.bullet_content, 60),
          perspective_id: e.perspective_id,
          perspective_preview: truncate(e.perspective_content, 60),
        },
        is_cloned: e.entry_content !== null,
      }))

      subheadings.push({
        id: syntheticUUID('subheading', `${orgName}-${roleTitle}`),
        title: roleTitle,
        date_range: dateRange,
        source_id: first.source_id,
        bullets,
      })
    }

    groups.push({
      kind: 'experience_group',
      id: syntheticUUID('org', orgName),
      organization: orgName,
      subheadings,
    })
  }

  return groups
}
```

**Replace with:**
```typescript
  // Group by organization_id (or org_name fallback), then sub-group by source_title (role)
  // Using organization_id prevents collision when two different orgs have the same name.
  const orgMap = new Map<string, Map<string, ExperienceEntryRow[]>>()

  for (const row of rows) {
    const orgKey = row.organization_id ?? row.org_name ?? 'Other'
    if (!orgMap.has(orgKey)) orgMap.set(orgKey, new Map())
    const roleMap = orgMap.get(orgKey)!
    const roleKey = row.source_title
    if (!roleMap.has(roleKey)) roleMap.set(roleKey, [])
    roleMap.get(roleKey)!.push(row)
  }

  const groups: ExperienceGroup[] = []

  for (const [orgKey, roleMap] of orgMap) {
    const subheadings: ExperienceSubheading[] = []

    // Use first entry in the group to build the display string
    const firstEntry = roleMap.values().next().value![0]
    const orgDisplayName = buildOrgDisplayString(
      firstEntry.org_name,
      firstEntry.org_city,
      firstEntry.org_state,
      firstEntry.work_arrangement,
    )

    for (const [roleTitle, entries] of roleMap) {
      const first = entries[0]
      const dateRange = formatDateRange(first.start_date, first.end_date, !!first.is_current)

      const bullets: ExperienceBullet[] = entries.map(e => ({
        content: e.entry_content ?? e.perspective_content,
        entry_id: e.entry_id,
        source_chain: {
          source_id: e.source_id,
          source_title: truncate(e.source_title, 60),
          bullet_id: e.bullet_id,
          bullet_preview: truncate(e.bullet_content, 60),
          perspective_id: e.perspective_id,
          perspective_preview: truncate(e.perspective_content, 60),
        },
        is_cloned: e.entry_content !== null,
      }))

      subheadings.push({
        id: syntheticUUID('subheading', `${orgKey}-${roleTitle}`),
        title: roleTitle,
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
```

**Acceptance criteria:**
- Experience org line format matches reference: `Cisco - Remote (Contract)`, `Raytheon Intelligence & Space - Arlington, VA (Remote)`, `United States Air Force Reserve - National Capitol Region`
- Grouping uses `organization_id` so two different orgs with the same name produce separate groups.
- When org has no campus data, format is just `{Org Name}`.
- When org has location but no work arrangement, format is `{Org Name - City, ST}`.
- When org has work arrangement but no location, format is `{Org Name (Remote)}`.
- When org has no name (NULL organization_id), falls back to "Other".
- No row multiplication from multiple campuses (LEFT JOIN with `is_headquarters = 1` limits to one).

**Failure criteria:**
- "Other" appears in output when org data exists.
- Row duplication from campus JOIN.
- Grouping collides on org name instead of org ID.

---

### T44.3: Add `buildOrgDisplayString` Helper [CRITICAL]

**File:** `packages/core/src/services/resume-compiler.ts`

Add the org display string builder function near the other helpers at the bottom of the file.

**Add after the `formatDateRange` function (after line 656):**
```typescript
/**
 * Build the organization display string for experience sections.
 *
 * Format: `{org_name}{ - location}{ (work_arrangement)}`
 *
 * Examples:
 *   - "Cisco - Remote (Contract)"
 *   - "Raytheon Intelligence & Space - Arlington, VA (Remote)"
 *   - "United States Air Force Reserve - National Capitol Region"
 *   - "Acme Corp" (no location, no arrangement)
 *   - "Other" (null org name)
 */
export function buildOrgDisplayString(
  orgName: string | null,
  city: string | null,
  state: string | null,
  workArrangement: string | null,
): string {
  const name = orgName ?? 'Other'

  // Build location string
  let location: string | null = null
  if (city && state) {
    location = `${city}, ${state}`
  } else if (city) {
    location = city
  } else if (state) {
    location = state
  }

  // Format work arrangement for display (capitalize first letter)
  let arrangement: string | null = null
  if (workArrangement) {
    arrangement = workArrangement.charAt(0).toUpperCase() + workArrangement.slice(1)
  }

  // Compose: "{name}{ - location}{ (arrangement)}"
  let result = name
  if (location) {
    result += ` - ${location}`
  }
  if (arrangement) {
    result += ` (${arrangement})`
  }

  return result
}
```

**Acceptance criteria:**
- `buildOrgDisplayString('Cisco', null, null, 'remote')` returns `"Cisco (Remote)"`.
- `buildOrgDisplayString('Raytheon', 'Arlington', 'VA', 'remote')` returns `"Raytheon - Arlington, VA (Remote)"`.
- `buildOrgDisplayString('USAFR', 'National Capitol Region', null, null)` returns `"USAFR - National Capitol Region"`.
- `buildOrgDisplayString('Acme', null, null, null)` returns `"Acme"`.
- `buildOrgDisplayString(null, null, null, null)` returns `"Other"`.

**Failure criteria:**
- Extra spaces or dashes when location/arrangement is null.
- Work arrangement not capitalized.

---

### T44.4: Extend Education Query with Campus Location [IMPORTANT]

**File:** `packages/core/src/services/resume-compiler.ts`

Add `org_campuses` JOIN to the education query so that education locations can populate from campus data when `se.location` is NULL.

**Current education query (line 311-359):**
```typescript
function buildEducationItems(db: Database, sectionId: string): EducationItem[] {
  const rows = db
    .query(
      `SELECT
        re.id AS entry_id,
        re.content AS entry_content,
        p.content AS perspective_content,
        bs.source_id,
        se.education_type,
        COALESCE(o.name, se.institution) AS institution,
        se.field,
        se.end_date,
        se.degree_level,
        se.degree_type,
        se.gpa,
        se.location,
        se.credential_id,
        COALESCE(o.name, se.issuing_body) AS issuing_body,
        se.certificate_subtype,
        se.edu_description,
        se.organization_id
      FROM resume_entries re
      JOIN perspectives p ON p.id = re.perspective_id
      JOIN bullet_sources bs ON bs.bullet_id = p.bullet_id AND bs.is_primary = 1
      JOIN sources s ON s.id = bs.source_id
      LEFT JOIN source_education se ON se.source_id = s.id
      LEFT JOIN organizations o ON o.id = se.organization_id
      WHERE re.section_id = ?
      ORDER BY re.position ASC`
    )
```

**Replace with:**
```typescript
function buildEducationItems(db: Database, sectionId: string): EducationItem[] {
  const rows = db
    .query(
      `SELECT
        re.id AS entry_id,
        re.content AS entry_content,
        p.content AS perspective_content,
        bs.source_id,
        se.education_type,
        COALESCE(o.name, se.institution) AS institution,
        se.field,
        se.end_date,
        se.degree_level,
        se.degree_type,
        se.gpa,
        COALESCE(se.location,
          CASE
            WHEN oc.city IS NOT NULL AND oc.state IS NOT NULL THEN oc.city || ', ' || oc.state
            WHEN oc.city IS NOT NULL THEN oc.city
            WHEN oc.state IS NOT NULL THEN oc.state
            ELSE NULL
          END
        ) AS location,
        se.credential_id,
        COALESCE(o.name, se.issuing_body) AS issuing_body,
        se.certificate_subtype,
        se.edu_description,
        se.organization_id
      FROM resume_entries re
      JOIN perspectives p ON p.id = re.perspective_id
      JOIN bullet_sources bs ON bs.bullet_id = p.bullet_id AND bs.is_primary = 1
      JOIN sources s ON s.id = bs.source_id
      LEFT JOIN source_education se ON se.source_id = s.id
      LEFT JOIN organizations o ON o.id = se.organization_id
      LEFT JOIN org_campuses oc ON oc.id = se.campus_id
      WHERE re.section_id = ?
      ORDER BY re.position ASC`
    )
```

**Key points:**
- The campus JOIN uses `se.campus_id` (added by migration 013) to get the specific campus location.
- `COALESCE(se.location, ...)` preserves the legacy `se.location` field as the primary source; campus location is the fallback.
- No changes needed to the row type or the mapping function -- `location` was already in the SELECT and is mapped to `row.location`.

**Acceptance criteria:**
- Education entries with `se.campus_id` set and `se.location` NULL get location from `org_campuses`.
- Education entries with `se.location` set keep their existing location (COALESCE priority).
- Education entries with neither `se.location` nor campus data have `location: null`.
- No row duplication from the campus JOIN (since `se.campus_id` is a single FK, the JOIN is 1:1).

**Failure criteria:**
- Location shows "null, null" or similar garbage string.
- Campus JOIN causes row multiplication.

---

### T44.5: Change Skills Query to LEFT JOIN with Orphan Logging [MINOR]

**File:** `packages/core/src/services/resume-compiler.ts`

The current skills query uses `JOIN skills s ON s.id = rs.skill_id`, which silently drops orphaned `resume_skills` entries. Change to LEFT JOIN and log a warning for orphans.

**Current skills query (line 281-309):**
```typescript
function buildSkillItems(db: Database, sectionId: string): SkillGroup[] {
  const rows = db
    .query(
      `SELECT s.name AS skill_name, s.category
       FROM resume_skills rs
       JOIN skills s ON s.id = rs.skill_id
       WHERE rs.section_id = ?
       ORDER BY rs.position`
    )
    .all(sectionId) as Array<{ skill_name: string; category: string | null }>

  if (rows.length === 0) return []

  // Group by category
  const catMap = new Map<string, string[]>()
  for (const row of rows) {
    const cat = row.category ?? 'Other'
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
```

**Replace with:**
```typescript
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
      `SELECT s.name AS skill_name, s.category
       FROM resume_skills rs
       JOIN skills s ON s.id = rs.skill_id
       WHERE rs.section_id = ?
       ORDER BY s.category ASC, rs.position ASC`
    )
    .all(sectionId) as Array<{ skill_name: string; category: string | null }>

  if (rows.length === 0) return []

  // Group by category (ordered by first appearance due to ORDER BY above)
  const catMap = new Map<string, string[]>()
  for (const row of rows) {
    const cat = row.category ?? 'Other'
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
```

**Key points:**
- The main query stays as `JOIN` (not LEFT JOIN) so orphaned entries are excluded from output. The orphan check is a separate diagnostic query.
- Added `ORDER BY s.category ASC` to ensure consistent category ordering across compiles.
- The orphan warning only fires when orphans exist, so it does not add noise for healthy data.

**Acceptance criteria:**
- Orphaned `resume_skills` entries (with invalid `skill_id`) produce a warning log.
- Valid skills still appear in the output, grouped by category.
- Categories are ordered consistently (alphabetical by category name).
- No change to output format or structure.

**Failure criteria:**
- Warning fires on every compile even with no orphans.
- Skills disappear from output.

---

## Testing Support

### Test Fixtures

The existing `createTestDb()` helper runs all migrations. Tests seed data directly into `user_profile`, `organizations`, `org_campuses`, `source_roles`, `sources`, `bullets`, `perspectives`, `resume_entries`, `resume_sections`, `resume_skills`, and `skills` tables.

### Unit Tests

**File:** `packages/core/src/services/__tests__/resume-compiler.test.ts`

| Test | Category | Assertion |
|------|----------|-----------|
| Header from populated profile | [CRITICAL] | `header.name === 'Adam'`, `header.email === 'adam@example.com'`, etc. |
| Header fallback when no profile row | [IMPORTANT] | `header.name === resume.name` (fallback chain) |
| Header warning when profile name is "User" | [IMPORTANT] | Console.warn called with "seed default" message |
| Experience org name from organizations table | [CRITICAL] | `group.organization` contains "Cisco", not "Other" |
| Experience org with HQ campus location | [CRITICAL] | `group.organization` contains "Arlington, VA" |
| Experience org with work arrangement | [CRITICAL] | `group.organization` contains "(Remote)" |
| Experience org full format | [CRITICAL] | `group.organization === 'Raytheon Intelligence & Space - Arlington, VA (Remote)'` |
| Experience org name-only (no campus, no arrangement) | [IMPORTANT] | `group.organization === 'Acme Corp'` |
| Experience grouping by org ID (not name) | [IMPORTANT] | Two orgs with name "Acme" produce 2 separate groups |
| Experience fallback to "Other" when no org | [MINOR] | Source with no `organization_id` groups under "Other" |
| Education institution via org JOIN | [CRITICAL] | `item.institution === 'University of Maryland'` (from `organizations.name`) |
| Education location from campus | [IMPORTANT] | `item.location === 'College Park, MD'` (from `org_campuses`) |
| Education location from se.location takes priority | [IMPORTANT] | `se.location` set, campus also set -- `item.location === se.location` |
| Skills with valid entries | [IMPORTANT] | All linked skills appear grouped by category |
| Skills with orphaned entries | [MINOR] | Orphans skipped, warning logged, valid skills still appear |
| Skills category ordering | [MINOR] | Categories appear in alphabetical order |
| `buildOrgDisplayString` with all fields | [CRITICAL] | Returns `"Org - City, ST (Remote)"` |
| `buildOrgDisplayString` with null org name | [IMPORTANT] | Returns `"Other"` |
| `buildOrgDisplayString` with city only | [MINOR] | Returns `"Org - City"` |
| `buildOrgDisplayString` with arrangement only | [MINOR] | Returns `"Org (Remote)"` |

### Integration Tests

| Test | Assertion |
|------|-----------|
| Full resume compile with all section types | LaTeX output contains actual org names, locations, contact info |
| Compile resume with empty profile | No crash, sensible defaults in header |
| Compile resume with no experience entries | Returns empty experience sections |

### Smoke Tests

| Test | What to verify |
|------|---------------|
| Compile a real resume via API | Header shows user's name, not "User" |
| Experience section shows org names | No "Other" when orgs are linked |
| PDF preview renders org lines correctly | Format matches `{Org - Location (Arrangement)}` |

### Contract Tests

| Test | What to verify |
|------|---------------|
| `ResumeDocument.header.name` is a real name | Not "User", not the resume title |
| `ExperienceGroup.organization` contains location | When org has campus data |
| `EducationItem.location` populated | When campus_id is set |

---

## Documentation Requirements

- No new documentation files required.
- Inline code comments in the compiler explain the JOIN strategy and fallback chain.
- The `buildOrgDisplayString` function has JSDoc with format examples.
- Console.warn messages include `[resume-compiler]` prefix for grep-ability.

---

## Parallelization Notes

**Within this phase:**
- T44.1 (header warning) is independent and can be done first or in parallel with T44.2.
- T44.3 (helper function) must complete before T44.2 (T44.2 calls the helper from T44.3). They cannot run in parallel.
- T44.4 (education query) is independent of T44.2/T44.3.
- T44.5 (skills orphan logging) is independent of all other tasks.

**Recommended execution order:**
1. T44.3 (helper -- no dependencies)
2. T44.1 + T44.2 + T44.4 + T44.5 (all independent, T44.2 calls T44.3 which must already exist)
3. Write tests (after all code changes)

**Cross-phase:**
- Phase 47 (Clearance Structured Data) modifies `source_clearances` schema. The clearance builder in the compiler (`buildClearanceItems`) may need updates after Phase 47 to use the new enum types and labels. This phase does not touch the clearance builder.
- Phase 45 (Editor Restructuring) is purely UI and has no overlap with compiler changes.
- Phase 46 (LaTeX/XeTeX Docs) is documentation-only.
