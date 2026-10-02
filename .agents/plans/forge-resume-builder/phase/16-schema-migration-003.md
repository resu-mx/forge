# Phase 16: Schema Migration (003_renderer_and_entities.sql)

**Goal:** Write the shared schema migration that supports the resume renderer (header, markdown/LaTeX overrides), editable domains & archetypes (replacing hardcoded constants), and organization status tracking. This is the foundation migration for Phases 17-20.

**Architecture:** A single migration file (`003_renderer_and_entities.sql`) applies on top of `002_schema_evolution.sql`. It adds columns to `resumes` and `organizations`, creates three new entity tables (`domains`, `archetypes`, `archetype_domains`), seeds them from the current hardcoded constants, and registers itself in `_migrations`. Test helpers are extended but not changed. TypeScript types and SDK types are updated to match the new schema.

**Tech Stack:** SQLite (via Bun's bundled 3.45+), TypeScript, `bun:test`

**Depends on:** Phase 15 (stable baseline with all existing tests passing)
**Blocks:** Phase 17 (editable domains & archetypes), Phase 18 (organization updates), Phase 19 (resume IR + compilers), Phase 20 (resume renderer views)
**Parallelizable:** T16.1 must complete first. T16.2, T16.3, T16.4 can run in parallel after T16.1.

**Reference:** `refs/specs/2026-03-29-resume-renderer-and-entity-updates.md` sections 1.6 (Override Storage), 2.1 (Domains/Archetypes Schema), 3.1 (Organization Status)

---

## Context

The codebase currently has:
- `001_initial.sql` and `002_schema_evolution.sql` defining the complete schema through Phase 15
- `resumes` table with columns: `id`, `name`, `target_role`, `target_employer`, `archetype`, `status`, `notes`, `created_at`, `updated_at`
- `organizations` table with no `status` column
- Hardcoded constants in `packages/core/src/constants/archetypes.ts`: `ARCHETYPES` (6 values), `DOMAINS` (6 values), `ARCHETYPE_EXPECTED_DOMAINS` (mapping), `THIN_COVERAGE_THRESHOLD`, `RESUME_SECTIONS` (6 values)
- `ResumeService.analyzeGaps` reads from `ARCHETYPE_EXPECTED_DOMAINS` constant
- `DerivationService` uses archetype/domain strings without DB validation
- Test helpers: `seedOrganization`, `seedSource`, `seedBullet`, `seedPerspective`, `seedResume`, `seedResumeEntry`, `seedUserNote`
- SDK types mirror core types without `header`, override fields, or `status` on Organization

After this phase:
- `resumes` has `header`, `markdown_override`, `markdown_override_updated_at`, `latex_override`, `latex_override_updated_at` columns
- `domains`, `archetypes`, `archetype_domains` tables exist with seed data matching current constants
- `organizations` has a `status` column with CHECK constraint
- Test helpers include `seedDomain` and `seedArchetype`
- Core and SDK types updated to include all new fields
- All existing tests still pass without modification

## Goals

- Write `003_renderer_and_entities.sql` that applies cleanly on top of `002_schema_evolution.sql`
- Add resume renderer columns (header, overrides) to `resumes`
- Create `domains`, `archetypes`, `archetype_domains` tables with proper constraints
- Seed 6 domains and 6 archetypes with correct domain associations
- Add `status` column to `organizations`
- Add test helpers for the new entity tables
- Update TypeScript types in both `@forge/core` and `@forge/sdk`

## Non-Goals

- Changing any repository implementations (Phase 17 for domains/archetypes, Phase 18 for renderer)
- Changing any service implementations (Phase 17, 18)
- Changing any route handlers (Phase 17, 18, 19)
- Building UI views (Phase 17, 19, 20)
- Writing the IR compiler or format compilers (Phase 18)
- Removing the hardcoded constants (Phase 17 does that)

## Fallback Strategies

- If `ALTER TABLE ADD COLUMN` fails for any resume column, use the table-rebuild pattern (CREATE new, INSERT SELECT, DROP old, RENAME) with `PRAGMA foreign_keys = OFF/ON`
- If UUID generation in SQL is problematic, use fixed UUIDs in the seed data (deterministic, makes testing easier)
- If `STRICT` mode conflicts with the `DEFAULT (strftime(...))` expression on `created_at`, use an explicit INSERT with values instead of relying on defaults

---

## Tasks

### Task 16.1: Write 003_renderer_and_entities.sql migration

**Files:**
- Create: `packages/core/src/db/migrations/003_renderer_and_entities.sql`

**Goal:** Create the DDL migration that adds renderer support columns, entity tables with seed data, and organization status tracking.

- [ ] Create the file with the header comment:

```sql
-- Forge Resume Builder — Renderer & Entity Tables
-- Migration: 003_renderer_and_entities
-- Date: 2026-03-29
--
-- Adds resume renderer columns (header, markdown/LaTeX overrides),
-- creates editable domain and archetype entity tables with seed data,
-- and adds status tracking to organizations.
-- Builds on 002_schema_evolution.
```

- [ ] **Step 1: Add renderer columns to `resumes`.** These support the resume header JSON blob and markdown/LaTeX override storage with staleness tracking.

```sql
-- Step 1: Add renderer columns to resumes
ALTER TABLE resumes ADD COLUMN header TEXT;
ALTER TABLE resumes ADD COLUMN markdown_override TEXT;
ALTER TABLE resumes ADD COLUMN markdown_override_updated_at TEXT;
ALTER TABLE resumes ADD COLUMN latex_override TEXT;
ALTER TABLE resumes ADD COLUMN latex_override_updated_at TEXT;
```

- [ ] **Step 2: Create `domains` table.** STRICT mode, UUID primary key, unique name constraint.

```sql
-- Step 2: Create domains table
CREATE TABLE domains (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  name TEXT NOT NULL UNIQUE,
  description TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;
```

- [ ] **Step 3: Create `archetypes` table.** Same constraints as domains.

```sql
-- Step 3: Create archetypes table
CREATE TABLE archetypes (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  name TEXT NOT NULL UNIQUE,
  description TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;
```

- [ ] **Step 4: Create `archetype_domains` junction table.** Composite primary key, cascading deletes, `created_at` for audit trail.

```sql
-- Step 4: Create archetype_domains junction table
CREATE TABLE archetype_domains (
  archetype_id TEXT NOT NULL REFERENCES archetypes(id) ON DELETE CASCADE,
  domain_id TEXT NOT NULL REFERENCES domains(id) ON DELETE CASCADE,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  PRIMARY KEY (archetype_id, domain_id)
) STRICT;
```

- [ ] **Step 5: Seed 6 domains.** Using deterministic UUIDs for reproducibility. Names and descriptions match the current `DOMAINS` constant.

```sql
-- Step 5: Seed domains
INSERT INTO domains (id, name, description) VALUES
  ('d0000001-0000-4000-8000-000000000001', 'systems_engineering', 'Architecture, distributed systems, and infrastructure design'),
  ('d0000001-0000-4000-8000-000000000002', 'software_engineering', 'Application development, code quality, and software lifecycle'),
  ('d0000001-0000-4000-8000-000000000003', 'security', 'Information security, offensive/defensive operations, and compliance'),
  ('d0000001-0000-4000-8000-000000000004', 'devops', 'CI/CD, automation, observability, and platform engineering'),
  ('d0000001-0000-4000-8000-000000000005', 'ai_ml', 'Machine learning, AI systems, and data engineering'),
  ('d0000001-0000-4000-8000-000000000006', 'leadership', 'Team leadership, mentoring, cross-functional coordination');
```

- [ ] **Step 6: Seed 6 archetypes.** Deterministic UUIDs. Names match the current `ARCHETYPES` constant.

```sql
-- Step 6: Seed archetypes
INSERT INTO archetypes (id, name, description) VALUES
  ('a0000001-0000-4000-8000-000000000001', 'agentic-ai', 'AI/ML engineer building autonomous systems and intelligent agents'),
  ('a0000001-0000-4000-8000-000000000002', 'infrastructure', 'Infrastructure and platform engineer focused on scalable systems'),
  ('a0000001-0000-4000-8000-000000000003', 'security-engineer', 'Security engineer spanning offensive, defensive, and compliance domains'),
  ('a0000001-0000-4000-8000-000000000004', 'solutions-architect', 'Solutions architect bridging business needs with technical design'),
  ('a0000001-0000-4000-8000-000000000005', 'public-sector', 'Public sector engineer with clearance and government systems experience'),
  ('a0000001-0000-4000-8000-000000000006', 'hft', 'High-frequency trading systems engineer focused on ultra-low latency');
```

- [ ] **Step 7: Seed `archetype_domains` junction.** Maps each archetype to its expected domains, matching the current `ARCHETYPE_EXPECTED_DOMAINS` constant exactly.

```sql
-- Step 7: Seed archetype_domains from ARCHETYPE_EXPECTED_DOMAINS
-- agentic-ai: ai_ml, software_engineering, leadership
INSERT INTO archetype_domains (archetype_id, domain_id)
  SELECT a.id, d.id FROM archetypes a, domains d
  WHERE a.name = 'agentic-ai' AND d.name IN ('ai_ml', 'software_engineering', 'leadership');

-- infrastructure: systems_engineering, devops, software_engineering
INSERT INTO archetype_domains (archetype_id, domain_id)
  SELECT a.id, d.id FROM archetypes a, domains d
  WHERE a.name = 'infrastructure' AND d.name IN ('systems_engineering', 'devops', 'software_engineering');

-- security-engineer: security, systems_engineering, devops
INSERT INTO archetype_domains (archetype_id, domain_id)
  SELECT a.id, d.id FROM archetypes a, domains d
  WHERE a.name = 'security-engineer' AND d.name IN ('security', 'systems_engineering', 'devops');

-- solutions-architect: systems_engineering, software_engineering, leadership
INSERT INTO archetype_domains (archetype_id, domain_id)
  SELECT a.id, d.id FROM archetypes a, domains d
  WHERE a.name = 'solutions-architect' AND d.name IN ('systems_engineering', 'software_engineering', 'leadership');

-- public-sector: security, systems_engineering, leadership
INSERT INTO archetype_domains (archetype_id, domain_id)
  SELECT a.id, d.id FROM archetypes a, domains d
  WHERE a.name = 'public-sector' AND d.name IN ('security', 'systems_engineering', 'leadership');

-- hft: systems_engineering, software_engineering
INSERT INTO archetype_domains (archetype_id, domain_id)
  SELECT a.id, d.id FROM archetypes a, domains d
  WHERE a.name = 'hft' AND d.name IN ('systems_engineering', 'software_engineering');
```

- [ ] **Step 8: Add `status` column to `organizations`.** Nullable with CHECK constraint for the status lifecycle values.

```sql
-- Step 8: Add status to organizations
ALTER TABLE organizations ADD COLUMN status TEXT CHECK (status IN (
  'interested', 'review', 'targeting', 'excluded'
));
```

Note: `NULL` is implicitly allowed because `NOT NULL` is not specified. The CHECK constraint only validates non-NULL values.

- [ ] **Step 9: Rename `work_history` section values to `experience`.** Aligns existing data with the IR compiler's expected section name. Both values remain valid in the `ResumeSection` type for backward compatibility, but new data should use `experience`.

```sql
-- Step 9: Rename work_history -> experience in resume_entries
UPDATE resume_entries SET section = 'experience' WHERE section = 'work_history';
```

- [ ] **Step 10: Register migration.**

```sql
-- Step 10: Register migration
INSERT INTO _migrations (name) VALUES ('003_renderer_and_entities');
```

**Acceptance Criteria:**
- [ ] Migration applies without error on top of `001_initial.sql` + `002_schema_evolution.sql`
- [ ] `resumes` table has all 5 new columns (`header`, `markdown_override`, `markdown_override_updated_at`, `latex_override`, `latex_override_updated_at`)
- [ ] `domains` table exists with 6 seeded rows
- [ ] `archetypes` table exists with 6 seeded rows
- [ ] `archetype_domains` table has correct associations (16 total rows: 3+3+3+3+3+2)
- [ ] `organizations` table has `status` column accepting NULL and the 4 enum values
- [ ] Migration is registered in `_migrations`

**Testing:**
- Unit: `createTestDb()` succeeds without error (validates migration applies cleanly)
- Unit: Query `SELECT COUNT(*) FROM domains` returns 6
- Unit: Query `SELECT COUNT(*) FROM archetypes` returns 6
- Unit: Query `SELECT COUNT(*) FROM archetype_domains` returns 17 (3+3+3+3+3+2)
- Unit: Verify archetype_domains associations match `ARCHETYPE_EXPECTED_DOMAINS` constant exactly
- Unit: Insert and retrieve a resume with `header` JSON, verify round-trip
- Unit: Insert and retrieve a resume with `markdown_override` and `latex_override`, verify override timestamps
- Unit: Update organization with each valid `status` value, verify accepted
- Unit: Attempt invalid organization status, verify CHECK constraint rejects it

---

### Task 16.2: Update test helpers

**Files:**
- Modify: `packages/core/src/db/__tests__/helpers.ts`

**Goal:** Add seed helpers for `domains` and `archetypes` tables, matching the existing helper patterns.

- [ ] Add `seedDomain` helper:

```typescript
/** Seed a test domain and return its ID */
export function seedDomain(db: Database, opts: {
  name?: string
  description?: string
} = {}): string {
  const id = testUuid()
  db.run(
    `INSERT INTO domains (id, name, description)
     VALUES (?, ?, ?)`,
    [
      id,
      opts.name ?? 'test_domain',
      opts.description ?? null,
    ]
  )
  return id
}
```

- [ ] Add `seedArchetype` helper:

```typescript
/** Seed a test archetype and return its ID */
export function seedArchetype(db: Database, opts: {
  name?: string
  description?: string
} = {}): string {
  const id = testUuid()
  db.run(
    `INSERT INTO archetypes (id, name, description)
     VALUES (?, ?, ?)`,
    [
      id,
      opts.name ?? 'test-archetype',
      opts.description ?? null,
    ]
  )
  return id
}
```

- [ ] Add `seedArchetypeDomain` helper for junction table:

```typescript
/** Link an archetype to a domain in archetype_domains */
export function seedArchetypeDomain(db: Database, archetypeId: string, domainId: string): void {
  db.run(
    `INSERT INTO archetype_domains (archetype_id, domain_id)
     VALUES (?, ?)`,
    [archetypeId, domainId]
  )
}
```

- [ ] **Update `seedResumeEntry` default section** from `'work_history'` to `'experience'`:

```typescript
// In seedResumeEntry, change:
opts.section ?? 'work_history'
// To:
opts.section ?? 'experience'
```

This aligns the test helper with the migration that renames `work_history` to `experience` in Step 9.

- [ ] Verify existing helpers still work by running all existing tests:

```bash
bun test
```

**Acceptance Criteria:**
- [ ] `seedDomain` creates a row in `domains` and returns a valid UUID
- [ ] `seedArchetype` creates a row in `archetypes` and returns a valid UUID
- [ ] `seedArchetypeDomain` creates a junction row
- [ ] All existing tests pass without modification

**Testing:**
- Unit: `seedDomain(db)` inserts row, can be queried back with correct name
- Unit: `seedArchetype(db)` inserts row, can be queried back with correct name
- Unit: `seedArchetypeDomain(db, archetypeId, domainId)` creates junction row
- Unit: Duplicate `seedDomain(db, { name: 'x' })` followed by `seedDomain(db, { name: 'x' })` throws UNIQUE constraint error (validates schema)

---

### Task 16.3: Update types

**Files:**
- Modify: `packages/core/src/types/index.ts`
- Modify: `packages/sdk/src/types.ts`
- Modify: `packages/sdk/src/index.ts`

**Goal:** Add type definitions for the new entities and updated fields so downstream code in Phases 17-20 can import them immediately.

- [ ] Add `Domain` interface to `packages/core/src/types/index.ts`:

```typescript
/** An editable experience domain. */
export interface Domain {
  id: string
  name: string
  description: string | null
  created_at: string
}
```

- [ ] Add `Archetype` interface to `packages/core/src/types/index.ts`:

```typescript
/** An editable resume archetype. */
export interface Archetype {
  id: string
  name: string
  description: string | null
  created_at: string
}
```

- [ ] Add `ArchetypeDomain` interface to `packages/core/src/types/index.ts`:

```typescript
/** Junction linking an archetype to an expected domain. */
export interface ArchetypeDomain {
  archetype_id: string
  domain_id: string
  created_at: string
}
```

- [ ] Add `OrganizationStatus` type to `packages/core/src/types/index.ts`:

```typescript
/** Valid status values for organization tracking. */
export type OrganizationStatus = 'interested' | 'review' | 'targeting' | 'excluded'
```

- [ ] Update `Organization` interface in `packages/core/src/types/index.ts` to add `status`:

```typescript
export interface Organization {
  // ... existing fields ...
  status: OrganizationStatus | null  // null = no tracking (default)
  // ... existing fields ...
}
```

- [ ] Update `Resume` interface in `packages/core/src/types/index.ts` to add renderer fields:

```typescript
export interface Resume {
  // ... existing fields ...
  header: string | null                          // JSON blob for ResumeHeader
  markdown_override: string | null
  markdown_override_updated_at: string | null
  latex_override: string | null
  latex_override_updated_at: string | null
  // ... existing fields ...
}
```

- [ ] Update `UpdateResume` interface in `packages/core/src/types/index.ts`:

```typescript
export interface UpdateResume {
  // ... existing fields ...
  header?: string | null
  markdown_override?: string | null
  latex_override?: string | null
}
```

- [ ] **Update `ResumeRepository.update()` method** in `packages/core/src/db/repositories/resume-repository.ts` to handle the new fields in the dynamic SET builder pattern. Add these conditions to the existing SET-building block:

```typescript
if (input.header !== undefined) { sets.push('header = ?'); params.push(input.header) }
if (input.markdown_override !== undefined) {
  sets.push('markdown_override = ?'); params.push(input.markdown_override)
  sets.push('markdown_override_updated_at = ?'); params.push(input.markdown_override !== null ? new Date().toISOString() : null)
}
if (input.latex_override !== undefined) {
  sets.push('latex_override = ?'); params.push(input.latex_override)
  sets.push('latex_override_updated_at = ?'); params.push(input.latex_override !== null ? new Date().toISOString() : null)
}
```

Note: Phase 19 T19.10 adds dedicated `updateHeader()`, `updateMarkdownOverride()`, and `updateLatexOverride()` methods as specialized alternatives. The generic `update()` should still handle these fields for completeness.

- [ ] Update `ResumeSection` type in `packages/core/src/types/index.ts` to include new section types:

```typescript
export type ResumeSection =
  | 'summary'
  | 'experience'        // replaces work_history
  | 'work_history'      // kept for backward compatibility
  | 'projects'
  | 'education'
  | 'skills'
  | 'certifications'
  | 'clearance'
  | 'presentations'
  | 'awards'
  | 'custom'
```

- [ ] Mirror all new types in `packages/sdk/src/types.ts`:

Add `Domain`, `Archetype`, `ArchetypeDomain` interfaces (same structure as core, no imports).

Update `Organization` to add `status: 'interested' | 'review' | 'targeting' | 'excluded' | null`.

Update `Resume` to add `header`, `markdown_override`, `markdown_override_updated_at`, `latex_override`, `latex_override_updated_at` (all `string | null`).

Update `UpdateResume` to add `header?: string | null`, `markdown_override?: string | null`, `latex_override?: string | null`.

- [ ] Add exports to `packages/sdk/src/index.ts`:

```typescript
// Domain/Archetype entity types
export type {
  Domain,
  Archetype,
  ArchetypeDomain,
} from './types'
```

**Acceptance Criteria:**
- [ ] `Domain`, `Archetype`, `ArchetypeDomain` interfaces exist in both core and SDK types
- [ ] `Organization` has `status` field in both core and SDK types
- [ ] `Resume` has `header`, `markdown_override`, `markdown_override_updated_at`, `latex_override`, `latex_override_updated_at` in both core and SDK types
- [ ] `UpdateResume` has `header`, `markdown_override`, `latex_override` optional fields
- [ ] `ResumeSection` includes `experience`, `certifications`, `clearance`, `presentations`, `custom`
- [ ] SDK barrel exports `Domain`, `Archetype`, `ArchetypeDomain`
- [ ] TypeScript compiles without errors: `bunx tsc --noEmit`

**Testing:**
- Compile: `bunx tsc --noEmit` passes
- Unit: All existing tests pass (type changes are backward-compatible because new fields on Resume and Organization are nullable with defaults)

---

### Task 16.4: Verify

**Files:**
- No files created or modified

**Goal:** Run the full test suite and verify the migration, helpers, and type changes are all coherent.

- [ ] Run `bun test` from the project root — all existing tests must pass
- [ ] Write a one-off verification script (not committed) or manually verify:

```typescript
// Verification queries (run in bun repl or a scratch test):
const db = createTestDb()

// 1. New tables exist
const tables = db.query("SELECT name FROM sqlite_master WHERE type='table' AND name IN ('domains', 'archetypes', 'archetype_domains')").all()
console.assert(tables.length === 3, 'All 3 new tables exist')

// 2. Seed data populated
const domainCount = db.query('SELECT COUNT(*) as c FROM domains').get() as { c: number }
console.assert(domainCount.c === 6, '6 domains seeded')

const archetypeCount = db.query('SELECT COUNT(*) as c FROM archetypes').get() as { c: number }
console.assert(archetypeCount.c === 6, '6 archetypes seeded')

const junctionCount = db.query('SELECT COUNT(*) as c FROM archetype_domains').get() as { c: number }
console.assert(junctionCount.c === 17, '17 archetype_domain rows seeded')

// 3. Resume columns exist
const resumeCols = db.query("PRAGMA table_info(resumes)").all() as Array<{ name: string }>
const colNames = resumeCols.map(c => c.name)
console.assert(colNames.includes('header'), 'header column exists')
console.assert(colNames.includes('markdown_override'), 'markdown_override column exists')
console.assert(colNames.includes('latex_override'), 'latex_override column exists')

// 4. Organization status column exists
const orgCols = db.query("PRAGMA table_info(organizations)").all() as Array<{ name: string }>
console.assert(orgCols.map(c => c.name).includes('status'), 'status column exists')

// 5. Verify archetype_domains match ARCHETYPE_EXPECTED_DOMAINS
const agenticDomains = db.query(`
  SELECT d.name FROM domains d
  JOIN archetype_domains ad ON d.id = ad.domain_id
  JOIN archetypes a ON a.id = ad.archetype_id
  WHERE a.name = 'agentic-ai'
  ORDER BY d.name
`).all() as Array<{ name: string }>
const expected = ['ai_ml', 'leadership', 'software_engineering']
console.assert(
  JSON.stringify(agenticDomains.map(d => d.name)) === JSON.stringify(expected),
  'agentic-ai domains match constant'
)
```

- [ ] Verify that `ResumeRepository.create()` still works (new columns default to NULL)
- [ ] Verify that `ResumeRepository.get()` returns the new fields (they will be NULL but present)

**Acceptance Criteria:**
- [ ] `bun test` passes with 0 failures
- [ ] All 3 new tables exist with correct schemas
- [ ] Seed data counts are correct (6 domains, 6 archetypes, 17 junction rows)
- [ ] Resume columns default to NULL for existing and new rows
- [ ] Organization status defaults to NULL
- [ ] No existing behavior is broken

**Testing:**
- Full suite: `bun test` passes
- Verification script confirms all assertions

---

## Documentation Requirements

No ADRs needed for this phase (schema migration is a continuation of existing patterns). Update the following when documentation is written in Phase 17:
- `docs/src/data/models/entity-types.md` — add Domain, Archetype, ArchetypeDomain
- `docs/src/api/routes.md` — will be updated when routes are added in Phase 17
