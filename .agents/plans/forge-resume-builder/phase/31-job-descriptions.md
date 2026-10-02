# Phase 31: Job Descriptions Entity (Migration 007 + Core + API + SDK + UI)

**Status:** Planning
**Date:** 2026-03-31
**Spec:** [2026-03-30-job-descriptions-entity.md](../refs/specs/2026-03-30-job-descriptions-entity.md)
**Depends on:** Phase 28 (stable baseline). **Soft deps:** Phase 29 (migration 005), Phase 30 (migration 006) -- migration numbering coordination only; DDL is independent.
**Blocks:** Future JD parsing, skill matching, resume-to-JD linking phases
**Parallelizable with:** Phase 29 (Profile), Phase 30 (Summaries), Phase 32 (Nav)

## Goal

Add a `job_descriptions` table to Forge for storing and tracking job postings. Implement the full vertical slice: migration 007, repository, service, API routes, SDK resource class, and UI views. Users will be able to paste a JD, link it to an organization, track its status through an 8-stage pipeline (interested -> analyzing -> applied -> interviewing -> offered / rejected / withdrawn / closed), and browse/filter their saved JDs. The migration also rebuilds `note_references` to add `'job_description'` as a valid `entity_type`.

## Non-Goals

- JD parsing / NLP extraction of requirements, skills, or experience levels
- Skill matching against the user's skill inventory
- Perspective or archetype suggestion based on JD content
- API-based JD fetching (Indeed, LinkedIn, Greenhouse scraping)
- Resume-to-JD gap analysis or linking (no junction table)
- Cover letter generation from JD
- JD deduplication detection (by URL or text similarity)
- JD expiration / auto-close
- Status transition validation (any status -> any status is allowed)
- Salary parsing or structured salary data

## Context

Forge currently tracks organizations and can link them to sources/resumes, but there is no entity for the actual job posting. Users need to paste a JD, track its status through an opportunity pipeline, and eventually match it against their skills and perspectives. This phase adds the minimal v1: paste text, save, track status. The `note_references` table must be rebuilt to add `'job_description'` to its CHECK constraint, since SQLite does not support `ALTER TABLE ... ALTER CONSTRAINT`.

## Scope

| Spec section | Covered here? |
|-------------|---------------|
| 1. Schema (migration 007) | Yes |
| 2. API endpoints (CRUD + filters) | Yes |
| 3. UI (list, detail, modal) | Yes |
| 4. Type changes (core + SDK) | Yes |
| 5. Acceptance criteria | Yes (mapped to tasks) |
| 6. Dependencies & parallelization | Yes |
| 7. Testing | Yes |

## Files to Create

- `packages/core/src/db/migrations/007_job_descriptions.sql`
- `packages/core/src/db/repositories/job-description-repository.ts`
- `packages/core/src/db/repositories/__tests__/job-description-repository.test.ts`
- `packages/core/src/services/job-description-service.ts`
- `packages/core/src/services/__tests__/job-description-service.test.ts`
- `packages/core/src/routes/job-descriptions.ts`
- `packages/core/src/routes/__tests__/job-descriptions.test.ts`
- `packages/sdk/src/resources/job-descriptions.ts`
- `packages/webui/src/pages/JobDescriptions.tsx` (list view)
- `packages/webui/src/pages/JobDescriptionDetail.tsx` (detail view)
- `packages/webui/src/components/NewJobDescriptionModal.tsx`
- `packages/webui/src/components/JobDescriptionCard.tsx`
- `packages/webui/src/components/StatusBadge.tsx` (JD status badge)

## Files to Modify

- `packages/core/src/types/index.ts` (add JD types, `NoteReferenceEntityType`, update `NoteReference`)
- `packages/sdk/src/types.ts` (add JD types, `NoteReferenceEntityType`, update `NoteReference`)
- `packages/sdk/src/client.ts` (register `JobDescriptionsResource`)
- `packages/sdk/src/index.ts` (export new types and resource class)
- `packages/core/src/services/index.ts` (add `JobDescriptionService` to `Services` interface + `createServices`)
- `packages/core/src/routes/server.ts` (mount `jobDescriptionRoutes`)
- `packages/core/src/db/__tests__/helpers.ts` (add `seedJobDescription` helper)
- `packages/core/src/routes/__tests__/contracts.test.ts` (add JD contract tests)

## Fallback Strategies

| Risk | Mitigation |
|------|-----------|
| Migrations 005/006 not yet written when 007 is needed | Migration 007 DDL is independent of 005/006. Only the migration numbering requires ordering. If 005/006 are not ready, temporarily renumber to 005 for local dev, renumber back before merge. |
| `PRAGMA foreign_keys = OFF/ON` silently ignored inside transaction | `PRAGMA foreign_keys` has no effect inside an active transaction. If the migration runner wraps each migration in BEGIN/COMMIT, the PRAGMA calls are silently ignored and FK checks remain active during the rebuild. Verify the runner at `packages/core/src/db/migrate.ts`. If it uses transactions, either (a) split the note_references rebuild into a separate migration file that runs outside a transaction, or (b) modify the runner to execute PRAGMA statements outside the transaction wrapper. |
| Organization autocomplete in UI requires an endpoint that does not exist | Use the existing `GET /api/organizations?search=<term>&limit=10` endpoint. No new endpoint needed. |
| JD text is very large (>100KB) | SQLite TEXT columns have no practical limit. The UI textarea may become sluggish; defer virtual scrolling to a future phase. |

---

## Tasks

### T31.1: Write `007_job_descriptions.sql`

**File:** `packages/core/src/db/migrations/007_job_descriptions.sql`

The migration creates the `job_descriptions` table and rebuilds `note_references` to add `'job_description'` to the entity_type CHECK constraint. The rebuild uses the `PRAGMA foreign_keys = OFF/ON` pattern established in migration 002.

```sql
-- Forge Resume Builder -- Job Descriptions Entity
-- Migration: 007_job_descriptions
-- Date: 2026-03-30
--
-- Adds a table for storing job descriptions linked to organizations.
-- Rebuilds note_references to add 'job_description' to entity_type CHECK.
-- Builds on 006_summaries (migration ordering only; DDL is independent).

-- Step 1: Create job_descriptions table
CREATE TABLE job_descriptions (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  organization_id TEXT REFERENCES organizations(id) ON DELETE SET NULL,
  title TEXT NOT NULL,
  url TEXT,
  raw_text TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'interested' CHECK (status IN (
    'interested', 'analyzing', 'applied', 'interviewing',
    'offered', 'rejected', 'withdrawn', 'closed'
  )),
  salary_range TEXT,
  location TEXT,
  notes TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

CREATE INDEX idx_job_descriptions_org ON job_descriptions(organization_id);
CREATE INDEX idx_job_descriptions_status ON job_descriptions(status);

-- Step 2: Disable FK checks for the note_references rebuild
PRAGMA foreign_keys = OFF;

-- Step 3: Rebuild note_references to add 'job_description' to entity_type CHECK constraint
-- The original CHECK constraint (migration 002) lists:
--   'source', 'bullet', 'perspective', 'resume_entry', 'resume', 'skill', 'organization'
-- SQLite does not support ALTER CHECK, so a table rebuild is needed.
CREATE TABLE note_references_new (
  note_id TEXT NOT NULL CHECK(typeof(note_id) = 'text' AND length(note_id) = 36)
    REFERENCES user_notes(id) ON DELETE CASCADE,
  entity_type TEXT NOT NULL CHECK (entity_type IN (
    'source', 'bullet', 'perspective', 'resume_entry',
    'resume', 'skill', 'organization', 'job_description'
  )),
  entity_id TEXT NOT NULL,
  PRIMARY KEY (note_id, entity_type, entity_id)
) STRICT;

INSERT INTO note_references_new SELECT * FROM note_references;
DROP TABLE note_references;
ALTER TABLE note_references_new RENAME TO note_references;
CREATE INDEX idx_note_refs_entity ON note_references(entity_type, entity_id);

-- Step 4: Re-enable FK checks
PRAGMA foreign_keys = ON;

-- Step 5: Register migration
INSERT INTO _migrations (name) VALUES ('007_job_descriptions');
```

**Key points:**
- `ON DELETE SET NULL` on `organization_id` means deleting an org nulls the FK on linked JDs rather than cascading the delete
- The `note_references` rebuild follows the identical pattern from migration 002/004
- `PRAGMA foreign_keys = OFF/ON` wraps only the table rebuild steps
- The CHECK constraint on `status` enforces the 8 valid pipeline values at the database level
- The `STRICT` table mode enforces column types

**IMPORTANT:** `PRAGMA foreign_keys` has no effect inside an active transaction. If the migration runner wraps each migration in BEGIN/COMMIT, the PRAGMA calls will be silently ignored. Verify the migration runner at `packages/core/src/db/migrate.ts` -- if it uses transactions, either (a) split the note_references rebuild into a separate migration file that runs outside a transaction, or (b) modify the runner to execute PRAGMA statements outside the transaction wrapper. Add an integration test that applies migration 007 to a DB with existing note_references rows to verify the rebuild succeeds.

**Acceptance criteria:**
- [ ] Migration applies cleanly on fresh database (001-007)
- [ ] `job_descriptions` table exists with correct columns, types, and constraints
- [ ] `note_references` CHECK constraint includes `'job_description'`
- [ ] Indexes `idx_job_descriptions_org` and `idx_job_descriptions_status` exist

**Failure criteria:**
- Migration fails if 001-004 have not been applied (missing `organizations` or `note_references` tables)
- `INSERT INTO job_descriptions` with `status = 'invalid'` must be rejected by CHECK constraint
- `INSERT INTO note_references` with `entity_type = 'invalid'` must be rejected by CHECK constraint

---

### T31.2: Update Types (core + SDK)

#### `packages/core/src/types/index.ts`

**Add `NoteReferenceEntityType` union type and JD types:**

```typescript
/** Valid entity types for note_references. Must match the CHECK constraint in the database. */
export type NoteReferenceEntityType =
  | 'source'
  | 'bullet'
  | 'perspective'
  | 'resume_entry'
  | 'resume'
  | 'skill'
  | 'organization'
  | 'job_description'
```

**Update `NoteReference` to use the union type:**

```typescript
// Before:
export interface NoteReference {
  note_id: string
  entity_type: string
  entity_id: string
}

// After:
export interface NoteReference {
  note_id: string
  entity_type: NoteReferenceEntityType
  entity_id: string
}
```

**Add JD entity types (after the Organization interface):**

```typescript
/** Valid statuses for a JobDescription record. */
export type JobDescriptionStatus =
  | 'interested'
  | 'analyzing'
  | 'applied'
  | 'interviewing'
  | 'offered'
  | 'rejected'
  | 'withdrawn'
  | 'closed'

/** A stored job description for a target opportunity (base row). */
export interface JobDescription {
  id: string
  organization_id: string | null
  title: string
  url: string | null
  raw_text: string
  status: JobDescriptionStatus
  salary_range: string | null
  location: string | null
  notes: string | null
  created_at: string
  updated_at: string
}

/** JobDescription with computed organization_name from JOIN. Used in API responses. */
export interface JobDescriptionWithOrg extends JobDescription {
  organization_name: string | null
}

/** Input for creating a new JobDescription. */
export interface CreateJobDescription {
  title: string
  organization_id?: string
  url?: string
  raw_text: string
  status?: JobDescriptionStatus
  salary_range?: string
  location?: string
  notes?: string
}

/** Input for partially updating a JobDescription. */
export interface UpdateJobDescription {
  title?: string
  organization_id?: string | null
  url?: string | null
  raw_text?: string
  status?: JobDescriptionStatus
  salary_range?: string | null
  location?: string | null
  notes?: string | null
}
```

#### `packages/sdk/src/types.ts`

**Add `NoteReferenceEntityType` union type:**

```typescript
export type NoteReferenceEntityType =
  | 'source'
  | 'bullet'
  | 'perspective'
  | 'resume_entry'
  | 'resume'
  | 'skill'
  | 'organization'
  | 'job_description'
```

**Update `NoteReference` to use the union type:**

```typescript
// Before:
export interface NoteReference {
  entity_type: 'source' | 'bullet' | 'perspective' | 'resume_entry' | 'resume' | 'skill' | 'organization'
  entity_id: string
}

// After:
export interface NoteReference {
  entity_type: NoteReferenceEntityType
  entity_id: string
}
```

**Add JD types (mirroring core):**

```typescript
export type JobDescriptionStatus =
  | 'interested'
  | 'analyzing'
  | 'applied'
  | 'interviewing'
  | 'offered'
  | 'rejected'
  | 'withdrawn'
  | 'closed'

export interface JobDescription {
  id: string
  organization_id: string | null
  title: string
  url: string | null
  raw_text: string
  status: JobDescriptionStatus
  salary_range: string | null
  location: string | null
  notes: string | null
  created_at: string
  updated_at: string
}

export interface JobDescriptionWithOrg extends JobDescription {
  organization_name: string | null
}

export interface CreateJobDescription {
  title: string
  organization_id?: string
  url?: string
  raw_text: string
  status?: JobDescriptionStatus
  salary_range?: string
  location?: string
  notes?: string
}

export interface UpdateJobDescription {
  title?: string
  organization_id?: string | null
  url?: string | null
  raw_text?: string
  status?: JobDescriptionStatus
  salary_range?: string | null
  location?: string | null
  notes?: string | null
}

export interface JobDescriptionFilter {
  status?: JobDescriptionStatus
  organization_id?: string
}
```

**Acceptance criteria:**
- [ ] `NoteReferenceEntityType` defined in both core and SDK types
- [ ] `NoteReference.entity_type` uses the union type (not raw `string`)
- [ ] All 7 JD-related types exist in both packages (JobDescriptionStatus, JobDescription, JobDescriptionWithOrg, CreateJobDescription, UpdateJobDescription, JobDescriptionFilter, NoteReferenceEntityType)
- [ ] `JobDescriptionWithOrg` extends `JobDescription` with `organization_name`

---

### T31.3: Write `JobDescriptionRepository`

**File:** `packages/core/src/db/repositories/job-description-repository.ts`

Follows the same stateless pattern as `organization-repository.ts`. All functions take `Database` as the first parameter. The key difference from organizations is the JOIN to `organizations.name` for the `WithOrg` response type.

```typescript
/**
 * JobDescriptionRepository -- CRUD operations for the job_descriptions table.
 *
 * All functions take a `Database` instance as the first parameter,
 * keeping the repository stateless and testable.
 */

import type { Database } from 'bun:sqlite'
import type {
  JobDescription,
  JobDescriptionWithOrg,
  CreateJobDescription,
  UpdateJobDescription,
  JobDescriptionStatus,
} from '../../types'

// ---------------------------------------------------------------------------
// Filter types
// ---------------------------------------------------------------------------

export interface JobDescriptionFilter {
  status?: JobDescriptionStatus
  organization_id?: string
}

// ---------------------------------------------------------------------------
// Internal: base SELECT with organization JOIN
// ---------------------------------------------------------------------------

const SELECT_WITH_ORG = `
  SELECT jd.*,
         o.name AS organization_name
  FROM job_descriptions jd
  LEFT JOIN organizations o ON o.id = jd.organization_id
`

// ---------------------------------------------------------------------------
// Repository functions
// ---------------------------------------------------------------------------

/** Insert a new job description and return the created row with org name. */
export function create(
  db: Database,
  input: CreateJobDescription,
): JobDescriptionWithOrg {
  const id = crypto.randomUUID()
  db.query(
    `INSERT INTO job_descriptions (id, organization_id, title, url, raw_text, status, salary_range, location, notes)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)`,
  ).run(
    id,
    input.organization_id ?? null,
    input.title,
    input.url ?? null,
    input.raw_text,
    input.status ?? 'interested',
    input.salary_range ?? null,
    input.location ?? null,
    input.notes ?? null,
  )

  return get(db, id)!
}

/** Retrieve a job description by ID with org name, or null if not found. */
export function get(
  db: Database,
  id: string,
): JobDescriptionWithOrg | null {
  return (
    (db
      .query(`${SELECT_WITH_ORG} WHERE jd.id = ?`)
      .get(id) as JobDescriptionWithOrg | null) ?? null
  )
}

/**
 * List job descriptions with optional filters: status, organization_id.
 * Returns data array and total count (before pagination).
 * Results are ordered by created_at DESC (newest first).
 */
export function list(
  db: Database,
  filter?: JobDescriptionFilter,
  offset = 0,
  limit = 50,
): { data: JobDescriptionWithOrg[]; total: number } {
  const conditions: string[] = []
  const params: unknown[] = []

  if (filter?.status !== undefined) {
    conditions.push('jd.status = ?')
    params.push(filter.status)
  }
  if (filter?.organization_id !== undefined) {
    conditions.push('jd.organization_id = ?')
    params.push(filter.organization_id)
  }

  const where =
    conditions.length > 0 ? `WHERE ${conditions.join(' AND ')}` : ''

  const countRow = db
    .query(
      `SELECT COUNT(*) AS total FROM job_descriptions jd ${where}`,
    )
    .get(...params) as { total: number }

  const dataParams = [...params, limit, offset]
  const rows = db
    .query(
      `${SELECT_WITH_ORG} ${where} ORDER BY jd.created_at DESC LIMIT ? OFFSET ?`,
    )
    .all(...dataParams) as JobDescriptionWithOrg[]

  return { data: rows, total: countRow.total }
}

/**
 * Partially update a job description.
 * Only the fields present in `input` are changed. `updated_at` is
 * always refreshed. Returns null if the job description does not exist.
 */
export function update(
  db: Database,
  id: string,
  input: UpdateJobDescription,
): JobDescriptionWithOrg | null {
  const existing = get(db, id)
  if (!existing) return null

  const sets: string[] = []
  const params: unknown[] = []

  if (input.title !== undefined) {
    sets.push('title = ?')
    params.push(input.title)
  }
  if (input.organization_id !== undefined) {
    sets.push('organization_id = ?')
    params.push(input.organization_id)
  }
  if (input.url !== undefined) {
    sets.push('url = ?')
    params.push(input.url)
  }
  if (input.raw_text !== undefined) {
    sets.push('raw_text = ?')
    params.push(input.raw_text)
  }
  if (input.status !== undefined) {
    sets.push('status = ?')
    params.push(input.status)
  }
  if (input.salary_range !== undefined) {
    sets.push('salary_range = ?')
    params.push(input.salary_range)
  }
  if (input.location !== undefined) {
    sets.push('location = ?')
    params.push(input.location)
  }
  if (input.notes !== undefined) {
    sets.push('notes = ?')
    params.push(input.notes)
  }

  sets.push("updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')")
  params.push(id)

  db.query(
    `UPDATE job_descriptions SET ${sets.join(', ')} WHERE id = ?`,
  ).run(...params)

  return get(db, id)
}

/** Delete a job description by ID. Returns true if a row was deleted. */
export function del(db: Database, id: string): boolean {
  const result = db.run('DELETE FROM job_descriptions WHERE id = ?', [id])
  return result.changes > 0
}
```

**Key design decisions:**
- `create` does an INSERT then re-fetches via `get()` to return the JOINed `organization_name`. This is simpler than a `RETURNING *` + separate org lookup.
- `update` follows the same re-fetch pattern to ensure the response always includes the latest `organization_name`.
- `list` orders by `created_at DESC` (newest first) rather than alphabetical, since JDs are typically tracked in recency order.
- The `SELECT_WITH_ORG` constant avoids duplicating the JOIN across queries.

**Acceptance criteria:**
- [ ] `create` returns `JobDescriptionWithOrg` with `organization_name` populated
- [ ] `get` returns `null` for nonexistent ID
- [ ] `list` filters by `status` and `organization_id` independently and combined
- [ ] `list` returns correct `total` count before pagination
- [ ] `update` returns `null` for nonexistent ID
- [ ] `update` only modifies specified fields, always refreshes `updated_at`
- [ ] `del` returns `true` when row deleted, `false` when not found

---

### T31.4: Write `JobDescriptionService`

**File:** `packages/core/src/services/job-description-service.ts`

Follows the same pattern as `OrganizationService`: validates input, delegates to repository, returns `Result<T>`.

```typescript
/**
 * JobDescriptionService -- business logic for job description entities.
 *
 * Validates input before delegating to the JobDescriptionRepository.
 * All methods return Result<T> (never throw).
 */

import type { Database } from 'bun:sqlite'
import type {
  JobDescriptionWithOrg,
  CreateJobDescription,
  UpdateJobDescription,
  Result,
  PaginatedResult,
} from '../types'
import * as JDRepo from '../db/repositories/job-description-repository'
import type { JobDescriptionFilter } from '../db/repositories/job-description-repository'

const VALID_STATUSES = [
  'interested',
  'analyzing',
  'applied',
  'interviewing',
  'offered',
  'rejected',
  'withdrawn',
  'closed',
]

export class JobDescriptionService {
  constructor(private db: Database) {}

  create(input: CreateJobDescription): Result<JobDescriptionWithOrg> {
    if (!input.title || input.title.trim().length === 0) {
      return {
        ok: false,
        error: { code: 'VALIDATION_ERROR', message: 'Title must not be empty' },
      }
    }
    if (!input.raw_text || input.raw_text.trim().length === 0) {
      return {
        ok: false,
        error: {
          code: 'VALIDATION_ERROR',
          message: 'Job description text (raw_text) must not be empty',
        },
      }
    }
    if (
      input.status !== undefined &&
      !VALID_STATUSES.includes(input.status)
    ) {
      return {
        ok: false,
        error: {
          code: 'VALIDATION_ERROR',
          message: `Invalid status: ${input.status}. Must be one of: ${VALID_STATUSES.join(', ')}`,
        },
      }
    }

    const jd = JDRepo.create(this.db, input)
    return { ok: true, data: jd }
  }

  get(id: string): Result<JobDescriptionWithOrg> {
    const jd = JDRepo.get(this.db, id)
    if (!jd) {
      return {
        ok: false,
        error: {
          code: 'NOT_FOUND',
          message: `Job description ${id} not found`,
        },
      }
    }
    return { ok: true, data: jd }
  }

  list(
    filter?: JobDescriptionFilter,
    offset?: number,
    limit?: number,
  ): PaginatedResult<JobDescriptionWithOrg> {
    if (
      filter?.status !== undefined &&
      !VALID_STATUSES.includes(filter.status)
    ) {
      return {
        ok: false,
        error: {
          code: 'VALIDATION_ERROR',
          message: `Invalid status filter: ${filter.status}. Must be one of: ${VALID_STATUSES.join(', ')}`,
        },
      }
    }

    const result = JDRepo.list(this.db, filter, offset, limit)
    return {
      ok: true,
      data: result.data,
      pagination: {
        total: result.total,
        offset: offset ?? 0,
        limit: limit ?? 50,
      },
    }
  }

  update(
    id: string,
    input: UpdateJobDescription,
  ): Result<JobDescriptionWithOrg> {
    if (input.title !== undefined && input.title.trim().length === 0) {
      return {
        ok: false,
        error: { code: 'VALIDATION_ERROR', message: 'Title must not be empty' },
      }
    }
    if (input.raw_text !== undefined && input.raw_text.trim().length === 0) {
      return {
        ok: false,
        error: {
          code: 'VALIDATION_ERROR',
          message: 'Job description text (raw_text) must not be empty',
        },
      }
    }
    if (
      input.status !== undefined &&
      !VALID_STATUSES.includes(input.status)
    ) {
      return {
        ok: false,
        error: {
          code: 'VALIDATION_ERROR',
          message: `Invalid status: ${input.status}. Must be one of: ${VALID_STATUSES.join(', ')}`,
        },
      }
    }

    const jd = JDRepo.update(this.db, id, input)
    if (!jd) {
      return {
        ok: false,
        error: {
          code: 'NOT_FOUND',
          message: `Job description ${id} not found`,
        },
      }
    }
    return { ok: true, data: jd }
  }

  delete(id: string): Result<void> {
    const deleted = JDRepo.del(this.db, id)
    if (!deleted) {
      return {
        ok: false,
        error: {
          code: 'NOT_FOUND',
          message: `Job description ${id} not found`,
        },
      }
    }
    return { ok: true, data: undefined }
  }
}
```

**Validation rules:**
- `title` must not be empty (create and update)
- `raw_text` must not be empty (create and update)
- `status` must be one of the 8 valid values (create, update, and list filter)
- `organization_id` is NOT validated against the organizations table at the service level -- the FK constraint in the database handles this, and the error will surface as an INTERNAL_ERROR if the org ID is invalid

Note: Passing a non-existent `organization_id` to create/update will trigger a SQLite FK constraint error, which surfaces as HTTP 500 (INTERNAL_ERROR). This is a known limitation -- service-level FK validation is intentionally omitted for simplicity.

**Acceptance criteria:**
- [ ] `create` rejects empty `title` with VALIDATION_ERROR
- [ ] `create` rejects empty `raw_text` with VALIDATION_ERROR
- [ ] `create` rejects invalid `status` with VALIDATION_ERROR
- [ ] `get` returns NOT_FOUND for nonexistent ID
- [ ] `list` validates `status` filter value
- [ ] `update` rejects empty `title` and `raw_text`
- [ ] `delete` returns NOT_FOUND for nonexistent ID

---

### T31.5: Register Service

#### `packages/core/src/services/index.ts`

**Add import:**

```typescript
import { JobDescriptionService } from './job-description-service'
```

**Update `Services` interface:**

```typescript
export interface Services {
  // ... existing services ...
  jobDescriptions: JobDescriptionService
}
```

**Update `createServices`:**

```typescript
export function createServices(db: Database): Services {
  const derivingBullets = new Set<string>()

  return {
    // ... existing services ...
    jobDescriptions: new JobDescriptionService(db),
  }
}
```

**Add re-export:**

```typescript
export { JobDescriptionService } from './job-description-service'
```

Note: The existing `Services` interface does not include a `SkillService` -- skills are handled via direct repository calls in routes. This phase follows the same pattern as the existing codebase and does not introduce a SkillService.

**Acceptance criteria:**
- [ ] `services.jobDescriptions` is available in route handlers
- [ ] `JobDescriptionService` is re-exported from the barrel

---

### T31.6: Write API Routes

**File:** `packages/core/src/routes/job-descriptions.ts`

Follows the identical pattern to `organizations.ts`: thin HTTP layer over the service.

```typescript
/**
 * Job description routes -- thin HTTP layer over JobDescriptionService.
 */

import { Hono } from 'hono'
import type { Services } from '../services'
import { mapStatusCode } from './server'

export function jobDescriptionRoutes(services: Services) {
  const app = new Hono()

  app.post('/job-descriptions', async (c) => {
    const body = await c.req.json()
    const result = services.jobDescriptions.create(body)
    if (!result.ok)
      return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data }, 201)
  })

  app.get('/job-descriptions', (c) => {
    const offset = Math.max(
      0,
      parseInt(c.req.query('offset') ?? '0', 10) || 0,
    )
    const limit = Math.min(
      200,
      Math.max(1, parseInt(c.req.query('limit') ?? '50', 10) || 50),
    )
    const filter: Record<string, string> = {}
    if (c.req.query('status')) filter.status = c.req.query('status')!
    if (c.req.query('organization_id'))
      filter.organization_id = c.req.query('organization_id')!

    const result = services.jobDescriptions.list(filter, offset, limit)
    if (!result.ok)
      return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data, pagination: result.pagination })
  })

  app.get('/job-descriptions/:id', (c) => {
    const result = services.jobDescriptions.get(c.req.param('id'))
    if (!result.ok)
      return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.patch('/job-descriptions/:id', async (c) => {
    const body = await c.req.json()
    const result = services.jobDescriptions.update(c.req.param('id'), body)
    if (!result.ok)
      return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.delete('/job-descriptions/:id', (c) => {
    const result = services.jobDescriptions.delete(c.req.param('id'))
    if (!result.ok)
      return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.body(null, 204)
  })

  return app
}
```

#### `packages/core/src/routes/server.ts`

**Add import:**

```typescript
import { jobDescriptionRoutes } from './job-descriptions'
```

**Mount route in `createApp`:**

```typescript
app.route('/', jobDescriptionRoutes(services))
```

**Acceptance criteria:**
- [ ] `POST /api/job-descriptions` returns 201 with `{ data: JobDescriptionWithOrg }`
- [ ] `GET /api/job-descriptions` returns `{ data: [...], pagination: {...} }`
- [ ] `GET /api/job-descriptions/:id` returns `{ data: JobDescriptionWithOrg }`
- [ ] `PATCH /api/job-descriptions/:id` returns `{ data: JobDescriptionWithOrg }`
- [ ] `DELETE /api/job-descriptions/:id` returns 204
- [ ] Invalid input returns 400 with `{ error: { code, message } }`
- [ ] Nonexistent ID returns 404

---

### T31.7: Write SDK Resource Class

**File:** `packages/sdk/src/resources/job-descriptions.ts`

```typescript
import type {
  CreateJobDescription,
  JobDescriptionFilter,
  JobDescriptionWithOrg,
  PaginatedResult,
  PaginationParams,
  RequestFn,
  RequestListFn,
  Result,
  UpdateJobDescription,
} from '../types'

function toParams(
  filter?: object,
): Record<string, string> | undefined {
  if (!filter) return undefined
  const out: Record<string, string> = {}
  for (const [k, v] of Object.entries(filter)) {
    if (v !== undefined && v !== null) out[k] = String(v)
  }
  return Object.keys(out).length > 0 ? out : undefined
}

export class JobDescriptionsResource {
  constructor(
    private request: RequestFn,
    private requestList: RequestListFn,
  ) {}

  create(
    input: CreateJobDescription,
  ): Promise<Result<JobDescriptionWithOrg>> {
    return this.request<JobDescriptionWithOrg>(
      'POST',
      '/api/job-descriptions',
      input,
    )
  }

  list(
    filter?: JobDescriptionFilter & PaginationParams,
  ): Promise<PaginatedResult<JobDescriptionWithOrg>> {
    return this.requestList<JobDescriptionWithOrg>(
      'GET',
      '/api/job-descriptions',
      toParams(filter),
    )
  }

  get(id: string): Promise<Result<JobDescriptionWithOrg>> {
    return this.request<JobDescriptionWithOrg>(
      'GET',
      `/api/job-descriptions/${id}`,
    )
  }

  update(
    id: string,
    input: UpdateJobDescription,
  ): Promise<Result<JobDescriptionWithOrg>> {
    return this.request<JobDescriptionWithOrg>(
      'PATCH',
      `/api/job-descriptions/${id}`,
      input,
    )
  }

  delete(id: string): Promise<Result<void>> {
    return this.request<void>('DELETE', `/api/job-descriptions/${id}`)
  }
}
```

**Note:** The `RequestFn` and `RequestListFn` types must be exported from `packages/sdk/src/types.ts`. If they are not currently exported, add:

```typescript
/** Internal type for single-entity requests (used by resource classes). */
export type RequestFn = <T>(
  method: string,
  path: string,
  body?: unknown,
) => Promise<Result<T>>

/** Internal type for paginated list requests (used by resource classes). */
export type RequestListFn = <T>(
  method: string,
  path: string,
  params?: Record<string, string>,
) => Promise<PaginatedResult<T>>
```

#### `packages/sdk/src/client.ts`

**Add import:**

```typescript
import { JobDescriptionsResource } from './resources/job-descriptions'
```

**Add property declaration:**

```typescript
/** Job description CRUD. */
public jobDescriptions: JobDescriptionsResource
```

**Add initialization in constructor:**

```typescript
this.jobDescriptions = new JobDescriptionsResource(req, reqList)
```

#### `packages/sdk/src/index.ts`

**Add type exports:**

```typescript
// Job description types
export type {
  JobDescription,
  JobDescriptionWithOrg,
  JobDescriptionStatus,
  JobDescriptionFilter,
  CreateJobDescription,
  UpdateJobDescription,
} from './types'

// Note reference entity type (shared)
export type {
  NoteReferenceEntityType,
} from './types'
```

**Add resource export:**

```typescript
export { JobDescriptionsResource } from './resources/job-descriptions'
```

**Acceptance criteria:**
- [ ] `client.jobDescriptions.create(input)` resolves to `Result<JobDescriptionWithOrg>`
- [ ] `client.jobDescriptions.list({ status: 'applied' })` resolves to `PaginatedResult<JobDescriptionWithOrg>`
- [ ] `client.jobDescriptions.get(id)` resolves to `Result<JobDescriptionWithOrg>`
- [ ] `client.jobDescriptions.update(id, input)` resolves to `Result<JobDescriptionWithOrg>`
- [ ] `client.jobDescriptions.delete(id)` resolves to `Result<void>`
- [ ] All JD types exported from SDK barrel

---

### T31.8: Update Test Helpers

**File:** `packages/core/src/db/__tests__/helpers.ts`

**Add `seedJobDescription` helper:**

```typescript
/** Seed a test job description and return its ID */
export function seedJobDescription(
  db: Database,
  opts: {
    organizationId?: string | null
    title?: string
    url?: string | null
    rawText?: string
    status?: string
    salaryRange?: string | null
    location?: string | null
    notes?: string | null
  } = {},
): string {
  const id = testUuid()
  db.run(
    `INSERT INTO job_descriptions (id, organization_id, title, url, raw_text, status, salary_range, location, notes)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)`,
    [
      id,
      opts.organizationId ?? null,
      opts.title ?? 'Senior Security Engineer',
      opts.url ?? null,
      opts.rawText ?? 'We are looking for a senior security engineer to join our team...',
      opts.status ?? 'interested',
      opts.salaryRange ?? null,
      opts.location ?? null,
      opts.notes ?? null,
    ],
  )
  return id
}
```

**Acceptance criteria:**
- [ ] `seedJobDescription(db)` creates a JD with default values
- [ ] `seedJobDescription(db, { organizationId: orgId })` links to an org
- [ ] All optional fields can be overridden

---

### T31.9: Write Repository Tests

**File:** `packages/core/src/db/repositories/__tests__/job-description-repository.test.ts`

```typescript
/**
 * Tests for JobDescriptionRepository -- CRUD operations for the job_descriptions table.
 */

import { describe, test, expect, beforeEach, afterEach } from 'bun:test'
import { Database } from 'bun:sqlite'
import { createTestDb, seedOrganization } from '../../__tests__/helpers'
import * as JDRepo from '../job-description-repository'

// ---------------------------------------------------------------------------
// Shared setup
// ---------------------------------------------------------------------------

let db: Database

beforeEach(() => {
  db = createTestDb()
})

afterEach(() => {
  db.close()
})

// ===========================================================================
// JobDescriptionRepository
// ===========================================================================

describe('JobDescriptionRepository', () => {
  // ── create ──────────────────────────────────────────────────────────

  test('create returns a job description with generated id and default status', () => {
    const jd = JDRepo.create(db, {
      title: 'Senior Security Engineer',
      raw_text: 'We are looking for a senior security engineer...',
    })

    expect(jd.id).toHaveLength(36)
    expect(jd.title).toBe('Senior Security Engineer')
    expect(jd.raw_text).toBe('We are looking for a senior security engineer...')
    expect(jd.status).toBe('interested')
    expect(jd.organization_id).toBeNull()
    expect(jd.organization_name).toBeNull()
    expect(jd.url).toBeNull()
    expect(jd.salary_range).toBeNull()
    expect(jd.location).toBeNull()
    expect(jd.notes).toBeNull()
    expect(jd.created_at).toBeTruthy()
    expect(jd.updated_at).toBeTruthy()
  })

  test('create with all optional fields', () => {
    const orgId = seedOrganization(db, { name: 'Cloudflare' })

    const jd = JDRepo.create(db, {
      title: 'Staff Engineer',
      organization_id: orgId,
      url: 'https://boards.greenhouse.io/cloudflare/123',
      raw_text: 'Full JD text here...',
      status: 'applied',
      salary_range: '$180k-$220k',
      location: 'Remote',
      notes: 'Referred by John',
    })

    expect(jd.organization_id).toBe(orgId)
    expect(jd.organization_name).toBe('Cloudflare')
    expect(jd.url).toBe('https://boards.greenhouse.io/cloudflare/123')
    expect(jd.status).toBe('applied')
    expect(jd.salary_range).toBe('$180k-$220k')
    expect(jd.location).toBe('Remote')
    expect(jd.notes).toBe('Referred by John')
  })

  test('create with organization includes organization_name in response', () => {
    const orgId = seedOrganization(db, { name: 'Anthropic' })
    const jd = JDRepo.create(db, {
      title: 'Security Engineer',
      organization_id: orgId,
      raw_text: 'Join us...',
    })

    expect(jd.organization_name).toBe('Anthropic')
  })

  // ── get ─────────────────────────────────────────────────────────────

  test('get returns the job description by id with org name', () => {
    const orgId = seedOrganization(db, { name: 'Google' })
    const jd = JDRepo.create(db, {
      title: 'SRE',
      organization_id: orgId,
      raw_text: 'Site reliability...',
    })
    const fetched = JDRepo.get(db, jd.id)

    expect(fetched).not.toBeNull()
    expect(fetched!.id).toBe(jd.id)
    expect(fetched!.title).toBe('SRE')
    expect(fetched!.organization_name).toBe('Google')
  })

  test('get returns null for nonexistent id', () => {
    expect(JDRepo.get(db, crypto.randomUUID())).toBeNull()
  })

  // ── list ────────────────────────────────────────────────────────────

  test('list returns all job descriptions ordered by created_at DESC', () => {
    JDRepo.create(db, { title: 'First', raw_text: 'text' })
    JDRepo.create(db, { title: 'Second', raw_text: 'text' })
    JDRepo.create(db, { title: 'Third', raw_text: 'text' })

    const result = JDRepo.list(db)
    expect(result.total).toBe(3)
    expect(result.data).toHaveLength(3)
    // Newest first (Third was created last)
    expect(result.data[0].title).toBe('Third')
  })

  test('list filters by status', () => {
    JDRepo.create(db, { title: 'A', raw_text: 'text', status: 'interested' })
    JDRepo.create(db, { title: 'B', raw_text: 'text', status: 'applied' })
    JDRepo.create(db, { title: 'C', raw_text: 'text', status: 'applied' })

    const result = JDRepo.list(db, { status: 'applied' })
    expect(result.total).toBe(2)
    expect(result.data.every((jd) => jd.status === 'applied')).toBe(true)
  })

  test('list filters by organization_id', () => {
    const org1 = seedOrganization(db, { name: 'Org1' })
    const org2 = seedOrganization(db, { name: 'Org2' })
    JDRepo.create(db, {
      title: 'JD1',
      organization_id: org1,
      raw_text: 'text',
    })
    JDRepo.create(db, {
      title: 'JD2',
      organization_id: org2,
      raw_text: 'text',
    })
    JDRepo.create(db, {
      title: 'JD3',
      organization_id: org1,
      raw_text: 'text',
    })

    const result = JDRepo.list(db, { organization_id: org1 })
    expect(result.total).toBe(2)
    expect(result.data.every((jd) => jd.organization_id === org1)).toBe(true)
  })

  test('list filters by both status AND organization_id', () => {
    const orgId = seedOrganization(db, { name: 'Target' })
    JDRepo.create(db, {
      title: 'Match',
      organization_id: orgId,
      raw_text: 'text',
      status: 'applied',
    })
    JDRepo.create(db, {
      title: 'WrongStatus',
      organization_id: orgId,
      raw_text: 'text',
      status: 'interested',
    })
    JDRepo.create(db, {
      title: 'WrongOrg',
      raw_text: 'text',
      status: 'applied',
    })

    const result = JDRepo.list(db, {
      status: 'applied',
      organization_id: orgId,
    })
    expect(result.total).toBe(1)
    expect(result.data[0].title).toBe('Match')
  })

  test('list includes organization_name per item', () => {
    const orgId = seedOrganization(db, { name: 'Anthropic' })
    JDRepo.create(db, {
      title: 'With Org',
      organization_id: orgId,
      raw_text: 'text',
    })
    JDRepo.create(db, { title: 'No Org', raw_text: 'text' })

    const result = JDRepo.list(db)
    const withOrg = result.data.find((jd) => jd.title === 'With Org')
    const noOrg = result.data.find((jd) => jd.title === 'No Org')

    expect(withOrg!.organization_name).toBe('Anthropic')
    expect(noOrg!.organization_name).toBeNull()
  })

  test('list supports pagination', () => {
    JDRepo.create(db, { title: 'A', raw_text: 'text' })
    JDRepo.create(db, { title: 'B', raw_text: 'text' })
    JDRepo.create(db, { title: 'C', raw_text: 'text' })

    const page1 = JDRepo.list(db, undefined, 0, 2)
    expect(page1.data).toHaveLength(2)
    expect(page1.total).toBe(3)

    const page2 = JDRepo.list(db, undefined, 2, 2)
    expect(page2.data).toHaveLength(1)
    expect(page2.total).toBe(3)
  })

  // ── update ──────────────────────────────────────────────────────────

  test('update modifies specified fields and refreshes updated_at', () => {
    const jd = JDRepo.create(db, {
      title: 'OldTitle',
      raw_text: 'old text',
    })
    const updated = JDRepo.update(db, jd.id, {
      title: 'NewTitle',
      status: 'applied',
    })

    expect(updated).not.toBeNull()
    expect(updated!.title).toBe('NewTitle')
    expect(updated!.status).toBe('applied')
    expect(updated!.raw_text).toBe('old text') // unchanged
  })

  test('update can set organization_id to null', () => {
    const orgId = seedOrganization(db, { name: 'Corp' })
    const jd = JDRepo.create(db, {
      title: 'JD',
      organization_id: orgId,
      raw_text: 'text',
    })
    expect(jd.organization_id).toBe(orgId)

    const updated = JDRepo.update(db, jd.id, { organization_id: null })
    expect(updated!.organization_id).toBeNull()
    expect(updated!.organization_name).toBeNull()
  })

  test('update can change organization_id and organization_name updates', () => {
    const org1 = seedOrganization(db, { name: 'OrgOne' })
    const org2 = seedOrganization(db, { name: 'OrgTwo' })
    const jd = JDRepo.create(db, {
      title: 'JD',
      organization_id: org1,
      raw_text: 'text',
    })
    expect(jd.organization_name).toBe('OrgOne')

    const updated = JDRepo.update(db, jd.id, { organization_id: org2 })
    expect(updated!.organization_name).toBe('OrgTwo')
  })

  test('update returns null for nonexistent id', () => {
    expect(
      JDRepo.update(db, crypto.randomUUID(), { title: 'X' }),
    ).toBeNull()
  })

  // ── del ─────────────────────────────────────────────────────────────

  test('del removes the job description', () => {
    const jd = JDRepo.create(db, { title: 'Temp', raw_text: 'text' })
    const deleted = JDRepo.del(db, jd.id)

    expect(deleted).toBe(true)
    expect(JDRepo.get(db, jd.id)).toBeNull()
  })

  test('del returns false for nonexistent id', () => {
    expect(JDRepo.del(db, crypto.randomUUID())).toBe(false)
  })

  // ── ON DELETE SET NULL ──────────────────────────────────────────────

  test('deleting organization sets organization_id to null on linked JDs', () => {
    const orgId = seedOrganization(db, { name: 'Doomed Corp' })
    const jd = JDRepo.create(db, {
      title: 'Linked JD',
      organization_id: orgId,
      raw_text: 'text',
    })
    expect(jd.organization_id).toBe(orgId)

    // Delete the organization
    db.run('DELETE FROM organizations WHERE id = ?', [orgId])

    // JD should still exist but with organization_id = null
    const fetched = JDRepo.get(db, jd.id)
    expect(fetched).not.toBeNull()
    expect(fetched!.organization_id).toBeNull()
    expect(fetched!.organization_name).toBeNull()
  })

  // ── CHECK constraint ───────────────────────────────────────────────

  test('CHECK constraint rejects invalid status values', () => {
    expect(() => {
      db.run(
        `INSERT INTO job_descriptions (id, title, raw_text, status)
         VALUES (?, ?, ?, ?)`,
        [crypto.randomUUID(), 'Bad', 'text', 'invalid_status'],
      )
    }).toThrow()
  })

  // ── note_references entity_type ────────────────────────────────────

  test('note_references accepts job_description entity_type', () => {
    const jdId = JDRepo.create(db, {
      title: 'JD',
      raw_text: 'text',
    }).id

    // Create a note first
    const noteId = crypto.randomUUID()
    db.run(
      `INSERT INTO user_notes (id, content) VALUES (?, ?)`,
      [noteId, 'Note about this JD'],
    )

    // Link note to JD
    expect(() => {
      db.run(
        `INSERT INTO note_references (note_id, entity_type, entity_id)
         VALUES (?, 'job_description', ?)`,
        [noteId, jdId],
      )
    }).not.toThrow()

    // Verify it was inserted
    const ref = db
      .query(
        `SELECT * FROM note_references WHERE note_id = ? AND entity_type = 'job_description'`,
      )
      .get(noteId)
    expect(ref).not.toBeNull()
  })

  test('note_references still rejects invalid entity_type', () => {
    const noteId = crypto.randomUUID()
    db.run(
      `INSERT INTO user_notes (id, content) VALUES (?, ?)`,
      [noteId, 'Test'],
    )

    expect(() => {
      db.run(
        `INSERT INTO note_references (note_id, entity_type, entity_id)
         VALUES (?, 'invalid_type', ?)`,
        [noteId, 'some-id'],
      )
    }).toThrow()
  })
})
```

**Test count:** 20 tests covering CRUD, filtering, pagination, ON DELETE SET NULL, CHECK constraints, and note_references.

Note: No separate migration test file is needed. The `createTestDb()` helper runs all migrations (001-007), so any failure in migration 007 will surface as a test setup error across this entire test suite. The `note_references` and `CHECK constraint` tests above explicitly verify the migration's DDL effects.

---

### T31.10: Write Service Tests

**File:** `packages/core/src/services/__tests__/job-description-service.test.ts`

```typescript
/**
 * Tests for JobDescriptionService -- business logic validation.
 */

import { describe, test, expect, beforeEach, afterEach } from 'bun:test'
import { Database } from 'bun:sqlite'
import { createTestDb, seedOrganization } from '../../db/__tests__/helpers'
import { JobDescriptionService } from '../job-description-service'

let db: Database
let service: JobDescriptionService

beforeEach(() => {
  db = createTestDb()
  service = new JobDescriptionService(db)
})

afterEach(() => {
  db.close()
})

describe('JobDescriptionService', () => {
  // ── create validation ───────────────────────────────────────────────

  test('create rejects empty title', () => {
    const result = service.create({ title: '', raw_text: 'text' })
    expect(result.ok).toBe(false)
    if (!result.ok) {
      expect(result.error.code).toBe('VALIDATION_ERROR')
      expect(result.error.message).toContain('Title')
    }
  })

  test('create rejects whitespace-only title', () => {
    const result = service.create({ title: '   ', raw_text: 'text' })
    expect(result.ok).toBe(false)
  })

  test('create rejects empty raw_text', () => {
    const result = service.create({ title: 'Valid', raw_text: '' })
    expect(result.ok).toBe(false)
    if (!result.ok) {
      expect(result.error.code).toBe('VALIDATION_ERROR')
      expect(result.error.message).toContain('raw_text')
    }
  })

  test('create rejects invalid status', () => {
    const result = service.create({
      title: 'Valid',
      raw_text: 'text',
      status: 'bogus' as any,
    })
    expect(result.ok).toBe(false)
    if (!result.ok) {
      expect(result.error.code).toBe('VALIDATION_ERROR')
      expect(result.error.message).toContain('status')
    }
  })

  test('create succeeds with valid input', () => {
    const result = service.create({
      title: 'Security Engineer',
      raw_text: 'We need someone who can...',
      status: 'interested',
    })
    expect(result.ok).toBe(true)
    if (result.ok) {
      expect(result.data.title).toBe('Security Engineer')
      expect(result.data.status).toBe('interested')
    }
  })

  test('create succeeds with organization', () => {
    const orgId = seedOrganization(db, { name: 'Anthropic' })
    const result = service.create({
      title: 'ML Engineer',
      raw_text: 'text',
      organization_id: orgId,
    })
    expect(result.ok).toBe(true)
    if (result.ok) {
      expect(result.data.organization_name).toBe('Anthropic')
    }
  })

  // ── get ─────────────────────────────────────────────────────────────

  test('get returns NOT_FOUND for nonexistent id', () => {
    const result = service.get(crypto.randomUUID())
    expect(result.ok).toBe(false)
    if (!result.ok) {
      expect(result.error.code).toBe('NOT_FOUND')
    }
  })

  test('get returns the job description', () => {
    const created = service.create({
      title: 'JD',
      raw_text: 'text',
    })
    expect(created.ok).toBe(true)
    if (!created.ok) return

    const result = service.get(created.data.id)
    expect(result.ok).toBe(true)
    if (result.ok) {
      expect(result.data.title).toBe('JD')
    }
  })

  // ── list ────────────────────────────────────────────────────────────

  test('list validates invalid status filter', () => {
    const result = service.list({ status: 'invalid' })
    expect(result.ok).toBe(false)
    if (!result.ok) {
      expect(result.error.code).toBe('VALIDATION_ERROR')
    }
  })

  test('list returns paginated results', () => {
    service.create({ title: 'A', raw_text: 'text' })
    service.create({ title: 'B', raw_text: 'text' })

    const result = service.list(undefined, 0, 50)
    expect(result.ok).toBe(true)
    if (result.ok) {
      expect(result.data).toHaveLength(2)
      expect(result.pagination.total).toBe(2)
    }
  })

  // ── update validation ───────────────────────────────────────────────

  test('update rejects empty title', () => {
    const created = service.create({ title: 'JD', raw_text: 'text' })
    if (!created.ok) return

    const result = service.update(created.data.id, { title: '' })
    expect(result.ok).toBe(false)
    if (!result.ok) {
      expect(result.error.code).toBe('VALIDATION_ERROR')
    }
  })

  test('update rejects empty raw_text', () => {
    const created = service.create({ title: 'JD', raw_text: 'text' })
    if (!created.ok) return

    const result = service.update(created.data.id, { raw_text: '' })
    expect(result.ok).toBe(false)
    if (!result.ok) {
      expect(result.error.code).toBe('VALIDATION_ERROR')
    }
  })

  test('update rejects invalid status', () => {
    const created = service.create({ title: 'JD', raw_text: 'text' })
    if (!created.ok) return

    const result = service.update(created.data.id, {
      status: 'bogus' as any,
    })
    expect(result.ok).toBe(false)
  })

  test('update returns NOT_FOUND for nonexistent id', () => {
    const result = service.update(crypto.randomUUID(), { title: 'X' })
    expect(result.ok).toBe(false)
    if (!result.ok) {
      expect(result.error.code).toBe('NOT_FOUND')
    }
  })

  test('update succeeds with valid input', () => {
    const created = service.create({ title: 'JD', raw_text: 'text' })
    if (!created.ok) return

    const result = service.update(created.data.id, {
      title: 'Updated',
      status: 'applied',
    })
    expect(result.ok).toBe(true)
    if (result.ok) {
      expect(result.data.title).toBe('Updated')
      expect(result.data.status).toBe('applied')
    }
  })

  // ── delete ──────────────────────────────────────────────────────────

  test('delete returns NOT_FOUND for nonexistent id', () => {
    const result = service.delete(crypto.randomUUID())
    expect(result.ok).toBe(false)
    if (!result.ok) {
      expect(result.error.code).toBe('NOT_FOUND')
    }
  })

  test('delete succeeds', () => {
    const created = service.create({ title: 'JD', raw_text: 'text' })
    if (!created.ok) return

    const result = service.delete(created.data.id)
    expect(result.ok).toBe(true)

    const fetched = service.get(created.data.id)
    expect(fetched.ok).toBe(false)
  })
})
```

**Test count:** 17 tests covering validation, CRUD, and error paths.

---

### T31.11: Write Route Tests

**File:** `packages/core/src/routes/__tests__/job-descriptions.test.ts`

```typescript
/**
 * Tests for job description API routes.
 */

import { describe, test, expect, beforeEach, afterEach } from 'bun:test'
import { createTestApp, apiRequest, type TestContext } from './helpers'
import { seedOrganization, seedJobDescription } from '../../db/__tests__/helpers'

let ctx: TestContext

beforeEach(() => {
  ctx = createTestApp()
})

afterEach(() => {
  ctx.db.close()
})

describe('Job Description Routes', () => {
  // ── POST /job-descriptions ──────────────────────────────────────────

  test('POST creates a job description and returns 201', async () => {
    const res = await apiRequest(ctx.app, 'POST', '/job-descriptions', {
      title: 'Security Engineer',
      raw_text: 'Full job description text...',
    })
    expect(res.status).toBe(201)
    const body = await res.json()
    expect(body.data.title).toBe('Security Engineer')
    expect(body.data.status).toBe('interested')
    expect(body.data.id).toHaveLength(36)
  })

  test('POST with organization includes organization_name', async () => {
    const orgId = seedOrganization(ctx.db, { name: 'Anthropic' })
    const res = await apiRequest(ctx.app, 'POST', '/job-descriptions', {
      title: 'ML Engineer',
      raw_text: 'text',
      organization_id: orgId,
    })
    expect(res.status).toBe(201)
    const body = await res.json()
    expect(body.data.organization_name).toBe('Anthropic')
  })

  test('POST rejects empty title with 400', async () => {
    const res = await apiRequest(ctx.app, 'POST', '/job-descriptions', {
      title: '',
      raw_text: 'text',
    })
    expect(res.status).toBe(400)
    const body = await res.json()
    expect(body.error.code).toBe('VALIDATION_ERROR')
  })

  test('POST rejects empty raw_text with 400', async () => {
    const res = await apiRequest(ctx.app, 'POST', '/job-descriptions', {
      title: 'Valid',
      raw_text: '',
    })
    expect(res.status).toBe(400)
  })

  // ── GET /job-descriptions ───────────────────────────────────────────

  test('GET list returns paginated envelope', async () => {
    seedJobDescription(ctx.db, { title: 'JD1' })
    seedJobDescription(ctx.db, { title: 'JD2' })

    const res = await apiRequest(ctx.app, 'GET', '/job-descriptions')
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data).toBeArray()
    expect(body.data).toHaveLength(2)
    expect(body.pagination.total).toBe(2)
    expect(body.pagination.offset).toBe(0)
    expect(body.pagination.limit).toBe(50)
  })

  test('GET list filters by status', async () => {
    seedJobDescription(ctx.db, { title: 'A', status: 'applied' })
    seedJobDescription(ctx.db, { title: 'B', status: 'interested' })

    const res = await apiRequest(
      ctx.app,
      'GET',
      '/job-descriptions?status=applied',
    )
    const body = await res.json()
    expect(body.data).toHaveLength(1)
    expect(body.data[0].title).toBe('A')
  })

  test('GET list filters by organization_id', async () => {
    const orgId = seedOrganization(ctx.db, { name: 'FilterOrg' })
    seedJobDescription(ctx.db, {
      title: 'Match',
      organizationId: orgId,
    })
    seedJobDescription(ctx.db, { title: 'NoMatch' })

    const res = await apiRequest(
      ctx.app,
      'GET',
      `/job-descriptions?organization_id=${orgId}`,
    )
    const body = await res.json()
    expect(body.data).toHaveLength(1)
    expect(body.data[0].title).toBe('Match')
  })

  test('GET list includes organization_name per item', async () => {
    const orgId = seedOrganization(ctx.db, { name: 'OrgName' })
    seedJobDescription(ctx.db, {
      title: 'With Org',
      organizationId: orgId,
    })
    seedJobDescription(ctx.db, { title: 'No Org' })

    const res = await apiRequest(ctx.app, 'GET', '/job-descriptions')
    const body = await res.json()
    const withOrg = body.data.find(
      (jd: any) => jd.title === 'With Org',
    )
    const noOrg = body.data.find(
      (jd: any) => jd.title === 'No Org',
    )
    expect(withOrg.organization_name).toBe('OrgName')
    expect(noOrg.organization_name).toBeNull()
  })

  // ── GET /job-descriptions/:id ───────────────────────────────────────

  test('GET single returns job description with org name', async () => {
    const orgId = seedOrganization(ctx.db, { name: 'TestCorp' })
    const jdId = seedJobDescription(ctx.db, {
      title: 'Target JD',
      organizationId: orgId,
    })

    const res = await apiRequest(
      ctx.app,
      'GET',
      `/job-descriptions/${jdId}`,
    )
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.title).toBe('Target JD')
    expect(body.data.organization_name).toBe('TestCorp')
  })

  test('GET single returns 404 for nonexistent id', async () => {
    const res = await apiRequest(
      ctx.app,
      'GET',
      `/job-descriptions/${crypto.randomUUID()}`,
    )
    expect(res.status).toBe(404)
  })

  // ── PATCH /job-descriptions/:id ─────────────────────────────────────

  test('PATCH updates fields and returns updated JD', async () => {
    const jdId = seedJobDescription(ctx.db, { title: 'Old' })

    const res = await apiRequest(
      ctx.app,
      'PATCH',
      `/job-descriptions/${jdId}`,
      { title: 'New', status: 'applied' },
    )
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.title).toBe('New')
    expect(body.data.status).toBe('applied')
  })

  test('PATCH returns 404 for nonexistent id', async () => {
    const res = await apiRequest(
      ctx.app,
      'PATCH',
      `/job-descriptions/${crypto.randomUUID()}`,
      { title: 'X' },
    )
    expect(res.status).toBe(404)
  })

  test('PATCH with organization_id: null clears the organization', async () => {
    const orgId = seedOrganization(ctx.db, { name: 'ClearMe' })
    const jdId = seedJobDescription(ctx.db, {
      title: 'Has Org',
      organizationId: orgId,
    })

    const res = await apiRequest(
      ctx.app,
      'PATCH',
      `/job-descriptions/${jdId}`,
      { organization_id: null },
    )
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.organization_id).toBeNull()
  })

  test('PATCH rejects invalid status with 400', async () => {
    const jdId = seedJobDescription(ctx.db)

    const res = await apiRequest(
      ctx.app,
      'PATCH',
      `/job-descriptions/${jdId}`,
      { status: 'invalid' },
    )
    expect(res.status).toBe(400)
  })

  // ── DELETE /job-descriptions/:id ────────────────────────────────────

  test('DELETE returns 204', async () => {
    const jdId = seedJobDescription(ctx.db)

    const res = await apiRequest(
      ctx.app,
      'DELETE',
      `/job-descriptions/${jdId}`,
    )
    expect(res.status).toBe(204)

    // Verify deleted
    const get = await apiRequest(
      ctx.app,
      'GET',
      `/job-descriptions/${jdId}`,
    )
    expect(get.status).toBe(404)
  })

  test('DELETE returns 404 for nonexistent id', async () => {
    const res = await apiRequest(
      ctx.app,
      'DELETE',
      `/job-descriptions/${crypto.randomUUID()}`,
    )
    expect(res.status).toBe(404)
  })
})
```

**Test count:** 17 tests covering all 5 route endpoints and edge cases.

---

### T31.12: Update Contract Tests

**File:** `packages/core/src/routes/__tests__/contracts.test.ts`

**Add JD contract tests at the end of the file:**

```typescript
// ── Job Descriptions Contract ─────────────────────────────────────────

test('POST /job-descriptions returns { data: entity } envelope', async () => {
  const res = await apiRequest(ctx.app, 'POST', '/job-descriptions', {
    title: 'Contract JD',
    raw_text: 'Testing contract shape.',
  })
  expect(res.status).toBe(201)
  const body = await res.json()

  expect(body).toHaveProperty('data')
  expect(body.data).toHaveProperty('id')
  expect(body.data).toHaveProperty('title')
  expect(body.data).toHaveProperty('status')
  expect(body.data).toHaveProperty('organization_name')
  expect(body.data).toHaveProperty('created_at')
  expect(body).not.toHaveProperty('error')
})

test('GET /job-descriptions returns { data: [], pagination: {} } envelope', async () => {
  seedJobDescription(ctx.db)

  const res = await apiRequest(ctx.app, 'GET', '/job-descriptions')
  expect(res.status).toBe(200)
  const body = await res.json()

  expect(body).toHaveProperty('data')
  expect(body.data).toBeArray()
  expect(body).toHaveProperty('pagination')
  expect(body.pagination).toHaveProperty('total')
  expect(body.pagination).toHaveProperty('offset')
  expect(body.pagination).toHaveProperty('limit')
})
```

**Note:** Add the following import to the top of `contracts.test.ts`:

```typescript
import { seedJobDescription } from '../../db/__tests__/helpers'
```

---

### T31.14: UI Components (Deferred Detail)

The UI implementation depends on the webui framework patterns already established. The following files are outlined here for completeness; full implementation will be filled in during the implementation phase based on the existing component patterns in `packages/webui/`.

**Files:**
- `packages/webui/src/pages/JobDescriptions.tsx` -- List view at `/opportunities/job-descriptions`
- `packages/webui/src/pages/JobDescriptionDetail.tsx` -- Detail view
- `packages/webui/src/components/NewJobDescriptionModal.tsx` -- Creation modal
- `packages/webui/src/components/JobDescriptionCard.tsx` -- Card component for list
- `packages/webui/src/components/StatusBadge.tsx` -- Status pill with color

Note: The route path `/opportunities/job-descriptions` depends on Phase 32 (Nav Restructuring). If Phase 32 is not yet merged, use a temporary route path and update after Phase 32 lands.

**Key UI behaviors from spec:**
- Status badge colors: interested=gray, analyzing=blue, applied=indigo, interviewing=purple, offered=green, rejected=red, withdrawn=orange, closed=dark-gray
- Organization autocomplete uses `GET /api/organizations?search=<term>&limit=10`
- Status dropdown on detail view saves immediately on change (PATCH with `{ status }`)
- Delete requires confirmation dialog
- `raw_text` displayed with preserved whitespace (`white-space: pre-wrap`)
- Organization name displayed as "No organization" when `organization_id` is null

**Acceptance criteria:**
- [ ] List view renders at `/opportunities/job-descriptions`
- [ ] Cards show title, org name, status badge, location, salary, created date
- [ ] Status filter dropdown filters the list
- [ ] Organization filter dropdown filters the list
- [ ] "New JD" button opens creation modal
- [ ] Creation modal has fields: title, organization (autocomplete), URL, salary, location, raw_text (textarea), notes
- [ ] Detail view shows all fields with inline editing
- [ ] Status dropdown saves immediately on change
- [ ] Delete button shows confirmation dialog
- [ ] Copy-to-clipboard button for raw text

---

## Testing Support

### Fixtures

The `seedJobDescription` helper (T31.8) provides a reusable fixture for all test files. It defaults to realistic values:
- `title`: "Senior Security Engineer"
- `rawText`: "We are looking for a senior security engineer to join our team..."
- `status`: "interested"
- All nullable fields default to null

### Test Kinds

| Test file | Kind | Count | What it validates |
|-----------|------|-------|-------------------|
| `job-description-repository.test.ts` | Unit | 20 | CRUD, filtering, pagination, ON DELETE SET NULL, CHECK constraints, note_references |
| `job-description-service.test.ts` | Unit | 17 | Input validation, error codes, business logic |
| `job-descriptions.test.ts` (routes) | Integration | 17 | HTTP status codes, response envelopes, query params |
| `contracts.test.ts` (additions) | Contract | 2 | Response envelope shape conformance |
| **Total** | | **56** | |

### Test Cases Summary

**Repository layer (unit):**
- create with defaults / all fields / with org
- get by ID / nonexistent ID
- list all / filter by status / filter by org_id / combined filter / pagination / org_name included
- update partial / set org_id to null / change org_id / nonexistent ID
- delete / nonexistent ID
- ON DELETE SET NULL when org is deleted
- CHECK constraint rejects invalid status
- note_references accepts 'job_description' entity_type
- note_references rejects invalid entity_type

**Service layer (unit):**
- create validation: empty title, whitespace title, empty raw_text, invalid status, valid input, with org
- get: nonexistent ID, valid ID
- list: invalid status filter, paginated results
- update validation: empty title, empty raw_text, invalid status, nonexistent ID, valid input
- delete: nonexistent ID, valid ID

**Route layer (integration):**
- POST: creates and returns 201, with org includes org_name, rejects empty title (400), rejects empty raw_text (400)
- GET list: paginated envelope, filters by status, filters by org_id, includes org_name
- GET single: returns with org_name, 404 for nonexistent
- PATCH: updates fields, org_id null clears org, 404 for nonexistent, 400 for invalid status
- DELETE: returns 204, 404 for nonexistent

**Contract layer:**
- POST returns `{ data }` envelope with expected fields
- GET list returns `{ data: [], pagination: {} }` envelope

---

## Documentation Requirements

- [ ] Add JSDoc comments to all public functions in `job-description-repository.ts`
- [ ] Add JSDoc comments to all public methods in `JobDescriptionService`
- [ ] Add JSDoc comments to the `JobDescriptionsResource` SDK class
- [ ] Add JSDoc to new types (`JobDescription`, `JobDescriptionWithOrg`, `JobDescriptionStatus`, etc.)
- [ ] Comment the migration SQL with step numbers and purpose descriptions
- [ ] Document the `NoteReferenceEntityType` union type in both core and SDK

---

## Parallelization Notes

### Internal Parallelization (within Phase 31)

| Stream | Tasks | Prerequisite |
|--------|-------|-------------|
| A | T31.1 (migration SQL) | None |
| B | T31.2 (types in core + SDK) | None (interface-first) |
| C | T31.3 (repository) | A + B |
| D | T31.4 (service) | C |
| E | T31.5 (register service) | D |
| F | T31.6 (routes) | E |
| G | T31.7 (SDK resource + client registration) | B (parallel with C-F) |
| H | T31.8 (test helpers) | A |
| I | T31.9 (repository tests) | C + H |
| J | T31.10 (service tests) | D + H |
| K | T31.11 (route tests) | F + H |
| L | T31.12 (contract tests) | K |
| M | T31.14 (UI) | F + G |

**Fastest critical path:** A -> C -> D -> E -> F -> K (6 serial steps)
**SDK stream (G) runs parallel** with C -> D -> E -> F since it only depends on types (B).

### External Parallelization (with other phases)

| Phase | Can run in parallel? | Notes |
|-------|---------------------|-------|
| Phase 29 (Profile, migration 005) | Yes | DDL independent; only migration numbering overlaps |
| Phase 30 (Summaries, migration 006) | Yes | DDL independent; only migration numbering overlaps |
| Phase 32 (Nav Restructuring) | Yes | JD views will mount under `/opportunities/job-descriptions` which nav restructuring defines, but the view code can be developed independently |

**Migration ordering constraint:** Migrations 005, 006, and 007 can be developed in any order, but they must be _applied_ in numeric order (005 -> 006 -> 007). The `_migrations` table tracks applied names, and the runner sorts by filename. If developing 007 before 005/006, temporarily name it `005_job_descriptions.sql` for local testing, then rename to `007_` before merge.

---

## Acceptance Criteria (Full List)

- [ ] `job_descriptions` table exists after migration 007 with correct schema and constraints
- [ ] `note_references` CHECK constraint includes `'job_description'`
- [ ] CRUD operations work: create, list, get, update, delete JDs via API
- [ ] Organization linking: JD can reference an organization; `organization_name` is included in API responses
- [ ] Status tracking: status field accepts all 8 valid values; status can be changed via PATCH
- [ ] List filtering: `?status=applied` and `?organization_id=...` filters work correctly
- [ ] Combined filtering: `?status=applied&organization_id=...` returns intersection
- [ ] `ON DELETE SET NULL`: deleting an organization sets `organization_id = NULL` on linked JDs
- [ ] `ON DELETE SET NULL` tested explicitly
- [ ] List view handles `organization_id = null` gracefully (shows "No organization")
- [ ] CHECK constraint rejects invalid status values
- [ ] JD list view renders at `/opportunities/job-descriptions`
- [ ] New JD modal allows pasting full JD text with metadata fields
- [ ] JD detail view shows full text with inline editing for all fields
- [ ] Status dropdown on detail view saves immediately on change
- [ ] Organization autocomplete on creation form searches existing orgs
- [ ] SDK `JobDescriptionsResource` with all 5 CRUD methods
- [ ] Delete confirmation dialog before deleting a JD
- [ ] `NoteReferenceEntityType` union type defined in core and SDK
- [ ] `NoteReference.entity_type` uses union type (not raw `string`)
- [ ] All 56 tests pass
- [ ] Types in sync between `@forge/core` and `@forge/sdk`

## Estimated Effort

| Task | Lines changed (est.) |
|------|---------------------|
| T31.1 Migration SQL | ~50 |
| T31.2 Types (core + SDK) | ~120 |
| T31.3 Repository | ~130 |
| T31.4 Service | ~110 |
| T31.5 Register service | ~15 |
| T31.6 Routes + server mount | ~60 |
| T31.7 SDK resource + client + barrel | ~90 |
| T31.8 Test helpers | ~25 |
| T31.9 Repository tests | ~250 |
| T31.10 Service tests | ~180 |
| T31.11 Route tests | ~200 |
| T31.12 Contract tests | ~30 |
| T31.14 UI components | ~400 |
| **Total** | **~1,660** |
