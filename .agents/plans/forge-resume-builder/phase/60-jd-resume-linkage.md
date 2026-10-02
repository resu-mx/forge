# Phase 60: JD Resume Linkage (Spec E2)

**Status:** Planning
**Date:** 2026-04-03
**Spec:** [2026-04-03-jd-resume-linkage.md](../refs/specs/2026-04-03-jd-resume-linkage.md)
**Depends on:** Phase 49 (JD Detail Page -- migration 018, JD skills, split-panel CRUD UI)
**Blocks:** None
**Parallelizable with:** Phase 61 (JD Kanban Pipeline), Phase 62 (JD Skill Extraction AI) -- no file conflicts except shared modifications to `job-descriptions.ts` routes and SDK resource (serialize carefully)

## Goal

Add a `job_description_resumes` junction table (migration 019) with API endpoints for linking resumes to job descriptions, SDK methods for both sides of the relationship, and UI components on both the JD editor ("Linked Resumes" section) and resume editor ("Targeted JDs" section). The linking is manual, bidirectional, and idempotent. Either side can create or remove associations.

## Non-Goals

- Auto-generating tailored resumes from JD skills
- Cover letter generation
- Application submission or tracking (status tracking remains on the JD entity itself)
- Skill gap analysis between JD required skills and resume content
- Resume ranking or scoring against a JD
- Auto-linking based on matching skills/archetypes
- Duplicate link detection (the PRIMARY KEY constraint handles this)
- Link ordering or priority (no position field)

## Context

Phase 49 builds the JD detail page with a split-panel layout, skill tagging (migration 018 `job_description_skills`), and full CRUD. This phase adds the many-to-many relationship between JDs and resumes. The junction table pattern follows `job_description_skills` (018), `source_skills` (016), and `bullet_skills` (001). The `created_at` column on the junction is unique to this table -- other junction tables lack it -- but the JD-resume relationship benefits from knowing when a resume was associated with a JD.

## Scope

| Spec section | Covered here? |
|-------------|---------------|
| 1. Schema changes (migration 019: `job_description_resumes`) | Yes |
| 2. API endpoints (JD-side: GET/POST/DELETE, resume-side: GET) | Yes |
| 3. SDK changes (`listResumes`, `linkResume`, `unlinkResume`, `listJobDescriptions`) | Yes |
| 4. UI: JD editor "Linked Resumes" section + ResumePickerModal | Yes |
| 5. UI: Resume editor "Targeted JDs" section + JDPickerModal | Yes |
| 6. Component architecture | Yes |
| 7. Files to create | Yes |
| 8. Files to modify | Yes |
| 9. Testing | Yes |
| 10. Acceptance criteria | Yes |

## Files to Create

| File | Description |
|------|-------------|
| `packages/core/src/db/migrations/019_job_description_resumes.sql` | Junction table for JD-to-resume linking with `created_at` timestamp |
| `packages/webui/src/lib/components/jd/JDLinkedResumes.svelte` | "Linked Resumes" section for JD editor, including picker trigger |
| `packages/webui/src/lib/components/jd/ResumePickerModal.svelte` | Modal for selecting a resume to link to a JD |
| `packages/webui/src/lib/components/resume/ResumeLinkedJDs.svelte` | "Targeted JDs" section for resume editor, including picker trigger |
| `packages/webui/src/lib/components/resume/JDPickerModal.svelte` | Modal for selecting a JD to link to a resume |

## Files to Modify

| File | Change |
|------|--------|
| `packages/core/src/routes/job-descriptions.ts` | Add `GET/POST /:id/resumes`, `DELETE /:jdId/resumes/:resumeId` endpoints |
| `packages/core/src/routes/resumes.ts` | Add `GET /:id/job-descriptions` endpoint |
| `packages/sdk/src/resources/job-descriptions.ts` | Add `listResumes(jdId)`, `linkResume(jdId, resumeId)`, `unlinkResume(jdId, resumeId)` methods |
| `packages/sdk/src/resources/resumes.ts` | Add `listJobDescriptions(resumeId)` method |
| `packages/sdk/src/types.ts` | Add `ResumeLink` and `JDLink` interfaces |
| `packages/core/src/types/index.ts` | Add `ResumeLink` and `JDLink` interfaces |
| `packages/webui/src/lib/components/jd/JDEditor.svelte` | Mount `JDLinkedResumes` section in edit mode |
| Resume editor component (see Phase 45 -- if not landed: `+page.svelte`; if landed: main editor section) | Mount `ResumeLinkedJDs` section |

## Fallback Strategies

- **No resumes exist yet:** The "Linked Resumes" section shows an empty state message ("No resumes linked. Create a resume first, then link it here."). The "+ Link Resume" button opens the picker, which shows an empty list with a "No resumes found" message. No crash.
- **No JDs exist yet (resume side):** Same pattern -- the "Targeted JDs" section shows an empty state, and the JD picker shows "No job descriptions found."
- **POST duplicate link:** The `INSERT OR IGNORE` pattern makes linking idempotent. If the link already exists, `changes() === 0`, so the handler does a SELECT to return existing link data with status 200 (not 201). No error shown to the user.
- **DELETE nonexistent link:** Returns 204 regardless of whether the row existed. Idempotent.
- **Network error during link/unlink:** Show error toast. The UI does not optimistically update -- it waits for the API response before adding/removing the card from the list. On failure, the list remains unchanged.
- **Resume or JD deleted while picker is open:** The POST validates that both entities exist before inserting. Returns 404 if either is missing. The picker refreshes its list on open, so stale entries are unlikely but handled gracefully.
- **Large resume/JD lists (>200):** The picker fetches with `limit: 200`. If the user has more, only the 200 most recent appear. Pagination is not added in this phase.

---

## Tasks

### T60.1: Write Migration 019 -- Job Description Resumes Junction

**File:** `packages/core/src/db/migrations/019_job_description_resumes.sql`

[CRITICAL] The junction table must reference `job_descriptions(id)` and `resumes(id)` with ON DELETE CASCADE on both foreign keys. The composite PRIMARY KEY `(job_description_id, resume_id)` prevents duplicate links. Both indexes are required for efficient lookups from either side.

[IMPORTANT] The `created_at` column is unique to this junction table -- other junction tables (`source_skills`, `bullet_skills`, `job_description_skills`) do not have it. This records when the link was created, enabling "most recently linked" sorting and audit trail.

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

**Acceptance criteria:**
- Migration creates `job_description_resumes` table with `STRICT` mode.
- Composite PRIMARY KEY on `(job_description_id, resume_id)`.
- Both columns have `NOT NULL` and `REFERENCES ... ON DELETE CASCADE`.
- `created_at` defaults to UTC ISO-8601 timestamp.
- Both covering indexes exist (`idx_jd_resumes_jd`, `idx_jd_resumes_resume`).
- Migration row inserted into `_migrations`.

**Failure criteria:**
- Missing CASCADE on either FK -- deleting a JD or resume would fail if links exist.
- Missing STRICT mode -- SQLite would silently coerce types.
- Missing indexes -- reverse lookups (`WHERE resume_id = ?`) would be full table scans.

---

### T60.2: Add Core Type Definitions

**File:** `packages/core/src/types/index.ts`

[IMPORTANT] The `ResumeLink` and `JDLink` types represent JOINed query results, not raw junction rows. They include display fields from the joined entity to avoid round-trips.

Add after the existing `JobDescriptionWithOrg` interface:

```typescript
/** A resume linked to a JD, with display fields from the resumes table. */
export interface ResumeLink {
  resume_id: string
  resume_name: string
  target_role: string
  target_employer: string
  archetype: string
  status: ResumeStatus
  created_at: string         // from junction table -- when the link was created
  resume_created_at: string  // from resumes table -- when the resume was created
}

/** A JD linked to a resume, with display fields from the job_descriptions/organizations tables. */
export interface JDLink {
  job_description_id: string
  title: string
  organization_name: string | null
  status: JobDescriptionStatus
  location: string | null
  salary_range: string | null
  created_at: string         // from junction table -- when the link was created
  jd_created_at: string      // from job_descriptions table
}
```

**Acceptance criteria:**
- `ResumeLink` and `JDLink` export correctly from the types barrel.
- `ResumeLink.status` uses the existing `ResumeStatus` type.
- `JDLink.status` uses the existing `JobDescriptionStatus` type.
- `organization_name`, `location`, `salary_range` are nullable (matching the source tables).

---

### T60.3: Add SDK Type Definitions

**File:** `packages/sdk/src/types.ts`

Mirror the same `ResumeLink` and `JDLink` interfaces from T60.2. The SDK types file maintains its own copies of core types for bundle independence.

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

**Acceptance criteria:**
- Both interfaces are exported from the SDK types barrel.
- Field names and types match T60.2 exactly.

---

### T60.4: Add JD-Side API Endpoints

**File:** `packages/core/src/routes/job-descriptions.ts`

[CRITICAL] The POST endpoint uses `INSERT OR IGNORE` to make linking idempotent. On duplicate (`changes() === 0`), it does a SELECT to return existing link data with status 200 (not 201). Both the JD and resume must exist before inserting -- return 404 if either is missing.

[IMPORTANT] The GET endpoint JOINs `resumes` to include display fields, ordered by `jdr.created_at DESC`.

[IMPORTANT] The DELETE endpoint returns 204 regardless of whether the row existed (idempotent).

Add after the existing `app.delete('/job-descriptions/:id', ...)` handler, before `return app`:

```typescript
  // ── JD-Resume Linkage ─────────────────────────────────────────────

  app.get('/job-descriptions/:id/resumes', (c) => {
    const jdId = c.req.param('id')

    // Verify JD exists
    const jd = services.jobDescriptions.get(jdId)
    if (!jd.ok) return c.json({ error: jd.error }, mapStatusCode(jd.error.code))

    const db = services.db
    const rows = db
      .query(`
        SELECT r.id AS resume_id, r.name AS resume_name, r.target_role,
               r.target_employer, r.archetype, r.status,
               jdr.created_at, r.created_at AS resume_created_at
        FROM job_description_resumes jdr
        JOIN resumes r ON r.id = jdr.resume_id
        WHERE jdr.job_description_id = ?
        ORDER BY jdr.created_at DESC
      `)
      .all(jdId)

    return c.json({ data: rows })
  })

  app.post('/job-descriptions/:id/resumes', async (c) => {
    const jdId = c.req.param('id')
    const body = await c.req.json()
    const resumeId = body.resume_id

    if (!resumeId || typeof resumeId !== 'string') {
      return c.json(
        { error: { code: 'VALIDATION_ERROR', message: 'resume_id is required' } },
        400,
      )
    }

    // Verify JD exists
    const jd = services.jobDescriptions.get(jdId)
    if (!jd.ok) return c.json({ error: jd.error }, mapStatusCode(jd.error.code))

    // Verify resume exists
    const resume = services.resumes.getResume(resumeId)
    if (!resume.ok) {
      return c.json(
        { error: { code: 'NOT_FOUND', message: `Resume ${resumeId} not found` } },
        404,
      )
    }

    const db = services.db

    // INSERT OR IGNORE for idempotent linking.
    // [FIX] Use `result.changes` from `db.run()` return value -- NOT a
    // separate `SELECT changes()` query. SQLite's `changes()` function
    // reports rows modified by the most recent statement, but issuing
    // `db.query('SELECT changes()')` is itself a statement, making the
    // result unreliable. Bun's SQLite driver returns `{ changes: number }`
    // from `db.run()` directly.
    const result = db.run(
      `INSERT OR IGNORE INTO job_description_resumes (job_description_id, resume_id) VALUES (?, ?)`,
      [jdId, resumeId],
    )

    // Fetch the link data (whether just created or already existed)
    const link = db
      .query(`
        SELECT r.id AS resume_id, r.name AS resume_name, r.target_role,
               r.target_employer, r.archetype, r.status,
               jdr.created_at, r.created_at AS resume_created_at
        FROM job_description_resumes jdr
        JOIN resumes r ON r.id = jdr.resume_id
        WHERE jdr.job_description_id = ? AND jdr.resume_id = ?
      `)
      .get(jdId, resumeId)

    // Determine status code: 201 if new, 200 if already existed
    const status = result.changes > 0 ? 201 : 200

    return c.json({ data: link }, status as any)
  })

  app.delete('/job-descriptions/:jdId/resumes/:resumeId', (c) => {
    const { jdId, resumeId } = c.req.param()
    const db = services.db

    db.run(
      `DELETE FROM job_description_resumes WHERE job_description_id = ? AND resume_id = ?`,
      [jdId, resumeId],
    )

    return c.body(null, 204)
  })
```

[GAP] The `services.db` access pattern assumes the Hono route has access to the raw `Database` instance through services. If services does not expose `db` directly, the queries must be extracted into a repository method or the services layer must be extended. Verify the `Services` type exposes `db`.

[RESOLVED] The buggy `SELECT changes()` pattern has been corrected in the primary code block above to use `result.changes` from `db.run()`.

**Acceptance criteria:**
- `GET /api/job-descriptions/:id/resumes` returns `{ data: ResumeLink[] }` ordered by `created_at DESC`.
- `POST /api/job-descriptions/:id/resumes { resume_id }` returns 201 on new link, 200 on existing.
- `POST` returns 404 if JD or resume does not exist.
- `POST` returns 400 if `resume_id` is missing.
- `DELETE /api/job-descriptions/:jdId/resumes/:resumeId` returns 204 (idempotent).
- All response `ResumeLink` objects include JOINed resume display fields.

**Failure criteria:**
- Missing existence checks -- linking to a nonexistent entity silently creates a dangling FK reference.
- Using `INSERT` instead of `INSERT OR IGNORE` -- duplicate links would return 500.
- The `changes()` pattern breaks under Bun's SQLite driver.

---

### T60.5: Add Resume-Side API Endpoint

**File:** `packages/core/src/routes/resumes.ts`

[IMPORTANT] The GET endpoint JOINs `job_descriptions` and LEFT JOINs `organizations` to include display fields, ordered by `jdr.created_at DESC`.

Add after the existing `app.get('/resumes/:id/gaps', ...)` handler:

```typescript
  // ── Linked JDs (reverse lookup) ──────────────────────────────────

  app.get('/resumes/:id/job-descriptions', (c) => {
    const resumeId = c.req.param('id')

    // Verify resume exists
    const resume = services.resumes.getResume(resumeId)
    if (!resume.ok) return c.json({ error: resume.error }, mapStatusCode(resume.error.code))

    const db = services.db
    const rows = db
      .query(`
        SELECT jd.id AS job_description_id, jd.title, o.name AS organization_name,
               jd.status, jd.location, jd.salary_range,
               jdr.created_at, jd.created_at AS jd_created_at
        FROM job_description_resumes jdr
        JOIN job_descriptions jd ON jd.id = jdr.job_description_id
        LEFT JOIN organizations o ON o.id = jd.organization_id
        WHERE jdr.resume_id = ?
        ORDER BY jdr.created_at DESC
      `)
      .all(resumeId)

    return c.json({ data: rows })
  })
```

**Acceptance criteria:**
- `GET /api/resumes/:id/job-descriptions` returns `{ data: JDLink[] }` ordered by `created_at DESC`.
- Returns 404 for nonexistent resume.
- `organization_name` is null when the JD has no linked organization (LEFT JOIN).
- All JDLink fields match the spec definition.

---

### T60.6: Add SDK Methods -- JobDescriptionsResource

**File:** `packages/sdk/src/resources/job-descriptions.ts`

[IMPORTANT] Add the `ResumeLink` import to the existing import block from `'../types'`.

```typescript
  /** List resumes linked to a JD. */
  listResumes(jdId: string): Promise<Result<ResumeLink[]>> {
    return this.request<ResumeLink[]>(
      'GET',
      `/api/job-descriptions/${jdId}/resumes`,
    )
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

**Acceptance criteria:**
- `listResumes(jdId)` calls `GET /api/job-descriptions/:jdId/resumes`.
- `linkResume(jdId, resumeId)` calls `POST` with `{ resume_id }` body.
- `unlinkResume(jdId, resumeId)` calls `DELETE` with both IDs in the path.
- Return types match the SDK type definitions from T60.3.

---

### T60.7: Add SDK Methods -- ResumesResource

**File:** `packages/sdk/src/resources/resumes.ts`

[IMPORTANT] Add the `JDLink` import to the existing import block from `'../types'`.

```typescript
  /** List JDs linked to a resume. */
  listJobDescriptions(resumeId: string): Promise<Result<JDLink[]>> {
    return this.request<JDLink[]>(
      'GET',
      `/api/resumes/${resumeId}/job-descriptions`,
    )
  }
```

**Acceptance criteria:**
- `listJobDescriptions(resumeId)` calls `GET /api/resumes/:resumeId/job-descriptions`.
- Return type matches the SDK `JDLink` definition.

---

### T60.8: Build JDLinkedResumes Component

**File:** `packages/webui/src/lib/components/jd/JDLinkedResumes.svelte`

[IMPORTANT] This component is only rendered in edit mode (when a JD ID exists). It manages its own data fetching and state. The linked resumes list refreshes after every link/unlink operation.

```svelte
<script lang="ts">
  import { onMount } from 'svelte'
  import type { ForgeClient } from '@forge/sdk'
  import type { ResumeLink } from '@forge/sdk/types'
  import ResumePickerModal from './ResumePickerModal.svelte'

  let { jdId, forge }: {
    jdId: string
    forge: ForgeClient
  } = $props()

  let linkedResumes = $state<ResumeLink[]>([])
  let showPicker = $state(false)
  let loading = $state(true)

  async function loadLinkedResumes() {
    const result = await forge.jobDescriptions.listResumes(jdId)
    if (result.ok) linkedResumes = result.data
    loading = false
  }

  async function handleUnlink(resumeId: string) {
    await forge.jobDescriptions.unlinkResume(jdId, resumeId)
    linkedResumes = linkedResumes.filter(r => r.resume_id !== resumeId)
  }

  async function handleLink(resumeId: string) {
    const result = await forge.jobDescriptions.linkResume(jdId, resumeId)
    if (result.ok) {
      linkedResumes = [result.data, ...linkedResumes]
    }
    showPicker = false
  }

  // Use onMount for initial data load.
  onMount(() => { loadLinkedResumes() })

  // Use a separate $effect for reactive jdId prop changes.
  // This is safe because jdId is a prop (read-only from this component's
  // perspective), not state written inside the effect.
  $effect(() => {
    // Read jdId to subscribe to prop changes
    const _id = jdId
    if (_id) loadLinkedResumes()
  })
</script>

<section class="jd-linked-resumes">
  <h3>Linked Resumes</h3>

  {#if loading}
    <p class="muted">Loading...</p>
  {:else if linkedResumes.length === 0}
    <p class="muted">No resumes linked to this job description.</p>
  {:else}
    <ul class="linked-list">
      {#each linkedResumes as resume (resume.resume_id)}
        <li class="linked-card">
          <div class="linked-card-info">
            <span class="linked-card-name">{resume.resume_name}</span>
            <span class="status-badge status-{resume.status}">{resume.status}</span>
            <span class="linked-card-target muted">
              {resume.target_role} @ {resume.target_employer}
            </span>
          </div>
          <button
            class="unlink-btn"
            onclick={() => handleUnlink(resume.resume_id)}
            aria-label="Unlink {resume.resume_name}"
          >
            &times;
          </button>
        </li>
      {/each}
    </ul>
  {/if}

  <button class="link-btn" onclick={() => showPicker = true}>
    + Link Resume
  </button>

  {#if showPicker}
    <ResumePickerModal
      excludeIds={linkedResumes.map(r => r.resume_id)}
      {forge}
      onselect={handleLink}
      onclose={() => showPicker = false}
    />
  {/if}
</section>
```

**Acceptance criteria:**
- Section renders only when `jdId` is provided (edit mode).
- Linked resumes display name, status badge, target role @ target employer, and unlink button.
- Clicking unlink removes the card immediately from the list.
- "+ Link Resume" opens the ResumePickerModal.
- After linking, the new resume appears at the top of the list.
- Loading state shown while fetching.
- Empty state message when no resumes are linked.

---

### T60.9: Build ResumePickerModal Component

**File:** `packages/webui/src/lib/components/jd/ResumePickerModal.svelte`

[IMPORTANT] The picker fetches all resumes and filters out those already linked (using `excludeIds`). Search is client-side on name, target_role, and target_employer.

```svelte
<script lang="ts">
  import type { ForgeClient } from '@forge/sdk'
  import type { Resume } from '@forge/sdk/types'

  let { excludeIds, forge, onselect, onclose }: {
    excludeIds: string[]
    forge: ForgeClient
    onselect: (resumeId: string) => void
    onclose: () => void
  } = $props()

  let allResumes = $state<Resume[]>([])
  let search = $state('')
  let loading = $state(true)

  let availableResumes = $derived(
    allResumes
      .filter(r => !excludeIds.includes(r.id))
      .filter(r =>
        !search ||
        r.name.toLowerCase().includes(search.toLowerCase()) ||
        r.target_role.toLowerCase().includes(search.toLowerCase()) ||
        r.target_employer.toLowerCase().includes(search.toLowerCase())
      )
  )

  $effect(() => {
    forge.resumes.list({ limit: 200 }).then(result => {
      if (result.ok) allResumes = result.data
      loading = false
    })
  })
</script>

<div class="modal-backdrop" onclick={onclose} role="presentation">
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="modal-content" onclick|stopPropagation role="dialog" aria-label="Link Resume">
    <div class="modal-header">
      <h3>Link Resume</h3>
      <button class="close-btn" onclick={onclose} aria-label="Close">&times;</button>
    </div>

    <input
      type="text"
      class="search-input"
      placeholder="Search resumes..."
      bind:value={search}
    />

    {#if loading}
      <p class="muted">Loading resumes...</p>
    {:else if availableResumes.length === 0}
      <p class="muted">No resumes found.</p>
    {:else}
      <ul class="picker-list">
        {#each availableResumes as resume (resume.id)}
          <li>
            <button class="picker-item" onclick={() => onselect(resume.id)}>
              <span class="picker-name">{resume.name}</span>
              <span class="status-badge status-{resume.status}">{resume.status}</span>
              <span class="picker-target muted">
                {resume.target_role} @ {resume.target_employer}
              </span>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</div>
```

**Acceptance criteria:**
- Modal fetches all resumes on mount.
- Already-linked resumes are filtered out via `excludeIds`.
- Search filters by name, target_role, target_employer (case-insensitive).
- Clicking a resume calls `onselect(resume.id)`.
- Clicking backdrop or close button calls `onclose`.
- Loading and empty states rendered.

---

### T60.10: Build ResumeLinkedJDs Component

**File:** `packages/webui/src/lib/components/resume/ResumeLinkedJDs.svelte`

[IMPORTANT] The JD linking POST is always on the JD side (`POST /api/job-descriptions/:jdId/resumes`), but the SDK call `forge.jobDescriptions.linkResume(jdId, resumeId)` works from either direction. The component passes the resume ID as the second argument.

```svelte
<script lang="ts">
  import type { ForgeClient } from '@forge/sdk'
  import type { JDLink } from '@forge/sdk/types'
  import JDPickerModal from './JDPickerModal.svelte'

  let { resumeId, forge }: {
    resumeId: string
    forge: ForgeClient
  } = $props()

  let linkedJDs = $state<JDLink[]>([])
  let showPicker = $state(false)
  let loading = $state(true)

  async function loadLinkedJDs() {
    const result = await forge.resumes.listJobDescriptions(resumeId)
    if (result.ok) linkedJDs = result.data
    loading = false
  }

  async function handleUnlink(jdId: string) {
    await forge.jobDescriptions.unlinkResume(jdId, resumeId)
    linkedJDs = linkedJDs.filter(j => j.job_description_id !== jdId)
  }

  async function handleLink(jdId: string) {
    const result = await forge.jobDescriptions.linkResume(jdId, resumeId)
    if (result.ok) {
      // Refresh the full list to get JDLink format (the POST returns ResumeLink)
      await loadLinkedJDs()
    }
    showPicker = false
  }

  $effect(() => {
    if (resumeId) loadLinkedJDs()
  })
</script>

<section class="resume-linked-jds">
  <h3>Targeted Job Descriptions</h3>

  {#if loading}
    <p class="muted">Loading...</p>
  {:else if linkedJDs.length === 0}
    <p class="muted">No job descriptions linked to this resume.</p>
  {:else}
    <ul class="linked-list">
      {#each linkedJDs as jd (jd.job_description_id)}
        <li class="linked-card">
          <div class="linked-card-info">
            <span class="linked-card-name">{jd.title}</span>
            <span class="status-badge status-{jd.status}">{jd.status}</span>
            <span class="linked-card-details muted">
              {#if jd.organization_name}{jd.organization_name}{/if}
              {#if jd.location} &bull; {jd.location}{/if}
              {#if jd.salary_range} &bull; {jd.salary_range}{/if}
            </span>
          </div>
          <button
            class="unlink-btn"
            onclick={() => handleUnlink(jd.job_description_id)}
            aria-label="Unlink {jd.title}"
          >
            &times;
          </button>
        </li>
      {/each}
    </ul>
  {/if}

  <button class="link-btn" onclick={() => showPicker = true}>
    + Link JD
  </button>

  {#if showPicker}
    <JDPickerModal
      excludeIds={linkedJDs.map(j => j.job_description_id)}
      {forge}
      onselect={handleLink}
      onclose={() => showPicker = false}
    />
  {/if}
</section>
```

[INCONSISTENCY] The `handleLink` call uses `forge.jobDescriptions.linkResume(jdId, resumeId)` which returns a `ResumeLink`, not a `JDLink`. Since the response shapes differ, the handler refreshes the full list via `loadLinkedJDs()` rather than optimistically inserting the response. This adds one extra API call but ensures data consistency.

**Acceptance criteria:**
- Section renders only when `resumeId` is provided.
- Linked JDs display title, status badge, org name, location, salary range, and unlink button.
- Nullable fields (org, location, salary) are conditionally rendered.
- Clicking unlink removes the card immediately.
- "+ Link JD" opens the JDPickerModal.
- After linking, the list refreshes to show the new JD.

---

### T60.11: Build JDPickerModal Component

**File:** `packages/webui/src/lib/components/resume/JDPickerModal.svelte`

```svelte
<script lang="ts">
  import type { ForgeClient } from '@forge/sdk'
  import type { JobDescriptionWithOrg } from '@forge/sdk/types'

  let { excludeIds, forge, onselect, onclose }: {
    excludeIds: string[]
    forge: ForgeClient
    onselect: (jdId: string) => void
    onclose: () => void
  } = $props()

  let allJDs = $state<JobDescriptionWithOrg[]>([])
  let search = $state('')
  let loading = $state(true)

  let availableJDs = $derived(
    allJDs
      .filter(jd => !excludeIds.includes(jd.id))
      .filter(jd =>
        !search ||
        jd.title.toLowerCase().includes(search.toLowerCase()) ||
        (jd.organization_name ?? '').toLowerCase().includes(search.toLowerCase())
      )
  )

  $effect(() => {
    forge.jobDescriptions.list({ limit: 200 }).then(result => {
      if (result.ok) allJDs = result.data
      loading = false
    })
  })
</script>

<div class="modal-backdrop" onclick={onclose} role="presentation">
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="modal-content" onclick|stopPropagation role="dialog" aria-label="Link Job Description">
    <div class="modal-header">
      <h3>Link Job Description</h3>
      <button class="close-btn" onclick={onclose} aria-label="Close">&times;</button>
    </div>

    <input
      type="text"
      class="search-input"
      placeholder="Search job descriptions..."
      bind:value={search}
    />

    {#if loading}
      <p class="muted">Loading job descriptions...</p>
    {:else if availableJDs.length === 0}
      <p class="muted">No job descriptions found.</p>
    {:else}
      <ul class="picker-list">
        {#each availableJDs as jd (jd.id)}
          <li>
            <button class="picker-item" onclick={() => onselect(jd.id)}>
              <span class="picker-name">{jd.title}</span>
              <span class="status-badge status-{jd.status}">{jd.status}</span>
              {#if jd.organization_name}
                <span class="picker-org muted">{jd.organization_name}</span>
              {/if}
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</div>
```

**Acceptance criteria:**
- Modal fetches all JDs on mount.
- Already-linked JDs filtered out via `excludeIds`.
- Search filters by title and organization_name (case-insensitive, null-safe).
- Clicking a JD calls `onselect(jd.id)`.
- Clicking backdrop or close calls `onclose`.

---

### T60.12: Mount Components in Editor Pages

**File (JD side):** `packages/webui/src/lib/components/jd/JDEditor.svelte`

[IMPORTANT] The `JDLinkedResumes` section is only visible in edit mode (when a JD ID exists, not create mode). It should be placed below the "Required Skills" section and above the "Notes" section.

```svelte
<!-- In JDEditor.svelte, after the skills section, before notes -->
{#if mode === 'edit' && selectedId}
  <JDLinkedResumes jdId={selectedId} {forge} />
{/if}
```

**File (Resume side):** The resume editor component mount point depends on Phase 45 (Editor Restructuring). If Phase 45 has not landed, mount in `packages/webui/src/routes/resumes/+page.svelte`. If Phase 45 has landed, mount in the main editor section of the restructured resume editor.

```svelte
<!-- In the resume editor, after appropriate section -->
{#if resumeId}
  <ResumeLinkedJDs {resumeId} {forge} />
{/if}
```

**Acceptance criteria:**
- JD editor shows "Linked Resumes" in edit mode only.
- Resume editor shows "Targeted JDs" when a resume is selected.
- Both components receive the correct `forge` client instance.

---

## Testing Support

### Migration Tests

| Test | Assertion |
|------|-----------|
| Migration 019 creates table | `SELECT name FROM sqlite_master WHERE name = 'job_description_resumes'` returns a row |
| Insert valid link | `INSERT INTO job_description_resumes` with valid JD and resume IDs succeeds |
| Reject duplicate link | `INSERT INTO job_description_resumes` with same pair raises PRIMARY KEY error |
| `INSERT OR IGNORE` duplicate | Does not raise an error, `changes() = 0` |
| Cascade on JD delete | Delete JD, verify junction rows removed, resume survives |
| Cascade on resume delete | Delete resume, verify junction rows removed, JD survives |
| `created_at` default | Insert without explicit `created_at`, verify ISO-8601 UTC timestamp set |

### API Tests (JD Side)

| Test | Assertion |
|------|-----------|
| `GET /:id/resumes` empty | Returns `{ data: [] }` for JD with no links |
| `POST /:id/resumes` new link | Returns 201 with `ResumeLink` data |
| `POST /:id/resumes` duplicate | Returns 200 with existing link data (idempotent) |
| `POST /:id/resumes` missing `resume_id` | Returns 400 VALIDATION_ERROR |
| `POST /:id/resumes` nonexistent resume | Returns 404 NOT_FOUND |
| `POST /:id/resumes` nonexistent JD | Returns 404 NOT_FOUND |
| `DELETE /:jdId/resumes/:resumeId` existing | Returns 204 |
| `DELETE /:jdId/resumes/:resumeId` nonexistent | Returns 204 (idempotent) |
| `GET /:id/resumes` JOINed fields | Returns resume_name, target_role, target_employer, archetype, status |
| `GET /:id/resumes` ordering | Results ordered by `created_at DESC` |

### API Tests (Resume Side)

| Test | Assertion |
|------|-----------|
| `GET /:id/job-descriptions` empty | Returns `{ data: [] }` for resume with no links |
| `GET /:id/job-descriptions` JOINed fields | Returns title, organization_name, status, location, salary_range |
| `GET /:id/job-descriptions` ordering | Results ordered by `created_at DESC` |
| `GET /:id/job-descriptions` nonexistent resume | Returns 404 |

### SDK Tests

| Test | Assertion |
|------|-----------|
| `listResumes(jdId)` | Returns `{ data: ResumeLink[] }` |
| `linkResume(jdId, resumeId)` | Returns `{ data: ResumeLink }` |
| `unlinkResume(jdId, resumeId)` | Returns void (204) |
| `listJobDescriptions(resumeId)` | Returns `{ data: JDLink[] }` |

### Component Smoke Tests (Manual / Future Playwright)

| Test | What to verify |
|------|---------------|
| JD editor shows "Linked Resumes" in edit mode | Section visible, "+ Link Resume" button present |
| JD editor hides "Linked Resumes" in create mode | Section not rendered |
| Linked resumes display correctly | Name, status badge, target role/employer visible |
| Unlink removes card | Card disappears from list after click |
| Resume picker opens | Modal with search input and resume list |
| Resume picker filters | Search reduces visible resumes |
| Resume picker excludes linked | Already-linked resumes not shown |
| Resume picker selects and closes | Clicking a resume links it and closes the modal |
| Resume editor shows "Targeted JDs" | Section visible with JD cards |
| JD picker opens and works | Same behavior as resume picker |
| Both sections refresh after operations | Lists update after link/unlink |

### Integration Tests

| Test | Assertion |
|------|-----------|
| Bidirectional verification | Link R1 to J1 via JD side. Verify R1 in `GET /jd/J1/resumes`. Verify J1 in `GET /resumes/R1/job-descriptions`. |
| Cascade on JD delete | Delete J1. Verify junction rows gone. Verify R1 still exists. |
| Cascade on resume delete | Delete R1. Verify junction rows gone. Verify J1 still exists. |
| Unlink from either side | Unlink via DELETE. Verify removed from both lookups. |

---

## Documentation Requirements

- No new documentation files required.
- The spec file serves as the design document.
- This plan file serves as the implementation reference.
- Inline TSDoc comments on all exported interfaces:
  - `ResumeLink`: explain that fields are JOINed from `resumes` table, `created_at` is from the junction table.
  - `JDLink`: explain that fields are JOINed from `job_descriptions` and `organizations` tables.
- Inline comments in route handlers for:
  - `INSERT OR IGNORE` idempotency rationale
  - Status code 200 vs 201 distinction
  - LEFT JOIN on organizations for nullable org name

---

## Parallelization Notes

**Within this phase:**
- T60.1 (migration) has no code dependencies -- can be written immediately.
- T60.2 (core types) and T60.3 (SDK types) can be written in parallel.
- T60.4 (JD routes) and T60.5 (resume routes) depend on T60.1 (migration must exist for the queries) and T60.2 (core types for response shapes). Can be written in parallel with each other.
- T60.6 (SDK JD methods) and T60.7 (SDK resume methods) depend on T60.3 (SDK types). Can be written in parallel with each other.
- T60.8-T60.11 (UI components) depend on T60.6 and T60.7 (SDK methods must exist). Can be written in parallel with each other.
- T60.12 (mount in editors) depends on T60.8 and T60.10 (components must exist).

**Recommended execution order:**
1. T60.1 (migration -- foundational)
2. T60.2 + T60.3 (types -- parallel)
3. T60.4 + T60.5 (routes -- parallel)
4. T60.6 + T60.7 (SDK methods -- parallel)
5. T60.8 + T60.9 + T60.10 + T60.11 (UI components -- parallel)
6. T60.12 (mount in editors -- depends on all above)

**Cross-phase:**
- Phase 61 (JD Kanban Pipeline) modifies the same `job-descriptions.ts` route file and `JobDescriptionStatus` types. Serialize carefully if both phases are in progress simultaneously.
- Phase 62 (JD Skill Extraction) also adds a route to `job-descriptions.ts`. Same serialization concern.
- This phase does not conflict with Phase 48 (GraphView) or Phase 43 (Generic Kanban).
