# Phase 27: Schema Migration 004 + Core Layer (Section Entities)

**Status:** Planning
**Date:** 2026-03-30
**Spec:** [2026-03-30-resume-sections-as-entities.md](../refs/specs/2026-03-30-resume-sections-as-entities.md)
**Depends on:** Phase 26 (per-role bullet add — uses the current section model)
**Blocks:** Phase 28 (API routes + SDK + UI)

## Goal

Write migration 004, update types, repositories, services, and test helpers so that resume sections are first-class database entities with user-defined names and enforced entry types. After this phase, the core layer is fully migrated. NO API routes, SDK methods, or UI changes — those go in Phase 28.

## Scope

| Spec section | Covered here? |
|-------------|---------------|
| 1. Schema (migration 004) | Yes |
| 2.6. Repository changes | Yes |
| 3. IR compiler changes | Yes |
| 5. Type changes | Yes |
| 2.1-2.5. API routes | No (Phase 28) |
| 4. DragNDrop UI | No (Phase 28) |

## Files to Create

- `packages/core/src/db/migrations/004_resume_sections.sql`

## Files to Modify

- `packages/core/src/types/index.ts`
- `packages/sdk/src/types.ts`
- `packages/core/src/db/__tests__/helpers.ts`
- `packages/core/src/db/repositories/resume-repository.ts`
- `packages/core/src/db/repositories/resume-entry-repository.ts`
- `packages/core/src/services/resume-service.ts`
- `packages/core/src/services/resume-compiler.ts`
- `packages/core/src/db/repositories/__tests__/resume-repository.test.ts`
- `packages/core/src/db/repositories/__tests__/resume-entry-repository.test.ts`
- `packages/core/src/services/__tests__/resume-service.test.ts`
- `packages/core/src/services/__tests__/resume-compiler.test.ts`
- `packages/core/src/services/__tests__/perspective-service.test.ts`
- `packages/core/src/routes/__tests__/contracts.test.ts`
- `packages/core/src/routes/__tests__/resumes.test.ts`
- `packages/core/src/routes/__tests__/perspectives.test.ts`
- `packages/core/src/__tests__/e2e/e2e.test.ts`

---

## Tasks

### T27.1: Write `004_resume_sections.sql`

**File:** `packages/core/src/db/migrations/004_resume_sections.sql`

All 5 migration steps from spec section 1.3. The migration must be idempotent-safe (runs once, registered in `_migrations`).

```sql
-- Forge Resume Builder — Resume Sections as Entities
-- Migration: 004_resume_sections
-- Date: 2026-03-30
--
-- Promotes resume sections from hardcoded strings to first-class entities.
-- Creates resume_sections and resume_skills tables, migrates existing data,
-- and rebuilds resume_entries to replace the section string with section_id FK.
-- Builds on 003_renderer_and_entities.

-- ── New tables ──────────────────────────────────────────────────────────

-- Step 1: Create resume_sections table
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

-- Step 2: Create resume_skills table
CREATE TABLE resume_skills (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  section_id TEXT NOT NULL REFERENCES resume_sections(id) ON DELETE CASCADE,
  skill_id TEXT NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
  position INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  UNIQUE(section_id, skill_id)
) STRICT;

CREATE INDEX idx_resume_skills_section ON resume_skills(section_id, position);

-- ── Data migration ──────────────────────────────────────────────────────

-- Step 3: Add section_id column to resume_entries (temporary, nullable)
ALTER TABLE resume_entries ADD COLUMN section_id TEXT REFERENCES resume_sections(id) ON DELETE CASCADE;

-- Step 4: Convert summary entries to freeform
-- Summary entries were created with synthetic perspectives during v1 import.
-- Copy perspective content into entry content, null out perspective_id.
UPDATE resume_entries
SET content = (SELECT p.content FROM perspectives p WHERE p.id = resume_entries.perspective_id),
    perspective_id = NULL
WHERE section = 'summary'
  AND perspective_id IS NOT NULL;

-- Step 5: Create resume_sections from existing entries
-- One section per distinct (resume_id, section) pair.
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

-- Step 6: Update entries to reference their sections
-- Safe during migration: exactly one section per entry_type per resume.
UPDATE resume_entries SET section_id = (
  SELECT rs.id FROM resume_sections rs
  WHERE rs.resume_id = resume_entries.resume_id
  AND rs.entry_type = CASE resume_entries.section
    WHEN 'summary' THEN 'freeform'
    ELSE resume_entries.section
  END
);

-- Step 7: Auto-populate resume_skills from existing bullet_skills derivation
-- Without this, existing resumes would show empty skills sections after migration.
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

-- ── Table rebuild ───────────────────────────────────────────────────────

-- Step 8: Rebuild resume_entries — drop section column, enforce section_id NOT NULL,
-- make perspective_id nullable (for freeform entries).
-- Must disable FK enforcement during table rebuild (same pattern as migration 002).
PRAGMA foreign_keys = OFF;

-- **The DDL below includes ALL columns from the current schema. Do NOT omit `perspective_content_snapshot` or `notes` -- doing so silently drops data.**

CREATE TABLE resume_entries_new (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  resume_id TEXT NOT NULL REFERENCES resumes(id) ON DELETE CASCADE,
  section_id TEXT NOT NULL REFERENCES resume_sections(id) ON DELETE CASCADE,
  perspective_id TEXT REFERENCES perspectives(id) ON DELETE RESTRICT,
  content TEXT,
  perspective_content_snapshot TEXT,
  position INTEGER NOT NULL DEFAULT 0,
  notes TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

INSERT INTO resume_entries_new (id, resume_id, section_id, perspective_id, content, perspective_content_snapshot, position, notes, created_at, updated_at)
  SELECT id, resume_id, section_id, perspective_id, content, perspective_content_snapshot, position, notes, created_at, updated_at
  FROM resume_entries;

DROP TABLE resume_entries;
ALTER TABLE resume_entries_new RENAME TO resume_entries;

-- Recreate indexes
CREATE INDEX idx_resume_entries_section ON resume_entries(section_id, position);
CREATE INDEX idx_resume_entries_resume ON resume_entries(resume_id);

PRAGMA foreign_keys = ON;

-- ── Register migration ──────────────────────────────────────────────────

INSERT INTO _migrations (name) VALUES ('004_resume_sections');
```

**Key points:**
- Step 4 (summary conversion) must run BEFORE step 5 (section creation) so `section = 'summary'` maps to `entry_type = 'freeform'`
- Step 6 relies on the one-section-per-type guarantee from step 5
- Step 7 uses `GROUP BY rs.id, bs.skill_id` to deduplicate skills
- Step 8 drops `section` column while preserving `perspective_content_snapshot` and `notes` (the table rebuild defines the new schema explicitly)
- `PRAGMA foreign_keys = OFF/ON` wraps only the table rebuild, same pattern as migration 002

**Verify:** The table rebuild drops only the `section` column (replaced by `section_id` FK). The `perspective_content_snapshot` and `notes` columns are preserved in the new schema.

**Decision:** Keep `perspective_content_snapshot` and `notes` — they are used by the copy-on-write system in `ResumeRepository.updateEntry`. The table rebuild DDL above already includes them.

---

### T27.2: Update Types (core + SDK)

#### `packages/core/src/types/index.ts`

**Add new entity interfaces:**

```typescript
/** A resume section entity — defines a titled section with an entry type. */
export interface ResumeSectionEntity {
  id: string
  resume_id: string
  title: string
  entry_type: string
  position: number
  created_at: string
  updated_at: string
}

/** A skill pinned to a resume section (for skills-type sections). */
export interface ResumeSkill {
  id: string
  section_id: string
  skill_id: string
  position: number
  created_at: string
}

/** Input for creating a resume section. */
export interface CreateResumeSection {
  title: string
  entry_type: string
  position?: number
}

/** Input for updating a resume section. */
export interface UpdateResumeSection {
  title?: string
  position?: number
  // entry_type is immutable
}
```

**Update `ResumeEntry`:** Replace `section: string` with `section_id: string`, make `perspective_id` optional:

```typescript
// Before:
export interface ResumeEntry {
  id: string
  resume_id: string
  perspective_id: string
  content: string | null
  perspective_content_snapshot: string | null
  section: string
  position: number
  notes: string | null
  created_at: string
  updated_at: string
}

// After:
export interface ResumeEntry {
  id: string
  resume_id: string
  section_id: string
  perspective_id: string | null  // null for freeform entries
  content: string | null
  perspective_content_snapshot: string | null
  position: number
  notes: string | null
  created_at: string
  updated_at: string
}
```

**Update `AddResumeEntry`:**

```typescript
// Before:
export interface AddResumeEntry {
  perspective_id: string
  section: string
  position: number
  content?: string | null
  notes?: string | null
}

// After:
export interface AddResumeEntry {
  section_id: string
  perspective_id?: string    // optional — null for freeform
  content?: string | null
  position?: number
  notes?: string | null
}
```

**Update `ResumeWithEntries`:**

```typescript
// Before:
export interface ResumeWithEntries extends Resume {
  sections: Record<string, Array<ResumeEntry & { perspective_content: string }>>
}

// After:
export interface ResumeWithEntries extends Resume {
  sections: Array<{
    id: string
    title: string
    entry_type: string
    position: number
    entries: Array<ResumeEntry & { perspective_content: string | null }>
  }>
}
```

**Update `IRSectionType`:** Add `'freeform'`, remove `'summary'` and `'custom'` (summary is now freeform, custom is now freeform):

```typescript
// Before:
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

// After:
export type IRSectionType =
  | 'experience'
  | 'skills'
  | 'education'
  | 'projects'
  | 'certifications'
  | 'clearance'
  | 'presentations'
  | 'awards'
  | 'freeform'
```

**Note:** Keep `'summary'` and `'custom'` as aliases if existing markdown/LaTeX templates reference them. Safer to add `'freeform'` and keep the rest:

```typescript
export type IRSectionType =
  | 'summary'       // kept for backward compat — maps to freeform internally
  | 'experience'
  | 'skills'
  | 'education'
  | 'projects'
  | 'certifications'
  | 'clearance'
  | 'presentations'
  | 'awards'
  | 'custom'        // kept for backward compat
  | 'freeform'      // new
```

**Remove `ResumeSection` type alias** (the old union of section name strings). It conflicts with the new entity name. Rename the old one to `ResumeSectionName` if still needed for backward compat, or remove entirely since it's no longer used (sections are entities now, not string unions).

```typescript
// Remove:
export type ResumeSection = 'summary' | 'experience' | ...

// The ResumePerspective interface also uses ResumeSection — remove ResumePerspective too
// (it was deprecated in favor of ResumeEntry)
```

**Remove `ResumePerspective`** (deprecated, uses `section: ResumeSection`).

#### `packages/sdk/src/types.ts`

Mirror the same changes:
- Add `ResumeSectionEntity`, `ResumeSkill`, `CreateResumeSection`, `UpdateResumeSection`
- Update `ResumeEntry`: `section` -> `section_id`, `perspective_id` optional
- Update `AddResumeEntry`: `section` -> `section_id`, `perspective_id` optional
- Update `ResumeWithEntries`: `sections` from `Record<string, ...>` to `Array<{ id, title, entry_type, position, entries }>`
- Update `IRSectionType`: add `'freeform'`
- Update `UpdateResumeEntry`: `section` -> `section_id`

---

### T27.3: Update Test Helpers

**File:** `packages/core/src/db/__tests__/helpers.ts`

**New helper: `seedResumeSection`**

```typescript
/** Seed a test resume section and return its ID */
export function seedResumeSection(db: Database, resumeId: string, opts: {
  title?: string
  entryType?: string
  position?: number
} = {}): string {
  const id = testUuid()
  db.run(
    `INSERT INTO resume_sections (id, resume_id, title, entry_type, position)
     VALUES (?, ?, ?, ?, ?)`,
    [
      id,
      resumeId,
      opts.title ?? 'Experience',
      opts.entryType ?? 'experience',
      opts.position ?? 0,
    ]
  )
  return id
}
```

**Update `seedResumeEntry`:** Replace `section` parameter with `sectionId`, make `perspectiveId` optional:

```typescript
// Before:
export function seedResumeEntry(db: Database, resumeId: string, perspectiveId: string, opts: {
  section?: string
  position?: number
  content?: string | null
} = {}): string {
  const id = testUuid()
  db.run(
    `INSERT INTO resume_entries (id, resume_id, perspective_id, content, section, position)
     VALUES (?, ?, ?, ?, ?, ?)`,
    [id, resumeId, perspectiveId, opts.content ?? null, opts.section ?? 'experience', opts.position ?? 0]
  )
  return id
}

// After:
export function seedResumeEntry(db: Database, resumeId: string, sectionId: string, opts: {
  perspectiveId?: string | null
  position?: number
  content?: string | null
} = {}): string {
  const id = testUuid()
  db.run(
    `INSERT INTO resume_entries (id, resume_id, section_id, perspective_id, content, position)
     VALUES (?, ?, ?, ?, ?, ?)`,
    [id, resumeId, sectionId, opts.perspectiveId ?? null, opts.content ?? null, opts.position ?? 0]
  )
  return id
}
```

**Signature change summary:**
- Old: `seedResumeEntry(db, resumeId, perspectiveId, { section?, position?, content? })`
- New: `seedResumeEntry(db, resumeId, sectionId, { perspectiveId?, position?, content? })`

**New helper: `seedResumeSkill`**

```typescript
/** Seed a test resume skill and return its ID */
export function seedResumeSkill(db: Database, sectionId: string, skillId: string, opts: {
  position?: number
} = {}): string {
  const id = testUuid()
  db.run(
    `INSERT INTO resume_skills (id, section_id, skill_id, position)
     VALUES (?, ?, ?, ?)`,
    [id, sectionId, skillId, opts.position ?? 0]
  )
  return id
}
```

**New helper: `seedSkill`** (if not already present)

```typescript
/** Seed a test skill and return its ID */
export function seedSkill(db: Database, opts: {
  name?: string
  category?: string | null
} = {}): string {
  const id = testUuid()
  db.run(
    `INSERT INTO skills (id, name, category)
     VALUES (?, ?, ?)`,
    [id, opts.name ?? 'Python', opts.category ?? 'Languages']
  )
  return id
}
```

---

### T27.4: Update ALL Existing Test Files

Every test file that calls `seedResumeEntry` must be updated to:
1. First create a section via `seedResumeSection`
2. Pass the section ID to `seedResumeEntry` (new signature)
3. Move `perspectiveId` into the opts object

**Files and call sites to update:**

#### `packages/core/src/db/repositories/__tests__/resume-repository.test.ts`
~12 calls. Pattern:

```typescript
// Before:
const entryId = seedResumeEntry(db, resumeId, perspId)
seedResumeEntry(db, resumeId, p1, { section: 'work_history', position: 0 })

// After:
const sectionId = seedResumeSection(db, resumeId, { entryType: 'experience' })
const entryId = seedResumeEntry(db, resumeId, sectionId, { perspectiveId: perspId })

const whSection = seedResumeSection(db, resumeId, { title: 'Experience', entryType: 'experience' })
seedResumeEntry(db, resumeId, whSection, { perspectiveId: p1, position: 0 })
```

#### `packages/core/src/services/__tests__/resume-service.test.ts`
~12 calls. Same pattern. Also update `reorderEntries` calls: `{ id, section, position }` -> `{ id, section_id, position }`.

#### `packages/core/src/services/__tests__/resume-compiler.test.ts`
~14 calls. Each test creates entries with specific sections — create the section first, then pass section ID.

#### `packages/core/src/services/__tests__/perspective-service.test.ts`
1 call.

#### `packages/core/src/routes/__tests__/contracts.test.ts`
1 call.

#### `packages/core/src/routes/__tests__/resumes.test.ts`
1 call.

#### `packages/core/src/routes/__tests__/perspectives.test.ts`
1 call.

#### `packages/core/src/__tests__/e2e/e2e.test.ts`
1 call.

**Total:** ~50 call sites across 9 files.

**Note:** Run `grep -rn 'seedResumeEntry' packages/core/src/ | wc -l` to verify the exact count before starting. Every call site must be updated -- any missed site will hard-fail with a SQLite FK constraint violation.

**Pattern for each update:**

```typescript
// Before:
const sourceId = seedSource(db)
const bulletId = seedBullet(db, [{ id: sourceId }])
const perspId = seedPerspective(db, bulletId)
const resumeId = seedResume(db)
seedResumeEntry(db, resumeId, perspId, { section: 'experience', position: 0 })

// After:
const sourceId = seedSource(db)
const bulletId = seedBullet(db, [{ id: sourceId }])
const perspId = seedPerspective(db, bulletId)
const resumeId = seedResume(db)
const sectionId = seedResumeSection(db, resumeId, { title: 'Experience', entryType: 'experience' })
seedResumeEntry(db, resumeId, sectionId, { perspectiveId: perspId, position: 0 })
```

---

### T27.5: Update Repositories

#### `resume-repository.ts` — `getWithEntries`

**Change INNER JOIN to LEFT JOIN, add resume_sections JOIN, change grouping from string key to section entity:**

```typescript
// Update EntryJoinRow to include section entity fields
interface EntryJoinRow {
  entry_id: string
  section_id: string
  section_title: string
  section_entry_type: string
  section_position: number
  position: number
  entry_content: string | null
  perspective_content_snapshot: string | null
  entry_notes: string | null
  entry_created_at: string
  entry_updated_at: string
  perspective_id: string | null  // nullable for freeform
  bullet_id: string | null       // nullable for freeform
  perspective_content: string | null  // nullable for freeform
}
```

```typescript
getWithEntries(db: Database, id: string): ResumeWithEntries | null {
  const resume = ResumeRepository.get(db, id)
  if (!resume) return null

  const rows = db
    .query(
      `SELECT
         re.id AS entry_id,
         re.section_id,
         rs.title AS section_title,
         rs.entry_type AS section_entry_type,
         rs.position AS section_position,
         re.position,
         re.content AS entry_content,
         re.perspective_content_snapshot,
         re.notes AS entry_notes,
         re.created_at AS entry_created_at,
         re.updated_at AS entry_updated_at,
         re.perspective_id,
         p.bullet_id,
         p.content AS perspective_content
       FROM resume_entries re
       JOIN resume_sections rs ON rs.id = re.section_id
       LEFT JOIN perspectives p ON p.id = re.perspective_id
       WHERE re.resume_id = ?
       ORDER BY rs.position, re.position`,
    )
    .all(id) as EntryJoinRow[]

  // Also fetch sections that have no entries (so empty sections appear)
  const allSections = db
    .query(
      `SELECT id, title, entry_type, position
       FROM resume_sections WHERE resume_id = ? ORDER BY position`
    )
    .all(id) as Array<{ id: string; title: string; entry_type: string; position: number }>

  // Build sections array
  const sectionMap = new Map<string, {
    id: string; title: string; entry_type: string; position: number;
    entries: Array<ResumeEntry & { perspective_content: string | null }>
  }>()

  // Initialize all sections (even empty ones)
  for (const sec of allSections) {
    sectionMap.set(sec.id, {
      id: sec.id,
      title: sec.title,
      entry_type: sec.entry_type,
      position: sec.position,
      entries: [],
    })
  }

  // Populate entries
  for (const row of rows) {
    const section = sectionMap.get(row.section_id)
    if (!section) continue
    section.entries.push({
      id: row.entry_id,
      resume_id: id,
      section_id: row.section_id,
      perspective_id: row.perspective_id,
      content: row.entry_content,
      perspective_content_snapshot: row.perspective_content_snapshot,
      position: row.position,
      notes: row.entry_notes,
      created_at: row.entry_created_at,
      updated_at: row.entry_updated_at,
      perspective_content: row.perspective_content,
    })
  }

  const sections = [...sectionMap.values()].sort((a, b) => a.position - b.position)

  return { ...resume, sections }
}
```

#### `resume-repository.ts` — `addEntry`

Update to use `section_id` instead of `section`:

```typescript
addEntry(db: Database, resumeId: string, input: AddResumeEntry): ResumeEntry {
  const id = crypto.randomUUID()
  db.run(
    `INSERT INTO resume_entries (id, resume_id, section_id, perspective_id, content, position, notes)
     VALUES (?, ?, ?, ?, ?, ?, ?)`,
    [id, resumeId, input.section_id, input.perspective_id ?? null,
     input.content ?? null, input.position ?? 0, input.notes ?? null],
  )
  return db.query('SELECT * FROM resume_entries WHERE id = ?').get(id) as ResumeEntry
}
```

#### `resume-repository.ts` — `reorderEntries`

Change `section` to `section_id`:

```typescript
reorderEntries(db: Database, resumeId: string, entries: Array<{ id: string; section_id: string; position: number }>): void {
  const txn = db.transaction(() => {
    const stmt = db.prepare(
      `UPDATE resume_entries SET section_id = ?, position = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
       WHERE id = ? AND resume_id = ?`,
    )
    for (const entry of entries) {
      stmt.run(entry.section_id, entry.position, entry.id, resumeId)
    }
  })
  txn()
}
```

#### `resume-repository.ts` — `updateEntry`

Change `section` to `section_id` in the input type and SQL:

```typescript
updateEntry(db: Database, entryId: string, input: {
  content?: string | null
  section_id?: string
  position?: number
  notes?: string | null
}): ResumeEntry | null {
  // ... same logic, but replace:
  // if (input.section !== undefined) { sets.push('section = ?'); params.push(input.section) }
  // with:
  if (input.section_id !== undefined) { sets.push('section_id = ?'); params.push(input.section_id) }
  // ...
}
```

#### `resume-repository.ts` — New section CRUD methods

```typescript
// ── Section CRUD ─────────────────────────────────────────────────────

createSection(db: Database, resumeId: string, input: CreateResumeSection): ResumeSectionEntity {
  const id = crypto.randomUUID()
  const row = db
    .query(
      `INSERT INTO resume_sections (id, resume_id, title, entry_type, position)
       VALUES (?, ?, ?, ?, ?)
       RETURNING *`
    )
    .get(id, resumeId, input.title, input.entry_type, input.position ?? 0) as ResumeSectionEntity
  return row
},

listSections(db: Database, resumeId: string): ResumeSectionEntity[] {
  return db
    .query('SELECT * FROM resume_sections WHERE resume_id = ? ORDER BY position')
    .all(resumeId) as ResumeSectionEntity[]
},

getSection(db: Database, sectionId: string): ResumeSectionEntity | null {
  return db
    .query('SELECT * FROM resume_sections WHERE id = ?')
    .get(sectionId) as ResumeSectionEntity | null
},

updateSection(db: Database, sectionId: string, input: UpdateResumeSection): ResumeSectionEntity | null {
  const sets: string[] = []
  const params: unknown[] = []

  if (input.title !== undefined) { sets.push('title = ?'); params.push(input.title) }
  if (input.position !== undefined) { sets.push('position = ?'); params.push(input.position) }

  if (sets.length === 0) {
    return ResumeRepository.getSection(db, sectionId)
  }

  sets.push("updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')")
  params.push(sectionId)

  return db
    .query(`UPDATE resume_sections SET ${sets.join(', ')} WHERE id = ? RETURNING *`)
    .get(...params) as ResumeSectionEntity | null
},

deleteSection(db: Database, sectionId: string): boolean {
  const result = db.run('DELETE FROM resume_sections WHERE id = ?', [sectionId])
  return result.changes > 0
},

// ── Resume Skills ────────────────────────────────────────────────────

addSkill(db: Database, sectionId: string, skillId: string, position?: number): ResumeSkill {
  const id = crypto.randomUUID()
  const pos = position ?? (db.query(
    'SELECT COALESCE(MAX(position), -1) + 1 AS next_pos FROM resume_skills WHERE section_id = ?'
  ).get(sectionId) as { next_pos: number }).next_pos

  return db
    .query(
      `INSERT INTO resume_skills (id, section_id, skill_id, position)
       VALUES (?, ?, ?, ?)
       RETURNING *`
    )
    .get(id, sectionId, skillId, pos) as ResumeSkill
},

removeSkill(db: Database, sectionId: string, skillId: string): boolean {
  const result = db.run(
    'DELETE FROM resume_skills WHERE section_id = ? AND skill_id = ?',
    [sectionId, skillId]
  )
  return result.changes > 0
},

listSkills(db: Database, sectionId: string): Array<ResumeSkill & { name: string; category: string | null }> {
  return db
    .query(
      `SELECT rs.*, s.name, s.category
       FROM resume_skills rs
       JOIN skills s ON s.id = rs.skill_id
       WHERE rs.section_id = ?
       ORDER BY rs.position`
    )
    .all(sectionId) as Array<ResumeSkill & { name: string; category: string | null }>
},

reorderSkills(db: Database, sectionId: string, skillIds: string[]): void {
  const txn = db.transaction(() => {
    const stmt = db.prepare(
      'UPDATE resume_skills SET position = ? WHERE section_id = ? AND skill_id = ?'
    )
    skillIds.forEach((skillId, i) => stmt.run(i, sectionId, skillId))
  })
  txn()
},
```

#### `resume-entry-repository.ts` — `resolveContent`

Change INNER JOIN to LEFT JOIN:

```typescript
// Before:
`SELECT COALESCE(re.content, p.content) AS resolved_content
 FROM resume_entries re
 JOIN perspectives p ON re.perspective_id = p.id
 WHERE re.id = ?`

// After:
`SELECT COALESCE(re.content, p.content) AS resolved_content
 FROM resume_entries re
 LEFT JOIN perspectives p ON re.perspective_id = p.id
 WHERE re.id = ?`
```

This works correctly for freeform entries: `p.content` is NULL, so COALESCE returns `re.content`.

#### `resume-entry-repository.ts` — `create`

Update to accept `section_id` instead of `section`, make `perspective_id` optional:

```typescript
export interface CreateResumeEntryInput {
  resume_id: string
  section_id: string           // was: section: string
  perspective_id?: string | null  // was: required string
  content?: string | null
  position?: number
  notes?: string | null
}
```

The `create` function in `resume-entry-repository.ts` must also be updated:
- INSERT column list: replace `section` with `section_id`
- `input.section` -> `input.section_id`
- Handle `perspective_id` being null/undefined for freeform entries (skip snapshot capture when null)

Update the INSERT statement accordingly.

#### `resume-entry-repository.ts` — `listByResume`

Replace `ORDER BY section ASC` with `ORDER BY section_id`:

```typescript
export function listByResume(db: Database, resumeId: string): ResumeEntry[] {
  return db
    .query(
      `SELECT re.* FROM resume_entries re
       JOIN resume_sections rs ON rs.id = re.section_id
       WHERE re.resume_id = ?
       ORDER BY rs.position ASC, re.position ASC`
    )
    .all(resumeId) as ResumeEntry[]
}
```

#### `resume-entry-repository.ts` — `update`

Replace `section` with `section_id`:

```typescript
export interface UpdateResumeEntryInput {
  content?: string | null
  section_id?: string      // was: section?: string
  position?: number
  notes?: string | null
}
```

---

### T27.6: Update Services

#### `resume-service.ts` — `analyzeGaps`

Add null guard for freeform entries:

```typescript
analyzeGaps(resumeId: string): Result<GapAnalysis> {
  const resume = ResumeRepository.getWithEntries(this.db, resumeId)
  if (!resume) {
    return { ok: false, error: { code: 'NOT_FOUND', message: `Resume ${resumeId} not found` } }
  }

  const includedDomains = new Map<string, number>()
  let perspectivesIncluded = 0

  // Iterate the new array-of-sections format
  for (const section of resume.sections) {
    for (const entry of section.entries) {
      // Skip freeform entries (no perspective)
      if (!entry.perspective_id) continue

      perspectivesIncluded++
      const perspective = PerspectiveRepository.get(this.db, entry.perspective_id)
      if (perspective?.domain) {
        includedDomains.set(perspective.domain, (includedDomains.get(perspective.domain) ?? 0) + 1)
      }
    }
  }
  // ... rest stays the same
}
```

#### `resume-service.ts` — `addEntry`

Update to handle optional `perspective_id` for freeform entries:

```typescript
addEntry(resumeId: string, input: AddResumeEntry): Result<ResumeEntry> {
  const resume = ResumeRepository.get(this.db, resumeId)
  if (!resume) {
    return { ok: false, error: { code: 'NOT_FOUND', message: `Resume ${resumeId} not found` } }
  }

  // Verify section exists and belongs to this resume
  const section = ResumeRepository.getSection(this.db, input.section_id)
  if (!section || section.resume_id !== resumeId) {
    return { ok: false, error: { code: 'NOT_FOUND', message: `Section ${input.section_id} not found` } }
  }

  // For perspective-based entries, verify perspective is approved
  if (input.perspective_id) {
    const perspective = PerspectiveRepository.get(this.db, input.perspective_id)
    if (!perspective) {
      return { ok: false, error: { code: 'NOT_FOUND', message: `Perspective ${input.perspective_id} not found` } }
    }
    if (perspective.status !== 'approved') {
      return {
        ok: false,
        error: { code: 'VALIDATION_ERROR', message: 'Only approved perspectives can be added to resumes' },
      }
    }
  } else {
    // Freeform entry — content is required
    if (!input.content) {
      return {
        ok: false,
        error: { code: 'VALIDATION_ERROR', message: 'Content is required for freeform entries' },
      }
    }
  }

  try {
    const entry = ResumeRepository.addEntry(this.db, resumeId, input)
    return { ok: true, data: entry }
  } catch (err: unknown) {
    const message = err instanceof Error ? err.message : String(err)
    if (message.includes('UNIQUE constraint')) {
      return { ok: false, error: { code: 'CONFLICT', message: 'Entry already exists' } }
    }
    throw err
  }
}
```

#### `resume-service.ts` — `reorderEntries`

Change `section` to `section_id`:

```typescript
reorderEntries(resumeId: string, entries: Array<{ id: string; section_id: string; position: number }>): Result<void> {
  // ... validate resume exists ...

  const existing = ResumeRepository.getWithEntries(this.db, resumeId)
  if (!existing) { ... }

  const existingIds = new Set<string>()
  for (const section of existing.sections) {
    for (const e of section.entries) {
      existingIds.add(e.id)
    }
  }

  for (const item of entries) {
    if (!existingIds.has(item.id)) {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: `Entry ${item.id} is not in this resume` } }
    }
  }

  ResumeRepository.reorderEntries(this.db, resumeId, entries)
  return { ok: true, data: undefined }
}
```

#### `resume-service.ts` — New section CRUD methods

```typescript
createSection(resumeId: string, input: CreateResumeSection): Result<ResumeSectionEntity> {
  const resume = ResumeRepository.get(this.db, resumeId)
  if (!resume) {
    return { ok: false, error: { code: 'NOT_FOUND', message: `Resume ${resumeId} not found` } }
  }
  const section = ResumeRepository.createSection(this.db, resumeId, input)
  return { ok: true, data: section }
}

listSections(resumeId: string): Result<ResumeSectionEntity[]> {
  const resume = ResumeRepository.get(this.db, resumeId)
  if (!resume) {
    return { ok: false, error: { code: 'NOT_FOUND', message: `Resume ${resumeId} not found` } }
  }
  return { ok: true, data: ResumeRepository.listSections(this.db, resumeId) }
}

updateSection(resumeId: string, sectionId: string, input: UpdateResumeSection): Result<ResumeSectionEntity> {
  const section = ResumeRepository.getSection(this.db, sectionId)
  if (!section || section.resume_id !== resumeId) {
    return { ok: false, error: { code: 'NOT_FOUND', message: `Section ${sectionId} not found` } }
  }
  const updated = ResumeRepository.updateSection(this.db, sectionId, input)
  if (!updated) {
    return { ok: false, error: { code: 'NOT_FOUND', message: `Section ${sectionId} not found` } }
  }
  return { ok: true, data: updated }
}

deleteSection(resumeId: string, sectionId: string): Result<void> {
  const section = ResumeRepository.getSection(this.db, sectionId)
  if (!section || section.resume_id !== resumeId) {
    return { ok: false, error: { code: 'NOT_FOUND', message: `Section ${sectionId} not found` } }
  }
  ResumeRepository.deleteSection(this.db, sectionId)
  return { ok: true, data: undefined }
}

addSkill(resumeId: string, sectionId: string, skillId: string): Result<ResumeSkill> {
  const section = ResumeRepository.getSection(this.db, sectionId)
  if (!section || section.resume_id !== resumeId) {
    return { ok: false, error: { code: 'NOT_FOUND', message: `Section ${sectionId} not found` } }
  }
  if (section.entry_type !== 'skills') {
    return { ok: false, error: { code: 'VALIDATION_ERROR', message: 'Skills can only be added to skills sections' } }
  }
  try {
    const skill = ResumeRepository.addSkill(this.db, sectionId, skillId)
    return { ok: true, data: skill }
  } catch (err: unknown) {
    const message = err instanceof Error ? err.message : String(err)
    if (message.includes('UNIQUE constraint')) {
      return { ok: false, error: { code: 'CONFLICT', message: 'Skill already in this section' } }
    }
    throw err
  }
}

removeSkill(resumeId: string, sectionId: string, skillId: string): Result<void> {
  const section = ResumeRepository.getSection(this.db, sectionId)
  if (!section || section.resume_id !== resumeId) {
    return { ok: false, error: { code: 'NOT_FOUND', message: `Section ${sectionId} not found` } }
  }
  const removed = ResumeRepository.removeSkill(this.db, sectionId, skillId)
  if (!removed) {
    return { ok: false, error: { code: 'NOT_FOUND', message: 'Skill not found in this section' } }
  }
  return { ok: true, data: undefined }
}

reorderSkills(resumeId: string, sectionId: string, skillIds: string[]): Result<void> {
  // Verify section belongs to resume and is a skills section
  const section = ResumeRepository.getSection(this.db, sectionId)
  if (!section || section.resume_id !== resumeId) {
    return { ok: false, error: { code: 'NOT_FOUND', message: 'Section not found' } }
  }
  if (section.entry_type !== 'skills') {
    return { ok: false, error: { code: 'VALIDATION_ERROR', message: 'Section is not a skills section' } }
  }
  ResumeRepository.reorderSkills(this.db, sectionId, skillIds)
  return { ok: true, data: undefined }
}

listSkillsForSection(sectionId: string): Array<ResumeSkill & { name: string; category: string | null }> {
  return ResumeRepository.listSkills(this.db, sectionId)
}
```

---

### T27.7: Update IR Compiler

**File:** `packages/core/src/services/resume-compiler.ts`

Replace the hardcoded section builder dispatch with a section-driven approach.

#### Main function — `compileResumeIR`

```typescript
interface ResumeSectionRow {
  id: string
  title: string
  entry_type: string
  position: number
}

export function compileResumeIR(db: Database, resumeId: string): ResumeDocument | null {
  const resume = db
    .query('SELECT id, name, target_role, header FROM resumes WHERE id = ?')
    .get(resumeId) as ResumeRow | null

  if (!resume) return null

  const header = parseHeader(resume)

  // Fetch sections from resume_sections table (not hardcoded)
  const sectionRows = db
    .query('SELECT id, title, entry_type, position FROM resume_sections WHERE resume_id = ? ORDER BY position')
    .all(resumeId) as ResumeSectionRow[]

  const sections: IRSection[] = sectionRows.map(section => ({
    id: section.id,              // real UUID from DB, not synthetic
    type: section.entry_type as IRSectionType,
    title: section.title,
    display_order: section.position,
    items: buildSectionItems(db, section),
  }))

  return { resume_id: resumeId, header, sections }
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

#### Section builders — query change

All section builders change their WHERE clause from `WHERE re.resume_id = ? AND re.section = 'experience'` to `WHERE re.section_id = ?`.

**Experience builder:**

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
  // ... same grouping logic ...
}
```

**Skills builder — reads from `resume_skills` instead of `bullet_skills`:**

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
    .all(sectionId) as SkillRow[]

  if (rows.length === 0) return []

  const catMap = new Map<string, string[]>()
  for (const row of rows) {
    const cat = row.category ?? 'Other'
    if (!catMap.has(cat)) catMap.set(cat, [])
    catMap.get(cat)!.push(row.skill_name)
  }

  return [{
    kind: 'skill_group',
    categories: Array.from(catMap.entries()).map(([label, skills]) => ({ label, skills })),
  }]
}
```

**Freeform builder (NEW):**

```typescript
function buildFreeformItems(db: Database, sectionId: string): SummaryItem[] {
  const rows = db
    .query(
      `SELECT re.id AS entry_id, re.content
       FROM resume_entries re
       WHERE re.section_id = ? AND re.perspective_id IS NULL
       ORDER BY re.position`
    )
    .all(sectionId) as Array<{ entry_id: string; content: string | null }>

  return rows
    .filter(r => r.content)
    .map(r => ({
      kind: 'summary' as const,  // reuse SummaryItem shape
      content: r.content!,
      entry_id: r.entry_id,
    }))
}
```

**Other builders** (education, projects, clearance, presentations): same pattern — change `WHERE re.resume_id = ? AND re.section = 'X'` to `WHERE re.section_id = ?`. The function signature changes from `(db, resumeId)` to `(db, sectionId)`.

#### Remove `syntheticUUID`

No longer needed for section IDs (sections have real UUIDs). Keep it only for `subheading` and `org` synthetic IDs within experience groups. Export it if compiler tests use it.

---

### T27.8: Tests

#### Migration test

**File:** `packages/core/src/db/__tests__/migration-004.test.ts` (new file)

```typescript
import { describe, test, expect } from 'bun:test'
import { Database } from 'bun:sqlite'
import { getDatabase } from '../connection'
import { runMigrations } from '../migrate'
import { resolve } from 'path'

const MIGRATIONS_DIR = resolve(import.meta.dir, '../migrations')

describe('Migration 004: resume_sections', () => {
  test('applies cleanly on empty database', () => {
    const db = getDatabase(':memory:')
    runMigrations(db, MIGRATIONS_DIR)

    // resume_sections table exists
    const tables = db.query(
      "SELECT name FROM sqlite_master WHERE type='table' AND name='resume_sections'"
    ).all()
    expect(tables.length).toBe(1)

    // resume_skills table exists
    const skillTables = db.query(
      "SELECT name FROM sqlite_master WHERE type='table' AND name='resume_skills'"
    ).all()
    expect(skillTables.length).toBe(1)

    // resume_entries has section_id, no section column
    const cols = db.query("PRAGMA table_info(resume_entries)").all() as Array<{ name: string }>
    const colNames = cols.map(c => c.name)
    expect(colNames).toContain('section_id')
    expect(colNames).not.toContain('section')
    // perspective_id should exist but be nullable
    const perspCol = cols.find(c => c.name === 'perspective_id') as any
    expect(perspCol.notnull).toBe(0)
  })

  test('migrates existing data correctly', () => {
    // Migration tests must use a special pattern: run migrations 001-003,
    // insert pre-004 data using RAW SQL (not helpers -- helpers target
    // the post-004 schema), then apply migration 004, then assert.

    // 1. Create db with only migrations 001-003
    const db = getDatabase(':memory:')
    runMigrations(db, MIGRATIONS_DIR, { upTo: '003_renderer_and_entities' })

    // 2. Insert pre-004 data using raw SQL (old schema)
    const resumeId = crypto.randomUUID()
    db.run('INSERT INTO resumes (id, name, target_role, target_employer, archetype) VALUES (?, ?, ?, ?, ?)',
      [resumeId, 'Test Resume', 'Engineer', 'Acme', 'agentic-ai'])

    const perspId = crypto.randomUUID()
    // ... insert perspective, bullet, source ...

    db.run('INSERT INTO resume_entries (id, resume_id, perspective_id, section, position) VALUES (?, ?, ?, ?, ?)',
      [crypto.randomUUID(), resumeId, perspId, 'experience', 0])
    db.run('INSERT INTO resume_entries (id, resume_id, perspective_id, section, position) VALUES (?, ?, ?, ?, ?)',
      [crypto.randomUUID(), resumeId, perspId, 'summary', 0])

    // 3. Apply migration 004
    runMigrations(db, MIGRATIONS_DIR)  // runs only 004 since 001-003 are already applied

    // 4. Assert
    const sections = db.query('SELECT * FROM resume_sections WHERE resume_id = ?').all(resumeId)
    expect(sections.length).toBe(2)  // experience + freeform(summary)

    const entries = db.query('SELECT * FROM resume_entries WHERE resume_id = ?').all(resumeId)
    expect(entries.every(e => e.section_id !== null)).toBe(true)
    expect(entries.every(e => e.section === undefined)).toBe(true)  // column dropped

    // Summary entry should be freeform (perspective_id null, content set)
    const summarySection = sections.find(s => s.entry_type === 'freeform')
    const summaryEntries = db.query('SELECT * FROM resume_entries WHERE section_id = ?').all(summarySection.id)
    expect(summaryEntries[0].perspective_id).toBeNull()
    expect(summaryEntries[0].content).toBeTruthy()

    db.close()

    // Note: If the migration runner doesn't support `upTo`, apply the
    // migrations manually by reading each .sql file.
  })

  test('summary entries migrated to freeform', () => {
    // Same pattern as above: run 001-003, insert summary entries
    // with perspective_id set, run 004, verify:
    //   - perspective_id = NULL
    //   - content = perspective's original content
    //   - Section entry_type = 'freeform', title = 'Summary'
    const db = getDatabase(':memory:')
    runMigrations(db, MIGRATIONS_DIR, { upTo: '003_renderer_and_entities' })

    const resumeId = crypto.randomUUID()
    db.run('INSERT INTO resumes (id, name, target_role, target_employer, archetype) VALUES (?, ?, ?, ?, ?)',
      [resumeId, 'Test', 'Eng', 'Co', 'agentic-ai'])

    const bulletId = crypto.randomUUID()
    // ... insert source, bullet ...

    const perspId = crypto.randomUUID()
    db.run('INSERT INTO perspectives (id, bullet_id, content, status) VALUES (?, ?, ?, ?)',
      [perspId, bulletId, 'Summary perspective content', 'approved'])

    db.run('INSERT INTO resume_entries (id, resume_id, perspective_id, section, position) VALUES (?, ?, ?, ?, ?)',
      [crypto.randomUUID(), resumeId, perspId, 'summary', 0])

    runMigrations(db, MIGRATIONS_DIR)

    const sections = db.query('SELECT * FROM resume_sections WHERE resume_id = ?').all(resumeId)
    const freeformSection = sections.find(s => s.entry_type === 'freeform')
    expect(freeformSection).toBeDefined()
    expect(freeformSection.title).toBe('Summary')

    const entries = db.query('SELECT * FROM resume_entries WHERE section_id = ?').all(freeformSection.id)
    expect(entries[0].perspective_id).toBeNull()
    expect(entries[0].content).toBe('Summary perspective content')

    db.close()
  })

  test('skills section auto-populated from bullet_skills', () => {
    // Same pattern: run 001-003, insert resume with experience entries
    // whose perspectives have bullet_skills, run 004, verify resume_skills
    const db = getDatabase(':memory:')
    runMigrations(db, MIGRATIONS_DIR, { upTo: '003_renderer_and_entities' })

    const resumeId = crypto.randomUUID()
    db.run('INSERT INTO resumes (id, name, target_role, target_employer, archetype) VALUES (?, ?, ?, ?, ?)',
      [resumeId, 'Test', 'Eng', 'Co', 'agentic-ai'])

    const skillId = crypto.randomUUID()
    db.run('INSERT INTO skills (id, name, category) VALUES (?, ?, ?)',
      [skillId, 'Python', 'Languages'])

    // ... insert source, bullet, bullet_skills, perspective, resume_entry with section='skills' ...

    runMigrations(db, MIGRATIONS_DIR)

    const sections = db.query('SELECT * FROM resume_sections WHERE resume_id = ?').all(resumeId)
    const skillsSection = sections.find(s => s.entry_type === 'skills')
    expect(skillsSection).toBeDefined()

    const resumeSkills = db.query('SELECT * FROM resume_skills WHERE section_id = ?').all(skillsSection.id)
    expect(resumeSkills.length).toBeGreaterThan(0)
    expect(resumeSkills.some(rs => rs.skill_id === skillId)).toBe(true)

    db.close()
  })

  test('cascade delete works: delete section removes entries and skills', () => {
    const db = getDatabase(':memory:')
    runMigrations(db, MIGRATIONS_DIR)

    // Create resume, section, entry, skill
    const resumeId = crypto.randomUUID()
    db.run("INSERT INTO resumes (id, name, target_role, target_employer, archetype) VALUES (?, 'Test', 'Eng', 'Co', 'ai')", [resumeId])

    const sectionId = crypto.randomUUID()
    db.run("INSERT INTO resume_sections (id, resume_id, title, entry_type, position) VALUES (?, ?, 'Experience', 'experience', 0)", [sectionId, resumeId])

    const entryId = crypto.randomUUID()
    db.run("INSERT INTO resume_entries (id, resume_id, section_id, position) VALUES (?, ?, ?, 0)", [entryId, resumeId, sectionId])

    // Delete section
    db.run('DELETE FROM resume_sections WHERE id = ?', [sectionId])

    // Entry should be gone (CASCADE)
    const entry = db.query('SELECT * FROM resume_entries WHERE id = ?').get(entryId)
    expect(entry).toBeNull()
  })
})
```

#### Repository tests

**File:** `packages/core/src/db/repositories/__tests__/resume-repository.test.ts`

Update all existing tests (see T27.4 for pattern). Add new tests:

```typescript
describe('Section CRUD', () => {
  test('createSection returns entity with UUID', () => {
    const resumeId = seedResume(db)
    const section = ResumeRepository.createSection(db, resumeId, {
      title: 'Work History',
      entry_type: 'experience',
      position: 0,
    })
    expect(section.id).toHaveLength(36)
    expect(section.title).toBe('Work History')
    expect(section.entry_type).toBe('experience')
  })

  test('listSections returns ordered by position', () => {
    const resumeId = seedResume(db)
    ResumeRepository.createSection(db, resumeId, { title: 'Skills', entry_type: 'skills', position: 1 })
    ResumeRepository.createSection(db, resumeId, { title: 'Summary', entry_type: 'freeform', position: 0 })

    const sections = ResumeRepository.listSections(db, resumeId)
    expect(sections[0].title).toBe('Summary')
    expect(sections[1].title).toBe('Skills')
  })

  test('updateSection changes title, position unchanged', () => {
    const resumeId = seedResume(db)
    const section = ResumeRepository.createSection(db, resumeId, { title: 'Old', entry_type: 'experience' })
    const updated = ResumeRepository.updateSection(db, section.id, { title: 'New Title' })
    expect(updated!.title).toBe('New Title')
    expect(updated!.entry_type).toBe('experience')  // immutable
  })

  test('deleteSection cascades to entries', () => {
    const resumeId = seedResume(db)
    const sectionId = seedResumeSection(db, resumeId, { entryType: 'experience' })
    const sourceId = seedSource(db)
    const bulletId = seedBullet(db, [{ id: sourceId }])
    const perspId = seedPerspective(db, bulletId)
    seedResumeEntry(db, resumeId, sectionId, { perspectiveId: perspId })

    ResumeRepository.deleteSection(db, sectionId)
    const entries = db.query('SELECT * FROM resume_entries WHERE section_id = ?').all(sectionId)
    expect(entries).toHaveLength(0)
  })
})

describe('Resume Skills', () => {
  test('addSkill and listSkills', () => {
    const resumeId = seedResume(db)
    const sectionId = seedResumeSection(db, resumeId, { title: 'Skills', entryType: 'skills' })
    const skillId = seedSkill(db, { name: 'Python', category: 'Languages' })

    const rs = ResumeRepository.addSkill(db, sectionId, skillId)
    expect(rs.skill_id).toBe(skillId)

    const skills = ResumeRepository.listSkills(db, sectionId)
    expect(skills).toHaveLength(1)
    expect(skills[0].name).toBe('Python')
    expect(skills[0].category).toBe('Languages')
  })

  test('removeSkill', () => {
    const resumeId = seedResume(db)
    const sectionId = seedResumeSection(db, resumeId, { title: 'Skills', entryType: 'skills' })
    const skillId = seedSkill(db, { name: 'Go' })

    ResumeRepository.addSkill(db, sectionId, skillId)
    const removed = ResumeRepository.removeSkill(db, sectionId, skillId)
    expect(removed).toBe(true)

    const skills = ResumeRepository.listSkills(db, sectionId)
    expect(skills).toHaveLength(0)
  })

  test('duplicate skill returns UNIQUE constraint error', () => {
    const resumeId = seedResume(db)
    const sectionId = seedResumeSection(db, resumeId, { title: 'Skills', entryType: 'skills' })
    const skillId = seedSkill(db, { name: 'Rust' })

    ResumeRepository.addSkill(db, sectionId, skillId)
    expect(() => ResumeRepository.addSkill(db, sectionId, skillId)).toThrow(/UNIQUE/)
  })
})

describe('getWithEntries with freeform', () => {
  test('includes freeform entries with null perspective', () => {
    const resumeId = seedResume(db)
    const sectionId = seedResumeSection(db, resumeId, { title: 'Summary', entryType: 'freeform' })
    seedResumeEntry(db, resumeId, sectionId, { content: 'I am a software engineer.' })

    const resume = ResumeRepository.getWithEntries(db, resumeId)
    expect(resume).not.toBeNull()
    expect(resume!.sections).toHaveLength(1)
    expect(resume!.sections[0].entry_type).toBe('freeform')
    expect(resume!.sections[0].entries).toHaveLength(1)
    expect(resume!.sections[0].entries[0].content).toBe('I am a software engineer.')
    expect(resume!.sections[0].entries[0].perspective_id).toBeNull()
  })

  test('returns sections as array (not Record)', () => {
    const resumeId = seedResume(db)
    const s1 = seedResumeSection(db, resumeId, { title: 'Experience', entryType: 'experience', position: 0 })
    const s2 = seedResumeSection(db, resumeId, { title: 'Skills', entryType: 'skills', position: 1 })

    const resume = ResumeRepository.getWithEntries(db, resumeId)
    expect(Array.isArray(resume!.sections)).toBe(true)
    expect(resume!.sections).toHaveLength(2)
    expect(resume!.sections[0].title).toBe('Experience')
    expect(resume!.sections[1].title).toBe('Skills')
  })

  test('includes empty sections (no entries)', () => {
    const resumeId = seedResume(db)
    seedResumeSection(db, resumeId, { title: 'Empty Section', entryType: 'experience' })

    const resume = ResumeRepository.getWithEntries(db, resumeId)
    expect(resume!.sections).toHaveLength(1)
    expect(resume!.sections[0].entries).toHaveLength(0)
  })
})
```

#### Service tests

```typescript
describe('analyzeGaps with freeform', () => {
  test('does not crash on freeform entries (null perspective_id)', () => {
    const resumeId = seedResume(db)
    const sectionId = seedResumeSection(db, resumeId, { title: 'Summary', entryType: 'freeform' })
    seedResumeEntry(db, resumeId, sectionId, { content: 'A summary.' })

    const service = new ResumeService(db)
    const result = service.analyzeGaps(resumeId)
    expect(result.ok).toBe(true)
  })
})

describe('addEntry with section_id', () => {
  test('validates section exists', () => {
    const resumeId = seedResume(db)
    const service = new ResumeService(db)
    const result = service.addEntry(resumeId, {
      section_id: 'nonexistent-uuid-aaaa-bbbb-ccccddddeeee',
      perspective_id: 'some-id',
      position: 0,
    })
    expect(result.ok).toBe(false)
    expect((result as any).error.code).toBe('NOT_FOUND')
  })

  test('allows freeform entry with null perspective_id and content', () => {
    const resumeId = seedResume(db)
    const sectionId = seedResumeSection(db, resumeId, { title: 'Summary', entryType: 'freeform' })
    const service = new ResumeService(db)
    const result = service.addEntry(resumeId, {
      section_id: sectionId,
      content: 'Hello world',
      position: 0,
    })
    expect(result.ok).toBe(true)
    if (result.ok) {
      expect(result.data.perspective_id).toBeNull()
      expect(result.data.content).toBe('Hello world')
    }
  })

  test('rejects freeform entry without content', () => {
    const resumeId = seedResume(db)
    const sectionId = seedResumeSection(db, resumeId, { title: 'Summary', entryType: 'freeform' })
    const service = new ResumeService(db)
    const result = service.addEntry(resumeId, {
      section_id: sectionId,
      position: 0,
    })
    expect(result.ok).toBe(false)
  })
})
```

#### Compiler tests

```typescript
describe('section-driven compilation', () => {
  test('reads sections from resume_sections table', () => {
    const resumeId = seedResume(db)
    const sectionId = seedResumeSection(db, resumeId, { title: 'My Experience', entryType: 'experience', position: 0 })
    // ... seed source, bullet, perspective, entry ...
    seedResumeEntry(db, resumeId, sectionId, { perspectiveId: perspId, position: 0 })

    const ir = compileResumeIR(db, resumeId)
    expect(ir).not.toBeNull()
    expect(ir!.sections[0].id).toBe(sectionId)  // real UUID, not synthetic
    expect(ir!.sections[0].title).toBe('My Experience')
    expect(ir!.sections[0].type).toBe('experience')
  })

  test('skills section reads from resume_skills', () => {
    const resumeId = seedResume(db)
    const sectionId = seedResumeSection(db, resumeId, { title: 'Skills', entryType: 'skills', position: 0 })
    const skillId = seedSkill(db, { name: 'Python', category: 'Languages' })
    seedResumeSkill(db, sectionId, skillId)

    const ir = compileResumeIR(db, resumeId)
    expect(ir!.sections).toHaveLength(1)
    const skillGroup = ir!.sections[0].items[0] as SkillGroup
    expect(skillGroup.kind).toBe('skill_group')
    expect(skillGroup.categories[0].label).toBe('Languages')
    expect(skillGroup.categories[0].skills).toContain('Python')
  })

  test('freeform section renders text entries', () => {
    const resumeId = seedResume(db)
    const sectionId = seedResumeSection(db, resumeId, { title: 'Summary', entryType: 'freeform', position: 0 })
    seedResumeEntry(db, resumeId, sectionId, { content: 'Experienced engineer...', position: 0 })

    const ir = compileResumeIR(db, resumeId)
    expect(ir!.sections).toHaveLength(1)
    expect(ir!.sections[0].type).toBe('freeform')
    const item = ir!.sections[0].items[0] as SummaryItem
    expect(item.kind).toBe('summary')
    expect(item.content).toBe('Experienced engineer...')
  })

  test('empty sections appear in IR', () => {
    const resumeId = seedResume(db)
    seedResumeSection(db, resumeId, { title: 'Empty', entryType: 'experience', position: 0 })

    const ir = compileResumeIR(db, resumeId)
    expect(ir!.sections).toHaveLength(1)
    expect(ir!.sections[0].items).toHaveLength(0)
  })

  test('two sections of same entry_type — entries in correct section', () => {
    const resumeId = seedResume(db)
    const s1 = seedResumeSection(db, resumeId, { title: 'Civilian Work', entryType: 'experience', position: 0 })
    const s2 = seedResumeSection(db, resumeId, { title: 'Military Service', entryType: 'experience', position: 1 })

    // Seed entries into each section
    const sourceId = seedSource(db)
    const bulletId = seedBullet(db, [{ id: sourceId }])
    const p1 = seedPerspective(db, bulletId, { content: 'Civilian bullet' })
    const p2 = seedPerspective(db, bulletId, { content: 'Military bullet' })
    seedResumeEntry(db, resumeId, s1, { perspectiveId: p1, position: 0 })
    seedResumeEntry(db, resumeId, s2, { perspectiveId: p2, position: 0 })

    const ir = compileResumeIR(db, resumeId)
    expect(ir!.sections).toHaveLength(2)
    expect(ir!.sections[0].title).toBe('Civilian Work')
    expect(ir!.sections[1].title).toBe('Military Service')
    // Each section has exactly 1 item
    expect(ir!.sections[0].items).toHaveLength(1)
    expect(ir!.sections[1].items).toHaveLength(1)
  })
})
```

#### `resolveContent` with freeform

```typescript
test('resolveContent with freeform entry returns content directly', () => {
  const resumeId = seedResume(db)
  const sectionId = seedResumeSection(db, resumeId, { entryType: 'freeform' })
  const entryId = seedResumeEntry(db, resumeId, sectionId, { content: 'Direct text' })

  const resolved = resolveContent(db, entryId)
  expect(resolved).toBe('Direct text')
})
```

---

### T27.9: Documentation

- [ ] Add JSDoc to all new repository methods (`createSection`, `addSkill`, etc.)
- [ ] Add JSDoc to new service methods
- [ ] Update any existing JSDoc that references `section` string parameters
- [ ] Update `refs/schemas/erd.md` (if it exists) to include `resume_sections` and `resume_skills` tables with FK relationships
- [ ] Update `docs/src/data/models/entity-types.md` to include `ResumeSectionEntity` and `ResumeSkill` interfaces

---

## Acceptance Criteria

- [ ] Migration 004 applies cleanly on fresh database (001-004)
- [ ] Migration 004 correctly migrates existing data (summary -> freeform, skills auto-populated)
- [ ] `resume_entries.section` string column replaced by `section_id` FK
- [ ] `resume_entries.perspective_id` is nullable (for freeform entries)
- [ ] `getWithEntries` returns array-of-sections format (not `Record<string, ...>`)
- [ ] `getWithEntries` uses LEFT JOIN (freeform entries included)
- [ ] `resolveContent` uses LEFT JOIN (freeform entries included)
- [ ] `analyzeGaps` skips freeform entries (no crash on null `perspective_id`)
- [ ] Section CRUD: create, list, update title, delete with cascade
- [ ] Skills CRUD: add, remove, list, reorder
- [ ] IR compiler reads from `resume_sections` table
- [ ] Skills section reads from `resume_skills` instead of `bullet_skills`
- [ ] Freeform section renders text entries
- [ ] `seedResumeEntry` updated to new signature (all ~50 call sites across 9 files)
- [ ] `seedResumeSection` and `seedResumeSkill` helpers created
- [ ] All existing tests pass after updates
- [ ] New tests for migration, sections, skills, freeform, compiler
- [ ] Types in sync between `@forge/core` and `@forge/sdk`

## Estimated Effort

| Task | Lines changed (est.) |
|------|---------------------|
| T27.1 Migration SQL | ~120 |
| T27.2 Types (core + SDK) | ~100 |
| T27.3 Test helpers | ~60 |
| T27.4 Test file updates (~50 call sites) | ~200 |
| T27.5 Repositories | ~250 |
| T27.6 Services | ~150 |
| T27.7 IR compiler | ~200 |
| T27.8 New tests | ~250 |
| **Total** | **~1,330** |
