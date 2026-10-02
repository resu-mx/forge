# Phase 61: JD Kanban Pipeline (Spec E3)

**Status:** Planning
**Date:** 2026-04-03
**Spec:** [2026-04-03-jd-kanban-pipeline.md](../refs/specs/2026-04-03-jd-kanban-pipeline.md)
**Depends on:** Phase 49 (JD Detail Page -- JD entity with CRUD UI), Phase 43 (Generic Kanban -- `GenericKanban.svelte`, `ViewToggle.svelte`, column/card infrastructure). **Phase 61 is BLOCKED until Phase 43's GenericKanban API is stable and committed. This is a hard prerequisite, not just a soft dependency.**
**Blocks:** None
**Parallelizable with:** Phase 60 (JD Resume Linkage), Phase 62 (JD Skill Extraction AI) -- Phase 61 modifies shared files (`job-descriptions.ts` routes, core/SDK types, `StatusBadge.svelte`); serialize changes to those files

## Goal

Add a kanban board view for the JD application pipeline at `/opportunities/job-descriptions` with 7 columns mapping to 10 status values. Expand `JobDescriptionStatus` via migration 020 (table rebuild) to add `discovered` and `applying`, rename `interested` to `discovered`. Integrate `GenericKanban` from Phase 43 with JD-specific column definitions, card renderer, and filter bar. Provide a list/board view toggle persisted in localStorage.

## Non-Goals

- Sub-step checklists within pipeline stages (data model documented in spec, NOT implemented)
- Automated status transitions (e.g., auto-move to "Applied" when a resume is linked)
- Application deadline tracking or reminders
- Interview scheduling integration
- Offer comparison tooling
- Pipeline analytics or conversion rate metrics
- Drag-and-drop card reordering within a column (no position field)
- Column customization or user-defined columns
- Batch operations (multi-select, bulk status change)
- Mobile/responsive kanban layout

## Context

Phase 49 builds the JD detail page as a split-panel layout (list + editor) with CRUD and skill tagging. Phase 43 extracts a reusable `GenericKanban.svelte` with Svelte 5 generics, `svelte-dnd-action` for drag-and-drop, column definitions, and card renderer snippets. The JD pipeline has 7 columns mapping to 10 statuses -- fundamentally different from the unified 5-status model (Phase 43) because JDs track an external hiring lifecycle. The Closed column groups three terminal statuses (`rejected`, `withdrawn`, `closed`).

The current `JobDescriptionStatus` type has 8 values: `interested | analyzing | applied | interviewing | offered | rejected | withdrawn | closed`. This phase adds `discovered` (replacing `interested`) and `applying` (new), bringing the total to 9 distinct values across 7 columns.

## Scope

| Spec section | Covered here? |
|-------------|---------------|
| 1. Pipeline columns (7 columns, 10 statuses, accent colors) | Yes |
| 2. Schema migration (020: table rebuild, status expansion) | Yes |
| 3. Type changes (core + SDK `JobDescriptionStatus`) | Yes |
| 4. GenericKanban integration (usage, drop handler, data loading, view toggle) | Yes |
| 5. JD kanban card (component, content, styling) | Yes |
| 6. Filter bar (Organization, Location, Search) | Yes |
| 7. Sub-step checklists (deferred -- data model only) | No (documented in spec only) |
| 8. Files to create | Yes |
| 9. Files to modify | Yes |
| 10. Status badge colors | Yes |
| 11. Testing | Yes |
| 12. Acceptance criteria | Yes |

## Files to Create

| File | Description |
|------|-------------|
| `packages/core/src/db/migrations/020_jd_pipeline_statuses.sql` | Table rebuild migration: expand JD status CHECK, add `discovered`/`applying`, rename `interested` -> `discovered` |
| `packages/webui/src/lib/components/kanban/JDKanbanCard.svelte` | JD card content for kanban board |
| `packages/webui/src/lib/components/filters/JDFilterBar.svelte` | JD filter controls (Organization, Location, Search) |

## Files to Modify

| File | Change |
|------|--------|
| `packages/core/src/types/index.ts` | Update `JobDescriptionStatus` union: add `discovered`, `applying`; remove `interested` |
| `packages/sdk/src/types.ts` | Mirror `JobDescriptionStatus` change |
| `packages/core/src/constants/index.ts` | Add `JD_PIPELINE_STATUSES` and `JD_PIPELINE_COLUMNS` constants |
| `packages/core/src/routes/job-descriptions.ts` | Update status validation in POST/PATCH handlers: accept `discovered`/`applying`, reject `interested` |
| `packages/core/src/db/repositories/job-description-repository.ts` | Update any hardcoded status references (e.g., default status `interested` -> `discovered`) |
| `packages/webui/src/routes/opportunities/job-descriptions/+page.svelte` | Add view toggle (List/Board), integrate GenericKanban in board mode |
| `packages/webui/src/lib/components/jd/JDEditor.svelte` | Update status dropdown options: replace `interested` with `discovered`, add `applying` |
| `packages/webui/src/lib/components/jd/JDCard.svelte` | Update status display for new values |
| `packages/webui/src/lib/components/StatusBadge.svelte` | Add color mappings for `discovered` and `applying` statuses |

## Fallback Strategies

- **Migration 020 runs on empty database:** The `INSERT INTO job_descriptions_new ... SELECT FROM job_descriptions` simply inserts zero rows. No crash. The new table is created with the expanded CHECK constraint.
- **JDs with `interested` status:** The migration's `CASE WHEN status = 'interested' THEN 'discovered' ELSE status END` ensures all existing data is migrated. No orphaned status values.
- **Junction table safety during table rebuild:** `PRAGMA foreign_keys = OFF` prevents cascade deletes during `DROP TABLE job_descriptions` + `ALTER TABLE job_descriptions_new RENAME TO job_descriptions`. Both `job_description_skills` and `job_description_resumes` continue to reference valid IDs after the rebuild.
- **GenericKanban not yet available (Phase 43 incomplete):** If Phase 43 is not yet merged, the board view cannot render. The page defaults to list view and the board toggle is hidden until GenericKanban is importable. Implement the board view behind a dynamic import with a fallback.
- **Empty kanban board:** Shows "No job descriptions yet. Create one to start tracking your pipeline." via GenericKanban's `emptyMessage` prop.
- **Closed column with many cards:** The column is collapsed by default. Expanding it shows all cards with `opacity: 0.6` for visual distinction.
- **Organization dropdown empty (filter bar):** The dropdown shows "All organizations" as the default option. No crash if no orgs exist.
- **View mode not in localStorage:** Defaults to "list" view. The toggle writes to localStorage on change.

---

## Tasks

### T61.1: Write Migration 020 -- JD Pipeline Statuses

**File:** `packages/core/src/db/migrations/020_jd_pipeline_statuses.sql`

[CRITICAL] This migration uses the table rebuild pattern. SQLite cannot ALTER CHECK constraints, so the table must be dropped and recreated. `PRAGMA foreign_keys = OFF` MUST be set before the DROP to prevent cascade deletes on junction tables (`job_description_skills`, `job_description_resumes`). The PRAGMA is restored to ON at the end.

[CRITICAL] The `CASE WHEN status = 'interested' THEN 'discovered' ELSE status END` data migration MUST happen in the INSERT...SELECT. If this is omitted, rows with `interested` will violate the new CHECK constraint.

[IMPORTANT] The index on `job_descriptions(status)` must be recreated after the table rebuild. The index on `job_descriptions(organization_id)` must also be recreated.

```sql
-- JD Pipeline Statuses
-- Migration: 020_jd_pipeline_statuses
-- Expands job_descriptions.status CHECK for pipeline kanban.
-- Adds: discovered, applying
-- Renames: interested -> discovered
-- Uses table rebuild pattern (SQLite cannot ALTER CHECK constraints).

PRAGMA foreign_keys = OFF;
-- NOTE: PRAGMA foreign_keys = OFF is defensive only -- the runner's transaction
-- provides the actual FK protection.

CREATE TABLE job_descriptions_new (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  organization_id TEXT REFERENCES organizations(id) ON DELETE SET NULL,
  title TEXT NOT NULL,
  url TEXT,
  raw_text TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'discovered' CHECK (status IN (
    'discovered', 'analyzing', 'applying', 'applied', 'interviewing',
    'offered', 'rejected', 'withdrawn', 'closed'
  )),
  salary_range TEXT,
  location TEXT,
  notes TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

INSERT INTO job_descriptions_new (id, organization_id, title, url, raw_text,
  status, salary_range, location, notes, created_at, updated_at)
SELECT id, organization_id, title, url, raw_text,
  CASE status
    WHEN 'interested' THEN 'discovered'
    ELSE status
  END,
  salary_range, location, notes, created_at, updated_at
FROM job_descriptions;

DROP TABLE job_descriptions;
ALTER TABLE job_descriptions_new RENAME TO job_descriptions;

CREATE INDEX idx_job_descriptions_org ON job_descriptions(organization_id);
CREATE INDEX idx_job_descriptions_status ON job_descriptions(status);

-- Junction tables referencing job_descriptions(id):
-- job_description_skills: references job_descriptions(id) ON DELETE CASCADE.
-- job_description_resumes: references job_descriptions(id) ON DELETE CASCADE.
-- Same IDs preserved; no action needed.

-- [FIX] Recreate the updated_at trigger after the table rebuild.
-- Without this, updating a JD will no longer auto-set updated_at.
CREATE TRIGGER jd_updated_at AFTER UPDATE ON job_descriptions
BEGIN
  UPDATE job_descriptions SET updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = NEW.id;
END;

PRAGMA foreign_keys = ON;

INSERT INTO _migrations (name) VALUES ('020_jd_pipeline_statuses');
```

**Acceptance criteria:**
- Table rebuild succeeds without data loss.
- Existing JDs with `interested` status become `discovered`.
- All other statuses preserved as-is.
- New `applying` status can be inserted.
- `interested` status rejected by CHECK constraint post-migration.
- Default status for new JDs is `discovered`.
- Both indexes recreated.
- Junction tables (`job_description_skills`, `job_description_resumes`) remain intact.
- Migration row inserted into `_migrations`.

**Failure criteria:**
- Missing `PRAGMA foreign_keys = OFF` -- DROP TABLE cascades to junction rows.
- Missing `CASE` in INSERT...SELECT -- rows with `interested` violate new CHECK.
- Missing index recreation -- status and org_id lookups become full scans.
- Missing `STRICT` mode on new table.

---

### T61.2: Update Core Type Definitions

**File:** `packages/core/src/types/index.ts`

[CRITICAL] The `JobDescriptionStatus` union must be updated atomically with the migration. Any code reading status values will break if the type does not match the database.

Replace the existing `JobDescriptionStatus` type:

```typescript
/** Valid statuses for a JobDescription record. */
export type JobDescriptionStatus =
  | 'discovered'
  | 'analyzing'
  | 'applying'
  | 'applied'
  | 'interviewing'
  | 'offered'
  | 'rejected'
  | 'withdrawn'
  | 'closed'
```

**Acceptance criteria:**
- `interested` removed from the union.
- `discovered` and `applying` added.
- All 9 values match the migration CHECK constraint exactly.

---

### T61.3: Update SDK Type Definitions

**File:** `packages/sdk/src/types.ts`

Mirror the same `JobDescriptionStatus` change from T61.2.

```typescript
export type JobDescriptionStatus =
  | 'discovered'
  | 'analyzing'
  | 'applying'
  | 'applied'
  | 'interviewing'
  | 'offered'
  | 'rejected'
  | 'withdrawn'
  | 'closed'
```

**Acceptance criteria:**
- SDK type matches core type exactly.
- All downstream SDK consumers see the updated union.

---

### T61.4: Add Pipeline Constants

**File:** `packages/core/src/constants/index.ts`

[IMPORTANT] The `ColumnDef` type must match `GenericKanban`'s expected interface from Phase 43. Verify the exact interface shape before implementation. The `dropStatus` field on the Closed column tells GenericKanban which status to set when a card is dropped into a column that maps to multiple statuses.

```typescript
// [FIX] Do NOT import ColumnDef from @forge/webui — that creates a circular
// dependency (core -> webui -> core). Define the ColumnDef type locally in
// packages/core/src/constants/ or in packages/sdk/src/types.ts so it can be
// shared without circular imports. Alternatively, define JD_PIPELINE_COLUMNS
// directly in the webui package.
export interface ColumnDef {
  key: string
  label: string
  statuses: string[]
  accent: string
  dropStatus?: string
}

export const JD_PIPELINE_STATUSES = [
  'discovered', 'analyzing', 'applying', 'applied',
  'interviewing', 'offered', 'rejected', 'withdrawn', 'closed',
] as const

export const JD_PIPELINE_COLUMNS: ColumnDef[] = [
  { key: 'discovered', label: 'Discovered', statuses: ['discovered'], accent: '#a5b4fc' },
  { key: 'analyzing', label: 'Analyzing', statuses: ['analyzing'], accent: '#60a5fa' },
  { key: 'applying', label: 'Applying', statuses: ['applying'], accent: '#fbbf24' },
  { key: 'applied', label: 'Applied', statuses: ['applied'], accent: '#818cf8' },
  { key: 'interviewing', label: 'Interviewing', statuses: ['interviewing'], accent: '#a78bfa' },
  { key: 'offered', label: 'Offered', statuses: ['offered'], accent: '#22c55e' },
  {
    key: 'closed',
    label: 'Closed',
    statuses: ['rejected', 'withdrawn', 'closed'],
    dropStatus: 'closed',
    accent: '#d1d5db',
  },
]
```

[RESOLVED] The `ColumnDef` type is now defined inline in `packages/core/src/constants/index.ts` to avoid circular dependencies between core and webui. If Phase 43 defines its own `ColumnDef`, the core version should be canonical and Phase 43's should re-export or alias it.

**Acceptance criteria:**
- `JD_PIPELINE_STATUSES` is a readonly tuple of all 9 status values.
- `JD_PIPELINE_COLUMNS` has 7 entries with correct labels, statuses, and accent colors.
- Closed column has `dropStatus: 'closed'` and `statuses: ['rejected', 'withdrawn', 'closed']`.
- Column order matches the pipeline progression (Discovered -> Closed).

---

### T61.5: Update Route Status Validation

**File:** `packages/core/src/routes/job-descriptions.ts`

[IMPORTANT] If the route layer delegates status validation to the service layer or repository, this change may need to happen in `JobDescriptionService` or `JobDescriptionRepository` instead. Verify where the `VALID_STATUSES` set is defined.

[IMPORTANT] The routes file uses `services.jobDescriptions.create(body)` and `services.jobDescriptions.update(id, body)` which pass through to the service layer. If the service validates status values, update the service's valid statuses list.

**File (if validation is in service):** `packages/core/src/services/job-description-service.ts`

Update the valid statuses set (wherever it is defined):

```typescript
const VALID_JD_STATUSES = new Set([
  'discovered', 'analyzing', 'applying', 'applied',
  'interviewing', 'offered', 'rejected', 'withdrawn', 'closed',
])
```

[ANTI-PATTERN] Hardcoding the valid statuses in multiple places (service, repository, migration CHECK) creates drift risk. Consider importing `JD_PIPELINE_STATUSES` from constants. However, the service and repository may be in `packages/core` which already has access to the constants file.

**File:** `packages/core/src/db/repositories/job-description-repository.ts`

Update the default status from `'interested'` to `'discovered'`:

```typescript
// In create method, change default:
const status = input.status ?? 'discovered'
```

**Acceptance criteria:**
- `POST /api/job-descriptions` without explicit status creates JD with `discovered`.
- `PATCH /api/job-descriptions/:id { status: 'discovered' }` succeeds.
- `PATCH /api/job-descriptions/:id { status: 'applying' }` succeeds.
- `PATCH /api/job-descriptions/:id { status: 'interested' }` returns 400 (rejected by CHECK or validation).
- All 9 valid statuses accepted.

---

### T61.6: Build JDKanbanCard Component

**File:** `packages/webui/src/lib/components/kanban/JDKanbanCard.svelte`

[IMPORTANT] The card content must be dense -- kanban cards should be scannable at a glance. Truncate the title at ~50 characters with ellipsis. Show up to 3 skill pills with "+N more" overflow.

[MINOR] The skill pills require the JD to have skills tagged (from Phase 49 / Spec E1). If skills are not loaded with the JD list data, they will not be available on the card. Verify whether `forge.jobDescriptions.list()` returns skills or if a separate fetch is needed. If skills are not included, omit skill pills from the card (they can be added when the API supports eager loading).

```svelte
<script lang="ts">
  import type { JobDescriptionWithOrg } from '@forge/sdk/types'

  let { jd, onclick }: {
    jd: JobDescriptionWithOrg & { skills?: Array<{ name: string }> }
    onclick: () => void
  } = $props()

  let truncatedTitle = $derived(
    jd.title.length > 50 ? jd.title.slice(0, 47) + '...' : jd.title
  )

  let displaySkills = $derived(jd.skills?.slice(0, 3) ?? [])
  let overflowCount = $derived(Math.max(0, (jd.skills?.length ?? 0) - 3))

  let isOffered = $derived(jd.status === 'offered')
  let isClosed = $derived(['rejected', 'withdrawn', 'closed'].includes(jd.status))
  let isRejected = $derived(jd.status === 'rejected')
  let isWithdrawn = $derived(jd.status === 'withdrawn')
</script>

<button
  class="jd-kanban-card"
  class:offered={isOffered}
  class:closed={isClosed}
  class:rejected={isRejected}
  class:withdrawn={isWithdrawn}
  {onclick}
>
  <div class="card-title">{truncatedTitle}</div>

  {#if jd.organization_name}
    <div class="card-org muted">{jd.organization_name}</div>
  {/if}

  {#if jd.location || jd.salary_range}
    <div class="card-meta muted">
      {#if jd.location}{jd.location}{/if}
      {#if jd.location && jd.salary_range} &bull; {/if}
      {#if jd.salary_range}{jd.salary_range}{/if}
    </div>
  {/if}

  {#if displaySkills.length > 0}
    <div class="card-skills">
      {#each displaySkills as skill}
        <span class="skill-pill">{skill.name}</span>
      {/each}
      {#if overflowCount > 0}
        <span class="skill-pill overflow">+{overflowCount} more</span>
      {/if}
    </div>
  {/if}

  {#if isClosed}
    <span class="sub-status-badge sub-status-{jd.status}">{jd.status}</span>
  {/if}
</button>

<style>
  .jd-kanban-card {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 8px 12px;
    border: 1px solid var(--border-color, #e5e7eb);
    border-radius: 6px;
    background: white;
    cursor: pointer;
    text-align: left;
    width: 100%;
  }

  .jd-kanban-card:hover {
    border-color: var(--border-hover, #d1d5db);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.08);
  }

  .jd-kanban-card.offered {
    background: #f0fdf4;
    border-left: 3px solid #22c55e;
  }

  .jd-kanban-card.closed {
    opacity: 0.6;
  }

  .jd-kanban-card.rejected {
    border-left: 3px solid #ef4444;
  }

  .jd-kanban-card.withdrawn {
    border-left: 3px solid #f97316;
  }

  .card-title {
    font-weight: 600;
    font-size: 0.875rem;
    line-height: 1.25;
  }

  .card-org, .card-meta {
    font-size: 0.75rem;
    color: var(--text-muted, #6b7280);
  }

  .card-skills {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 2px;
  }

  .skill-pill {
    font-size: 0.625rem;
    padding: 1px 6px;
    border-radius: 9999px;
    background: var(--pill-bg, #f3f4f6);
    color: var(--pill-text, #4b5563);
  }

  .skill-pill.overflow {
    color: var(--text-muted, #6b7280);
    font-style: italic;
  }

  .sub-status-badge {
    font-size: 0.625rem;
    padding: 1px 6px;
    border-radius: 4px;
    margin-top: 2px;
    width: fit-content;
  }

  .sub-status-rejected {
    background: #fef2f2;
    color: #dc2626;
  }

  .sub-status-withdrawn {
    background: #fff7ed;
    color: #ea580c;
  }

  .sub-status-closed {
    background: #f3f4f6;
    color: #6b7280;
  }
</style>
```

**Acceptance criteria:**
- Card renders title (truncated at 50 chars), org name, location/salary, skill pills.
- Offered cards have green background and green left border.
- Closed cards have `opacity: 0.6` with sub-status badge.
- Rejected cards have red left border; withdrawn have orange left border.
- Skill pills show up to 3 with "+N more" overflow.
- Clicking the card fires `onclick`.

**Failure criteria:**
- Title not truncated -- overflows card width.
- Missing null checks on optional fields (org, location, salary, skills).

---

### T61.7: Build JDFilterBar Component

**File:** `packages/webui/src/lib/components/filters/JDFilterBar.svelte`

[IMPORTANT] Filters are applied client-side after fetching all JDs. This is consistent with the org kanban pattern from Phase 38 and GenericKanban from Phase 43. Filters combine with AND logic.

```svelte
<script lang="ts">
  import { onMount } from 'svelte'
  import type { ForgeClient } from '@forge/sdk'

  let { filters, forge, onchange }: {
    filters: { organization_id: string; location: string; search: string }
    forge: ForgeClient
    onchange: () => void
  } = $props()

  interface OrgOption {
    id: string
    name: string
  }

  let organizations = $state<OrgOption[]>([])

  // [FIX] Use onMount for org loading instead of $effect.
  // $effect would re-run when `organizations` is written, causing an
  // infinite loop (effect reads forge -> writes organizations -> re-runs).
  onMount(() => {
    forge.organizations.list({ limit: 500 }).then(result => {
      if (result.ok) {
        organizations = result.data.map(o => ({ id: o.id, name: o.name }))
      }
    })
  })
</script>

<div class="filter-bar">
  <select
    class="filter-select"
    bind:value={filters.organization_id}
    onchange={onchange}
  >
    <option value="">All organizations</option>
    {#each organizations as org (org.id)}
      <option value={org.id}>{org.name}</option>
    {/each}
  </select>

  <input
    type="text"
    class="filter-input"
    placeholder="Filter by location..."
    bind:value={filters.location}
    oninput={onchange}
  />

  <input
    type="text"
    class="filter-input"
    placeholder="Search title or org..."
    bind:value={filters.search}
    oninput={onchange}
  />
</div>

<style>
  .filter-bar {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    padding: 8px 0;
  }

  .filter-select, .filter-input {
    font-size: 0.875rem;
    padding: 4px 8px;
    border: 1px solid var(--border-color, #e5e7eb);
    border-radius: 4px;
    background: white;
  }

  .filter-input {
    min-width: 160px;
  }
</style>
```

**Acceptance criteria:**
- Organization dropdown populated from API.
- Location text input filters by substring.
- Search text input filters by title and organization_name.
- All filters emit `onchange` callback for parent to apply filtering.
- Empty filter value means "show all" for that dimension.

---

### T61.8: Integrate GenericKanban on JD Page

**File:** `packages/webui/src/routes/opportunities/job-descriptions/+page.svelte`

[CRITICAL] This task modifies the existing JD page to add a List/Board view toggle and integrate GenericKanban in board mode. The existing split-panel layout from Phase 49 becomes the "list" view. The board view uses GenericKanban with JD-specific column definitions.

[IMPORTANT] View mode is persisted in localStorage with key `forge:viewMode:jobDescriptions`. Default is `'list'`.

[IMPORTANT] Clicking a kanban card switches to list view with that JD auto-selected (`viewMode = 'list'`, `selectedId = jd.id`). This is the recommended behavior from the spec since Phase 49 builds a full split-panel editor (not a modal).

```svelte
<script lang="ts">
  import { onMount } from 'svelte'
  import ViewToggle from '$lib/components/ViewToggle.svelte'
  import GenericKanban from '$lib/components/kanban/GenericKanban.svelte'
  import JDKanbanCard from '$lib/components/kanban/JDKanbanCard.svelte'
  import JDFilterBar from '$lib/components/filters/JDFilterBar.svelte'
  import { JD_PIPELINE_COLUMNS } from '@forge/core/constants'
  import type { JobDescriptionWithOrg, JobDescriptionStatus } from '@forge/sdk/types'
  // ... existing imports for list view components ...

  // View mode state
  const STORAGE_KEY = 'forge:viewMode:jobDescriptions'
  let viewMode = $state<'list' | 'board'>(
    (typeof localStorage !== 'undefined'
      ? localStorage.getItem(STORAGE_KEY) as 'list' | 'board'
      : null) ?? 'list'
  )

  $effect(() => {
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(STORAGE_KEY, viewMode)
    }
  })

  // Data state
  let allJDs = $state<JobDescriptionWithOrg[]>([])
  let loading = $state(true)
  let selectedId = $state<string | null>(null)

  // Filter state
  let filters = $state({ organization_id: '', location: '', search: '' })

  let filteredJDs = $derived(
    allJDs.filter(jd => {
      if (filters.organization_id && jd.organization_id !== filters.organization_id) return false
      if (filters.location && !(jd.location ?? '').toLowerCase().includes(filters.location.toLowerCase())) return false
      if (filters.search) {
        const s = filters.search.toLowerCase()
        if (
          !jd.title.toLowerCase().includes(s) &&
          !(jd.organization_name ?? '').toLowerCase().includes(s)
        ) return false
      }
      return true
    })
  )

  async function loadJDs() {
    const result = await forge.jobDescriptions.list({ limit: 500 })
    if (result.ok) allJDs = result.data
    loading = false
  }

  async function handleJDDrop(itemId: string, newStatus: string) {
    // Optimistic update.
    // [FIX] Use array reassignment via .map() instead of index mutation.
    // Array index mutation (`allJDs[idx] = ...`) does not reliably trigger
    // Svelte 5 reactivity. Full array reassignment does.
    const idx = allJDs.findIndex(jd => jd.id === itemId)
    if (idx >= 0) {
      const prev = allJDs[idx].status
      allJDs = allJDs.map((jd, i) =>
        i === idx ? { ...jd, status: newStatus as JobDescriptionStatus } : jd
      )

      const result = await forge.jobDescriptions.update(itemId, {
        status: newStatus as JobDescriptionStatus,
      })

      if (!result.ok) {
        // Revert on failure
        allJDs = allJDs.map((jd, i) =>
          i === idx ? { ...jd, status: prev } : jd
        )
      }
    }
  }

  function selectJD(id: string) {
    viewMode = 'list'
    selectedId = id
  }

  onMount(loadJDs)
</script>

<div class="jd-page">
  <div class="page-header">
    <h1>Job Descriptions</h1>
    <ViewToggle bind:mode={viewMode} />
  </div>

  {#if viewMode === 'board'}
    <GenericKanban
      columns={JD_PIPELINE_COLUMNS}
      items={filteredJDs}
      onDrop={handleJDDrop}
      {loading}
      emptyMessage="No job descriptions yet. Create one to start tracking your pipeline."
      defaultCollapsed="closed"
      sortItems={(a, b) => (b.updated_at ?? '').localeCompare(a.updated_at ?? '')}
    >
      {#snippet filterBar()}
        <JDFilterBar bind:filters {forge} onchange={() => {}} />
      {/snippet}

      {#snippet cardContent(jd)}
        <JDKanbanCard {jd} onclick={() => selectJD(jd.id)} />
      {/snippet}
    </GenericKanban>
  {:else}
    <!-- Existing split-panel list view from Phase 49 -->
    <!-- Pass selectedId to auto-select a JD when switching from board -->
    <!-- ... existing list view code ... -->
  {/if}
</div>
```

[GAP] The exact prop names for `GenericKanban` (`onDrop`, `defaultCollapsed`, `sortItems`, `filterBar` snippet, `cardContent` snippet) must match Phase 43's actual implementation. Verify the `GenericKanban.svelte` component's props interface before implementation.

[INCONSISTENCY] The `sortItems` prop takes a comparator `(a, b) => number`. If `GenericKanban` sorts internally per column, the comparator applies within each column. If it sorts globally, the column assignment must happen before sorting. Verify Phase 43's sort behavior.

**Acceptance criteria:**
- Page renders a List/Board toggle in the header, defaulting to List.
- Board view renders GenericKanban with 7 pipeline columns.
- Closed column is collapsed by default.
- Dragging a card between columns calls `PATCH /api/job-descriptions/:id { status }`.
- Optimistic update: card moves immediately; reverts on API failure.
- Filter bar appears above the board with Organization, Location, Search.
- Filters reduce visible cards across all columns (client-side AND).
- Clicking a kanban card switches to list view with that JD selected.
- View mode persists in localStorage.
- Empty board shows message.

**Failure criteria:**
- GenericKanban props mismatch -- component fails to render.
- Missing optimistic revert -- card stuck in wrong column on failure.
- View mode not persisted -- resets on page reload.

---

### T61.9: Update StatusBadge Colors

**File:** `packages/webui/src/lib/components/StatusBadge.svelte`

[IMPORTANT] Add color mappings for `discovered` and `applying` statuses. Ensure existing status colors are not affected.

Add to the status color map:

```typescript
const STATUS_COLORS: Record<string, { bg: string; text: string }> = {
  // ... existing entries ...
  discovered: { bg: '#ede9fe', text: '#7c3aed' },  // Light purple
  applying: { bg: '#fffbeb', text: '#d97706' },     // Amber
  // Ensure existing JD statuses are also present:
  analyzing: { bg: '#dbeafe', text: '#2563eb' },     // Blue
  applied: { bg: '#e0e7ff', text: '#4f46e5' },       // Indigo
  interviewing: { bg: '#ede9fe', text: '#7c3aed' },  // Purple
  offered: { bg: '#dcfce7', text: '#16a34a' },        // Green
  rejected: { bg: '#fef2f2', text: '#dc2626' },       // Red
  withdrawn: { bg: '#fff7ed', text: '#ea580c' },      // Orange
  closed: { bg: '#f3f4f6', text: '#6b7280' },         // Dark gray
}
```

[MINOR] Some of these colors may already exist in `StatusBadge.svelte` from Phase 49. Only add entries that are missing. Do not duplicate.

**Acceptance criteria:**
- `discovered` renders with light purple background.
- `applying` renders with amber background.
- All 9 JD statuses have color mappings.
- Existing status colors for other entity types are not affected.

---

### T61.10: Update JD Editor Status Dropdown

**File:** `packages/webui/src/lib/components/jd/JDEditor.svelte`

[IMPORTANT] Replace `interested` with `discovered` in the status dropdown options. Add `applying` in the correct position (between `analyzing` and `applied`).

Update the status options array:

```typescript
const JD_STATUS_OPTIONS = [
  { value: 'discovered', label: 'Discovered' },
  { value: 'analyzing', label: 'Analyzing' },
  { value: 'applying', label: 'Applying' },
  { value: 'applied', label: 'Applied' },
  { value: 'interviewing', label: 'Interviewing' },
  { value: 'offered', label: 'Offered' },
  { value: 'rejected', label: 'Rejected' },
  { value: 'withdrawn', label: 'Withdrawn' },
  { value: 'closed', label: 'Closed' },
]
```

**Acceptance criteria:**
- Dropdown shows all 9 statuses in pipeline order.
- `interested` no longer appears.
- `discovered` is the first option.
- `applying` appears between `analyzing` and `applied`.

---

### T61.11: Update JD Card Status Display

**File:** `packages/webui/src/lib/components/jd/JDCard.svelte`

[MINOR] If `JDCard.svelte` uses `StatusBadge.svelte` for status display, no changes are needed here -- the StatusBadge update (T61.9) handles it. If it has inline status styling, update to include `discovered` and `applying`.

**Acceptance criteria:**
- JD cards in list view display the correct status label/color for all 9 statuses.
- No references to `interested` remain.

---

## Testing Support

### Migration Tests

| Test | Assertion |
|------|-----------|
| Apply migration 020 on existing DB | Table rebuilt without data loss |
| `interested` -> `discovered` migration | All existing `interested` JDs become `discovered` |
| Other statuses preserved | `analyzing`, `applied`, etc. unchanged |
| New `discovered` status insertable | `INSERT INTO job_descriptions (..., status, ...) VALUES (..., 'discovered', ...)` succeeds |
| New `applying` status insertable | `INSERT ... VALUES (..., 'applying', ...)` succeeds |
| `interested` rejected | `INSERT ... VALUES (..., 'interested', ...)` fails CHECK |
| Default status is `discovered` | `INSERT INTO job_descriptions (id, title, raw_text) VALUES (...)` sets status to `discovered` |
| Junction tables intact | `SELECT COUNT(*) FROM job_description_skills` unchanged after migration |
| Junction tables intact (resumes) | `SELECT COUNT(*) FROM job_description_resumes` unchanged after migration |
| Both indexes exist | `SELECT name FROM sqlite_master WHERE type='index' AND name LIKE 'idx_job_descriptions%'` returns 2 rows |

### API Tests

| Test | Assertion |
|------|-----------|
| POST without status | Creates JD with `discovered` |
| PATCH to `discovered` | Succeeds (200) |
| PATCH to `applying` | Succeeds (200) |
| PATCH to `interested` | Returns 400 (rejected) |
| GET with `status=discovered` filter | Returns only `discovered` JDs |
| All 9 statuses accepted | PATCH with each status succeeds |

### Component Smoke Tests (Manual / Future Playwright)

| Test | What to verify |
|------|---------------|
| Board renders 7 columns | Columns: Discovered, Analyzing, Applying, Applied, Interviewing, Offered, Closed |
| Cards in correct columns | JDs appear in column matching their status |
| Closed column collapsed by default | Closed column shows count badge, content hidden |
| Expand closed column | Clicking reveals closed/rejected/withdrawn cards |
| Closed sub-status badges | Cards in Closed show rejected/withdrawn/closed badge |
| Offered card styling | Green background, green left border |
| Drag card between columns | Card moves, API PATCH called |
| Drop into Closed | Status set to `closed` (not `rejected` or `withdrawn`) |
| JDKanbanCard content | Title, org, location/salary, skill pills rendered |
| Filter bar renders | Organization dropdown, Location input, Search input visible |
| Filters reduce cards | Client-side AND logic across all columns |
| View toggle works | Switch between List and Board views |
| View mode persists | Reload page, same view mode |
| Card click -> list view | Clicking card switches to list with JD selected |
| Empty board message | Shows "No job descriptions yet..." |
| StatusBadge colors | `discovered` = purple, `applying` = amber |

### Integration Tests

| Test | Assertion |
|------|-----------|
| Board drag -> API | Drag JD from Discovered to Analyzing, verify `PATCH { status: 'analyzing' }` called |
| Optimistic revert | Mock API failure on PATCH, verify card reverts to original column |
| Create JD in list -> board | Create JD (list view), switch to Board, verify in Discovered column |
| Filter by org | Select org, verify only matching JDs visible across all columns |
| localStorage persistence | Set board mode, reload, verify board mode restored |

---

## Documentation Requirements

- No new documentation files required.
- The spec file serves as the design document.
- This plan file serves as the implementation reference.
- Inline TSDoc comments on:
  - `JD_PIPELINE_COLUMNS`: explain the `dropStatus` field on the Closed column.
  - `JD_PIPELINE_STATUSES`: explain the distinction from the unified 5-status model.
- Inline comments in the page component for:
  - View mode localStorage persistence rationale.
  - Optimistic update + revert pattern.
  - Card click -> list view switch behavior.
- Inline comments in migration 020 for:
  - `PRAGMA foreign_keys = OFF` rationale.
  - `CASE WHEN` data migration.
  - Junction table safety.

---

## Parallelization Notes

**Within this phase:**
- T61.1 (migration) has no code dependencies -- write first.
- T61.2 + T61.3 (types) can be written in parallel, depend on nothing.
- T61.4 (constants) depends on T61.2 (imports `ColumnDef` type or defines inline).
- T61.5 (route validation) depends on T61.2 (core types must be updated).
- T61.6 (kanban card) and T61.7 (filter bar) can be written in parallel -- no shared state.
- T61.8 (page integration) depends on T61.4, T61.6, T61.7 (imports constants, card, filter bar).
- T61.9 (StatusBadge), T61.10 (editor dropdown), T61.11 (card display) can be written in parallel.

**Recommended execution order:**
1. T61.1 (migration -- foundational)
2. T61.2 + T61.3 (types -- parallel)
3. T61.4 + T61.5 (constants + validation -- parallel)
4. T61.6 + T61.7 + T61.9 + T61.10 + T61.11 (UI components -- all parallel)
5. T61.8 (page integration -- depends on all above)

**Cross-phase:**
- Phase 60 (JD Resume Linkage) adds migration 019. Migration 020 depends on migration 019 if `job_description_resumes` must exist before the table rebuild. However, the rebuild preserves IDs and `PRAGMA foreign_keys = OFF` protects junction tables. If 020 runs before 019, the `job_description_resumes` table simply does not exist yet -- no conflict.
- Phase 60 and Phase 62 both modify `packages/core/src/routes/job-descriptions.ts`. Serialize changes to this file.
- Phase 60 and Phase 62 both modify `packages/sdk/src/types.ts` and `packages/core/src/types/index.ts`. This phase also modifies those files. Serialize.
- This phase modifies `packages/sdk/src/types.ts` (JobDescriptionStatus change). Phase 60 adds `ResumeLink`/`JDLink` to the same file. No field conflicts, but merge carefully.
- Phase 43 (Generic Kanban) must be complete before T61.8 can be implemented. If Phase 43 is still in progress, all other tasks can proceed independently.
