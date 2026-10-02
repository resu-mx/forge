# Phase 19: Resume IR & Compilers

**Goal:** Build the resume IR type system, escape utilities, IR compiler, and format compilers (Markdown + LaTeX). This is the core rendering engine -- no UI yet. By the end of this phase, the system can take a resume ID and produce a `ResumeDocument` IR, a Markdown string, a LaTeX string, and a PDF binary.

**Non-Goals:** DragNDrop view, Markdown/LaTeX editors with CodeMirror, split-view preview, override editing UI. Those belong to Phase 20 (UI). This phase is pure server-side logic + API endpoints.

**Depends on:** Phase 16 (migration adds `header`, `markdown_override`, `latex_override` columns to `resumes`)
**Blocks:** Phase 20 (UI needs compilers and API endpoints from this phase)
**Parallelizable with:** Phase 17, Phase 18

**Internal task parallelization:** After T19.1 (types), T19.2 (escape) and T19.5 (template) can run in parallel. T19.4 (markdown compiler) depends on T19.2. T19.6 (LaTeX compiler) depends on T19.2 + T19.5. T19.7 and T19.8 (linters) can run in parallel with T19.3-T19.6. T19.9-T19.11 (endpoints) depend on T19.3, T19.4, T19.6, T19.7, T19.8.

**Shared file warning:** Phase 19 (T19.9, T19.10) and Phase 17 (T17.4) both modify `packages/core/src/services/resume-service.ts`. Execute sequentially or in separate branches and merge carefully.

**Tech Stack:** TypeScript, SQLite (via `bun:sqlite`), Hono, `bun:test`, `crypto.randomUUID()`

**Reference:** `refs/specs/2026-03-29-resume-renderer-and-entity-updates.md` sections 1.1-1.8

**Architecture:**
- IR types live in `packages/sdk/src/types.ts` (shared between server and WebUI client)
- Escape utilities live in `packages/core/src/lib/escape.ts`
- IR compiler lives in `packages/core/src/services/resume-compiler.ts`
- Format compilers live in `packages/core/src/lib/markdown-compiler.ts` and `packages/core/src/lib/latex-compiler.ts`
- Template implementation lives in `packages/core/src/templates/sb2nov.ts`
- Linters live in `packages/core/src/lib/markdown-linter.ts` and `packages/core/src/lib/latex-linter.ts`
- API routes extend `packages/core/src/routes/resumes.ts`
- SDK methods extend `packages/sdk/src/resources/resumes.ts`

---

## Context

The resume currently stores entries as references to perspectives, grouped by section. There is no rendering pipeline -- the existing `/resumes/:id/export` endpoint returns 501.

This phase builds the full pipeline:

```
Resume + Entries + Perspectives + Bullets + Sources + Organizations
  |  compile (resume-compiler.ts)
  v
ResumeDocument (IR)
  |  render (markdown-compiler.ts or latex-compiler.ts)
  v
Markdown string  or  LaTeX string
  |  (latex only) tectonic --untrusted
  v
PDF binary
```

The IR is the single source of truth. All view modes render from it. The compiler fetches all chain data in a small number of queries and assembles the IR structure.

---

## Tasks

### Task 19.1: IR Type Definitions

**File:** `packages/sdk/src/types.ts`

**Goal:** Add all IR types so both server and client code can reference them.

- [ ] **Add `ResumeDocument` interface** and all sub-types. Place these after the existing `ResumeWithEntries` type, before the Review Queue section:

```typescript
// ---------------------------------------------------------------------------
// Resume IR (Intermediate Representation)
// ---------------------------------------------------------------------------

export interface ResumeDocument {
  resume_id: string
  header: ResumeHeader
  sections: IRSection[]
}

export interface ResumeHeader {
  name: string
  tagline: string | null
  location: string | null
  email: string | null
  phone: string | null
  linkedin: string | null
  github: string | null
  website: string | null
  clearance: string | null
}

export interface IRSection {
  id: string
  type: IRSectionType
  title: string
  display_order: number
  items: IRSectionItem[]
}

export type IRSectionType =
  | 'summary'
  | 'experience'
  | 'skills'
  | 'education'
  | 'projects'
  | 'certifications'
  | 'clearance'
  | 'presentations'
  | 'awards'
  | 'custom'

export type IRSectionItem =
  | SummaryItem
  | ExperienceGroup
  | SkillGroup
  | EducationItem
  | ProjectItem
  | CertificationGroup
  | ClearanceItem
  | PresentationItem

export interface SummaryItem {
  kind: 'summary'
  content: string
  entry_id: string | null
}

export interface ExperienceGroup {
  kind: 'experience_group'
  id: string
  organization: string
  subheadings: ExperienceSubheading[]
}

export interface ExperienceSubheading {
  id: string
  title: string
  date_range: string
  source_id: string | null
  bullets: ExperienceBullet[]
}

export interface ExperienceBullet {
  content: string
  entry_id: string | null
  source_chain?: {
    perspective_id: string
    bullet_id: string
    source_id: string
  }
  is_cloned: boolean
}

export interface SkillGroup {
  kind: 'skill_group'
  categories: Array<{
    label: string
    skills: string[]
  }>
}

export interface EducationItem {
  kind: 'education'
  institution: string
  degree: string
  date: string
  entry_id: string | null
  source_id: string | null
}

export interface ProjectItem {
  kind: 'project'
  name: string
  date: string | null
  entry_id: string | null
  source_id: string | null
  bullets: ExperienceBullet[]
}

export interface CertificationGroup {
  kind: 'certification_group'
  categories: Array<{
    label: string
    certs: Array<{
      name: string
      entry_id: string | null
      source_id: string | null
    }>
  }>
}

export interface ClearanceItem {
  kind: 'clearance'
  content: string
  entry_id: string | null
  source_id: string | null
}

export interface PresentationItem {
  kind: 'presentation'
  title: string
  venue: string
  date: string | null
  entry_id: string | null
  source_id: string | null
  bullets: ExperienceBullet[]
}
```

- [ ] **Add `LatexTemplate` interface:**

```typescript
export interface LatexTemplate {
  preamble: string
  renderHeader: (header: ResumeHeader) => string
  renderSection: (section: IRSection) => string
  renderSectionFallback: (section: IRSection) => string
  footer: string
}
```

- [ ] **Add `LintResult` type:**

```typescript
export type LintResult =
  | { ok: true }
  | { ok: false; errors: string[] }
```

- [ ] **Export all new types from SDK barrel.** Update `packages/sdk/src/index.ts` with explicit type exports:

```typescript
// Resume IR types
export type {
  ResumeDocument,
  ResumeHeader,
  IRSection,
  IRSectionType,
  IRSectionItem,
  SummaryItem,
  ExperienceGroup,
  ExperienceSubheading,
  ExperienceBullet,
  SkillGroup,
  EducationItem,
  ProjectItem,
  CertificationGroup,
  ClearanceItem,
  PresentationItem,
  LatexTemplate,
  LintResult,
} from './types'
```

**Note:** The SDK `ResumesResource` is updated with these methods across T19.9-T19.11:
- `ir(id: string): Promise<Result<ResumeDocument>>` (T19.9)
- `updateHeader(id: string, header: Record<string, unknown>): Promise<Result<Resume>>` (T19.10)
- `updateMarkdownOverride(id: string, content: string | null): Promise<Result<Resume>>` (T19.10)
- `updateLatexOverride(id: string, content: string | null): Promise<Result<Resume>>` (T19.10)
- `pdf(id: string, latex?: string): Promise<Result<Blob>>` (T19.11)

**Acceptance Criteria:**
- [ ] All IR types compile without errors
- [ ] Types are importable from `@forge/sdk` in both server and client contexts
- [ ] `LatexTemplate` and `LintResult` are exported
- [ ] All IR types are listed in `packages/sdk/src/index.ts` explicit exports

---

### Task 19.2: Escape Utilities

**Files:**
- Create: `packages/core/src/lib/escape.ts`
- Create: `packages/core/src/lib/__tests__/escape.test.ts`

**Goal:** Provide `escapeLatex()` and `escapeMarkdown()` functions for safe content insertion into compiled output.

- [ ] **Create `packages/core/src/lib/escape.ts`:**

```typescript
/**
 * Escape utilities for resume format compilers.
 *
 * These functions escape user-authored content before insertion into
 * LaTeX or Markdown output. Template structural strings are NEVER escaped.
 */

/**
 * Escape the 10 LaTeX special characters.
 *
 * Order matters: backslash MUST be escaped LAST to avoid double-escaping
 * the backslashes introduced by earlier replacements. Wait -- actually
 * backslash must be FIRST, because if we escape & to \& first, then
 * escape \ to \textbackslash{}, the \ in \& becomes \textbackslash{}&.
 *
 * Correct order: \ first (to \textbackslash{}), then all others.
 */
export function escapeLatex(text: string): string {
  if (!text) return text

  return text
    .replace(/\\/g, '\\textbackslash{}')
    .replace(/&/g, '\\&')
    .replace(/%/g, '\\%')
    .replace(/\$/g, '\\$')
    .replace(/#/g, '\\#')
    .replace(/_/g, '\\_')
    .replace(/\{/g, '\\{')
    .replace(/\}/g, '\\}')
    .replace(/~/g, '\\textasciitilde{}')
    .replace(/\^/g, '\\textasciicircum{}')
}

/**
 * Escape Markdown special characters in inline positions.
 *
 * Only escapes [, ], and \ -- IR content is plain text with no
 * inline formatting in MVP. Does NOT escape * or _ (they only
 * matter in formatting contexts which IR content doesn't use).
 */
export function escapeMarkdown(text: string): string {
  if (!text) return text

  return text
    .replace(/\\/g, '\\\\')
    .replace(/\[/g, '\\[')
    .replace(/\]/g, '\\]')
}
```

- [ ] **Create comprehensive tests in `packages/core/src/lib/__tests__/escape.test.ts`:**

```typescript
import { describe, test, expect } from 'bun:test'
import { escapeLatex, escapeMarkdown } from '../escape'

describe('escapeLatex', () => {
  test('escapes ampersand', () => {
    expect(escapeLatex('R&D')).toBe('R\\&D')
  })

  test('escapes percent', () => {
    expect(escapeLatex('80% reduction')).toBe('80\\% reduction')
  })

  test('escapes dollar', () => {
    expect(escapeLatex('$100K')).toBe('\\$100K')
  })

  test('escapes hash', () => {
    expect(escapeLatex('issue #42')).toBe('issue \\#42')
  })

  test('escapes underscore', () => {
    expect(escapeLatex('snake_case')).toBe('snake\\_case')
  })

  test('escapes curly braces', () => {
    expect(escapeLatex('{value}')).toBe('\\{value\\}')
  })

  test('escapes tilde', () => {
    expect(escapeLatex('~approx')).toBe('\\textasciitilde{}approx')
  })

  test('escapes caret', () => {
    expect(escapeLatex('2^10')).toBe('2\\textasciicircum{}10')
  })

  test('escapes backslash', () => {
    expect(escapeLatex('path\\to\\file')).toBe('path\\textbackslash{}to\\textbackslash{}file')
  })

  test('handles empty string', () => {
    expect(escapeLatex('')).toBe('')
  })

  test('passes through plain text unchanged', () => {
    expect(escapeLatex('Hello world')).toBe('Hello world')
  })

  test('passes through unicode unchanged', () => {
    expect(escapeLatex('Resume for Rene')).toBe('Resume for Rene')
    expect(escapeLatex('em dash — here')).toBe('em dash — here')
  })

  test('handles multiple special chars in one string', () => {
    expect(escapeLatex('R&D: 80% of $budget'))
      .toBe('R\\&D: 80\\% of \\$budget')
  })

  test('backslash does not double-escape other replacements', () => {
    // Input: "A\B&C" -> backslash first: "A\textbackslash{}B&C" -> then &: "A\textbackslash{}B\&C"
    expect(escapeLatex('A\\B&C')).toBe('A\\textbackslash{}B\\&C')
  })
})

describe('escapeMarkdown', () => {
  test('escapes square brackets', () => {
    expect(escapeMarkdown('[link]')).toBe('\\[link\\]')
  })

  test('escapes backslash', () => {
    expect(escapeMarkdown('path\\to')).toBe('path\\\\to')
  })

  test('does not escape asterisk', () => {
    expect(escapeMarkdown('*bold*')).toBe('*bold*')
  })

  test('does not escape underscore', () => {
    expect(escapeMarkdown('snake_case')).toBe('snake_case')
  })

  test('handles empty string', () => {
    expect(escapeMarkdown('')).toBe('')
  })

  test('passes through unicode unchanged', () => {
    expect(escapeMarkdown('cafe')).toBe('cafe')
  })

  test('handles combined special chars', () => {
    expect(escapeMarkdown('see [docs] at path\\ref'))
      .toBe('see \\[docs\\] at path\\\\ref')
  })
})
```

**Acceptance Criteria:**
- [ ] `escapeLatex` handles all 10 special characters
- [ ] Backslash ordering prevents double-escape
- [ ] `escapeMarkdown` handles `[`, `]`, `\`
- [ ] Both pass through empty strings and unicode
- [ ] All tests pass

---

### Task 19.3: IR Compiler

**Files:**
- Create: `packages/core/src/services/resume-compiler.ts`
- Create: `packages/core/src/services/__tests__/resume-compiler.test.ts`

**Goal:** Build the `compileResumeIR()` function that fetches all chain data and assembles a `ResumeDocument`.

**Note:** The IR compiler queries for `section = 'experience'` (not `'work_history'`). Phase 16 migration step 9 renames all existing `work_history` values to `experience` in `resume_entries.section`, so this is consistent.

- [ ] **Create `packages/core/src/services/resume-compiler.ts`:**

```typescript
/**
 * Resume IR Compiler — assembles a ResumeDocument from database state.
 *
 * Fetches the resume, entries, perspectives, bullets, sources, and orgs
 * in a small number of queries, then groups and structures them into the IR.
 */

import type { Database } from 'bun:sqlite'
import type {
  ResumeDocument,
  ResumeHeader,
  IRSection,
  IRSectionType,
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
} from '@forge/sdk'

// ── Row types for query results ─────────────────────────────────────

interface ResumeRow {
  id: string
  name: string
  target_role: string
  header: string | null  // JSON blob
}

interface ExperienceEntryRow {
  entry_id: string
  entry_content: string | null
  perspective_id: string
  perspective_content: string
  bullet_id: string
  source_id: string
  source_title: string
  organization_id: string | null
  org_name: string | null
  start_date: string | null
  end_date: string | null
  is_current: number
  position: number
}

interface SkillRow {
  category: string | null
  skill_name: string
}

interface GenericEntryRow {
  entry_id: string
  entry_content: string | null
  perspective_id: string
  perspective_content: string
  bullet_id: string
  source_id: string
  section: string
  position: number
}

// ── Section order ────────────────────────────────────────────────────

const DEFAULT_SECTION_ORDER: IRSectionType[] = [
  'summary',
  'experience',
  'skills',
  'education',
  'certifications',
  'clearance',
  'projects',
  'presentations',
  'awards',
  'custom',
]

// ── Compiler ─────────────────────────────────────────────────────────

/**
 * Compile a resume into the IR format.
 *
 * Returns null if the resume does not exist.
 */
export function compileResumeIR(db: Database, resumeId: string): ResumeDocument | null {
  // 1. Fetch resume base data
  const resume = db
    .query('SELECT id, name, target_role, header FROM resumes WHERE id = ?')
    .get(resumeId) as ResumeRow | null

  if (!resume) return null

  // 2. Parse header (JSON blob or build default)
  const header = parseHeader(resume)

  // 3. Build sections from entries
  const sections: IRSection[] = []
  let sectionOrder = 0

  // 3a. Summary section
  const summaryItems = buildSummarySection(db, resumeId)
  if (summaryItems.length > 0) {
    sections.push({
      id: syntheticUUID('summary', resumeId),
      type: 'summary',
      title: 'Summary',
      display_order: sectionOrder++,
      items: summaryItems,
    })
  }

  // 3b. Experience section (grouped by org, sub-grouped by role)
  const experienceGroups = buildExperienceSection(db, resumeId)
  if (experienceGroups.length > 0) {
    sections.push({
      id: syntheticUUID('experience', resumeId),
      type: 'experience',
      title: 'Experience',
      display_order: sectionOrder++,
      items: experienceGroups,
    })
  }

  // 3c. Skills section (derived from bullet_skills of included perspectives)
  const skillGroups = buildSkillsSection(db, resumeId)
  if (skillGroups.length > 0) {
    sections.push({
      id: syntheticUUID('skills', resumeId),
      type: 'skills',
      title: 'Technical Skills',
      display_order: sectionOrder++,
      items: skillGroups,
    })
  }

  // 3d. Education section
  const educationItems = buildEducationSection(db, resumeId)
  if (educationItems.length > 0) {
    sections.push({
      id: syntheticUUID('education', resumeId),
      type: 'education',
      title: 'Education & Certifications',
      display_order: sectionOrder++,
      items: educationItems,
    })
  }

  // 3e. Projects section
  const projectItems = buildProjectsSection(db, resumeId)
  if (projectItems.length > 0) {
    sections.push({
      id: syntheticUUID('projects', resumeId),
      type: 'projects',
      title: 'Selected Projects',
      display_order: sectionOrder++,
      items: projectItems,
    })
  }

  // 3f. Clearance section
  const clearanceItems = buildClearanceSection(db, resumeId)
  if (clearanceItems.length > 0) {
    sections.push({
      id: syntheticUUID('clearance', resumeId),
      type: 'clearance',
      title: 'Security Clearance',
      display_order: sectionOrder++,
      items: clearanceItems,
    })
  }

  // 3g. Presentations section
  const presentationItems = buildPresentationsSection(db, resumeId)
  if (presentationItems.length > 0) {
    sections.push({
      id: syntheticUUID('presentations', resumeId),
      type: 'presentations',
      title: 'Presentations',
      display_order: sectionOrder++,
      items: presentationItems,
    })
  }

  return { resume_id: resumeId, header, sections }
}

// ── Section builders ─────────────────────────────────────────────────

function parseHeader(resume: ResumeRow): ResumeHeader {
  if (resume.header) {
    try {
      return JSON.parse(resume.header) as ResumeHeader
    } catch {
      // Fall through to default
    }
  }
  return {
    name: resume.name,
    tagline: resume.target_role,
    location: null,
    email: null,
    phone: null,
    linkedin: null,
    github: null,
    website: null,
    clearance: null,
  }
}

function buildSummarySection(db: Database, resumeId: string): SummaryItem[] {
  const rows = db
    .query(
      `SELECT re.id AS entry_id, re.content AS entry_content,
              p.content AS perspective_content
       FROM resume_entries re
       JOIN perspectives p ON p.id = re.perspective_id
       WHERE re.resume_id = ? AND re.section = 'summary'
       ORDER BY re.position ASC`
    )
    .all(resumeId) as Array<{ entry_id: string; entry_content: string | null; perspective_content: string }>

  return rows.map(row => ({
    kind: 'summary' as const,
    content: row.entry_content ?? row.perspective_content,
    entry_id: row.entry_id,
  }))
}

function buildExperienceSection(db: Database, resumeId: string): ExperienceGroup[] {
  // The key query: join entries -> perspectives -> bullets -> bullet_sources -> sources -> source_roles -> organizations
  const rows = db
    .query(
      `SELECT
        re.id AS entry_id,
        re.content AS entry_content,
        re.perspective_id,
        re.position,
        p.content AS perspective_content,
        p.bullet_id,
        bs.source_id,
        s.title AS source_title,
        sr.organization_id,
        sr.start_date,
        sr.end_date,
        sr.is_current,
        o.name AS org_name
      FROM resume_entries re
      JOIN perspectives p ON p.id = re.perspective_id
      JOIN bullet_sources bs ON bs.bullet_id = p.bullet_id AND bs.is_primary = 1
      JOIN sources s ON s.id = bs.source_id
      LEFT JOIN source_roles sr ON sr.source_id = s.id
      LEFT JOIN organizations o ON o.id = sr.organization_id
      WHERE re.resume_id = ? AND re.section = 'experience'
      ORDER BY sr.is_current DESC, sr.start_date DESC, re.position ASC`
    )
    .all(resumeId) as ExperienceEntryRow[]

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
          perspective_id: e.perspective_id,
          bullet_id: e.bullet_id,
          source_id: e.source_id,
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

function buildSkillsSection(db: Database, resumeId: string): SkillGroup[] {
  // Derive skills from bullet_skills of included perspectives
  const rows = db
    .query(
      `SELECT DISTINCT sk.category, sk.name AS skill_name
       FROM resume_entries re
       JOIN perspectives p ON p.id = re.perspective_id
       JOIN bullet_skills bsk ON bsk.bullet_id = p.bullet_id
       JOIN skills sk ON sk.id = bsk.skill_id
       WHERE re.resume_id = ?
       ORDER BY sk.category ASC, sk.name ASC`
    )
    .all(resumeId) as SkillRow[]

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

function buildEducationSection(db: Database, resumeId: string): EducationItem[] {
  const rows = db
    .query(
      `SELECT
        re.id AS entry_id,
        re.content AS entry_content,
        p.content AS perspective_content,
        bs.source_id,
        se.institution,
        se.field,
        se.end_date
      FROM resume_entries re
      JOIN perspectives p ON p.id = re.perspective_id
      JOIN bullet_sources bs ON bs.bullet_id = p.bullet_id AND bs.is_primary = 1
      JOIN sources s ON s.id = bs.source_id
      LEFT JOIN source_education se ON se.source_id = s.id
      WHERE re.resume_id = ? AND re.section = 'education'
      ORDER BY re.position ASC`
    )
    .all(resumeId) as Array<{
      entry_id: string
      entry_content: string | null
      perspective_content: string
      source_id: string
      institution: string | null
      field: string | null
      end_date: string | null
    }>

  return rows.map(row => ({
    kind: 'education' as const,
    institution: row.institution ?? 'Unknown',
    degree: row.entry_content ?? row.perspective_content,
    date: row.end_date ? new Date(row.end_date).getFullYear().toString() : '',
    entry_id: row.entry_id,
    source_id: row.source_id,
  }))
}

function buildProjectsSection(db: Database, resumeId: string): ProjectItem[] {
  // Fetch project entries - for now, simple bullet-per-entry model
  const rows = db
    .query(
      `SELECT
        re.id AS entry_id,
        re.content AS entry_content,
        re.perspective_id,
        p.content AS perspective_content,
        p.bullet_id,
        bs.source_id,
        s.title AS source_title,
        sp.start_date,
        sp.end_date
      FROM resume_entries re
      JOIN perspectives p ON p.id = re.perspective_id
      JOIN bullet_sources bs ON bs.bullet_id = p.bullet_id AND bs.is_primary = 1
      JOIN sources s ON s.id = bs.source_id
      LEFT JOIN source_projects sp ON sp.source_id = s.id
      WHERE re.resume_id = ? AND re.section = 'projects'
      ORDER BY re.position ASC`
    )
    .all(resumeId) as Array<{
      entry_id: string
      entry_content: string | null
      perspective_id: string
      perspective_content: string
      bullet_id: string
      source_id: string
      source_title: string
      start_date: string | null
      end_date: string | null
    }>

  // Group by source_title (project name)
  const projectMap = new Map<string, typeof rows>()
  for (const row of rows) {
    if (!projectMap.has(row.source_title)) projectMap.set(row.source_title, [])
    projectMap.get(row.source_title)!.push(row)
  }

  return Array.from(projectMap.entries()).map(([name, entries]) => ({
    kind: 'project' as const,
    name,
    date: entries[0].end_date ? new Date(entries[0].end_date).getFullYear().toString() : null,
    entry_id: entries[0].entry_id,
    source_id: entries[0].source_id,
    bullets: entries.map(e => ({
      content: e.entry_content ?? e.perspective_content,
      entry_id: e.entry_id,
      source_chain: {
        perspective_id: e.perspective_id,
        bullet_id: e.bullet_id,
        source_id: e.source_id,
      },
      is_cloned: e.entry_content !== null,
    })),
  }))
}

function buildClearanceSection(db: Database, resumeId: string): ClearanceItem[] {
  const rows = db
    .query(
      `SELECT
        re.id AS entry_id,
        re.content AS entry_content,
        p.content AS perspective_content,
        bs.source_id,
        sc.level,
        sc.polygraph,
        sc.status AS clearance_status
      FROM resume_entries re
      JOIN perspectives p ON p.id = re.perspective_id
      JOIN bullet_sources bs ON bs.bullet_id = p.bullet_id AND bs.is_primary = 1
      JOIN sources s ON s.id = bs.source_id
      LEFT JOIN source_clearances sc ON sc.source_id = s.id
      WHERE re.resume_id = ? AND re.section = 'clearance'
      ORDER BY re.position ASC`
    )
    .all(resumeId) as Array<{
      entry_id: string
      entry_content: string | null
      perspective_content: string
      source_id: string
      level: string | null
      polygraph: string | null
      clearance_status: string | null
    }>

  return rows.map(row => {
    // Build clearance string from structured data if available
    let content = row.entry_content ?? row.perspective_content
    if (row.level) {
      content = row.level
      if (row.polygraph) content += ` with ${row.polygraph}`
      if (row.clearance_status) content += ` - ${row.clearance_status}`
    }
    return {
      kind: 'clearance' as const,
      content,
      entry_id: row.entry_id,
      source_id: row.source_id,
    }
  })
}

function buildPresentationsSection(db: Database, resumeId: string): PresentationItem[] {
  const rows = db
    .query(
      `SELECT
        re.id AS entry_id,
        re.content AS entry_content,
        re.perspective_id,
        p.content AS perspective_content,
        p.bullet_id,
        bs.source_id,
        s.title AS source_title,
        s.end_date
      FROM resume_entries re
      JOIN perspectives p ON p.id = re.perspective_id
      JOIN bullet_sources bs ON bs.bullet_id = p.bullet_id AND bs.is_primary = 1
      JOIN sources s ON s.id = bs.source_id
      WHERE re.resume_id = ? AND re.section = 'presentations'
      ORDER BY re.position ASC`
    )
    .all(resumeId) as Array<{
      entry_id: string
      entry_content: string | null
      perspective_id: string
      perspective_content: string
      bullet_id: string
      source_id: string
      source_title: string
      end_date: string | null
    }>

  // Group by source_title (presentation name)
  const presMap = new Map<string, typeof rows>()
  for (const row of rows) {
    if (!presMap.has(row.source_title)) presMap.set(row.source_title, [])
    presMap.get(row.source_title)!.push(row)
  }

  return Array.from(presMap.entries()).map(([title, entries]) => ({
    kind: 'presentation' as const,
    title,
    venue: '', // TODO: extract from source metadata when available
    date: entries[0].end_date ? new Date(entries[0].end_date).getFullYear().toString() : null,
    entry_id: entries[0].entry_id,
    source_id: entries[0].source_id,
    bullets: entries.map(e => ({
      content: e.entry_content ?? e.perspective_content,
      entry_id: e.entry_id,
      source_chain: {
        perspective_id: e.perspective_id,
        bullet_id: e.bullet_id,
        source_id: e.source_id,
      },
      is_cloned: e.entry_content !== null,
    })),
  }))
}

// ── Helpers ───────────────────────────────────────────────────────────

/**
 * Generate a deterministic synthetic UUID from a namespace and key.
 * Uses a simple hash to produce a stable ID for DnD addressing.
 */
function syntheticUUID(namespace: string, key: string): string {
  // Use crypto.randomUUID() seeded by content for determinism
  // Simple approach: hash namespace+key to produce a stable UUID-shaped string
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
function formatDateRange(
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
```

- [ ] **Write tests in `packages/core/src/services/__tests__/resume-compiler.test.ts`:**

Tests should seed a database with:
- A resume
- Organizations (e.g., "Raytheon", "USAF Reserve")
- Sources (roles at those orgs, plus a project, education, clearance)
- Bullets derived from those sources
- Perspectives on those bullets
- Resume entries linking perspectives to sections
- Skills linked via bullet_skills

Then call `compileResumeIR(db, resumeId)` and verify:
  - [ ] Header is populated from resume.header JSON
  - [ ] Experience section groups entries by organization
  - [ ] Within an org, roles ordered by is_current DESC, start_date DESC
  - [ ] Each subheading has correct date_range
  - [ ] Bullets use entry_content when cloned, perspective_content when reference
  - [ ] Skills section derives from bullet_skills junction
  - [ ] Education section populates institution/degree from source_education
  - [ ] Clearance section builds content string from structured clearance data
  - [ ] Projects section groups by source title
  - [ ] Sections have stable synthetic UUIDs
  - [ ] Returns null for non-existent resume ID

**Acceptance Criteria:**
- [ ] `compileResumeIR()` produces correct `ResumeDocument` structure
- [ ] Experience entries grouped by org, sub-grouped by role
- [ ] Skills derived from bullet_skills, not from a separate table
- [ ] All section builders handle missing data gracefully
- [ ] All tests pass

---

### Task 19.4: Markdown Compiler

**Files:**
- Create: `packages/core/src/lib/markdown-compiler.ts`
- Create: `packages/core/src/lib/__tests__/markdown-compiler.test.ts`

**Goal:** Convert IR to GitHub Flavored Markdown.

- [ ] **Create `packages/core/src/lib/markdown-compiler.ts`:**

```typescript
/**
 * IR -> Markdown compiler.
 *
 * Pure function: takes a ResumeDocument, returns a GHFM string.
 * All user-authored content is escaped via escapeMarkdown().
 */

import type { ResumeDocument, IRSection, ResumeHeader } from '@forge/sdk'
import { escapeMarkdown } from './escape'

export function compileToMarkdown(doc: ResumeDocument): string {
  const lines: string[] = []

  // Header
  lines.push(...renderHeader(doc.header))
  lines.push('')

  // Sections
  for (const section of doc.sections) {
    lines.push(...renderSection(section))
    lines.push('')
  }

  return lines.join('\n').trimEnd() + '\n'
}

function renderHeader(header: ResumeHeader): string[] {
  const lines: string[] = []
  lines.push(`# ${escapeMarkdown(header.name)}`)

  if (header.tagline) {
    lines.push(escapeMarkdown(header.tagline))
  }

  // Contact line
  const contact: string[] = []
  if (header.location) contact.push(escapeMarkdown(header.location))
  if (header.email) contact.push(escapeMarkdown(header.email))
  if (header.phone) contact.push(escapeMarkdown(header.phone))
  if (header.linkedin) contact.push(`[LinkedIn](${header.linkedin})`)
  if (header.github) contact.push(`[GitHub](${header.github})`)
  if (header.website) contact.push(`[Website](${header.website})`)

  if (contact.length > 0) {
    lines.push(contact.join(' | '))
  }

  return lines
}

function renderSection(section: IRSection): string[] {
  const lines: string[] = []
  lines.push(`## ${escapeMarkdown(section.title)}`)
  lines.push('')

  switch (section.type) {
    case 'summary':
      for (const item of section.items) {
        if (item.kind === 'summary') {
          lines.push(escapeMarkdown(item.content))
        }
      }
      break

    case 'experience':
      for (const item of section.items) {
        if (item.kind === 'experience_group') {
          lines.push(`### ${escapeMarkdown(item.organization)}`)
          for (const sub of item.subheadings) {
            lines.push(`**${escapeMarkdown(sub.title)}** | ${escapeMarkdown(sub.date_range)}`)
            for (const bullet of sub.bullets) {
              lines.push(`- ${escapeMarkdown(bullet.content)}`)
            }
            lines.push('')
          }
        }
      }
      break

    case 'skills':
      for (const item of section.items) {
        if (item.kind === 'skill_group') {
          for (const cat of item.categories) {
            lines.push(`**${escapeMarkdown(cat.label)}**: ${cat.skills.map(s => escapeMarkdown(s)).join(', ')}`)
          }
        }
      }
      break

    case 'education':
      for (const item of section.items) {
        if (item.kind === 'education') {
          lines.push(`**${escapeMarkdown(item.institution)}**`)
          lines.push(`${escapeMarkdown(item.degree)} | ${escapeMarkdown(item.date)}`)
          lines.push('')
        }
      }
      break

    case 'projects':
      for (const item of section.items) {
        if (item.kind === 'project') {
          const dateSuffix = item.date ? ` | ${escapeMarkdown(item.date)}` : ''
          lines.push(`### ${escapeMarkdown(item.name)}${dateSuffix}`)
          for (const bullet of item.bullets) {
            lines.push(`- ${escapeMarkdown(bullet.content)}`)
          }
          lines.push('')
        }
      }
      break

    case 'certifications':
      for (const item of section.items) {
        if (item.kind === 'certification_group') {
          for (const cat of item.categories) {
            lines.push(`**${escapeMarkdown(cat.label)}**: ${cat.certs.map(c => escapeMarkdown(c.name)).join(', ')}`)
          }
        }
      }
      break

    case 'clearance':
      for (const item of section.items) {
        if (item.kind === 'clearance') {
          lines.push(escapeMarkdown(item.content))
        }
      }
      break

    case 'presentations':
      for (const item of section.items) {
        if (item.kind === 'presentation') {
          const venueDate = [item.venue, item.date].filter(Boolean).join(', ')
          lines.push(`### ${escapeMarkdown(item.title)}${venueDate ? ` | ${escapeMarkdown(venueDate)}` : ''}`)
          for (const bullet of item.bullets) {
            lines.push(`- ${escapeMarkdown(bullet.content)}`)
          }
          lines.push('')
        }
      }
      break

    default:
      // custom / awards / fallback
      for (const item of section.items) {
        if ('content' in item && typeof item.content === 'string') {
          lines.push(`- ${escapeMarkdown(item.content)}`)
        }
      }
      break
  }

  return lines
}
```

- [ ] **Write tests.** Build a known `ResumeDocument` in the test, compile it, verify the output:
  - [ ] Header renders with name, tagline, contact line
  - [ ] Summary renders as plain paragraph
  - [ ] Experience renders with H3 org name, bold role + date, bullet list
  - [ ] Skills renders as `**Label**: skill, skill` lines
  - [ ] Education renders with institution and degree
  - [ ] Projects renders with H3 name and bullets
  - [ ] Clearance renders as plain text
  - [ ] Presentations renders with H3 title, venue, bullets
  - [ ] Special characters are escaped (brackets, backslash)
  - [ ] Empty sections are skipped

**Acceptance Criteria:**
- [ ] `compileToMarkdown()` produces valid GHFM
- [ ] All user content is escaped
- [ ] Output matches expected format for each section type
- [ ] All tests pass

---

### Task 19.5: LaTeX Template (sb2nov)

**Files:**
- Create: `packages/core/src/templates/sb2nov.ts`

**Goal:** Implement the `LatexTemplate` interface with render functions that produce LaTeX matching the existing Guidepoint resume format.

- [ ] **Create `packages/core/src/templates/sb2nov.ts`:**

```typescript
/**
 * sb2nov LaTeX resume template.
 *
 * Implements the LatexTemplate interface. Render functions produce
 * LaTeX structural commands; user content is NOT escaped here
 * (the latex-compiler handles escaping before calling template functions).
 *
 * Based on: https://github.com/sb2nov/resume
 * Reference output: zettelkasten/proj/job-hunting/applications/guidepoint/senior-security-engineer/resume.tex
 */

import type { LatexTemplate, IRSection, ResumeHeader } from '@forge/sdk'

export const sb2nov: LatexTemplate = {
  preamble: `%-------------------------
% Resume in LaTeX
% Based on sb2nov template (https://github.com/sb2nov/resume)
% Generated by Forge Resume Builder
% License: MIT
%------------------------

\\documentclass[letterpaper,10pt]{article}

\\usepackage[empty]{fullpage}
\\usepackage{titlesec}
\\usepackage[usenames,dvipsnames]{color}
\\usepackage{enumitem}
\\usepackage[hidelinks]{hyperref}
\\usepackage{fancyhdr}
\\usepackage[english]{babel}
\\usepackage{tabularx}
\\input{glyphtounicode}

\\pagestyle{fancy}
\\fancyhf{}
\\fancyfoot{}
\\renewcommand{\\headrulewidth}{0pt}
\\renewcommand{\\footrulewidth}{0pt}

\\addtolength{\\oddsidemargin}{-0.55in}
\\addtolength{\\evensidemargin}{-0.55in}
\\addtolength{\\textwidth}{1.1in}
\\addtolength{\\topmargin}{-.55in}
\\addtolength{\\textheight}{1.1in}

\\urlstyle{same}
\\raggedbottom
\\raggedright
\\setlength{\\tabcolsep}{0in}

\\titleformat{\\section}{
  \\vspace{-4pt}\\scshape\\raggedright\\large
}{}{0em}{}[\\color{black}\\titlerule \\vspace{-5pt}]

\\pdfgentounicode=1

\\newcommand{\\resumeItem}[1]{\\item\\small{#1 \\vspace{-2pt}}}
\\newcommand{\\resumeSubheading}[4]{
  \\vspace{-1pt}\\item
    \\begin{tabular*}{0.97\\textwidth}[t]{l@{\\extracolsep{\\fill}}r}
      \\textbf{#1} & #2 \\\\
      \\textit{\\small #3} & \\textit{\\small #4} \\\\
    \\end{tabular*}\\vspace{-5pt}
}
\\newcommand{\\resumeSubSubheading}[2]{
    \\begin{tabular*}{0.97\\textwidth}{l@{\\extracolsep{\\fill}}r}
      \\textit{\\small #1} & \\textit{\\small #2} \\\\
    \\end{tabular*}\\vspace{-5pt}
}
\\newcommand{\\resumeProjectHeading}[2]{
    \\item
    \\begin{tabular*}{0.97\\textwidth}{l@{\\extracolsep{\\fill}}r}
      \\small #1 & #2 \\\\
    \\end{tabular*}\\vspace{-7pt}
}
\\newcommand{\\resumeSubItem}[1]{\\resumeItem{#1}\\vspace{-4pt}}
\\renewcommand{\\labelitemii}{$\\circ$}
\\newcommand{\\resumeSubHeadingListStart}{\\begin{itemize}[leftmargin=*]}
\\newcommand{\\resumeSubHeadingListEnd}{\\end{itemize}}
\\newcommand{\\resumeItemListStart}{\\begin{itemize}}
\\newcommand{\\resumeItemListEnd}{\\end{itemize}\\vspace{-5pt}}
`,

  renderHeader(header: ResumeHeader): string {
    const lines: string[] = []
    lines.push('\\begin{center}')
    lines.push(`    \\textbf{\\Huge \\scshape ${header.name}} \\\\ \\vspace{1pt}`)

    if (header.tagline) {
      lines.push(`    \\small ${header.tagline} \\\\ \\vspace{3pt}`)
    }

    // Contact line
    const parts: string[] = []
    if (header.location) {
      parts.push(`\\small ${header.location}`)
    }
    if (header.email) {
      parts.push(`\\href{mailto:${header.email}}{\\underline{${header.email}}}`)
    }
    if (header.phone) {
      // Strip non-digits for tel: link, keep formatted for display
      const digits = header.phone.replace(/\D/g, '')
      parts.push(`\\href{tel:+${digits}}{\\underline{${header.phone}}}`)
    }
    if (header.linkedin) {
      parts.push(`\\href{${header.linkedin}}{\\underline{LinkedIn}}`)
    }
    if (header.github) {
      parts.push(`\\href{${header.github}}{\\underline{GitHub}}`)
    }
    if (header.website) {
      parts.push(`\\href{${header.website}}{\\underline{Website}}`)
    }

    if (parts.length > 0) {
      lines.push(`    ${parts.join(' $|$\n    ')}`)
    }

    lines.push('\\end{center}')
    return lines.join('\n')
  },

  renderSection(section: IRSection): string {
    switch (section.type) {
      case 'summary':
        return renderSummarySection(section)
      case 'experience':
        return renderExperienceSection(section)
      case 'skills':
        return renderSkillsSection(section)
      case 'education':
        return renderEducationSection(section)
      case 'projects':
        return renderProjectsSection(section)
      case 'certifications':
        return renderCertificationsSection(section)
      case 'clearance':
        return renderClearanceSection(section)
      case 'presentations':
        return renderPresentationsSection(section)
      default:
        return sb2nov.renderSectionFallback(section)
    }
  },

  renderSectionFallback(section: IRSection): string {
    const lines: string[] = []
    lines.push(`\\section{${section.title}}`)
    lines.push('  \\resumeSubHeadingListStart')
    for (const item of section.items) {
      if ('content' in item && typeof item.content === 'string') {
        lines.push(`    \\resumeItem{${item.content}}`)
      }
    }
    lines.push('  \\resumeSubHeadingListEnd')
    return lines.join('\n')
  },

  footer: `
%-------------------------------------------
\\end{document}
`,
}

// ── Section renderers ────────────────────────────────────────────────

function renderSummarySection(section: IRSection): string {
  const lines: string[] = []
  lines.push(`\\section{${section.title}}`)
  for (const item of section.items) {
    if (item.kind === 'summary') {
      lines.push(item.content)
    }
  }
  return lines.join('\n')
}

function renderExperienceSection(section: IRSection): string {
  const lines: string[] = []
  lines.push(`\\section{${section.title}}`)
  lines.push('  \\resumeSubHeadingListStart')

  let isFirst = true
  for (const item of section.items) {
    if (item.kind !== 'experience_group') continue

    if (!isFirst) lines.push('')
    if (!isFirst) lines.push('    \\vspace{5pt}')
    isFirst = false

    // First subheading uses \resumeSubheading, subsequent use \resumeSubSubheading
    for (let i = 0; i < item.subheadings.length; i++) {
      const sub = item.subheadings[i]

      if (i === 0) {
        lines.push('')
        lines.push(`    \\resumeSubheading`)
        lines.push(`      {${item.organization}}{}`)
        lines.push(`      {${sub.title}}{${sub.date_range}}`)
      } else {
        lines.push('    \\vspace{5pt}')
        lines.push(`    \\resumeSubSubheading`)
        lines.push(`      {${sub.title}}{${sub.date_range}}`)
      }

      lines.push('      \\resumeItemListStart')
      for (const bullet of sub.bullets) {
        lines.push(`        \\resumeItem{${bullet.content}}`)
      }
      lines.push('      \\resumeItemListEnd')
    }
  }

  lines.push('  \\resumeSubHeadingListEnd')
  return lines.join('\n')
}

function renderSkillsSection(section: IRSection): string {
  const lines: string[] = []
  lines.push(`\\section{${section.title}}`)
  lines.push(' \\begin{itemize}[leftmargin=0.15in, label={}]')
  lines.push('    \\small{\\item{')

  for (const item of section.items) {
    if (item.kind !== 'skill_group') continue
    for (const cat of item.categories) {
      lines.push(`     \\textbf{${cat.label}}{: ${cat.skills.join(', ')}} \\\\`)
    }
  }

  lines.push('    }}')
  lines.push(' \\end{itemize}')
  return lines.join('\n')
}

function renderEducationSection(section: IRSection): string {
  const lines: string[] = []
  lines.push(`\\section{${section.title}}`)
  lines.push('  \\resumeSubHeadingListStart')

  for (const item of section.items) {
    if (item.kind !== 'education') continue
    lines.push(`    \\resumeSubheading`)
    lines.push(`      {${item.institution}}{}`)
    lines.push(`      {${item.degree}}{${item.date}}`)
  }

  lines.push('  \\resumeSubHeadingListEnd')
  return lines.join('\n')
}

function renderProjectsSection(section: IRSection): string {
  const lines: string[] = []
  lines.push(`\\section{${section.title}}`)
  lines.push('  \\resumeSubHeadingListStart')

  for (const item of section.items) {
    if (item.kind !== 'project') continue
    lines.push(`    \\resumeProjectHeading`)
    lines.push(`      {\\textbf{${item.name}}}{${item.date ?? ''}}`)

    if (item.bullets.length > 0) {
      lines.push('      \\resumeItemListStart')
      for (const bullet of item.bullets) {
        lines.push(`        \\resumeItem{${bullet.content}}`)
      }
      lines.push('      \\resumeItemListEnd')
    }
  }

  lines.push('  \\resumeSubHeadingListEnd')
  return lines.join('\n')
}

function renderCertificationsSection(section: IRSection): string {
  const lines: string[] = []
  lines.push(`\\section{${section.title}}`)
  lines.push('  \\begin{itemize}[leftmargin=0.15in, label={}]')
  lines.push('    \\small{\\item{')

  for (const item of section.items) {
    if (item.kind !== 'certification_group') continue
    for (const cat of item.categories) {
      const certNames = cat.certs.map(c => c.name).join(', ')
      lines.push(`     \\textbf{${cat.label}}{: ${certNames}} \\\\`)
    }
  }

  lines.push('    }}')
  lines.push(' \\end{itemize}')
  return lines.join('\n')
}

function renderClearanceSection(section: IRSection): string {
  const lines: string[] = []
  lines.push(`\\section{${section.title}}`)

  for (const item of section.items) {
    if (item.kind !== 'clearance') continue
    lines.push(item.content)
  }

  return lines.join('\n')
}

function renderPresentationsSection(section: IRSection): string {
  const lines: string[] = []
  lines.push(`\\section{${section.title}}`)
  lines.push('  \\resumeSubHeadingListStart')

  for (const item of section.items) {
    if (item.kind !== 'presentation') continue
    // LaTeX opening double quotes use backtick pairs: `` ... ''
    // Inside a JS template literal, the backticks produce literal ` characters in the output,
    // which LaTeX interprets as opening double quotes. Closing quotes use '' (two single quotes).
    const titlePart = `\\textbf{\`\`${item.title}''}`
    const venuePart = item.venue ? ` $|$ \\emph{${item.venue}${item.date ? `, ${item.date}` : ''}}` : ''
    lines.push(`    \\resumeProjectHeading`)
    lines.push(`      {${titlePart}${venuePart}}{}`)

    if (item.bullets.length > 0) {
      lines.push('      \\resumeItemListStart')
      for (const bullet of item.bullets) {
        lines.push(`        \\resumeItem{${bullet.content}}`)
      }
      lines.push('      \\resumeItemListEnd')
    }
  }

  lines.push('  \\resumeSubHeadingListEnd')
  return lines.join('\n')
}
```

- [ ] **Write tests for each render function.** Build known IRSection objects, call render, verify output matches expected LaTeX fragments. Key tests:
  - [ ] `renderHeader` produces `\begin{center}...\end{center}` with name, tagline, contact $|$-delimited
  - [ ] Experience with multiple roles at same org: first uses `\resumeSubheading`, subsequent use `\resumeSubSubheading`
  - [ ] Skills renders `\textbf{label}{: skill1, skill2}` lines inside itemize
  - [ ] Education uses `\resumeSubheading` with institution and degree
  - [ ] Projects uses `\resumeProjectHeading` with `\textbf{name}`
  - [ ] Certifications uses same pattern as skills
  - [ ] Clearance renders plain text under `\section`
  - [ ] Presentations uses `\resumeProjectHeading` with LaTeX-quoted title (`` `` ... '' ``) and emph venue

**Acceptance Criteria:**
- [ ] Template preamble matches the actual resume.tex preamble (command definitions, packages)
- [ ] Each section type renders to correct LaTeX structure
- [ ] Experience grouping produces correct subheading/sub-subheading pattern
- [ ] All tests pass

---

### Task 19.6: LaTeX Compiler

**Files:**
- Create: `packages/core/src/lib/latex-compiler.ts`
- Create: `packages/core/src/lib/__tests__/latex-compiler.test.ts`

**Goal:** Assemble the full LaTeX document from IR + template, escaping all user content.

- [ ] **Create `packages/core/src/lib/latex-compiler.ts`:**

```typescript
/**
 * IR -> LaTeX compiler.
 *
 * Pure function: takes a ResumeDocument and a LatexTemplate, returns
 * a complete LaTeX document string. All user-authored content is
 * escaped via escapeLatex() before being passed to template functions.
 *
 * IMPORTANT: Template structural strings (commands, environments) are
 * NOT escaped. Only the content fields from the IR are escaped.
 */

import type { ResumeDocument, LatexTemplate, IRSection, ResumeHeader } from '@forge/sdk'
import { escapeLatex } from './escape'

export function compileToLatex(doc: ResumeDocument, template: LatexTemplate): string {
  // 1. Escape all user content in the IR (deep clone + escape)
  const escaped = escapeIR(doc)

  // 2. Assemble document
  const parts: string[] = []
  parts.push(template.preamble)
  parts.push('')
  parts.push('\\begin{document}')
  parts.push('')
  parts.push(template.renderHeader(escaped.header))
  parts.push('')

  for (const section of escaped.sections) {
    parts.push(template.renderSection(section))
    parts.push('')
  }

  parts.push(template.footer)

  return parts.join('\n')
}

/**
 * Deep-clone the IR and escape all user-authored string fields.
 * Returns a new ResumeDocument with escaped content.
 */
function escapeIR(doc: ResumeDocument): ResumeDocument {
  return {
    resume_id: doc.resume_id,
    header: escapeHeader(doc.header),
    sections: doc.sections.map(escapeSection),
  }
}

function escapeHeader(h: ResumeHeader): ResumeHeader {
  return {
    name: escapeLatex(h.name),
    tagline: h.tagline ? escapeLatex(h.tagline) : null,
    location: h.location ? escapeLatex(h.location) : null,
    email: h.email, // emails are used in \href — do NOT escape
    phone: h.phone ? escapeLatex(h.phone) : null,
    linkedin: h.linkedin, // URLs are used in \href — do NOT escape
    github: h.github,
    website: h.website,
    clearance: h.clearance ? escapeLatex(h.clearance) : null,
  }
}

function escapeSection(section: IRSection): IRSection {
  return {
    ...section,
    title: escapeLatex(section.title),
    items: section.items.map(item => {
      switch (item.kind) {
        case 'summary':
          return { ...item, content: escapeLatex(item.content) }

        case 'experience_group':
          return {
            ...item,
            organization: escapeLatex(item.organization),
            subheadings: item.subheadings.map(sub => ({
              ...sub,
              title: escapeLatex(sub.title),
              date_range: escapeLatex(sub.date_range),
              bullets: sub.bullets.map(b => ({
                ...b,
                content: escapeLatex(b.content),
              })),
            })),
          }

        case 'skill_group':
          return {
            ...item,
            categories: item.categories.map(cat => ({
              label: escapeLatex(cat.label),
              skills: cat.skills.map(s => escapeLatex(s)),
            })),
          }

        case 'education':
          return {
            ...item,
            institution: escapeLatex(item.institution),
            degree: escapeLatex(item.degree),
            date: escapeLatex(item.date),
          }

        case 'project':
          return {
            ...item,
            name: escapeLatex(item.name),
            date: item.date ? escapeLatex(item.date) : null,
            bullets: item.bullets.map(b => ({
              ...b,
              content: escapeLatex(b.content),
            })),
          }

        case 'certification_group':
          return {
            ...item,
            categories: item.categories.map(cat => ({
              label: escapeLatex(cat.label),
              certs: cat.certs.map(c => ({
                ...c,
                name: escapeLatex(c.name),
              })),
            })),
          }

        case 'clearance':
          return { ...item, content: escapeLatex(item.content) }

        case 'presentation':
          return {
            ...item,
            title: escapeLatex(item.title),
            venue: escapeLatex(item.venue),
            date: item.date ? escapeLatex(item.date) : null,
            bullets: item.bullets.map(b => ({
              ...b,
              content: escapeLatex(b.content),
            })),
          }

        default:
          return item
      }
    }),
  }
}
```

- [ ] **Write tests.** Build a known `ResumeDocument` with special characters in content, compile with sb2nov template, verify:
  - [ ] Output contains `\begin{document}` and `\end{document}`
  - [ ] Header is rendered between document begin and sections
  - [ ] User content with `&`, `%`, `$` is escaped in output
  - [ ] URLs in header (email, linkedin) are NOT escaped
  - [ ] Section titles appear as `\section{...}`
  - [ ] Experience bullets appear inside `\resumeItemListStart`/`\resumeItemListEnd`
  - [ ] Skills appear inside itemize with `\textbf{label}` pattern
  - [ ] Preamble contains all package declarations

**Acceptance Criteria:**
- [ ] `compileToLatex()` produces valid LaTeX
- [ ] All user content is escaped, URLs are not
- [ ] Template structural commands are preserved verbatim
- [ ] Output matches the structure of the reference resume.tex
- [ ] All tests pass

---

### Task 19.7: Markdown Linter

**Files:**
- Create: `packages/core/src/lib/markdown-linter.ts`
- Create: `packages/core/src/lib/__tests__/markdown-linter.test.ts`

**Goal:** Validate Markdown overrides against 6 structural rules.

- [ ] **Create `packages/core/src/lib/markdown-linter.ts`:**

```typescript
/**
 * Markdown linter for resume content.
 *
 * Validates structural rules before accepting Markdown overrides.
 * Returns { ok: true } or { ok: false, errors: [...] }.
 */

import type { LintResult } from '@forge/sdk'

export function lintMarkdown(content: string): LintResult {
  const errors: string[] = []
  const lines = content.split('\n')

  // Rule 1: Document begins with # Name (H1)
  const firstNonEmpty = lines.find(l => l.trim().length > 0)
  if (!firstNonEmpty || !firstNonEmpty.startsWith('# ')) {
    errors.push('Document must begin with a level-1 heading (# Name)')
  }

  // Rule 2: Sections start with ## (H2)
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i].trim()
    // H3+ used for org names/projects is fine, but H1 after the first line is not
    if (i > 0 && line.startsWith('# ') && !line.startsWith('## ') && !line.startsWith('### ')) {
      errors.push(`Line ${i + 1}: Only one level-1 heading allowed. Sections must use ## (H2)`)
    }
  }

  // Rule 3: Bullet items start with "- " (not "*" or "+")
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]
    const trimmed = line.trimStart()
    if ((trimmed.startsWith('* ') || trimmed.startsWith('+ ')) && !trimmed.startsWith('**')) {
      errors.push(`Line ${i + 1}: Bullet items must start with "- " (not "*" or "+")`)
    }
  }

  // Rule 4: No blank lines within a bullet item (consecutive bullets should not have blanks between them)
  // This checks for patterns like: "- item1\n\n- item2" which is fine in markdown but
  // creates loose lists. We check for blank line between two bullet lines.
  // Actually, the rule is about blank lines WITHIN a multi-line bullet, not between bullets.
  // Skip this for MVP -- it's hard to detect without full parsing.

  // Rule 5: No more than 2 consecutive blank lines
  let consecutiveBlanks = 0
  for (let i = 0; i < lines.length; i++) {
    if (lines[i].trim().length === 0) {
      consecutiveBlanks++
      if (consecutiveBlanks > 2) {
        errors.push(`Line ${i + 1}: No more than 2 consecutive blank lines allowed`)
        break // Report once
      }
    } else {
      consecutiveBlanks = 0
    }
  }

  // Rule 6: Skills section items match **Label**: content pattern
  let inSkillsSection = false
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i].trim()

    if (line.startsWith('## ') && line.toLowerCase().includes('skill')) {
      inSkillsSection = true
      continue
    }
    if (line.startsWith('## ') && !line.toLowerCase().includes('skill')) {
      inSkillsSection = false
      continue
    }

    if (inSkillsSection && line.length > 0 && !line.startsWith('## ')) {
      // Each non-empty line in skills section should match **Label**: content
      if (!line.match(/^\*\*[^*]+\*\*\s*:/)) {
        errors.push(`Line ${i + 1}: Skills section items must match "**Label**: content" pattern`)
      }
    }
  }

  if (errors.length === 0) {
    return { ok: true }
  }
  return { ok: false, errors }
}
```

- [ ] **Write tests** for each rule:
  - [ ] Valid document passes
  - [ ] Missing H1 fails with rule 1 error
  - [ ] Multiple H1 headings fails with rule 2 error
  - [ ] `* ` bullet fails with rule 3 error; `- ` bullet passes
  - [ ] `**bold**` text is not flagged as a bullet (rule 3 edge case)
  - [ ] 3+ consecutive blank lines fails with rule 5 error
  - [ ] Skills section with missing `**Label**:` pattern fails with rule 6 error
  - [ ] Skills section with correct pattern passes

**Acceptance Criteria:**
- [ ] All 6 rules (5 implemented, 1 deferred) are checked
- [ ] Errors include line numbers for actionable feedback
- [ ] Valid documents return `{ ok: true }`
- [ ] All tests pass

---

### Task 19.8: LaTeX Linter

**Files:**
- Create: `packages/core/src/lib/latex-linter.ts`
- Create: `packages/core/src/lib/__tests__/latex-linter.test.ts`

**Goal:** Validate LaTeX overrides against 5 structural + security rules.

- [ ] **Create `packages/core/src/lib/latex-linter.ts`:**

```typescript
/**
 * LaTeX linter for resume content.
 *
 * Validates structural rules and security gates before accepting
 * LaTeX overrides. The security gate (\write18, \input) is critical --
 * prevents filesystem access via LaTeX compilation.
 */

import type { LintResult } from '@forge/sdk'

export function lintLatex(content: string): LintResult {
  const errors: string[] = []

  // Rule 1: Document contains \begin{document} and \end{document}
  if (!content.includes('\\begin{document}')) {
    errors.push('Document must contain \\begin{document}')
  }
  if (!content.includes('\\end{document}')) {
    errors.push('Document must contain \\end{document}')
  }

  // Rule 2: All \resumeItemListStart matched by \resumeItemListEnd
  const itemListStarts = (content.match(/\\resumeItemListStart/g) || []).length
  const itemListEnds = (content.match(/\\resumeItemListEnd/g) || []).length
  if (itemListStarts !== itemListEnds) {
    errors.push(
      `Unmatched \\resumeItemListStart/\\resumeItemListEnd: ${itemListStarts} starts, ${itemListEnds} ends`
    )
  }

  // Rule 3: All \resumeSubHeadingListStart matched by \resumeSubHeadingListEnd
  const subHeadingStarts = (content.match(/\\resumeSubHeadingListStart/g) || []).length
  const subHeadingEnds = (content.match(/\\resumeSubHeadingListEnd/g) || []).length
  if (subHeadingStarts !== subHeadingEnds) {
    errors.push(
      `Unmatched \\resumeSubHeadingListStart/\\resumeSubHeadingListEnd: ${subHeadingStarts} starts, ${subHeadingEnds} ends`
    )
  }

  // Rule 4: Warn on unescaped & and % outside math mode
  // Check each line for unescaped & or % (not preceded by \)
  const lines = content.split('\n')
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]
    // Skip comment lines
    if (line.trimStart().startsWith('%')) continue
    // Skip lines that are likely in math mode ($...$)
    // Simple heuristic: skip if line contains $ (imperfect but good enough)
    if (line.includes('$')) continue

    // Check for unescaped & (not preceded by \)
    const ampMatch = line.match(/(?<!\\)&/)
    if (ampMatch) {
      // Allow & in tabular contexts (lines containing tabular* or @{})
      if (!line.includes('tabular') && !line.includes('@{')) {
        errors.push(`Line ${i + 1}: Possibly unescaped '&' (use \\& in text)`)
      }
    }

    // Check for unescaped % (not preceded by \)
    // But % starts a comment, so any unescaped % IS a comment start
    // This is actually fine in LaTeX -- skip this check
  }

  // Rule 5: Security gate — No \write18 or \input commands
  if (/\\write18/i.test(content)) {
    errors.push('SECURITY: \\write18 is forbidden (enables shell escape)')
  }
  if (/\\input\s*\{/i.test(content)) {
    // Allow \input{glyphtounicode} which is in the preamble
    const inputMatches = content.match(/\\input\s*\{([^}]+)\}/gi) || []
    for (const match of inputMatches) {
      if (!match.includes('glyphtounicode')) {
        errors.push(`SECURITY: \\input commands are forbidden (prevents filesystem access): ${match}`)
      }
    }
  }

  if (errors.length === 0) {
    return { ok: true }
  }
  return { ok: false, errors }
}
```

- [ ] **Write tests:**
  - [ ] Valid LaTeX document passes
  - [ ] Missing `\begin{document}` fails
  - [ ] Missing `\end{document}` fails
  - [ ] Unmatched `\resumeItemListStart` without `\resumeItemListEnd` fails
  - [ ] Matched start/end passes
  - [ ] `\write18` anywhere in document fails with SECURITY error
  - [ ] `\input{/etc/passwd}` fails with SECURITY error
  - [ ] `\input{glyphtounicode}` is allowed (preamble standard)
  - [ ] Unescaped `&` outside tabular context warns

**Acceptance Criteria:**
- [ ] All 5 rules are checked
- [ ] Security gate catches `\write18` and arbitrary `\input`
- [ ] `\input{glyphtounicode}` is exempted
- [ ] Valid documents return `{ ok: true }`
- [ ] All tests pass

---

### Task 19.9: Resume IR API Endpoint

**Files:**
- Modify: `packages/core/src/routes/resumes.ts`
- Modify: `packages/core/src/services/resume-service.ts`
- Modify: `packages/sdk/src/resources/resumes.ts`

**Goal:** Add `GET /api/resumes/:id/ir` endpoint.

- [ ] **Add IR method to ResumeService:**

```typescript
import { compileResumeIR } from './resume-compiler'
import type { ResumeDocument } from '@forge/sdk'

// In ResumeService class:
getIR(id: string): Result<ResumeDocument> {
  const ir = compileResumeIR(this.db, id)
  if (!ir) {
    return { ok: false, error: { code: 'NOT_FOUND', message: `Resume ${id} not found` } }
  }
  return { ok: true, data: ir }
}
```

- [ ] **Add route in resumes.ts:**

```typescript
app.get('/resumes/:id/ir', (c) => {
  const result = services.resumes.getIR(c.req.param('id'))
  if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
  return c.json({ data: result.data })
})
```

- [ ] **Add SDK method:**

```typescript
// In ResumesResource:
ir(id: string): Promise<Result<ResumeDocument>> {
  return this.request<ResumeDocument>('GET', `/api/resumes/${id}/ir`)
}
```

Import `ResumeDocument` in the SDK resource imports.

**Acceptance Criteria:**
- [ ] `GET /api/resumes/:id/ir` returns compiled IR
- [ ] Returns 404 for non-existent resume
- [ ] SDK `forge.resumes.ir(id)` calls the endpoint
- [ ] Response has standard `{ data: ResumeDocument }` envelope

---

### Task 19.10: Resume Override + Header Endpoints

**Files:**
- Modify: `packages/core/src/routes/resumes.ts`
- Modify: `packages/core/src/services/resume-service.ts`
- Modify: `packages/core/src/db/repositories/resume-repository.ts`
- Modify: `packages/sdk/src/resources/resumes.ts`
- Modify: `packages/sdk/src/types.ts`

**Goal:** Add PATCH endpoints for header and overrides.

- [ ] **Update ResumeRepository** with header/override methods:

```typescript
// In ResumeRepository object:

updateHeader(db: Database, id: string, header: Record<string, unknown>): Resume | null {
  const row = db
    .query(
      `UPDATE resumes SET header = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
       WHERE id = ? RETURNING *`
    )
    .get(JSON.stringify(header), id) as ResumeRow | null
  if (!row) return null
  return rowToResume(row)
},

updateMarkdownOverride(db: Database, id: string, content: string | null): Resume | null {
  const row = db
    .query(
      `UPDATE resumes SET
        markdown_override = ?,
        markdown_override_updated_at = CASE WHEN ? IS NOT NULL THEN strftime('%Y-%m-%dT%H:%M:%SZ', 'now') ELSE NULL END,
        updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
       WHERE id = ? RETURNING *`
    )
    .get(content, content, id) as ResumeRow | null
  if (!row) return null
  return rowToResume(row)
},

updateLatexOverride(db: Database, id: string, content: string | null): Resume | null {
  const row = db
    .query(
      `UPDATE resumes SET
        latex_override = ?,
        latex_override_updated_at = CASE WHEN ? IS NOT NULL THEN strftime('%Y-%m-%dT%H:%M:%SZ', 'now') ELSE NULL END,
        updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
       WHERE id = ? RETURNING *`
    )
    .get(content, content, id) as ResumeRow | null
  if (!row) return null
  return rowToResume(row)
},
```

- [ ] **Add service methods:**

```typescript
// In ResumeService:
updateHeader(id: string, header: Record<string, unknown>): Result<Resume> {
  if (!header.name || typeof header.name !== 'string' || header.name.trim().length === 0) {
    return { ok: false, error: { code: 'VALIDATION_ERROR', message: 'Header name must not be empty' } }
  }
  const resume = ResumeRepository.updateHeader(this.db, id, header)
  if (!resume) return { ok: false, error: { code: 'NOT_FOUND', message: `Resume ${id} not found` } }
  return { ok: true, data: resume }
}

updateMarkdownOverride(id: string, content: string | null): Result<Resume> {
  if (content !== null) {
    const lint = lintMarkdown(content)
    if (!lint.ok) {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: `Markdown lint errors: ${lint.errors.join('; ')}` } }
    }
  }
  const resume = ResumeRepository.updateMarkdownOverride(this.db, id, content)
  if (!resume) return { ok: false, error: { code: 'NOT_FOUND', message: `Resume ${id} not found` } }
  return { ok: true, data: resume }
}

updateLatexOverride(id: string, content: string | null): Result<Resume> {
  if (content !== null) {
    const lint = lintLatex(content)
    if (!lint.ok) {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: `LaTeX lint errors: ${lint.errors.join('; ')}` } }
    }
  }
  const resume = ResumeRepository.updateLatexOverride(this.db, id, content)
  if (!resume) return { ok: false, error: { code: 'NOT_FOUND', message: `Resume ${id} not found` } }
  return { ok: true, data: resume }
}
```

- [ ] **Add routes:**

```typescript
app.patch('/resumes/:id/header', async (c) => {
  const body = await c.req.json()
  const result = services.resumes.updateHeader(c.req.param('id'), body)
  if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
  return c.json({ data: result.data })
})

app.patch('/resumes/:id/markdown-override', async (c) => {
  const body = await c.req.json<{ content: string | null }>()
  const result = services.resumes.updateMarkdownOverride(c.req.param('id'), body.content)
  if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
  return c.json({ data: result.data })
})

app.patch('/resumes/:id/latex-override', async (c) => {
  const body = await c.req.json<{ content: string | null }>()
  const result = services.resumes.updateLatexOverride(c.req.param('id'), body.content)
  if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
  return c.json({ data: result.data })
})
```

- [ ] **Add SDK methods:**

```typescript
// In ResumesResource:
updateHeader(id: string, header: Record<string, unknown>): Promise<Result<Resume>> {
  return this.request<Resume>('PATCH', `/api/resumes/${id}/header`, header)
}

updateMarkdownOverride(id: string, content: string | null): Promise<Result<Resume>> {
  return this.request<Resume>('PATCH', `/api/resumes/${id}/markdown-override`, { content })
}

updateLatexOverride(id: string, content: string | null): Promise<Result<Resume>> {
  return this.request<Resume>('PATCH', `/api/resumes/${id}/latex-override`, { content })
}
```

- [ ] **Update SDK Resume type** to include header/override fields:

```typescript
export interface Resume {
  // ... existing fields ...
  header: string | null            // JSON string of ResumeHeader
  markdown_override: string | null
  markdown_override_updated_at: string | null
  latex_override: string | null
  latex_override_updated_at: string | null
}
```

**Acceptance Criteria:**
- [ ] `PATCH /api/resumes/:id/header` updates header JSON
- [ ] `PATCH /api/resumes/:id/markdown-override` stores/clears override
- [ ] `PATCH /api/resumes/:id/latex-override` stores/clears override
- [ ] Overrides are validated by linters before storage
- [ ] Setting `content: null` clears the override
- [ ] Override timestamps are set on save, cleared on null
- [ ] SDK methods work correctly
- [ ] All endpoints return 404 for non-existent resumes

---

### Task 19.11: PDF Endpoint

**Files:**
- Modify: `packages/core/src/routes/resumes.ts`
- Modify: `packages/core/src/services/resume-service.ts`
- Modify: `packages/sdk/src/resources/resumes.ts`

**Goal:** Add `POST /api/resumes/:id/pdf` that spawns tectonic to compile LaTeX to PDF.

- [ ] **Add tectonic availability check.** In the service constructor or as a module-level check:

```typescript
import { $ } from 'bun'

let tectonicAvailable: boolean | null = null

async function checkTectonic(): Promise<boolean> {
  if (tectonicAvailable !== null) return tectonicAvailable
  try {
    const result = await $`which tectonic`.quiet()
    tectonicAvailable = result.exitCode === 0
  } catch {
    tectonicAvailable = false
  }
  if (!tectonicAvailable) {
    console.warn('[forge] tectonic not found — PDF generation will be unavailable')
  }
  return tectonicAvailable
}
```

- [ ] **Add PDF generation method to ResumeService:**

```typescript
async generatePDF(id: string, latex?: string): Promise<Result<Buffer>> {
  // Check tectonic availability
  if (!(await checkTectonic())) {
    return { ok: false, error: { code: 'TECTONIC_NOT_AVAILABLE', message: 'tectonic is not installed. Install it for PDF generation.' } }
  }

  // Get LaTeX content
  let latexContent = latex
  if (!latexContent) {
    // Check for override first
    const resume = ResumeRepository.get(this.db, id)
    if (!resume) return { ok: false, error: { code: 'NOT_FOUND', message: `Resume ${id} not found` } }

    if (resume.latex_override) {
      latexContent = resume.latex_override
    } else {
      // Compile from IR
      const ir = compileResumeIR(this.db, id)
      if (!ir) return { ok: false, error: { code: 'NOT_FOUND', message: `Resume ${id} not found` } }
      latexContent = compileToLatex(ir, sb2nov)
    }
  }

  // Write to temp file
  const tmpDir = '/tmp'
  const uuid = crypto.randomUUID()
  const texPath = `${tmpDir}/forge-pdf-${uuid}.tex`
  const pdfPath = `${tmpDir}/forge-pdf-${uuid}.pdf`

  try {
    await Bun.write(texPath, latexContent)

    // Spawn tectonic with --untrusted and timeout
    const proc = Bun.spawn(
      ['tectonic', '--untrusted', texPath],
      { cwd: tmpDir, stdout: 'pipe', stderr: 'pipe' }
    )

    // Timeout after 60 seconds
    const timeout = setTimeout(() => {
      proc.kill()
    }, 60_000)

    const exitCode = await proc.exited
    clearTimeout(timeout)

    if (exitCode !== 0) {
      const stderr = await new Response(proc.stderr).text()
      // Check if it was killed (timeout)
      if (proc.killed) {
        return { ok: false, error: { code: 'TECTONIC_TIMEOUT', message: 'PDF compilation timed out after 60 seconds' } }
      }
      return {
        ok: false,
        error: {
          code: 'LATEX_COMPILE_ERROR',
          message: 'LaTeX compilation failed',
          details: { tectonic_stderr: stderr.slice(-2000) },
        },
      }
    }

    // Read PDF
    const pdfBytes = await Bun.file(pdfPath).arrayBuffer()
    return { ok: true, data: Buffer.from(pdfBytes) }

  } finally {
    // Cleanup temp files
    try { await Bun.file(texPath).exists() && (await $`rm -f ${texPath} ${pdfPath} ${tmpDir}/forge-pdf-${uuid}.*`.quiet()) } catch { /* ignore */ }
  }
}
```

- [ ] **Add route:**

```typescript
app.post('/resumes/:id/pdf', async (c) => {
  let latex: string | undefined
  try {
    const body = await c.req.json()
    latex = body.latex
  } catch {
    // No body is fine -- compile from IR
  }

  const result = await services.resumes.generatePDF(c.req.param('id'), latex)
  if (!result.ok) {
    const code = result.error.code
    const status = code === 'TECTONIC_NOT_AVAILABLE' ? 501
                 : code === 'TECTONIC_TIMEOUT' ? 504
                 : code === 'LATEX_COMPILE_ERROR' ? 422
                 : mapStatusCode(code)
    return c.json({ error: result.error }, status as any)
  }

  return new Response(result.data, {
    status: 200,
    headers: {
      'Content-Type': 'application/pdf',
      'Content-Disposition': 'inline; filename="resume.pdf"',
    },
  })
})
```

- [ ] **Replace the existing 501 export endpoint.** Remove or update the existing `app.post('/resumes/:id/export', ...)` handler since PDF generation replaces it.

- [ ] **Add SDK method:**

```typescript
// In ResumesResource:
async pdf(id: string, latex?: string): Promise<Result<Blob>> {
  try {
    const url = `/api/resumes/${id}/pdf`
    const options: RequestInit = { method: 'POST' }
    if (latex) {
      options.headers = { 'Content-Type': 'application/json' }
      options.body = JSON.stringify({ latex })
    }

    const response = await fetch(`${this.baseUrl}${url}`, options)

    if (response.headers.get('content-type')?.includes('application/pdf')) {
      const blob = await response.blob()
      return { ok: true, data: blob }
    }

    // Error response
    const json = await response.json() as Record<string, unknown>
    return { ok: false, error: json.error as ForgeError }
  } catch (err) {
    return { ok: false, error: { code: 'NETWORK_ERROR', message: String(err) } }
  }
}
```

Note: The SDK `pdf()` method needs direct fetch access rather than using the standard `request()` helper because the response is binary (not JSON). The ResumesResource needs a `baseUrl` property or a raw fetch function. Implementation detail: either pass `baseUrl` to the constructor or add a `requestRaw` helper to the client.

**Acceptance Criteria:**
- [ ] `POST /api/resumes/:id/pdf` returns PDF binary on success (200)
- [ ] Returns 422 with tectonic stderr on compile error
- [ ] Returns 501 when tectonic is not installed
- [ ] Returns 504 on timeout (60s)
- [ ] Returns 404 for non-existent resume
- [ ] Temp files are cleaned up in finally block
- [ ] `--untrusted` flag is always passed to tectonic
- [ ] SDK method handles binary response
- [ ] Request body `{ latex }` uses provided LaTeX instead of compiling from IR

---

### Task 19.12: Tests

**Files:** Various test files created in earlier tasks, plus integration tests.

**Goal:** Comprehensive test coverage for the entire rendering pipeline.

- [ ] **IR compiler unit tests** (T19.3):
  - Experience grouping: multiple roles at same org produce single ExperienceGroup with multiple subheadings
  - Skills derivation: skills come from bullet_skills of included perspectives, not a separate table
  - Section ordering: sections appear in deterministic order
  - Empty resume: no entries produces no sections
  - Missing org: experience entries without org get grouped under "Other"

- [ ] **Escape function tests** (T19.2):
  - Every special character individually
  - Combined special characters in one string
  - Empty string passthrough
  - Unicode passthrough

- [ ] **Markdown compiler roundtrip** (T19.4):
  - Build IR, compile to markdown, verify structure matches expected output
  - Special characters in content are escaped

- [ ] **LaTeX compiler output** (T19.6):
  - Build IR, compile to LaTeX with sb2nov template
  - Verify `\begin{document}` / `\end{document}` wrapping
  - Verify special characters in content are escaped
  - Verify URLs in header are NOT escaped
  - Verify experience section uses `\resumeSubheading` / `\resumeSubSubheading` pattern

- [ ] **Linter tests** (T19.7, T19.8):
  - Markdown: valid passes, each rule has a failing test case
  - LaTeX: valid passes, `\write18` security gate catches injection, missing `\begin{document}` fails

- [ ] **API endpoint tests** (T19.9, T19.10, T19.11):
  - `GET /api/resumes/:id/ir` returns IR structure
  - `PATCH /api/resumes/:id/header` updates header
  - `PATCH /api/resumes/:id/markdown-override` stores and validates
  - `PATCH /api/resumes/:id/markdown-override` with `content: null` clears override
  - `PATCH /api/resumes/:id/latex-override` stores and validates
  - `POST /api/resumes/:id/pdf` with mock tectonic (or skip if not available)

**Acceptance Criteria:**
- [ ] All tests pass
- [ ] No test.skip markers except for tectonic-dependent tests on CI without tectonic
- [ ] Coverage includes all section types in the IR compiler

---

### Task 19.13: Documentation

**Files:**
- Create: `docs/src/adrs/006-structured-ir.md`
- Create: `docs/src/adrs/007-one-way-overrides.md`
- Modify: `docs/src/api/routes.md`
- Modify: `docs/src/lib/services.md`

**Goal:** Document the architectural decisions and new API surface.

- [ ] **ADR 006: Structured IR**

```markdown
# ADR 006: Structured Intermediate Representation for Resumes

**Status:** Accepted
**Date:** 2026-03-29

## Context

Resumes need to be rendered in multiple formats (DragNDrop, Markdown, LaTeX, PDF).
Each format has different escaping requirements and structural conventions.

## Decision

Introduce a `ResumeDocument` IR (Intermediate Representation) that serves as the
single source of truth for resume content. All view modes render from the IR.

The IR is compiled server-side from structured data (resume + entries + perspectives +
bullets + sources + organizations) via `compileResumeIR()`. Format compilers
(`compileToMarkdown()`, `compileToLatex()`) are pure functions that transform IR to
output strings.

## Consequences

- One compilation path instead of N format-specific query/render paths
- Adding a new format means implementing one compiler function
- IR types are shared between server and client (via @forge/sdk)
- Experience grouping logic lives in one place (the IR compiler)
- Skills are derived from bullet_skills of included perspectives (no separate resume_skills table)
```

- [ ] **ADR 007: One-Way Overrides**

```markdown
# ADR 007: One-Way Override Model for Markdown/LaTeX

**Status:** Accepted
**Date:** 2026-03-29

## Context

Users need to hand-edit Markdown and LaTeX output. Parsing edited Markdown/LaTeX
back into the structured IR is complex and error-prone (especially for LaTeX).

## Decision

Adopt a one-way override model:
- IR -> Markdown and IR -> LaTeX compilation is one-way
- User edits are stored as document-level overrides (text blobs)
- Overrides do NOT parse back into IR
- DragNDrop always works from the IR, never from overrides
- Staleness detection: when resume.updated_at > override_updated_at, the override
  may not reflect recent structural changes

## Consequences

- Simple implementation: no Markdown/LaTeX -> IR parser needed
- Users can freely edit output formats without breaking the structured data
- Trade-off: override edits are not reflected in other formats
- "Regenerate" button recompiles IR -> format, replacing stale overrides
```

- [ ] **Update routes.md** with new endpoints:
  - `GET /api/resumes/:id/ir` -- returns compiled IR
  - `PATCH /api/resumes/:id/header` -- updates header JSON
  - `PATCH /api/resumes/:id/markdown-override` -- stores/clears override
  - `PATCH /api/resumes/:id/latex-override` -- stores/clears override
  - `POST /api/resumes/:id/pdf` -- generates PDF

- [ ] **Update services.md** with:
  - `resume-compiler.ts` -- IR compilation from database state
  - `packages/core/src/lib/escape.ts` -- escaping utilities
  - `packages/core/src/lib/markdown-compiler.ts` -- IR to Markdown
  - `packages/core/src/lib/latex-compiler.ts` -- IR to LaTeX
  - `packages/core/src/templates/sb2nov.ts` -- LaTeX template
  - `packages/core/src/lib/markdown-linter.ts` -- Markdown validation
  - `packages/core/src/lib/latex-linter.ts` -- LaTeX validation

**Acceptance Criteria:**
- [ ] ADR 006 explains IR architecture and consequences
- [ ] ADR 007 explains override model and staleness detection
- [ ] routes.md lists all new endpoints with request/response formats
- [ ] services.md lists all new modules with descriptions
