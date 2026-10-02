# Summaries as Standalone Entities

**Date:** 2026-03-30
**Status:** Design
**Builds on:** Resume Renderer (Phases 19-20), Resume Sections as Entities (Phase 27-28), Navigation Restructuring (2026-03-30)

## Purpose

Summaries (the professional summary / tagline / role title that appear at the top of a resume) are currently stored as a JSON blob in `resumes.header`. This creates several problems:

1. **No reuse** — the same summary text must be copy-pasted across resumes
2. **No management** — there is no UI for browsing, searching, or editing summaries independently
3. **Mixed concerns** — `resumes.header` mashes together contact info (name, email, phone) with role-specific content (tagline, description, role title)

This spec extracts summaries into a standalone `summaries` table. Contact info moves to a global Profile (separate spec). The IR compiler combines both at render time.

## Goals

1. `summaries` table with title, role, tagline, description fields
2. `resumes.summary_id` FK linking a resume to its summary
3. CRUD API endpoints for summaries
4. Summaries view under Data > Summaries with tabs
5. Resume creation flow picks or creates a summary
6. Existing `resumes.header` JSON data migrated into `summaries` rows
7. Template support via `is_template` flag (detailed in Spec 3)

## Non-Goals

- Contact info management (separate Profile spec)
- Summary generation via AI (future)
- Summary versioning / history
- Summary sharing across users (single-user system)
- Summary diff/comparison view

---

## 1. Schema Changes (Migration 006)

### 1.1 `summaries` Table

```sql
-- Forge Resume Builder — Summaries as Standalone Entities
-- Migration: 006_summaries
-- Date: 2026-03-30
--
-- Extracts resume summaries from the header JSON blob into a standalone table.
-- Adds summary_id FK to resumes. Migrates existing header data.
-- Note: 006's DDL is schema-independent of 005_user_profile. The ordering
-- dependency is only in the _migrations table sequence, not in the schema.

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
```

**Column semantics:**

| Column | Purpose | Example |
|--------|---------|---------|
| `title` | Internal label for the summary (not rendered on resume) | "Security Engineer - Cloud Focus" |
| `role` | Role title rendered on the resume | "Senior Security Engineer" |
| `tagline` | One-line positioning statement | "Cloud + DevSecOps + Detection Engineering" |
| `description` | Full summary paragraph | "Security engineer with 8+ years..." |
| `is_template` | Whether this is a reusable template (see Spec 3) | 0 or 1 |
| `notes` | Internal notes (not rendered) | "Used for FAANG applications" |

### 1.2 Add `summary_id` FK to `resumes`

```sql
-- Step 2: Add summary_id to resumes
ALTER TABLE resumes ADD COLUMN summary_id TEXT REFERENCES summaries(id) ON DELETE SET NULL;
```

### 1.3 Data Migration

Migrate existing `resumes.header` JSON data into `summaries` rows. The `header` column currently stores a JSON string with `ResumeHeader` fields. We extract the role-specific fields; contact fields are ignored here (they will come from Profile).

**Data migration MUST NOT be done in SQL.** A pure-SQL approach using a correlated subquery (matching on `title`/`role`/`created_at`) is fragile: `NULL target_role` causes `NULL = NULL` which evaluates to FALSE, silently dropping those resumes from the UPDATE. Instead, data migration is handled by a TypeScript helper.

**TypeScript migration helper** (`packages/core/src/db/migrations/006_summaries_data.ts`):

1. **Step 1 — INSERT:** SELECT each resume row where `header IS NOT NULL`. For each row, generate a UUID in TypeScript, extract `tagline` from the header JSON, and INSERT into `summaries` with the generated UUID.
2. **Step 2 — UPDATE:** UPDATE `resumes.summary_id` with the generated UUID for that resume row (keyed by `resumes.id`, not by any composite join).

This two-step approach guarantees every resume (including those with `NULL target_role`) gets a linked summary.

The SQL migration file contains only DDL (Steps 1-2 above: CREATE TABLE, ALTER TABLE) and the migration registration:

```sql
-- Step 3: Register migration
INSERT INTO _migrations (name) VALUES ('006_summaries');
```

**Note:** The `resumes.header` column is NOT dropped in this migration. It is retained as a legacy fallback until the IR compiler is updated to read from `summaries` + Profile. A future migration will drop it.

### 1.4 Files to Create

| File | Purpose |
|------|---------|
| `packages/core/src/db/migrations/006_summaries.sql` | DDL only: CREATE TABLE summaries, ALTER TABLE resumes ADD COLUMN summary_id, INSERT INTO _migrations |
| `packages/core/src/db/migrations/006_summaries_data.ts` | TypeScript data migration helper: reads each resume with non-null header, generates UUID, INSERTs summary, UPDATEs resume.summary_id |
| `packages/core/src/db/repositories/summary-repository.ts` | SummaryRepository with CRUD + clone |
| `packages/core/src/routes/summaries.ts` | API route handlers for summaries CRUD |

---

## 2. API Endpoints

All endpoints follow the existing Forge API conventions (JSON envelope, `/api/` prefix, Hono router).

### 2.1 Summaries CRUD

| Method | Path | Description | Request Body | Response |
|--------|------|-------------|-------------|----------|
| `POST` | `/api/summaries` | Create a summary | `CreateSummary` | `Result<Summary>` |
| `GET` | `/api/summaries` | List summaries | Query: `?is_template=0&limit=50&offset=0` | `PaginatedResult<Summary>` |
| `GET` | `/api/summaries/:id` | Get a summary | - | `Result<Summary>` |
| `PATCH` | `/api/summaries/:id` | Update a summary | `UpdateSummary` | `Result<Summary>` |
| `DELETE` | `/api/summaries/:id` | Delete a summary | - | `Result<void>` |
| `POST` | `/api/summaries/:id/clone` | Clone a summary | - | `Result<Summary>` |

**Clone endpoint behavior:** The `POST /api/summaries/:id/clone` endpoint creates a copy with `title = 'Copy of ' + original.title` and `is_template = 0` (always, even if cloning a template). All other fields (`role`, `tagline`, `description`, `notes`) are copied verbatim. A new UUID, `created_at`, and `updated_at` are generated. Returns `201 Created` on success. If the source summary ID is not found, returns `404` with error code `SUMMARY_NOT_FOUND`.

### 2.2 Resume-Summary Linking

The existing `PATCH /api/resumes/:id` endpoint accepts `summary_id` in the update body. No new endpoint needed.

**File to modify:** `packages/core/src/routes/resumes.ts` -- the `PATCH /api/resumes/:id` handler must read and forward `summary_id` from the request body to `ResumeRepository.update()`.

### 2.3 Linked Resumes (future endpoint)

`GET /api/summaries/:id/linked-resumes` -- returns resumes that reference this summary via `summary_id`. Required by Spec 3 (Summary Templates) for the 'also used by' warning. Can be deferred until Spec 3 implementation.

**Response type:** `PaginatedResult<Resume>` with standard pagination params (`?limit=50&offset=0`). When implemented, must use `PaginatedResult<Resume>` with standard pagination params to avoid unbounded result sets.

### 2.4 Route File

New file: `packages/core/src/routes/summaries.ts`

Follows the same pattern as `packages/core/src/routes/organizations.ts`.

---

## 3. UI Changes

### 3.1 Summaries View (`/data/summaries`)

Located under Data > Summaries in the restructured nav (Spec 1).

**Tab bar:** Description | Title | Role | Contact Info (read-only stub)

**Description tab (default):**
- List of summaries with title, role, tagline preview, template badge
- Click to expand inline editor with description textarea
- "New Summary" button opens a creation form
- "Clone" button on each summary

**Title tab:**
- Same list, but the editable field is `title` (the internal label)
- Useful for batch-renaming summaries

**Role tab:**
- Same list, but the editable field is `role`
- Useful for seeing what role each summary targets

**Contact Info tab (read-only stub):**
- Render stub if `GET /api/profile` returns 404 or the profile entity does not exist; render full Contact Info if profile data is available
- Read-only view showing "Edit in Profile" link until Spec 6 (Config Profile) is implemented
- Once Spec 6 (Config Profile) is complete: shows global Profile contact info (name, email, phone, linkedin, github, website, clearance)
- "Edit Profile" link navigates to `/config/profile`
- Informational: "Contact info is shared across all resumes via your Profile."

### 3.2 Resume Creation Flow

When creating a new resume:

1. Existing fields: name, target_role, target_employer, archetype
2. New step: "Pick Summary" — shows a list of existing summaries (templates first, then instances)
3. Options:
   - Pick an existing summary (links it directly)
   - Pick a template (clones it, links the clone)
   - "Create New" (opens inline summary creation form)
   - "Skip" (create resume without a summary, can add later)

### 3.3 Resume Detail View

- Summary section shows the linked summary's role, tagline, description
- "Change Summary" button opens the picker
- "Edit Summary" button navigates to the summary's detail view
- "Detach Summary" button sets `summary_id = NULL`

---

## 4. Type Changes

### 4.1 Core Types (`packages/core/src/types/index.ts`)

```typescript
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

Add `summary_id` to existing types:

```typescript
// In Resume interface:
summary_id: string | null

// In CreateResume interface:
summary_id?: string

// In UpdateResume interface:
summary_id?: string | null
```

#### 4.1.1 `ResumeRow` and `ResumeRepository.update()` Changes

Also update `ResumeRow` interface in `packages/core/src/db/repositories/resume-repository.ts`:

- Add `summary_id: string | null` to `ResumeRow`
- Add a `summary_id` case to the `update()` method's field-building loop so that `PATCH /api/resumes/:id` with `{ summary_id: '<uuid>' }` correctly generates `SET summary_id = ?` in the UPDATE statement

### 4.2 SDK Types (`packages/sdk/src/types.ts`)

Mirror the core types with SDK conventions (boolean instead of integer for `is_template`):

```typescript
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

export interface CreateSummary {
  title: string
  role?: string
  tagline?: string
  description?: string
  is_template?: boolean
  notes?: string
}

export interface UpdateSummary {
  title?: string
  role?: string | null
  tagline?: string | null
  description?: string | null
  is_template?: boolean
  notes?: string | null
}

export interface SummaryFilter {
  is_template?: boolean
}

// SummariesResource.list() method signature:
//   list(filter?: SummaryFilter, pagination?: PaginationParams): Promise<PaginatedResult<Summary>>
//
// `is_template` goes in SummaryFilter (domain filter).
// `limit` and `offset` go in PaginationParams (standard pagination).
//
// The SDK serializes `is_template: true` to query param `?is_template=1`
// via `toParams()`. The filter type uses `boolean` for developer ergonomics;
// serialization handles the conversion.
```

Add to `Resume`: `summary_id: string | null`
Add to `CreateResume`: `summary_id?: string`
Add to `UpdateResume`: `summary_id?: string | null`

### 4.3 IR Compiler Changes

The `ResumeHeader` interface remains unchanged. The IR compiler is updated to build `ResumeHeader` from two sources:

1. **Summary** (via `resume.summary_id` -> `summaries` row): provides `tagline`
2. **Profile** (global config, future spec): provides `name`, `email`, `phone`, `linkedin`, `github`, `website`, `location`, `clearance`

Until the Profile spec is implemented, the compiler falls back to `resumes.header` JSON for contact fields.

### 4.4 `parseHeader()` Evolution — Final Target State

The `parseHeader` function is extended in two phases:

1. **Spec 6 (Config Profile, migration 005):** Extends to `parseHeader(resume, profile)` -- contact fields from profile
2. **This spec (Summaries, migration 006):** Extends to `parseHeader(resume, profile, summary)` -- tagline from summary

**Intermediate signature if shipping before Spec 6:** This spec CAN ship before Spec 6 (Config Profile). If it does, `parseHeader` should use the intermediate signature `parseHeader(resume, summary)` where summary provides the tagline and contact fields continue to fall back to `resumes.header` JSON. Spec 6 later extends this to `parseHeader(resume, profile, summary)`.

**If Spec 6 ships first:** The IR compiler already accepts a `profile` parameter, and this spec extends it to `parseHeader(resume, profile, summary)`.

The final state: `parseHeader(resume, profile, summary)` -- tagline comes from `summary.tagline`.

```typescript
// Final state after both Profile (005) and Summaries (006) are implemented:
function parseHeader(
  resume: ResumeRow,
  profile: UserProfile | null,
  summary: SummaryRow | null
): ResumeHeader {
  return {
    name: profile?.name ?? resume.name,
    tagline: summary?.tagline ?? resume.target_role,
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

---

## 5. Acceptance Criteria

1. **`summaries` table** exists after migration 006 with correct schema
2. **Existing header data** migrated: each resume with a non-null `header` has a linked summary
3. **CRUD operations** work: create, list, get, update, delete summaries via API
4. **Clone endpoint** creates a new summary with identical fields and `is_template = 0`
5. **Summaries view** renders at `/data/summaries` with 4 tabs (Contact Info tab is a read-only stub showing 'Edit in Profile' link until Spec 6 (Config Profile) is implemented)
6. **Resume creation** includes summary picker step
7. **Resume detail** shows linked summary with edit/change/detach actions
8. **SDK resource** `SummariesResource` with `create`, `list`, `get`, `update`, `delete`, `clone` methods
9. **`resumes.header` column** retained as fallback (not dropped)
10. **IR compiler** reads from summary + falls back to header JSON for contact info
11. **`summary_id` round-trip:** Verify `PATCH /api/resumes/:id` with `{ summary_id: '<uuid>' }` updates `resumes.summary_id` and the returned Resume object reflects the change

---

## 6. Dependencies & Parallelization

### Dependencies

| Dependency | Required For | Blocking? |
|-----------|-------------|-----------|
| Migration 005 (user_profile, Spec 6) | Migration sequence ordering in `_migrations` table | Soft (006's DDL is schema-independent of 005; the dependency is only on `_migrations` table ordering) |
| Nav Restructuring (Spec 1) | `/data/summaries` route | Soft (can develop view independently) |
| Config -- Profile (Spec 6) | Contact Info tab content, IR compiler contact fields | Soft (contact info falls back to `resumes.header` JSON until Spec 6 is implemented) |

### Parallelization

| Stream | Description | Can run in parallel |
|--------|-------------|-------------------|
| A | Migration 006 SQL + TS data migration helper | Independent (006's DDL does not depend on 005's schema; only `_migrations` ordering) |
| B | `SummaryRepository` in core | After A |
| C | API routes (`summaries.ts`) | After B |
| D | SDK `SummariesResource` | After C's interface is defined |
| E | Summaries view (UI) | After D |
| F | Resume creation flow updates | After B + E |
| G | IR compiler updates | After B, parallel with E/F |

---

## 7. Testing

- Verify TS data migration helper correctly links summaries for all resumes (no mis-links on name collision)
- Verify NULL `target_role` resumes receive a linked summary after migration
- Verify migrated summaries for resumes with NULL `target_role` are correctly linked (summary.role is NULL, summary_id is set)
- Verify `GET /api/summaries` without filter returns all (templates + instances)
- Verify `GET /api/summaries` with `?is_template=1` returns only templates
- Verify `PATCH /api/resumes/:id` with `summary_id: null` detaches summary
- Verify `PATCH /api/resumes/:id` with `{ summary_id: '<uuid>' }` updates `resumes.summary_id` and the returned Resume object reflects the change
- Verify `PATCH /api/resumes/:id` with `{ summary_id: 'nonexistent' }` returns FK violation error
- Verify delete summary sets resume `summary_id` to NULL (ON DELETE SET NULL)
- Verify `compileResumeIR()` falls back to `resumes.header` when `summary_id = NULL`
- Verify IR compiler uses `summary.tagline` over header JSON's tagline when both exist
- Verify clone endpoint produces 'Copy of ' title with `is_template = 0`
- Verify clone of non-existent summary ID returns 404 with code `SUMMARY_NOT_FOUND`

---

## 8. Known Limitations

1. **No Profile entity yet** — contact info fields (name, email, phone, linkedin, github, website, clearance) are NOT in the summaries table. The Contact Info tab on the Summaries view is a read-only placeholder until the Profile spec is implemented. Until then, the IR compiler falls back to `resumes.header` JSON for these fields.
2. **Migration data fidelity** — the migration extracts `tagline` from the header JSON but cannot extract `description` (it was never stored in `header`). Migrated summaries will have `description = NULL` and need manual editing.
3. **`resumes.header` is not dropped** — the column remains to avoid breaking existing IR compilation. It will be removed in a future migration once the compiler is fully updated.
4. **No AI-assisted summary generation** — users must write summaries manually for now.
5. **No summary-to-archetype linking** — summaries are not formally linked to archetypes, though the `role` field serves a similar purpose informally.
6. **Clone does not deep-copy** — cloning a summary is a shallow copy (all scalar fields). If summaries gain related entities in the future, clone logic must be updated.
