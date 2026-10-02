# Phase 49: JD Detail Page (Spec E1)

**Status:** Planning
**Date:** 2026-04-03
**Spec:** [2026-04-03-jd-detail-page.md](../refs/specs/2026-04-03-jd-detail-page.md)
**Depends on:** None (foundational for E2-E6)
**Blocks:** E2 (JD Parsing), E3 (JD Pipeline/Kanban), E4 (JD-Resume Matching), E5 (JD Skill Gap Analysis), E6 (JD Prompt Logging)
**Parallelizable with:** Phase 50 (Contacts Support), Phase 48 (Generic GraphView) -- no file conflicts except `job-descriptions.ts` (see note below)

> **Parallelization conflict with Phase 50:** Phase 50 (Contacts) also modifies `packages/core/src/routes/job-descriptions.ts` routes file. These phases require merge coordination on this file.

## Goal

Build a full CRUD UI for job descriptions at `/opportunities/job-descriptions` using a split-panel layout (list + editor). Add a `job_description_skills` junction table (migration 018) with API endpoints for skill tagging. Replace the placeholder `EmptyState` component with a production-ready JD management page including status filtering, search, organization dropdown, skill tag picker, and immediate-save status changes.

## Non-Goals

- JD parsing / NLP extraction of requirements, skills, experience levels
- Skill matching or gap analysis against user's skill inventory
- Resume-to-JD linking or recommendation
- JD fetching from job boards (Indeed, LinkedIn, Greenhouse)
- Kanban pipeline view for JDs (deferred to Spec E3)
- Contact linking on the JD page (deferred to Spec G / Phase 50)
- Structured salary parsing (salary_range remains free-text per the existing schema)
- Duplicate detection
- Cover letter generation

## Context

Phase 31 built the full JD backend: migration 007 (`job_descriptions` table), `JobDescriptionRepository`, `JobDescriptionService`, API routes (`/api/job-descriptions` CRUD), and the SDK `JobDescriptionsResource`. The placeholder at `/opportunities/job-descriptions` exists but only renders an `EmptyState` component. Source skills (migration 016, `source_skills` junction) provide the exact pattern for JD skill tagging.

## Scope

| Spec section | Covered here? |
|-------------|---------------|
| 1. Schema changes (migration 018: `job_description_skills`) | Yes |
| 2. API changes (existing CRUD + new skill endpoints) | Yes |
| 3. UI layout (split-panel, list panel, editor panel) | Yes |
| 4. Skill tagging (tag picker, state management) | Yes |
| 5. Component architecture (page, card, editor, skill picker) | Yes |
| 6. Files to create | Yes |
| 7. Files to modify | Yes |
| 8. Testing | Yes |
| 9. Acceptance criteria | Yes |

## Files to Create

| File | Description |
|------|-------------|
| `packages/core/src/db/migrations/018_job_description_skills.sql` | Junction table for JD-to-skill linking |
| `packages/webui/src/lib/components/jd/JDCard.svelte` | JD card for list panel |
| `packages/webui/src/lib/components/jd/JDEditor.svelte` | JD editor form (create + edit modes) |
| `packages/webui/src/lib/components/jd/JDSkillPicker.svelte` | Skill tag picker for JD required skills |

## Files to Modify

| File | Change |
|------|--------|
| `packages/webui/src/routes/opportunities/job-descriptions/+page.svelte` | Replace placeholder EmptyState with full split-panel CRUD layout |
| `packages/core/src/routes/job-descriptions.ts` | Add JD skills endpoints: `GET/POST /:id/skills`, `DELETE /:jdId/skills/:skillId` |
| `packages/sdk/src/resources/job-descriptions.ts` | Add `listSkills(jdId)`, `addSkill(jdId, input)`, `removeSkill(jdId, skillId)` methods |

> **Reference file:** `packages/core/src/routes/sources.ts` is the pattern source for the skill endpoint implementation (no changes made to it).
| `packages/webui/src/lib/components/StatusBadge.svelte` | Add JD status color + label mappings for all 8 statuses |
| `packages/core/src/db/repositories/job-description-repository.ts` | Update `list()` to order by `updated_at DESC` instead of `created_at DESC` |

## Fallback Strategies

- **Organization dropdown empty:** If `forge.organizations.list()` returns no orgs, the dropdown shows only "None". Users can still create/edit JDs without an org link. No crash.
- **Skill search returns no matches:** The dropdown shows "Press Enter to create '[typed text]'" hint. The POST endpoint creates the skill inline.
- **JD list exceeds 200 items:** The initial load uses `limit: 200`. If the user has more, only the 200 most recently updated appear. Pagination is not added in this phase (future enhancement).
- **Status dropdown immediate save fails:** On network error, display toast error and revert the dropdown to the previous value. The JD list is not refreshed until save succeeds.
- **Migration 018 runs before migration 016 (source_skills):** Migration 018 only depends on `job_descriptions` (007) and `skills` (001). No dependency on migration 016. Safe to run in any order relative to 016.

---

## Tasks

### T49.1: Write Migration 018 -- Job Description Skills Junction

**File:** `packages/core/src/db/migrations/018_job_description_skills.sql`

[CRITICAL] The junction table must reference `job_descriptions(id)` and `skills(id)` with ON DELETE CASCADE. Both tables must exist (migrations 007 and 001). The composite PRIMARY KEY `(job_description_id, skill_id)` prevents duplicate links.

```sql
-- Job Description Skills Junction
-- Migration: 018_job_description_skills
-- Links skills to job descriptions for requirement tracking.
-- Skills describe what technologies/tools the JD requires.

CREATE TABLE job_description_skills (
  job_description_id TEXT NOT NULL REFERENCES job_descriptions(id) ON DELETE CASCADE,
  skill_id TEXT NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
  PRIMARY KEY (job_description_id, skill_id)
) STRICT;

CREATE INDEX idx_jd_skills_jd ON job_description_skills(job_description_id);
CREATE INDEX idx_jd_skills_skill ON job_description_skills(skill_id);

INSERT INTO _migrations (name) VALUES ('018_job_description_skills');
```

**Acceptance criteria:**
- Migration creates `job_description_skills` table with correct schema.
- `INSERT INTO job_description_skills` with valid JD and skill IDs succeeds.
- `INSERT INTO job_description_skills` with duplicate pair is rejected (PRIMARY KEY constraint).
- Deleting a JD cascades to `job_description_skills` rows.
- Deleting a skill cascades to `job_description_skills` rows.
- Both indexes exist.

**Failure criteria:**
- Foreign key references point to wrong tables.
- Missing `STRICT` mode.
- Missing `ON DELETE CASCADE` on either FK.

---

### T49.2: Update Job Description Repository -- Sort by `updated_at DESC`

**File:** `packages/core/src/db/repositories/job-description-repository.ts`

[IMPORTANT] The `list()` function currently orders by `created_at DESC`. The spec requires `updated_at DESC` so recently edited JDs appear first in the list panel.

Change line 112:

```typescript
// Before:
`${SELECT_WITH_ORG} ${where} ORDER BY jd.created_at DESC LIMIT ? OFFSET ?`,

// After:
`${SELECT_WITH_ORG} ${where} ORDER BY jd.updated_at DESC LIMIT ? OFFSET ?`,
```

**Acceptance criteria:**
- `list()` returns JDs ordered by `updated_at DESC`.
- A JD that was updated more recently than another appears first, regardless of creation order.

**Failure criteria:**
- Sort order still uses `created_at` after the change.

---

### T49.3: Add JD Skills Endpoints to Routes

**File:** `packages/core/src/routes/job-descriptions.ts`

[CRITICAL] The endpoints must follow the exact pattern from `packages/core/src/routes/sources.ts` lines 88-152 (source skills). The route handler accesses `db` directly for skill operations (not through the service layer), matching the source skills pattern.

[IMPORTANT] The route function signature must change from `jobDescriptionRoutes(services: Services)` to `jobDescriptionRoutes(services: Services, db: Database)` to enable direct DB access for skill queries.

[ANTI-PATTERN] The source skills routes access `db` directly instead of going through a service. This is an existing pattern in the codebase. For consistency, JD skills follow the same approach. A future refactor could move all skill operations into services.

```typescript
/**
 * Job description routes -- thin HTTP layer over JobDescriptionService.
 */

import { Hono } from 'hono'
import type { Database } from 'bun:sqlite'
import type { Services } from '../services'
import { mapStatusCode } from './server'

export function jobDescriptionRoutes(services: Services, db: Database) {
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

  // ── JD Skills ───────────────────────────────────────────────────────

  app.get('/job-descriptions/:id/skills', (c) => {
    const rows = db.query(
      `SELECT s.* FROM skills s
       JOIN job_description_skills jds ON jds.skill_id = s.id
       WHERE jds.job_description_id = ?
       ORDER BY s.name ASC`
    ).all(c.req.param('id'))
    return c.json({ data: rows })
  })

  app.post('/job-descriptions/:id/skills', async (c) => {
    const body = await c.req.json()
    const jdId = c.req.param('id')

    // If skill_id is provided, link existing skill
    if (body.skill_id) {
      try {
        db.run('INSERT OR IGNORE INTO job_description_skills (job_description_id, skill_id) VALUES (?, ?)',
          [jdId, body.skill_id])
      } catch (err: any) {
        if (err.message?.includes('FOREIGN KEY')) {
          return c.json({ error: { code: 'NOT_FOUND', message: 'Job description or skill not found' } }, 404)
        }
        throw err
      }
      const skill = db.query('SELECT * FROM skills WHERE id = ?').get(body.skill_id)
      return c.json({ data: skill }, 201)
    }

    // If name is provided, create new skill and link it
    if (body.name?.trim()) {
      const raw = body.name.trim()
      const name = raw.charAt(0).toUpperCase() + raw.slice(1)
      let skill = db.query('SELECT * FROM skills WHERE name = ? COLLATE NOCASE').get(name) as any
      if (!skill) {
        const id = crypto.randomUUID()
        skill = db.query(
          `INSERT INTO skills (id, name, category) VALUES (?, ?, ?) RETURNING *`
        ).get(id, name, body.category ?? 'general')
      }
      db.run('INSERT OR IGNORE INTO job_description_skills (job_description_id, skill_id) VALUES (?, ?)',
        [jdId, skill.id])
      return c.json({ data: skill }, 201)
    }

    return c.json({ error: { code: 'VALIDATION_ERROR', message: 'skill_id or name is required' } }, 400)
  })

  app.delete('/job-descriptions/:jdId/skills/:skillId', (c) => {
    db.run(
      'DELETE FROM job_description_skills WHERE job_description_id = ? AND skill_id = ?',
      [c.req.param('jdId'), c.req.param('skillId')]
    )
    return c.body(null, 204)
  })

  return app
}
```

[IMPORTANT] The server route registration in `packages/core/src/routes/server.ts` must also be updated to pass `db`:

```typescript
// Before (line 114):
app.route('/', jobDescriptionRoutes(services))

// After:
app.route('/', jobDescriptionRoutes(services, db))
```

[INCONSISTENCY] The DELETE endpoint for JD skills returns 204 unconditionally (idempotent), while the source skills DELETE returns 404 if the link did not exist. The spec says "Return 204 regardless of whether the row existed (idempotent)." This is intentional divergence from the source skills pattern for better UX.

**Acceptance criteria:**
- `GET /api/job-descriptions/:id/skills` returns `{ data: Skill[] }` (empty array for JD with no skills).
- `POST /api/job-descriptions/:id/skills` with `{ skill_id }` links existing skill, returns 201.
- `POST /api/job-descriptions/:id/skills` with `{ skill_id }` duplicate is idempotent (INSERT OR IGNORE).
- `POST /api/job-descriptions/:id/skills` with `{ name: 'python' }` creates "Python" skill (capitalizeFirst) and links it.
- `POST /api/job-descriptions/:id/skills` with `{ name: 'python' }` when "Python" exists links existing (case-insensitive dedup).
- `POST /api/job-descriptions/:id/skills` with neither returns 400.
- `DELETE /api/job-descriptions/:jdId/skills/:skillId` removes junction row, returns 204.
- FK constraint error returns 404.

**Failure criteria:**
- Route function not updated to accept `db` parameter.
- `server.ts` not updated to pass `db` to `jobDescriptionRoutes`.
- Skill creation does not apply capitalizeFirst.
- Case-insensitive dedup uses `=` instead of `COLLATE NOCASE`.

---

### T49.4: Add Skill Methods to SDK JobDescriptionsResource

**File:** `packages/sdk/src/resources/job-descriptions.ts`

[IMPORTANT] The SDK methods follow the same pattern as existing resource methods. `listSkills` uses `request` (not `requestList`) because the response is a simple `{ data: Skill[] }` without pagination.

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
  Skill,
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

  // ── JD Skills ─────────────────────────────────────────────────────

  /** List all skills linked to a job description. */
  listSkills(jdId: string): Promise<Result<Skill[]>> {
    return this.request<Skill[]>(
      'GET',
      `/api/job-descriptions/${jdId}/skills`,
    )
  }

  /** Link a skill to a job description. Pass skill_id to link existing, or name to create+link. */
  addSkill(
    jdId: string,
    input: { skill_id: string } | { name: string; category?: string },
  ): Promise<Result<Skill>> {
    return this.request<Skill>(
      'POST',
      `/api/job-descriptions/${jdId}/skills`,
      input,
    )
  }

  /** Remove a skill link from a job description. */
  removeSkill(jdId: string, skillId: string): Promise<Result<void>> {
    return this.request<void>(
      'DELETE',
      `/api/job-descriptions/${jdId}/skills/${skillId}`,
    )
  }
}
```

**Acceptance criteria:**
- `forge.jobDescriptions.listSkills(jdId)` returns `Result<Skill[]>`.
- `forge.jobDescriptions.addSkill(jdId, { skill_id })` returns `Result<Skill>`.
- `forge.jobDescriptions.addSkill(jdId, { name })` returns `Result<Skill>`.
- `forge.jobDescriptions.removeSkill(jdId, skillId)` returns `Result<void>`.
- All existing CRUD methods remain unchanged.

**Failure criteria:**
- `Skill` type not imported from SDK types.
- `listSkills` uses `requestList` instead of `request` (response is not paginated).

---

### T49.5: Update StatusBadge for JD Statuses

**File:** `packages/webui/src/lib/components/StatusBadge.svelte`

[IMPORTANT] The existing `StatusBadge.svelte` only has 6 entries (draft, approved, pending_review, rejected, deriving, final). It needs 8 JD-specific status entries added to both `colorMap` and `labelMap`. The keys will not collide because JD statuses use different names.

```svelte
<script lang="ts">
  let { status }: { status: string } = $props()

  const colorMap: Record<string, string> = {
    // Existing statuses (source, bullet, perspective, resume)
    draft: '#6b7280',
    approved: '#22c55e',
    pending_review: '#f59e0b',
    rejected: '#ef4444',
    deriving: '#3b82f6',
    final: '#8b5cf6',
    // JD statuses (Spec E1)
    interested: '#6b7280',
    analyzing: '#3b82f6',
    applied: '#6366f1',
    interviewing: '#a855f7',
    offered: '#22c55e',
    // 'rejected' already exists above with #ef4444 -- shared
    withdrawn: '#f97316',
    closed: '#374151',
  }

  const labelMap: Record<string, string> = {
    // Existing statuses
    draft: 'Draft',
    approved: 'Approved',
    pending_review: 'Pending Review',
    rejected: 'Rejected',
    deriving: 'Deriving',
    final: 'Final',
    // JD statuses (Spec E1)
    interested: 'Interested',
    analyzing: 'Analyzing',
    applied: 'Applied',
    interviewing: 'Interviewing',
    offered: 'Offered',
    // 'rejected' already exists above -- shared
    withdrawn: 'Withdrawn',
    closed: 'Closed',
  }

  let color = $derived(colorMap[status] ?? '#6b7280')
  let label = $derived(labelMap[status] ?? status)
  let pulsing = $derived(status === 'deriving')
</script>

<span
  class="badge"
  class:pulsing
  style:background={color}
>
  {label}
</span>

<style>
  .badge {
    display: inline-block;
    padding: 0.2em 0.6em;
    border-radius: 999px;
    font-size: 0.75rem;
    font-weight: 600;
    color: #fff;
    line-height: 1.4;
    white-space: nowrap;
    letter-spacing: 0.01em;
  }

  .pulsing {
    animation: pulse 1.5s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% {
      opacity: 1;
    }
    50% {
      opacity: 0.6;
    }
  }
</style>
```

[GAP] The `rejected` status is shared between bullet/perspective and JD entities. Both use `#ef4444` (red). This is correct behavior -- the same status name maps to the same color.

**Acceptance criteria:**
- All 8 JD statuses render with correct colors: interested (gray), analyzing (blue), applied (indigo), interviewing (purple), offered (green), rejected (red), withdrawn (orange), closed (dark gray).
- Existing statuses (draft, approved, pending_review, rejected, deriving, final) continue working unchanged.
- No duplicate keys in colorMap or labelMap.

**Failure criteria:**
- JD status keys collide with existing keys (only `rejected` overlaps and that is intentional).
- Colors do not match spec Section 3.6.

---

### T49.6: Write JDCard Component

**File:** `packages/webui/src/lib/components/jd/JDCard.svelte`

[MINOR] Card follows the same visual pattern as organization cards in the data pages.

```svelte
<!--
  JDCard.svelte -- Card for the JD list panel.
  Displays title, organization name, status badge, location, and salary range.
-->
<script lang="ts">
  import { StatusBadge } from '$lib/components'
  import type { JobDescriptionWithOrg } from '@forge/sdk'

  let {
    jd,
    selected = false,
    onclick,
  }: {
    jd: JobDescriptionWithOrg
    selected?: boolean
    onclick: () => void
  } = $props()
</script>

<button
  class="jd-card"
  class:selected
  onclick={onclick}
  type="button"
>
  <div class="card-header">
    <span class="title">{jd.title}</span>
    <StatusBadge status={jd.status} />
  </div>
  <span class="org-name">{jd.organization_name ?? 'No organization'}</span>
  {#if jd.location || jd.salary_range}
    <div class="card-meta">
      {#if jd.location}
        <span class="meta-item">{jd.location}</span>
      {/if}
      {#if jd.salary_range}
        <span class="meta-item">{jd.salary_range}</span>
      {/if}
    </div>
  {/if}
</button>

<style>
  .jd-card {
    display: block;
    width: 100%;
    text-align: left;
    padding: 0.75rem;
    border: 1px solid #e5e7eb;
    border-radius: 0.5rem;
    background: #fff;
    cursor: pointer;
    transition: border-color 0.15s, background-color 0.15s;
  }

  .jd-card:hover {
    border-color: #93c5fd;
    background: #f0f9ff;
  }

  .jd-card.selected {
    border-color: #3b82f6;
    background: #eff6ff;
  }

  .card-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 0.5rem;
    margin-bottom: 0.25rem;
  }

  .title {
    font-weight: 600;
    font-size: 0.9rem;
    color: #1a1a2e;
    line-height: 1.3;
  }

  .org-name {
    display: block;
    font-size: 0.8rem;
    color: #6b7280;
    margin-bottom: 0.25rem;
  }

  .card-meta {
    display: flex;
    gap: 0.75rem;
    flex-wrap: wrap;
  }

  .meta-item {
    font-size: 0.75rem;
    color: #9ca3af;
  }
</style>
```

**Acceptance criteria:**
- Card displays title, organization name, status badge, location, salary range.
- Missing org shows "No organization".
- Missing location/salary_range rows do not render empty space.
- Selected state has highlighted border.
- Clicking fires the `onclick` callback.

---

### T49.7: Write JDSkillPicker Component

**File:** `packages/webui/src/lib/components/jd/JDSkillPicker.svelte`

[CRITICAL] All skill operations are immediate (no "Save" needed). Adding a skill calls `POST /api/job-descriptions/:id/skills` immediately. Removing calls `DELETE` immediately.

[IMPORTANT] The skill dropdown filters out skills already linked to this JD (`allSkills.filter(s => !jdSkills.some(js => js.id === s.id))`).

```svelte
<!--
  JDSkillPicker.svelte -- Skill tag picker for JD required skills.
  Matches the source/bullet skill picker pattern: immediate persistence.
-->
<script lang="ts">
  import { forge, friendlyError } from '$lib/sdk'
  import { addToast } from '$lib/stores/toast.svelte'
  import type { Skill } from '@forge/sdk'

  let {
    jdId,
    jdSkills = $bindable([]),
  }: {
    jdId: string
    jdSkills: Skill[]
  } = $props()

  let allSkills = $state<Skill[]>([])
  let skillSearch = $state('')
  let showDropdown = $state(false)
  let loading = $state(false)

  let filteredSkills = $derived.by(() => {
    const linked = new Set(jdSkills.map(s => s.id))
    let available = allSkills.filter(s => !linked.has(s.id))
    if (skillSearch.trim()) {
      const q = skillSearch.toLowerCase()
      available = available.filter(s => s.name.toLowerCase().includes(q))
    }
    return available.slice(0, 20)
  })

  $effect(() => {
    loadAllSkills()
  })

  async function loadAllSkills() {
    const res = await forge.skills.list()
    if (res.ok) {
      allSkills = res.data
    }
  }

  async function addSkillById(skillId: string) {
    loading = true
    const res = await forge.jobDescriptions.addSkill(jdId, { skill_id: skillId })
    if (res.ok) {
      jdSkills = [...jdSkills, res.data]
    } else {
      addToast({ type: 'error', message: friendlyError(res.error) })
    }
    skillSearch = ''
    showDropdown = false
    loading = false
  }

  async function addSkillByName(name: string) {
    if (!name.trim()) return
    loading = true
    const res = await forge.jobDescriptions.addSkill(jdId, { name: name.trim() })
    if (res.ok) {
      jdSkills = [...jdSkills, res.data]
      // Refresh all skills list so new skill appears in dropdown next time
      await loadAllSkills()
    } else {
      addToast({ type: 'error', message: friendlyError(res.error) })
    }
    skillSearch = ''
    showDropdown = false
    loading = false
  }

  async function removeSkill(skillId: string) {
    const res = await forge.jobDescriptions.removeSkill(jdId, skillId)
    if (res.ok) {
      jdSkills = jdSkills.filter(s => s.id !== skillId)
    } else {
      addToast({ type: 'error', message: friendlyError(res.error) })
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault()
      if (filteredSkills.length > 0) {
        addSkillById(filteredSkills[0].id)
      } else if (skillSearch.trim()) {
        addSkillByName(skillSearch)
      }
    }
    if (e.key === 'Escape') {
      showDropdown = false
      skillSearch = ''
    }
  }
</script>

<div class="skill-picker">
  <div class="skill-tags">
    {#each jdSkills as skill (skill.id)}
      <span class="skill-pill">
        {skill.name}
        <button
          class="remove-btn"
          onclick={() => removeSkill(skill.id)}
          type="button"
          aria-label="Remove {skill.name}"
        >&times;</button>
      </span>
    {/each}
  </div>

  <div class="search-wrapper">
    <input
      type="text"
      class="skill-input"
      placeholder="Search or add skill..."
      bind:value={skillSearch}
      onfocus={() => (showDropdown = true)}
      onkeydown={handleKeydown}
      disabled={loading}
    />
    {#if showDropdown && (filteredSkills.length > 0 || skillSearch.trim())}
      <div class="dropdown">
        {#each filteredSkills as skill (skill.id)}
          <button
            class="dropdown-item"
            type="button"
            onclick={() => addSkillById(skill.id)}
          >
            {skill.name}
            {#if skill.category && skill.category !== 'general'}
              <span class="category-label">{skill.category}</span>
            {/if}
          </button>
        {/each}
        {#if skillSearch.trim() && filteredSkills.length === 0}
          <button
            class="dropdown-item create-item"
            type="button"
            onclick={() => addSkillByName(skillSearch)}
          >
            Create "{skillSearch.trim()}"
          </button>
        {/if}
      </div>
    {/if}
  </div>
</div>

<style>
  .skill-picker {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .skill-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 0.375rem;
  }

  .skill-pill {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.2em 0.5em;
    background: #e0e7ff;
    color: #3730a3;
    border-radius: 999px;
    font-size: 0.8rem;
    font-weight: 500;
  }

  .remove-btn {
    background: none;
    border: none;
    color: #6366f1;
    cursor: pointer;
    font-size: 1rem;
    line-height: 1;
    padding: 0 0.15em;
  }

  .remove-btn:hover {
    color: #ef4444;
  }

  .search-wrapper {
    position: relative;
  }

  .skill-input {
    width: 100%;
    padding: 0.4rem 0.6rem;
    border: 1px solid #d1d5db;
    border-radius: 0.375rem;
    font-size: 0.85rem;
    outline: none;
  }

  .skill-input:focus {
    border-color: #6366f1;
    box-shadow: 0 0 0 2px rgba(99, 102, 241, 0.15);
  }

  .dropdown {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    background: #fff;
    border: 1px solid #d1d5db;
    border-radius: 0.375rem;
    box-shadow: 0 4px 6px rgba(0, 0, 0, 0.07);
    max-height: 200px;
    overflow-y: auto;
    z-index: 50;
    margin-top: 0.25rem;
  }

  .dropdown-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
    padding: 0.5rem 0.75rem;
    background: none;
    border: none;
    text-align: left;
    cursor: pointer;
    font-size: 0.85rem;
    color: #1a1a2e;
  }

  .dropdown-item:hover {
    background: #f0f9ff;
  }

  .category-label {
    font-size: 0.7rem;
    color: #9ca3af;
    font-style: italic;
  }

  .create-item {
    color: #6366f1;
    font-style: italic;
  }
</style>
```

**Acceptance criteria:**
- Linked skills display as pill tags with x remove buttons.
- Typing in the input filters the dropdown (case-insensitive substring).
- Selecting a skill from dropdown calls `POST` with `{ skill_id }` immediately.
- Pressing Enter with unmatched text calls `POST` with `{ name }` (creates skill).
- Removing a skill calls `DELETE` immediately and removes the pill.
- Already-linked skills are excluded from the dropdown.
- Dropdown closes on Escape.
- Loading state disables input.

**Failure criteria:**
- Skill operations require clicking "Save" to persist.
- New skills do not appear in the global skills list.
- Duplicate skills can be linked.

---

### T49.8: Write JDEditor Component

**File:** `packages/webui/src/lib/components/jd/JDEditor.svelte`

[CRITICAL] Status dropdown changes save immediately (no "Save" click). All other field changes require clicking "Save".

[IMPORTANT] The `organization_id` dropdown must include a "None" option that sets `organization_id = null`.

[IMPORTANT] The URL field renders as a clickable external link when populated in view mode (edit mode shows text input).

```svelte
<!--
  JDEditor.svelte -- JD editor form for create + edit modes.
  Status changes save immediately. Other changes require Save.
-->
<script lang="ts">
  import { forge, friendlyError } from '$lib/sdk'
  import { addToast } from '$lib/stores/toast.svelte'
  import { ConfirmDialog } from '$lib/components'
  import JDSkillPicker from './JDSkillPicker.svelte'
  import type { JobDescriptionWithOrg, Organization, Skill } from '@forge/sdk'

  let {
    jd = null,
    organizations = [],
    createMode = false,
    oncreated,
    onupdated,
    ondeleted,
  }: {
    jd: JobDescriptionWithOrg | null
    organizations: Organization[]
    createMode?: boolean
    oncreated: (jd: JobDescriptionWithOrg) => void
    onupdated: (jd: JobDescriptionWithOrg) => void
    ondeleted: (id: string) => void
  } = $props()

  // Form state
  let title = $state('')
  let organizationId = $state<string | null>(null)
  let status = $state('interested')
  let location = $state('')
  let salaryRange = $state('')
  let url = $state('')
  let rawText = $state('')
  let notes = $state('')

  let jdSkills = $state<Skill[]>([])
  let saving = $state(false)
  let confirmDeleteOpen = $state(false)

  // Track dirty state for non-status fields
  let isDirty = $derived.by(() => {
    if (createMode || !jd) return false
    return (
      title !== jd.title ||
      organizationId !== (jd.organization_id ?? null) ||
      location !== (jd.location ?? '') ||
      salaryRange !== (jd.salary_range ?? '') ||
      url !== (jd.url ?? '') ||
      rawText !== jd.raw_text ||
      notes !== (jd.notes ?? '')
    )
  })

  // Populate form when jd changes
  $effect(() => {
    if (jd && !createMode) {
      title = jd.title
      organizationId = jd.organization_id ?? null
      status = jd.status
      location = jd.location ?? ''
      salaryRange = jd.salary_range ?? ''
      url = jd.url ?? ''
      rawText = jd.raw_text
      notes = jd.notes ?? ''
      loadSkills(jd.id)
    } else if (createMode) {
      title = ''
      organizationId = null
      status = 'interested'
      location = ''
      salaryRange = ''
      url = ''
      rawText = ''
      notes = ''
      jdSkills = []
    }
  })

  async function loadSkills(jdId: string) {
    const res = await forge.jobDescriptions.listSkills(jdId)
    if (res.ok) {
      jdSkills = res.data
    }
  }

  async function handleStatusChange(e: Event) {
    const newStatus = (e.target as HTMLSelectElement).value
    if (!jd || createMode) {
      status = newStatus
      return
    }
    // Immediate save for status change in edit mode
    const res = await forge.jobDescriptions.update(jd.id, { status: newStatus as any })
    if (res.ok) {
      status = newStatus
      onupdated(res.data)
      addToast({ type: 'success', message: `Status changed to ${newStatus}` })
    } else {
      addToast({ type: 'error', message: friendlyError(res.error) })
    }
  }

  async function handleSave() {
    if (!title.trim()) {
      addToast({ type: 'error', message: 'Title is required' })
      return
    }
    if (!rawText.trim()) {
      addToast({ type: 'error', message: 'Description is required' })
      return
    }

    saving = true
    const payload = {
      title: title.trim(),
      organization_id: organizationId,
      status: status as any,
      location: location.trim() || null,
      salary_range: salaryRange.trim() || null,
      url: url.trim() || null,
      raw_text: rawText.trim(),
      notes: notes.trim() || null,
    }

    if (createMode) {
      const res = await forge.jobDescriptions.create(payload as any)
      if (res.ok) {
        oncreated(res.data)
        addToast({ type: 'success', message: 'Job description created' })
      } else {
        addToast({ type: 'error', message: friendlyError(res.error) })
      }
    } else if (jd) {
      const res = await forge.jobDescriptions.update(jd.id, payload)
      if (res.ok) {
        onupdated(res.data)
        addToast({ type: 'success', message: 'Job description updated' })
      } else {
        addToast({ type: 'error', message: friendlyError(res.error) })
      }
    }
    saving = false
  }

  async function handleDelete() {
    if (!jd) return
    const res = await forge.jobDescriptions.delete(jd.id)
    if (res.ok) {
      ondeleted(jd.id)
      addToast({ type: 'success', message: 'Job description deleted' })
    } else {
      addToast({ type: 'error', message: friendlyError(res.error) })
    }
    confirmDeleteOpen = false
  }
</script>

<div class="editor">
  <div class="field">
    <label for="jd-title">Title <span class="required">*</span></label>
    <input id="jd-title" type="text" bind:value={title} placeholder="Job title" />
  </div>

  <div class="field">
    <label for="jd-org">Organization</label>
    <select id="jd-org" bind:value={organizationId}>
      <option value={null}>None</option>
      {#each organizations.sort((a, b) => a.name.localeCompare(b.name)) as org (org.id)}
        <option value={org.id}>{org.name}</option>
      {/each}
    </select>
  </div>

  <div class="field">
    <label for="jd-status">Status</label>
    <select id="jd-status" value={status} onchange={handleStatusChange}>
      <option value="interested">Interested</option>
      <option value="analyzing">Analyzing</option>
      <option value="applied">Applied</option>
      <option value="interviewing">Interviewing</option>
      <option value="offered">Offered</option>
      <option value="rejected">Rejected</option>
      <option value="withdrawn">Withdrawn</option>
      <option value="closed">Closed</option>
    </select>
  </div>

  <div class="field-row">
    <div class="field half">
      <label for="jd-location">Location</label>
      <input id="jd-location" type="text" bind:value={location} placeholder="Remote, San Francisco, CA" />
    </div>
    <div class="field half">
      <label for="jd-salary">Salary Range</label>
      <input id="jd-salary" type="text" bind:value={salaryRange} placeholder="$150k-$200k" />
    </div>
  </div>

  <div class="field">
    <label for="jd-url">URL</label>
    <div class="url-field">
      <input id="jd-url" type="url" bind:value={url} placeholder="https://..." />
      {#if url.trim() && !createMode}
        <a href={url} target="_blank" rel="noopener noreferrer" class="url-link">
          Open &#8599;
        </a>
      {/if}
    </div>
  </div>

  <div class="field">
    <label for="jd-description">Description <span class="required">*</span></label>
    <textarea
      id="jd-description"
      bind:value={rawText}
      placeholder="Paste the full job description here..."
      rows="10"
      class="description-textarea"
    ></textarea>
  </div>

  {#if !createMode && jd}
    <div class="field">
      <label>Required Skills</label>
      <JDSkillPicker jdId={jd.id} bind:jdSkills />
    </div>
  {/if}

  <div class="field">
    <label for="jd-notes">Notes</label>
    <textarea id="jd-notes" bind:value={notes} placeholder="Your private notes..." rows="4"></textarea>
  </div>

  <div class="actions">
    <button
      class="btn-primary"
      onclick={handleSave}
      disabled={saving || (!createMode && !isDirty)}
    >
      {saving ? 'Saving...' : createMode ? 'Create' : 'Save'}
    </button>
    {#if !createMode && jd}
      <button
        class="btn-danger"
        onclick={() => (confirmDeleteOpen = true)}
        type="button"
      >
        Delete
      </button>
    {/if}
  </div>
</div>

{#if confirmDeleteOpen}
  <ConfirmDialog
    title="Delete Job Description"
    message="Are you sure you want to delete this job description? This cannot be undone."
    onconfirm={handleDelete}
    oncancel={() => (confirmDeleteOpen = false)}
  />
{/if}

<style>
  .editor {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    padding: 1rem;
    overflow-y: auto;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .field-row {
    display: flex;
    gap: 1rem;
  }

  .half {
    flex: 1;
  }

  label {
    font-size: 0.8rem;
    font-weight: 600;
    color: #374151;
  }

  .required {
    color: #ef4444;
  }

  input[type="text"],
  input[type="url"],
  select,
  textarea {
    padding: 0.5rem 0.6rem;
    border: 1px solid #d1d5db;
    border-radius: 0.375rem;
    font-size: 0.9rem;
    outline: none;
    font-family: inherit;
  }

  input:focus,
  select:focus,
  textarea:focus {
    border-color: #3b82f6;
    box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.15);
  }

  .description-textarea {
    min-height: 200px;
    white-space: pre-wrap;
    font-family: inherit;
    resize: vertical;
  }

  .url-field {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .url-field input {
    flex: 1;
  }

  .url-link {
    font-size: 0.8rem;
    color: #3b82f6;
    text-decoration: none;
    white-space: nowrap;
  }

  .url-link:hover {
    text-decoration: underline;
  }

  .actions {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding-top: 0.5rem;
    border-top: 1px solid #e5e7eb;
  }

  .btn-primary {
    padding: 0.5rem 1.25rem;
    background: #3b82f6;
    color: #fff;
    border: none;
    border-radius: 0.375rem;
    font-weight: 600;
    cursor: pointer;
    font-size: 0.9rem;
  }

  .btn-primary:hover:not(:disabled) {
    background: #2563eb;
  }

  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-danger {
    padding: 0.5rem 1.25rem;
    background: none;
    color: #ef4444;
    border: 1px solid #ef4444;
    border-radius: 0.375rem;
    font-weight: 600;
    cursor: pointer;
    font-size: 0.9rem;
  }

  .btn-danger:hover {
    background: #fef2f2;
  }
</style>
```

**Acceptance criteria:**
- Create mode shows empty form with "Create" button.
- Edit mode populates from selected JD with "Save" button (disabled until dirty).
- Status dropdown saves immediately on change in edit mode (no Save click).
- Organization dropdown shows all orgs alphabetically + "None" option.
- URL field shows clickable external link when populated.
- Title and description are required; validation errors show as toasts.
- Delete shows ConfirmDialog.
- Skill picker only appears in edit mode (skills can't be tagged until JD is created).
- Description textarea has min-height 200px and preserves whitespace.

**Failure criteria:**
- Status change requires clicking Save.
- Organization dropdown missing "None" option.
- No dirty tracking (Save always enabled).
- Skills available in create mode before JD has an ID.

---

### T49.9: Write Main Page Component

**File:** `packages/webui/src/routes/opportunities/job-descriptions/+page.svelte`

[CRITICAL] Replaces the existing `EmptyState` placeholder with the full split-panel layout.

```svelte
<!--
  JD Detail Page -- split-panel layout with list + editor.
  Manages JD list state, selection, create/edit mode, and filters.
-->
<script lang="ts">
  import { forge, friendlyError } from '$lib/sdk'
  import { addToast } from '$lib/stores/toast.svelte'
  import { LoadingSpinner, EmptyState } from '$lib/components'
  import JDCard from '$lib/components/jd/JDCard.svelte'
  import JDEditor from '$lib/components/jd/JDEditor.svelte'
  import type { JobDescriptionWithOrg, Organization, JobDescriptionStatus } from '@forge/sdk'

  let jds = $state<JobDescriptionWithOrg[]>([])
  let organizations = $state<Organization[]>([])
  let selectedId = $state<string | null>(null)
  let createMode = $state(false)
  let statusFilter = $state<JobDescriptionStatus | 'all'>('all')
  let searchText = $state('')
  let loading = $state(true)

  let filteredJds = $derived.by(() => {
    let result = jds
    if (statusFilter !== 'all') {
      result = result.filter(jd => jd.status === statusFilter)
    }
    if (searchText.trim()) {
      const q = searchText.toLowerCase()
      result = result.filter(jd =>
        jd.title.toLowerCase().includes(q) ||
        jd.organization_name?.toLowerCase().includes(q)
      )
    }
    return result
  })

  let selectedJd = $derived(jds.find(jd => jd.id === selectedId) ?? null)

  $effect(() => {
    loadData()
  })

  async function loadData() {
    loading = true
    const [jdRes, orgRes] = await Promise.all([
      forge.jobDescriptions.list({ limit: 200 }),
      forge.organizations.list({ limit: 500 }),
    ])

    if (jdRes.ok) {
      jds = jdRes.data
    } else {
      addToast({ type: 'error', message: friendlyError(jdRes.error) })
    }

    if (orgRes.ok) {
      organizations = orgRes.data
    }
    loading = false
  }

  async function refreshJds() {
    const res = await forge.jobDescriptions.list({ limit: 200 })
    if (res.ok) {
      jds = res.data
    }
  }

  function selectJd(id: string) {
    createMode = false
    selectedId = id
  }

  function startCreate() {
    selectedId = null
    createMode = true
  }

  function handleCreated(jd: JobDescriptionWithOrg) {
    createMode = false
    selectedId = jd.id
    refreshJds()
  }

  function handleUpdated(jd: JobDescriptionWithOrg) {
    refreshJds()
  }

  function handleDeleted(id: string) {
    selectedId = null
    refreshJds()
  }
</script>

<div class="jd-page">
  {#if loading}
    <div class="loading-container">
      <LoadingSpinner />
    </div>
  {:else}
    <div class="split-panel">
      <!-- List Panel -->
      <div class="list-panel">
        <div class="list-header">
          <h2 class="panel-title">Job Descriptions</h2>
          <button class="btn-new" onclick={startCreate} type="button">
            + New JD
          </button>
        </div>

        <div class="list-filters">
          <select
            class="status-filter"
            bind:value={statusFilter}
          >
            <option value="all">All Statuses</option>
            <option value="interested">Interested</option>
            <option value="analyzing">Analyzing</option>
            <option value="applied">Applied</option>
            <option value="interviewing">Interviewing</option>
            <option value="offered">Offered</option>
            <option value="rejected">Rejected</option>
            <option value="withdrawn">Withdrawn</option>
            <option value="closed">Closed</option>
          </select>
          <input
            type="text"
            class="search-input"
            placeholder="Search title or org..."
            bind:value={searchText}
          />
        </div>

        <div class="card-list">
          {#if filteredJds.length === 0}
            <p class="empty-list">No job descriptions match your filters.</p>
          {:else}
            {#each filteredJds as jd (jd.id)}
              <JDCard
                {jd}
                selected={selectedId === jd.id}
                onclick={() => selectJd(jd.id)}
              />
            {/each}
          {/if}
        </div>
      </div>

      <!-- Editor Panel -->
      <div class="editor-panel">
        {#if createMode}
          <JDEditor
            jd={null}
            {organizations}
            createMode={true}
            oncreated={handleCreated}
            onupdated={handleUpdated}
            ondeleted={handleDeleted}
          />
        {:else if selectedJd}
          {#key selectedJd.id}
            <JDEditor
              jd={selectedJd}
              {organizations}
              createMode={false}
              oncreated={handleCreated}
              onupdated={handleUpdated}
              ondeleted={handleDeleted}
            />
          {/key}
        {:else}
          <div class="empty-editor">
            <EmptyState
              title="No job description selected"
              description="Select a job description or create a new one"
            />
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .jd-page {
    height: 100%;
    display: flex;
    flex-direction: column;
  }

  .loading-container {
    display: flex;
    justify-content: center;
    align-items: center;
    height: 300px;
  }

  .split-panel {
    display: flex;
    height: 100%;
    min-height: 0;
  }

  .list-panel {
    width: 320px;
    min-width: 280px;
    border-right: 1px solid #e5e7eb;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .list-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem 1rem;
    border-bottom: 1px solid #e5e7eb;
  }

  .panel-title {
    font-size: 1rem;
    font-weight: 700;
    color: #1a1a2e;
    margin: 0;
  }

  .btn-new {
    padding: 0.35rem 0.75rem;
    background: #3b82f6;
    color: #fff;
    border: none;
    border-radius: 0.375rem;
    font-size: 0.8rem;
    font-weight: 600;
    cursor: pointer;
  }

  .btn-new:hover {
    background: #2563eb;
  }

  .list-filters {
    display: flex;
    gap: 0.5rem;
    padding: 0.5rem 1rem;
    border-bottom: 1px solid #f3f4f6;
  }

  .status-filter {
    flex: 0 0 auto;
    padding: 0.35rem 0.5rem;
    border: 1px solid #d1d5db;
    border-radius: 0.375rem;
    font-size: 0.8rem;
    outline: none;
  }

  .search-input {
    flex: 1;
    padding: 0.35rem 0.5rem;
    border: 1px solid #d1d5db;
    border-radius: 0.375rem;
    font-size: 0.8rem;
    outline: none;
  }

  .search-input:focus,
  .status-filter:focus {
    border-color: #3b82f6;
  }

  .card-list {
    flex: 1;
    overflow-y: auto;
    padding: 0.5rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .empty-list {
    text-align: center;
    color: #9ca3af;
    padding: 2rem 1rem;
    font-size: 0.85rem;
  }

  .editor-panel {
    flex: 1;
    overflow-y: auto;
    min-width: 0;
  }

  .empty-editor {
    display: flex;
    justify-content: center;
    align-items: center;
    height: 100%;
  }
</style>
```

[IMPORTANT] The `{#key selectedJd.id}` block on the editor forces a re-mount when the selected JD changes. This ensures the `$effect` inside `JDEditor` re-fires to load skills for the new JD and repopulate form fields.

**Acceptance criteria:**
- Split-panel renders with list on left (320px) and editor on right.
- "New JD" button switches to create mode with empty form.
- Creating a JD adds it to the list and auto-selects it.
- Clicking a card selects it and populates the editor.
- Status filter dropdown reduces visible cards.
- Search filter matches on title and organization name (case-insensitive).
- Empty state shows when no JD is selected and not in create mode.
- Loading spinner shows during initial data load.
- JD list refreshes after any mutation (create, update, delete).

**Failure criteria:**
- Editor does not re-mount on JD selection change (stale form data).
- Filters apply to the full JD set rather than the displayed list.
- Page exceeds viewport height (scroll must be within panels, not page-level).

---

### T49.10: Update Server Route Registration

**File:** `packages/core/src/routes/server.ts`

[IMPORTANT] Update the `jobDescriptionRoutes` call at line 114 to pass `db`.

```typescript
// Before (line 114):
app.route('/', jobDescriptionRoutes(services))

// After:
app.route('/', jobDescriptionRoutes(services, db))
```

**Acceptance criteria:**
- `db` parameter is passed to `jobDescriptionRoutes`.
- No TypeScript compilation errors.

**Failure criteria:**
- Calling JD skills endpoints results in `db is not defined` runtime error.

---

## Testing Support

### Unit Tests

| Test file | Tests |
|-----------|-------|
| `packages/core/src/db/repositories/__tests__/job-description-repository.test.ts` | Verify `list()` returns results ordered by `updated_at DESC` |

### Integration Tests

| Test file | Tests |
|-----------|-------|
| `packages/core/src/routes/__tests__/job-description-skills.test.ts` | `GET /api/job-descriptions/:id/skills` returns empty array for JD with no skills |
| | `POST /api/job-descriptions/:id/skills` with `{ skill_id }` links existing skill (201) |
| | `POST /api/job-descriptions/:id/skills` with `{ skill_id }` duplicate is idempotent |
| | `POST /api/job-descriptions/:id/skills` with `{ name: 'python' }` creates "Python" and links it |
| | `POST /api/job-descriptions/:id/skills` with `{ name: 'python' }` when "Python" exists links existing |
| | `POST /api/job-descriptions/:id/skills` with neither returns 400 |
| | `DELETE /api/job-descriptions/:jdId/skills/:skillId` removes junction row (204) |
| | `GET /api/job-descriptions/:id/skills` after linking returns correct skills |

### Migration Tests

| Test file | Tests |
|-----------|-------|
| `packages/core/src/db/__tests__/migration-018.test.ts` | Migration 018 creates `job_description_skills` table |
| | Composite PK prevents duplicate links |
| | Deleting a JD cascades to junction rows |
| | Deleting a skill cascades to junction rows |
| | Indexes `idx_jd_skills_jd` and `idx_jd_skills_skill` exist |

### Component Tests

| Component | Tests |
|-----------|-------|
| `JDCard.svelte` | Renders title, org name, status badge, location, salary |
| | Shows "No organization" when `organization_name` is null |
| | Selected state applies correct CSS class |
| `JDEditor.svelte` | Create mode: empty form, "Create" button |
| | Edit mode: populated form, "Save" disabled until dirty |
| | Status dropdown triggers immediate save in edit mode |
| | Delete shows ConfirmDialog |
| | Validation errors for empty title/description |
| `JDSkillPicker.svelte` | Renders linked skills as pills |
| | Search input filters dropdown |
| | Adding skill removes it from dropdown |
| | Remove button calls DELETE and removes pill |
| `+page.svelte` | Split-panel layout renders |
| | Status filter and search filter work |
| | "New JD" enters create mode |
| | Card click selects and populates editor |

### SDK Tests

| Test file | Tests |
|-----------|-------|
| `packages/sdk/src/__tests__/job-descriptions.test.ts` | `listSkills(jdId)` returns `Result<Skill[]>` |
| | `addSkill(jdId, { skill_id })` returns `Result<Skill>` |
| | `addSkill(jdId, { name })` returns `Result<Skill>` |
| | `removeSkill(jdId, skillId)` returns `Result<void>` |

### Smoke Tests

- Navigate to `/opportunities/job-descriptions` and see split-panel layout.
- Create a JD with title and description; verify it appears in the list.
- Click a JD card; verify the editor populates.
- Change status dropdown; verify immediate save (toast appears).
- Add a skill via the picker; verify it persists on page reload.
- Remove a skill; verify it disappears.
- Filter by status and search text; verify card list updates.
- Delete a JD; verify it is removed from the list.

---

## Documentation Requirements

- No new documentation files. Component usage is self-evident from the split-panel pattern established by other data pages.
- Update the spec tracker if one exists to mark E1 as in-progress or complete.

---

## Parallelization Notes

- **T49.1** (migration) and **T49.2** (repository update) can run in parallel.
- **T49.3** (routes) depends on T49.1 (migration must exist for integration tests).
- **T49.4** (SDK) can run in parallel with T49.3 (no file conflicts).
- **T49.5** (StatusBadge) can run in parallel with all other tasks.
- **T49.6** (JDCard) can run in parallel with T49.7 and T49.8 (they share no files).
- **T49.7** (JDSkillPicker) depends on T49.4 (SDK methods must exist).
- **T49.8** (JDEditor) depends on T49.6, T49.7 (imports both sub-components).
- **T49.9** (main page) depends on T49.6, T49.7, T49.8 (imports all sub-components).
- **T49.10** (server.ts) depends on T49.3 (route signature change).

Dependency graph:

```
T49.1 (migration) ──────────┐
T49.2 (repo sort) ──────────┤
T49.5 (StatusBadge) ─────── │ ── can all start immediately
T49.6 (JDCard) ──────────── │
                             │
T49.3 (routes) ─── depends on T49.1
T49.4 (SDK) ──── parallel with T49.3
T49.10 (server.ts) ─── depends on T49.3
T49.7 (SkillPicker) ─── depends on T49.4
T49.8 (JDEditor) ─── depends on T49.6, T49.7
T49.9 (main page) ─── depends on T49.6, T49.7, T49.8
```
