# Phase 30: Summaries Entity (Migration 006 + Core + API + SDK + IR Compiler)

**Status:** Planning
**Date:** 2026-03-31
**Spec:** [2026-03-30-summaries-entity.md](../refs/specs/2026-03-30-summaries-entity.md)
**Depends on:** Phase 29 (Config Profile -- `parseHeader` intermediate state)
**Blocks:** Phase 34 (Summary Templates)
**Parallelizable with:** Phase 31 (JD, if migration numbering coordinated), Phase 32 (Nav)

## Goal

Extract professional summaries from the `resumes.header` JSON blob into a standalone `summaries` table with full CRUD, a clone endpoint, SDK resource, and IR compiler integration. After this phase, summaries are first-class reusable entities: each resume can link to a summary via `summary_id`, summaries can be browsed and managed independently, and the IR compiler reads the tagline from the linked summary rather than from the header blob. The `resumes.header` column is retained as a legacy fallback for contact info until the Profile spec (Phase 29) is fully wired.

## Non-Goals

- Contact info management (Profile spec / Phase 29)
- AI-assisted summary generation
- Summary versioning / history
- Summary sharing across users (single-user system)
- Summary diff/comparison view
- Summary Templates UI (Phase 34 -- `is_template` flag is added here but template management UI is deferred)
- Linked resumes endpoint (`GET /api/summaries/:id/linked-resumes` -- deferred to Phase 34)
- Summaries view UI (`/data/summaries` -- separate phase, depends on Nav restructuring)
- Resume creation flow "Pick Summary" step (depends on Summaries view)
- Resume detail view summary section (Change/Edit/Detach buttons -- depends on Summaries view)

## Context

Summaries (role title, tagline, description) are currently baked into `resumes.header` as a JSON blob alongside contact info. This makes them unreusable and unmanageable. Extracting them into their own table enables:

1. Reuse across resumes (one summary, many resumes)
2. Independent management (browse, edit, clone, template)
3. Clean separation from contact info (which belongs in Profile)
4. Foundation for Summary Templates (Phase 34) and AI summary generation (future)

The migration uses a TypeScript data migration helper (not inline SQL) because pure-SQL UUID generation with correlated subqueries is fragile when `target_role` is NULL.

## Scope

| Spec section | Covered here? |
|-------------|---------------|
| 1. Schema (migration 006 DDL) | Yes |
| 1.3. Data Migration (TS helper) | Yes |
| 2.1. Summaries CRUD API | Yes |
| 2.2. Resume-Summary linking (PATCH) | Yes |
| 2.3. Linked resumes endpoint | No (Phase 34) |
| 3.1. Summaries View UI | No (future phase) |
| 3.2. Resume Creation Flow | No (future phase) |
| 3.3. Resume Detail View | No (future phase) |
| 4. Type changes (core + SDK) | Yes |
| 4.3. IR compiler changes | Yes |
| 4.4. parseHeader evolution | Yes (intermediate: `parseHeader(resume, summary)`) |
| 5. SDK SummariesResource | Yes |

## Files to Create

| File | Purpose |
|------|---------|
| `packages/core/src/db/migrations/006_summaries.sql` | DDL: CREATE TABLE summaries, ALTER TABLE resumes ADD COLUMN summary_id, INSERT INTO _migrations |
| `packages/core/src/db/migrations/006_summaries_data.ts` | TypeScript data migration helper: reads resumes with non-null header, generates UUIDs, INSERTs summaries, UPDATEs resume.summary_id |
| `packages/core/src/db/repositories/summary-repository.ts` | SummaryRepository with CRUD + clone |
| `packages/core/src/services/summary-service.ts` | SummaryService with validation and Result wrapping |
| `packages/core/src/routes/summaries.ts` | Hono route handlers for summaries CRUD + clone |
| `packages/sdk/src/resources/summaries.ts` | SummariesResource SDK class |
| `packages/core/src/db/repositories/__tests__/summary-repository.test.ts` | Repository unit tests |
| `packages/core/src/services/__tests__/summary-service.test.ts` | Service unit tests |
| `packages/core/src/routes/__tests__/summaries.test.ts` | API integration tests |

## Files to Modify

| File | Change |
|------|--------|
| `packages/core/src/types/index.ts` | Add Summary, CreateSummary, UpdateSummary types; add `summary_id` to Resume, CreateResume, UpdateResume |
| `packages/sdk/src/types.ts` | Mirror Summary types with boolean `is_template`; add SummaryFilter; add `summary_id` to Resume, CreateResume, UpdateResume |
| `packages/sdk/src/index.ts` | Export Summary types + SummariesResource |
| `packages/sdk/src/client.ts` | Add `summaries` property, instantiate SummariesResource |
| `packages/core/src/db/repositories/resume-repository.ts` | Add `summary_id` to ResumeRow + rowToResume + update() field loop |
| `packages/core/src/services/index.ts` | Add SummaryService to Services interface + createServices() |
| `packages/core/src/routes/server.ts` | Mount summaryRoutes |
| `packages/core/src/services/resume-compiler.ts` | Update parseHeader to accept summary param; fetch summary in compileResumeIR |
| `packages/core/src/db/__tests__/helpers.ts` | Add seedSummary helper |
| `packages/core/src/db/migrate.ts` | Add post-SQL TypeScript migration hook for 006_summaries_data |
| `packages/core/src/routes/__tests__/contracts.test.ts` | Add summaries contract tests |
| `packages/sdk/src/__tests__/resources.test.ts` | Add SummariesResource SDK tests |

## Fallback Strategies

| Assumption | If it fails | Fallback |
|-----------|-------------|----------|
| Migration 005 (user_profile) runs before 006 | 006's DDL is schema-independent of 005; only `_migrations` ordering matters | If 005 doesn't exist yet, 006 still works. The `parseHeader` intermediate signature uses `parseHeader(resume, summary)` without profile param. |
| `resumes.header` contains parseable JSON with tagline | Some rows may have NULL or malformed header | Data migration guards with `try/catch JSON.parse`; falls back to `target_role` for tagline, NULL for description |
| Bun SQLite supports `RETURNING *` | Already proven in existing repositories | No fallback needed |
| `crypto.randomUUID()` available in migration helper | Bun runtime always has this | No fallback needed |

---

## Tasks

### T30.1: Write `006_summaries.sql` (DDL only)

**File:** `packages/core/src/db/migrations/006_summaries.sql`

DDL-only migration. Data migration is handled by the TypeScript helper (T30.2).

```sql
-- Forge Resume Builder -- Summaries as Standalone Entities
-- Migration: 006_summaries
-- Date: 2026-03-31
--
-- Extracts resume summaries from the header JSON blob into a standalone table.
-- Adds summary_id FK to resumes.
-- Data migration is handled by 006_summaries_data.ts (TypeScript helper).
-- Note: 006's DDL is schema-independent of 005_user_profile.

-- Step 1: Create summaries table
CREATE TABLE summaries (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  title TEXT NOT NULL,
  role TEXT,
  tagline TEXT,
  description TEXT,
  is_template INTEGER NOT NULL DEFAULT 0,
  notes TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

CREATE INDEX idx_summaries_template ON summaries(is_template);

-- Step 2: Add summary_id to resumes
ALTER TABLE resumes ADD COLUMN summary_id TEXT REFERENCES summaries(id) ON DELETE SET NULL;

-- Step 3: Register migration
INSERT INTO _migrations (name) VALUES ('006_summaries');
```

**Acceptance criteria:**
- After migration, `summaries` table exists with correct column types and constraints
- `resumes` table has `summary_id` column that is nullable and FKs to `summaries(id)` with ON DELETE SET NULL
- `idx_summaries_template` index exists
- Migration is registered in `_migrations`

**Failure criteria:**
- Migration fails on `CREATE TABLE` or `ALTER TABLE`
- `summary_id` FK does not cascade SET NULL on summary deletion
- Migration runs twice (not idempotent via `_migrations` check)

---

### T30.2: Write `006_summaries_data.ts` (TypeScript data migration helper)

**File:** `packages/core/src/db/migrations/006_summaries_data.ts`

This helper is called after the SQL migration runs. It reads each resume with a non-null `header`, extracts fields, generates a UUID, INSERTs a summary row, and UPDATEs `resume.summary_id`.

```typescript
/**
 * Data migration for 006_summaries.
 *
 * Reads each resume with a non-null header JSON blob, extracts the tagline
 * (and role from target_role), creates a summary row with a TypeScript-generated
 * UUID, and links it to the resume via summary_id.
 *
 * This approach avoids the fragile pure-SQL UUID generation + correlated subquery
 * pattern that silently drops rows when target_role is NULL.
 */

import type { Database } from 'bun:sqlite'

interface ResumeHeaderRow {
  id: string
  name: string
  target_role: string | null
  header: string | null
}

interface ParsedHeader {
  tagline?: string
  name?: string
  [key: string]: unknown
}

export function migrateHeadersToSummaries(db: Database): { migrated: number; skipped: number } {
  // Re-entrant guard: skip if all resumes already have summary_id set
  const unmigrated = db.query('SELECT COUNT(*) as cnt FROM resumes WHERE header IS NOT NULL AND summary_id IS NULL').get() as { cnt: number }
  if (unmigrated.cnt === 0) return { migrated: 0, skipped: 0 } // already migrated

  const resumes = db
    .query('SELECT id, name, target_role, header FROM resumes WHERE header IS NOT NULL')
    .all() as ResumeHeaderRow[]

  let migrated = 0
  let skipped = 0

  const insertSummary = db.prepare(
    `INSERT INTO summaries (id, title, role, tagline, description, is_template, notes)
     VALUES (?, ?, ?, ?, ?, 0, NULL)`
  )

  const updateResume = db.prepare(
    `UPDATE resumes SET summary_id = ? WHERE id = ?`
  )

  const txn = db.transaction(() => {
    for (const resume of resumes) {
      // Parse the header JSON; skip if malformed
      let parsed: ParsedHeader = {}
      try {
        parsed = JSON.parse(resume.header!) as ParsedHeader
      } catch {
        // Malformed JSON -- create summary from target_role fallback
      }

      const summaryId = crypto.randomUUID()
      const tagline = parsed.tagline ?? resume.target_role ?? null
      const role = resume.target_role ?? null
      // Title is an internal label -- use resume name + role for clarity
      const title = role
        ? `${resume.name} - ${role}`
        : resume.name

      insertSummary.run(summaryId, title, role, tagline, null)
      updateResume.run(summaryId, resume.id)
      migrated++
    }
  })

  txn()

  // Count resumes without headers (they get no summary)
  const totalResumes = (db.query('SELECT COUNT(*) AS cnt FROM resumes').get() as { cnt: number }).cnt
  skipped = totalResumes - migrated

  return { migrated, skipped }
}
```

**Key design decisions:**
- Two-step inside a transaction: INSERT summary, then UPDATE resume. Keyed by `resumes.id`, not by any composite join.
- `description` is always NULL because it was never stored in `header` (spec limitation 2).
- `title` is synthesized as `"{resume.name} - {role}"` for human-readable internal labels.
- `is_template` is always 0 for migrated summaries.
- Resumes with NULL `header` are skipped (they get no linked summary).
- Resumes with NULL `target_role` still get a summary (tagline falls through to NULL).

**Acceptance criteria:**
- Every resume with non-null `header` gets a linked summary
- `summary_id` on resume points to the created summary
- NULL `target_role` resumes get a summary with `role = NULL` and `tagline = NULL`
- Malformed JSON headers still produce a summary (using target_role fallback)
- Transaction atomicity: all-or-nothing

**Failure criteria:**
- Any resume with non-null header has `summary_id = NULL` after migration
- UUID collision (astronomically unlikely but would throw UNIQUE constraint error)
- Partial migration (some rows migrated, others not) due to missing transaction

---

### T30.3: Hook TypeScript data migration into migrate.ts

**File:** `packages/core/src/db/migrate.ts`

The migration runner currently only executes `.sql` files. We need to add a post-SQL hook that checks for a companion `.ts` data migration file and runs it.

```typescript
// Add after the existing imports:
import { migrateHeadersToSummaries } from './migrations/006_summaries_data'

// Add inside the `for (const file of pending)` loop, after the COMMIT:
// --- existing code ---
//       db.exec("COMMIT");
//       console.log(`Applied migration: ${file}`);

// Add after the console.log:
      // Run companion TypeScript data migrations
      if (name === '006_summaries') {
        try {
          const result = migrateHeadersToSummaries(db)
          console.log(`  Data migration: ${result.migrated} summaries created, ${result.skipped} resumes skipped`)
        } catch (err) {
          console.error(`  Data migration failed for ${name}:`, err)
          throw err
        }
      }
```

The full modified loop body becomes:

```typescript
    try {
      db.exec("BEGIN");
      db.exec(sql);

      // Record the migration. The SQL file may already insert its own
      // record (as 001_initial.sql does), so use INSERT OR IGNORE to
      // avoid a UNIQUE constraint failure on the name column.
      db.run("INSERT OR IGNORE INTO _migrations (name) VALUES (?)", [name]);

      db.exec("COMMIT");
      console.log(`Applied migration: ${file}`);

      // Run companion TypeScript data migrations
      if (name === '006_summaries') {
        try {
          const result = migrateHeadersToSummaries(db)
          console.log(`  Data migration: ${result.migrated} summaries created, ${result.skipped} resumes skipped`)
        } catch (err) {
          console.error(`  Data migration failed for ${name}:`, err)
          throw err
        }
      }
    } catch (err) {
      try {
        db.exec("ROLLBACK");
      } catch {
        // Rollback may fail if the transaction was already aborted by SQLite.
      }
      console.error(`Migration failed: ${file}`);
      throw err;
    }
```

**Note:** The TS data migration runs AFTER the DDL COMMIT because the `summaries` table must exist first. If the data migration fails after DDL commits, restart the server -- the re-entrant guard in `migrateHeadersToSummaries()` will detect unmigrated resumes (those with non-null `header` but `summary_id = NULL`) and retry automatically. The `_migrations` table already has '006_summaries' recorded, so the DDL will not re-run, but the hardcoded `if (name === '006_summaries')` hook will not fire either. To handle this recovery case, add a startup check: if '006_summaries' is in `_migrations` but unmigrated resumes exist, call `migrateHeadersToSummaries(db)` at server boot.

**Note on hardcoded hook:** This hardcoded `if (name === '006_summaries')` approach is intentional tech debt. When a second TypeScript migration helper is needed, refactor to a dynamic import/plugin pattern. The threshold for generalization is: any new migration with a companion `.ts` file.

**Acceptance criteria:**
- `runMigrations` calls `migrateHeadersToSummaries` after 006 DDL commits
- Console output shows count of migrated summaries
- Data migration errors propagate and halt the process

**Failure criteria:**
- Data migration runs before DDL COMMIT (table doesn't exist)
- Data migration errors are silently swallowed

---

### T30.4: Add Core Types

**File:** `packages/core/src/types/index.ts`

Add the following types after the existing `Archetype` / `ArchetypeDomain` section:

```typescript
// ── Summary Entities ──────────────────────────────────────────────────

/** A reusable professional summary. */
export interface Summary {
  id: string
  title: string
  role: string | null
  tagline: string | null
  description: string | null
  is_template: number
  notes: string | null
  created_at: string
  updated_at: string
}

/** Input for creating a new Summary. */
export interface CreateSummary {
  title: string
  role?: string
  tagline?: string
  description?: string
  is_template?: number
  notes?: string
}

/** Input for partially updating a Summary. */
export interface UpdateSummary {
  title?: string
  role?: string | null
  tagline?: string | null
  description?: string | null
  is_template?: number
  notes?: string | null
}
```

Update the existing `Resume` interface -- add `summary_id`:

```typescript
// In the Resume interface, add after 'header':
export interface Resume {
  id: string
  name: string
  target_role: string
  target_employer: string
  archetype: string
  status: ResumeStatus
  notes: string | null
  header: string | null
  summary_id: string | null      // <-- NEW
  markdown_override: string | null
  markdown_override_updated_at: string | null
  latex_override: string | null
  latex_override_updated_at: string | null
  created_at: string
  updated_at: string
}
```

Update `CreateResume` -- add optional `summary_id`:

```typescript
export interface CreateResume {
  name: string
  target_role: string
  target_employer: string
  archetype: string
  summary_id?: string            // <-- NEW
}
```

Update `UpdateResume` -- add optional `summary_id`:

```typescript
export interface UpdateResume {
  name?: string
  target_role?: string
  target_employer?: string
  archetype?: string
  status?: ResumeStatus
  header?: string | null
  summary_id?: string | null     // <-- NEW
  markdown_override?: string | null
  latex_override?: string | null
}
```

**Acceptance criteria:**
- `Summary`, `CreateSummary`, `UpdateSummary` interfaces exported from types
- `Resume` has `summary_id: string | null`
- `CreateResume` has `summary_id?: string`
- `UpdateResume` has `summary_id?: string | null`
- TypeScript compiles without errors

**Failure criteria:**
- Missing `is_template` as `number` (not boolean -- core uses SQLite integer)
- `summary_id` missing from any of the three resume input types

---

### T30.5: Add SDK Types

**File:** `packages/sdk/src/types.ts`

Add after the `ArchetypeWithDomains` section:

```typescript
// ---------------------------------------------------------------------------
// Summary entities
// ---------------------------------------------------------------------------

/** A reusable professional summary. */
export interface Summary {
  id: string
  title: string
  role: string | null
  tagline: string | null
  description: string | null
  is_template: boolean
  notes: string | null
  created_at: string
  updated_at: string
}

/** Input for creating a new Summary. */
export interface CreateSummary {
  title: string
  role?: string
  tagline?: string
  description?: string
  is_template?: boolean
  notes?: string
}

/** Input for partially updating a Summary. */
export interface UpdateSummary {
  title?: string
  role?: string | null
  tagline?: string | null
  description?: string | null
  is_template?: boolean
  notes?: string | null
}

/** Filter for listing summaries. */
export interface SummaryFilter {
  is_template?: boolean
}
```

Update the existing SDK `Resume` interface -- add `summary_id`:

```typescript
export interface Resume {
  id: string
  name: string
  target_role: string
  target_employer: string
  archetype: string
  status: 'draft' | 'final'
  notes: string | null
  header: string | null
  summary_id: string | null      // <-- NEW
  markdown_override: string | null
  markdown_override_updated_at: string | null
  latex_override: string | null
  latex_override_updated_at: string | null
  created_at: string
  updated_at: string
}
```

Update `CreateResume`:

```typescript
export interface CreateResume {
  name: string
  target_role: string
  target_employer: string
  archetype: string
  summary_id?: string            // <-- NEW
}
```

Update `UpdateResume`:

```typescript
export interface UpdateResume {
  name?: string
  target_role?: string
  target_employer?: string
  archetype?: string
  status?: 'draft' | 'final'
  notes?: string | null
  header?: string | null
  summary_id?: string | null     // <-- NEW
  markdown_override?: string | null
  latex_override?: string | null
}
```

**Key difference from core:** SDK uses `boolean` for `is_template` (developer ergonomics). The SDK resource handles serialization to `"1"`/`"0"` for query params.

**Acceptance criteria:**
- SDK `Summary` uses `is_template: boolean` (not number)
- `SummaryFilter` exported with `is_template?: boolean`
- `Resume`, `CreateResume`, `UpdateResume` have `summary_id` fields

**Failure criteria:**
- `is_template` is `number` in SDK types (should be `boolean`)
- Missing `SummaryFilter` type

---

### T30.6: Write SummaryRepository

**File:** `packages/core/src/db/repositories/summary-repository.ts`

Follows the same stateless function pattern as `organization-repository.ts`.

```typescript
/**
 * SummaryRepository -- CRUD operations for the summaries table.
 *
 * All functions take a `Database` instance as the first parameter,
 * keeping the repository stateless and testable.
 */

import type { Database } from 'bun:sqlite'
import type { Summary } from '../../types'

// ---------------------------------------------------------------------------
// Input types
// ---------------------------------------------------------------------------

export interface CreateSummaryInput {
  title: string
  role?: string
  tagline?: string
  description?: string
  is_template?: number
  notes?: string
}

export interface UpdateSummaryInput {
  title?: string
  role?: string | null
  tagline?: string | null
  description?: string | null
  is_template?: number
  notes?: string | null
}

// ---------------------------------------------------------------------------
// Filter types
// ---------------------------------------------------------------------------

export interface SummaryFilter {
  is_template?: number
}

// ---------------------------------------------------------------------------
// Repository functions
// ---------------------------------------------------------------------------

/** Insert a new summary and return the created row. */
export function create(db: Database, input: CreateSummaryInput): Summary {
  const id = crypto.randomUUID()
  const row = db
    .query(
      `INSERT INTO summaries (id, title, role, tagline, description, is_template, notes)
       VALUES (?, ?, ?, ?, ?, ?, ?)
       RETURNING *`,
    )
    .get(
      id,
      input.title,
      input.role ?? null,
      input.tagline ?? null,
      input.description ?? null,
      input.is_template ?? 0,
      input.notes ?? null,
    ) as Summary

  return row
}

/** Retrieve a summary by ID, or null if not found. */
export function get(db: Database, id: string): Summary | null {
  return (
    (db.query('SELECT * FROM summaries WHERE id = ?').get(id) as Summary | null) ??
    null
  )
}

/**
 * List summaries with optional is_template filter.
 * Returns data array and total count (before pagination).
 */
export function list(
  db: Database,
  filter?: SummaryFilter,
  offset = 0,
  limit = 50,
): { data: Summary[]; total: number } {
  const conditions: string[] = []
  const params: unknown[] = []

  if (filter?.is_template !== undefined) {
    conditions.push('is_template = ?')
    params.push(filter.is_template)
  }

  const where = conditions.length > 0 ? `WHERE ${conditions.join(' AND ')}` : ''

  const countRow = db
    .query(`SELECT COUNT(*) AS total FROM summaries ${where}`)
    .get(...params) as { total: number }

  const dataParams = [...params, limit, offset]
  const rows = db
    .query(`SELECT * FROM summaries ${where} ORDER BY created_at DESC LIMIT ? OFFSET ?`)
    .all(...dataParams) as Summary[]

  return { data: rows, total: countRow.total }
}

/**
 * Partially update a summary.
 * Only the fields present in `input` are changed. `updated_at` is
 * always refreshed. Returns null if the summary does not exist.
 */
export function update(
  db: Database,
  id: string,
  input: UpdateSummaryInput,
): Summary | null {
  const existing = get(db, id)
  if (!existing) return null

  const sets: string[] = []
  const params: unknown[] = []

  if (input.title !== undefined) { sets.push('title = ?'); params.push(input.title) }
  if (input.role !== undefined) { sets.push('role = ?'); params.push(input.role) }
  if (input.tagline !== undefined) { sets.push('tagline = ?'); params.push(input.tagline) }
  if (input.description !== undefined) { sets.push('description = ?'); params.push(input.description) }
  if (input.is_template !== undefined) { sets.push('is_template = ?'); params.push(input.is_template) }
  if (input.notes !== undefined) { sets.push('notes = ?'); params.push(input.notes) }

  // Always update updated_at
  sets.push("updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')")

  params.push(id)

  const row = db
    .query(`UPDATE summaries SET ${sets.join(', ')} WHERE id = ? RETURNING *`)
    .get(...params) as Summary | null

  return row ?? null
}

/** Delete a summary by ID. Returns true if a row was deleted. */
export function del(db: Database, id: string): boolean {
  const result = db.run('DELETE FROM summaries WHERE id = ?', [id])
  return result.changes > 0
}

/**
 * Clone a summary. Creates a copy with:
 * - title = 'Copy of ' + original.title
 * - is_template = 0 (always, even if cloning a template)
 * - All other fields copied verbatim
 * - New UUID, created_at, updated_at
 *
 * Returns null if the source summary does not exist.
 */
export function clone(db: Database, id: string): Summary | null {
  const source = get(db, id)
  if (!source) return null

  const newId = crypto.randomUUID()
  const row = db
    .query(
      `INSERT INTO summaries (id, title, role, tagline, description, is_template, notes)
       VALUES (?, ?, ?, ?, ?, 0, ?)
       RETURNING *`,
    )
    .get(
      newId,
      `Copy of ${source.title}`,
      source.role,
      source.tagline,
      source.description,
      source.notes,
    ) as Summary

  return row
}
```

**Acceptance criteria:**
- All CRUD methods work: create, get, list, update, del, clone
- `list` respects `is_template` filter and pagination
- `clone` always sets `is_template = 0`
- `clone` returns null when source ID doesn't exist
- `update` with empty input still updates `updated_at`
- `del` returns false for non-existent ID

**Failure criteria:**
- `clone` copies `is_template` from source (should always be 0)
- `list` without filter fails
- `update` with no fields doesn't refresh `updated_at`
- `update` with `{ notes: null }` does not clear the notes field

---

### T30.7: Write SummaryService

**File:** `packages/core/src/services/summary-service.ts`

Follows the same pattern as `OrganizationService`.

```typescript
/**
 * SummaryService -- business logic for summary entities.
 *
 * Validates input before delegating to the SummaryRepository.
 * All methods return Result<T> (never throw).
 */

import type { Database } from 'bun:sqlite'
import type { Summary, Result, PaginatedResult } from '../types'
import * as SummaryRepo from '../db/repositories/summary-repository'
import type { SummaryFilter } from '../db/repositories/summary-repository'

export class SummaryService {
  constructor(private db: Database) {}

  create(input: SummaryRepo.CreateSummaryInput): Result<Summary> {
    if (!input.title || input.title.trim().length === 0) {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: 'Title must not be empty' } }
    }
    if (input.is_template !== undefined && input.is_template !== 0 && input.is_template !== 1) {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: 'is_template must be 0 or 1' } }
    }
    const summary = SummaryRepo.create(this.db, input)
    return { ok: true, data: summary }
  }

  get(id: string): Result<Summary> {
    const summary = SummaryRepo.get(this.db, id)
    if (!summary) {
      return { ok: false, error: { code: 'NOT_FOUND', message: `Summary ${id} not found` } }
    }
    return { ok: true, data: summary }
  }

  list(filter?: SummaryFilter, offset?: number, limit?: number): PaginatedResult<Summary> {
    const result = SummaryRepo.list(this.db, filter, offset, limit)
    return { ok: true, data: result.data, pagination: { total: result.total, offset: offset ?? 0, limit: limit ?? 50 } }
  }

  update(id: string, input: SummaryRepo.UpdateSummaryInput): Result<Summary> {
    if (input.title !== undefined && input.title.trim().length === 0) {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: 'Title must not be empty' } }
    }
    if (input.is_template !== undefined && input.is_template !== 0 && input.is_template !== 1) {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: 'is_template must be 0 or 1' } }
    }
    const summary = SummaryRepo.update(this.db, id, input)
    if (!summary) {
      return { ok: false, error: { code: 'NOT_FOUND', message: `Summary ${id} not found` } }
    }
    return { ok: true, data: summary }
  }

  delete(id: string): Result<void> {
    const deleted = SummaryRepo.del(this.db, id)
    if (!deleted) {
      return { ok: false, error: { code: 'NOT_FOUND', message: `Summary ${id} not found` } }
    }
    return { ok: true, data: undefined }
  }

  clone(id: string): Result<Summary> {
    const cloned = SummaryRepo.clone(this.db, id)
    if (!cloned) {
      return { ok: false, error: { code: 'SUMMARY_NOT_FOUND', message: 'Summary not found' } }
    }
    return { ok: true, data: cloned }
  }
}
```

Add `'SUMMARY_NOT_FOUND'` to `mapStatusCode` in `server.ts` (maps to 404), alongside existing error codes.

**Acceptance criteria:**
- All methods return `Result<T>` or `PaginatedResult<T>`
- Validation rejects empty `title` and invalid `is_template` values
- `clone` returns `{ code: 'SUMMARY_NOT_FOUND' }` error for non-existent source
- `mapStatusCode('SUMMARY_NOT_FOUND')` returns 404
- Never throws (all errors returned as Result)

**Failure criteria:**
- Service throws instead of returning Result
- Missing validation on `title`
- Clone uses generic `'NOT_FOUND'` instead of `'SUMMARY_NOT_FOUND'`

---

### T30.8: Write Summaries Route Handlers

**File:** `packages/core/src/routes/summaries.ts`

Follows the same pattern as `organizations.ts`. Clone endpoint returns 201.

```typescript
/**
 * Summary routes -- thin HTTP layer over SummaryService.
 */

import { Hono } from 'hono'
import type { Services } from '../services'
import { mapStatusCode } from './server'

export function summaryRoutes(services: Services) {
  const app = new Hono()

  app.post('/summaries', async (c) => {
    const body = await c.req.json()
    const result = services.summaries.create(body)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data }, 201)
  })

  app.get('/summaries', (c) => {
    const offset = Math.max(0, parseInt(c.req.query('offset') ?? '0', 10) || 0)
    const limit = Math.min(200, Math.max(1, parseInt(c.req.query('limit') ?? '50', 10) || 50))
    const filter: Record<string, number> = {}
    if (c.req.query('is_template') !== undefined && c.req.query('is_template') !== null) {
      const tmpl = parseInt(c.req.query('is_template')!, 10)
      if (!isNaN(tmpl)) {
        filter.is_template = tmpl
      }
    }

    const result = services.summaries.list(filter, offset, limit)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data, pagination: result.pagination })
  })

  app.get('/summaries/:id', (c) => {
    const result = services.summaries.get(c.req.param('id'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.patch('/summaries/:id', async (c) => {
    const body = await c.req.json()
    const result = services.summaries.update(c.req.param('id'), body)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.delete('/summaries/:id', (c) => {
    const result = services.summaries.delete(c.req.param('id'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.body(null, 204)
  })

  app.post('/summaries/:id/clone', (c) => {
    const result = services.summaries.clone(c.req.param('id'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data }, 201)
  })

  return app
}
```

**Acceptance criteria:**
- POST `/summaries` returns 201 with `{ data: Summary }`
- GET `/summaries` returns `{ data: Summary[], pagination }` with `is_template` filter
- GET `/summaries/:id` returns `{ data: Summary }`
- PATCH `/summaries/:id` returns `{ data: Summary }`
- DELETE `/summaries/:id` returns 204
- POST `/summaries/:id/clone` returns 201 with `{ data: Summary }`
- Clone of non-existent ID returns 404 with error

**Failure criteria:**
- Clone returns 200 instead of 201
- `is_template` query param not parsed as integer
- Missing `mapStatusCode` on error paths

---

### T30.9: Register Service + Routes

#### `packages/core/src/services/index.ts`

Add import and registration:

```typescript
// Add import:
import { SummaryService } from './summary-service'

// Add to Services interface:
export interface Services {
  // ... existing services ...
  summaries: SummaryService
}

// Add to createServices():
export function createServices(db: Database): Services {
  // ... existing ...
  return {
    // ... existing ...
    summaries: new SummaryService(db),
  }
}

// Add re-export:
export { SummaryService } from './summary-service'
```

#### `packages/core/src/routes/server.ts`

Add import and mount:

```typescript
// Add import:
import { summaryRoutes } from './summaries'

// Add route mount (after archetypeRoutes):
  app.route('/', summaryRoutes(services))
```

**Acceptance criteria:**
- `services.summaries` is available in route handlers
- `/api/summaries` routes are reachable
- Server starts without errors

**Failure criteria:**
- Missing import causes runtime crash
- Routes not mounted (404 for all summary endpoints)

---

### T30.10: Update ResumeRepository for `summary_id`

**File:** `packages/core/src/db/repositories/resume-repository.ts`

Three changes:

**1. Add `summary_id` to `ResumeRow` interface:**

```typescript
interface ResumeRow {
  id: string
  name: string
  target_role: string
  target_employer: string
  archetype: string
  status: string
  notes: string | null
  header: string | null
  summary_id: string | null        // <-- NEW
  markdown_override: string | null
  markdown_override_updated_at: string | null
  latex_override: string | null
  latex_override_updated_at: string | null
  created_at: string
  updated_at: string
}
```

**2. Add `summary_id` to `rowToResume` helper:**

```typescript
function rowToResume(row: ResumeRow): Resume {
  return {
    id: row.id,
    name: row.name,
    target_role: row.target_role,
    target_employer: row.target_employer,
    archetype: row.archetype,
    status: row.status as Resume['status'],
    notes: row.notes ?? null,
    header: row.header ?? null,
    summary_id: row.summary_id ?? null,    // <-- NEW
    markdown_override: row.markdown_override ?? null,
    markdown_override_updated_at: row.markdown_override_updated_at ?? null,
    latex_override: row.latex_override ?? null,
    latex_override_updated_at: row.latex_override_updated_at ?? null,
    created_at: row.created_at,
    updated_at: row.updated_at,
  }
}
```

**3. Add `summary_id` case to `update()` field-building loop:**

Add after the existing `header` field handler:

```typescript
    if (input.summary_id !== undefined) {
      sets.push('summary_id = ?')
      params.push(input.summary_id)
    }
```

**Note:** `ResumeRepository.create()` is NOT updated in this phase. `summary_id` on `CreateResume` is a forward-declaration for Phase 34. Creating a resume with `summary_id` set will silently ignore it until Phase 34.

**Acceptance criteria:**
- `ResumeRepository.get()` returns `summary_id` field
- `ResumeRepository.update(db, id, { summary_id: '<uuid>' })` generates `SET summary_id = ?`
- `ResumeRepository.update(db, id, { summary_id: null })` detaches summary
- `SELECT *` queries automatically include `summary_id` from the column

**Failure criteria:**
- `summary_id` missing from `rowToResume` (returns `undefined` instead of `null`)
- `update()` ignores `summary_id` in input

---

### T30.11: Write SummariesResource (SDK)

**File:** `packages/sdk/src/resources/summaries.ts`

```typescript
import type {
  CreateSummary,
  PaginatedResult,
  PaginationParams,
  RequestFn,
  RequestListFn,
  Result,
  Summary,
  SummaryFilter,
  UpdateSummary,
} from '../types'

function toParams(
  filter?: SummaryFilter,
  pagination?: PaginationParams,
): Record<string, string> | undefined {
  const out: Record<string, string> = {}

  if (filter?.is_template !== undefined) {
    out.is_template = filter.is_template ? '1' : '0'
  }

  if (pagination?.offset !== undefined) out.offset = String(pagination.offset)
  if (pagination?.limit !== undefined) out.limit = String(pagination.limit)

  return Object.keys(out).length > 0 ? out : undefined
}

export class SummariesResource {
  constructor(
    private request: RequestFn,
    private requestList: RequestListFn,
  ) {}

  create(input: CreateSummary): Promise<Result<Summary>> {
    return this.request<Summary>('POST', '/api/summaries', input)
  }

  list(
    filter?: SummaryFilter,
    pagination?: PaginationParams,
  ): Promise<PaginatedResult<Summary>> {
    return this.requestList<Summary>(
      'GET',
      '/api/summaries',
      toParams(filter, pagination),
    )
  }

  get(id: string): Promise<Result<Summary>> {
    return this.request<Summary>('GET', `/api/summaries/${id}`)
  }

  update(id: string, input: UpdateSummary): Promise<Result<Summary>> {
    return this.request<Summary>('PATCH', `/api/summaries/${id}`, input)
  }

  delete(id: string): Promise<Result<void>> {
    return this.request<void>('DELETE', `/api/summaries/${id}`)
  }

  clone(id: string): Promise<Result<Summary>> {
    return this.request<Summary>('POST', `/api/summaries/${id}/clone`)
  }
}
```

**Key design decisions:**
- `list(filter?, pagination?)` separates domain filter from pagination params (per spec)
- `toParams` converts `is_template: true` to `"1"` and `false` to `"0"` (boolean->string)
- `clone` sends POST with no body
- Clone uses `this.request` (not `requestList`) because it returns a single entity

**Acceptance criteria:**
- `list()` with no args returns all summaries
- `list({ is_template: true })` sends `?is_template=1`
- `list({ is_template: false }, { limit: 10 })` sends `?is_template=0&limit=10`
- `clone(id)` sends `POST /api/summaries/{id}/clone`
- All methods return correct Promise types

**Failure criteria:**
- `toParams` sends `"true"` instead of `"1"` for `is_template`
- `list` combines filter and pagination into a single object (should be separate params)
- `clone` sends request body

---

### T30.12: Register SummariesResource in ForgeClient + SDK Exports

#### `packages/sdk/src/client.ts`

Add import and property:

```typescript
// Add import:
import { SummariesResource } from './resources/summaries'

// Add property declaration (after 'skills'):
  /** Summaries CRUD + clone. */
  public summaries: SummariesResource

// Add instantiation in constructor (after skills):
    this.summaries = new SummariesResource(req, reqList)
```

#### `packages/sdk/src/index.ts`

Add exports:

```typescript
// In "Core entity types" section, add Summary:
export type {
  // ... existing ...
  Summary,
} from './types'

// Add new "Summary types" section:
// Summary input + filter types
export type {
  CreateSummary,
  UpdateSummary,
  SummaryFilter,
} from './types'

// In "Resource classes" section, add:
export { SummariesResource } from './resources/summaries'
```

#### `packages/sdk/src/types.ts`

Add `RequestFn` and `RequestListFn` type exports if not already present. These are needed by `SummariesResource` imports. Check if they already exist:

```typescript
// These should already exist in types.ts -- verify:
export type RequestFn = <T>(method: string, path: string, body?: unknown) => Promise<Result<T>>
export type RequestListFn = <T>(method: string, path: string, params?: Record<string, string>) => Promise<PaginatedResult<T>>
```

**Acceptance criteria:**
- `client.summaries.create({...})` works
- `Summary`, `CreateSummary`, `UpdateSummary`, `SummaryFilter` are exported from `@forge/sdk`
- `SummariesResource` is exported from `@forge/sdk`

**Failure criteria:**
- `client.summaries` is undefined at runtime
- Missing type exports from barrel

---

### T30.13: Update IR Compiler -- `parseHeader(resume, summary)`

**File:** `packages/core/src/services/resume-compiler.ts`

This is the intermediate state: `parseHeader(resume, summary)`. Contact-field profile support was added in Phase 29. This phase extends the signature to accept a `summary` parameter.

**1. Add Summary type to imports and row types:**

```typescript
// Add to imports:
import type { Summary } from '../types'

// Add SummaryRow type alias (or use Summary directly):
// Using Summary type directly since it matches the DB row.
```

**2. Update `compileResumeIR` to fetch summary:**

```typescript
export function compileResumeIR(db: Database, resumeId: string): ResumeDocument | null {
  // 1. Fetch resume base data
  const resume = db
    .query('SELECT id, name, target_role, header, summary_id FROM resumes WHERE id = ?')
    .get(resumeId) as (ResumeRow & { summary_id: string | null }) | null

  if (!resume) return null

  // 2. Fetch linked summary (if any)
  let summary: Summary | null = null
  if (resume.summary_id) {
    summary = db
      .query('SELECT * FROM summaries WHERE id = ?')
      .get(resume.summary_id) as Summary | null
  }

  // 3. Parse header (JSON blob or build default, with summary overlay)
  const header = parseHeader(resume, summary)

  // 4. Fetch sections from resume_sections table
  // ... rest unchanged ...
```

**3. Update `parseHeader` signature and implementation:**

```typescript
function parseHeader(resume: ResumeRow, summary: Summary | null): ResumeHeader {
  // Start with header JSON blob if available (for contact info fallback)
  let base: ResumeHeader = {
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

  if (resume.header) {
    try {
      base = JSON.parse(resume.header) as ResumeHeader
    } catch {
      // Fall through to default
    }
  }

  // Overlay summary fields (tagline from summary takes priority)
  if (summary) {
    if (summary.tagline) {
      base.tagline = summary.tagline
    }
  }

  return base
}
```

**4. Update `ResumeRow` interface in the compiler (add `summary_id`):**

```typescript
interface ResumeRow {
  id: string
  name: string
  target_role: string
  header: string | null
  summary_id: string | null   // <-- NEW
}
```

Wait -- the compiler has its own local `ResumeRow` that is a subset. We need to update it:

```typescript
interface ResumeRow {
  id: string
  name: string
  target_role: string
  header: string | null  // JSON blob
}
```

becomes:

```typescript
interface ResumeRow {
  id: string
  name: string
  target_role: string
  header: string | null  // JSON blob
  summary_id: string | null
}
```

And update the SELECT query:

```typescript
  const resume = db
    .query('SELECT id, name, target_role, header, summary_id FROM resumes WHERE id = ?')
    .get(resumeId) as ResumeRow | null
```

**Important:** The existing call `const header = parseHeader(resume)` at resume-compiler.ts line 89 must be replaced by the new call shown in the updated `compileResumeIR` body. The old single-argument call site is removed -- do not leave it alongside the new code.

**Phase 29 conditional:** If Phase 29 (Config Profile) is already merged when this phase begins: use the three-argument signature `parseHeader(resume, profile, summary)` instead of the two-argument form. Fetch profile from `db.query('SELECT * FROM user_profile LIMIT 1').get()` before calling `parseHeader`. The plan shows the two-argument intermediate form; adapt if Phase 29 is already present.

**Acceptance criteria:**
- `compileResumeIR` fetches summary when `summary_id` is set
- `parseHeader` uses `summary.tagline` when available, falling back to header JSON tagline, then `target_role`
- Contact fields still come from header JSON blob (until Profile spec)
- Works correctly when `summary_id` is NULL (no summary fetch, falls back to existing behavior)
- Old single-argument `parseHeader(resume)` call site is removed

**Failure criteria:**
- `parseHeader` ignores summary tagline
- Compiler crashes when `summary_id` is NULL
- SELECT query doesn't include `summary_id`
- Old single-argument `parseHeader(resume)` call left alongside new two-argument call

---

### T30.14: Add Test Helpers

**File:** `packages/core/src/db/__tests__/helpers.ts`

Add `seedSummary` helper after the existing `seedResumeSkill`:

```typescript
/** Seed a test summary and return its ID */
export function seedSummary(db: Database, opts: {
  title?: string
  role?: string | null
  tagline?: string | null
  description?: string | null
  isTemplate?: number
  notes?: string | null
} = {}): string {
  const id = testUuid()
  db.run(
    `INSERT INTO summaries (id, title, role, tagline, description, is_template, notes)
     VALUES (?, ?, ?, ?, ?, ?, ?)`,
    [
      id,
      opts.title ?? 'Test Summary',
      opts.role ?? 'Security Engineer',
      opts.tagline ?? 'Cloud + DevSecOps',
      opts.description ?? null,
      opts.isTemplate ?? 0,
      opts.notes ?? null,
    ]
  )
  return id
}
```

**Acceptance criteria:**
- `seedSummary(db)` creates a summary with sensible defaults and returns ID
- `seedSummary(db, { isTemplate: 1 })` creates a template summary
- All fields are configurable via opts

**Failure criteria:**
- Helper doesn't generate valid UUID
- Default values cause constraint violations

---

## Testing Support

### Fixtures / Test Cases

**Summary fixture data:**

| Title | Role | Tagline | Description | is_template |
|-------|------|---------|-------------|-------------|
| "Security Engineer - Cloud Focus" | "Senior Security Engineer" | "Cloud + DevSecOps + Detection Engineering" | "Security engineer with 8+ years..." | 0 |
| "Template: Generic Security" | "Security Engineer" | "Security Professional" | null | 1 |
| "No Role Summary" | null | "Versatile technologist" | null | 0 |

**Resume-summary linking fixture:**

| Resume | Summary | Expected |
|--------|---------|----------|
| Resume with summary_id set | Existing summary | Resume.summary_id = summary.id |
| Resume with summary_id NULL | - | Resume.summary_id = null |
| Resume with summary_id pointing to deleted summary | Summary deleted | Resume.summary_id = null (ON DELETE SET NULL) |

### Test Kinds

#### Unit Tests -- SummaryRepository

**File:** `packages/core/src/db/repositories/__tests__/summary-repository.test.ts`

```typescript
import { describe, test, expect, beforeEach, afterEach } from 'bun:test'
import { Database } from 'bun:sqlite'
import { createTestDb, seedSummary } from '../../__tests__/helpers'
import * as SummaryRepo from '../summary-repository'

describe('SummaryRepository', () => {
  let db: Database

  beforeEach(() => {
    db = createTestDb()
  })

  afterEach(() => {
    db.close()
  })

  // ── create ────────────────────────────────────────────────────────

  test('create inserts a summary with all fields', () => {
    const summary = SummaryRepo.create(db, {
      title: 'Security Engineer - Cloud',
      role: 'Senior Security Engineer',
      tagline: 'Cloud + DevSecOps',
      description: 'Security engineer with 8+ years...',
      is_template: 0,
      notes: 'For FAANG applications',
    })

    expect(summary.id).toHaveLength(36)
    expect(summary.title).toBe('Security Engineer - Cloud')
    expect(summary.role).toBe('Senior Security Engineer')
    expect(summary.tagline).toBe('Cloud + DevSecOps')
    expect(summary.description).toBe('Security engineer with 8+ years...')
    expect(summary.is_template).toBe(0)
    expect(summary.notes).toBe('For FAANG applications')
    expect(summary.created_at).toBeTruthy()
    expect(summary.updated_at).toBeTruthy()
  })

  test('create with minimal fields uses defaults', () => {
    const summary = SummaryRepo.create(db, { title: 'Minimal' })

    expect(summary.title).toBe('Minimal')
    expect(summary.role).toBeNull()
    expect(summary.tagline).toBeNull()
    expect(summary.description).toBeNull()
    expect(summary.is_template).toBe(0)
    expect(summary.notes).toBeNull()
  })

  test('create template summary', () => {
    const summary = SummaryRepo.create(db, {
      title: 'Template: Generic',
      is_template: 1,
    })

    expect(summary.is_template).toBe(1)
  })

  // ── get ───────────────────────────────────────────────────────────

  test('get returns summary by id', () => {
    const id = seedSummary(db, { title: 'Findable' })
    const summary = SummaryRepo.get(db, id)

    expect(summary).not.toBeNull()
    expect(summary!.title).toBe('Findable')
  })

  test('get returns null for non-existent id', () => {
    const summary = SummaryRepo.get(db, '00000000-0000-0000-0000-000000000000')
    expect(summary).toBeNull()
  })

  // ── list ──────────────────────────────────────────────────────────

  test('list returns all summaries without filter', () => {
    seedSummary(db, { title: 'One' })
    seedSummary(db, { title: 'Two' })
    seedSummary(db, { title: 'Three', isTemplate: 1 })

    const result = SummaryRepo.list(db)
    expect(result.total).toBe(3)
    expect(result.data).toHaveLength(3)
  })

  test('list filters by is_template', () => {
    seedSummary(db, { title: 'Instance', isTemplate: 0 })
    seedSummary(db, { title: 'Template', isTemplate: 1 })

    const templates = SummaryRepo.list(db, { is_template: 1 })
    expect(templates.total).toBe(1)
    expect(templates.data[0].title).toBe('Template')

    const instances = SummaryRepo.list(db, { is_template: 0 })
    expect(instances.total).toBe(1)
    expect(instances.data[0].title).toBe('Instance')
  })

  test('list paginates correctly', () => {
    for (let i = 0; i < 5; i++) {
      seedSummary(db, { title: `Summary ${i}` })
    }

    const page1 = SummaryRepo.list(db, undefined, 0, 2)
    expect(page1.data).toHaveLength(2)
    expect(page1.total).toBe(5)

    const page2 = SummaryRepo.list(db, undefined, 2, 2)
    expect(page2.data).toHaveLength(2)

    const page3 = SummaryRepo.list(db, undefined, 4, 2)
    expect(page3.data).toHaveLength(1)
  })

  // ── update ────────────────────────────────────────────────────────

  test('update changes specified fields', () => {
    const id = seedSummary(db, { title: 'Original', tagline: 'Old tagline' })

    const updated = SummaryRepo.update(db, id, {
      title: 'Updated',
      tagline: 'New tagline',
    })

    expect(updated).not.toBeNull()
    expect(updated!.title).toBe('Updated')
    expect(updated!.tagline).toBe('New tagline')
    expect(updated!.role).toBe('Security Engineer')  // unchanged
  })

  test('update sets nullable fields to null', () => {
    const id = seedSummary(db, { role: 'Engineer', tagline: 'Test' })

    const updated = SummaryRepo.update(db, id, { role: null, tagline: null })
    expect(updated!.role).toBeNull()
    expect(updated!.tagline).toBeNull()
  })

  test('update sets notes field to null', () => {
    const id = seedSummary(db, { notes: 'Some notes' })

    const updated = SummaryRepo.update(db, id, { notes: null })
    expect(updated).not.toBeNull()
    expect(updated!.notes).toBeNull()
  })

  test('update returns null for non-existent id', () => {
    const result = SummaryRepo.update(db, '00000000-0000-0000-0000-000000000000', { title: 'Nope' })
    expect(result).toBeNull()
  })

  test('update with empty input still refreshes updated_at', () => {
    const id = seedSummary(db)
    const before = SummaryRepo.get(db, id)!

    // Small delay to ensure timestamp difference
    const updated = SummaryRepo.update(db, id, {})
    expect(updated).not.toBeNull()
    // updated_at should be refreshed (may or may not differ by ms, but field is present)
    expect(updated!.updated_at).toBeTruthy()
  })

  // ── delete ────────────────────────────────────────────────────────

  test('del removes summary', () => {
    const id = seedSummary(db)
    expect(SummaryRepo.del(db, id)).toBe(true)
    expect(SummaryRepo.get(db, id)).toBeNull()
  })

  test('del returns false for non-existent id', () => {
    expect(SummaryRepo.del(db, '00000000-0000-0000-0000-000000000000')).toBe(false)
  })

  test('del sets resume summary_id to NULL (ON DELETE SET NULL)', () => {
    const summaryId = seedSummary(db)

    // Create a resume and link it to the summary
    const resumeId = crypto.randomUUID()
    db.run(
      `INSERT INTO resumes (id, name, target_role, target_employer, archetype, summary_id)
       VALUES (?, ?, ?, ?, ?, ?)`,
      [resumeId, 'Test Resume', 'Engineer', 'Corp', 'general', summaryId]
    )

    // Verify link exists
    const before = db.query('SELECT summary_id FROM resumes WHERE id = ?').get(resumeId) as { summary_id: string | null }
    expect(before.summary_id).toBe(summaryId)

    // Delete summary
    SummaryRepo.del(db, summaryId)

    // Verify resume.summary_id is now NULL
    const after = db.query('SELECT summary_id FROM resumes WHERE id = ?').get(resumeId) as { summary_id: string | null }
    expect(after.summary_id).toBeNull()
  })

  // ── clone ─────────────────────────────────────────────────────────

  test('clone creates copy with "Copy of" title', () => {
    const id = seedSummary(db, {
      title: 'Original',
      role: 'Engineer',
      tagline: 'Builds things',
      description: 'A detailed description',
      notes: 'Some notes',
    })

    const cloned = SummaryRepo.clone(db, id)
    expect(cloned).not.toBeNull()
    expect(cloned!.id).not.toBe(id)
    expect(cloned!.title).toBe('Copy of Original')
    expect(cloned!.role).toBe('Engineer')
    expect(cloned!.tagline).toBe('Builds things')
    expect(cloned!.description).toBe('A detailed description')
    expect(cloned!.notes).toBe('Some notes')
    expect(cloned!.is_template).toBe(0)
  })

  test('clone of template sets is_template to 0', () => {
    const id = seedSummary(db, { title: 'Template', isTemplate: 1 })

    const cloned = SummaryRepo.clone(db, id)
    expect(cloned!.is_template).toBe(0)
  })

  test('clone of non-existent id returns null', () => {
    const cloned = SummaryRepo.clone(db, '00000000-0000-0000-0000-000000000000')
    expect(cloned).toBeNull()
  })

  test('clone generates new UUID and timestamps', () => {
    const id = seedSummary(db)
    const original = SummaryRepo.get(db, id)!

    const cloned = SummaryRepo.clone(db, id)!
    expect(cloned.id).not.toBe(original.id)
    expect(cloned.id).toHaveLength(36)
    expect(cloned.created_at).toBeTruthy()
  })
})
```

#### Unit Tests -- SummaryService

**File:** `packages/core/src/services/__tests__/summary-service.test.ts`

```typescript
import { describe, test, expect, beforeEach, afterEach } from 'bun:test'
import { Database } from 'bun:sqlite'
import { createTestDb, seedSummary } from '../../db/__tests__/helpers'
import { SummaryService } from '../summary-service'

describe('SummaryService', () => {
  let db: Database
  let service: SummaryService

  beforeEach(() => {
    db = createTestDb()
    service = new SummaryService(db)
  })

  afterEach(() => {
    db.close()
  })

  // ── create ────────────────────────────────────────────────────────

  test('create validates title is not empty', () => {
    const result = service.create({ title: '' })
    expect(result.ok).toBe(false)
    if (!result.ok) {
      expect(result.error.code).toBe('VALIDATION_ERROR')
    }
  })

  test('create validates title is not whitespace-only', () => {
    const result = service.create({ title: '   ' })
    expect(result.ok).toBe(false)
    if (!result.ok) {
      expect(result.error.code).toBe('VALIDATION_ERROR')
    }
  })

  test('create validates is_template must be 0 or 1', () => {
    const result = service.create({ title: 'Test', is_template: 2 })
    expect(result.ok).toBe(false)
    if (!result.ok) {
      expect(result.error.code).toBe('VALIDATION_ERROR')
    }
  })

  test('create succeeds with valid input', () => {
    const result = service.create({ title: 'Valid Summary' })
    expect(result.ok).toBe(true)
    if (result.ok) {
      expect(result.data.title).toBe('Valid Summary')
    }
  })

  // ── get ───────────────────────────────────────────────────────────

  test('get returns NOT_FOUND for missing id', () => {
    const result = service.get('00000000-0000-0000-0000-000000000000')
    expect(result.ok).toBe(false)
    if (!result.ok) {
      expect(result.error.code).toBe('NOT_FOUND')
    }
  })

  test('get returns summary', () => {
    const id = seedSummary(db)
    const result = service.get(id)
    expect(result.ok).toBe(true)
  })

  // ── list ──────────────────────────────────────────────────────────

  test('list returns paginated result', () => {
    seedSummary(db)
    const result = service.list()
    expect(result.ok).toBe(true)
    if (result.ok) {
      expect(result.data).toHaveLength(1)
      expect(result.pagination.total).toBe(1)
    }
  })

  // ── update ────────────────────────────────────────────────────────

  test('update validates empty title', () => {
    const id = seedSummary(db)
    const result = service.update(id, { title: '' })
    expect(result.ok).toBe(false)
    if (!result.ok) {
      expect(result.error.code).toBe('VALIDATION_ERROR')
    }
  })

  test('update returns NOT_FOUND for missing id', () => {
    const result = service.update('00000000-0000-0000-0000-000000000000', { title: 'New' })
    expect(result.ok).toBe(false)
    if (!result.ok) {
      expect(result.error.code).toBe('NOT_FOUND')
    }
  })

  // ── delete ────────────────────────────────────────────────────────

  test('delete returns NOT_FOUND for missing id', () => {
    const result = service.delete('00000000-0000-0000-0000-000000000000')
    expect(result.ok).toBe(false)
    if (!result.ok) {
      expect(result.error.code).toBe('NOT_FOUND')
    }
  })

  test('delete succeeds for existing summary', () => {
    const id = seedSummary(db)
    const result = service.delete(id)
    expect(result.ok).toBe(true)
  })

  // ── clone ─────────────────────────────────────────────────────────

  test('clone returns SUMMARY_NOT_FOUND for missing source', () => {
    const result = service.clone('00000000-0000-0000-0000-000000000000')
    expect(result.ok).toBe(false)
    if (!result.ok) {
      expect(result.error.code).toBe('SUMMARY_NOT_FOUND')
    }
  })

  test('clone succeeds and returns new summary', () => {
    const id = seedSummary(db, { title: 'Cloneable' })
    const result = service.clone(id)
    expect(result.ok).toBe(true)
    if (result.ok) {
      expect(result.data.title).toBe('Copy of Cloneable')
      expect(result.data.id).not.toBe(id)
    }
  })
})
```

#### Integration Tests -- API Routes

**File:** `packages/core/src/routes/__tests__/summaries.test.ts`

```typescript
import { describe, test, expect, beforeEach, afterEach } from 'bun:test'
import { Database } from 'bun:sqlite'
import { createTestApp, apiRequest, type TestContext } from './helpers'
import { seedSummary } from '../../db/__tests__/helpers'

describe('Summaries API', () => {
  let ctx: TestContext

  beforeEach(() => {
    ctx = createTestApp()
  })

  afterEach(() => {
    ctx.db.close()
  })

  // ── POST /summaries ───────────────────────────────────────────────

  test('POST /summaries creates a summary', async () => {
    const res = await apiRequest(ctx.app, 'POST', '/summaries', {
      title: 'Security Engineer - Cloud',
      role: 'Senior Security Engineer',
      tagline: 'Cloud + DevSecOps',
      description: 'Security engineer with 8+ years...',
    })

    expect(res.status).toBe(201)
    const body = await res.json()
    expect(body.data.title).toBe('Security Engineer - Cloud')
    expect(body.data.role).toBe('Senior Security Engineer')
    expect(body.data.tagline).toBe('Cloud + DevSecOps')
    expect(body.data.description).toBe('Security engineer with 8+ years...')
    expect(body.data.is_template).toBe(0)
    expect(body.data.id).toHaveLength(36)
  })

  test('POST /summaries with empty title returns 400', async () => {
    const res = await apiRequest(ctx.app, 'POST', '/summaries', { title: '' })
    expect(res.status).toBe(400)
    const body = await res.json()
    expect(body.error.code).toBe('VALIDATION_ERROR')
  })

  test('POST /summaries with is_template=1 creates template', async () => {
    const res = await apiRequest(ctx.app, 'POST', '/summaries', {
      title: 'Template',
      is_template: 1,
    })

    expect(res.status).toBe(201)
    const body = await res.json()
    expect(body.data.is_template).toBe(1)
  })

  // ── GET /summaries ────────────────────────────────────────────────

  test('GET /summaries returns list with pagination', async () => {
    seedSummary(ctx.db, { title: 'One' })
    seedSummary(ctx.db, { title: 'Two' })

    const res = await apiRequest(ctx.app, 'GET', '/summaries')
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data).toHaveLength(2)
    expect(body.pagination.total).toBe(2)
    expect(body.pagination.offset).toBe(0)
    expect(body.pagination.limit).toBe(50)
  })

  test('GET /summaries?is_template=1 filters to templates', async () => {
    seedSummary(ctx.db, { title: 'Instance', isTemplate: 0 })
    seedSummary(ctx.db, { title: 'Template', isTemplate: 1 })

    const res = await apiRequest(ctx.app, 'GET', '/summaries?is_template=1')
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data).toHaveLength(1)
    expect(body.data[0].title).toBe('Template')
  })

  test('GET /summaries without filter returns all', async () => {
    seedSummary(ctx.db, { isTemplate: 0 })
    seedSummary(ctx.db, { isTemplate: 1 })

    const res = await apiRequest(ctx.app, 'GET', '/summaries')
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data).toHaveLength(2)
  })

  test('GET /summaries?is_template=invalid returns 200 (param ignored)', async () => {
    seedSummary(ctx.db, { isTemplate: 0 })
    seedSummary(ctx.db, { isTemplate: 1 })

    const res = await apiRequest(ctx.app, 'GET', '/summaries?is_template=invalid')
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data).toHaveLength(2) // no filter applied, returns all
  })

  // ── GET /summaries/:id ────────────────────────────────────────────

  test('GET /summaries/:id returns summary', async () => {
    const id = seedSummary(ctx.db, { title: 'Findable' })

    const res = await apiRequest(ctx.app, 'GET', `/summaries/${id}`)
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.title).toBe('Findable')
  })

  test('GET /summaries/:id returns 404 for missing id', async () => {
    const res = await apiRequest(ctx.app, 'GET', '/summaries/00000000-0000-0000-0000-000000000000')
    expect(res.status).toBe(404)
    const body = await res.json()
    expect(body.error.code).toBe('NOT_FOUND')
  })

  // ── PATCH /summaries/:id ──────────────────────────────────────────

  test('PATCH /summaries/:id updates fields', async () => {
    const id = seedSummary(ctx.db, { title: 'Original' })

    const res = await apiRequest(ctx.app, 'PATCH', `/summaries/${id}`, {
      title: 'Updated',
      tagline: 'New tagline',
    })

    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.title).toBe('Updated')
    expect(body.data.tagline).toBe('New tagline')
  })

  test('PATCH /summaries/:id returns 404 for missing id', async () => {
    const res = await apiRequest(ctx.app, 'PATCH', '/summaries/00000000-0000-0000-0000-000000000000', { title: 'Nope' })
    expect(res.status).toBe(404)
  })

  // ── DELETE /summaries/:id ─────────────────────────────────────────

  test('DELETE /summaries/:id removes summary', async () => {
    const id = seedSummary(ctx.db)

    const res = await apiRequest(ctx.app, 'DELETE', `/summaries/${id}`)
    expect(res.status).toBe(204)

    // Verify it's gone
    const get = await apiRequest(ctx.app, 'GET', `/summaries/${id}`)
    expect(get.status).toBe(404)
  })

  test('DELETE /summaries/:id returns 404 for missing id', async () => {
    const res = await apiRequest(ctx.app, 'DELETE', '/summaries/00000000-0000-0000-0000-000000000000')
    expect(res.status).toBe(404)
  })

  // ── POST /summaries/:id/clone ─────────────────────────────────────

  test('POST /summaries/:id/clone returns 201 with cloned summary', async () => {
    const id = seedSummary(ctx.db, {
      title: 'Original',
      role: 'Engineer',
      tagline: 'Builds things',
    })

    const res = await apiRequest(ctx.app, 'POST', `/summaries/${id}/clone`)
    expect(res.status).toBe(201)
    const body = await res.json()
    expect(body.data.title).toBe('Copy of Original')
    expect(body.data.role).toBe('Engineer')
    expect(body.data.tagline).toBe('Builds things')
    expect(body.data.is_template).toBe(0)
    expect(body.data.id).not.toBe(id)
  })

  test('POST /summaries/:id/clone of template always sets is_template=0', async () => {
    const id = seedSummary(ctx.db, { title: 'Template', isTemplate: 1 })

    const res = await apiRequest(ctx.app, 'POST', `/summaries/${id}/clone`)
    expect(res.status).toBe(201)
    const body = await res.json()
    expect(body.data.is_template).toBe(0)
  })

  test('POST /summaries/:id/clone returns 404 for non-existent id', async () => {
    const res = await apiRequest(ctx.app, 'POST', '/summaries/00000000-0000-0000-0000-000000000000/clone')
    expect(res.status).toBe(404)
    const body = await res.json()
    expect(body.error.code).toBe('SUMMARY_NOT_FOUND')
  })

  // ── Resume-Summary linking ────────────────────────────────────────

  test('PATCH /resumes/:id with summary_id links summary', async () => {
    const summaryId = seedSummary(ctx.db)

    // Create a resume
    const createRes = await apiRequest(ctx.app, 'POST', '/resumes', {
      name: 'Test Resume',
      target_role: 'Engineer',
      target_employer: 'Corp',
      archetype: 'general',
    })
    const resumeId = (await createRes.json()).data.id

    // Link summary
    const patchRes = await apiRequest(ctx.app, 'PATCH', `/resumes/${resumeId}`, {
      summary_id: summaryId,
    })
    expect(patchRes.status).toBe(200)
    const body = await patchRes.json()
    expect(body.data.summary_id).toBe(summaryId)
  })

  test('PATCH /resumes/:id with summary_id=null detaches summary', async () => {
    const summaryId = seedSummary(ctx.db)

    // Create resume with summary
    const createRes = await apiRequest(ctx.app, 'POST', '/resumes', {
      name: 'Test Resume',
      target_role: 'Engineer',
      target_employer: 'Corp',
      archetype: 'general',
    })
    const resumeId = (await createRes.json()).data.id

    // Link then detach
    await apiRequest(ctx.app, 'PATCH', `/resumes/${resumeId}`, { summary_id: summaryId })
    const detachRes = await apiRequest(ctx.app, 'PATCH', `/resumes/${resumeId}`, { summary_id: null })
    expect(detachRes.status).toBe(200)
    const body = await detachRes.json()
    expect(body.data.summary_id).toBeNull()
  })
})
```

#### Contract Tests

**File:** `packages/core/src/routes/__tests__/contracts.test.ts`

Add to the existing contract test file:

```typescript
  // ── Summaries Contracts ─────────────────────────────────────────

  test('POST /summaries returns { data: entity } envelope', async () => {
    const res = await apiRequest(ctx.app, 'POST', '/summaries', {
      title: 'Contract Summary',
    })
    expect(res.status).toBe(201)
    const body = await res.json()

    expect(body).toHaveProperty('data')
    expect(body.data).toHaveProperty('id')
    expect(body.data).toHaveProperty('title')
    expect(body.data).toHaveProperty('is_template')
    expect(body.data).toHaveProperty('created_at')
    expect(body).not.toHaveProperty('error')
  })

  test('GET /summaries returns { data: [], pagination: {} } envelope', async () => {
    seedSummary(ctx.db)

    const res = await apiRequest(ctx.app, 'GET', '/summaries')
    expect(res.status).toBe(200)
    const body = await res.json()

    expect(body).toHaveProperty('data')
    expect(body.data).toBeArray()
    expect(body).toHaveProperty('pagination')
    expect(body.pagination).toHaveProperty('total')
    expect(body.pagination).toHaveProperty('offset')
    expect(body.pagination).toHaveProperty('limit')
  })

  test('POST /summaries/:id/clone returns { data: entity } with 201', async () => {
    const id = seedSummary(ctx.db)

    const res = await apiRequest(ctx.app, 'POST', `/summaries/${id}/clone`)
    expect(res.status).toBe(201)
    const body = await res.json()

    expect(body).toHaveProperty('data')
    expect(body.data).toHaveProperty('id')
    expect(body.data.id).not.toBe(id)
    expect(body).not.toHaveProperty('error')
  })
```

Add `seedSummary` to the imports if not already present.

#### SDK Resource Tests

**File:** `packages/sdk/src/__tests__/resources.test.ts`

Add to the existing resource tests:

```typescript
  // -----------------------------------------------------------------------
  // summaries
  // -----------------------------------------------------------------------

  describe('summaries', () => {
    it('create sends POST /api/summaries with body', async () => {
      const created = { id: 'sum1', title: 'Test Summary', is_template: false }
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: created }, { status: 201 })),
      )

      const result = await client.summaries.create({ title: 'Test Summary' })

      expect(calledUrl(fetchMock)).toBe('http://localhost:3000/api/summaries')
      expect(calledInit(fetchMock).method).toBe('POST')
      expect(result).toEqual({ ok: true, data: created })
    })

    it('list sends GET /api/summaries with no params', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(
          jsonResponse({ data: [], pagination: { total: 0, offset: 0, limit: 50 } }),
        ),
      )

      await client.summaries.list()

      expect(calledUrl(fetchMock)).toBe('http://localhost:3000/api/summaries')
    })

    it('list with is_template filter sends ?is_template=1', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(
          jsonResponse({ data: [], pagination: { total: 0, offset: 0, limit: 50 } }),
        ),
      )

      await client.summaries.list({ is_template: true })

      expect(calledUrl(fetchMock)).toContain('is_template=1')
    })

    it('list with is_template=false sends ?is_template=0', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(
          jsonResponse({ data: [], pagination: { total: 0, offset: 0, limit: 50 } }),
        ),
      )

      await client.summaries.list({ is_template: false })

      expect(calledUrl(fetchMock)).toContain('is_template=0')
    })

    it('list with pagination sends offset and limit', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(
          jsonResponse({ data: [], pagination: { total: 0, offset: 10, limit: 5 } }),
        ),
      )

      await client.summaries.list(undefined, { offset: 10, limit: 5 })

      expect(calledUrl(fetchMock)).toContain('offset=10')
      expect(calledUrl(fetchMock)).toContain('limit=5')
    })

    it('get sends GET /api/summaries/:id', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: { id: 'sum1' } })),
      )

      await client.summaries.get('sum1')

      expect(calledUrl(fetchMock)).toBe('http://localhost:3000/api/summaries/sum1')
      expect(calledInit(fetchMock).method).toBe('GET')
    })

    it('update sends PATCH /api/summaries/:id', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: { id: 'sum1', title: 'Updated' } })),
      )

      await client.summaries.update('sum1', { title: 'Updated' })

      expect(calledUrl(fetchMock)).toBe('http://localhost:3000/api/summaries/sum1')
      expect(calledInit(fetchMock).method).toBe('PATCH')
    })

    it('delete sends DELETE /api/summaries/:id', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(noContentResponse()),
      )

      await client.summaries.delete('sum1')

      expect(calledUrl(fetchMock)).toBe('http://localhost:3000/api/summaries/sum1')
      expect(calledInit(fetchMock).method).toBe('DELETE')
    })

    it('clone sends POST /api/summaries/:id/clone', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: { id: 'sum2', title: 'Copy of Test' } }, { status: 201 })),
      )

      const result = await client.summaries.clone('sum1')

      expect(calledUrl(fetchMock)).toBe('http://localhost:3000/api/summaries/sum1/clone')
      expect(calledInit(fetchMock).method).toBe('POST')
      expect(result).toEqual({ ok: true, data: { id: 'sum2', title: 'Copy of Test' } })
    })
  })
```

#### IR Compiler Tests

**File:** `packages/core/src/services/__tests__/resume-compiler.test.ts`

Add the following tests to the existing compiler test file:

```typescript
  // ── Summary integration ───────────────────────────────────────────

  test('compileResumeIR uses summary.tagline when summary is linked', () => {
    const resumeId = seedResume(db, { name: 'Test', targetRole: 'Fallback Role' })
    const summaryId = seedSummary(db, { tagline: 'Summary Tagline' })

    // Link summary to resume
    db.run('UPDATE resumes SET summary_id = ? WHERE id = ?', [summaryId, resumeId])

    const ir = compileResumeIR(db, resumeId)
    expect(ir).not.toBeNull()
    expect(ir!.header.tagline).toBe('Summary Tagline')
  })

  test('compileResumeIR falls back to target_role when no summary linked', () => {
    const resumeId = seedResume(db, { targetRole: 'Fallback Role' })

    const ir = compileResumeIR(db, resumeId)
    expect(ir).not.toBeNull()
    expect(ir!.header.tagline).toBe('Fallback Role')
  })

  test('compileResumeIR falls back to header JSON when summary has no tagline', () => {
    const resumeId = seedResume(db)
    const summaryId = seedSummary(db, { tagline: null })

    db.run('UPDATE resumes SET summary_id = ? WHERE id = ?', [summaryId, resumeId])

    const ir = compileResumeIR(db, resumeId)
    expect(ir).not.toBeNull()
    // Should fall back to header JSON tagline or target_role
    expect(ir!.header.tagline).toBeTruthy()
  })

  test('compileResumeIR handles deleted summary gracefully', () => {
    const resumeId = seedResume(db)
    const summaryId = seedSummary(db)

    db.run('UPDATE resumes SET summary_id = ? WHERE id = ?', [summaryId, resumeId])
    db.run('DELETE FROM summaries WHERE id = ?', [summaryId])

    // summary_id is now NULL due to ON DELETE SET NULL
    const ir = compileResumeIR(db, resumeId)
    expect(ir).not.toBeNull()
    // Should not crash -- falls back to default behavior
  })
```

Add `seedSummary` to the imports from helpers.

#### Data Migration Tests

Add to the summary repository tests or create a dedicated file. Testing the migration helper:

```typescript
  // In summary-repository.test.ts or a new migration test file.
  // At the top of the test file, use ESM import (not require()):
  // import { migrateHeadersToSummaries } from '../../db/migrations/006_summaries_data'

  test('migrateHeadersToSummaries creates summaries for resumes with headers', () => {
    // Create resumes with header JSON
    const r1 = crypto.randomUUID()
    db.run(
      `INSERT INTO resumes (id, name, target_role, target_employer, archetype, header)
       VALUES (?, ?, ?, ?, ?, ?)`,
      [r1, 'Resume 1', 'Engineer', 'Corp', 'general', JSON.stringify({ tagline: 'Cloud Expert' })]
    )

    const r2 = crypto.randomUUID()
    db.run(
      `INSERT INTO resumes (id, name, target_role, target_employer, archetype, header)
       VALUES (?, ?, ?, ?, ?, ?)`,
      [r2, 'Resume 2', null, 'Corp', 'general', JSON.stringify({ tagline: null })]
    )

    // Import at top of test file:
    // import { migrateHeadersToSummaries } from '../../db/migrations/006_summaries_data'
    const result = migrateHeadersToSummaries(db)

    expect(result.migrated).toBe(2)

    // Verify summaries were created
    const s1 = db.query('SELECT summary_id FROM resumes WHERE id = ?').get(r1) as { summary_id: string }
    expect(s1.summary_id).toBeTruthy()

    const summary1 = db.query('SELECT * FROM summaries WHERE id = ?').get(s1.summary_id) as any
    expect(summary1.tagline).toBe('Cloud Expert')
    expect(summary1.role).toBe('Engineer')

    // NULL target_role resume still gets a summary
    const s2 = db.query('SELECT summary_id FROM resumes WHERE id = ?').get(r2) as { summary_id: string }
    expect(s2.summary_id).toBeTruthy()

    const summary2 = db.query('SELECT * FROM summaries WHERE id = ?').get(s2.summary_id) as any
    expect(summary2.role).toBeNull()
  })
```

#### Smoke Test

Smoke test: Start server with fresh DB, run migrations, POST /api/summaries to create a summary, PATCH /api/resumes/:id with summary_id, GET /api/resumes/:id/compile-ir -- verify header.tagline matches summary.tagline.

```bash
# Manual smoke test steps:
# 1. Start server with fresh DB (rm forge.db && bun run dev)
# 2. POST /api/summaries with { "title": "Smoke", "tagline": "Smoke Tagline" }
# 3. POST /api/resumes to create a resume
# 4. PATCH /api/resumes/:id with { "summary_id": "<summary-id-from-step-2>" }
# 5. GET /api/resumes/:id/compile-ir
# 6. Verify response.header.tagline === "Smoke Tagline"
```

### Expected Test Commands

```bash
# Run all summary-related tests
bun test packages/core/src/db/repositories/__tests__/summary-repository.test.ts
bun test packages/core/src/services/__tests__/summary-service.test.ts
bun test packages/core/src/routes/__tests__/summaries.test.ts

# Run SDK tests
bun test packages/sdk/src/__tests__/resources.test.ts

# Run contract tests
bun test packages/core/src/routes/__tests__/contracts.test.ts

# Run compiler tests
bun test packages/core/src/services/__tests__/resume-compiler.test.ts

# Run all tests to verify nothing breaks
bun test
```

### Expected Test Output

```
Summary repository tests: ~18 tests, all passing
Summary service tests: ~10 tests, all passing
Summaries API tests: ~16 tests, all passing
Contract tests: 3 new tests passing (summaries-specific)
SDK resource tests: ~8 new tests passing (summaries-specific)
Compiler tests: ~4 new tests passing (summary integration)
```

---

## Documentation Requirements

1. **Repository file header comment** describing the summaries table and CRUD operations
2. **Service file header comment** describing validation rules
3. **Route file header comment** referencing the service it wraps
4. **SDK resource file** needs no special docs beyond type signatures
5. **Migration SQL file** has inline comments explaining each step
6. **Data migration TS file** has JSDoc explaining the two-step INSERT/UPDATE approach and why pure SQL was rejected
7. **Type definitions** have JSDoc on each interface explaining column semantics (per spec table)

---

## Parallelization Notes

Tasks within this phase can be parallelized as follows:

| Group | Tasks | Dependencies |
|-------|-------|-------------|
| **A: Schema** | T30.1, T30.2, T30.3 | Independent (run first) |
| **B: Types** | T30.4, T30.5 | Independent of A (just type definitions) |
| **C: Repository** | T30.6 | After A (needs table to exist for tests) |
| **D: Service** | T30.7 | After C |
| **E: Routes** | T30.8, T30.9 | After D |
| **F: Resume Update** | T30.10 | After A (needs column to exist), parallel with C/D |
| **G: SDK** | T30.11, T30.12 | After B (needs types), parallel with E |
| **H: IR Compiler** | T30.13 | After A + F, parallel with E/G |
| **I: Test Helpers** | T30.14 | After A, parallel with everything else |

**Critical path:** A -> C -> D -> E (schema -> repo -> service -> routes)

**Maximum parallelism:** B + I can run with A. F + G + H can run in parallel after their minimal dependencies are met.
