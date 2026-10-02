# Job Descriptions Entity

**Date:** 2026-03-30
**Status:** Design
**Builds on:** Organizations (Migration 002), Navigation Restructuring (Spec 1, 2026-03-30)

## Purpose

Provide a place to store job descriptions for target opportunities. Currently, Forge tracks organizations and can link them to sources/resumes, but there is no entity for the actual job posting. Users need to paste a JD, track its status (interested -> applied -> interviewing), and eventually match it against their skills and perspectives.

This is a minimal v1: paste text, save, track status. Parsing, matching, and suggestion features are deferred.

## Goals

1. `job_descriptions` table with title, org link, raw text, status, and metadata
2. CRUD API endpoints for job descriptions
3. Job Descriptions view under Opportunities > Job Descriptions
4. Status tracking through a defined pipeline
5. Organization linking (optional FK to `organizations`)
6. URL storage for the original posting

## Non-Goals

- JD parsing / NLP extraction of requirements, skills, experience levels
- Skill matching against user's skill inventory
- Perspective or archetype suggestion based on JD requirements
- API-based JD fetching (Indeed, LinkedIn scraping)
- Resume-to-JD gap analysis
- Cover letter generation from JD
- JD deduplication detection
- JD expiration / auto-close

---

## 1. Schema Changes (Migration 007)

### 1.1 `job_descriptions` Table

```sql
-- Forge Resume Builder — Job Descriptions Entity
-- Migration: 007_job_descriptions
-- Date: 2026-03-30
--
-- Adds a table for storing job descriptions linked to organizations.
-- Builds on 006_summaries.

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

**Note:** Migration 007 must rebuild the `note_references` table to add `'job_description'` to the `entity_type` CHECK constraint. The original CHECK constraint (in `002_schema_evolution.sql`) lists valid types as `('source', 'bullet', 'perspective', 'resume_entry', 'resume', 'skill', 'organization')`. SQLite does not support `ALTER TABLE ... ALTER CONSTRAINT`, so a table rebuild (create new table, copy data, drop old, rename) is required. The rebuild is wrapped in `PRAGMA foreign_keys = OFF / ON` to prevent FK violations during the table swap.

**Migration dependency note:** This migration (007) requires migrations 001-004 to have been applied. It is independent of migrations 005 (Profile, Spec 6) and 006 (Summaries, Spec 2) at the DDL level, but must be numbered sequentially in the `_migrations` table.

### 1.2 Column Semantics

| Column | Purpose | Example |
|--------|---------|---------|
| `title` | Job title from the posting | "Senior Security Engineer" |
| `organization_id` | FK to `organizations` (nullable) | Links to "Cloudflare" org |
| `url` | Link to the original posting | "https://boards.greenhouse.io/..." |
| `raw_text` | Full JD text, pasted by user | Multi-paragraph job description |
| `status` | Current stage in the opportunity pipeline | "applied" |
| `salary_range` | User-entered salary info | "$150k-$200k", "DOE", "GS-13" |
| `location` | Work location | "Remote", "San Francisco, CA", "Hybrid - DC" |
| `notes` | User's private notes | "Referred by John, hiring manager is..." |

### 1.3 Status Pipeline

```
interested ──> analyzing ──> applied ──> interviewing ──> offered
                                │              │              │
                                └──> withdrawn  └──> rejected  └──> rejected
                                └──> closed     └──> withdrawn └──> withdrawn
                                                └──> closed    └──> closed
```

**Status definitions:**

| Status | Meaning |
|--------|---------|
| `interested` | Found the posting, saved for review |
| `analyzing` | Actively reviewing JD, comparing against skills |
| `applied` | Application submitted |
| `interviewing` | In active interview process |
| `offered` | Received an offer |
| `rejected` | Rejected by employer (at any stage) |
| `withdrawn` | User withdrew (at any stage) |
| `closed` | Posting closed or role filled (not by user) |

**Transition rules:** Status transitions are NOT enforced at the database level. The CHECK constraint only validates the value is one of the allowed statuses. Any status can transition to any other status — the UI provides a dropdown, and the user is trusted to manage their pipeline.

---

## 2. API Endpoints

### 2.1 Job Descriptions CRUD

| Method | Path | Description | Request Body | Response |
|--------|------|-------------|-------------|----------|
| `POST` | `/api/job-descriptions` | Create a JD | `CreateJobDescription` | `Result<JobDescriptionWithOrg>` |
| `GET` | `/api/job-descriptions` | List JDs | Query: `?status=applied&organization_id=...&limit=50&offset=0` | `PaginatedResult<JobDescriptionWithOrg>` |
| `GET` | `/api/job-descriptions/:id` | Get a JD | - | `Result<JobDescriptionWithOrg>` |
| `PATCH` | `/api/job-descriptions/:id` | Update a JD | `UpdateJobDescription` | `Result<JobDescriptionWithOrg>` |
| `DELETE` | `/api/job-descriptions/:id` | Delete a JD | - | `Result<void>` |

### 2.2 Rich Response: JD with Organization

`GET /api/job-descriptions/:id` returns the JD with the linked organization name inlined:

```json
{
  "ok": true,
  "data": {
    "id": "...",
    "title": "Senior Security Engineer",
    "organization_id": "org-uuid",
    "organization_name": "Cloudflare",
    "url": "https://...",
    "raw_text": "...",
    "status": "applied",
    "salary_range": "$180k-$220k",
    "location": "Remote",
    "notes": "...",
    "created_at": "...",
    "updated_at": "..."
  }
}
```

The `organization_name` field is joined from `organizations.name` and included for display convenience. It is NOT stored in the `job_descriptions` table.

**Note:** The `GET /api/job-descriptions` list endpoint also JOINs `organizations` to include `organization_name` per item, same as the single GET. This allows the list view to display org names without a second round-trip.

### 2.3 List Filtering

The list endpoint supports these query parameters:

| Param | Type | Description |
|-------|------|-------------|
| `status` | `JobDescriptionStatus` | Filter by status (one of: 'interested', 'analyzing', 'applied', 'interviewing', 'offered', 'rejected', 'withdrawn', 'closed') |
| `organization_id` | string | Filter by organization |
| `limit` | number | Pagination limit (default 50) |
| `offset` | number | Pagination offset (default 0) |

### 2.4 Route File

New file: `packages/core/src/routes/job-descriptions.ts`

Follows the same pattern as `packages/core/src/routes/organizations.ts`.

### 2.5 Repository File

New file: `packages/core/src/db/repositories/job-description-repository.ts`

Follows the same pattern as `packages/core/src/db/repositories/organization-repository.ts`.

---

## 3. UI Changes

### 3.1 Job Descriptions List View (`/opportunities/job-descriptions`)

**Layout:** Card list with:
- Title (bold, large)
- Organization name (linked to org detail if exists)
- Status badge (colored pill)
- Location
- Salary range
- Created date
- Truncated notes preview

**Controls:**
- "New JD" button (top right)
- Status filter dropdown
- Organization filter dropdown
- Sort by: date (default), title, status

### 3.2 New JD Modal

"New JD" button opens a modal with:

```
Title:           [__________________________]
Organization:    [dropdown / autocomplete___]  (optional, can type new org name)
URL:             [__________________________]  (optional)
Salary Range:    [__________________________]  (optional)
Location:        [__________________________]  (optional)

Job Description: [                           ]
                 [                           ]
                 [     large textarea        ]
                 [                           ]
                 [                           ]

                              [Cancel]  [Save]
```

**Organization field behavior:**
- Autocomplete from existing organizations
- If typed name does not match, prompt: "Create new organization '[name]'?" → creates org on the fly
- Can be left blank

### 3.3 JD Detail View

Clicking a JD card expands it (or navigates to a detail panel) showing:

**Header:**
- Title (editable inline)
- Organization (link to org, editable via dropdown)
- URL (clickable link, editable)
- Status dropdown (immediate save on change)
- Salary range (editable inline)
- Location (editable inline)

**Body:**
- Full `raw_text` displayed in a readable format (preserving whitespace/paragraphs)
- "Edit" button switches to a textarea for editing the raw text
- Copy-to-clipboard button for the raw text

**Sidebar/Footer:**
- Notes textarea (auto-saves on blur)
- Created / Updated timestamps
- Delete button (with confirmation dialog)

### 3.4 Status Badge Colors

| Status | Color | Icon suggestion |
|--------|-------|----------------|
| `interested` | Gray | Bookmark |
| `analyzing` | Blue | Magnifying glass |
| `applied` | Indigo | Send/Arrow |
| `interviewing` | Purple | Calendar |
| `offered` | Green | Check circle |
| `rejected` | Red | X circle |
| `withdrawn` | Orange | Undo |
| `closed` | Dark gray | Lock |

---

## 4. Type Changes

### 4.1 Core Types (`packages/core/src/types/index.ts`)

**Note:** The base `JobDescription` type reflects the stored row. The `JobDescriptionWithOrg` type extends it with the computed `organization_name` field (joined from `organizations.name`). API responses and list endpoints return `JobDescriptionWithOrg`.

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
  organization_name: string | null   // joined from organizations.name, not stored
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

### 4.2 SDK Types (`packages/sdk/src/types.ts`)

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

/** Base stored row. */
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

/** JobDescription with computed organization_name from JOIN. Returned by API. */
export interface JobDescriptionWithOrg extends JobDescription {
  organization_name: string | null   // joined from organizations.name, not stored
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

### 4.3 SDK Resource Class

New file: `packages/sdk/src/resources/job-descriptions.ts`

```typescript
export class JobDescriptionsResource {
  constructor(
    private request: RequestFn,
    private requestList: RequestListFn,
  ) {}

  create(input: CreateJobDescription): Promise<Result<JobDescriptionWithOrg>> {
    return this.request<JobDescriptionWithOrg>('POST', '/api/job-descriptions', input)
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
    return this.request<JobDescriptionWithOrg>('GET', `/api/job-descriptions/${id}`)
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

### 4.4 SDK Client Registration

Add to `ForgeClient` class:

```typescript
jobDescriptions = new JobDescriptionsResource(this.request, this.requestList)
```

### 4.5 Note References

Extend the `NoteReference` entity type to include `'job_description'`. The `entity_type` field MUST use a union type (not raw `string`) to match the CHECK constraint in the database:

```typescript
// Shared union type for entity_type (used in both core and SDK)
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

```typescript
// In core types (packages/core/src/types/index.ts):
export interface NoteReference {
  note_id: string
  entity_type: NoteReferenceEntityType
  entity_id: string
}
```

```typescript
// In SDK types (packages/sdk/src/types.ts):
export interface NoteReference {
  entity_type: NoteReferenceEntityType
  entity_id: string
}
```

**Files to modify:** Both `packages/sdk/src/types.ts` and `packages/core/src/types/index.ts` must define `NoteReferenceEntityType` and update the `NoteReference` interface to use it instead of raw `string`.

---

## 5. Acceptance Criteria

1. **`job_descriptions` table** exists after migration 007 with correct schema and constraints
2. **CRUD operations** work: create, list, get, update, delete JDs via API
3. **Organization linking**: JD can reference an organization; `organization_name` is included in API responses
4. **Status tracking**: status field accepts all 8 valid values; status can be changed via PATCH
5. **List filtering**: `?status=applied` and `?organization_id=...` filters work correctly
6. **JD list view** renders at `/opportunities/job-descriptions` with title, org, status badge, date
7. **New JD modal** allows pasting full JD text with metadata fields
8. **JD detail view** shows full text with inline editing for all fields
9. **Status dropdown** on detail view saves immediately on change
10. **Organization autocomplete** on creation form searches existing orgs
11. **SDK resource** `JobDescriptionsResource` with all CRUD methods
12. **Delete confirmation** dialog before deleting a JD
13. **`ON DELETE SET NULL`** — deleting an organization sets `organization_id = NULL` on linked JDs, does not delete the JDs
14. **`ON DELETE SET NULL` tested**: deleting an org nulls JD `organization_id`
15. **List view handles `organization_id = null`** gracefully (shows 'No organization')
16. **CHECK constraint rejects** invalid status values

---

## 6. Dependencies & Parallelization

### Dependencies

| Dependency | Required For | Blocking? |
|-----------|-------------|-----------|
| Migration 006 (summaries) | Migration 007 builds on it sequentially | Yes (migration ordering) |
| Organizations table (Migration 002) | FK reference | Already exists |
| Nav Restructuring (Spec 1) | `/opportunities/job-descriptions` route | Soft (can develop view independently) |

### Parallelization

| Stream | Description | Can run in parallel |
|--------|-------------|-------------------|
| A | Migration 007 SQL | After migration 006 |
| B | `JobDescriptionRepository` in core | After A |
| C | API routes (`job-descriptions.ts`) | After B |
| D | SDK `JobDescriptionsResource` + types | Parallel with C (interface-first) |
| E | JD list view (UI) | After D |
| F | JD detail view (UI) | After E (or parallel if shared component patterns) |
| G | New JD modal with org autocomplete | After E |

**Note:** Streams B-D can be developed in parallel with Specs 1-3 since the `job_descriptions` table has no dependencies on `summaries` or nav changes -- only the migration ordering requires 006 to run first.

---

## 7. Testing

- Verify CRUD operations work: create, list, get, update, delete JDs via API
- Delete org with linked JDs -- verify JDs survive with `organization_id = NULL`
- Create JD with invalid status -- CHECK rejects
- List JDs filtered by both status AND organization_id
- PATCH JD to set `organization_id = null`
- Verify `organization_name` is included in both list and single GET responses
- Verify SDK `JobDescriptionsResource` all CRUD methods work
- Verify FK constraint prevents deleting an organization that has linked job descriptions (when not using ON DELETE SET NULL -- test the SET NULL behavior is correct)
- Verify listing JDs by `organization_id` returns only that org's JDs (no cross-org leakage)
- Verify adding `'job_description'` entity_type to `note_references` CHECK constraint works (insert a note_reference with entity_type = 'job_description' succeeds; insert with an invalid entity_type is rejected)

---

## 8. Known Limitations

1. **No JD parsing** — the raw text is stored as-is. There is no extraction of requirements, skills, experience levels, or keywords. This is a significant future feature that will enable skill matching and gap analysis.
2. **No skill matching** — the system cannot compare a JD's requirements against the user's skill inventory. This requires JD parsing first.
3. **No resume-to-JD linking** — there is no FK or junction table connecting resumes to job descriptions. Users must manually track which resume was submitted for which JD. A `resume_job_descriptions` junction table is a natural future addition.
4. **No JD fetching** — users must manually copy-paste JD text. API-based fetching from job boards (Indeed, LinkedIn, Greenhouse) is deferred.
5. **No status transition validation** — any status can change to any other status. The UI presents them in pipeline order, but the backend does not enforce transition rules.
6. **No duplicate detection** — pasting the same JD twice creates two separate rows. Deduplication (by URL or text similarity) is deferred.
7. **No salary parsing** — `salary_range` is a free-text field, not structured data. Comparison, filtering by range, or normalization is not supported.
8. **No archetype suggestion** — the system does not suggest which archetype or perspective set would be best for a given JD. This requires JD parsing + skill matching.
9. **Raw text formatting** — the `raw_text` field preserves whatever the user pastes. There is no HTML rendering, markdown conversion, or formatting normalization. Whitespace and line breaks are preserved as-is.
