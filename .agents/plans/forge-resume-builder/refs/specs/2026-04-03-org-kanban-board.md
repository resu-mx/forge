# Organization Kanban Board

**Date:** 2026-04-03
**Spec:** 10 (Org Kanban)
**Phase:** TBD (next available)
**Builds on:** migration 003 (which added organizations.status), migration 011 (org_tags)

## Overview

Replace the split-panel list at `/opportunities/organizations` with a kanban board for vetting organizations. The board models a research pipeline — orgs move from discovery through research to a targeting decision. Cards are dragged between columns to update status. The Targeting column uses color-coded interest levels (exciting/interested/acceptable). No new tables or API routes — the board reads/writes the existing `organizations.status` field.

## 1. Schema Changes

Expand the valid `status` values for organizations. The current CHECK constraint allows `interested | review | targeting | excluded`. The new set:

```
backlog | researching | exciting | interested | acceptable | excluded
```

### Migration

```sql
-- Organization Kanban Statuses
-- Migration: 012_org_kanban_statuses
-- Expands organizations.status CHECK constraint for kanban pipeline.
-- Uses table rebuild pattern (SQLite cannot ALTER CHECK constraints).

PRAGMA foreign_keys = OFF;

-- **Note:** The migration runner wraps each migration in BEGIN/COMMIT.
-- `PRAGMA foreign_keys = OFF` is silently ignored inside an active transaction.
-- The PRAGMA calls are defensive only — the actual FK protection comes from
-- the runner's transaction ensuring all statements execute atomically.
-- This is consistent with how migrations 002 and 007 handle table rebuilds.

CREATE TABLE organizations_new (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  name TEXT NOT NULL,
  org_type TEXT DEFAULT 'company' CHECK (org_type IN (
    'company', 'nonprofit', 'government', 'military',
    'education', 'volunteer', 'freelance', 'other'
  )),
  industry TEXT,
  size TEXT,
  worked INTEGER NOT NULL DEFAULT 0,
  employment_type TEXT CHECK (employment_type IN (
    'civilian', 'contractor', 'military_active',
    'military_reserve', 'volunteer', 'intern', NULL
  )),
  location TEXT,
  headquarters TEXT,
  website TEXT,
  linkedin_url TEXT,
  glassdoor_url TEXT,
  glassdoor_rating REAL,
  reputation_notes TEXT,
  notes TEXT,
  status TEXT CHECK (status IS NULL OR status IN (
    'backlog', 'researching', 'exciting', 'interested', 'acceptable', 'excluded'
  )),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

-- Status remapping done inline via CASE WHEN to avoid CHECK constraint violations on the source table.
INSERT INTO organizations_new (id, name, org_type, industry, size, worked, employment_type,
  location, headquarters, website, linkedin_url, glassdoor_url, glassdoor_rating,
  reputation_notes, notes, status, created_at, updated_at)
SELECT id, name, org_type, industry, size, worked, employment_type,
  location, headquarters, website, linkedin_url, glassdoor_url, glassdoor_rating,
  reputation_notes, notes,
  CASE status
    WHEN 'interested' THEN 'backlog'
    WHEN 'review' THEN 'researching'
    WHEN 'targeting' THEN 'interested'
    ELSE status
  END,
  created_at, updated_at
FROM organizations;

-- Swap tables
DROP TABLE organizations;
ALTER TABLE organizations_new RENAME TO organizations;

-- Create index on name (new — improves picker search performance)
CREATE INDEX idx_organizations_name ON organizations(name);

PRAGMA foreign_keys = ON;

-- Note: org_tags FK references organizations(id) via ON DELETE CASCADE.
-- Since we INSERT all rows into the new table with the same IDs before
-- dropping the old table, and PRAGMA foreign_keys is OFF during the
-- rebuild, no FK violations occur. After PRAGMA foreign_keys = ON,
-- the org_tags rows still reference valid organization IDs.

INSERT INTO _migrations (name) VALUES ('012_org_kanban_statuses');
```

### Service validation update

In `organization-service.ts`, update `VALID_STATUSES`:

```typescript
const VALID_STATUSES = ['backlog', 'researching', 'exciting', 'interested', 'acceptable', 'excluded']
```

## 2. Column-to-Status Mapping

| Column | Statuses | Color Accent | Description |
|--------|----------|-------------|-------------|
| Backlog | `backlog` | Purple (#a5b4fc) | Just discovered, added to pipeline |
| Researching | `researching` | Amber (#fbbf24) | Actively vetting |
| Targeting | `exciting`, `interested`, `acceptable` | Green (#22c55e) | Passed vetting — interest-level cards |
| Excluded | `excluded` | Gray, collapsed | Red flags found, rejected |

### Interest levels within Targeting column

| Interest | Status value | Card style |
|----------|-------------|------------|
| Exciting | `exciting` | Green background (#f0fdf4), green left border (#22c55e), green badge |
| Interested | `interested` | Blue background (#eff6ff), blue left border (#3b82f6), blue badge |
| Acceptable | `acceptable` | Gray background (#fafafa), gray left border (#9ca3af), gray badge |

Interest level is changed via a dropdown in the OrgDetailModal, not by drag position within the column.

When a card is dragged INTO the Targeting column from another column, its status is set to `interested` (the default interest level).

## 3. Components

### 3.1 KanbanBoard.svelte

Top-level component at `/opportunities/organizations`.

**Responsibilities:**
- Fetches all orgs with a status via `forge.organizations.list({ limit: 500 })`
- Groups orgs into columns by status
- Renders 4 `KanbanColumn` instances
- Manages the `OrgPickerModal` (add button) and `OrgDetailModal` (card click)
- Handles cross-column drag-and-drop status updates

**State:**
```typescript
let organizations = $state<Organization[]>([])
let loading = $state(true)
let excludedExpanded = $state(false)
let showPicker = $state(false)
let detailOrgId = $state<string | null>(null)
```

**Derived columns:**
```typescript
const COLUMNS = [
  { key: 'backlog', label: 'Backlog', statuses: ['backlog'], accent: '#a5b4fc' },
  { key: 'researching', label: 'Researching', statuses: ['researching'], accent: '#fbbf24' },
  { key: 'targeting', label: 'Targeting', statuses: ['exciting', 'interested', 'acceptable'], accent: '#22c55e' },
  { key: 'excluded', label: 'Excluded', statuses: ['excluded'], accent: '#d1d5db' },
]

let columnData = $derived(COLUMNS.map(col => ({
  ...col,
  items: organizations
    .filter(o => col.statuses.includes(o.status!))
    .sort((a, b) => a.name.localeCompare(b.name)),
})))
```

**Data loading:**
- On mount, fetch all orgs
- Filter to only those with a non-null status (orgs without status are not on the board)

Show `<LoadingSpinner>` while `loading` is true, matching the existing pattern.

When no orgs have a status set (empty board), show an empty state message: 'No organizations in the pipeline yet. Click + Add Organization to start tracking.' centered across all columns.

### 3.2 KanbanColumn.svelte

**Props:**
```typescript
let { label, accent, items, collapsed, onToggleCollapse, onDrop, onCardClick }: Props = $props()
```

**Behavior:**
- Uses `svelte-dnd-action` `dndzone` for drop targets
- Renders column header with label, count badge, and accent color top border
- When `collapsed` is true (Excluded only), renders as a thin vertical strip with rotated text and count
- Click the collapsed strip to expand

**Drop handling:**
- `onconsider` / `onfinalize` from `svelte-dnd-action`
- On finalize, calls `onDrop(orgId, columnKey)` which the parent uses to PATCH the status
- Use Svelte 5 event handler syntax (`onconsider`, `onfinalize`), not Svelte 4 directive syntax (`on:consider`). Verify that the installed version of `svelte-dnd-action` supports Svelte 5 runes mode.

### 3.3 KanbanCard.svelte

**Props:**
```typescript
let { org, onClick }: Props = $props()
```

**Renders:**
- Org name (bold)
- Tag pills (from `org.tags`)
- Industry + location (muted text)
- If in Targeting column: interest badge (EXCITING / INTERESTED / ACCEPTABLE) with color
- If excluded: `opacity: 0.6`, `text-decoration: line-through` on name

**Styling by status:**
- `exciting`: background `#f0fdf4`, left border `4px solid #22c55e`
- `interested`: background `#eff6ff`, left border `4px solid #3b82f6`
- `acceptable`: background `#fafafa`, left border `4px solid #9ca3af`
- All others: white background, standard border

If `org.worked` is true, show a small checkmark or 'Worked' badge on the card to indicate the user has past experience at this organization.

### 3.4 OrgPickerModal

**Purpose:** Add an org to the pipeline.

**Deduplication enforcement:**
1. Shows only orgs where `status IS NULL` (already on board = hidden from picker)
2. Re-fetches org list on every modal open (no stale data)
3. "Create New" checks for name collision before inserting — if name exists, shows: "An organization named X already exists. Add the existing one instead?" with a button to select it. Name collision check uses case-insensitive comparison (`name.toLowerCase()` matching or SQL `COLLATE NOCASE`). For example, creating 'google' when 'Google' exists should trigger the warning.

**Layout:**
- Search bar (filters by name)
- Tag filter dropdown
- Scrollable list of org cards
- Click an org → `PATCH` status to `backlog`, close modal, card appears in Backlog
- After adding an org (setting status to `backlog`), call `loadOrganizations()` on the parent KanbanBoard to refresh the board state. The picker closes and the new card appears in the Backlog column.
- "Create New" section at bottom: inline form (name + org_type + tags + website)

Note: `OrganizationFilter` cannot express `status IS NULL` via the API. The OrgPickerModal fetches all orgs (`limit: 500`) and filters client-side with `.filter(o => !o.status)` to show only orgs not yet in the pipeline.

### 3.5 OrgDetailModal

**Purpose:** View/edit org details from the board. Opens on card click.

**Layout:**
- Header: org name + current column indicator
- Interest level dropdown (only shown when status is `exciting | interested | acceptable`):
  - Options: Exciting, Interested, Acceptable
  - Changing calls `PATCH` with new status
- Read-only fields: tags (pills), industry, location, website (link), LinkedIn (link), Glassdoor (link + rating)
- Editable: notes textarea (saves on blur via `PATCH`)
- Editable: reputation notes textarea (saves on blur)
- Footer actions:
  - "Edit Full Details" → `/data/organizations` already exists (created earlier in this session). The link navigates there. Optionally pass `?id={orgId}` as a query param to auto-select the org.
  - "Remove from Pipeline" → `PATCH { status: null }`, org disappears from board, modal closes

## 4. Drag-and-Drop Behavior

Uses `svelte-dnd-action` (already a project dependency).

**Status mapping on drop:**

| Drop target column | Status set to |
|--------------------|---------------|
| Backlog | `backlog` |
| Researching | `researching` |
| Targeting | `interested` (default interest level) |
| Excluded | `excluded` |

**Optimistic update:** Update local state immediately on drop, fire `PATCH` in background. If PATCH fails, revert local state and show error toast.

**Card identity:** `svelte-dnd-action` requires an `id` field on each item. Use `org.id` directly.

**Drag handles:** The entire card is draggable (no separate handle). `cursor: grab` on hover, `cursor: grabbing` while dragging.

**API:** No new API routes needed. The kanban uses the existing `PATCH /api/organizations/:id` route (defined in `packages/core/src/routes/organizations.ts`) which already supports partial updates including `{ status: 'backlog' }`. The existing `GET /api/organizations` with `?limit=500` fetches the board data.

## 5. Files to Create

| File | Purpose |
|------|---------|
| `packages/core/src/db/migrations/012_org_kanban_statuses.sql` | Expand status CHECK |
| `packages/webui/src/lib/components/kanban/KanbanBoard.svelte` | Top-level board |
| `packages/webui/src/lib/components/kanban/KanbanColumn.svelte` | Single column |
| `packages/webui/src/lib/components/kanban/KanbanCard.svelte` | Single card |
| `packages/webui/src/lib/components/kanban/OrgPickerModal.svelte` | Add-to-pipeline modal |
| `packages/webui/src/lib/components/kanban/OrgDetailModal.svelte` | Card detail modal |

## 6. Files to Modify

| File | Change |
|------|--------|
| `packages/core/src/services/organization-service.ts` | Update `VALID_STATUSES` array |
| `packages/sdk/src/types.ts` | Update `Organization.status` union type |
| `packages/webui/src/routes/opportunities/organizations/+page.svelte` | Replace split-panel with `<KanbanBoard />` |

Also update `CreateOrganization.status` and `UpdateOrganization.status` in `packages/sdk/src/types.ts` to use the new union type `'backlog' | 'researching' | 'exciting' | 'interested' | 'acceptable' | 'excluded' | null`. Leave `OrganizationFilter.status` as `string` for flexibility.

## 7. Testing

### Unit tests
- Service: validate new statuses (`backlog`, `researching`, `exciting`, `interested`, `acceptable`, `excluded`) are accepted
- Service: old statuses (`review`) are rejected after migration

### Integration tests
- API: `PATCH /api/organizations/:id` with `{ status: 'backlog' }` succeeds
- API: `PATCH /api/organizations/:id` with `{ status: 'review' }` returns 400 (old status)
- API: `PATCH /api/organizations/:id` with `{ status: null }` succeeds (remove from pipeline)
- API: `GET /api/organizations?status=exciting` returns only exciting orgs

### Component smoke tests
- Board renders 4 columns with correct labels
- Cards appear in correct columns based on status
- Excluded column is collapsed by default
- Clicking collapsed Excluded expands it
- Targeting cards show interest-level badges with correct colors
- Excluded cards show muted/strikethrough styling
- OrgPickerModal shows only orgs with `status = null`
- OrgPickerModal "Create New" warns on name collision
- OrgDetailModal interest dropdown only visible for targeting statuses
- Org with `worked=true` and `status='backlog'` appears on the kanban board with a 'Worked' badge
- 'Remove from Pipeline' on a `worked` org sets `status=null` but does NOT delete the org

## 8. Non-Goals

- Research checklists / sub-steps within Researching column
- Job description linking from kanban cards
- Sorting within columns (alpha by name for now)
- Column reordering or custom columns
- Batch operations (multi-select, bulk status change)
- Card reordering within a column (no position field)
- Delete org from kanban (use master list at `/data/organizations`)
- Keyboard-based drag-and-drop (Tab/Space/Arrow navigation for card reordering) — `svelte-dnd-action` provides some keyboard support but it is not a requirement for this spec

## 9. Acceptance Criteria

1. `/opportunities/organizations` renders a kanban board with 4 columns: Backlog, Researching, Targeting, Excluded
2. Dragging a card between columns updates `organizations.status` via API
3. Dropping in Targeting sets status to `interested` (default interest level)
4. Targeting cards are color-coded: green (exciting), blue (interested), gray (acceptable)
5. Excluded column is collapsed by default; click to expand; cards are muted with strikethrough
6. "+" button opens OrgPickerModal showing only orgs not already on the board (`status IS NULL`)
7. Picker deduplicates: name collision on "Create New" shows warning with option to add existing
8. Clicking a card opens OrgDetailModal with interest level dropdown, notes, links
9. "Remove from Pipeline" sets status to null, card disappears
10. Migration 012 expands status CHECK and migrates `interested→backlog`, `review→researching`, `targeting→interested`
11. Old status values (`review`, `targeting` as column-level) are rejected by the service after migration
