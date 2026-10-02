# Resume Renderer, Editable Domains, and Organization Updates

**Date:** 2026-03-29
**Status:** Design
**Builds on:** 2026-03-28-forge-resume-builder-design.md, 2026-03-29-forge-schema-evolution-design.md

## Purpose

Add a multi-format resume renderer (DragNDrop → Markdown → LaTeX → PDF), make domains and archetypes user-editable entities, and refine the organization model with status tracking and personal project handling.

## Goals

1. Resume Renderer with structured IR and four view modes (DragNDrop, Markdown, LaTeX, PDF)
2. Editable domains and archetypes (move from hardcoded constants to DB entities)
3. Organization status lifecycle (interested → review → targeting | excluded)
4. Personal project handling (null org, `is_personal` flag, hidden from orgs view)

## Non-Goals

- Full bi-directional LaTeX↔IR parsing (deferred — one-way IR→LaTeX for now, with manual overrides)
- Multiple LaTeX templates (architecture supports it, but MVP ships one template)
- Collaborative editing
- Cloud-based LaTeX compilation (external services forbidden)

---

## 1. Resume Renderer

### 1.1 Intermediate Representation (IR)

The IR is the single source of truth for resume content. All four view modes render from it. The IR types live in `packages/sdk/src/types.ts` (accessible to both server and WebUI client).

```typescript
interface ResumeDocument {
  resume_id: string
  header: ResumeHeader
  sections: ResumeSection[]
}

interface ResumeHeader {
  name: string
  tagline: string | null        // e.g. "Security Engineer | Cloud + DevSecOps"
  location: string | null
  email: string | null
  phone: string | null
  linkedin: string | null
  github: string | null
  website: string | null
  clearance: string | null      // e.g. "TS/SCI with CI Polygraph - Active"
}

interface ResumeSection {
  id: string                     // stable UUID for DnD addressing
  type: ResumeSectionType
  title: string                  // display title, e.g. "Experience", "Technical Skills"
  display_order: number          // section ordering for DnD reorder
  items: ResumeSectionItem[]
}

type ResumeSectionType =
  | 'summary'
  | 'experience'
  | 'skills'
  | 'education'
  | 'projects'
  | 'certifications'
  | 'clearance'
  | 'presentations'
  | 'awards'
  | 'custom'                     // fallback for user-defined sections

type ResumeSectionItem =
  | SummaryItem
  | ExperienceGroup
  | SkillGroup
  | EducationItem
  | ProjectItem
  | CertificationGroup
  | ClearanceItem
  | PresentationItem

interface SummaryItem {
  kind: 'summary'
  content: string
  entry_id: string | null
}

interface ExperienceGroup {
  kind: 'experience_group'
  id: string                     // synthetic UUID for DnD addressing
  organization: string
  subheadings: ExperienceSubheading[]
}

interface ExperienceSubheading {
  id: string                     // synthetic UUID for DnD addressing
  title: string                  // role title
  date_range: string             // "Mar 2024 - Jul 2025"
  source_id: string | null       // links to source for provenance
  bullets: ExperienceBullet[]
}

interface ExperienceBullet {
  content: string
  entry_id: string | null        // links to resume_entry
  source_chain?: {               // provenance for clickable linking
    perspective_id: string
    bullet_id: string
    source_id: string
  }
  is_cloned: boolean             // true if entry has custom content (not reference)
}

interface SkillGroup {
  kind: 'skill_group'
  categories: Array<{
    label: string
    skills: string[]
  }>
}

interface EducationItem {
  kind: 'education'
  institution: string
  degree: string
  date: string
  entry_id: string | null
  source_id: string | null       // links to source_education
}

interface ProjectItem {
  kind: 'project'
  name: string
  date: string | null
  entry_id: string | null        // links to resume_entry
  source_id: string | null       // links to source_project
  bullets: ExperienceBullet[]
}

interface CertificationGroup {
  kind: 'certification_group'
  categories: Array<{
    label: string
    certs: Array<{
      name: string
      entry_id: string | null    // links to resume_entry
      source_id: string | null   // links to source_education (certificate type)
    }>
  }>
}

interface ClearanceItem {
  kind: 'clearance'
  content: string
  entry_id: string | null
  source_id: string | null       // links to source_clearance
}

interface PresentationItem {
  kind: 'presentation'
  title: string
  venue: string
  date: string | null
  entry_id: string | null
  source_id: string | null
  bullets: ExperienceBullet[]
}
```

### 1.2 IR Compilation

The IR is compiled server-side from the resume's structured data.

```
Resume + Entries + Perspectives + Bullets + Sources + Organizations
  ↓ compile (GET /api/resumes/:id/ir)
ResumeDocument (IR)
  ↓ render (client-side)
DragNDrop View | Markdown | LaTeX | PDF
```

**Compiler logic (in `packages/core/src/services/resume-compiler.ts`):**
1. Fetch resume with entries (grouped by section)
2. For each entry, resolve perspective → bullet → source (via chain)
3. Group experience entries by organization (via `source_roles.organization_id`)
4. Within each org group, sub-group by role, ordered by `is_current DESC, start_date DESC`
5. Derive skills from included entries' perspectives' `bullet_skills` (no `resume_skills` table — skills are derived)
6. Build the `ResumeDocument` structure with synthetic UUIDs for sections and groups

**Experience grouping query:**
```sql
SELECT
  re.id AS entry_id, re.content AS entry_content, re.perspective_id, re.position,
  p.content AS perspective_content, p.bullet_id,
  sr.organization_id, sr.start_date, sr.end_date, sr.is_current,
  o.name AS org_name,
  s.id AS source_id, s.title AS source_title
FROM resume_entries re
JOIN perspectives p ON p.id = re.perspective_id
JOIN bullet_sources bs ON bs.bullet_id = p.bullet_id AND bs.is_primary = 1
JOIN sources s ON s.id = bs.source_id
LEFT JOIN source_roles sr ON sr.source_id = s.id
LEFT JOIN organizations o ON o.id = sr.organization_id
WHERE re.resume_id = ? AND re.section = 'experience'
ORDER BY sr.is_current DESC, sr.start_date DESC, re.position ASC
```

Application code groups by `org_name`, sub-groups by `source_title`.

### 1.3 Escaping Utilities

Located in `packages/core/src/lib/escape.ts`, importable by both server and client.

**`escapeLatex(text: string): string`** — Escapes the 10 LaTeX special characters:
```
& → \&    % → \%    $ → \$    # → \#    _ → \_
{ → \{    } → \}    ~ → \textasciitilde{}
^ → \textasciicircum{}    \ → \textbackslash{}
```
Applied to ALL user-authored content fields before LaTeX template insertion. Never applied to template structural strings.

**`escapeMarkdown(text: string): string`** — Escapes `[`, `]`, `\` in inline positions. Does NOT escape `*` or `_` (IR content is plain text, no inline formatting in MVP).

**Unicode handling:** The LaTeX template uses `\pdfgentounicode=1` with pdflatex, which handles UTF-8 for most characters. Tectonic also handles UTF-8 natively. No special Unicode escaping needed for common characters (é, ñ, —, etc.).

### 1.4 View Modes

The Resume view has a tabbed preview area with four modes:

#### DragNDrop (default)
- Visual representation of the resume with editable sections
- Each bullet is clickable → shows provenance chain (source → bullet → perspective → entry)
- Each bullet is hoverable → shows metadata tooltip (domain, archetype, framing)
- Edit button per bullet → enters clone mode (edits the resume_entry.content)
- Delete clone button → resets entry to reference mode
- Drag to reorder within sections (updates entry positions via `PATCH /api/resumes/:id/entries/:eid`)
- Drag between sections (updates entry section + position)
- Add section button (for custom sections), add entry button (opens perspective picker)
- Since all edits go through the IR, formatting is implicitly enforced

#### Markdown
- GHFM rendered in a split view:
  - Left: syntax-highlighted editor (CodeMirror with markdown mode)
  - Right: rendered preview
- Compiled from IR → Markdown:
  ```markdown
  # Adam Smith
  Security Engineer | Cloud + DevSecOps
  Reston, VA | adam.smith@arusty.dev | [LinkedIn](...) | [GitHub](...)

  ## Summary
  Security engineer with 14+ years...

  ## Experience
  ### United States Air Force Reserve
  **Cyber Warfare Operator** | Sep 2018 - Present
  - Planned, designed, and implemented red-team infrastructure...
  ```
- Edits are **one-way overrides** (stored as text blob, do not parse back into IR)
- Internal linter validates before accepting edits:
  1. Document begins with `# Name` (H1)
  2. Sections start with `##` (H2)
  3. Bullet items start with `- ` (not `*` or `+`)
  4. No blank lines within a bullet item
  5. No more than 2 consecutive blank lines
  6. Skills section items match `**Label**: content` pattern
- Toggle between Edit and View modes

#### LaTeX
- Split view: syntax-highlighted editor (CodeMirror with LaTeX mode) + rendered preview
- Compiled from IR → LaTeX via template (sb2nov-based)
- LaTeX preview rendered client-side via custom HTML renderer (maps `\resumeSubheading` → HTML layout, `\resumeItem` → list item)
- Edits are **one-way overrides** (stored as text blob, do not parse back)
- Internal linter validates before accepting:
  1. Document contains `\begin{document}` and `\end{document}`
  2. All `\resumeItemListStart` matched by `\resumeItemListEnd`
  3. All `\resumeSubHeadingListStart` matched by `\resumeSubHeadingListEnd`
  4. Warn on unescaped `&` and `%` outside math mode
  5. **Security gate:** No `\write18` or `\input` commands (prevents filesystem access)
- Template commands map to IR:
  - `\resumeSubheading{org}{}{role}{dates}` ← ExperienceSubheading
  - `\resumeSubSubheading{role}{dates}` ← Sub-role within same org
  - `\resumeItem{text}` ← ExperienceBullet
  - `\resumeProjectHeading{name}{date}` ← ProjectItem
  - `\section{Title}` ← ResumeSection

#### PDF
- Read-only view showing the compiled PDF
- Generated server-side: `POST /api/resumes/:id/pdf`
- Displayed in `<iframe>` or PDF.js viewer
- Fallback: "Install tectonic for PDF generation" when unavailable

### 1.5 IR → Format Compilers

Each compiler is a pure function: `IR → string`.

**IR → Markdown:**
```typescript
function compileToMarkdown(doc: ResumeDocument): string
```
Walks the IR, calls `escapeMarkdown()` on all content fields. Deterministic.

**IR → LaTeX:**
```typescript
function compileToLatex(doc: ResumeDocument, template: LatexTemplate): string
```
Calls `escapeLatex()` on all user-authored content fields. Template structural strings are NOT escaped.

**`LatexTemplate` interface (function-based, not string-based):**
```typescript
interface LatexTemplate {
  preamble: string                            // static: everything before \begin{document}
  renderHeader: (header: ResumeHeader) => string
  renderSection: (section: ResumeSection) => string
  renderSectionFallback: (section: ResumeSection) => string  // for 'custom' type
  footer: string                              // static: \end{document}
}
```

The `renderSection` function dispatches internally based on `section.type`, calling type-specific renderers for experience, skills, education, etc. This makes templates extensible — a new template only needs to implement the `LatexTemplate` interface.

**MVP template stored as:** `packages/core/src/templates/sb2nov.ts` — a TypeScript module exporting a `LatexTemplate` object with the render functions.

**Markdown → IR (deferred):**
Not implemented in MVP. When implemented, enables round-trip editing.

### 1.6 Override Storage & Staleness

When a user edits in Markdown or LaTeX mode, the edit is stored as a document-level override:

```sql
ALTER TABLE resumes ADD COLUMN header TEXT;                          -- JSON blob for ResumeHeader
ALTER TABLE resumes ADD COLUMN markdown_override TEXT;
ALTER TABLE resumes ADD COLUMN markdown_override_updated_at TEXT;
ALTER TABLE resumes ADD COLUMN latex_override TEXT;
ALTER TABLE resumes ADD COLUMN latex_override_updated_at TEXT;
```

**Behavior:**
- `NULL` override = render from IR (default)
- Non-NULL override = use the override text, show it in the editor
- "Reset to generated" button sets override back to NULL

**Staleness detection:**
- When `resume.updated_at > override_updated_at`: the structured data (DnD) has changed since the override was saved
- UI shows a prominent warning banner: "The structured resume has been updated since this override was saved. The text below may not reflect recent changes."
- "Regenerate" button recompiles IR→format, replacing the stale override
- When `resume.updated_at <= override_updated_at`: override is current, show neutral info banner

**DragNDrop always works from the IR** — never reads from overrides. If an override exists and the user switches to DnD, they edit the IR directly. Switching back to Markdown/LaTeX shows the (potentially stale) override with the appropriate banner.

### 1.7 PDF Generation Pipeline

```
IR → compileToLatex() → LaTeX string → POST /api/resumes/:id/pdf → tectonic → PDF bytes
```

**API endpoint:**
```
POST /api/resumes/:id/pdf
Request: { latex?: string }
  - If latex provided: compile that string
  - If omitted: use latex_override if exists, else compile from IR
Response:
  - 200: application/pdf binary, Content-Disposition: inline; filename="resume.pdf"
  - 422: { error: { code: "LATEX_COMPILE_ERROR", message: "...", details: { tectonic_stderr: "..." } } }
  - 501: { error: { code: "TECTONIC_NOT_AVAILABLE", message: "..." } }
  - 504: { error: { code: "TECTONIC_TIMEOUT", message: "..." } }
```

**Server implementation:**
1. Write LaTeX string to temp file (`/tmp/forge-pdf-<uuid>.tex`)
2. Spawn `tectonic --untrusted <tempfile.tex>` (`--untrusted` disables shell escape for security)
3. On success: read PDF, return as response
4. On compile error (exit code != 0): return 422 with stderr (last 2000 chars)
5. On timeout (>60s): kill process, return 504
6. Always: clean up temp files in `finally` block

**Tectonic startup check:** Same pattern as Claude CLI check in `index.ts`. If not found, log warning — server works for everything except PDF generation.

### 1.8 Resume Header Management

**API endpoints:**
```
PATCH /api/resumes/:id/header
Body: { name, tagline?, location?, email?, phone?, linkedin?, github?, website?, clearance? }
Response: 200 with updated resume

PATCH /api/resumes/:id/markdown-override
Body: { content: string | null }  // null = reset
Response: 200

PATCH /api/resumes/:id/latex-override
Body: { content: string | null }  // null = reset
Response: 200
```

**IR endpoint:**
```
GET /api/resumes/:id/ir
Response: 200 with { data: ResumeDocument }
```

---

## 2. Editable Domains & Archetypes

### 2.1 Schema

Replace hardcoded constants with database entities.

```sql
CREATE TABLE domains (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  name TEXT NOT NULL UNIQUE,
  description TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

CREATE TABLE archetypes (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  name TEXT NOT NULL UNIQUE,
  description TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

CREATE TABLE archetype_domains (
  archetype_id TEXT NOT NULL REFERENCES archetypes(id) ON DELETE CASCADE,
  domain_id TEXT NOT NULL REFERENCES domains(id) ON DELETE CASCADE,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  PRIMARY KEY (archetype_id, domain_id)
) STRICT;
```

**Seed data (in migration):**

```sql
-- Seed domains
INSERT INTO domains (id, name, description) VALUES
  (lower(hex(randomblob(4))||'-'||hex(randomblob(2))||'-4'||substr(hex(randomblob(2)),2)||'-'||substr('89ab',abs(random())%4+1,1)||substr(hex(randomblob(2)),2)||'-'||hex(randomblob(6))), 'systems_engineering', 'Architecture and distributed systems'),
  -- ... (6 domains total: systems_engineering, software_engineering, security, devops, ai_ml, leadership)
;

-- Seed archetypes
INSERT INTO archetypes (id, name, description) VALUES
  -- ... (6 archetypes: agentic-ai, infrastructure, security-engineer, solutions-architect, public-sector, hft)
;

-- Seed archetype_domains from ARCHETYPE_EXPECTED_DOMAINS
INSERT INTO archetype_domains (archetype_id, domain_id)
  SELECT a.id, d.id FROM archetypes a, domains d
  WHERE a.name = 'agentic-ai' AND d.name IN ('ai_ml', 'software_engineering', 'leadership');
-- ... (one INSERT per archetype)
```

**Impact on existing code:**
- `ARCHETYPES`, `DOMAINS`, `ARCHETYPE_EXPECTED_DOMAINS`, `THIN_COVERAGE_THRESHOLD` constants become seed data. The constants file can remain for `THIN_COVERAGE_THRESHOLD` and `RESUME_SECTIONS`.
- `RESUME_SECTIONS` updated to include: `'summary', 'experience', 'projects', 'education', 'skills', 'certifications', 'clearance', 'presentations', 'awards', 'custom'`
- `ResumeService.analyzeGaps` queries `archetype_domains` junction instead of the constant map
- `DerivationService` validates archetype/domain against DB rows
- Perspective `target_archetype` and `domain` reference names (text), not IDs

### 2.2 Domains View

New view at `/domains`:
- List of domains with name, description
- CRUD (add, edit, delete)
- Delete blocked if domain is referenced by perspectives or archetype_domains
- Shows count of perspectives using each domain

### 2.3 Archetypes View (Updated)

The existing `/archetypes` view becomes editable:
- List of archetypes with name, description
- Each archetype shows its expected domains (from `archetype_domains`)
- Add/remove domain associations via tag picker
- CRUD for archetypes
- Delete blocked if archetype is referenced by resumes or perspectives
- Shows count of resumes and perspectives using each archetype

### 2.4 API

New endpoints:

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/domains` | Create domain |
| GET | `/api/domains` | List domains |
| GET | `/api/domains/:id` | Get domain |
| PATCH | `/api/domains/:id` | Update domain |
| DELETE | `/api/domains/:id` | Delete (blocked if referenced) |
| POST | `/api/archetypes` | Create archetype |
| GET | `/api/archetypes` | List archetypes (with domain associations) |
| GET | `/api/archetypes/:id` | Get archetype with domains |
| PATCH | `/api/archetypes/:id` | Update archetype |
| DELETE | `/api/archetypes/:id` | Delete (blocked if referenced) |
| POST | `/api/archetypes/:id/domains` | Add domain to archetype |
| DELETE | `/api/archetypes/:id/domains/:domainId` | Remove domain from archetype |
| GET | `/api/archetypes/:id/domains` | List domains for archetype |

---

## 3. Organization Updates

### 3.1 Status Lifecycle

Add a `status` column to `organizations`:

```sql
ALTER TABLE organizations ADD COLUMN status TEXT CHECK (status IN (
  'interested', 'review', 'targeting', 'excluded', NULL
));
```

Status values:
- `NULL` — no tracking (default for historical employers)
- `interested` — on the radar, not yet researched
- `review` — actively researching
- `targeting` — actively pursuing opportunities
- `excluded` — decided against (with reason in notes)

`worked` (boolean) is orthogonal to `status`.

### 3.2 Status Transitions

```
NULL → interested → review → targeting
                          → excluded
targeting → excluded (changed mind)
excluded → interested (reconsidering)
```

No hard enforcement in the DB — the UI guides the flow.

### 3.3 Organizations View Updates

- Add status column to the list
- Add status filter (All | Interested | Review | Targeting | Excluded | Worked)
- Status badge colors: interested=blue, review=amber, targeting=green, excluded=gray/strikethrough
- Status dropdown in editor

### 3.4 Personal Project Handling

`organization_id = NULL` + `is_personal = 1` on `source_projects`. No phantom org.

- Organizations view: does NOT show "Personal"
- Sources view (Projects tab): shows "Personal" label when `is_personal = 1`
- Source editor: "Personal project" checkbox hides org dropdown

No schema change needed.

---

## 4. Acceptance Criteria

### Resume Renderer
- [ ] `ResumeDocument` IR type defined in SDK with all section types, `id`/`display_order` on sections, `id` on groups/subheadings
- [ ] IR compiler builds document from resume + entries + chain + organizations
- [ ] Experience entries grouped by organization, ordered by is_current DESC, start_date DESC
- [ ] Skills derived from included entries' perspectives' bullet_skills
- [ ] `escapeLatex()` and `escapeMarkdown()` utility functions handle all special characters
- [ ] `GET /api/resumes/:id/ir` returns compiled IR
- [ ] DragNDrop view renders IR with clickable/hoverable provenance links
- [ ] DragNDrop supports clone edit and clone delete (reset to ref)
- [ ] DragNDrop supports drag-to-reorder within and between sections
- [ ] Markdown compiler produces GHFM from IR with escaped content
- [ ] Markdown editor with syntax highlighting (CodeMirror) and split preview
- [ ] Markdown linter validates 6 rules before accepting edits
- [ ] Markdown override stored with `markdown_override_updated_at`
- [ ] LaTeX compiler produces valid LaTeX from IR using sb2nov template
- [ ] LaTeX template stored as TypeScript module with render functions
- [ ] LaTeX editor with syntax highlighting and split preview
- [ ] LaTeX linter validates 5 rules (including `\write18` security gate)
- [ ] LaTeX override stored with `latex_override_updated_at`
- [ ] Stale override detection: warning banner when `resume.updated_at > override_updated_at`
- [ ] "Regenerate" button recompiles IR→format, replacing stale override
- [ ] `POST /api/resumes/:id/pdf` generates PDF via tectonic with `--untrusted`
- [ ] PDF displayed in iframe in the PDF tab
- [ ] Tectonic failure returns 422 with stderr, 501 if not installed, 504 on timeout
- [ ] `PATCH /api/resumes/:id/header` updates resume header JSON
- [ ] `PATCH /api/resumes/:id/markdown-override` and `latex-override` endpoints
- [ ] Resume header (name, contact, clearance) editable and stored as JSON

### Editable Domains & Archetypes
- [ ] `domains`, `archetypes`, `archetype_domains` tables created with seed data
- [ ] `archetype_domains` has `created_at` column
- [ ] Seed migration inserts 6 domains and 6 archetypes with correct associations
- [ ] Domains CRUD API endpoints
- [ ] Archetypes CRUD API endpoints with domain association management
- [ ] `GET /api/archetypes/:id/domains` returns associated domains
- [ ] Domains view with CRUD
- [ ] Archetypes view with CRUD + domain tag picker
- [ ] Delete blocked if domain/archetype referenced by perspectives or resumes
- [ ] Gap analysis queries `archetype_domains` instead of constants
- [ ] Derivation validates archetype/domain against DB rows
- [ ] `RESUME_SECTIONS` updated to include `experience`, `certifications`, `clearance`, `presentations`, `custom`

### Organization Updates
- [ ] `status` column on organizations (interested/review/targeting/excluded/null)
- [ ] Organizations view shows status with filter and colored badges
- [ ] Status dropdown in organization editor
- [ ] Personal projects: "Personal" label in sources view, no phantom org

### Tests
- [ ] IR compiler unit tests: builds correct structure from test data
- [ ] Markdown compiler: escapes special characters, produces valid GHFM
- [ ] LaTeX compiler: escapes special characters, produces valid LaTeX matching template
- [ ] Linter tests: Markdown accepts valid / rejects invalid; LaTeX accepts valid / rejects `\write18`
- [ ] Override staleness: correct detection and banner behavior
- [ ] PDF endpoint: returns PDF on success, 422 on compile error, 501 when tectonic absent
- [ ] Experience grouping: entries grouped by org, ordered correctly
- [ ] Skills derivation: skills derived from included perspectives' bullet_skills
- [ ] Domain/archetype CRUD with delete-blocked-if-referenced
- [ ] Gap analysis works with DB-backed archetype_domains

---

## 5. Dependencies & Parallelization

### Sequential
1. Schema migration (003_renderer_and_entities.sql) — must be first
2. IR type definitions — before any compiler or view
3. Escape utilities — before any compiler
4. IR compiler — before any view mode

### Parallel (after schema + IR + escaping + compiler)
- DragNDrop view (requires svelte-dnd-action)
- Markdown compiler + editor (requires CodeMirror)
- LaTeX compiler + template + editor (requires CodeMirror with LaTeX mode)
- PDF endpoint (depends on LaTeX compiler + tectonic)
- Domains/Archetypes CRUD (repos, services, routes, SDK, UI) — fully independent
- Organization status update — simple column add, independent

### Dependencies
- LaTeX → PDF requires tectonic installed on the server
- DragNDrop drag requires svelte-dnd-action or similar
- Markdown/LaTeX editors require CodeMirror 6
- IR compiler requires the experience grouping query to work (depends on Phase 10-11 schema)
- Skills derivation requires `bullet_skills` junction data
