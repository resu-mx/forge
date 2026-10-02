# Phase 34: Summary Templates (Spec 3)

**Status:** Planning
**Date:** 2026-03-31
**Spec:** [2026-03-30-summary-templates.md](../refs/specs/2026-03-30-summary-templates.md)
**Depends on:** Phase 30 (Summaries Entity — `summaries` table, `SummaryRepository`, CRUD API, `SummariesResource`)
**Blocks:** None
**Parallelizable with:** Phase 33 (Chain View Modal), Phase 35+ (any phase not touching summaries)

> **PREREQUISITE:** Phase 30 must add `summary_id: string | null` to the `resumes` table (migration 006), the `Resume` type, `UpdateResume` type, `ResumeRow`, and `ResumeRepository.update()`. Verify these exist before beginning this phase. If Phase 30 has not been merged, this phase cannot implement the resume-summary linking mechanism.

## Goal

Implement the behavioral template layer on top of the `summaries` table created in Phase 30. Templates (`is_template = 1`) are reusable summary blueprints: the user can promote any summary to a template, clone a template when creating a resume, and see which resumes are linked to a given summary. This phase adds the toggle-template endpoint, the linked-resumes endpoint, the `linked_resume_count` computed field, SDK wiring for both, `ForgeClient.summaries` registration, and the full UI: a templates/summaries split list on `/data/summaries`, a summary picker in the resume creation flow, and informational banners during editing.

## Non-Goals

- Template categories or tagging
- Template versioning (change history)
- Template inheritance (parent/child)
- Template import/export
- AI-assisted template generation
- Archetype-to-template auto-mapping
- Template usage analytics beyond `linked_resume_count`
- Clone behavior implementation (already defined in Phase 30 / Spec 2)

## Context

Phase 30 creates the `summaries` table with an `is_template INTEGER NOT NULL DEFAULT 0` column and an index `idx_summaries_template`. It also creates `SummaryRepository` with full CRUD + clone, the `/api/summaries` route file, and `SummariesResource` in the SDK with `create`, `list`, `get`, `update`, `delete`, `clone` methods. This phase extends that foundation with template-specific behavior without any schema changes.

The `clone` endpoint (Phase 30) already sets `is_template = 0` on the clone and prefixes the title with `'Copy of '`. This phase does NOT duplicate that logic. It only adds the toggle, linked-resumes query, computed count, and the UI flows that consume them.

**Known Limitations:**

- The resume is created BEFORE the summary picker opens. Cancelling the picker does not delete the newly created resume -- the user gets a resume with no summary linked. This is intentional to avoid the complexity of deferred resume creation.

**Fallback Strategies:**

- If Phase 32 (Nav Restructuring) is incomplete, the Summaries view can be developed at `/summaries` and re-routed to `/data/summaries` later. The component logic is route-independent.
- If `linked_resume_count` subquery causes performance issues on large datasets, add a materialized column via a trigger-based counter cache in a future migration. For now the subquery approach is sufficient for a single-user system.
- If the summary picker modal proves too complex for a single phase, defer the `[Create New]` inline form to a follow-up; the [Use], [Link], and [Skip] actions are the core behavior.

---

## Scope

| Spec section | Covered here? |
|-------------|---------------|
| 1. Schema Changes | No (already in Phase 30) |
| 2.1 Existing CRUD endpoints | No (already in Phase 30) |
| 2.2 Toggle-template endpoint | Yes |
| 2.3 Clone behavior | No (already in Phase 30) |
| 2.4 Linked resumes endpoint | Yes |
| 3.1 Summaries view — template section | Yes |
| 3.2 Resume creation flow — summary picker | Yes |
| 3.3 Summary detail/edit view banners | Yes |
| 4. Type changes (`linked_resume_count`) | Yes |
| 4.1 SDK resource method additions | Yes |
| 5. Acceptance criteria | Yes |

## Files to Create

| File | Purpose |
|------|---------|
| `packages/core/src/db/repositories/__tests__/summary-repository.test.ts` | Repository unit tests for toggle + linked-resumes + computed count |
| `packages/core/src/services/__tests__/summary-service.test.ts` | Service unit tests for toggle + linked-resumes |
| `packages/core/src/routes/__tests__/summaries.test.ts` | Route integration tests |
| `packages/sdk/src/__tests__/summaries-resource.test.ts` | SDK resource unit tests |
| `packages/webui/src/lib/components/SummaryPicker.svelte` | Summary picker modal for resume creation flow |

## Files to Modify

| File | Change |
|------|--------|
| `packages/core/src/types/index.ts` | Add `linked_resume_count` to `Summary` interface |
| `packages/sdk/src/types.ts` | Add `linked_resume_count` to `Summary`; add `SummaryFilter`; add `ForgeClient.summaries` typing |
| `packages/core/src/db/repositories/summary-repository.ts` | Add `toggleTemplate`, `getLinkedResumes`, `linked_resume_count` subquery to SELECT |
| `packages/core/src/services/summary-service.ts` | Add `toggleTemplate`, `getLinkedResumes` service methods |
| `packages/core/src/routes/summaries.ts` | Add `POST /:id/toggle-template`, `GET /:id/linked-resumes` route handlers |
| `packages/sdk/src/resources/summaries.ts` | Add `toggleTemplate()`, `linkedResumes()` methods; fix `toParams` boolean conversion |
| `packages/sdk/src/client.ts` | Add `public summaries: SummariesResource` property and constructor wiring |
| `packages/sdk/src/index.ts` | Export `SummariesResource`, `Summary`, `CreateSummary`, `UpdateSummary`, `SummaryFilter` |
| `packages/core/src/db/__tests__/helpers.ts` | Add `seedSummary` test helper |
| `packages/webui/src/routes/resumes/+page.svelte` | Integrate summary picker into resume creation flow |
| `packages/webui/src/routes/data/summaries/+page.svelte` | New page (or modify stub from Phase 32) with templates/summaries split |
| `packages/sdk/src/__tests__/resources.test.ts` | Add summaries resource test cases |

---

## Tasks

### T34.0: Pre-Implementation Verification

Before beginning implementation, verify:

1. **`resumes` table has `summary_id` column** — migration 006 must add `summary_id TEXT REFERENCES summaries(id) ON DELETE SET NULL` to the `resumes` table.
2. **`Resume` type has `summary_id: string | null`** — in `packages/core/src/types/index.ts`.
3. **`UpdateResume` has `summary_id?: string | null`** — in `packages/core/src/types/index.ts` and `packages/sdk/src/types.ts`.
4. **`ResumeRow` has `summary_id`** — if a separate row type exists in the repository.
5. **`ResumeRepository.update()` handles `summary_id` in its field loop** — the PATCH handler must be able to set `summary_id` on a resume.

If any of these are missing, Phase 30 has not been completed and this phase cannot proceed. File issues or complete Phase 30 first.

---

### T34.1: Add `seedSummary` Test Helper

**File:** `packages/core/src/db/__tests__/helpers.ts`

Add a seed helper following the existing pattern:

```typescript
/** Seed a test summary and return its ID */
export function seedSummary(db: Database, opts: {
  title?: string
  role?: string | null
  tagline?: string | null
  description?: string | null
  isTemplate?: boolean
  notes?: string | null
} = {}): string {
  const id = testUuid()
  db.run(
    `INSERT INTO summaries (id, title, role, tagline, description, is_template, notes)
     VALUES (?, ?, ?, ?, ?, ?, ?)`,
    [
      id,
      opts.title ?? 'Test Summary',
      opts.role ?? null,
      opts.tagline ?? null,
      opts.description ?? null,
      opts.isTemplate ? 1 : 0,
      opts.notes ?? null,
    ]
  )
  return id
}
```

**Acceptance criteria:**
- `seedSummary(db)` inserts a row with `is_template = 0` and returns a UUID
- `seedSummary(db, { isTemplate: true })` inserts a row with `is_template = 1`
- The row is retrievable via `SELECT * FROM summaries WHERE id = ?`

**Failure criteria:**
- Calling `seedSummary` fails with a SQLite error -> the `summaries` table does not exist -> Phase 30 migration has not been applied

---

### T34.2: Add `linked_resume_count` Computed Field to Repository SELECTs

**File:** `packages/core/src/db/repositories/summary-repository.ts`

Update the `get` and `list` functions to include a subquery that computes `linked_resume_count`:

**Update `get`:**

```typescript
export function get(db: Database, id: string): Summary | null {
  return (
    db.query(
      `SELECT s.*,
              (SELECT COUNT(*) FROM resumes WHERE summary_id = s.id) AS linked_resume_count
       FROM summaries s
       WHERE s.id = ?`
    ).get(id) as Summary | null
  ) ?? null
}
```

**Update `list`:**

Replace the `SELECT *` in the list query with:

```typescript
const rows = db
  .query(
    `SELECT s.*,
            (SELECT COUNT(*) FROM resumes WHERE summary_id = s.id) AS linked_resume_count
     FROM summaries s
     ${where}
     ORDER BY s.is_template DESC, s.updated_at DESC
     LIMIT ? OFFSET ?`
  )
  .all(...dataParams) as Summary[]
```

Also update the `COUNT(*)` query to select from `summaries s` for consistency:

```typescript
const countRow = db
  .query(`SELECT COUNT(*) AS total FROM summaries s ${where}`)
  .get(...params) as { total: number }
```

**Note on list ordering:** `ORDER BY s.is_template DESC` puts templates first (1 before 0), then by recency. This matches the UI requirement of templates appearing above regular summaries.

**Acceptance criteria:**
- `get(db, id)` returns an object with `linked_resume_count: number`
- A summary linked to 0 resumes returns `linked_resume_count: 0`
- A summary linked to 3 resumes returns `linked_resume_count: 3`
- `list` results include `linked_resume_count` on every row

**Failure criteria:**
- `linked_resume_count` is undefined or missing -> the subquery was not added to SELECT

---

### T34.3: Add `toggleTemplate` Repository Method

**File:** `packages/core/src/db/repositories/summary-repository.ts`

Add an atomic toggle method that flips `is_template` without a read-modify-write cycle:

```typescript
/** Toggle the is_template flag atomically. Returns the updated summary or null if not found. */
export function toggleTemplate(db: Database, id: string): Summary | null {
  const row = db
    .query(
      `UPDATE summaries
       SET is_template = ((is_template + 1) % 2),
           updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
       WHERE id = ?
       RETURNING *`
    )
    .get(id) as (Omit<Summary, 'linked_resume_count'>) | null

  if (!row) return null

  // Fetch the full row with linked_resume_count
  return get(db, id)
}
```

**Key design decision:** The atomic `((is_template + 1) % 2)` expression avoids a SELECT-then-UPDATE race condition. Since SQLite serializes writes, this is safe even in concurrent scenarios. The RETURNING clause gives us the updated row, but we re-fetch via `get` to include `linked_resume_count`.

**Acceptance criteria:**
- Calling `toggleTemplate` on a summary with `is_template = 0` sets it to `1` and returns the updated row
- Calling `toggleTemplate` again on the same summary sets it back to `0`
- Calling `toggleTemplate` with a non-existent ID returns `null`
- `updated_at` is refreshed on each toggle
- The returned object includes `linked_resume_count`
- The returned Summary object has `linked_resume_count` equal to the current count of resumes referencing this summary at the time of the toggle call

**Failure criteria:**
- Toggle sets `is_template` to a value other than 0 or 1 -> modular arithmetic error

---

### T34.4: Add `getLinkedResumes` Repository Method

**File:** `packages/core/src/db/repositories/summary-repository.ts`

Add a paginated query for resumes linked to a summary:

```typescript
import type { Resume } from '../../types'

/** List resumes linked to a summary via summary_id, with pagination. */
export function getLinkedResumes(
  db: Database,
  summaryId: string,
  offset = 0,
  limit = 50,
): { data: Resume[]; total: number } {
  const countRow = db
    .query('SELECT COUNT(*) AS total FROM resumes WHERE summary_id = ?')
    .get(summaryId) as { total: number }

  const rows = db
    .query(
      `SELECT * FROM resumes
       WHERE summary_id = ?
       ORDER BY updated_at DESC
       LIMIT ? OFFSET ?`
    )
    .all(summaryId, limit, offset) as Resume[]

  return { data: rows, total: countRow.total }
}
```

**Acceptance criteria:**
- Returns `{ data: Resume[], total: number }` with standard pagination
- A summary linked to 0 resumes returns `{ data: [], total: 0 }`
- A summary linked to 5 resumes with `limit=2, offset=0` returns `{ data: [r1, r2], total: 5 }`
- Results are ordered by `updated_at DESC` (most recently updated resume first)

**Failure criteria:**
- Query fails because `resumes.summary_id` column does not exist -> Phase 30 migration not applied

---

### T34.5: Add `toggleTemplate` and `getLinkedResumes` Service Methods

**File:** `packages/core/src/services/summary-service.ts`

Add two methods to the existing `SummaryService` class:

```typescript
import type { Summary, Resume, Result, PaginatedResult } from '../types'
import * as SummaryRepo from '../db/repositories/summary-repository'

// Inside the SummaryService class:

toggleTemplate(id: string): Result<Summary> {
  const summary = SummaryRepo.toggleTemplate(this.db, id)
  if (!summary) {
    return { ok: false, error: { code: 'NOT_FOUND', message: `Summary ${id} not found` } }
  }
  return { ok: true, data: summary }
}

getLinkedResumes(
  id: string,
  offset?: number,
  limit?: number,
): PaginatedResult<Resume> {
  // Verify the summary exists before querying linked resumes
  const summary = SummaryRepo.get(this.db, id)
  if (!summary) {
    return { ok: false, error: { code: 'NOT_FOUND', message: `Summary ${id} not found` } }
  }

  const result = SummaryRepo.getLinkedResumes(this.db, id, offset, limit)
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
```

**Acceptance criteria:**
- `toggleTemplate('nonexistent')` returns `{ ok: false, error: { code: 'NOT_FOUND' } }`
- `toggleTemplate(existingId)` returns `{ ok: true, data: { is_template: 1, linked_resume_count: 0 } }` on first call
- `getLinkedResumes('nonexistent')` returns `{ ok: false, error: { code: 'NOT_FOUND' } }`
- `getLinkedResumes(existingId)` returns `{ ok: true, data: [...], pagination: { ... } }`

**Failure criteria:**
- Service does not validate summary existence before `getLinkedResumes` -> returns empty data instead of 404 for nonexistent IDs

---

### T34.6: Add Route Handlers for Toggle and Linked Resumes

**File:** `packages/core/src/routes/summaries.ts`

Add two new route handlers to the existing summaries Hono router:

```typescript
// POST /summaries/:id/toggle-template
app.post('/summaries/:id/toggle-template', (c) => {
  const result = services.summaries.toggleTemplate(c.req.param('id'))
  if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
  return c.json({ data: result.data })
})

// GET /summaries/:id/linked-resumes
app.get('/summaries/:id/linked-resumes', (c) => {
  const offset = Math.max(0, parseInt(c.req.query('offset') ?? '0', 10) || 0)
  const limit = Math.min(200, Math.max(1, parseInt(c.req.query('limit') ?? '50', 10) || 50))

  const result = services.summaries.getLinkedResumes(c.req.param('id'), offset, limit)
  if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
  return c.json({ data: result.data, pagination: result.pagination })
})
```

**Route registration order:** These routes MUST be registered BEFORE the `/:id` catch-all GET route. If `toggle-template` is registered after `/:id`, Hono will match `toggle-template` as an ID parameter. Place them immediately after the `POST /summaries` (create) route and before `GET /summaries/:id`.

**Acceptance criteria:**
- `POST /api/summaries/:id/toggle-template` returns 200 with the updated summary
- `POST /api/summaries/nonexistent/toggle-template` returns 404
- `GET /api/summaries/:id/linked-resumes` returns 200 with `{ data: [...], pagination: {...} }`
- `GET /api/summaries/nonexistent/linked-resumes` returns 404
- Pagination query params (`offset`, `limit`) are respected
- `limit` is clamped to `[1, 200]`

**Failure criteria:**
- Hono matches `toggle-template` as an `:id` parameter -> route registered in wrong order

---

### T34.7: Update `is_template` Filter in Summaries List Route

**File:** `packages/core/src/routes/summaries.ts`

The existing `GET /summaries` route handler (from Phase 30) reads `is_template` as a query parameter. Ensure it converts the string `"1"` / `"0"` to an integer before passing to the repository filter:

```typescript
app.get('/summaries', (c) => {
  const offset = Math.max(0, parseInt(c.req.query('offset') ?? '0', 10) || 0)
  const limit = Math.min(200, Math.max(1, parseInt(c.req.query('limit') ?? '50', 10) || 50))
  const filter: Record<string, unknown> = {}

  const isTemplateParam = c.req.query('is_template')
  if (isTemplateParam !== undefined && isTemplateParam !== null) {
    filter.is_template = parseInt(isTemplateParam, 10)
  }

  const result = services.summaries.list(filter, offset, limit)
  if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
  return c.json({ data: result.data, pagination: result.pagination })
})
```

**Key detail:** The `is_template` column is `INTEGER` in SQLite. Query params come in as strings. The route must `parseInt` the value. The SDK's `toParams` converts booleans to `"true"/"false"` which will NOT match — this is addressed in T34.8.

**Acceptance criteria:**
- `GET /api/summaries?is_template=1` returns only templates
- `GET /api/summaries?is_template=0` returns only regular summaries
- `GET /api/summaries` (no filter) returns all

**Failure criteria:**
- Passing `is_template=true` (string) returns all summaries because `parseInt("true")` is `NaN` -> filter silently dropped

---

### T34.8: Add SDK `SummariesResource` with Template Methods

**File:** `packages/sdk/src/resources/summaries.ts`

Create the full SDK resource class. Phase 30 may have already created this file with basic CRUD. This task adds `toggleTemplate()` and `linkedResumes()`, and fixes `toParams` boolean-to-integer conversion:

```typescript
import type {
  CreateSummary,
  PaginatedResult,
  PaginationParams,
  RequestFn,
  RequestListFn,
  Result,
  Resume,
  Summary,
  SummaryFilter,
  UpdateSummary,
} from '../types'

function toParams(
  filter?: object,
): Record<string, string> | undefined {
  if (!filter) return undefined
  const out: Record<string, string> = {}
  for (const [k, v] of Object.entries(filter)) {
    if (v === undefined || v === null) continue
    // Boolean→"1"/"0" conversion for is_template (SQLite INTEGER column)
    if (k === 'is_template' && typeof v === 'boolean') {
      out[k] = v ? '1' : '0'
    } else {
      out[k] = String(v)
    }
  }
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
    filter?: SummaryFilter & PaginationParams,
  ): Promise<PaginatedResult<Summary>> {
    return this.requestList<Summary>(
      'GET',
      '/api/summaries',
      toParams(filter),
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

  toggleTemplate(id: string): Promise<Result<Summary>> {
    return this.request<Summary>('POST', `/api/summaries/${id}/toggle-template`)
  }

  linkedResumes(
    id: string,
    params?: PaginationParams,
  ): Promise<PaginatedResult<Resume>> {
    return this.requestList<Resume>(
      'GET',
      `/api/summaries/${id}/linked-resumes`,
      toParams(params),
    )
  }
}
```

**Key design decision — `toParams` boolean conversion:** The generic `toParams` helper uses `String(v)` which converts `true` to `"true"`. SQLite will not match `"true"` against `INTEGER` columns. The `is_template` field needs explicit `'1'`/`'0'` conversion. This is handled with a field-specific check inside `toParams` rather than modifying the generic helper shared across resources.

**Note on boolean scoping:** The field-specific `if (k === 'is_template')` check in `toParams` handles only this boolean field. If additional boolean filter fields are added to `SummaryFilter` in the future, each must be explicitly handled. Consider refactoring to a `booleanFields: Set<string>` approach when a second boolean field is needed.

**Acceptance criteria:**
- `forge.summaries.list({ is_template: true })` sends `GET /api/summaries?is_template=1`
- `forge.summaries.list({ is_template: false })` sends `GET /api/summaries?is_template=0`
- `forge.summaries.list()` sends `GET /api/summaries` (no query param)
- `forge.summaries.toggleTemplate('abc')` sends `POST /api/summaries/abc/toggle-template`
- `forge.summaries.linkedResumes('abc', { limit: 10 })` sends `GET /api/summaries/abc/linked-resumes?limit=10`

**Failure criteria:**
- `list({ is_template: true })` sends `?is_template=true` -> SQLite filter fails silently, returns all rows

---

### T34.9: Wire `SummariesResource` into `ForgeClient`

**File:** `packages/sdk/src/client.ts`

Add the import and property:

```typescript
// Add to imports at top:
import { SummariesResource } from './resources/summaries'

// Add to class properties (after skills):
/** Summary CRUD, templates, linked resumes. */
public summaries: SummariesResource

// Add to constructor body (after skills):
this.summaries = new SummariesResource(req, reqList)
```

**Acceptance criteria:**
- `new ForgeClient({ baseUrl: '...' }).summaries` is an instance of `SummariesResource`
- `forge.summaries.list()` is callable

**Failure criteria:**
- `forge.summaries` is `undefined` -> property not assigned in constructor

---

### T34.10: Update SDK Types

**File:** `packages/sdk/src/types.ts`

Add the `Summary` entity type with `linked_resume_count`, and the input/filter types. If Phase 30 already added `Summary`, `CreateSummary`, `UpdateSummary`, update `Summary` to include `linked_resume_count` and add `SummaryFilter`:

```typescript
// ---------------------------------------------------------------------------
// Summary entity
// ---------------------------------------------------------------------------

/** A reusable professional summary. */
export interface Summary {
  id: string
  title: string
  role: string | null
  tagline: string | null
  description: string | null
  is_template: boolean
  linked_resume_count: number
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

/** Filter options for listing summaries. */
export interface SummaryFilter {
  is_template?: boolean
}
```

**Note on `is_template` type:** The SDK uses `boolean` for developer ergonomics. SQLite stores `INTEGER` (0/1). The SDK's `toParams` handles the conversion (T34.8). The API returns the raw INTEGER from SQLite. JavaScript treats `0` as falsy and `1` as truthy, so consumers using `if (summary.is_template)` will work correctly without explicit conversion. However, strict equality checks (`=== true`) will fail. Document this in JSDoc.

**Acceptance criteria:**
- `Summary.linked_resume_count` is `number`
- `Summary.is_template` is `boolean` in the SDK type (even though the API returns integer)
- `SummaryFilter` is exported and usable in `list()` calls

**Failure criteria:**
- `linked_resume_count` missing from `Summary` type -> TypeScript errors in UI when accessing it

---

### T34.11: Update SDK Barrel Exports

**File:** `packages/sdk/src/index.ts`

Add summary-related exports:

```typescript
// Summary entity + input types
export type {
  Summary,
  CreateSummary,
  UpdateSummary,
  SummaryFilter,
} from './types'

// Summary resource class
export { SummariesResource } from './resources/summaries'
```

**Acceptance criteria:**
- `import type { Summary, SummaryFilter } from '@forge/sdk'` compiles
- `import { SummariesResource } from '@forge/sdk'` compiles

---

### T34.12: Update Core Types

**File:** `packages/core/src/types/index.ts`

If Phase 30 already added `Summary`, update it to include `linked_resume_count`:

```typescript
/** A reusable professional summary. */
export interface Summary {
  id: string
  title: string
  role: string | null
  tagline: string | null
  description: string | null
  is_template: number  // 0 or 1 (SQLite integer)
  linked_resume_count: number  // computed via subquery
  notes: string | null
  created_at: string
  updated_at: string
}
```

**Note:** Core types use `number` for `is_template` (matching the SQLite INTEGER column). SDK types use `boolean`. This follows the existing pattern where core types reflect the database schema directly and SDK types provide ergonomic wrappers.

**Acceptance criteria:**
- `Summary.linked_resume_count` exists as `number`
- No breaking changes to `CreateSummary` or `UpdateSummary` (they do not include `linked_resume_count`)

---

### T34.13: Summaries View — Templates Section Split

**File:** `packages/webui/src/routes/data/summaries/+page.svelte` (create if not existing from Phase 32; modify if stub exists)

**Soft dep:** Phase 32 (Nav Restructuring) -- for the correct route path `/data/summaries`.

Build the summaries list page with two sections: Templates above, Summaries below.

```svelte
<script lang="ts">
  import { forge, friendlyError } from '$lib/sdk'
  import { addToast } from '$lib/stores/toast.svelte'
  import { LoadingSpinner, EmptyState, ConfirmDialog } from '$lib/components'
  import type { Summary } from '@forge/sdk'

  let summaries = $state<Summary[]>([])
  let loading = $state(true)
  let confirmDeleteId = $state<string | null>(null)
  let editing = $state<string | null>(null)
  let saving = $state(false)

  // Form fields for inline edit
  let formTitle = $state('')
  let formRole = $state('')
  let formTagline = $state('')
  let formDescription = $state('')
  let formNotes = $state('')

  let templates = $derived(summaries.filter(s => s.is_template))
  let instances = $derived(summaries.filter(s => !s.is_template))

  $effect(() => { loadSummaries() })

  async function loadSummaries() {
    loading = true
    const result = await forge.summaries.list({ limit: 500 })
    if (result.ok) {
      summaries = result.data
    } else {
      addToast({ message: friendlyError(result.error, 'Failed to load summaries'), type: 'error' })
    }
    loading = false
  }

  async function toggleTemplate(id: string) {
    const result = await forge.summaries.toggleTemplate(id)
    if (result.ok) {
      summaries = summaries.map(s => s.id === id ? result.data : s)
      addToast({
        message: result.data.is_template ? 'Promoted to template' : 'Demoted from template',
        type: 'success',
      })
    } else {
      addToast({ message: friendlyError(result.error, 'Failed to toggle template'), type: 'error' })
    }
  }

  async function cloneSummary(id: string) {
    const result = await forge.summaries.clone(id)
    if (result.ok) {
      summaries = [result.data, ...summaries]
      addToast({ message: `Cloned as "${result.data.title}"`, type: 'success' })
    } else {
      addToast({ message: friendlyError(result.error, 'Failed to clone'), type: 'error' })
    }
  }

  async function deleteSummary(id: string) {
    const result = await forge.summaries.delete(id)
    if (result.ok) {
      summaries = summaries.filter(s => s.id !== id)
      confirmDeleteId = null
      if (editing === id) editing = null
      addToast({ message: 'Summary deleted', type: 'success' })
    } else {
      addToast({ message: friendlyError(result.error, 'Failed to delete'), type: 'error' })
    }
  }

  function startEdit(summary: Summary) {
    editing = summary.id
    formTitle = summary.title
    formRole = summary.role ?? ''
    formTagline = summary.tagline ?? ''
    formDescription = summary.description ?? ''
    formNotes = summary.notes ?? ''
  }

  async function saveEdit(id: string) {
    saving = true
    const result = await forge.summaries.update(id, {
      title: formTitle,
      role: formRole || null,
      tagline: formTagline || null,
      description: formDescription || null,
      notes: formNotes || null,
    })
    if (result.ok) {
      summaries = summaries.map(s => s.id === id ? result.data : s)
      editing = null
      addToast({ message: 'Summary updated', type: 'success' })
    } else {
      addToast({ message: friendlyError(result.error, 'Failed to update'), type: 'error' })
    }
    saving = false
  }

  async function createSummary() {
    const result = await forge.summaries.create({ title: 'New Summary' })
    if (result.ok) {
      summaries = [result.data, ...summaries]
      startEdit(result.data)
      addToast({ message: 'Summary created', type: 'success' })
    } else {
      addToast({ message: friendlyError(result.error, 'Failed to create'), type: 'error' })
    }
  }
</script>

<div class="p-6 max-w-4xl mx-auto">
  <div class="flex items-center justify-between mb-6">
    <h1 class="text-2xl font-bold">Summaries</h1>
    <button onclick={createSummary}
            class="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700">
      + New Summary
    </button>
  </div>

  {#if loading}
    <LoadingSpinner />
  {:else if summaries.length === 0}
    <EmptyState message="No summaries yet. Create one to get started." />
  {:else}
    <!-- Templates Section -->
    {#if templates.length > 0}
      <section class="mb-8">
        <h2 class="text-lg font-semibold mb-3 flex items-center gap-2">
          <span class="text-yellow-500">&#9733;</span> Templates
        </h2>
        <div class="space-y-2">
          {#each templates as summary (summary.id)}
            {@render summaryRow(summary, true)}
          {/each}
        </div>
      </section>
    {/if}

    <!-- Regular Summaries Section -->
    <section>
      <h2 class="text-lg font-semibold mb-3">Summaries</h2>
      {#if instances.length === 0}
        <p class="text-gray-500 text-sm">No regular summaries. All summaries are templates.</p>
      {:else}
        <div class="space-y-2">
          {#each instances as summary (summary.id)}
            {@render summaryRow(summary, false)}
          {/each}
        </div>
      {/if}
    </section>
  {/if}

  <ConfirmDialog
    open={confirmDeleteId !== null}
    title="Delete Summary"
    message="Are you sure? This will detach this summary from any linked resumes."
    onconfirm={() => confirmDeleteId && deleteSummary(confirmDeleteId)}
    oncancel={() => confirmDeleteId = null}
  />
</div>

{#snippet summaryRow(summary: Summary, isTemplate: boolean)}
  <div class="border rounded p-4 {isTemplate ? 'border-yellow-300 bg-yellow-50' : 'border-gray-200'}">
    {#if editing === summary.id}
      <!-- Edit mode -->
      <div class="space-y-3">
        <div>
          <label class="block text-sm font-medium text-gray-700 mb-1">Title</label>
          <input bind:value={formTitle} class="w-full border rounded px-3 py-1.5 text-sm" />
        </div>
        <div>
          <label class="block text-sm font-medium text-gray-700 mb-1">Role</label>
          <input bind:value={formRole} class="w-full border rounded px-3 py-1.5 text-sm"
                 placeholder="e.g. Senior Security Engineer" />
        </div>
        <div>
          <label class="block text-sm font-medium text-gray-700 mb-1">Tagline</label>
          <input bind:value={formTagline} class="w-full border rounded px-3 py-1.5 text-sm"
                 placeholder="e.g. Cloud + DevSecOps + Detection Engineering" />
        </div>
        <div>
          <label class="block text-sm font-medium text-gray-700 mb-1">Description</label>
          <textarea bind:value={formDescription} rows="3"
                    class="w-full border rounded px-3 py-1.5 text-sm"
                    placeholder="Full summary paragraph..." />
        </div>
        <div>
          <label class="block text-sm font-medium text-gray-700 mb-1">Notes (internal)</label>
          <textarea bind:value={formNotes} rows="2"
                    class="w-full border rounded px-3 py-1.5 text-sm" />
        </div>

        <!-- Template/multi-resume banners -->
        {#if isTemplate}
          <div class="bg-blue-50 border border-blue-200 rounded px-3 py-2 text-sm text-blue-800">
            This is a template. Changes here will NOT affect resumes that were previously created from this template.
          </div>
        {:else if summary.linked_resume_count > 1}
          <div class="bg-amber-50 border border-amber-200 rounded px-3 py-2 text-sm text-amber-800">
            This summary is linked to {summary.linked_resume_count} resumes. Changes will be reflected in all of them.
          </div>
        {/if}

        <div class="flex gap-2">
          <button onclick={() => saveEdit(summary.id)} disabled={saving}
                  class="px-3 py-1.5 bg-blue-600 text-white text-sm rounded hover:bg-blue-700 disabled:opacity-50">
            {saving ? 'Saving...' : 'Save'}
          </button>
          <button onclick={() => editing = null}
                  class="px-3 py-1.5 bg-gray-200 text-sm rounded hover:bg-gray-300">
            Cancel
          </button>
        </div>
      </div>
    {:else}
      <!-- Display mode -->
      <div class="flex items-start justify-between">
        <div class="flex-1 min-w-0">
          <div class="flex items-center gap-2">
            {#if isTemplate}
              <span class="text-yellow-500" title="Template">&#9733;</span>
            {/if}
            <h3 class="font-medium text-sm truncate">{summary.title}</h3>
            {#if summary.linked_resume_count > 0}
              <span class="text-xs text-gray-500">
                ({summary.linked_resume_count} resume{summary.linked_resume_count !== 1 ? 's' : ''})
              </span>
            {/if}
          </div>
          {#if summary.role}
            <p class="text-sm text-gray-600 mt-0.5">{summary.role}</p>
          {/if}
          {#if summary.tagline}
            <p class="text-xs text-gray-500 mt-0.5 italic">{summary.tagline}</p>
          {/if}
        </div>
        <div class="flex gap-1 ml-4 flex-shrink-0">
          <button onclick={() => startEdit(summary)}
                  class="px-2 py-1 text-xs bg-gray-100 rounded hover:bg-gray-200">Edit</button>
          <button onclick={() => cloneSummary(summary.id)}
                  class="px-2 py-1 text-xs bg-gray-100 rounded hover:bg-gray-200">Clone</button>
          <button onclick={() => toggleTemplate(summary.id)}
                  class="px-2 py-1 text-xs bg-gray-100 rounded hover:bg-gray-200">
            {isTemplate ? 'Demote' : 'Promote'}
          </button>
          <button onclick={() => confirmDeleteId = summary.id}
                  class="px-2 py-1 text-xs bg-red-50 text-red-600 rounded hover:bg-red-100">Delete</button>
        </div>
      </div>
    {/if}
  </div>
{/snippet}
```

**Acceptance criteria:**
- Templates appear above regular summaries with a star icon and yellow background
- "Promote" button on regular summaries calls `toggleTemplate` and moves the summary to the templates section
- "Demote" button on templates calls `toggleTemplate` and moves it to the regular section
- "Clone" button creates a copy and adds it to the list
- Edit mode shows informational banners:
  - Template: "Changes here will NOT affect resumes..."
  - Multi-linked (N > 1): "This summary is linked to N resumes..."
- `linked_resume_count` is displayed next to each summary title

**Failure criteria:**
- UI does not re-derive `templates`/`instances` after toggle -> summary appears in both sections or neither

---

### T34.14: Summary Picker Component

**File:** `packages/webui/src/lib/components/SummaryPicker.svelte`

A modal component used during resume creation. Shows templates first, then existing summaries, with action buttons.

```svelte
<script lang="ts">
  import { forge, friendlyError } from '$lib/sdk'
  import { addToast } from '$lib/stores/toast.svelte'
  import { LoadingSpinner } from '$lib/components'
  import type { Summary } from '@forge/sdk'

  interface Props {
    open: boolean
    /** Called when the user picks a summary. `id` is the summary to link, or null to skip. */
    onpick: (summaryId: string | null) => void
    oncancel: () => void
  }

  let { open, onpick, oncancel }: Props = $props()

  let summaries = $state<Summary[]>([])
  let loading = $state(true)
  let showCreateForm = $state(false)
  let creating = $state(false)
  let warningForId = $state<string | null>(null)
  let warningResumeName = $state<string | null>(null)

  // Create form fields
  let newTitle = $state('')
  let newRole = $state('')
  let newTagline = $state('')
  let newDescription = $state('')

  let templates = $derived(summaries.filter(s => s.is_template))
  let instances = $derived(summaries.filter(s => !s.is_template))

  $effect(() => {
    if (open) {
      loadSummaries()
      showCreateForm = false
      warningForId = null
      newTitle = ''
      newRole = ''
      newTagline = ''
      newDescription = ''
    }
  })

  async function loadSummaries() {
    loading = true
    const result = await forge.summaries.list({ limit: 500 })
    if (result.ok) {
      summaries = result.data
    } else {
      addToast({ message: friendlyError(result.error, 'Failed to load summaries'), type: 'error' })
    }
    loading = false
  }

  /** [Use] a template: clone it, then link the clone */
  async function useTemplate(id: string) {
    const result = await forge.summaries.clone(id)
    if (result.ok) {
      onpick(result.data.id)
    } else {
      addToast({ message: friendlyError(result.error, 'Failed to clone template'), type: 'error' })
    }
  }

  /** [Link] an existing summary directly */
  async function linkSummary(id: string) {
    // Check if this summary is already linked to another resume
    const summary = summaries.find(s => s.id === id)
    if (summary && summary.linked_resume_count > 0) {
      // Fetch linked resumes to show the warning
      const linkedResult = await forge.summaries.linkedResumes(id, { limit: 1 })
      if (linkedResult.ok && linkedResult.data.length > 0) {
        warningForId = id
        warningResumeName = linkedResult.data[0].name
        return
      }
    }
    onpick(id)
  }

  /** User confirmed [Link Anyway] after seeing the shared-summary warning */
  function confirmLink() {
    if (warningForId) {
      onpick(warningForId)
      warningForId = null
    }
  }

  /** User chose [Clone] from the shared-summary warning dialog */
  async function cloneInstead() {
    if (warningForId) {
      const result = await forge.summaries.clone(warningForId)
      if (result.ok) {
        onpick(result.data.id)
      } else {
        addToast({ message: friendlyError(result.error, 'Failed to clone'), type: 'error' })
      }
      warningForId = null
    }
  }

  /** [Create New] inline form submission */
  async function createAndLink() {
    if (!newTitle.trim()) {
      addToast({ message: 'Title is required', type: 'error' })
      return
    }
    creating = true
    const result = await forge.summaries.create({
      title: newTitle.trim(),
      role: newRole.trim() || undefined,
      tagline: newTagline.trim() || undefined,
      description: newDescription.trim() || undefined,
    })
    if (result.ok) {
      onpick(result.data.id)
    } else {
      addToast({ message: friendlyError(result.error, 'Failed to create summary'), type: 'error' })
    }
    creating = false
  }
</script>

{#if open}
  <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
    <div class="bg-white rounded-lg shadow-xl max-w-lg w-full max-h-[80vh] flex flex-col">
      <div class="flex items-center justify-between px-6 py-4 border-b">
        <h2 class="text-lg font-semibold">Pick a Summary</h2>
        <button onclick={() => { showCreateForm = true; warningForId = null }}
                class="px-3 py-1.5 text-sm bg-blue-600 text-white rounded hover:bg-blue-700">
          Create New
        </button>
      </div>

      <div class="overflow-y-auto flex-1 px-6 py-4">
        {#if loading}
          <LoadingSpinner />
        {:else if showCreateForm}
          <!-- Inline creation form -->
          <div class="space-y-3">
            <div>
              <label class="block text-sm font-medium text-gray-700 mb-1">Title *</label>
              <input bind:value={newTitle} class="w-full border rounded px-3 py-1.5 text-sm"
                     placeholder="e.g. Security Engineer - Cloud Focus" />
            </div>
            <div>
              <label class="block text-sm font-medium text-gray-700 mb-1">Role</label>
              <input bind:value={newRole} class="w-full border rounded px-3 py-1.5 text-sm"
                     placeholder="e.g. Senior Security Engineer" />
            </div>
            <div>
              <label class="block text-sm font-medium text-gray-700 mb-1">Tagline</label>
              <input bind:value={newTagline} class="w-full border rounded px-3 py-1.5 text-sm"
                     placeholder="e.g. Cloud + DevSecOps + Detection Engineering" />
            </div>
            <div>
              <label class="block text-sm font-medium text-gray-700 mb-1">Description</label>
              <textarea bind:value={newDescription} rows="3"
                        class="w-full border rounded px-3 py-1.5 text-sm"
                        placeholder="Full summary paragraph..." />
            </div>
            <div class="flex gap-2">
              <button onclick={createAndLink} disabled={creating}
                      class="px-3 py-1.5 bg-blue-600 text-white text-sm rounded hover:bg-blue-700 disabled:opacity-50">
                {creating ? 'Creating...' : 'Create & Link'}
              </button>
              <button onclick={() => showCreateForm = false}
                      class="px-3 py-1.5 bg-gray-200 text-sm rounded hover:bg-gray-300">
                Back
              </button>
            </div>
          </div>
        {:else if warningForId}
          <!-- Shared summary warning -->
          <div class="bg-amber-50 border border-amber-200 rounded p-4">
            <p class="text-sm text-amber-800 mb-3">
              This summary is also used by <strong>{warningResumeName}</strong>.
              Changes will affect both resumes. Clone instead?
            </p>
            <div class="flex gap-2">
              <button onclick={cloneInstead}
                      class="px-3 py-1.5 bg-blue-600 text-white text-sm rounded hover:bg-blue-700">
                Clone
              </button>
              <button onclick={confirmLink}
                      class="px-3 py-1.5 bg-gray-200 text-sm rounded hover:bg-gray-300">
                Link Anyway
              </button>
              <button onclick={() => warningForId = null}
                      class="px-3 py-1.5 bg-gray-200 text-sm rounded hover:bg-gray-300">
                Cancel
              </button>
            </div>
          </div>
        {:else}
          <!-- Templates -->
          {#if templates.length > 0}
            <div class="mb-4">
              <h3 class="text-sm font-semibold text-gray-600 mb-2">Templates</h3>
              <div class="space-y-1">
                {#each templates as summary (summary.id)}
                  <div class="flex items-center justify-between border rounded px-3 py-2 border-yellow-200 bg-yellow-50">
                    <div class="flex items-center gap-2 min-w-0">
                      <span class="text-yellow-500 flex-shrink-0">&#9733;</span>
                      <span class="text-sm truncate">{summary.title}</span>
                    </div>
                    <button onclick={() => useTemplate(summary.id)}
                            class="px-2 py-1 text-xs bg-blue-600 text-white rounded hover:bg-blue-700 flex-shrink-0 ml-2">
                      Use
                    </button>
                  </div>
                {/each}
              </div>
            </div>
          {/if}

          <!-- Existing Summaries -->
          {#if instances.length > 0}
            <div>
              <h3 class="text-sm font-semibold text-gray-600 mb-2">Existing Summaries</h3>
              <div class="space-y-1">
                {#each instances as summary (summary.id)}
                  <div class="flex items-center justify-between border rounded px-3 py-2">
                    <span class="text-sm truncate min-w-0">{summary.title}</span>
                    <button onclick={() => linkSummary(summary.id)}
                            class="px-2 py-1 text-xs bg-gray-200 rounded hover:bg-gray-300 flex-shrink-0 ml-2">
                      Link
                    </button>
                  </div>
                {/each}
              </div>
            </div>
          {/if}

          {#if templates.length === 0 && instances.length === 0}
            <p class="text-sm text-gray-500 text-center py-4">
              No summaries yet. Create one or skip for now.
            </p>
          {/if}
        {/if}
      </div>

      <!-- Footer -->
      <div class="flex justify-between px-6 py-3 border-t bg-gray-50 rounded-b-lg">
        <button onclick={oncancel}
                class="px-3 py-1.5 text-sm text-gray-600 hover:text-gray-800">
          Cancel
        </button>
        <button onclick={() => onpick(null)}
                class="px-3 py-1.5 text-sm text-gray-600 hover:text-gray-800">
          Skip
        </button>
      </div>
    </div>
  </div>
{/if}
```

**Acceptance criteria:**
- Templates section appears above Existing Summaries
- [Use] on a template clones it and calls `onpick(cloneId)`
- [Link] on a regular summary calls `onpick(summaryId)` directly
- [Link] on a summary with `linked_resume_count > 0` shows the shared-summary warning
- Warning shows [Clone], [Link Anyway], [Cancel]
- [Create New] opens inline form; submitting creates summary and calls `onpick(newId)`
- [Skip] calls `onpick(null)`
- [Cancel] calls `oncancel`

**Failure criteria:**
- [Use] links the template directly instead of cloning -> shared editing between template and resume
- [Link] on an already-linked summary does not show warning -> user unknowingly creates shared reference

---

### T34.15: Integrate Summary Picker into Resume Creation Flow

**File:** `packages/webui/src/routes/resumes/+page.svelte`

> **PREREQUISITE:** Phase 30 must add `summary_id: string | null` to the `resumes` table (migration 006), the `Resume` type, `UpdateResume` type, `ResumeRow`, and `ResumeRepository.update()`. Verify these exist before beginning this phase. If Phase 30 has not been merged, this phase cannot implement the resume-summary linking mechanism.

Add the summary picker to the resume creation flow. After the user fills in the basic resume fields (name, target_role, target_employer, archetype) and clicks "Create", show the summary picker before finalizing:

```svelte
<script lang="ts">
  // Add to existing imports:
  import SummaryPicker from '$lib/components/SummaryPicker.svelte'

  // Add state:
  let showSummaryPicker = $state(false)
  let pendingResumeId = $state<string | null>(null)

  // Modify the existing createResume function:
  async function createResume() {
    // ... existing validation ...
    const result = await forge.resumes.create({
      name: formName,
      target_role: formTargetRole,
      target_employer: formTargetEmployer,
      archetype: formArchetype,
    })
    if (result.ok) {
      pendingResumeId = result.data.id
      showSummaryPicker = true
    } else {
      addToast({ message: friendlyError(result.error, 'Failed to create resume'), type: 'error' })
    }
  }

  async function handleSummaryPick(summaryId: string | null) {
    showSummaryPicker = false
    if (pendingResumeId && summaryId) {
      // Link the picked/cloned summary to the new resume
      const result = await forge.resumes.update(pendingResumeId, { summary_id: summaryId })
      if (!result.ok) {
        addToast({ message: friendlyError(result.error, 'Failed to link summary'), type: 'error' })
      }
    }
    // Refresh the resume list regardless
    await loadResumes()
    pendingResumeId = null
    addToast({ message: 'Resume created', type: 'success' })
  }

  function handleSummaryCancel() {
    // User cancelled the picker — resume was already created, just close
    showSummaryPicker = false
    loadResumes()
    pendingResumeId = null
    addToast({ message: 'Resume created (no summary linked)', type: 'success' })
  }
</script>

<!-- Add to template, after existing content: -->
<SummaryPicker
  open={showSummaryPicker}
  onpick={handleSummaryPick}
  oncancel={handleSummaryCancel}
/>
```

**Note:** The resume is created FIRST (without summary), then the summary is linked via `PATCH /api/resumes/:id`. This avoids a complex two-step transaction and works because `summary_id` is nullable.

**Acceptance criteria:**
- After clicking "Create Resume", the summary picker appears
- Picking a summary (or skipping) links it to the resume and refreshes the list
- Cancelling the picker still creates the resume (without a summary)
- The resume list shows the newly created resume

**Failure criteria:**
- Resume creation fails silently if summary linking fails -> resume exists but user sees an error without knowing the resume was created

---

### T34.16: Update `services/index.ts` — Register SummaryService

**File:** `packages/core/src/services/index.ts`

Add the `SummaryService` to the `Services` interface and `createServices` factory. Phase 30 may have already done this; verify and add if missing:

```typescript
import { SummaryService } from './summary-service'

export interface Services {
  // ... existing services ...
  summaries: SummaryService
}

export function createServices(db: Database): Services {
  // ... existing ...
  return {
    // ... existing ...
    summaries: new SummaryService(db),
  }
}

export { SummaryService } from './summary-service'
```

**Acceptance criteria:**
- `services.summaries` is an instance of `SummaryService`
- Route handlers can call `services.summaries.toggleTemplate(id)`

---

### T34.17: Register Summary Routes in `server.ts`

**File:** `packages/core/src/routes/server.ts`

Add the summary routes to the app. Phase 30 may have already done this; verify and add if missing:

```typescript
import { summaryRoutes } from './summaries'

// In createApp(), add after existing route registrations:
app.route('/', summaryRoutes(services))
```

**Acceptance criteria:**
- `POST /api/summaries/:id/toggle-template` is reachable
- `GET /api/summaries/:id/linked-resumes` is reachable

---

## Testing Support

### Test Fixtures

| Fixture | Setup |
|---------|-------|
| Template summary | `seedSummary(db, { title: 'Cloud Security Template', isTemplate: true, role: 'Security Engineer', tagline: 'Cloud + DevSecOps' })` |
| Regular summary | `seedSummary(db, { title: 'Platform Engineer AWS' })` |
| Summary linked to resume | `const sId = seedSummary(db); const rId = seedResume(db); db.run('UPDATE resumes SET summary_id = ? WHERE id = ?', [sId, rId])` |
| Summary linked to 3 resumes | Same as above, repeated 3 times with different resume IDs |

### Test Cases

#### Repository Tests (`summary-repository.test.ts`)

| Test | Kind | Description |
|------|------|-------------|
| `toggleTemplate flips 0 to 1` | unit | Create summary with `is_template=0`, toggle, verify it's `1` |
| `toggleTemplate flips 1 to 0` | unit | Create template, toggle, verify it's `0` |
| `toggleTemplate on nonexistent returns null` | unit | Call with fake UUID, verify `null` |
| `toggleTemplate updates updated_at` | unit | Record `updated_at` before toggle, verify it changed after |
| `get includes linked_resume_count 0` | unit | Create summary with no linked resumes, verify `linked_resume_count === 0` |
| `get includes linked_resume_count 2` | unit | Create summary, link 2 resumes, verify `linked_resume_count === 2` |
| `list includes linked_resume_count` | unit | Seed summaries with varying linked counts, verify all have the field |
| `list with is_template=1 filters correctly` | unit | Seed templates and non-templates, filter, verify only templates returned |
| `list orders templates first` | unit | Seed both types, list without filter, verify templates come first |
| `getLinkedResumes returns paginated results` | unit | Seed summary with 3 linked resumes, query with limit=2, verify `data.length===2` and `total===3` |
| `getLinkedResumes returns empty for unlinked summary` | unit | Seed summary with no linked resumes, verify `data: [], total: 0` |
| `delete template does not cascade` | unit | Create template, clone it, delete template, verify clone still exists |

```typescript
import { describe, test, expect, beforeEach } from 'bun:test'
import { createTestDb, seedSummary, seedResume, testUuid } from '../../db/__tests__/helpers'
import * as SummaryRepo from '../summary-repository'
import type { Database } from 'bun:sqlite'

describe('SummaryRepository — template extensions', () => {
  let db: Database

  beforeEach(() => {
    db = createTestDb()
  })

  describe('toggleTemplate', () => {
    test('flips 0 to 1', () => {
      const id = seedSummary(db, { isTemplate: false })
      const result = SummaryRepo.toggleTemplate(db, id)
      expect(result).not.toBeNull()
      expect(result!.is_template).toBe(1)
    })

    test('flips 1 to 0', () => {
      const id = seedSummary(db, { isTemplate: true })
      const result = SummaryRepo.toggleTemplate(db, id)
      expect(result).not.toBeNull()
      expect(result!.is_template).toBe(0)
    })

    test('returns null for nonexistent id', () => {
      const result = SummaryRepo.toggleTemplate(db, testUuid())
      expect(result).toBeNull()
    })

    test('updates updated_at', () => {
      const id = seedSummary(db)
      const before = SummaryRepo.get(db, id)!.updated_at
      // Small delay to ensure timestamp changes
      SummaryRepo.toggleTemplate(db, id)
      const after = SummaryRepo.get(db, id)!.updated_at
      // updated_at should be at least as recent (SQLite second precision)
      expect(after >= before).toBe(true)
    })
  })

  describe('linked_resume_count', () => {
    test('get includes linked_resume_count = 0 when no resumes linked', () => {
      const id = seedSummary(db)
      const summary = SummaryRepo.get(db, id)
      expect(summary).not.toBeNull()
      expect(summary!.linked_resume_count).toBe(0)
    })

    test('get includes correct linked_resume_count', () => {
      const sId = seedSummary(db)
      const r1 = seedResume(db)
      const r2 = seedResume(db)
      db.run('UPDATE resumes SET summary_id = ? WHERE id = ?', [sId, r1])
      db.run('UPDATE resumes SET summary_id = ? WHERE id = ?', [sId, r2])

      const summary = SummaryRepo.get(db, sId)
      expect(summary!.linked_resume_count).toBe(2)
    })

    test('list includes linked_resume_count on every row', () => {
      const s1 = seedSummary(db, { title: 'A' })
      const s2 = seedSummary(db, { title: 'B' })
      const r1 = seedResume(db)
      db.run('UPDATE resumes SET summary_id = ? WHERE id = ?', [s1, r1])

      const result = SummaryRepo.list(db)
      const found1 = result.data.find(s => s.id === s1)
      const found2 = result.data.find(s => s.id === s2)
      expect(found1!.linked_resume_count).toBe(1)
      expect(found2!.linked_resume_count).toBe(0)
    })
  })

  describe('getLinkedResumes', () => {
    test('returns paginated results', () => {
      const sId = seedSummary(db)
      for (let i = 0; i < 3; i++) {
        const rId = seedResume(db, { name: `Resume ${i}` })
        db.run('UPDATE resumes SET summary_id = ? WHERE id = ?', [sId, rId])
      }

      const result = SummaryRepo.getLinkedResumes(db, sId, 0, 2)
      expect(result.data).toHaveLength(2)
      expect(result.total).toBe(3)
    })

    test('returns empty for unlinked summary', () => {
      const sId = seedSummary(db)
      const result = SummaryRepo.getLinkedResumes(db, sId)
      expect(result.data).toHaveLength(0)
      expect(result.total).toBe(0)
    })
  })

  describe('list filtering', () => {
    test('is_template=1 returns only templates', () => {
      seedSummary(db, { title: 'Template', isTemplate: true })
      seedSummary(db, { title: 'Instance', isTemplate: false })

      const result = SummaryRepo.list(db, { is_template: 1 })
      expect(result.data).toHaveLength(1)
      expect(result.data[0].title).toBe('Template')
    })

    test('is_template=0 returns only instances', () => {
      seedSummary(db, { title: 'Template', isTemplate: true })
      seedSummary(db, { title: 'Instance', isTemplate: false })

      const result = SummaryRepo.list(db, { is_template: 0 })
      expect(result.data).toHaveLength(1)
      expect(result.data[0].title).toBe('Instance')
    })

    test('no filter returns all, templates first', () => {
      seedSummary(db, { title: 'Instance', isTemplate: false })
      seedSummary(db, { title: 'Template', isTemplate: true })

      const result = SummaryRepo.list(db)
      expect(result.data).toHaveLength(2)
      expect(result.data[0].is_template).toBe(1)  // templates first
      expect(result.data[1].is_template).toBe(0)
    })
  })

  describe('delete safety', () => {
    test('deleting a template does not affect cloned summaries', () => {
      const templateId = seedSummary(db, { title: 'My Template', isTemplate: true })
      const cloneId = seedSummary(db, { title: 'Copy of My Template', isTemplate: false })

      SummaryRepo.del(db, templateId)
      const clone = SummaryRepo.get(db, cloneId)
      expect(clone).not.toBeNull()
      expect(clone!.title).toBe('Copy of My Template')
    })
  })
})
```

#### Service Tests (`summary-service.test.ts`)

| Test | Kind | Description |
|------|------|-------------|
| `toggleTemplate returns updated summary` | unit | Service wraps repo call correctly |
| `toggleTemplate returns NOT_FOUND for missing id` | unit | Error handling |
| `getLinkedResumes returns NOT_FOUND for missing summary` | unit | Error handling |
| `getLinkedResumes returns paginated results for valid summary` | unit | Happy path |

```typescript
import { describe, test, expect, beforeEach } from 'bun:test'
import { createTestDb, seedSummary, seedResume } from '../../db/__tests__/helpers'
import { SummaryService } from '../summary-service'
import type { Database } from 'bun:sqlite'

describe('SummaryService — template extensions', () => {
  let db: Database
  let service: SummaryService

  beforeEach(() => {
    db = createTestDb()
    service = new SummaryService(db)
  })

  describe('toggleTemplate', () => {
    test('returns updated summary', () => {
      const id = seedSummary(db, { isTemplate: false })
      const result = service.toggleTemplate(id)
      expect(result.ok).toBe(true)
      if (result.ok) {
        expect(result.data.is_template).toBe(1)
      }
    })

    test('returns NOT_FOUND for nonexistent id', () => {
      const result = service.toggleTemplate('00000000-0000-0000-0000-000000000000')
      expect(result.ok).toBe(false)
      if (!result.ok) {
        expect(result.error.code).toBe('NOT_FOUND')
      }
    })
  })

  describe('getLinkedResumes', () => {
    test('returns NOT_FOUND for nonexistent summary', () => {
      const result = service.getLinkedResumes('00000000-0000-0000-0000-000000000000')
      expect(result.ok).toBe(false)
      if (!result.ok) {
        expect(result.error.code).toBe('NOT_FOUND')
      }
    })

    test('returns paginated results', () => {
      const sId = seedSummary(db)
      const rId = seedResume(db, { name: 'Linked Resume' })
      db.run('UPDATE resumes SET summary_id = ? WHERE id = ?', [sId, rId])

      const result = service.getLinkedResumes(sId)
      expect(result.ok).toBe(true)
      if (result.ok) {
        expect(result.data).toHaveLength(1)
        expect(result.data[0].name).toBe('Linked Resume')
        expect(result.pagination.total).toBe(1)
      }
    })
  })
})
```

#### Route Integration Tests (`summaries.test.ts`)

| Test | Kind | Description |
|------|------|-------------|
| `POST /toggle-template returns 200 with toggled summary` | integration | Full HTTP round-trip |
| `POST /toggle-template returns 404 for missing id` | integration | Error path |
| `GET /linked-resumes returns 200 with paginated data` | integration | Full HTTP round-trip |
| `GET /linked-resumes returns 404 for missing id` | integration | Error path |
| `GET /summaries?is_template=1 filters correctly` | integration | Filter verification |

#### SDK Resource Tests (`summaries-resource.test.ts`)

| Test | Kind | Description |
|------|------|-------------|
| `toggleTemplate sends POST to correct path` | unit | URL verification |
| `linkedResumes sends GET with pagination params` | unit | URL + params verification |
| `list with is_template=true sends ?is_template=1` | unit | Boolean-to-integer conversion |
| `list with is_template=false sends ?is_template=0` | unit | Boolean-to-integer conversion |
| `list without filter sends no query params` | unit | No spurious params |

```typescript
// In packages/sdk/src/__tests__/summaries-resource.test.ts
import { describe, expect, it, beforeEach, afterEach, mock } from 'bun:test'
import { ForgeClient } from '../client'

function jsonResponse(body: unknown, init: { status?: number } = {}): Response {
  return new Response(JSON.stringify(body), {
    status: init.status ?? 200,
    headers: { 'Content-Type': 'application/json' },
  })
}

function calledUrl(fetchMock: ReturnType<typeof mock>): string {
  return (fetchMock.mock.calls[0] as [string])[0]
}

function calledInit(fetchMock: ReturnType<typeof mock>): RequestInit {
  return (fetchMock.mock.calls[0] as [string, RequestInit])[1]
}

describe('SummariesResource', () => {
  const originalFetch = globalThis.fetch
  let fetchMock: ReturnType<typeof mock>
  let client: ForgeClient

  beforeEach(() => {
    fetchMock = mock(() => Promise.resolve(new Response()))
    globalThis.fetch = fetchMock as typeof fetch
    client = new ForgeClient({ baseUrl: 'http://localhost:3000' })
  })

  afterEach(() => {
    globalThis.fetch = originalFetch
  })

  it('toggleTemplate sends POST to /api/summaries/:id/toggle-template', async () => {
    const summary = { id: 's1', title: 'Test', is_template: 1 }
    fetchMock.mockImplementation(() =>
      Promise.resolve(jsonResponse({ data: summary })),
    )

    await client.summaries.toggleTemplate('s1')

    expect(calledUrl(fetchMock)).toBe('http://localhost:3000/api/summaries/s1/toggle-template')
    expect(calledInit(fetchMock).method).toBe('POST')
  })

  it('linkedResumes sends GET with pagination params', async () => {
    fetchMock.mockImplementation(() =>
      Promise.resolve(jsonResponse({ data: [], pagination: { total: 0, offset: 0, limit: 10 } })),
    )

    await client.summaries.linkedResumes('s1', { limit: 10, offset: 5 })

    expect(calledUrl(fetchMock)).toContain('/api/summaries/s1/linked-resumes')
    expect(calledUrl(fetchMock)).toContain('limit=10')
    expect(calledUrl(fetchMock)).toContain('offset=5')
  })

  it('list with is_template=true sends ?is_template=1', async () => {
    fetchMock.mockImplementation(() =>
      Promise.resolve(jsonResponse({ data: [], pagination: { total: 0, offset: 0, limit: 50 } })),
    )

    await client.summaries.list({ is_template: true })

    expect(calledUrl(fetchMock)).toContain('is_template=1')
    expect(calledUrl(fetchMock)).not.toContain('is_template=true')
  })

  it('list with is_template=false sends ?is_template=0', async () => {
    fetchMock.mockImplementation(() =>
      Promise.resolve(jsonResponse({ data: [], pagination: { total: 0, offset: 0, limit: 50 } })),
    )

    await client.summaries.list({ is_template: false })

    expect(calledUrl(fetchMock)).toContain('is_template=0')
  })

  it('list without filter sends no query params', async () => {
    fetchMock.mockImplementation(() =>
      Promise.resolve(jsonResponse({ data: [], pagination: { total: 0, offset: 0, limit: 50 } })),
    )

    await client.summaries.list()

    expect(calledUrl(fetchMock)).toBe('http://localhost:3000/api/summaries')
  })

  it('clone sends POST to /api/summaries/:id/clone', async () => {
    const cloned = { id: 's2', title: 'Copy of Test', is_template: 0 }
    fetchMock.mockImplementation(() =>
      Promise.resolve(jsonResponse({ data: cloned }, { status: 201 })),
    )

    await client.summaries.clone('s1')

    expect(calledUrl(fetchMock)).toBe('http://localhost:3000/api/summaries/s1/clone')
    expect(calledInit(fetchMock).method).toBe('POST')
  })
})
```

#### Component Smoke Tests

| Test | Kind | Description |
|------|------|-------------|
| `SummaryPicker renders templates above instances` | component | DOM structure |
| `SummaryPicker [Use] calls onpick after clone` | component | Callback behavior |
| `SummaryPicker [Skip] calls onpick with null` | component | Callback behavior |
| `SummaryPicker [Link] on summary with linked_resume_count === 0 calls onpick(id) directly` | component | No warning dialog when summary is unlinked |
| `SummaryPicker [Link] on shared summary (count > 0) shows warning with other resume name` | component | Warning dialog displays the linked resume's name |
| `SummaryPicker [Clone] in warning dialog clones and calls onpick with clone id` | component | Clone-from-warning flow |
| `SummaryPicker [Link Anyway] in warning dialog calls onpick with original id` | component | Direct-link-from-warning flow |
| `Summaries page splits templates and summaries` | component | Section rendering |

---

## Acceptance Criteria

- [ ] `POST /api/summaries/:id/toggle-template` toggles `is_template` and returns the updated summary
- [ ] `POST /api/summaries/:id/toggle-template` on nonexistent ID returns 404
- [ ] Toggle uses atomic SQL `((is_template + 1) % 2)` — no read-modify-write race
- [ ] `GET /api/summaries/:id/linked-resumes` returns `PaginatedResult<Resume>`
- [ ] `GET /api/summaries/:id/linked-resumes` on nonexistent ID returns 404
- [ ] `Summary` response includes `linked_resume_count: number` on both `get` and `list`
- [ ] `GET /api/summaries?is_template=1` returns only templates
- [ ] `GET /api/summaries?is_template=0` returns only instances
- [ ] `GET /api/summaries` (no filter) returns all, templates first
- [ ] SDK `SummariesResource` has `toggleTemplate()`, `linkedResumes()`, `clone()`, and CRUD methods
- [ ] SDK `toParams` converts `is_template: true` to `"1"`, not `"true"`
- [ ] `ForgeClient.summaries` is wired as `SummariesResource`
- [ ] Summaries view shows templates above regular summaries with visual distinction
- [ ] Promote/Demote buttons toggle template status and move items between sections
- [ ] Edit mode shows banner: "This is a template. Changes will NOT affect..." for templates
- [ ] Edit mode shows banner: "This summary is linked to N resumes..." for N > 1
- [ ] Resume creation flow opens summary picker after basic fields
- [ ] [Use] on a template clones it and links the clone to the resume
- [ ] [Link] on a shared summary (linked_resume_count > 0) shows the "also used by" warning
- [ ] [Link Anyway] links directly; [Clone] clones then links
- [ ] [Create New] opens inline form, creates summary, links to resume
- [ ] [Skip] creates resume with `summary_id = NULL`
- [ ] Deleting a template does not affect previously cloned summaries
- [ ] Clone behavior (title prefix, `is_template = 0`) is inherited from Phase 30 — not reimplemented

## Documentation Requirements

- [ ] Add JSDoc to `toggleTemplate` repository and service methods
- [ ] Add JSDoc to `getLinkedResumes` repository and service methods
- [ ] Add JSDoc to `SummariesResource.toggleTemplate()` and `linkedResumes()` in the SDK
- [ ] Document `linked_resume_count` as a computed field in the `Summary` type JSDoc
- [ ] Document the `is_template` boolean/integer discrepancy between SDK and core types
- [ ] Document the `toParams` boolean-to-integer conversion for `is_template`

## Parallelization Notes

| Stream | Tasks | Dependencies |
|--------|-------|-------------|
| A: Backend (repo + service + routes) | T34.1, T34.2, T34.3, T34.4, T34.5, T34.6, T34.7, T34.16, T34.17 | Phase 30 must be complete |
| B: SDK | T34.8, T34.9, T34.10, T34.11 | Depends on A's API contract (can start once endpoints are defined) |
| C: UI — Summaries view | T34.13 | Depends on B (needs `SummariesResource`) |
| D: UI — Summary picker + resume flow | T34.14, T34.15 | Depends on B + C |
| E: Types (core + SDK) | T34.12 | Can start in parallel with A |

**Recommended implementation order:**
1. T34.1 (seed helper) + T34.12 (core types) — foundation
2. T34.2, T34.3, T34.4 (repository methods) — backend data layer
3. T34.5, T34.6, T34.7 (service + routes) — backend API
4. T34.8, T34.9, T34.10, T34.11 (SDK) — client library
5. T34.13 (summaries view) — primary UI
6. T34.14, T34.15 (summary picker + resume flow) — secondary UI
7. T34.16, T34.17 (service/route registration) — wiring (verify early, fix if needed)

**Streams A+E can be done by one developer. Stream B is a thin wrapper and takes ~30 minutes. Streams C+D can be done by a second developer once B is complete.**

## Estimated Effort

| Task | Lines changed (est.) |
|------|---------------------|
| T34.1 Seed helper | ~20 |
| T34.2 `linked_resume_count` subquery | ~30 |
| T34.3 `toggleTemplate` repo method | ~20 |
| T34.4 `getLinkedResumes` repo method | ~25 |
| T34.5 Service methods | ~40 |
| T34.6-T34.7 Route handlers | ~30 |
| T34.8 SDK resource | ~80 |
| T34.9 Client wiring | ~10 |
| T34.10-T34.11 SDK types + exports | ~60 |
| T34.12 Core types | ~10 |
| T34.13 Summaries view | ~250 |
| T34.14 Summary picker | ~200 |
| T34.15 Resume flow integration | ~40 |
| T34.16-T34.17 Service/route registration | ~15 |
| Tests (all) | ~300 |
| **Total** | **~1,130** |
