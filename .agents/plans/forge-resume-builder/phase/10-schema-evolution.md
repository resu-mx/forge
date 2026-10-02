# Phase 10: Schema Migration (002_schema_evolution.sql)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Evolve the Forge database schema from v1 (employers/projects/single-source-per-bullet) to v2 (organizations/polymorphic-sources/junction-based-bullet-sources/resume-entries) via a single migration file, then update all TypeScript types and test infrastructure to match.

**Architecture:** The migration (`002_schema_evolution.sql`) applies on top of `001_initial.sql` as a transactional DDL script. It creates new tables, migrates existing data, drops deprecated tables/columns, and registers itself in `_migrations`. The test helper file (`helpers.ts`) and all existing test files are then updated to use the new schema. TypeScript interfaces in `types/index.ts` are updated to match.

**Tech Stack:** SQLite (via Bun's bundled 3.45+), TypeScript, `bun:test`

**Depends on:** Phases 0-9 (existing schema + repository/service/route layer)
**Blocks:** Phase 11 (core layer updates), Phase 12 (API updates), Phase 13 (SDK/CLI updates)
**Parallelizable:** T10.2, T10.3, and T10.4 can run in parallel after T10.1 is complete. Within T10.3, individual test file updates are independent.

---

## Context

The codebase currently has:
- `001_initial.sql` with tables: `employers`, `projects`, `sources` (with `employer_id`, `project_id`), `bullets` (with `source_id`), `bullet_technologies`, `perspectives`, `skills`, `bullet_skills`, `perspective_skills`, `resumes`, `resume_perspectives`, `prompt_logs`, `_migrations`
- Repository layer: `EmployerRepository`, `ProjectRepository`, `SourceRepository`, `BulletRepository`, `PerspectiveRepository`, `ResumeRepository`, `SkillRepository`, `PromptLogRepository`
- Service layer: `SourceService`, `BulletService`, `PerspectiveService`, `DerivationService`, `ResumeService`, `AuditService`, `ReviewService`
- Test helpers: `createTestDb()`, `testUuid()`, `seedEmployer()`, `seedProject()`, `seedSource()`, `seedBullet()`, `seedPerspective()`, `seedResume()`
- ~294 tests across repositories, services, routes, and e2e

After this phase:
- `employers` and `projects` tables are gone, replaced by `organizations`
- `sources` is polymorphic via `source_type` + extension tables (`source_roles`, `source_projects`, `source_education`, `source_clearances`)
- `bullets.source_id` is gone, replaced by `bullet_sources` junction table
- `resume_perspectives` is gone, replaced by `resume_entries` with copy-on-write semantics
- `user_notes` + `note_references` tables exist for cross-entity notes
- `v1_import_map` exists for idempotent v1 data import
- All existing tests pass with updated helpers and assertions

## Goals

- Write `002_schema_evolution.sql` that applies cleanly on top of `001_initial.sql`
- Execute all 20 DDL steps from spec section 4 without error
- Migrate existing data: `employers` -> `organizations`, `bullets.source_id` -> `bullet_sources`, `resume_perspectives` -> `resume_entries`
- Update test helpers to use new schema
- Update all existing tests to pass with the new schema
- Update TypeScript type definitions to match new schema

## Non-Goals

- Updating repository implementations (that is Phase 11)
- Updating service implementations (that is Phase 11)
- Updating route handlers (that is Phase 12)
- Updating SDK types (that is Phase 13)
- Writing the v1 data import CLI command (separate phase)
- Writing new repository/service tests (Phase 11)

## Fallback Strategies

- If `ALTER TABLE DROP COLUMN` fails on a specific SQLite version, use the table-rebuild pattern throughout (CREATE temp, INSERT SELECT, DROP original, RENAME)
- If the migration is too complex for a single file and causes transaction issues, split into `002a_schema_evolution_tables.sql`, `002b_schema_evolution_data.sql`, `002c_schema_evolution_cleanup.sql` -- the migration runner handles multiple files in lexicographic order
- If a test file has too many interdependencies to update incrementally, rewrite the test file from scratch using the same structure but new helpers
- If the `RETURNING *` clause fails after table rebuild, use `INSERT ... ; SELECT * FROM table WHERE id = ?` pattern instead

---

## Tasks

### Task 10.1: Write 002_schema_evolution.sql migration

**Files:**
- Create: `packages/core/src/db/migrations/002_schema_evolution.sql`

**Goal:** Create the DDL migration that transforms the schema from v1 to v2, following the exact 20-step order from spec section 4.

- [ ] Create the file at `packages/core/src/db/migrations/002_schema_evolution.sql` with the header comment:

```sql
-- Forge Resume Builder — Schema Evolution
-- Migration: 002_schema_evolution
-- Date: 2026-03-29
--
-- Evolves schema from v1 (employers/projects/source_id) to v2
-- (organizations/polymorphic sources/bullet_sources junction/resume_entries).
-- Follows DDL order from spec section 4 (20 steps).
```

- [ ] **Step 1: Create `organizations` table.** Must come before any extension tables that reference it.

```sql
-- Step 1: Create organizations table (replaces employers)
CREATE TABLE organizations (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  name TEXT NOT NULL,
  org_type TEXT DEFAULT 'company' CHECK (org_type IN (
    'company', 'nonprofit', 'government', 'military',
    'education', 'volunteer', 'freelance', 'other'
  )),
  industry TEXT,
  size TEXT,
  worked INTEGER NOT NULL DEFAULT 0,
  employment_type TEXT CHECK (employment_type IN (
    'civilian', 'contractor', 'military_active',
    'military_reserve', 'volunteer', 'intern', NULL
  )),
  location TEXT,
  headquarters TEXT,
  website TEXT,
  linkedin_url TEXT,
  glassdoor_url TEXT,
  glassdoor_rating REAL,
  reputation_notes TEXT,
  notes TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;
```

- [ ] **Step 2: Migrate `employers` -> `organizations`.** Copy all employer rows with `worked=1`, `org_type='company'`.

```sql
-- Step 2: Migrate employers -> organizations
INSERT INTO organizations (id, name, worked, org_type, created_at, updated_at)
  SELECT id, name, 1, 'company', created_at, created_at
  FROM employers;
```

- [ ] **Step 3: Add `source_type` and `notes` to `sources`.**

```sql
-- Step 3: Add source_type and notes to sources
ALTER TABLE sources ADD COLUMN source_type TEXT NOT NULL DEFAULT 'general'
  CHECK (source_type IN ('role', 'project', 'education', 'clearance', 'general'));
ALTER TABLE sources ADD COLUMN notes TEXT;
```

- [ ] **Step 4: Create extension tables** (`source_roles`, `source_projects`, `source_education`, `source_clearances`).

```sql
-- Step 4: Create source extension tables
CREATE TABLE source_roles (
  source_id TEXT PRIMARY KEY CHECK(typeof(source_id) = 'text' AND length(source_id) = 36)
    REFERENCES sources(id) ON DELETE CASCADE,
  organization_id TEXT REFERENCES organizations(id) ON DELETE SET NULL,
  start_date TEXT,
  end_date TEXT,
  is_current INTEGER NOT NULL DEFAULT 0,
  work_arrangement TEXT,
  base_salary INTEGER,
  total_comp_notes TEXT
) STRICT;

CREATE TABLE source_projects (
  source_id TEXT PRIMARY KEY CHECK(typeof(source_id) = 'text' AND length(source_id) = 36)
    REFERENCES sources(id) ON DELETE CASCADE,
  organization_id TEXT REFERENCES organizations(id) ON DELETE SET NULL,
  is_personal INTEGER NOT NULL DEFAULT 0,
  url TEXT,
  start_date TEXT,
  end_date TEXT
) STRICT;

CREATE TABLE source_education (
  source_id TEXT PRIMARY KEY CHECK(typeof(source_id) = 'text' AND length(source_id) = 36)
    REFERENCES sources(id) ON DELETE CASCADE,
  education_type TEXT NOT NULL CHECK (education_type IN ('degree', 'certificate', 'course', 'self_taught')),
  institution TEXT,
  field TEXT,
  start_date TEXT,
  end_date TEXT,
  is_in_progress INTEGER NOT NULL DEFAULT 0,
  credential_id TEXT,
  expiration_date TEXT,
  issuing_body TEXT,
  url TEXT
) STRICT;

CREATE TABLE source_clearances (
  source_id TEXT PRIMARY KEY CHECK(typeof(source_id) = 'text' AND length(source_id) = 36)
    REFERENCES sources(id) ON DELETE CASCADE,
  level TEXT NOT NULL,
  polygraph TEXT,
  status TEXT,
  sponsoring_agency TEXT,
  investigation_date TEXT,
  adjudication_date TEXT,
  reinvestigation_date TEXT,
  read_on TEXT
) STRICT;
```

- [ ] **Step 5: Migrate existing `sources.employer_id` data into `source_roles`.** For any source that has an `employer_id`, create a `source_roles` row linking to the organization. Also update the source's `source_type` to `'role'`.

```sql
-- Step 5: Migrate sources with employer_id to source_roles
-- Mark sources that had an employer_id as role type
UPDATE sources SET source_type = 'role' WHERE employer_id IS NOT NULL;

-- Create source_roles rows for sources that had employer_id
INSERT INTO source_roles (source_id, organization_id, start_date, end_date)
  SELECT id, employer_id, start_date, end_date
  FROM sources
  WHERE employer_id IS NOT NULL;
```

- [ ] **Step 6: Drop `sources.employer_id` and `sources.project_id` columns** via table rebuild pattern.

> **IMPORTANT:** The migration runner's connection sets `PRAGMA foreign_keys = ON`. Table rebuilds (DROP + RENAME) temporarily break FK references, so foreign key enforcement MUST be explicitly disabled around the rebuild and re-enabled afterward.

```sql
-- Step 6: Drop employer_id and project_id from sources (table rebuild)
-- Disable FK enforcement during table rebuild to avoid constraint violations
-- when DROP TABLE invalidates FK references from other tables.
PRAGMA foreign_keys = OFF;

CREATE TABLE sources_new (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  title TEXT NOT NULL,
  description TEXT NOT NULL,
  source_type TEXT NOT NULL DEFAULT 'general'
    CHECK (source_type IN ('role', 'project', 'education', 'clearance', 'general')),
  start_date TEXT,
  end_date TEXT,
  status TEXT NOT NULL DEFAULT 'draft' CHECK(status IN ('draft', 'approved', 'deriving')),
  updated_by TEXT NOT NULL DEFAULT 'human' CHECK(updated_by IN ('human', 'ai')),
  last_derived_at TEXT,
  notes TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

INSERT INTO sources_new (id, title, description, source_type, start_date, end_date, status, updated_by, last_derived_at, notes, created_at, updated_at)
  SELECT id, title, description, source_type, start_date, end_date, status, updated_by, last_derived_at, notes, created_at, updated_at
  FROM sources;

DROP TABLE sources;
ALTER TABLE sources_new RENAME TO sources;

-- Recreate indexes on sources
CREATE INDEX idx_sources_status ON sources(status);
CREATE INDEX idx_sources_type ON sources(source_type);

-- Re-enable FK enforcement after rebuild
PRAGMA foreign_keys = ON;
```

- [ ] **Step 7: Drop `employers` table.**

```sql
-- Step 7: Drop employers table (data migrated to organizations in step 2)
DROP TABLE employers;
```

- [ ] **Step 8: Drop `projects` table** (subsumed by `source_projects`).

```sql
-- Step 8: Drop projects table (subsumed by source_projects)
DROP INDEX idx_projects_employer;
DROP TABLE projects;
```

- [ ] **Step 9: Create `bullet_sources` junction table.**

```sql
-- Step 9: Create bullet_sources junction table
CREATE TABLE bullet_sources (
  bullet_id TEXT NOT NULL CHECK(typeof(bullet_id) = 'text' AND length(bullet_id) = 36)
    REFERENCES bullets(id) ON DELETE CASCADE,
  source_id TEXT NOT NULL CHECK(typeof(source_id) = 'text' AND length(source_id) = 36)
    REFERENCES sources(id) ON DELETE CASCADE,
  is_primary INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (bullet_id, source_id)
) STRICT;

CREATE INDEX idx_bullet_sources_source ON bullet_sources(source_id);
```

- [ ] **Step 10: Migrate `bullets.source_id` -> `bullet_sources`.** Every existing bullet's `source_id` becomes a `bullet_sources` row with `is_primary=1`.

```sql
-- Step 10: Migrate bullets.source_id -> bullet_sources
INSERT INTO bullet_sources (bullet_id, source_id, is_primary)
  SELECT id, source_id, 1
  FROM bullets;
```

- [ ] **Step 11: Drop `bullets.source_id` column** via table rebuild.

> **IMPORTANT:** Same as Step 6 -- must disable FK enforcement during table rebuild.

```sql
-- Step 11: Drop source_id from bullets (table rebuild)
-- Disable FK enforcement during table rebuild (see Step 6 note).
PRAGMA foreign_keys = OFF;

CREATE TABLE bullets_new (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  content TEXT NOT NULL,
  source_content_snapshot TEXT NOT NULL,
  metrics TEXT,
  status TEXT NOT NULL DEFAULT 'pending_review' CHECK(status IN ('draft', 'pending_review', 'approved', 'rejected')),
  rejection_reason TEXT,
  prompt_log_id TEXT REFERENCES prompt_logs(id) ON DELETE SET NULL,
  approved_at TEXT,
  approved_by TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

INSERT INTO bullets_new (id, content, source_content_snapshot, metrics, status, rejection_reason, prompt_log_id, approved_at, approved_by, created_at)
  SELECT id, content, source_content_snapshot, metrics, status, rejection_reason, prompt_log_id, approved_at, approved_by, created_at
  FROM bullets;

DROP TABLE bullets;
ALTER TABLE bullets_new RENAME TO bullets;

-- Recreate indexes on bullets
CREATE INDEX idx_bullets_status ON bullets(status);

-- Re-enable FK enforcement after rebuild
PRAGMA foreign_keys = ON;
```

- [ ] **Step 12: Add `notes` column to `bullets`.**

```sql
-- Step 12: Add notes to bullets
ALTER TABLE bullets ADD COLUMN notes TEXT;
```

- [ ] **Step 13: Add `domain` column to `bullets`** (nullable TEXT, for v1 framing import).

```sql
-- Step 13: Add domain to bullets (for v1 framing import)
ALTER TABLE bullets ADD COLUMN domain TEXT;
```

- [ ] **Step 14: Add `notes` column to `perspectives` and `resumes`.**

```sql
-- Step 14: Add notes to perspectives, resumes
ALTER TABLE perspectives ADD COLUMN notes TEXT;
ALTER TABLE resumes ADD COLUMN notes TEXT;
```

- [ ] **Step 15: Create `resume_entries` table.**

```sql
-- Step 15: Create resume_entries table (replaces resume_perspectives)
CREATE TABLE resume_entries (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  resume_id TEXT NOT NULL REFERENCES resumes(id) ON DELETE CASCADE,
  perspective_id TEXT NOT NULL REFERENCES perspectives(id) ON DELETE RESTRICT,
  content TEXT,
  perspective_content_snapshot TEXT,
  section TEXT NOT NULL,
  position INTEGER NOT NULL DEFAULT 0,
  notes TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

CREATE INDEX idx_resume_entries_resume ON resume_entries(resume_id, section, position);
```

- [ ] **Step 16: Migrate `resume_perspectives` -> `resume_entries`** with `content=NULL` (reference mode). Each row gets a new UUID.

```sql
-- Step 16: Migrate resume_perspectives -> resume_entries
INSERT INTO resume_entries (id, resume_id, perspective_id, content, section, position)
  SELECT
    lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)),2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)),2) || '-' || hex(randomblob(6))),
    resume_id,
    perspective_id,
    NULL,
    section,
    position
  FROM resume_perspectives;
```

- [ ] **Step 17: Drop `resume_perspectives` table.**

```sql
-- Step 17: Drop resume_perspectives (data migrated to resume_entries)
DROP INDEX idx_resume_perspectives_resume;
DROP TABLE resume_perspectives;
```

- [ ] **Step 18: Create `user_notes` + `note_references` tables.**

```sql
-- Step 18: Create user_notes and note_references
CREATE TABLE user_notes (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  title TEXT,
  content TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

CREATE TABLE note_references (
  note_id TEXT NOT NULL CHECK(typeof(note_id) = 'text' AND length(note_id) = 36)
    REFERENCES user_notes(id) ON DELETE CASCADE,
  entity_type TEXT NOT NULL CHECK (entity_type IN (
    'source', 'bullet', 'perspective', 'resume_entry',
    'resume', 'skill', 'organization'
  )),
  entity_id TEXT NOT NULL,
  PRIMARY KEY (note_id, entity_type, entity_id)
) STRICT;

CREATE INDEX idx_note_refs_entity ON note_references(entity_type, entity_id);
```

- [ ] **Step 19: Create `v1_import_map` table.**

```sql
-- Step 19: Create v1_import_map for idempotent v1 data import
CREATE TABLE v1_import_map (
  v1_entity_type TEXT NOT NULL,
  v1_id INTEGER NOT NULL,
  forge_id TEXT NOT NULL,
  imported_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  PRIMARY KEY (v1_entity_type, v1_id)
) STRICT;
```

- [ ] **Step 20: Add `notes` column to `skills`.**

```sql
-- Step 20: Add notes to skills
ALTER TABLE skills ADD COLUMN notes TEXT;
```

- [ ] **Register migration in `_migrations`.**

```sql
-- Register this migration
INSERT INTO _migrations (name) VALUES ('002_schema_evolution');
```

- [ ] **Verify:** Run the migration against a fresh in-memory database. Use ES module `import` syntax (not `require()` -- Bun uses ESM):

```bash
cd /Users/adam/notes/job-hunting
bun -e "
  import { getDatabase } from './packages/core/src/db/connection';
  import { runMigrations } from './packages/core/src/db/migrate';
  import { resolve } from 'path';
  const db = getDatabase(':memory:');
  runMigrations(db, resolve('./packages/core/src/db/migrations'));
  const tables = db.query(\"SELECT name FROM sqlite_master WHERE type='table' ORDER BY name\").all();
  console.log('Tables:', tables.map(t => t.name));
  const migrations = db.query('SELECT name FROM _migrations ORDER BY id').all();
  console.log('Migrations:', migrations.map(m => m.name));
  db.close();
"
```

Alternatively (and more reliably), just run the test suite -- it applies all migrations on every test run:

```bash
cd /Users/adam/notes/job-hunting
bun test packages/core
```

Expected output should include tables: `bullet_sources`, `bullet_skills`, `bullet_technologies`, `bullets`, `note_references`, `organizations`, `perspectives`, `perspective_skills`, `prompt_logs`, `resume_entries`, `resumes`, `skills`, `source_clearances`, `source_education`, `source_projects`, `source_roles`, `sources`, `user_notes`, `v1_import_map`, `_migrations`.

Should NOT include: `employers`, `projects`, `resume_perspectives`.

---

### Task 10.2: Update test helpers

**Files:**
- Modify: `packages/core/src/db/__tests__/helpers.ts`

**Goal:** Update all seed functions to match the new schema so that test setup code works correctly.

- [ ] **Remove `seedEmployer` function.** Replace with `seedOrganization`.

```typescript
/** Seed a test organization and return its ID */
export function seedOrganization(db: Database, opts: {
  name?: string
  orgType?: string
  worked?: number
  employmentType?: string
  location?: string
} = {}): string {
  const id = testUuid()
  db.run(
    `INSERT INTO organizations (id, name, org_type, worked, employment_type, location)
     VALUES (?, ?, ?, ?, ?, ?)`,
    [
      id,
      opts.name ?? 'Test Corp',
      opts.orgType ?? 'company',
      opts.worked ?? 1,
      opts.employmentType ?? null,
      opts.location ?? null,
    ]
  )
  return id
}
```

- [ ] **Remove `seedProject` function entirely.** Projects are now created via `seedSource` with `sourceType: 'project'` + an entry in `source_projects`.

- [ ] **Update `seedSource` function.** Remove `employerId`/`projectId` params. Add `sourceType` param and optional extension data.

```typescript
/** Seed a test source and return its ID */
export function seedSource(db: Database, opts: {
  title?: string
  description?: string
  status?: string
  sourceType?: string
  // Extension data for 'role' type
  organizationId?: string
  isCurrent?: number
  workArrangement?: string
  // Extension data for 'project' type
  isPersonal?: number
  url?: string
  // Extension data for 'education' type
  educationType?: string
  institution?: string
  field?: string
  // Extension data for 'clearance' type
  level?: string
} = {}): string {
  const id = testUuid()
  const sourceType = opts.sourceType ?? 'general'

  db.run(
    `INSERT INTO sources (id, title, description, source_type, status)
     VALUES (?, ?, ?, ?, ?)`,
    [
      id,
      opts.title ?? 'Test Source',
      opts.description ?? 'Led a team of 4 engineers to migrate cloud forensics platform.',
      sourceType,
      opts.status ?? 'approved',
    ]
  )

  // Insert extension row based on source_type
  if (sourceType === 'role') {
    db.run(
      `INSERT INTO source_roles (source_id, organization_id, is_current, work_arrangement)
       VALUES (?, ?, ?, ?)`,
      [id, opts.organizationId ?? null, opts.isCurrent ?? 0, opts.workArrangement ?? null]
    )
  } else if (sourceType === 'project') {
    db.run(
      `INSERT INTO source_projects (source_id, organization_id, is_personal, url)
       VALUES (?, ?, ?, ?)`,
      [id, opts.organizationId ?? null, opts.isPersonal ?? 0, opts.url ?? null]
    )
  } else if (sourceType === 'education') {
    db.run(
      `INSERT INTO source_education (source_id, education_type, institution, field)
       VALUES (?, ?, ?, ?)`,
      [id, opts.educationType ?? 'certificate', opts.institution ?? null, opts.field ?? null]
    )
  } else if (sourceType === 'clearance') {
    db.run(
      `INSERT INTO source_clearances (source_id, level)
       VALUES (?, ?)`,
      [id, opts.level ?? 'SECRET']
    )
  }

  return id
}
```

- [ ] **Update `seedBullet` function.** Remove `sourceId` parameter. Add `sourceIds` parameter (array of `{id, isPrimary}`) and insert into `bullet_sources`.

```typescript
/** Seed a test bullet and return its ID */
export function seedBullet(db: Database, sourceIds: Array<{ id: string; isPrimary?: boolean }>, opts: {
  content?: string
  status?: string
  technologies?: string[]
  domain?: string
} = {}): string {
  const id = testUuid()
  db.run(
    `INSERT INTO bullets (id, content, source_content_snapshot, status, domain)
     VALUES (?, ?, ?, ?, ?)`,
    [
      id,
      opts.content ?? 'Led 4-engineer team migrating cloud forensics platform from ELK to AWS OpenSearch',
      'snapshot of source content',
      opts.status ?? 'approved',
      opts.domain ?? null,
    ]
  )

  // Insert bullet_sources junction rows
  for (const src of sourceIds) {
    db.run(
      'INSERT INTO bullet_sources (bullet_id, source_id, is_primary) VALUES (?, ?, ?)',
      [id, src.id, src.isPrimary !== false ? 1 : 0]
    )
  }

  // Insert technologies
  for (const tech of (opts.technologies ?? [])) {
    db.run(
      'INSERT INTO bullet_technologies (bullet_id, technology) VALUES (?, ?)',
      [id, tech.toLowerCase().trim()]
    )
  }
  return id
}
```

- [ ] **Add `seedResumeEntry` helper.**

```typescript
/** Seed a resume entry (links perspective to resume) and return its ID */
export function seedResumeEntry(db: Database, resumeId: string, perspectiveId: string, opts: {
  section?: string
  position?: number
  content?: string | null
} = {}): string {
  const id = testUuid()
  db.run(
    `INSERT INTO resume_entries (id, resume_id, perspective_id, content, section, position)
     VALUES (?, ?, ?, ?, ?, ?)`,
    [
      id,
      resumeId,
      perspectiveId,
      opts.content ?? null,
      opts.section ?? 'work_history',
      opts.position ?? 0,
    ]
  )
  return id
}
```

- [ ] **Add `seedUserNote` helper.**

```typescript
/** Seed a user note and return its ID */
export function seedUserNote(db: Database, opts: {
  title?: string
  content?: string
  references?: Array<{ entityType: string; entityId: string }>
} = {}): string {
  const id = testUuid()
  db.run(
    `INSERT INTO user_notes (id, title, content) VALUES (?, ?, ?)`,
    [id, opts.title ?? null, opts.content ?? 'Test note content']
  )
  for (const ref of (opts.references ?? [])) {
    db.run(
      'INSERT INTO note_references (note_id, entity_type, entity_id) VALUES (?, ?, ?)',
      [id, ref.entityType, ref.entityId]
    )
  }
  return id
}
```

- [ ] **Keep `seedPerspective` and `seedResume` unchanged** (their tables did not change structurally, only added a `notes` column).

- [ ] **Verify** the updated helpers compile:

```bash
cd /Users/adam/notes/job-hunting
bun build packages/core/src/db/__tests__/helpers.ts --no-bundle 2>&1 | head -20
```

---

### Task 10.3: Update existing tests to pass with new schema

**Files:**
- Modify: `packages/core/src/db/repositories/__tests__/source-repository.test.ts`
- Modify: `packages/core/src/db/repositories/__tests__/bullet-repository.test.ts`
- Modify: `packages/core/src/db/repositories/__tests__/resume-repository.test.ts`
- Modify: `packages/core/src/db/repositories/__tests__/perspective-repository.test.ts`
- Modify: `packages/core/src/db/repositories/__tests__/supporting-repositories.test.ts`
- Modify: `packages/core/src/services/__tests__/source-service.test.ts`
- Modify: `packages/core/src/services/__tests__/bullet-service.test.ts`
- Modify: `packages/core/src/services/__tests__/perspective-service.test.ts`
- Modify: `packages/core/src/services/__tests__/review-service.test.ts`
- Modify: `packages/core/src/services/__tests__/audit-service.test.ts`
- Modify: `packages/core/src/services/__tests__/resume-service.test.ts`
- Modify: `packages/core/src/services/__tests__/derivation-service.test.ts`
- Modify: `packages/core/src/routes/__tests__/sources.test.ts`
- Modify: `packages/core/src/routes/__tests__/bullets.test.ts`
- Modify: `packages/core/src/routes/__tests__/perspectives.test.ts`
- Modify: `packages/core/src/routes/__tests__/resumes.test.ts`
- Modify: `packages/core/src/routes/__tests__/review.test.ts`
- Modify: `packages/core/src/routes/__tests__/supporting.test.ts`
- Modify: `packages/core/src/routes/__tests__/contracts.test.ts`
- Modify: `packages/core/src/__tests__/e2e/e2e.test.ts`

**Goal:** Make every existing test pass by updating references to removed tables/columns and adapting to new seed function signatures. These tests will still test the OLD repository/service code, which now operates on the NEW schema. Some tests will need to be temporarily skipped (`test.skip`) if the underlying code hasn't been updated yet (repositories still reference `employers`, `source_id`, etc.). The goal is that `bun test` has zero failures -- tests either pass or are explicitly skipped with a `// TODO: Phase 11` comment.

- [ ] **Pattern: Replace all `seedEmployer` calls with `seedOrganization`.**

Before:
```typescript
const empId = seedEmployer(db, 'Anthropic')
```

After:
```typescript
const orgId = seedOrganization(db, { name: 'Anthropic' })
```

- [ ] **Pattern: Replace all `seedProject` calls.** If the test needs a project, create it as a source with `sourceType: 'project'`.

Before:
```typescript
const projId = seedProject(db, 'Atlas', empId)
```

After:
```typescript
const projSourceId = seedSource(db, {
  title: 'Atlas',
  sourceType: 'project',
  organizationId: orgId,
})
```

- [ ] **Pattern: Update all `seedSource` calls.** Remove `employerId` and `projectId` params. Add `sourceType` if needed.

Before:
```typescript
const srcId = seedSource(db, { employerId: empId, projectId: projId })
```

After:
```typescript
const srcId = seedSource(db, { sourceType: 'role', organizationId: orgId })
```

- [ ] **Pattern: Update all `seedBullet` calls.** Change from positional `sourceId` to array format.

Before:
```typescript
const bulletId = seedBullet(db, sourceId, { content: 'Test' })
```

After:
```typescript
const bulletId = seedBullet(db, [{ id: sourceId }], { content: 'Test' })
```

- [ ] **Pattern: Replace `resume_perspectives` references with `resume_entries`.** In resume-related tests, use `seedResumeEntry` instead of direct `resume_perspectives` inserts.

Before:
```typescript
db.run('INSERT INTO resume_perspectives (resume_id, perspective_id, section, position) VALUES (?, ?, ?, ?)',
  [resumeId, perspId, 'work_history', 0])
```

After:
```typescript
seedResumeEntry(db, resumeId, perspId, { section: 'work_history', position: 0 })
```

- [ ] **Pattern: Update assertions that check `employer_id`, `project_id`, `source_id`.** Replace with assertions on new fields (`source_type`, sources array, etc.) or remove them. For tests that check `source.employer_id` or `source.project_id`, either skip the test (if the code being tested hasn't been updated yet) or adjust assertions.

- [ ] **Pattern: Skip tests whose underlying code references removed columns/tables.** The following tests MUST be `test.skip`-ed with a `// TODO: Phase 11` comment because the repository/service code still references columns or tables that no longer exist:

  - **`PerspectiveRepository.getWithChain()`** — JOINs through `bullets.source_id` which no longer exists
  - **`review-service.test.ts`** — JOINs `bullets.source_id` to `sources.id` in pending queries
  - **`resume-service.test.ts`** — calls `addPerspective`/`removePerspective`/`reorderPerspectives` which reference `resume_perspectives` table
  - **Any test in `supporting-repositories.test.ts`** that tests `EmployerRepository` or `ProjectRepository` — these tables no longer exist
  - **`derivation-service.test.ts`** tests that insert `source_id` into bullets
  - **`audit-service.test.ts`** tests that read `bullet.source_id`

  Each skipped test gets `// TODO: Phase 11 — update for bullet_sources junction / resume_entries` comment.

- [ ] **Handle the supporting-repositories.test.ts file.** This tests `EmployerRepository` and `ProjectRepository` which will be DELETED in Phase 11. For now:
  - Skip the `EmployerRepository` and `ProjectRepository` test blocks with `describe.skip('EmployerRepository — REMOVED, see OrganizationRepository', ...)`
  - Leave `SkillRepository` and `PromptLogRepository` tests intact

- [ ] **For each test file, run the tests in isolation to verify:**

```bash
cd /Users/adam/notes/job-hunting
bun test packages/core/src/db/repositories/__tests__/source-repository.test.ts
bun test packages/core/src/db/repositories/__tests__/bullet-repository.test.ts
# ... etc
```

- [ ] **Run the full test suite and verify no failures:**

```bash
cd /Users/adam/notes/job-hunting
bun test
```

Expected: All tests either pass or are `test.skip`/`describe.skip` with `// TODO: Phase 11` comments. Zero red failures.

---

### Task 10.4: Update TypeScript types

**Files:**
- Modify: `packages/core/src/types/index.ts`

**Goal:** Update all type interfaces to reflect the new schema. Remove deprecated fields, add new entity types and input types.

- [ ] **Remove `Employer` interface entirely.** Add `Organization` interface.

```typescript
/** An organization (replaces Employer). */
export interface Organization {
  id: string
  name: string
  org_type: string
  industry: string | null
  size: string | null
  worked: number
  employment_type: string | null
  location: string | null
  headquarters: string | null
  website: string | null
  linkedin_url: string | null
  glassdoor_url: string | null
  glassdoor_rating: number | null
  reputation_notes: string | null
  notes: string | null
  created_at: string
  updated_at: string
}
```

- [ ] **Remove `Project` interface entirely.** Projects are now sources with `source_type = 'project'`.

- [ ] **Update `Source` interface.** Remove `employer_id`, `project_id`. Add `source_type`, `notes`.

```typescript
/** Valid source type values. */
export type SourceType = 'role' | 'project' | 'education' | 'clearance' | 'general'

/** A source experience entry — the root of the derivation chain. */
export interface Source {
  id: string
  title: string
  description: string
  source_type: SourceType
  start_date: string | null
  end_date: string | null
  status: SourceStatus
  updated_by: UpdatedBy
  last_derived_at: string | null
  notes: string | null
  created_at: string
  updated_at: string
}
```

- [ ] **Add source extension interfaces.**

```typescript
/** Extension data for role-type sources. */
export interface SourceRole {
  source_id: string
  organization_id: string | null
  start_date: string | null
  end_date: string | null
  is_current: number
  work_arrangement: string | null
  base_salary: number | null
  total_comp_notes: string | null
}

/** Extension data for project-type sources. */
export interface SourceProject {
  source_id: string
  organization_id: string | null
  is_personal: number
  url: string | null
  start_date: string | null
  end_date: string | null
}

/** Extension data for education-type sources. */
export interface SourceEducation {
  source_id: string
  education_type: string
  institution: string | null
  field: string | null
  start_date: string | null
  end_date: string | null
  is_in_progress: number
  credential_id: string | null
  expiration_date: string | null
  issuing_body: string | null
  url: string | null
}

/** Extension data for clearance-type sources. */
export interface SourceClearance {
  source_id: string
  level: string
  polygraph: string | null
  status: string | null
  sponsoring_agency: string | null
  investigation_date: string | null
  adjudication_date: string | null
  reinvestigation_date: string | null
  read_on: string | null
}

/** A source with its extension data resolved. */
export type SourceWithExtension = Source & {
  extension: SourceRole | SourceProject | SourceEducation | SourceClearance | null
}
```

- [ ] **Update `Bullet` interface.** Remove `source_id`. Add `domain`, `notes`.

```typescript
/** A bullet point derived from a source. */
export interface Bullet {
  id: string
  content: string
  source_content_snapshot: string
  technologies: string[]
  metrics: string | null
  domain: string | null
  status: BulletStatus
  rejection_reason: string | null
  prompt_log_id: string | null
  approved_at: string | null
  approved_by: string | null
  notes: string | null
  created_at: string
}
```

- [ ] **Add `notes` to `Perspective`, `Resume`, `Skill` interfaces.**

```typescript
// In Perspective interface, add:
notes: string | null

// In Resume interface, add:
notes: string | null

// In Skill interface, add:
notes: string | null
```

- [ ] **Replace `ResumePerspective` with `ResumeEntry`.**

```typescript
/** A resume entry — links a perspective to a resume section with optional content override. */
export interface ResumeEntry {
  id: string
  resume_id: string
  perspective_id: string
  content: string | null
  perspective_content_snapshot: string | null
  section: ResumeSection
  position: number
  notes: string | null
  created_at: string
  updated_at: string
}
```

- [ ] **Replace `ResumeWithPerspectives` with `ResumeWithEntries`.**

```typescript
/** A resume with entries grouped by section. */
export interface ResumeWithEntries extends Resume {
  sections: Record<string, ResumeEntry[]>
}
```

Keep `ResumeWithPerspectives` as a deprecated alias during Phase 10 to avoid breaking all imports simultaneously:

```typescript
/** @deprecated Use ResumeWithEntries instead. Will be removed in Phase 11. */
export type ResumeWithPerspectives = ResumeWithEntries
```

- [ ] **Add `UserNote` and `NoteReference` interfaces.**

```typescript
/** A user-created note that can be linked to multiple entities. */
export interface UserNote {
  id: string
  title: string | null
  content: string
  created_at: string
  updated_at: string
}

/** A link between a user note and an entity. */
export interface NoteReference {
  note_id: string
  entity_type: string
  entity_id: string
}

/** A user note with its linked entity references. */
export interface UserNoteWithReferences extends UserNote {
  references: NoteReference[]
}
```

- [ ] **Update `CreateSource` and `UpdateSource` input types.** Remove `employer_id`, `project_id`. Add `source_type` and extension fields.

```typescript
/** Input for creating a new Source. */
export interface CreateSource {
  title: string
  description: string
  source_type?: SourceType
  start_date?: string
  end_date?: string
  // Extension fields (included based on source_type)
  organization_id?: string
  is_current?: number
  work_arrangement?: string
  base_salary?: number
  total_comp_notes?: string
  is_personal?: number
  url?: string
  education_type?: string
  institution?: string
  field?: string
  is_in_progress?: number
  credential_id?: string
  expiration_date?: string
  issuing_body?: string
  level?: string
  polygraph?: string
  clearance_status?: string
  sponsoring_agency?: string
}

/** Input for partially updating a Source. */
export interface UpdateSource {
  title?: string
  description?: string
  source_type?: SourceType
  start_date?: string | null
  end_date?: string | null
  notes?: string | null
  // Extension fields
  organization_id?: string | null
  is_current?: number
  work_arrangement?: string | null
  base_salary?: number | null
  total_comp_notes?: string | null
  is_personal?: number
  url?: string | null
  education_type?: string
  institution?: string | null
  field?: string | null
  is_in_progress?: number
  credential_id?: string | null
  expiration_date?: string | null
  issuing_body?: string | null
  level?: string
  polygraph?: string | null
  clearance_status?: string | null
  sponsoring_agency?: string | null
}
```

- [ ] **Update `AddResumePerspective` to `AddResumeEntry`.**

```typescript
/** Input for adding an entry to a resume. */
export interface AddResumeEntry {
  perspective_id: string
  section: string
  position: number
  content?: string | null
  notes?: string | null
}

/** @deprecated Use AddResumeEntry instead. Will be removed in Phase 11. */
export type AddResumePerspective = AddResumeEntry
```

- [ ] **Update `BulletWithRelations` to use sources array instead of single source.**

```typescript
/** A bullet source reference (from junction table). */
export interface BulletSourceRef {
  id: string
  title: string
  is_primary: number
}

/** A bullet with its parent sources and perspective count. */
export interface BulletWithRelations extends Bullet {
  sources: BulletSourceRef[]
  perspective_count: number
}
```

- [ ] **Update `BulletReviewItem`.** Remove `source_id`, keep `source_title` (still resolved via junction).

- [ ] **Verify** the types compile:

```bash
cd /Users/adam/notes/job-hunting
bunx tsc --noEmit --project packages/core/tsconfig.json 2>&1 | head -30
```

Expected: Type errors from repository/service code that still references old fields. This is expected -- those will be fixed in Phase 11. The types file itself should have no internal errors.

---

## Testing Requirements

- **Unit tests:** After T10.1, `createTestDb()` should run both migrations without error. Write a simple smoke test:
  ```typescript
  test('002_schema_evolution applies without error', () => {
    const db = createTestDb()
    const tables = db.query("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name").all()
    const names = tables.map((t: any) => t.name)
    expect(names).toContain('organizations')
    expect(names).toContain('bullet_sources')
    expect(names).toContain('resume_entries')
    expect(names).toContain('source_roles')
    expect(names).not.toContain('employers')
    expect(names).not.toContain('projects')
    expect(names).not.toContain('resume_perspectives')
    db.close()
  })
  ```

- **Integration tests:** All seed helpers must work with the new schema. Verify by running a full chain:
  ```typescript
  test('full chain seeds work', () => {
    const db = createTestDb()
    const orgId = seedOrganization(db)
    const srcId = seedSource(db, { sourceType: 'role', organizationId: orgId })
    const bulletId = seedBullet(db, [{ id: srcId }])
    const perspId = seedPerspective(db, bulletId)
    const resumeId = seedResume(db)
    const entryId = seedResumeEntry(db, resumeId, perspId)
    expect(entryId).toHaveLength(36)
    db.close()
  })
  ```

- **Existing test suite:** `bun test` must have zero failures. Tests that reference deprecated code should be `test.skip` with `// TODO: Phase 11`.

- **Fixture data:** No new fixture files needed. All test data is created via seed helpers.

## Documentation Requirements

- No documentation files to create (this is an implementation plan, not a feature spec).
- The migration file itself serves as documentation of the schema changes.
- Add JSDoc comments to all new type interfaces explaining their purpose and relationship to old types.

## Acceptance Criteria

- [ ] `002_schema_evolution.sql` exists at `packages/core/src/db/migrations/002_schema_evolution.sql`
- [ ] The migration applies cleanly on top of `001_initial.sql` (both `db.exec()` of the full file and via the migration runner)
- [ ] All 20 DDL steps execute without SQLite error
- [ ] `organizations` table contains migrated employer data (with `worked=1`, `org_type='company'`)
- [ ] `bullet_sources` table contains migrated `bullets.source_id` data (with `is_primary=1`)
- [ ] `resume_entries` table contains migrated `resume_perspectives` data (with `content=NULL`)
- [ ] `employers`, `projects`, `resume_perspectives` tables are dropped
- [ ] `sources` table no longer has `employer_id` or `project_id` columns
- [ ] `bullets` table no longer has `source_id` column
- [ ] `bullets` table has new `notes` and `domain` columns
- [ ] `perspectives`, `resumes`, `skills` tables have new `notes` column
- [ ] `user_notes`, `note_references`, `v1_import_map` tables exist
- [ ] All new tables have UUID CHECK constraints and STRICT mode
- [ ] Test helpers (`helpers.ts`) compile and all seed functions work
- [ ] TypeScript types (`types/index.ts`) compile with no internal errors
- [ ] `bun test` has zero red failures (all pass or explicitly skipped)

## Failure Criteria

- **Migration fails with SQLite error** -- Check: STRICT mode type constraints (e.g., `INTEGER NOT NULL DEFAULT 0` not `BOOLEAN`), DDL ordering (create before reference), index name conflicts
- **Table rebuild drops data** -- Check: INSERT SELECT captures ALL columns from the original table; verify row counts before/after
- **`bullet_technologies` FK breaks after bullets rebuild** -- The `bullet_technologies` table references `bullets(id)` with ON DELETE CASCADE. When `bullets` is rebuilt, the FK reference goes to the new table. SQLite handles this correctly because the new table has the same name after RENAME. Verify with: `PRAGMA foreign_key_check`
- **Tests fail after migration** -- Check: test helpers match new schema column names; seed functions don't reference dropped columns
- **Type compilation fails** -- Check: all `import type` references updated; no circular dependencies; deprecated aliases use `export type`
- **UUID generation in SQL** -- The `randomblob` UUID generation in step 16 produces v4-like UUIDs. If the 36-char CHECK fails, verify the hex output length (should be 36 chars with hyphens)
