# JD Kanban Pipeline

**Date:** 2026-04-03
**Spec:** E3 (JD Kanban Pipeline)
**Phase:** TBD (next available)
**Builds on:** Spec E1 (JD Detail Page — JD entity with CRUD UI), Spec D (GenericKanban component)
**Dependencies:** Both E1 and D must be complete
**Blocks:** None

## Overview

Add a kanban board view for the JD application pipeline at `/opportunities/job-descriptions`. The board visualizes the progression of job opportunities from discovery through application, interviewing, and outcome. Users drag JD cards between pipeline columns to update status, providing a visual overview of their job search funnel.

The JD pipeline has 7 columns mapping to 10 status values — this is fundamentally different from the unified 5-status kanban (Spec D) because the JD pipeline tracks an external process (hiring lifecycle) rather than an internal content workflow. The board uses `GenericKanban` from Spec D for the drag-and-drop infrastructure, column rendering, and drop handling, but with JD-specific column definitions, card renderer, and filter bar.

This spec also expands the `JobDescriptionStatus` CHECK constraint to include two new statuses (`discovered` and `applying`) and renames the existing `interested` status to `discovered` to better reflect the pipeline semantics.

## Non-Goals

- Sub-step checklists within pipeline stages (spec'd below as a data model for future release, but NOT implemented)
- Automated status transitions (e.g., auto-move to "Applied" when a resume is linked)
- Application deadline tracking or reminders
- Interview scheduling integration
- Offer comparison tooling
- Pipeline analytics or conversion rate metrics
- Drag-and-drop card reordering within a column (no position field)
- Column customization or user-defined columns
- Batch operations (multi-select, bulk status change)
- Mobile/responsive kanban layout

---

## 1. Pipeline Columns

### 1.1 Column Definitions

| Column | Statuses | Drop Status | Accent Color | Description |
|--------|----------|-------------|-------------|-------------|
| Discovered | `discovered` | `discovered` | Gray (#a5b4fc) | Found the posting, saved for review |
| Analyzing | `analyzing` | `analyzing` | Blue (#60a5fa) | Actively reviewing JD, comparing skills |
| Applying | `applying` | `applying` | Amber (#fbbf24) | Preparing application materials |
| Applied | `applied` | `applied` | Indigo (#818cf8) | Application submitted |
| Interviewing | `interviewing` | `interviewing` | Purple (#a78bfa) | In active interview process |
| Offered | `offered` | `offered` | Green (#22c55e) | Received an offer |
| Closed | `rejected`, `withdrawn`, `closed` | `closed` | Dark gray (#d1d5db) | Terminal states (collapsed by default) |

### 1.2 Column Configuration

```typescript
const JD_PIPELINE_COLUMNS: ColumnDef[] = [
  { key: 'discovered', label: 'Discovered', statuses: ['discovered'], accent: '#a5b4fc' },
  { key: 'analyzing', label: 'Analyzing', statuses: ['analyzing'], accent: '#60a5fa' },
  { key: 'applying', label: 'Applying', statuses: ['applying'], accent: '#fbbf24' },
  { key: 'applied', label: 'Applied', statuses: ['applied'], accent: '#818cf8' },
  { key: 'interviewing', label: 'Interviewing', statuses: ['interviewing'], accent: '#a78bfa' },
  { key: 'offered', label: 'Offered', statuses: ['offered'], accent: '#22c55e' },
  { key: 'closed', label: 'Closed', statuses: ['rejected', 'withdrawn', 'closed'], dropStatus: 'closed', accent: '#d1d5db' },
]
```

### 1.3 Closed Column Behavior

The "Closed" column groups three terminal statuses. When a card is dragged INTO the Closed column, it receives the `closed` status (the default `dropStatus`). To set `rejected` or `withdrawn` specifically, the user must use the status dropdown in the JD editor or detail modal — drag-and-drop only sets the default drop status.

Cards in the Closed column show a sub-status badge indicating which terminal status they have:
- `rejected` — red badge, muted text
- `withdrawn` — orange badge, muted text
- `closed` — dark gray badge, muted text

The Closed column is collapsed by default (same pattern as the "Archived" column in GenericKanban and "Excluded" in the org kanban).

---

## 2. Schema Migration

### 2.1 Migration: `020_jd_pipeline_statuses.sql`

Expand the `job_descriptions.status` CHECK constraint to add `discovered` and `applying`, and rename `interested` to `discovered`. Since SQLite cannot ALTER CHECK constraints, this uses the table rebuild pattern.

```sql
-- JD Pipeline Statuses
-- Migration: 020_jd_pipeline_statuses
-- Expands job_descriptions.status CHECK for pipeline kanban.
-- Adds: discovered, applying
-- Renames: interested -> discovered
-- Uses table rebuild pattern (SQLite cannot ALTER CHECK constraints).

PRAGMA foreign_keys = OFF;
-- NOTE: PRAGMA foreign_keys = OFF is defensive only — the runner's transaction provides the actual FK protection.

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

PRAGMA foreign_keys = ON;

INSERT INTO _migrations (name) VALUES ('020_jd_pipeline_statuses');
```

### 2.2 Data Migration Notes

- `interested` is renamed to `discovered`. All existing JDs with `interested` status become `discovered`.
- All other statuses are preserved as-is (`analyzing`, `applied`, `interviewing`, `offered`, `rejected`, `withdrawn`, `closed`).
- The new `applying` status has no existing data.
- The default status for new JDs changes from `interested` to `discovered`.

### 2.3 Junction Table Safety

The table rebuild preserves IDs. With `PRAGMA foreign_keys = OFF`, the DROP + RENAME cycle does not trigger cascade deletes on `job_description_skills` or `job_description_resumes`. Both junction tables continue to reference valid IDs after the rebuild.

---

## 3. Type Changes

### 3.1 Core Types (`packages/core/src/types/index.ts`)

Update `JobDescriptionStatus`:

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

### 3.2 SDK Types (`packages/sdk/src/types.ts`)

Mirror the same change.

### 3.3 Constants

Add to `packages/core/src/constants/index.ts` (or wherever JD-related constants live):

```typescript
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
  { key: 'closed', label: 'Closed', statuses: ['rejected', 'withdrawn', 'closed'], dropStatus: 'closed', accent: '#d1d5db' },
]
```

---

## 4. GenericKanban Integration

### 4.1 Usage

The JD pipeline page uses `GenericKanban` from Spec D with JD-specific configuration:

```svelte
<GenericKanban
  columns={JD_PIPELINE_COLUMNS}
  items={filteredJDs}
  onDrop={handleJDDrop}
  loading={isLoading}
  emptyMessage="No job descriptions yet. Create one to start tracking your pipeline."
  defaultCollapsed="closed" <!-- The `defaultCollapsed` prop accepts a column key string (e.g., 'closed'). Verify this prop exists in Spec D's GenericKanban interface. -->
  sortItems={(a, b) => (b.updated_at ?? '').localeCompare(a.updated_at ?? '')}
>
  {#snippet filterBar()}
    <JDFilterBar bind:filters onchange={applyFilters} />
  {/snippet}

  {#snippet cardContent(jd)}
    <JDKanbanCard {jd} onclick={() => selectJD(jd.id)} />
  {/snippet}
</GenericKanban>
```

### 4.2 Drop Handler

```typescript
async function handleJDDrop(itemId: string, newStatus: string) {
  await forge.jobDescriptions.update(itemId, { status: newStatus as JobDescriptionStatus })
}
```

The `onDrop` callback receives the column's `dropStatus` (or the first entry in `statuses[]` if `dropStatus` is not set). For the Closed column, this is `closed`. For all other columns, the status matches the column key.

### 4.3 Data Loading

```typescript
let allJDs = $state<JobDescriptionWithOrg[]>([])
let loading = $state(true)

onMount(async () => {
  const result = await forge.jobDescriptions.list({ limit: 500 })
  if (result.ok) allJDs = result.data
  loading = false
})
```

### 4.4 View Toggle

The `/opportunities/job-descriptions` page offers a List/Board view toggle (using `ViewToggle.svelte` from Spec D):

- **List view:** The existing split-panel layout from Spec E1 (JD list + editor)
- **Board view:** The GenericKanban pipeline board

View mode is persisted in localStorage with key `forge:viewMode:jobDescriptions`.

Default view: **List** (consistent with Spec D defaults).

---

## 5. JD Kanban Card

### 5.1 Component: `JDKanbanCard.svelte`

**File:** `packages/webui/src/lib/components/kanban/JDKanbanCard.svelte`

**Props:**
```typescript
let { jd, onclick }: {
  jd: JobDescriptionWithOrg
  onclick: () => void
} = $props()
```

### 5.2 Card Content

```
┌────────────────────────────────┐
│ Sr Security Engineer           │
│ Cloudflare                     │
│ Remote • $180k-$220k           │
│ [Python] [AWS] [K8s]           │
└────────────────────────────────┘
```

**Fields displayed:**
- **Title** (bold, primary text, truncated to ~50 chars with ellipsis)
- **Organization name** (muted text, or omitted if null)
- **Location + Salary range** (muted text, combined on one line with bullet separator, omitted if both null)
- **Skill pills** (if JD has skills tagged via E1, show up to 3 skill names as small pills, "+N more" if > 3)

### 5.3 Card Styling by Column

- **Closed column cards:** `opacity: 0.6`, muted text
- **Closed + rejected:** `opacity: 0.6`, red left border
- **Closed + withdrawn:** `opacity: 0.6`, orange left border
- **Offered:** Green background `#f0fdf4`, green left border `#22c55e`
- **All others:** White background, standard border

### 5.4 Card Click

Clicking a card navigates to the JD detail view. If the page is in board mode, clicking a card switches to list mode with that JD selected. Alternatively, clicking opens a `JDDetailModal` (similar to `OrgDetailModal` from the org kanban). The implementation choice depends on the existing JD detail pattern from Spec E1 — if the JD detail is a split-panel editor, switch to list view with selection. If a modal exists, open it.

**Recommended:** Switch to list view with the clicked JD auto-selected, since Spec E1 builds a full split-panel editor (not a modal). Set `viewMode = 'list'` and `selectedId = jd.id`.

---

## 6. Filter Bar

### 6.1 Component: `JDFilterBar.svelte`

**File:** `packages/webui/src/lib/components/filters/JDFilterBar.svelte`

### 6.2 Filters

| Filter | Type | Source |
|--------|------|--------|
| Organization | Dropdown (org name) | `forge.organizations.list({ limit: 500 })` |
| Salary Range | Dropdown (predefined ranges) | Static: "Any", "<$100k", "$100k-$150k", "$150k-$200k", "$200k+" |
| Location | Text input (substring search) | Client-side filter on `location` field |
| Search | Text input (title search) | Client-side filter on `title` and `organization_name` |

### 6.3 Salary Range Filtering

Since `salary_range` is a free-text field (not structured data), the salary filter uses simple substring matching:
- "<$100k": matches JDs where `salary_range` contains a number < 100000 (best-effort regex parsing)
- "$100k-$150k": matches a range
- "$200k+": matches numbers >= 200000

**Note:** This is approximate and best-effort. The salary field is free text and may contain values like "DOE", "GS-13", or "competitive" that do not parse to a number. Non-parseable values are shown in all salary filter modes. Exact salary filtering requires structured salary data (deferred — see non-goals in the JD entity spec).

**Alternative (simpler):** Skip the salary range filter entirely and only offer Organization, Location, and Search. Add salary filtering when structured salary data is available. **Recommended: use the simpler approach.**

### 6.4 Final Filter Set (Simplified)

| Filter | Type | Source |
|--------|------|--------|
| Organization | Dropdown (org name) | `forge.organizations.list({ limit: 500 })` |
| Location | Text input (substring search) | Client-side filter on `location` field |
| Search | Text input (title + org search) | Client-side filter on `title` and `organization_name` |

### 6.5 Filtering Strategy

Filters are applied client-side after fetching all JDs. Consistent with the org kanban and GenericKanban patterns. Filters are combined with AND logic. Clearing a filter removes it from the active set.

---

## 7. Sub-Step Checklists (Deferred — Data Model Only)

This section documents the data model for within-stage progress tracking. This is NOT implemented in this spec — only the schema design is documented here for a future release.

### 7.1 Purpose

Some pipeline stages have multiple sub-steps. For example, "Applying" involves preparing a resume, writing a cover letter, and completing the application form. "Interviewing" involves multiple rounds (phone screen, hiring manager, technical, coding, system design, culture fit). Tracking progress within these stages provides finer-grained visibility.

### 7.2 Proposed Data Model

**Option A: Checklist Items Table**

```sql
-- DEFERRED — DO NOT IMPLEMENT
CREATE TABLE jd_checklist_items (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  job_description_id TEXT NOT NULL REFERENCES job_descriptions(id) ON DELETE CASCADE,
  stage TEXT NOT NULL CHECK (stage IN (
    'discovered', 'analyzing', 'applying', 'applied',
    'interviewing', 'offered', 'rejected', 'withdrawn', 'closed'
  )),
  label TEXT NOT NULL,
  completed INTEGER NOT NULL DEFAULT 0,
  position INTEGER NOT NULL DEFAULT 0,
  completed_at TEXT,
  notes TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

CREATE INDEX idx_jd_checklist_jd ON jd_checklist_items(job_description_id);
CREATE INDEX idx_jd_checklist_stage ON jd_checklist_items(job_description_id, stage);
```

**Option B: Template-Based Checklists**

Pre-defined checklist templates per stage, instantiated when a JD enters that stage:

```typescript
// DEFERRED — DO NOT IMPLEMENT
const STAGE_TEMPLATES = {
  applying: [
    { label: 'Tailor resume', position: 0 },
    { label: 'Write cover letter', position: 1 },
    { label: 'Complete application form', position: 2 },
    { label: 'Submit application', position: 3 },
  ],
  interviewing: [
    { label: 'Phone screen', position: 0 },
    { label: 'Hiring manager interview', position: 1 },
    { label: 'Technical interview', position: 2 },
    { label: 'Coding assessment', position: 3 },
    { label: 'System design interview', position: 4 },
    { label: 'Culture fit interview', position: 5 },
  ],
}
```

### 7.3 Recommended Approach (for Future)

Option A (free-form checklist items) is more flexible — users can add/remove/reorder items per JD. Option B (templates) provides a faster starting point but is rigid. The recommendation is Option A with optional templates: when a JD enters a stage for the first time, offer to populate the checklist from a template, but allow free editing afterward.

### 7.4 UI Sketch (for Future)

```
Applying (2/4 complete)
  [x] Tailor resume
  [x] Write cover letter
  [ ] Complete application form
  [ ] Submit application
```

The checklist would appear below the status dropdown in the JD editor, showing only items for the current stage. Completed items show a checkmark. The kanban card could show progress (e.g., "2/4") as a small badge.

---

## 8. Files to Create

| File | Purpose |
|------|---------|
| `packages/core/src/db/migrations/020_jd_pipeline_statuses.sql` | Expand JD status CHECK, add `discovered`/`applying`, rename `interested` -> `discovered` |
| `packages/webui/src/lib/components/kanban/JDKanbanCard.svelte` | JD card content for kanban board |
| `packages/webui/src/lib/components/filters/JDFilterBar.svelte` | JD filter controls for kanban/list |

## 9. Files to Modify

| File | Change |
|------|--------|
| `packages/core/src/types/index.ts` | Update `JobDescriptionStatus` union: add `discovered`, `applying`; remove `interested` |
| `packages/sdk/src/types.ts` | Mirror `JobDescriptionStatus` change |
| `packages/core/src/constants/index.ts` | Add `JD_PIPELINE_STATUSES` and `JD_PIPELINE_COLUMNS` constants |
| `packages/core/src/routes/job-descriptions.ts` | Update status validation in POST/PATCH handlers: accept `discovered`/`applying`, reject `interested` |
| `packages/core/src/db/repositories/job-description-repository.ts` | Update any hardcoded status references |
| `packages/core/src/services/job-description-service.ts` (if exists) | Update `VALID_STATUSES` to include `discovered` and `applying`, and remove `interested` |
| `packages/webui/src/routes/opportunities/job-descriptions/+page.svelte` | Add view toggle (List/Board), integrate GenericKanban in board mode |
| `packages/webui/src/lib/components/jd/JDEditor.svelte` | Update status dropdown options: replace `interested` with `discovered`, add `applying` |
| `packages/webui/src/lib/components/jd/JDCard.svelte` | Update status display for new values |
| `packages/webui/src/lib/components/StatusBadge.svelte` | Add color mappings for `discovered` and `applying` statuses |

---

## 10. Status Badge Colors (Updated)

| Status | Color | Visual |
|--------|-------|--------|
| `discovered` | Light purple | Muted, new |
| `analyzing` | Blue | Active research |
| `applying` | Amber | In preparation |
| `applied` | Indigo | Submitted |
| `interviewing` | Purple | In-progress |
| `offered` | Green | Positive outcome |
| `rejected` | Red | Negative outcome |
| `withdrawn` | Orange | User withdrew |
| `closed` | Dark gray | Terminal |

---

## 11. Testing

### 11.1 Migration Tests

- Apply migration 020 on a database with existing JDs
- Verify JDs with `interested` status are migrated to `discovered`
- Verify all other statuses are preserved
- Verify new status values (`discovered`, `applying`) can be inserted
- Verify `interested` is rejected by the CHECK constraint
- Verify junction tables (`job_description_skills`, `job_description_resumes`) remain intact after table rebuild
- Verify default status for new JDs is `discovered`

### 11.2 API Tests

- `POST /api/job-descriptions` without explicit status creates JD with `discovered`
- `PATCH /api/job-descriptions/:id { status: 'discovered' }` succeeds
- `PATCH /api/job-descriptions/:id { status: 'applying' }` succeeds
- `PATCH /api/job-descriptions/:id { status: 'interested' }` returns 400 (old status rejected)
- `GET /api/job-descriptions?status=discovered` returns only discovered JDs
- All 9 valid statuses are accepted by the API

### 11.3 Component Smoke Tests

- Board renders 7 columns with correct labels and accent colors
- Cards appear in correct columns based on status
- Closed column is collapsed by default
- Clicking collapsed Closed column expands it
- Closed column cards show sub-status badge (rejected/withdrawn/closed)
- Offered cards have green background and green left border
- Dropping a card into Closed column sets status to `closed`
- Dropping a card into any other column sets the matching status
- JDKanbanCard shows title, org name, location/salary, and skill pills
- Filter bar renders above the board with Organization, Location, and Search filters
- Filters reduce visible cards (client-side AND logic)
- View toggle switches between List and Board views
- View mode persists in localStorage under `forge:viewMode:jobDescriptions`
- Clicking a kanban card switches to list view with that JD selected
- Empty board shows "No job descriptions yet" message

### 11.4 Integration Tests

- Toggle to Board view, drag a JD from Discovered to Analyzing, verify API `PATCH` called with `{ status: 'analyzing' }`
- Optimistic update: card moves immediately; reverts on API failure
- Create a JD (list view), toggle to Board view, verify new JD appears in Discovered column
- Filter by organization, verify only matching JDs visible across all columns

---

## 12. Acceptance Criteria

1. `/opportunities/job-descriptions` offers a List/Board view toggle, defaulting to List
2. Board view renders `GenericKanban` with 7 pipeline columns: Discovered, Analyzing, Applying, Applied, Interviewing, Offered, Closed
3. Migration 020 expands `JobDescriptionStatus` CHECK: adds `discovered` and `applying`, renames `interested` to `discovered`
4. Existing JDs with `interested` status are migrated to `discovered`
5. Default status for new JDs is `discovered`
6. Dragging a card between columns updates JD status via `PATCH /api/job-descriptions/:id`
7. Dropping into Closed column sets status to `closed`; `rejected` and `withdrawn` are set via the editor dropdown
8. Closed column is collapsed by default; cards show sub-status badges
9. Offered cards have green background styling
10. JD kanban cards show title, organization name, location/salary, and up to 3 skill pills
11. Filter bar with Organization dropdown, Location text input, and Search text input
12. Filters apply client-side with AND logic across all columns
13. View mode persists in localStorage (`forge:viewMode:jobDescriptions`)
14. Clicking a kanban card switches to list view with that JD auto-selected
15. `StatusBadge.svelte` renders correct colors for `discovered` and `applying`
16. Sub-step checklist data model is documented (Section 7) but NOT implemented
17. The `interested` status is no longer accepted by the API after migration
