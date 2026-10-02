# JD ↔ Resume Linkage

**Date:** 2026-04-03
**Spec:** E2 (JD Resume Linkage)
**Phase:** TBD (next available)
**Builds on:** Spec E1 (JD Detail Page — migration 018, JD skills, split-panel CRUD UI)
**Dependencies:** Spec E1 must be complete (JD detail page with skill tagging exists)
**Blocks:** None

## Overview

Job descriptions and resumes have a many-to-many relationship: one resume can target multiple JDs, and one JD can have multiple resume versions submitted or in progress. Currently, users must mentally track which resume was built for which JD. This spec adds a `job_description_resumes` junction table and the API/UI to link them.

From the JD detail page, a "Linked Resumes" section shows which resumes are associated with the JD, with a picker to add more. From the resume builder, a "Targeted JDs" section shows which JDs this resume is linked to. The linking is manual and bidirectional — either side can create or remove the association.

## Non-Goals

- Auto-generating tailored resumes from JD skills
- Cover letter generation
- Application submission or tracking (status tracking remains on the JD entity itself)
- Skill gap analysis between JD required skills and resume content
- Resume ranking or scoring against a JD
- Auto-linking based on matching skills/archetypes
- Duplicate link detection (the PRIMARY KEY constraint handles this)
- Link ordering or priority (no position field)

---

## 1. Schema Changes

### 1.1 Migration: `019_job_description_resumes.sql`

A new junction table for linking resumes to job descriptions. Follows the same pattern as `job_description_skills` (migration 018), `source_skills` (migration 016), and `bullet_skills` (migration 001).

```sql
-- Job Description Resumes Junction
-- Migration: 019_job_description_resumes
-- Links resumes to job descriptions for application tracking.
-- Many-to-many: one resume can target multiple JDs, one JD can have multiple resume versions.

CREATE TABLE job_description_resumes (
  job_description_id TEXT NOT NULL REFERENCES job_descriptions(id) ON DELETE CASCADE,
  resume_id TEXT NOT NULL REFERENCES resumes(id) ON DELETE CASCADE,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  PRIMARY KEY (job_description_id, resume_id)
) STRICT;

CREATE INDEX idx_jd_resumes_jd ON job_description_resumes(job_description_id);
CREATE INDEX idx_jd_resumes_resume ON job_description_resumes(resume_id);

INSERT INTO _migrations (name) VALUES ('019_job_description_resumes');
```

### 1.2 Cascade Behavior

- Deleting a JD cascades to `job_description_resumes` rows (the links are removed, the resumes are NOT deleted)
- Deleting a resume cascades to `job_description_resumes` rows (the links are removed, the JDs are NOT deleted)
- This is consistent with all other junction tables in Forge (source_skills, bullet_skills, job_description_skills)

### 1.3 `created_at` Column

The `created_at` timestamp records when the link was created, allowing UI to sort by "most recently linked" and providing an audit trail. The other junction tables (source_skills, bullet_skills, job_description_skills) do not have `created_at`, but the JD-resume relationship benefits from knowing when a resume was associated with a JD (e.g., "linked this resume on 2026-04-01, applied 2026-04-02").

---

## 2. API Endpoints

### 2.1 JD-Side Endpoints

Add to `packages/core/src/routes/job-descriptions.ts`, after the existing JD skills endpoints:

| Method | Path | Description | Request Body | Response |
|--------|------|-------------|-------------|----------|
| `GET` | `/api/job-descriptions/:id/resumes` | List resumes linked to this JD | -- | `{ data: ResumeLink[] }` |
| `POST` | `/api/job-descriptions/:id/resumes` | Link a resume to this JD | `{ resume_id: string }` | `{ data: ResumeLink }` (201) |
| `DELETE` | `/api/job-descriptions/:jdId/resumes/:resumeId` | Unlink a resume from a JD | -- | 204 |

### 2.2 Resume-Side Endpoint (Reverse Lookup)

Add to `packages/core/src/routes/resumes.ts`:

| Method | Path | Description | Request Body | Response |
|--------|------|-------------|-------------|----------|
| `GET` | `/api/resumes/:id/job-descriptions` | List JDs linked to this resume | -- | `{ data: JDLink[] }` |

### 2.3 Response Types

**`ResumeLink`** — returned by `GET /api/job-descriptions/:id/resumes`:

```typescript
interface ResumeLink {
  resume_id: string
  resume_name: string
  target_role: string
  target_employer: string
  archetype: string
  status: ResumeStatus
  created_at: string         // from junction table — when the link was created
  resume_created_at: string  // from resumes table — when the resume was created
}
```

The response JOINs `resumes` to include display fields. This avoids a second round-trip to fetch resume details for the linked resumes list.

**`JDLink`** — returned by `GET /api/resumes/:id/job-descriptions`:

```typescript
interface JDLink {
  job_description_id: string
  title: string
  organization_name: string | null
  status: JobDescriptionStatus
  location: string | null
  salary_range: string | null
  created_at: string         // from junction table — when the link was created
  jd_created_at: string      // from job_descriptions table
}
```

The response JOINs `job_descriptions` and `organizations` to include display fields.

### 2.4 POST Behavior

```sql
INSERT OR IGNORE INTO job_description_resumes (job_description_id, resume_id)
VALUES (?, ?)
```

- If the link already exists, `INSERT OR IGNORE` makes it idempotent (no error, returns the existing link data)
- Validates that both the JD and resume exist before inserting (return 404 if either is missing)
- Returns 201 on successful creation

> **Note:** On duplicate (INSERT OR IGNORE with `changes = 0`), do a SELECT to return the existing link data. Return 200 (not 201) for duplicates.

### 2.5 DELETE Behavior

```sql
DELETE FROM job_description_resumes
WHERE job_description_id = ? AND resume_id = ?
```

- Returns 204 regardless of whether the row existed (idempotent)

### 2.6 GET Behavior (JD Side)

```sql
SELECT r.id AS resume_id, r.name AS resume_name, r.target_role, r.target_employer,
       r.archetype, r.status, jdr.created_at, r.created_at AS resume_created_at
FROM job_description_resumes jdr
JOIN resumes r ON r.id = jdr.resume_id
WHERE jdr.job_description_id = ?
ORDER BY jdr.created_at DESC
```

### 2.7 GET Behavior (Resume Side)

```sql
SELECT jd.id AS job_description_id, jd.title, o.name AS organization_name,
       jd.status, jd.location, jd.salary_range,
       jdr.created_at, jd.created_at AS jd_created_at
FROM job_description_resumes jdr
JOIN job_descriptions jd ON jd.id = jdr.job_description_id
LEFT JOIN organizations o ON o.id = jd.organization_id
WHERE jdr.resume_id = ?
ORDER BY jdr.created_at DESC
```

---

## 3. SDK Changes

### 3.1 JobDescriptionsResource (Existing File)

Add methods to `packages/sdk/src/resources/job-descriptions.ts`:

```typescript
/** List resumes linked to a JD. */
listResumes(jdId: string): Promise<Result<ResumeLink[]>> {
  return this.request<ResumeLink[]>('GET', `/api/job-descriptions/${jdId}/resumes`)
}

/** Link a resume to a JD. */
linkResume(jdId: string, resumeId: string): Promise<Result<ResumeLink>> {
  return this.request<ResumeLink>(
    'POST',
    `/api/job-descriptions/${jdId}/resumes`,
    { resume_id: resumeId },
  )
}

/** Unlink a resume from a JD. */
unlinkResume(jdId: string, resumeId: string): Promise<Result<void>> {
  return this.request<void>(
    'DELETE',
    `/api/job-descriptions/${jdId}/resumes/${resumeId}`,
  )
}
```

### 3.2 ResumesResource (Existing File)

Add to `packages/sdk/src/resources/resumes.ts`:

```typescript
/** List JDs linked to a resume. */
listJobDescriptions(resumeId: string): Promise<Result<JDLink[]>> {
  return this.request<JDLink[]>('GET', `/api/resumes/${resumeId}/job-descriptions`)
}
```

### 3.3 SDK Types

Add to `packages/sdk/src/types.ts`:

```typescript
export interface ResumeLink {
  resume_id: string
  resume_name: string
  target_role: string
  target_employer: string
  archetype: string
  status: ResumeStatus
  created_at: string
  resume_created_at: string
}

export interface JDLink {
  job_description_id: string
  title: string
  organization_name: string | null
  status: JobDescriptionStatus
  location: string | null
  salary_range: string | null
  created_at: string
  jd_created_at: string
}
```

---

## 4. UI: JD Detail Page — "Linked Resumes" Section

### 4.1 Placement

Add a "Linked Resumes" section to the JD editor panel (`JDEditor.svelte`), below the "Required Skills" section and above the "Notes" section. This section is only visible in edit mode (not create mode — a JD must exist before resumes can be linked).

### 4.2 Layout

```
Linked Resumes:
┌─────────────────────────────────────────────────┐
│ DevSecOps Engineer - Cloudflare         [draft] │
│ target: DevSecOps Engineer @ Cloudflare     [×] │
├─────────────────────────────────────────────────┤
│ Security Engineer - Cloudflare       [approved] │
│ target: Security Engineer @ Cloudflare      [×] │
└─────────────────────────────────────────────────┘
[+ Link Resume]
```

### 4.3 Resume Cards

Each linked resume is displayed as a compact card showing:
- **Resume name** (bold)
- **Status badge** (draft/final per the current schema)

> **Note:** Resume status badges show `draft` or `final` per the current schema. The 5-status model (`draft | in_review | approved | rejected | archived`) is added by Spec D's migration — if D lands before E2, use the expanded statuses.
- **Target role @ target employer** (muted text)
- **Unlink button** (x) on the right — clicking calls `DELETE /api/job-descriptions/:jdId/resumes/:resumeId` and removes the card from the list

### 4.4 "Link Resume" Flow

Clicking "+ Link Resume" opens a resume picker dropdown/modal:

```
┌─ Link Resume ────────────────────── × ─┐
│                                         │
│ [Search resumes...                    ] │
│                                         │
│ ┌─────────────────────────────────────┐ │
│ │ Platform Engineer - Google [draft]  │ │
│ │ Platform Engineer @ Google          │ │
│ ├─────────────────────────────────────┤ │
│ │ SRE Lead - AWS             [final]  │ │
│ │ SRE Lead @ Amazon                   │ │
│ └─────────────────────────────────────┘ │
│                                         │
└─────────────────────────────────────────┘
```

**Behavior:**
- Fetches all resumes via `forge.resumes.list({ limit: 200 })`
- Filters out resumes already linked to this JD (compare against `linkedResumes` state)
- Search input filters by resume name, target_role, and target_employer (client-side)
- Clicking a resume calls `POST /api/job-descriptions/:id/resumes { resume_id }` and adds it to the linked list
- Picker closes after selection (single-select per click; user clicks again to add more)

### 4.5 State Management

```typescript
// In JDEditor.svelte (or a sub-component)
let linkedResumes = $state<ResumeLink[]>([])
let showResumePicker = $state(false)
let allResumes = $state<Resume[]>([])   // for the picker
let resumeSearch = $state('')

// Load on JD selection
async function loadLinkedResumes(jdId: string) {
  const result = await forge.jobDescriptions.listResumes(jdId)
  if (result.ok) linkedResumes = result.data
}

// Filter for picker (exclude already-linked)
let availableResumes = $derived(
  allResumes
    .filter(r => !linkedResumes.some(lr => lr.resume_id === r.id))
    .filter(r =>
      !resumeSearch ||
      r.name.toLowerCase().includes(resumeSearch.toLowerCase()) ||
      r.target_role.toLowerCase().includes(resumeSearch.toLowerCase()) ||
      r.target_employer.toLowerCase().includes(resumeSearch.toLowerCase())
    )
)
```

---

## 5. UI: Resume Builder — "Targeted JDs" Section

### 5.1 Placement

Add a "Targeted JDs" section to the resume editor/detail view. The exact component depends on which resume page is being modified. If the resume page uses a split-panel layout (like the JD page), add the section to the editor panel. If it uses a modal, add a tab or section to the modal.

### 5.2 Layout

```
Targeted Job Descriptions:
┌─────────────────────────────────────────────────┐
│ Sr Security Engineer                 [applied]  │
│ Cloudflare • Remote • $180k-$220k          [×]  │
├─────────────────────────────────────────────────┤
│ DevOps Lead                       [interested]  │
│ Acme Corp • NYC • $150k                    [×]  │
└─────────────────────────────────────────────────┘
[+ Link JD]
```

### 5.3 JD Cards

Each linked JD is displayed as a compact card showing:
- **JD title** (bold)
- **Status badge** (interested/analyzing/applied/etc.)
- **Organization name** (if present, muted)
- **Location** (if present, muted)
- **Salary range** (if present, muted)
- **Unlink button** (x) — calls `DELETE /api/job-descriptions/:jdId/resumes/:resumeId`

### 5.4 "Link JD" Flow

Clicking "+ Link JD" opens a JD picker (same pattern as the resume picker):
- Fetches all JDs via `forge.jobDescriptions.list({ limit: 200 })`
- Filters out JDs already linked
- Search by title and organization_name
- Clicking a JD calls `POST /api/job-descriptions/:jdId/resumes { resume_id }` (note: the POST is on the JD side, but the SDK call can be made from either direction)

### 5.5 State Management

```typescript
let linkedJDs = $state<JDLink[]>([])
let showJDPicker = $state(false)
let allJDs = $state<JobDescriptionWithOrg[]>([])
let jdSearch = $state('')

async function loadLinkedJDs(resumeId: string) {
  const result = await forge.resumes.listJobDescriptions(resumeId)
  if (result.ok) linkedJDs = result.data
}

let availableJDs = $derived(
  allJDs
    .filter(jd => !linkedJDs.some(lj => lj.job_description_id === jd.id))
    .filter(jd =>
      !jdSearch ||
      jd.title.toLowerCase().includes(jdSearch.toLowerCase()) ||
      jd.organization_name?.toLowerCase().includes(jdSearch.toLowerCase())
    )
)
```

---

## 6. Component Architecture

### 6.1 New Components

| Component | File | Purpose |
|-----------|------|---------|
| `JDLinkedResumes.svelte` | `packages/webui/src/lib/components/jd/JDLinkedResumes.svelte` | "Linked Resumes" section for JD editor, including picker |
| `ResumeLinkedJDs.svelte` | `packages/webui/src/lib/components/resume/ResumeLinkedJDs.svelte` | "Targeted JDs" section for resume editor, including picker |
| `ResumePickerModal.svelte` | `packages/webui/src/lib/components/jd/ResumePickerModal.svelte` | Modal for selecting a resume to link |
| `JDPickerModal.svelte` | `packages/webui/src/lib/components/resume/JDPickerModal.svelte` | Modal for selecting a JD to link |

### 6.2 Component Props

```typescript
// JDLinkedResumes.svelte
let { jdId, forge }: {
  jdId: string
  forge: ForgeClient
} = $props()

// ResumeLinkedJDs.svelte
let { resumeId, forge }: {
  resumeId: string
  forge: ForgeClient
} = $props()

// ResumePickerModal.svelte
let { excludeIds, onselect, onclose }: {
  excludeIds: string[]     // resume IDs already linked
  onselect: (resumeId: string) => void
  onclose: () => void
} = $props()

// JDPickerModal.svelte
let { excludeIds, onselect, onclose }: {
  excludeIds: string[]     // JD IDs already linked
  onselect: (jdId: string) => void
  onclose: () => void
} = $props()
```

---

## 7. Files to Create

| File | Purpose |
|------|---------|
| `packages/core/src/db/migrations/019_job_description_resumes.sql` | Junction table for JD-to-resume linking |
| `packages/webui/src/lib/components/jd/JDLinkedResumes.svelte` | "Linked Resumes" section in JD editor |
| `packages/webui/src/lib/components/jd/ResumePickerModal.svelte` | Resume picker modal for linking |
| `packages/webui/src/lib/components/resume/ResumeLinkedJDs.svelte` | "Targeted JDs" section in resume editor |
| `packages/webui/src/lib/components/resume/JDPickerModal.svelte` | JD picker modal for linking |

## 8. Files to Modify

| File | Change |
|------|--------|
| `packages/core/src/routes/job-descriptions.ts` | Add `GET/POST /:id/resumes`, `DELETE /:jdId/resumes/:resumeId` endpoints |
| `packages/core/src/routes/resumes.ts` | Add `GET /:id/job-descriptions` endpoint |
| `packages/sdk/src/resources/job-descriptions.ts` | Add `listResumes(jdId)`, `linkResume(jdId, resumeId)`, `unlinkResume(jdId, resumeId)` methods |
| `packages/sdk/src/resources/resumes.ts` | Add `listJobDescriptions(resumeId)` method |
| `packages/sdk/src/types.ts` | Add `ResumeLink` and `JDLink` interfaces |
| `packages/core/src/types/index.ts` | Add `ResumeLink` and `JDLink` interfaces |
| `packages/webui/src/lib/components/jd/JDEditor.svelte` | Mount `JDLinkedResumes` section in edit mode |
| Resume editor component (TBD based on current layout) | Mount `ResumeLinkedJDs` section |

---

## 9. Testing

### 9.1 Migration Tests

- Migration 019 creates `job_description_resumes` table with correct schema
- `INSERT INTO job_description_resumes` with valid JD and resume IDs succeeds
- `INSERT INTO job_description_resumes` with duplicate pair is rejected (PRIMARY KEY constraint)
- Deleting a JD cascades to `job_description_resumes` rows (resume survives)
- Deleting a resume cascades to `job_description_resumes` rows (JD survives)
- `created_at` defaults to current timestamp on insert

### 9.2 API Tests (JD Side)

- `GET /api/job-descriptions/:id/resumes` returns empty array for JD with no linked resumes
- `POST /api/job-descriptions/:id/resumes` with `{ resume_id }` links a resume (201)
- `POST /api/job-descriptions/:id/resumes` with duplicate `{ resume_id }` is idempotent (INSERT OR IGNORE)
- `POST /api/job-descriptions/:id/resumes` with already-linked `resume_id` returns 200 with existing link data (idempotent)
- `POST /api/job-descriptions/:id/resumes` with nonexistent `resume_id` returns 404
- `POST /api/job-descriptions/:id/resumes` for nonexistent JD returns 404
- `POST /api/job-descriptions/:id/resumes` with missing `resume_id` returns 400
- `DELETE /api/job-descriptions/:jdId/resumes/:resumeId` removes the junction row (204)
- `DELETE /api/job-descriptions/:jdId/resumes/:resumeId` for non-linked pair returns 204 (idempotent)
- `GET /api/job-descriptions/:id/resumes` returns correct resume data with JOINed fields (name, target_role, target_employer, archetype, status)
- `GET /api/job-descriptions/:id/resumes` returns results ordered by `created_at DESC`

### 9.3 API Tests (Resume Side)

- `GET /api/resumes/:id/job-descriptions` returns empty array for resume with no linked JDs
- `GET /api/resumes/:id/job-descriptions` returns correct JD data with JOINed fields (title, organization_name, status, location, salary_range)
- `GET /api/resumes/:id/job-descriptions` returns results ordered by `created_at DESC`
- `GET /api/resumes/:id/job-descriptions` for nonexistent resume returns 404

### 9.4 SDK Tests

- `forge.jobDescriptions.listResumes(jdId)` returns `{ data: ResumeLink[] }`
- `forge.jobDescriptions.linkResume(jdId, resumeId)` returns `{ data: ResumeLink }`
- `forge.jobDescriptions.unlinkResume(jdId, resumeId)` returns void
- `forge.resumes.listJobDescriptions(resumeId)` returns `{ data: JDLink[] }`

### 9.5 UI Component Tests

- JD editor shows "Linked Resumes" section in edit mode (not create mode)
- Linked resumes display name, status badge, target role/employer, and unlink button
- Clicking unlink (x) removes the resume from the list immediately
- "+ Link Resume" opens the resume picker modal
- Resume picker shows all resumes not already linked
- Resume picker search filters by name, target_role, target_employer
- Selecting a resume in the picker links it and closes the picker
- Resume editor shows "Targeted JDs" section
- Linked JDs display title, status badge, org name, location, salary range, and unlink button
- "+ Link JD" opens the JD picker modal
- JD picker shows all JDs not already linked
- JD picker search filters by title and organization_name
- Selecting a JD in the picker links it and closes the picker
- Both sections refresh their lists after link/unlink operations

### 9.6 Integration Tests

- Link resume R1 to JD J1 via JD side. Verify R1 appears in `GET /api/job-descriptions/J1/resumes`. Verify J1 appears in `GET /api/resumes/R1/job-descriptions`.
- Unlink from the resume side (via JD picker). Verify the link is removed from both sides.
- Delete JD J1. Verify `job_description_resumes` rows for J1 are gone. Verify R1 still exists.
- Delete resume R1. Verify `job_description_resumes` rows for R1 are gone. Verify J1 still exists.

---

## 10. Acceptance Criteria

1. Migration 019 creates `job_description_resumes` junction table with CASCADE deletes on both FKs
2. `POST /api/job-descriptions/:id/resumes` links a resume to a JD; idempotent on duplicate
3. `DELETE /api/job-descriptions/:jdId/resumes/:resumeId` unlinks a resume from a JD; idempotent
4. `GET /api/job-descriptions/:id/resumes` returns linked resumes with JOINed display fields
5. `GET /api/resumes/:id/job-descriptions` returns linked JDs with JOINed display fields (reverse lookup)
6. JD editor panel shows a "Linked Resumes" section (edit mode only) with resume cards and unlink buttons
7. "+ Link Resume" on JD page opens a picker showing available (unlinked) resumes, searchable by name/role/employer
8. Selecting a resume in the picker immediately links it and updates the UI
9. Resume editor shows a "Targeted JDs" section with JD cards and unlink buttons
10. "+ Link JD" on resume page opens a picker showing available (unlinked) JDs, searchable by title/org
11. Selecting a JD in the picker immediately links it and updates the UI
12. Unlinking from either side (JD or resume) removes the junction row; the other entity is unaffected
13. SDK `JobDescriptionsResource` has `listResumes`, `linkResume`, `unlinkResume` methods
14. SDK `ResumesResource` has `listJobDescriptions` method
15. Both pickers filter out already-linked entities from the selection list
16. Deleting a JD does not delete linked resumes (CASCADE on junction only)
17. Deleting a resume does not delete linked JDs (CASCADE on junction only)
