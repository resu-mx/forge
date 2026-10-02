# Resume Sections as First-Class Entities

**Date:** 2026-03-30
**Status:** Design
**Builds on:** Resume Renderer (Phases 19-20), DnD Per-Role Bullet Add (Phase 26)

## Purpose

Make resume sections first-class database entities with user-defined names and enforced entry types. Currently sections are hardcoded strings (`'experience'`, `'summary'`, etc.) on `resume_entries.section`. This prevents users from adding custom sections, having multiple sections of the same type (e.g., "Civilian Work" and "Military Service" both as experience), or reordering sections.

## Goals

1. `resume_sections` table — sections are entities with id, title, entry_type, position
2. User-defined section names — "Technical Skills", "Conference Talks", anything
3. Enforced entry types — each section knows what kind of data goes in it
4. `resume_skills` table — skills section uses direct skill references, not perspectives
5. "Add Section" UI in DragNDrop view — dropdown of entry types not yet present
6. Type-specific pickers — each entry type has its own add flow

## Non-Goals

- Drag-to-reorder sections (use up/down arrows for MVP)
- Section templates / presets (future)
- Nested sections (sections within sections)
- Custom entry types beyond the fixed set

---

## 1. Schema Changes (Migration 004)

### 1.1 `resume_sections` Table

```sql
CREATE TABLE resume_sections (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  resume_id TEXT NOT NULL REFERENCES resumes(id) ON DELETE CASCADE,
  title TEXT NOT NULL,
  entry_type TEXT NOT NULL CHECK (entry_type IN (
    'experience', 'skills', 'education', 'projects',
    'clearance', 'presentations', 'certifications', 'awards', 'freeform'
  )),
  position INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

CREATE INDEX idx_resume_sections_resume ON resume_sections(resume_id, position);
```

> **Note:** `certifications` and `awards` are included in the CHECK constraint because they exist in `IRSectionType`. Even if not used at launch, the constraint should not block future sections from being created without a migration.

**Entry types and what they contain:**

| entry_type | What the picker shows | Data source | How it renders |
|-----------|----------------------|-------------|----------------|
| `experience` | Approved perspectives filtered by source (role) | `resume_entries` -> perspective -> bullet -> source (role) | Grouped by org -> role -> bullets |
| `skills` | Skills from skills table, grouped by category | `resume_skills` -> skill | Category: skill1, skill2, ... |
| `education` | Sources with `source_type = 'education'` | `resume_entries` -> perspective from education bullets | Institution, degree, date |
| `projects` | Sources with `source_type = 'project'` | `resume_entries` -> perspective from project bullets | Project name, bullets |
| `clearance` | Sources with `source_type = 'clearance'` | `resume_entries` -> clearance source content | Plain text: level, status |
| `presentations` | Sources or free text | `resume_entries` -> perspective or direct content | Title, venue, bullets |
| `certifications` | Sources with `source_type = 'certification'` | `resume_entries` -> certification source content | Name, issuer, date |
| `awards` | Sources or free text | `resume_entries` -> award content | Name, description |
| `freeform` | Free text input | `resume_entries` with direct `content` (no perspective) | Plain text paragraphs |

### 1.2 `resume_skills` Table

Skills don't go through the perspective pipeline — they're direct references to the skills table, scoped to a section.

```sql
CREATE TABLE resume_skills (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  section_id TEXT NOT NULL REFERENCES resume_sections(id) ON DELETE CASCADE,
  skill_id TEXT NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
  position INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  UNIQUE(section_id, skill_id)
) STRICT;

CREATE INDEX idx_resume_skills_section ON resume_skills(section_id, position);
```

### 1.3 Update `resume_entries` — Replace `section` String with `section_id` FK

```sql
-- Add section_id column
ALTER TABLE resume_entries ADD COLUMN section_id TEXT REFERENCES resume_sections(id) ON DELETE CASCADE;

-- Migrate existing data:
-- 1. Convert summary entries from synthetic perspectives to freeform content
-- 2. Create resume_sections for each unique (resume_id, section) pair
-- 3. Update resume_entries.section_id to point to the new section
-- 4. Auto-populate resume_skills from existing bullet_skills derivation
-- 5. Drop the old section column (table rebuild with PRAGMA foreign_keys = OFF)
```

**Step 1 — Convert summary entries to freeform:**

> Existing summary entries were created with synthetic perspectives during v1 import. The freeform builder expects `perspective_id = NULL` with content set directly. This step preserves the text while converting to the freeform model.

```sql
-- Convert summary entries: copy perspective content into entry content, null out perspective_id
UPDATE resume_entries
SET content = (SELECT p.content FROM perspectives p WHERE p.id = resume_entries.perspective_id),
    perspective_id = NULL
WHERE section = 'summary'
  AND perspective_id IS NOT NULL;
```

**Step 2 — Create sections from existing entries:**

```sql
-- For each resume, create sections from existing entries
INSERT INTO resume_sections (id, resume_id, title, entry_type, position)
  SELECT
    lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)),2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)),2) || '-' || hex(randomblob(6))),
    resume_id,
    CASE section
      WHEN 'experience' THEN 'Experience'
      WHEN 'summary' THEN 'Summary'
      WHEN 'skills' THEN 'Technical Skills'
      WHEN 'education' THEN 'Education & Certifications'
      WHEN 'projects' THEN 'Selected Projects'
      WHEN 'clearance' THEN 'Security Clearance'
      WHEN 'presentations' THEN 'Presentations'
      ELSE section
    END,
    CASE section
      WHEN 'summary' THEN 'freeform'
      ELSE section
    END,
    CASE section
      WHEN 'summary' THEN 0
      WHEN 'experience' THEN 1
      WHEN 'skills' THEN 2
      WHEN 'education' THEN 3
      WHEN 'projects' THEN 4
      WHEN 'clearance' THEN 5
      WHEN 'presentations' THEN 6
      ELSE 7
    END
  FROM (SELECT DISTINCT resume_id, section FROM resume_entries);
```

**Step 3 — Update entries to reference sections:**

```sql
-- NOTE: This subquery assumes one section per entry_type per resume, which is
-- guaranteed during migration (the INSERT above creates exactly one section per
-- distinct (resume_id, section) pair). After migration, users can create multiple
-- sections of the same type — but this UPDATE only runs once during migration
-- when the one-per-type constraint holds.
UPDATE resume_entries SET section_id = (
  SELECT rs.id FROM resume_sections rs
  WHERE rs.resume_id = resume_entries.resume_id
  AND rs.entry_type = CASE resume_entries.section
    WHEN 'summary' THEN 'freeform'
    ELSE resume_entries.section
  END
);
```

**Step 4 — Auto-populate `resume_skills` from existing data:**

> After migration, the skills section reads from `resume_skills` (empty for existing resumes) instead of deriving from `bullet_skills`. Without this step, existing resumes would show an empty skills section. This populates `resume_skills` from the existing `bullet_skills` derivation chain.

```sql
INSERT INTO resume_skills (id, section_id, skill_id, position)
SELECT
  lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)),2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)),2) || '-' || hex(randomblob(6))),
  rs.id,
  bs.skill_id,
  ROW_NUMBER() OVER (PARTITION BY rs.id ORDER BY bs.skill_id) - 1
FROM resume_sections rs
JOIN resume_entries re ON re.section_id = rs.id
JOIN perspectives p ON p.id = re.perspective_id
JOIN bullet_skills bs ON bs.bullet_id = p.bullet_id
WHERE rs.entry_type = 'skills'
GROUP BY rs.id, bs.skill_id;
```

**Step 5 — Table rebuild (drop `section` column, enforce `section_id NOT NULL`):**

```sql
-- IMPORTANT: Must disable FK enforcement during table rebuild.
-- The migration runner's connection sets foreign_keys = ON. Table rebuilds
-- (DROP + RENAME) temporarily break FK references, so enforcement must be
-- explicitly disabled. (Same pattern as migration 002, steps 6 and 11.)
PRAGMA foreign_keys = OFF;

-- Table rebuild: create new table without section column, with section_id NOT NULL
CREATE TABLE resume_entries_new (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  resume_id TEXT NOT NULL REFERENCES resumes(id) ON DELETE CASCADE,
  section_id TEXT NOT NULL REFERENCES resume_sections(id) ON DELETE CASCADE,
  perspective_id TEXT REFERENCES perspectives(id) ON DELETE RESTRICT,
  content TEXT,
  position INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

INSERT INTO resume_entries_new (id, resume_id, section_id, perspective_id, content, position, created_at, updated_at)
  SELECT id, resume_id, section_id, perspective_id, content, position, created_at, updated_at
  FROM resume_entries;

DROP TABLE resume_entries;
ALTER TABLE resume_entries_new RENAME TO resume_entries;

-- Recreate indexes
CREATE INDEX idx_resume_entries_section ON resume_entries(section_id, position);
CREATE INDEX idx_resume_entries_resume ON resume_entries(resume_id);

PRAGMA foreign_keys = ON;
```

After migration, the `section` string column is dropped. The `section_id` FK replaces it. `perspective_id` is now nullable (for freeform entries).

### 1.4 Allow Nullable `perspective_id` on `resume_entries`

Currently `perspective_id TEXT NOT NULL`. For `freeform` entries (summaries, custom text), there's no perspective — the content is entered directly. The table rebuild in step 5 above handles this change:

```sql
-- In the table rebuild, change:
perspective_id TEXT NOT NULL REFERENCES perspectives(id) ON DELETE RESTRICT
-- To:
perspective_id TEXT REFERENCES perspectives(id) ON DELETE RESTRICT
```

Freeform entries have `perspective_id = NULL` and `content` set directly by the user.

---

## 2. API

### 2.1 Section CRUD

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/resumes/:id/sections` | Create section (title, entry_type, position) |
| GET | `/api/resumes/:id/sections` | List sections for a resume (ordered by position) |
| PATCH | `/api/resumes/:id/sections/:sectionId` | Update section (title, position) — entry_type is immutable |
| DELETE | `/api/resumes/:id/sections/:sectionId` | Delete section (cascades to entries + skills) |

### 2.2 Resume Skills CRUD

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/resumes/:id/sections/:sectionId/skills` | Add skill to section (skill_id) |
| DELETE | `/api/resumes/:id/sections/:sectionId/skills/:skillId` | Remove skill from section |
| PATCH | `/api/resumes/:id/sections/:sectionId/skills/reorder` | Reorder skills within section |

### 2.3 Updated Resume Entry Endpoints

Existing entry endpoints (`POST /resumes/:id/entries`, etc.) now require `section_id` instead of `section` string:

```typescript
// Before:
{ perspective_id, section: 'experience', position: 0 }

// After:
{ perspective_id, section_id: 'section-uuid', position: 0 }
```

The `PATCH /resumes/:id/entries/reorder` body type changes from `{ entries: Array<{ id, section, position }> }` to `{ entries: Array<{ id, section_id, position }> }`.

### 2.4 Freeform Entry Creation

For `freeform` sections, entries are created without a `perspective_id`:

```
POST /api/resumes/:id/entries
{ section_id: 'section-uuid', content: 'Free text content here', position: 0 }
```

The `perspective_id` is null. The `content` is required (not copy-on-write — it's the primary content).

### 2.5 Updated IR Endpoint

`GET /api/resumes/:id/ir` now reads sections from `resume_sections` instead of deriving them from entry section strings. The IR compiler queries:

```sql
SELECT * FROM resume_sections WHERE resume_id = ? ORDER BY position
```

Then for each section, queries entries/skills based on `entry_type`.

### 2.6 Repository Changes

#### `getWithEntries` — LEFT JOIN required

The existing `getWithEntries` in `resume-repository.ts` uses `JOIN perspectives p ON p.id = re.perspective_id` (INNER JOIN). After freeform entries have `perspective_id = NULL`, they will be silently excluded. This must change to `LEFT JOIN`:

```sql
LEFT JOIN perspectives p ON p.id = re.perspective_id
```

The query must also SELECT `rs.id AS section_id, rs.title AS section_title, rs.entry_type, rs.position AS section_position` by JOINing `resume_sections rs ON re.section_id = rs.id`. The grouping key changes from `row.section` (a string) to `row.section_id` (a UUID).

#### `resolveContent` — LEFT JOIN required

`resolveContent` in `resume-entry-repository.ts` uses `JOIN perspectives p ON re.perspective_id = p.id` (INNER JOIN). Must change to `LEFT JOIN`. When `perspective_id IS NULL` (freeform), `resolveContent` should return `re.content` directly (it already does via `COALESCE(re.content, p.content)` — but with LEFT JOIN, `p.content` is NULL, so `COALESCE` returns `re.content`). This works correctly with LEFT JOIN.

#### `analyzeGaps` — null guard required

`analyzeGaps` in `resume-service.ts` iterates entries and calls `PerspectiveRepository.get(this.db, entry.perspective_id)`. For freeform entries where `perspective_id` is null, this must be guarded:

```typescript
if (!entry.perspective_id) continue  // skip freeform entries in gap analysis
```

#### `ResumeWithEntries` type change

`ResumeWithEntries.sections` type changes from `Record<string, Array<ResumeEntry & { perspective_content }>>` (keyed by section name string) to an array-of-sections structure which is more natural:

```typescript
interface ResumeWithEntries extends Resume {
  sections: Array<{
    id: string
    title: string
    entry_type: string
    position: number
    entries: Array<ResumeEntry & { perspective_content: string | null }>
  }>
}
```

---

## 3. IR Compiler Changes

### 3.1 Section-Driven Compilation

Currently the compiler has hardcoded section builders (`buildExperienceSection`, `buildSkillsSection`, etc.) called in a fixed order. After this change:

```typescript
export function compileResumeIR(db: Database, resumeId: string): ResumeDocument | null {
  const resume = getResume(db, resumeId)
  if (!resume) return null

  const header = parseHeader(resume)

  // Fetch sections from resume_sections table
  const sections = db.query(
    'SELECT * FROM resume_sections WHERE resume_id = ? ORDER BY position'
  ).all(resumeId) as ResumeSectionRow[]

  const irSections: IRSection[] = sections.map(section => {
    const items = buildSectionItems(db, section)
    return {
      id: section.id,  // real UUID from DB, not synthetic
      type: section.entry_type as IRSectionType,
      title: section.title,
      display_order: section.position,
      items,
    }
  })

  return { resume_id: resumeId, header, sections: irSections }
}

function buildSectionItems(db: Database, section: ResumeSectionRow): IRSectionItem[] {
  switch (section.entry_type) {
    case 'experience': return buildExperienceItems(db, section.id)
    case 'skills': return buildSkillItems(db, section.id)
    case 'education': return buildEducationItems(db, section.id)
    case 'projects': return buildProjectItems(db, section.id)
    case 'clearance': return buildClearanceItems(db, section.id)
    case 'presentations': return buildPresentationItems(db, section.id)
    case 'freeform': return buildFreeformItems(db, section.id)
    default: return []
  }
}
```

Section builders now receive `section_id` instead of `resume_id + section_name`. Queries change from `WHERE re.resume_id = ? AND re.section = 'experience'` to `WHERE re.section_id = ?`.

**Type changes:** Add `'freeform'` to `IRSectionType` in both `packages/core/src/types/index.ts` and `packages/sdk/src/types.ts`. The freeform type maps to free-text sections (summaries, custom text). The DragNDrop view, Markdown compiler, and LaTeX compiler must handle `'freeform'` sections — either via a dedicated renderer or by falling through to the existing summary/custom fallback. Also add `'certifications'` and `'awards'` to `IRSectionType` for consistency with the `entry_type` CHECK constraint.

### 3.2 Skills Section Builder

The skills section reads from `resume_skills` instead of deriving from `bullet_skills`:

```typescript
function buildSkillItems(db: Database, sectionId: string): IRSectionItem[] {
  const rows = db.query(`
    SELECT s.name, s.category
    FROM resume_skills rs
    JOIN skills s ON s.id = rs.skill_id
    WHERE rs.section_id = ?
    ORDER BY rs.position
  `).all(sectionId) as Array<{ name: string; category: string | null }>

  // Group by category
  const categoryMap = new Map<string, string[]>()
  for (const row of rows) {
    const cat = row.category ?? 'Other'
    if (!categoryMap.has(cat)) categoryMap.set(cat, [])
    categoryMap.get(cat)!.push(row.name)
  }

  if (categoryMap.size === 0) return []

  return [{
    kind: 'skill_group' as const,
    categories: [...categoryMap.entries()].map(([label, skills]) => ({ label, skills })),
  }]
}
```

### 3.3 Freeform Section Builder

```typescript
function buildFreeformItems(db: Database, sectionId: string): IRSectionItem[] {
  const rows = db.query(`
    SELECT re.id AS entry_id, re.content
    FROM resume_entries re
    WHERE re.section_id = ? AND re.perspective_id IS NULL
    ORDER BY re.position
  `).all(sectionId) as Array<{ entry_id: string; content: string | null }>

  return rows.filter(r => r.content).map(r => ({
    kind: 'summary' as const,  // reuse SummaryItem shape for freeform text
    content: r.content!,
    entry_id: r.entry_id,
  }))
}
```

---

## 4. DragNDrop View Changes

### 4.1 "Add Section" Button

At the bottom of the DnD view, a button that opens a dropdown of entry types not yet present:

```svelte
<div class="add-section">
  <button class="btn btn-add-section" onclick={() => showAddSection = !showAddSection}>
    + Add Section
  </button>
  {#if showAddSection}
    <div class="add-section-dropdown">
      {#each availableEntryTypes as entryType}
        <button class="dropdown-item" onclick={() => addSection(entryType)}>
          {entryType.label}
        </button>
      {/each}
    </div>
  {/if}
</div>
```

Where `availableEntryTypes` is derived from the set of entry types minus those already in the resume's sections. The user can add multiple sections of the same type (e.g., two experience sections) — the "not yet present" filter is a suggestion, not a hard limit. Include a "Custom (freeform)" option that always appears.

### 4.2 Section Header with Controls

Each section in the DnD view gets:
- Editable title (click to rename)
- Up/down arrows for reordering
- Delete button (with confirmation)
- Entry type badge (read-only, shows "experience", "skills", etc.)

### 4.3 Type-Specific Pickers

When "+ Add" is clicked within a section, the picker behavior depends on `entry_type`:

| entry_type | Picker |
|-----------|--------|
| `experience` | Perspective picker, filtered by source (existing — Phase 26) |
| `skills` | Skill picker — shows all skills grouped by category, checkboxes to add/remove |
| `education` | Source picker — shows sources with `source_type = 'education'`, adds as entry |
| `projects` | Source picker — shows sources with `source_type = 'project'` |
| `clearance` | Source picker — shows sources with `source_type = 'clearance'` |
| `presentations` | Source picker — shows sources or free text input |
| `freeform` | Text input — user types content directly |

**Per-role button scoping:** The per-role "+ Add from this role" button is inside the `{#each ir.sections as section}` loop. Each section has its own `section.id`. When there are two experience sections, each role appears under its parent section, and the button correctly uses the enclosing `section.id`. No ambiguity — the button is scoped to the section it's rendered in.

### 4.4 `onAddEntry` Callback — Critical Change

**Critical callback change:** The `onAddEntry` callback must pass the `section.id` (UUID from `resume_sections`) — NOT `section.type` (entry_type string). The current code passes `section.type` which is a string like `'experience'`. After migration, the API expects `section_id` (a UUID FK to `resume_sections`).

Updated prop signature:
```typescript
onAddEntry?: (sectionId: string, entryType: string, sourceId?: string, sourceLabel?: string) => void
```

The per-role button becomes:
```svelte
onclick={() => onAddEntry(section.id, section.type, sub.source_id ?? undefined, sub.title)}
```

The section-level button becomes:
```svelte
onclick={() => onAddEntry(section.id, section.type)}
```

The `openPicker` function receives both `sectionId` (for the API call) and `entryType` (to determine which picker to show):
```typescript
async function openPicker(sectionId: string, entryType: string, sourceId?: string, sourceLabel?: string) {
  pickerModal = { open: true, sectionId, entryType, sourceId: sourceId ?? null, sourceLabel: sourceLabel ?? null }
  // ... load perspectives/skills/sources based on entryType
}
```

The `addEntry` function sends `section_id` (not `section` string) to the API.

### 4.5 Skills Picker (New)

A modal/panel showing all skills from `forge.skills.list()`, grouped by category:

```
- AI/ML
  [x] Python    [x] TensorFlow    [ ] PyTorch
- Cloud
  [x] AWS    [x] Kubernetes    [ ] Azure
- Security
  [x] Splunk    [ ] SIEM    [x] Threat Hunting
```

Checkboxes add/remove skills from the section. Changes are persisted immediately via `POST/DELETE /api/resumes/:id/sections/:sectionId/skills`.

### 4.6 Source Picker (New)

For education/projects/clearance sections — shows sources of the matching type:

```
Select a source to add:
[ ] Western Governors University (B.S. Cybersecurity)
[ ] SANS Technology Institute (Graduate Certificate)
[ ] CompTIA Security+ (Certificate)
```

Adding a source creates a resume entry. If the source has approved perspectives, the user picks one. If not, a "direct add" creates an entry with `perspective_id = NULL` and `content` = source description.

---

## 5. Type Changes

### 5.1 `IRSectionType`

Add `'freeform'`, `'certifications'`, and `'awards'` to `IRSectionType` in both `packages/core/src/types/index.ts` and `packages/sdk/src/types.ts`:

```typescript
type IRSectionType = 'experience' | 'skills' | 'education' | 'projects'
  | 'clearance' | 'presentations' | 'certifications' | 'awards' | 'freeform'
```

### 5.2 `AddResumeEntry.perspective_id` — optional

In both `packages/core/src/types/index.ts` and `packages/sdk/src/types.ts`, change `AddResumeEntry.perspective_id` from `string` (required) to `string | undefined` (optional). Freeform entries are created without a perspective:

```typescript
interface AddResumeEntry {
  resume_id: string
  section_id: string        // was: section: string
  perspective_id?: string   // was: perspective_id: string (required)
  content?: string
  position?: number
}
```

---

## 6. Acceptance Criteria

### Schema
- [ ] `resume_sections` table with id, resume_id, title, entry_type, position, updated_at
- [ ] `resume_skills` table with section_id FK, skill_id FK
- [ ] `resume_entries.section` string column replaced by `section_id` FK
- [ ] `resume_entries.perspective_id` is nullable (for freeform entries)
- [ ] Existing entries migrated: section strings -> section entities
- [ ] `entry_type` is immutable after creation (CHECK constraint)
- [ ] Summary entries migrated: `perspective_id` nulled, content populated from perspective
- [ ] `PRAGMA foreign_keys = OFF/ON` wraps table rebuild in migration
- [ ] Skills section auto-populated from existing `bullet_skills` data during migration

### API
- [ ] Section CRUD endpoints (POST, GET list, PATCH, DELETE)
- [ ] Resume skills CRUD (POST, DELETE, reorder)
- [ ] Entry creation accepts `section_id` instead of `section` string
- [ ] Freeform entries can be created with `perspective_id = null` and `content`
- [ ] IR endpoint reads from `resume_sections` table

### Repository
- [ ] `getWithEntries` uses LEFT JOIN (freeform entries included)
- [ ] `resolveContent` uses LEFT JOIN (freeform entries included)
- [ ] `analyzeGaps` skips freeform entries (no crash on null `perspective_id`)
- [ ] `ResumeWithEntries` type updated to array-of-sections format
- [ ] `seedResumeEntry` updated to use `section_id`

### IR Compiler
- [ ] Compiler reads sections from `resume_sections` ordered by position
- [ ] Section builders receive `section_id`, not `resume_id + section name`
- [ ] Skills section reads from `resume_skills` instead of `bullet_skills`
- [ ] Freeform section renders text entries
- [ ] Sections with no entries produce empty IR sections (still rendered with "Add" prompt)
- [ ] `'freeform'` added to `IRSectionType`

### DragNDrop View
- [ ] "Add Section" button shows dropdown of entry types
- [ ] Section header shows editable title, entry type badge, up/down, delete
- [ ] Type-specific pickers open for each entry type
- [ ] Skills picker shows checkboxes grouped by category
- [ ] Source picker shows matching sources for education/projects/clearance
- [ ] Freeform picker shows text input
- [ ] Section reorder via up/down arrows persists position changes
- [ ] `onAddEntry` passes `section.id` (UUID) not `section.type` (string)

### Tests
- [ ] Migration applies cleanly, existing entries have correct section_id
- [ ] Section CRUD (create, list, update title, delete with cascade)
- [ ] Skills section: add/remove skills, verify IR output
- [ ] Freeform section: create entry with null perspective_id
- [ ] Experience section still works after migration (section_id instead of section string)
- [ ] Summary entries visible in freeform section after migration (perspective_id nulled)
- [ ] Two sections of same entry_type: entries appear in correct section
- [ ] `resolveContent` with freeform entry (perspective_id NULL) returns `content` directly
- [ ] `analyzeGaps` with freeform section doesn't crash
- [ ] Section cascade delete removes entries and skills
- [ ] Skills picker adds to correct section when multiple skills sections exist
- [ ] Freeform entry round-trip: create via API, GET IR, verify content appears

---

## 7. Dependencies & Parallelization

### Sequential Bottleneck
1. Schema migration (004) — must be first
2. Types + repository updates — after migration
3. IR compiler changes — after repositories
4. API routes — after services
5. UI changes — after API

### Parallel (after migration + types)
- Section repository/service (new)
- Resume skills repository (new)
- Resume entry repository update (section_id)
- IR compiler refactor

### Parallel (after API)
- DragNDrop section management UI
- Skills picker UI
- Source picker UI
- Freeform entry UI

### Files to Modify
- `packages/core/src/db/migrations/004_resume_sections.sql` — new migration
- `packages/core/src/types/index.ts` — ResumeSection entity, ResumeSkill, updated ResumeEntry, `AddResumeEntry.perspective_id` optional, `'freeform'`/`'certifications'`/`'awards'` added to `IRSectionType`
- `packages/sdk/src/types.ts` — same type updates
- `packages/core/src/db/repositories/resume-repository.ts` — section CRUD, `getWithEntries` LEFT JOIN, grouping key change to `section_id`
- `packages/core/src/db/repositories/resume-entry-repository.ts` — `section_id` instead of `section`, `resolveContent` LEFT JOIN
- `packages/core/src/db/__tests__/helpers.ts` — `seedResumeEntry` accepts `sectionId` instead of `section`, new `seedResumeSection` helper
- `packages/core/src/services/resume-service.ts` — section management, `analyzeGaps` null guard, `reorderEntries` section_id
- `packages/core/src/services/resume-compiler.ts` — section-driven compilation
- `packages/core/src/routes/resumes.ts` — section + skills endpoints
- `packages/sdk/src/resources/resumes.ts` — section + skills SDK methods
- `packages/webui/src/lib/components/resume/DragNDropView.svelte` — section management UI, `onAddEntry` callback fix
- `packages/webui/src/routes/resumes/+page.svelte` — skills picker, source picker, freeform input
- All existing test files that call `seedResumeEntry` — update to use `seedResumeSection` + `sectionId`

---

## 8. Known Limitations

- **No section drag-and-drop** — use up/down arrows for MVP. Drag would require `svelte-dnd-action` on sections as well as entries, adding complexity.
- **Skills section is not AI-derived** — skills go directly from the skills table to the resume, bypassing the derivation chain. This is intentional — skills are factual, not reframed.
- **Summary section uses `freeform` entry_type** — there's no special "summary" type. A summary is just a freeform section titled "Summary". The IR compiler maps `freeform` items to `SummaryItem` in the IR.
- **Education/clearance entries without perspectives** — if a source hasn't been through the derivation pipeline, it can still be added directly with `perspective_id = NULL` and content from the source description. This breaks the derivation chain provenance model but is practical for sections that don't need AI reframing.
- **Skills migration is best-effort** — The auto-populate step (1.3 step 4) derives skills from existing `bullet_skills` joins. If a resume's skills section had no entries with perspectives (unlikely but possible), the skills section will be empty after migration. Users can re-add skills via the picker. A banner in the UI should indicate when a skills section has 0 entries.
