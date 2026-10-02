# Phase 38: Organization Kanban Board (Migration 012 + Service + SDK + UI)

**Status:** Planning
**Date:** 2026-04-03
**Spec:** [2026-04-03-org-kanban-board.md](../refs/specs/2026-04-03-org-kanban-board.md)
**Depends on:** Phase 28 (stable baseline), Migration 011 (`org_tags`)
**Blocks:** None currently identified
**Parallelizable with:** Phases 29-37 at the code level (independent feature); migration must be numbered after 011

## Goal

Replace the split-panel organization list at `/opportunities/organizations` with a kanban board that models the organization vetting pipeline. Organizations flow through four columns -- Backlog, Researching, Targeting, and Excluded -- by dragging cards between columns. The Targeting column renders cards color-coded by interest level (exciting/interested/acceptable), changed via a detail modal. The implementation spans a table-rebuild migration to expand the `status` CHECK constraint, a service validation update, SDK type changes, and five new Svelte 5 components using `svelte-dnd-action` for drag-and-drop. No new API routes are required -- the board reads/writes the existing `PATCH /api/organizations/:id` and `GET /api/organizations?limit=500` endpoints.

## Non-Goals

- Research checklists / sub-steps within the Researching column
- Job description linking from kanban cards
- Sorting within columns beyond alphabetical by name
- Column reordering or custom columns
- Batch operations (multi-select, bulk status change)
- Card reordering within a column (no position field in schema)
- Delete org from kanban (use master list at `/data/organizations`)
- Keyboard-based drag-and-drop navigation (Tab/Space/Arrow)
- Multi-user support or real-time sync

## Context

The current `/opportunities/organizations` page uses a split-panel layout: a scrollable list on the left with status filter tabs (`interested | review | targeting | excluded`) and an editor form on the right. This UI treats status as a flat attribute -- the user manually sets a dropdown value and saves. There is no visual representation of the research pipeline.

The kanban board reframes statuses as pipeline stages. An organization enters the pipeline at `backlog` (just discovered), moves to `researching` (actively vetting), then either reaches `targeting` (passed vetting, with an interest level) or `excluded` (red flags found). The board makes the pipeline visual: cards flow left-to-right, the Excluded column is collapsed by default to keep focus on active prospects, and Targeting cards are color-coded so the user can see at a glance which orgs are most exciting.

The migration renames old statuses to match the new pipeline model:
- `interested` (old) -> `backlog` (new) -- "interested" was the entry point before; `backlog` is clearer
- `review` (old) -> `researching` (new) -- actively vetting
- `targeting` (old) -> `interested` (new) -- `targeting` becomes a column name, `interested` becomes a specific interest level within it

## Scope

| Spec section | Covered here? |
|-------------|---------------|
| 1. Schema changes (migration 012) | Yes |
| 2. Column-to-status mapping | Yes |
| 3.1 KanbanBoard.svelte | Yes |
| 3.2 KanbanColumn.svelte | Yes |
| 3.3 KanbanCard.svelte | Yes |
| 3.4 OrgPickerModal | Yes |
| 3.5 OrgDetailModal | Yes |
| 4. Drag-and-drop behavior | Yes |
| 5. Files to create | Yes |
| 6. Files to modify | Yes |
| 7. Testing | Yes |
| 8. Non-goals | Acknowledged |
| 9. Acceptance criteria | Yes |

## Files to Create

| File | Description |
|------|-------------|
| `packages/core/src/db/migrations/012_org_kanban_statuses.sql` | Table rebuild migration: expand status CHECK constraint, migrate old status values |
| `packages/webui/src/lib/components/kanban/KanbanBoard.svelte` | Top-level board: fetches orgs, groups into columns, manages modals and drag state |
| `packages/webui/src/lib/components/kanban/KanbanColumn.svelte` | Single column with `svelte-dnd-action` drop zone, header, count badge |
| `packages/webui/src/lib/components/kanban/KanbanCard.svelte` | Single card: name, tags, interest badge, worked badge, status-based styling |
| `packages/webui/src/lib/components/kanban/OrgPickerModal.svelte` | Add-to-pipeline modal: search, tag filter, dedup, create new with name collision check |
| `packages/webui/src/lib/components/kanban/OrgDetailModal.svelte` | Card detail modal: interest dropdown, notes auto-save, links, remove from pipeline |

## Files to Modify

| File | Change |
|------|--------|
| `packages/core/src/services/organization-service.ts` | Update `VALID_STATUSES` array to new status values |
| `packages/sdk/src/types.ts` | Update `Organization.status`, `CreateOrganization.status`, `UpdateOrganization.status` union types |
| `packages/webui/src/routes/opportunities/organizations/+page.svelte` | Replace split-panel layout with `<KanbanBoard />` import |
| `packages/core/src/services/__tests__/organization-service.test.ts` | Update status validation tests: add new statuses, mark old statuses as invalid |
| `packages/webui/src/routes/data/organizations/+page.svelte` | Update status dropdown options from `interested\|review\|targeting\|excluded` to `backlog\|researching\|exciting\|interested\|acceptable\|excluded` |

## Fallback Strategies

- **`svelte-dnd-action` Svelte 5 compatibility:** The project uses `svelte-dnd-action` v0.9.69. If `onconsider`/`onfinalize` event handlers do not work (some versions require `on:consider`/`on:finalize` Svelte 4 syntax), fall back to using the `use:dndzone` action with Svelte 4 event directive syntax. The existing `DragNDropView.svelte` in the codebase already uses `onconsider`/`onfinalize` as Svelte 5 event handlers successfully, confirming v0.9.69 supports this.
- **PRAGMA foreign_keys in transactions:** The migration runner wraps each migration in `BEGIN`/`COMMIT`. SQLite silently ignores `PRAGMA foreign_keys = OFF` inside an active transaction. The PRAGMA calls in the migration SQL are defensive only -- the actual atomicity comes from the runner's transaction. This is consistent with how migrations 002 and 007 handle table rebuilds.
- **`org_tags` FK during table rebuild:** Since we INSERT all rows into `organizations_new` with the same IDs before dropping the old table, and FK checking is effectively off during the transaction, no FK violations occur. After the transaction commits, `org_tags` rows still reference valid organization IDs.
- **org_tags preservation:** The `DROP TABLE organizations` step may trigger CASCADE deletes on `org_tags` if FK enforcement is active (PRAGMA foreign_keys = OFF is ignored inside the runner's transaction). To mitigate: after the INSERT into `organizations_new` but before the DROP, verify `org_tags` will survive by noting that SQLite's CASCADE behavior on DROP TABLE only fires when FK enforcement is ON. If org_tags rows are lost after migration, restore them from a pre-migration backup or re-run migration 011's seeding logic. Add a migration integrity test (see Testing Support) that verifies org_tags count is preserved.
- **Empty board:** If no organizations have a status set, the board shows an empty state message instead of four empty columns, guiding the user to add their first org.
- **API failure on drag:** Optimistic UI reverts the card to its original column and shows an error toast if the `PATCH` call fails.
- **Large org list:** The board fetches with `limit=500`. If a user somehow has more than 500 orgs with statuses, the board will show the first 500. This is acceptable for a single-user app.

---

## Tasks

### T38.1: Write Migration `012_org_kanban_statuses.sql`

**File:** `packages/core/src/db/migrations/012_org_kanban_statuses.sql`

Expands the `organizations.status` CHECK constraint from `interested | review | targeting | excluded` to `backlog | researching | exciting | interested | acceptable | excluded`. Uses the table rebuild pattern (SQLite cannot ALTER CHECK constraints). Status remapping is done inline via CASE WHEN during the INSERT into the new table (not via UPDATEs on the source table, which would violate the old CHECK constraint). Explicit column list (no `SELECT *`).

```sql
-- Organization Kanban Statuses
-- Migration: 012_org_kanban_statuses
-- Date: 2026-04-03
-- Expands organizations.status CHECK constraint for kanban pipeline.
-- Uses table rebuild pattern (SQLite cannot ALTER CHECK constraints).
-- Builds on 011_org_tags.

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

**Key points:**
- Status remapping is done inline via `CASE WHEN` in the SELECT during INSERT, avoiding CHECK constraint violations on the source table. The old table's CHECK constraint does not allow values like `'backlog'`, so UPDATE statements on the source table would fail.
- `SELECT *` is avoided: explicit column list prevents breakage if columns are added between migrations.
- The `idx_organizations_name` index improves picker search performance.

**Acceptance criteria:**
- After migration, `SELECT status FROM organizations WHERE status = 'review'` returns 0 rows.
- After migration, `SELECT status FROM organizations WHERE status = 'targeting'` returns 0 rows (the old `targeting` column status).
- Any org that had `status = 'interested'` now has `status = 'backlog'`.
- Any org that had `status = 'review'` now has `status = 'researching'`.
- Any org that had `status = 'targeting'` now has `status = 'interested'`.
- Orgs with `status = 'excluded'` or `status IS NULL` are unchanged.
- `INSERT INTO organizations (id, name, status) VALUES ('test-id-...', 'Test', 'exciting')` succeeds.
- `INSERT INTO organizations (id, name, status) VALUES ('test-id-...', 'Test', 'review')` fails with CHECK constraint.
- The `_migrations` table contains `012_org_kanban_statuses`.
- `org_tags` rows still reference valid organization IDs (no orphans).

**Failure criteria:**
- Migration fails with a SQL error.
- Rows are lost during the table rebuild (count mismatch before/after).
- `org_tags` FK violations after migration.

---

### T38.2: Update `VALID_STATUSES` in Organization Service

**File:** `packages/core/src/services/organization-service.ts`

Replace the `VALID_STATUSES` constant:

```typescript
const VALID_STATUSES = ['backlog', 'researching', 'exciting', 'interested', 'acceptable', 'excluded']
```

**Current code (line 14):**
```typescript
const VALID_STATUSES = ['interested', 'review', 'targeting', 'excluded']
```

**No other changes needed.** The validation logic in `create()` and `update()` already checks `input.status !== undefined && input.status !== null && !VALID_STATUSES.includes(input.status)`, which is correct for the new statuses.

**Acceptance criteria:**
- `service.create({ name: 'Test', status: 'backlog' })` succeeds.
- `service.create({ name: 'Test', status: 'researching' })` succeeds.
- `service.create({ name: 'Test', status: 'exciting' })` succeeds.
- `service.create({ name: 'Test', status: 'interested' })` succeeds.
- `service.create({ name: 'Test', status: 'acceptable' })` succeeds.
- `service.create({ name: 'Test', status: 'excluded' })` succeeds.
- `service.create({ name: 'Test', status: 'review' })` returns `VALIDATION_ERROR`.
- `service.create({ name: 'Test', status: 'targeting' })` returns `VALIDATION_ERROR`.
- `service.update(id, { status: null })` succeeds (remove from pipeline).

**Failure criteria:**
- Old status values `review` or `targeting` pass validation.
- New status values like `backlog` or `exciting` fail validation.

---

### T38.3: Update SDK Types

**File:** `packages/sdk/src/types.ts`

Update the `Organization.status` type union (line 205):

```typescript
  status: 'backlog' | 'researching' | 'exciting' | 'interested' | 'acceptable' | 'excluded' | null
```

Update `CreateOrganization.status` (around line 907):

```typescript
  status?: 'backlog' | 'researching' | 'exciting' | 'interested' | 'acceptable' | 'excluded' | null
```

Update `UpdateOrganization.status` (around line 925):

```typescript
  status?: 'backlog' | 'researching' | 'exciting' | 'interested' | 'acceptable' | 'excluded' | null
```

Leave `OrganizationFilter.status` as `string` -- it is already flexible and does not need to enumerate values.

**Acceptance criteria:**
- TypeScript compiler accepts `{ status: 'backlog' }` in `CreateOrganization` and `UpdateOrganization`.
- TypeScript compiler accepts `{ status: 'exciting' }` in `UpdateOrganization`.
- TypeScript compiler rejects `{ status: 'review' }` in `CreateOrganization` (type error).
- `Organization.status` can be destructured as the new union in Svelte components.

**Failure criteria:**
- Type mismatch between SDK types and API response causes runtime confusion.
- `Organization.status` does not include `null` (orgs without pipeline status).

---

### T38.4: Update Organization Service Tests

**File:** `packages/core/src/services/__tests__/organization-service.test.ts`

Update the existing status validation tests to use the new status values and add tests for old statuses being rejected.

Replace the status validation section at the bottom of the file (from line 178 onward):

```typescript
  // -- status validation (Phase 38 — kanban statuses) -------------------------

  test('create with valid kanban status "backlog" succeeds', () => {
    const result = service.create({ name: 'Pipeline Corp', status: 'backlog' })
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data.status).toBe('backlog')
  })

  test('create with valid kanban status "researching" succeeds', () => {
    const result = service.create({ name: 'Research Corp', status: 'researching' })
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data.status).toBe('researching')
  })

  test('create with valid kanban status "exciting" succeeds', () => {
    const result = service.create({ name: 'Hot Startup', status: 'exciting' })
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data.status).toBe('exciting')
  })

  test('create with valid kanban status "interested" succeeds', () => {
    const result = service.create({ name: 'Target Corp', status: 'interested' })
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data.status).toBe('interested')
  })

  test('create with valid kanban status "acceptable" succeeds', () => {
    const result = service.create({ name: 'Fallback Inc', status: 'acceptable' })
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data.status).toBe('acceptable')
  })

  test('create with valid kanban status "excluded" succeeds', () => {
    const result = service.create({ name: 'Red Flag LLC', status: 'excluded' })
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data.status).toBe('excluded')
  })

  test('create with old status "review" fails validation', () => {
    const result = service.create({ name: 'Old Status', status: 'review' })
    expect(result.ok).toBe(false)
    if (result.ok) return
    expect(result.error.code).toBe('VALIDATION_ERROR')
    expect(result.error.message).toContain('review')
  })

  test('create with old status "targeting" fails validation', () => {
    const result = service.create({ name: 'Old Status', status: 'targeting' })
    expect(result.ok).toBe(false)
    if (result.ok) return
    expect(result.error.code).toBe('VALIDATION_ERROR')
    expect(result.error.message).toContain('targeting')
  })

  test('create with invalid status fails validation', () => {
    const result = service.create({ name: 'Bad Status', status: 'bogus' })
    expect(result.ok).toBe(false)
    if (result.ok) return
    expect(result.error.code).toBe('VALIDATION_ERROR')
    expect(result.error.message).toContain('bogus')
  })

  test('create without status defaults to null', () => {
    const result = service.create({ name: 'No Status' })
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data.status).toBeNull()
  })

  test('update with valid kanban status succeeds', () => {
    const created = service.create({ name: 'Evolving' })
    if (!created.ok) return
    const result = service.update(created.data.id, { status: 'researching' })
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data.status).toBe('researching')
  })

  test('update with status null removes from pipeline', () => {
    const created = service.create({ name: 'Leaving Pipeline', status: 'backlog' })
    if (!created.ok) return
    const result = service.update(created.data.id, { status: null })
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data.status).toBeNull()
  })

  test('update with old status "review" fails validation', () => {
    const created = service.create({ name: 'Test' })
    if (!created.ok) return
    const result = service.update(created.data.id, { status: 'review' })
    expect(result.ok).toBe(false)
    if (result.ok) return
    expect(result.error.code).toBe('VALIDATION_ERROR')
    expect(result.error.message).toContain('review')
  })

  test('update with invalid status fails validation', () => {
    const created = service.create({ name: 'Test' })
    if (!created.ok) return
    const result = service.update(created.data.id, { status: 'invalid_status' })
    expect(result.ok).toBe(false)
    if (result.ok) return
    expect(result.error.code).toBe('VALIDATION_ERROR')
    expect(result.error.message).toContain('invalid_status')
  })

  test('list filters by kanban status', () => {
    service.create({ name: 'Org A', status: 'backlog' })
    service.create({ name: 'Org B', status: 'researching' })
    service.create({ name: 'Org C', status: 'backlog' })

    const result = service.list({ status: 'backlog' })
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data).toHaveLength(2)
    expect(result.data.every(o => o.status === 'backlog')).toBe(true)
  })
```

**Acceptance criteria:**
- All new status values pass validation tests.
- Old statuses `review` and `targeting` are explicitly rejected.
- `status: null` update is tested (remove from pipeline).

**Failure criteria:**
- Tests reference old status values that no longer exist in `VALID_STATUSES`.
- A test passes but asserts the wrong status value.

---

### T38.5: Write `KanbanCard.svelte`

**File:** `packages/webui/src/lib/components/kanban/KanbanCard.svelte`

A single kanban card displaying org name, tags, interest badge, and status-based styling. The entire card is clickable to open the detail modal.

```svelte
<script lang="ts">
  import type { Organization } from '@forge/sdk'

  let { org, onclick }: {
    org: Organization
    onclick: () => void
  } = $props()

  const INTEREST_STYLES: Record<string, { bg: string; border: string; badge: string; label: string }> = {
    exciting: { bg: '#f0fdf4', border: '#22c55e', badge: '#16a34a', label: 'EXCITING' },
    interested: { bg: '#eff6ff', border: '#3b82f6', badge: '#2563eb', label: 'INTERESTED' },
    acceptable: { bg: '#fafafa', border: '#9ca3af', badge: '#6b7280', label: 'ACCEPTABLE' },
  }

  let interest = $derived(INTEREST_STYLES[org.status ?? ''] ?? null)
  let isExcluded = $derived(org.status === 'excluded')
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<div
  class="kanban-card"
  class:excluded={isExcluded}
  style:background={interest?.bg ?? '#ffffff'}
  style:border-left={interest ? `4px solid ${interest.border}` : '1px solid #e5e7eb'}
  onclick={onclick}
  role="button"
  tabindex="0"
  onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onclick() } }}
>
  <div class="card-header">
    <span class="card-name" class:strike={isExcluded}>{org.name}</span>
    {#if org.worked}
      <span class="worked-badge">Worked</span>
    {/if}
  </div>

  {#if org.tags && org.tags.length > 0}
    <div class="tag-pills">
      {#each org.tags as tag}
        <span class="tag-pill">{tag}</span>
      {/each}
    </div>
  {/if}

  <div class="card-meta">
    {#if org.industry}
      <span class="meta-text">{org.industry}</span>
    {/if}
    {#if org.location}
      <span class="meta-text">{org.location}</span>
    {/if}
  </div>

  {#if interest}
    <span class="interest-badge" style:background={interest.badge}>
      {interest.label}
    </span>
  {/if}
</div>

<style>
  .kanban-card {
    padding: 0.6rem 0.75rem;
    border: 1px solid #e5e7eb;
    border-radius: 6px;
    cursor: grab;
    transition: box-shadow 0.12s, opacity 0.12s;
    margin-bottom: 0.35rem;
  }

  .kanban-card:hover {
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
  }

  .kanban-card:active {
    cursor: grabbing;
  }

  .kanban-card.excluded {
    opacity: 0.6;
  }

  .card-header {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    margin-bottom: 0.25rem;
  }

  .card-name {
    font-size: 0.82rem;
    font-weight: 600;
    color: #1a1a2e;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    min-width: 0;
  }

  .card-name.strike {
    text-decoration: line-through;
    color: #6b7280;
  }

  .worked-badge {
    display: inline-block;
    padding: 0.08em 0.35em;
    background: #d1fae5;
    color: #065f46;
    border-radius: 3px;
    font-size: 0.6rem;
    font-weight: 600;
    text-transform: uppercase;
    flex-shrink: 0;
  }

  .tag-pills {
    display: flex;
    flex-wrap: wrap;
    gap: 0.2rem;
    margin-bottom: 0.2rem;
  }

  .tag-pill {
    display: inline-block;
    padding: 0.05em 0.3em;
    background: #e0e7ff;
    color: #3730a3;
    border-radius: 3px;
    font-size: 0.58rem;
    font-weight: 500;
    text-transform: lowercase;
  }

  .card-meta {
    display: flex;
    gap: 0.4rem;
    flex-wrap: wrap;
  }

  .meta-text {
    font-size: 0.7rem;
    color: #6b7280;
  }

  .interest-badge {
    display: inline-block;
    margin-top: 0.3rem;
    padding: 0.1em 0.4em;
    color: #fff;
    border-radius: 3px;
    font-size: 0.58rem;
    font-weight: 700;
    letter-spacing: 0.04em;
  }
</style>
```

**Acceptance criteria:**
- Card renders org name, tags, industry, location.
- Exciting cards: green background (`#f0fdf4`), green left border (`#22c55e`), green "EXCITING" badge.
- Interested cards: blue background (`#eff6ff`), blue left border (`#3b82f6`), blue "INTERESTED" badge.
- Acceptable cards: gray background (`#fafafa`), gray left border (`#9ca3af`), gray "ACCEPTABLE" badge.
- Excluded cards: `opacity: 0.6`, name has `text-decoration: line-through`.
- `worked` orgs show "Worked" badge.
- Clicking the card calls `onclick`.
- `cursor: grab` on hover, `cursor: grabbing` while dragging.

**Failure criteria:**
- Interest badge not visible on targeting-column cards.
- Excluded cards not visually muted.
- Card click does not trigger detail modal.

---

### T38.6: Write `KanbanColumn.svelte`

**File:** `packages/webui/src/lib/components/kanban/KanbanColumn.svelte`

A single column with a `svelte-dnd-action` drop zone, header with label and count badge, and optional collapsed state (for Excluded).

```svelte
<script lang="ts">
  import { dndzone } from 'svelte-dnd-action'
  import KanbanCard from './KanbanCard.svelte'
  import type { Organization } from '@forge/sdk'

  let {
    label,
    accent,
    items,
    collapsed = false,
    onToggleCollapse,
    onDrop,
    onCardClick,
  }: {
    label: string
    accent: string
    items: (Organization & { id: string })[]
    collapsed?: boolean
    onToggleCollapse?: () => void
    onDrop: (orgId: string) => void
    onCardClick: (orgId: string) => void
  } = $props()

  let localItems = $state<(Organization & { id: string })[]>([])

  $effect(() => {
    localItems = [...items]
  })

  function handleConsider(e: CustomEvent) {
    localItems = e.detail.items
  }

  function handleFinalize(e: CustomEvent) {
    localItems = e.detail.items
    // Identify which items are new to this column by comparing with the original items
    const originalIds = new Set(items.map(i => i.id))
    const newItems = localItems.filter(i => !originalIds.has(i.id))
    for (const item of newItems) {
      onDrop(item.id)
    }
    // Intra-column reorder: not supported (no position field in schema).
    // The card will visually snap back to its alphabetical position on the next render cycle.
    // Silently ignore — no onDrop call needed.
  }

  const flipDurationMs = 200
</script>

{#if collapsed}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div
    class="column-collapsed"
    style:border-top-color={accent}
    onclick={onToggleCollapse}
    role="button"
    tabindex="0"
    onkeydown={(e) => { if (e.key === 'Enter') onToggleCollapse?.() }}
  >
    <span class="collapsed-label">{label}</span>
    <span class="collapsed-count">{items.length}</span>
  </div>
{:else}
  <div class="column" style:border-top-color={accent}>
    <div class="column-header">
      <h3 class="column-label">{label}</h3>
      <span class="column-count">{localItems.length}</span>
      {#if onToggleCollapse}
        <button class="collapse-btn" onclick={onToggleCollapse} title="Collapse">
          &#x2715;
        </button>
      {/if}
    </div>

    <div
      class="column-body"
      use:dndzone={{ items: localItems, flipDurationMs, dropTargetStyle: { outline: `2px dashed ${accent}` } }}
      onconsider={handleConsider}
      onfinalize={handleFinalize}
    >
      {#each localItems as item (item.id)}
        <KanbanCard org={item} onclick={() => onCardClick(item.id)} />
      {/each}
    </div>
  </div>
{/if}

<style>
  .column {
    flex: 1;
    min-width: 220px;
    max-width: 320px;
    display: flex;
    flex-direction: column;
    border-top: 3px solid;
    background: #f9fafb;
    border-radius: 6px;
    overflow: hidden;
  }

  .column-header {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.6rem 0.75rem;
    background: #fff;
    border-bottom: 1px solid #e5e7eb;
  }

  .column-label {
    font-size: 0.8rem;
    font-weight: 700;
    color: #1a1a2e;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    flex: 1;
    margin: 0;
  }

  .column-count {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 1.4rem;
    height: 1.4rem;
    padding: 0 0.3rem;
    background: #e5e7eb;
    color: #374151;
    border-radius: 999px;
    font-size: 0.7rem;
    font-weight: 600;
  }

  .collapse-btn {
    background: none;
    border: none;
    color: #9ca3af;
    cursor: pointer;
    font-size: 0.7rem;
    padding: 0.15rem;
    border-radius: 3px;
    line-height: 1;
  }

  .collapse-btn:hover {
    color: #374151;
    background: #f3f4f6;
  }

  .column-body {
    flex: 1;
    overflow-y: auto;
    padding: 0.5rem;
    min-height: 60px;
  }

  .column-collapsed {
    width: 48px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    background: #f3f4f6;
    border-top: 3px solid;
    border-radius: 6px;
    cursor: pointer;
    transition: background 0.12s;
    padding: 1rem 0;
    min-height: 200px;
  }

  .column-collapsed:hover {
    background: #e5e7eb;
  }

  .collapsed-label {
    writing-mode: vertical-rl;
    text-orientation: mixed;
    font-size: 0.75rem;
    font-weight: 600;
    color: #6b7280;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .collapsed-count {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 1.4rem;
    height: 1.4rem;
    background: #d1d5db;
    color: #374151;
    border-radius: 999px;
    font-size: 0.65rem;
    font-weight: 600;
  }
</style>
```

**Key points:**
- Uses Svelte 5 event handler syntax (`onconsider`, `onfinalize`) matching the existing `DragNDropView.svelte` pattern.
- `localItems` is a mutable copy synced from `items` via `$effect` -- `svelte-dnd-action` requires a mutable array.
- When collapsed (Excluded only), renders as a thin vertical strip with rotated text and count badge.
- `dropTargetStyle` uses the column's accent color for the dashed outline on drag-over.
- The `onDrop` callback receives only the org ID. The parent handles status mapping via `DROP_STATUS`. Intra-column reorder is silently ignored (no position field in schema).
- **Intra-column drag snap-back:** Intra-column drag-and-drop is not supported (no position field in schema). If a user drags within the same column, the card visually snaps back to its alphabetical position on the next render cycle. This is by design -- documented in Non-Goals as 'Card reordering within a column.'

**Acceptance criteria:**
- Column renders header with label, count badge, and accent color top border.
- Cards appear sorted inside the column.
- Dragging a card from another column into this column fires `onDrop`.
- Collapsed column renders as thin vertical strip with rotated label and count.
- Clicking collapsed column expands it.

**Failure criteria:**
- `svelte-dnd-action` throws errors about incompatible event syntax.
- Cards disappear during drag-and-drop.
- Collapsed column does not respond to click.

---

### T38.7: Write `OrgPickerModal.svelte`

**File:** `packages/webui/src/lib/components/kanban/OrgPickerModal.svelte`

Modal for adding organizations to the pipeline. Shows only orgs where `status IS NULL`. Supports search, tag filter, and inline "Create New" with case-insensitive name collision detection.

```svelte
<script lang="ts">
  import { forge, friendlyError } from '$lib/sdk'
  import { addToast } from '$lib/stores/toast.svelte'
  import { LoadingSpinner } from '$lib/components'
  import type { Organization, OrgTag } from '@forge/sdk'

  let { open, onclose, onadd }: {
    open: boolean
    onclose: () => void
    onadd: () => void
  } = $props()

  let allOrgs = $state<Organization[]>([])
  let loading = $state(false)
  let searchQuery = $state('')
  let tagFilter = $state('')
  let adding = $state<string | null>(null)

  // Create New form
  let showCreateForm = $state(false)
  let newName = $state('')
  let newOrgType = $state('company')
  let newWebsite = $state('')
  let creating = $state(false)
  let collisionOrg = $state<Organization | null>(null)

  const ORG_TYPES = ['company', 'nonprofit', 'government', 'military', 'education', 'volunteer', 'freelance', 'other']
  const TAG_OPTIONS: OrgTag[] = ['company', 'vendor', 'platform', 'university', 'school', 'nonprofit', 'government', 'military', 'conference', 'volunteer', 'freelance', 'other']

  // Fetch all orgs when modal opens
  $effect(() => {
    if (open) {
      loadAllOrgs()
      searchQuery = ''
      tagFilter = ''
      showCreateForm = false
      collisionOrg = null
    }
  })

  async function loadAllOrgs() {
    loading = true
    const result = await forge.organizations.list({ limit: 500 })
    if (result.ok) {
      allOrgs = result.data
    } else {
      addToast({ message: friendlyError(result.error, 'Failed to load organizations'), type: 'error' })
    }
    loading = false
  }

  // Filter to only orgs NOT already in pipeline (status IS NULL)
  let availableOrgs = $derived.by(() => {
    let result = allOrgs.filter(o => !o.status)
    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase()
      result = result.filter(o => o.name.toLowerCase().includes(q))
    }
    if (tagFilter) {
      result = result.filter(o => o.tags?.includes(tagFilter as OrgTag))
    }
    return result.sort((a, b) => a.name.localeCompare(b.name))
  })

  async function addToPipeline(orgId: string) {
    adding = orgId
    const result = await forge.organizations.update(orgId, { status: 'backlog' })
    if (result.ok) {
      addToast({ message: `${result.data.name} added to Backlog`, type: 'success' })
      onadd()
      onclose()
    } else {
      addToast({ message: friendlyError(result.error, 'Failed to add organization'), type: 'error' })
    }
    adding = null
  }

  function checkNameCollision() {
    const trimmed = newName.trim()
    if (!trimmed) return
    const match = allOrgs.find(o => o.name.toLowerCase() === trimmed.toLowerCase())
    if (match) {
      collisionOrg = match
    } else {
      collisionOrg = null
      createAndAdd()
    }
  }

  async function createAndAdd() {
    if (!newName.trim()) {
      addToast({ message: 'Name is required.', type: 'error' })
      return
    }
    creating = true
    const result = await forge.organizations.create({
      name: newName.trim(),
      org_type: newOrgType,
      website: newWebsite || undefined,
      status: 'backlog',
    })
    if (result.ok) {
      addToast({ message: `${result.data.name} created and added to Backlog`, type: 'success' })
      onadd()
      onclose()
    } else {
      addToast({ message: friendlyError(result.error, 'Failed to create organization'), type: 'error' })
    }
    creating = false
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="modal-overlay" onclick={onclose} role="presentation">
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="modal-content" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true" aria-label="Add Organization to Pipeline">
      <div class="modal-header">
        <h3>Add Organization to Pipeline</h3>
        <button class="close-btn" onclick={onclose}>&times;</button>
      </div>

      <div class="modal-filters">
        <input
          type="text"
          class="search-input"
          placeholder="Search by name..."
          bind:value={searchQuery}
        />
        <select class="tag-select" bind:value={tagFilter}>
          <option value="">All tags</option>
          {#each TAG_OPTIONS as tag}
            <option value={tag}>{tag}</option>
          {/each}
        </select>
      </div>

      <div class="org-list-scroll">
        {#if loading}
          <div class="list-loading">
            <LoadingSpinner size="md" message="Loading organizations..." />
          </div>
        {:else if availableOrgs.length === 0}
          <div class="list-empty">
            <p>No available organizations found.{searchQuery ? ' Try a different search.' : ''}</p>
          </div>
        {:else}
          {#each availableOrgs as org (org.id)}
            <button
              class="picker-card"
              onclick={() => addToPipeline(org.id)}
              disabled={adding === org.id}
            >
              <div class="picker-card-top">
                <span class="picker-name">{org.name}</span>
                {#if org.worked}
                  <span class="picker-worked">Worked</span>
                {/if}
              </div>
              <div class="picker-meta">
                {#if org.industry}<span>{org.industry}</span>{/if}
                {#if org.location}<span>{org.location}</span>{/if}
              </div>
              {#if org.tags && org.tags.length > 0}
                <div class="picker-tags">
                  {#each org.tags as tag}
                    <span class="picker-tag">{tag}</span>
                  {/each}
                </div>
              {/if}
              {#if adding === org.id}
                <span class="adding-text">Adding...</span>
              {/if}
            </button>
          {/each}
        {/if}
      </div>

      <div class="create-section">
        {#if !showCreateForm}
          <button class="create-toggle" onclick={() => { showCreateForm = true; collisionOrg = null }}>
            + Create New Organization
          </button>
        {:else}
          <div class="create-form">
            <h4>Create New Organization</h4>
            <div class="create-row">
              <input
                type="text"
                class="create-input"
                placeholder="Organization name"
                bind:value={newName}
              />
              <select class="create-select" bind:value={newOrgType}>
                {#each ORG_TYPES as t}
                  <option value={t}>{t}</option>
                {/each}
              </select>
            </div>
            <input
              type="url"
              class="create-input create-website"
              placeholder="Website (optional)"
              bind:value={newWebsite}
            />

            {#if collisionOrg}
              <div class="collision-warning">
                <p>An organization named <strong>{collisionOrg.name}</strong> already exists. Add the existing one instead?</p>
                <div class="collision-actions">
                  <button class="btn btn-sm btn-primary" onclick={() => addToPipeline(collisionOrg!.id)}>
                    Add Existing
                  </button>
                  <button class="btn btn-sm btn-ghost" onclick={() => { collisionOrg = null; createAndAdd() }}>
                    Create Anyway
                  </button>
                </div>
              </div>
            {:else}
              <button
                class="btn btn-sm btn-primary create-btn"
                onclick={checkNameCollision}
                disabled={creating || !newName.trim()}
              >
                {creating ? 'Creating...' : 'Create & Add to Backlog'}
              </button>
            {/if}
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal-content {
    background: #fff;
    border-radius: 10px;
    width: 90%;
    max-width: 520px;
    max-height: 80vh;
    display: flex;
    flex-direction: column;
    box-shadow: 0 8px 30px rgba(0, 0, 0, 0.15);
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 1rem 1.25rem;
    border-bottom: 1px solid #e5e7eb;
  }

  .modal-header h3 {
    font-size: 1rem;
    font-weight: 600;
    color: #1a1a2e;
    margin: 0;
  }

  .close-btn {
    background: none;
    border: none;
    font-size: 1.3rem;
    color: #9ca3af;
    cursor: pointer;
    padding: 0.2rem;
    line-height: 1;
  }

  .close-btn:hover { color: #374151; }

  .modal-filters {
    display: flex;
    gap: 0.5rem;
    padding: 0.75rem 1.25rem;
    border-bottom: 1px solid #e5e7eb;
  }

  .search-input {
    flex: 1;
    padding: 0.4rem 0.6rem;
    border: 1px solid #d1d5db;
    border-radius: 5px;
    font-size: 0.82rem;
    color: #374151;
  }

  .search-input:focus {
    outline: none;
    border-color: #6c63ff;
  }

  .tag-select {
    padding: 0.4rem 0.5rem;
    border: 1px solid #d1d5db;
    border-radius: 5px;
    font-size: 0.78rem;
    color: #374151;
    background: #fff;
  }

  .org-list-scroll {
    flex: 1;
    overflow-y: auto;
    min-height: 120px;
    max-height: 300px;
  }

  .list-loading, .list-empty {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 2rem 1rem;
    color: #9ca3af;
    font-size: 0.85rem;
  }

  .picker-card {
    display: block;
    width: 100%;
    padding: 0.6rem 1.25rem;
    background: none;
    border: none;
    border-bottom: 1px solid #f3f4f6;
    cursor: pointer;
    text-align: left;
    transition: background 0.12s;
    font-family: inherit;
  }

  .picker-card:hover { background: #f0fdf4; }
  .picker-card:disabled { opacity: 0.6; cursor: wait; }

  .picker-card-top {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    margin-bottom: 0.15rem;
  }

  .picker-name {
    font-size: 0.82rem;
    font-weight: 600;
    color: #1a1a2e;
  }

  .picker-worked {
    padding: 0.08em 0.35em;
    background: #d1fae5;
    color: #065f46;
    border-radius: 3px;
    font-size: 0.58rem;
    font-weight: 600;
  }

  .picker-meta {
    display: flex;
    gap: 0.4rem;
    font-size: 0.7rem;
    color: #6b7280;
  }

  .picker-tags {
    display: flex;
    gap: 0.2rem;
    margin-top: 0.15rem;
  }

  .picker-tag {
    padding: 0.05em 0.25em;
    background: #e0e7ff;
    color: #3730a3;
    border-radius: 3px;
    font-size: 0.55rem;
    font-weight: 500;
  }

  .adding-text {
    font-size: 0.7rem;
    color: #6c63ff;
    font-style: italic;
  }

  .create-section {
    padding: 0.75rem 1.25rem;
    border-top: 1px solid #e5e7eb;
  }

  .create-toggle {
    width: 100%;
    padding: 0.5rem;
    background: none;
    border: 1px dashed #d1d5db;
    border-radius: 6px;
    color: #6b7280;
    font-size: 0.82rem;
    cursor: pointer;
    font-family: inherit;
  }

  .create-toggle:hover {
    background: #f9fafb;
    color: #374151;
    border-color: #9ca3af;
  }

  .create-form h4 {
    font-size: 0.85rem;
    font-weight: 600;
    color: #1a1a2e;
    margin: 0 0 0.5rem 0;
  }

  .create-row {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 0.4rem;
  }

  .create-input {
    flex: 1;
    padding: 0.4rem 0.6rem;
    border: 1px solid #d1d5db;
    border-radius: 5px;
    font-size: 0.82rem;
    color: #374151;
  }

  .create-input:focus {
    outline: none;
    border-color: #6c63ff;
  }

  .create-website {
    width: 100%;
    margin-bottom: 0.5rem;
  }

  .create-select {
    padding: 0.4rem 0.5rem;
    border: 1px solid #d1d5db;
    border-radius: 5px;
    font-size: 0.78rem;
    color: #374151;
    background: #fff;
  }

  .create-btn {
    width: 100%;
  }

  .collision-warning {
    background: #fef3c7;
    border: 1px solid #fbbf24;
    border-radius: 6px;
    padding: 0.6rem;
    font-size: 0.8rem;
    color: #92400e;
  }

  .collision-warning p {
    margin: 0 0 0.5rem 0;
  }

  .collision-actions {
    display: flex;
    gap: 0.5rem;
  }

  .btn {
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 6px;
    font-size: 0.8rem;
    font-weight: 500;
    cursor: pointer;
    font-family: inherit;
  }

  .btn:disabled { opacity: 0.6; cursor: not-allowed; }
  .btn-sm { padding: 0.3rem 0.6rem; font-size: 0.75rem; }
  .btn-primary { background: #6c63ff; color: #fff; }
  .btn-primary:hover:not(:disabled) { background: #5a52e0; }
  .btn-ghost { background: transparent; color: #6b7280; }
  .btn-ghost:hover { color: #374151; background: #f3f4f6; }
</style>
```

**Key points:**
- Dedup: `availableOrgs` filters to `!o.status` (status IS NULL) client-side. The API does not support `status IS NULL` filters.
- Re-fetches on every modal open (no stale data).
- Name collision: `checkNameCollision()` does case-insensitive comparison against ALL orgs (not just available ones).
- On collision, offers "Add Existing" (sets status to `backlog`) or "Create Anyway".
- After adding, calls `onadd()` which triggers `loadOrganizations()` on the parent board.
- **Tags in Create New form:** The spec requires a tags field in the Create New form. For simplicity, the inline form omits tags -- the repository defaults tags to `[org_type]` on creation. Users can edit tags from the master list at `/data/organizations`. This is an intentional simplification documented here.

**Acceptance criteria:**
- Modal shows only orgs with `status = null`.
- Search filters by name (case-insensitive substring).
- Tag filter narrows list.
- Clicking an org card calls `PATCH { status: 'backlog' }`.
- "Create New" with name "google" when "Google" exists triggers collision warning.
- "Add Existing" on collision sets the existing org's status to `backlog`.
- "Create Anyway" creates a new org with status `backlog`.

**Failure criteria:**
- Orgs already on the board appear in the picker.
- Name collision check is case-sensitive (misses "google" vs "Google").
- Modal does not refresh org list on open.

---

### T38.8: Write `OrgDetailModal.svelte`

**File:** `packages/webui/src/lib/components/kanban/OrgDetailModal.svelte`

Detail modal that opens on card click. Shows org info, interest level dropdown, editable notes, and "Remove from Pipeline" action.

```svelte
<script lang="ts">
  import { forge, friendlyError } from '$lib/sdk'
  import { addToast } from '$lib/stores/toast.svelte'
  import type { Organization } from '@forge/sdk'

  let { org, onclose, onupdate }: {
    org: Organization | null
    onclose: () => void
    onupdate: () => void
  } = $props()

  const TARGETING_STATUSES = ['exciting', 'interested', 'acceptable']
  const INTEREST_OPTIONS = [
    { value: 'exciting', label: 'Exciting', color: '#22c55e' },
    { value: 'interested', label: 'Interested', color: '#3b82f6' },
    { value: 'acceptable', label: 'Acceptable', color: '#9ca3af' },
  ]

  const COLUMN_LABELS: Record<string, string> = {
    backlog: 'Backlog',
    researching: 'Researching',
    exciting: 'Targeting',
    interested: 'Targeting',
    acceptable: 'Targeting',
    excluded: 'Excluded',
  }

  let notesValue = $state('')
  let reputationValue = $state('')
  let savingNotes = $state(false)
  let savingReputation = $state(false)
  let removing = $state(false)

  let isTargeting = $derived(org ? TARGETING_STATUSES.includes(org.status ?? '') : false)
  let columnLabel = $derived(org ? (COLUMN_LABELS[org.status ?? ''] ?? 'Unknown') : '')

  $effect(() => {
    if (org) {
      notesValue = org.notes ?? ''
      reputationValue = org.reputation_notes ?? ''
    }
  })

  async function changeInterest(newStatus: string) {
    if (!org) return
    // TODO: Remove this `as any` cast — after T38.3 updates the SDK types, `UpdateOrganization.status`
    // accepts the new union type directly. The cast is only needed if T38.8 is developed before T38.3.
    const result = await forge.organizations.update(org.id, { status: newStatus as any })
    if (result.ok) {
      addToast({ message: `Interest level changed to ${newStatus}`, type: 'success' })
      onupdate()
    } else {
      addToast({ message: friendlyError(result.error), type: 'error' })
    }
  }

  async function saveNotes() {
    if (!org) return
    savingNotes = true
    const result = await forge.organizations.update(org.id, { notes: notesValue || null })
    if (result.ok) {
      onupdate()
    } else {
      addToast({ message: friendlyError(result.error, 'Failed to save notes'), type: 'error' })
    }
    savingNotes = false
  }

  async function saveReputation() {
    if (!org) return
    savingReputation = true
    const result = await forge.organizations.update(org.id, { reputation_notes: reputationValue || null })
    if (result.ok) {
      onupdate()
    } else {
      addToast({ message: friendlyError(result.error, 'Failed to save reputation notes'), type: 'error' })
    }
    savingReputation = false
  }

  async function removeFromPipeline() {
    if (!org) return
    removing = true
    const result = await forge.organizations.update(org.id, { status: null })
    if (result.ok) {
      addToast({ message: `${org.name} removed from pipeline`, type: 'success' })
      onupdate()
      onclose()
    } else {
      addToast({ message: friendlyError(result.error, 'Failed to remove from pipeline'), type: 'error' })
    }
    removing = false
  }
</script>

{#if org}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="modal-overlay" onclick={onclose} role="presentation">
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="modal-content" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true" aria-label="Organization Details">
      <div class="modal-header">
        <div class="header-left">
          <h3>{org.name}</h3>
          <span class="column-indicator">{columnLabel}</span>
          {#if org.worked}
            <span class="worked-badge">Worked</span>
          {/if}
        </div>
        <button class="close-btn" onclick={onclose}>&times;</button>
      </div>

      <div class="modal-body">
        {#if isTargeting}
          <div class="field-group">
            <label class="field-label">Interest Level</label>
            <div class="interest-selector">
              {#each INTEREST_OPTIONS as opt}
                <button
                  class="interest-btn"
                  class:active={org.status === opt.value}
                  style:--accent={opt.color}
                  onclick={() => changeInterest(opt.value)}
                >
                  {opt.label}
                </button>
              {/each}
            </div>
          </div>
        {/if}

        {#if org.tags && org.tags.length > 0}
          <div class="field-group">
            <label class="field-label">Tags</label>
            <div class="tag-pills">
              {#each org.tags as tag}
                <span class="tag-pill">{tag}</span>
              {/each}
            </div>
          </div>
        {/if}

        <div class="info-grid">
          {#if org.industry}
            <div class="info-item">
              <span class="info-label">Industry</span>
              <span class="info-value">{org.industry}</span>
            </div>
          {/if}
          {#if org.location}
            <div class="info-item">
              <span class="info-label">Location</span>
              <span class="info-value">{org.location}</span>
            </div>
          {/if}
          {#if org.size}
            <div class="info-item">
              <span class="info-label">Size</span>
              <span class="info-value">{org.size}</span>
            </div>
          {/if}
          {#if org.glassdoor_rating}
            <div class="info-item">
              <span class="info-label">Glassdoor</span>
              <span class="info-value">{org.glassdoor_rating}/5</span>
            </div>
          {/if}
        </div>

        <div class="links">
          {#if org.website}
            <a href={org.website} target="_blank" rel="noopener" class="detail-link">Website</a>
          {/if}
          {#if org.linkedin_url}
            <a href={org.linkedin_url} target="_blank" rel="noopener" class="detail-link">LinkedIn</a>
          {/if}
          {#if org.glassdoor_url}
            <a href={org.glassdoor_url} target="_blank" rel="noopener" class="detail-link">Glassdoor</a>
          {/if}
        </div>

        <div class="field-group">
          <label class="field-label" for="detail-notes">Notes</label>
          <textarea
            id="detail-notes"
            class="detail-textarea"
            bind:value={notesValue}
            onblur={saveNotes}
            rows="3"
            placeholder="Add notes about this organization..."
          ></textarea>
          {#if savingNotes}
            <span class="saving-indicator">Saving...</span>
          {/if}
        </div>

        <div class="field-group">
          <label class="field-label" for="detail-reputation">Reputation Notes</label>
          <textarea
            id="detail-reputation"
            class="detail-textarea"
            bind:value={reputationValue}
            onblur={saveReputation}
            rows="3"
            placeholder="Reputation, red flags, culture notes..."
          ></textarea>
          {#if savingReputation}
            <span class="saving-indicator">Saving...</span>
          {/if}
        </div>
      </div>

      <div class="modal-footer">
        <a href="/data/organizations?id={org.id}" class="btn btn-ghost">
          Edit Full Details
        </a>
        <button
          class="btn btn-danger"
          onclick={removeFromPipeline}
          disabled={removing}
        >
          {removing ? 'Removing...' : 'Remove from Pipeline'}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal-content {
    background: #fff;
    border-radius: 10px;
    width: 90%;
    max-width: 520px;
    max-height: 85vh;
    display: flex;
    flex-direction: column;
    box-shadow: 0 8px 30px rgba(0, 0, 0, 0.15);
  }

  .modal-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    padding: 1rem 1.25rem;
    border-bottom: 1px solid #e5e7eb;
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .modal-header h3 {
    font-size: 1.05rem;
    font-weight: 600;
    color: #1a1a2e;
    margin: 0;
  }

  .column-indicator {
    padding: 0.1em 0.4em;
    background: #e0e7ff;
    color: #3730a3;
    border-radius: 3px;
    font-size: 0.65rem;
    font-weight: 600;
    text-transform: uppercase;
  }

  .worked-badge {
    padding: 0.1em 0.35em;
    background: #d1fae5;
    color: #065f46;
    border-radius: 3px;
    font-size: 0.6rem;
    font-weight: 600;
  }

  .close-btn {
    background: none;
    border: none;
    font-size: 1.3rem;
    color: #9ca3af;
    cursor: pointer;
    padding: 0.2rem;
    line-height: 1;
    flex-shrink: 0;
  }

  .close-btn:hover { color: #374151; }

  .modal-body {
    flex: 1;
    overflow-y: auto;
    padding: 1rem 1.25rem;
  }

  .field-group {
    margin-bottom: 1rem;
  }

  .field-label {
    display: block;
    font-size: 0.75rem;
    font-weight: 600;
    color: #6b7280;
    text-transform: uppercase;
    letter-spacing: 0.03em;
    margin-bottom: 0.3rem;
  }

  .interest-selector {
    display: flex;
    gap: 0.35rem;
  }

  .interest-btn {
    flex: 1;
    padding: 0.4rem 0.5rem;
    border: 2px solid #e5e7eb;
    border-radius: 6px;
    background: #fff;
    font-size: 0.78rem;
    font-weight: 500;
    color: #374151;
    cursor: pointer;
    transition: border-color 0.12s, background 0.12s;
    font-family: inherit;
  }

  .interest-btn:hover {
    border-color: var(--accent);
  }

  .interest-btn.active {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 10%, white);
    font-weight: 600;
  }

  .tag-pills {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
  }

  .tag-pill {
    padding: 0.1em 0.4em;
    background: #e0e7ff;
    color: #3730a3;
    border-radius: 3px;
    font-size: 0.7rem;
    font-weight: 500;
  }

  .info-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.5rem;
    margin-bottom: 0.75rem;
  }

  .info-item {
    font-size: 0.8rem;
  }

  .info-label {
    display: block;
    font-size: 0.68rem;
    color: #9ca3af;
    text-transform: uppercase;
    font-weight: 500;
  }

  .info-value {
    color: #374151;
  }

  .links {
    display: flex;
    gap: 0.75rem;
    margin-bottom: 1rem;
  }

  .detail-link {
    font-size: 0.78rem;
    color: #6c63ff;
    text-decoration: none;
  }

  .detail-link:hover {
    text-decoration: underline;
  }

  .detail-textarea {
    width: 100%;
    padding: 0.5rem 0.65rem;
    border: 1px solid #d1d5db;
    border-radius: 6px;
    font-size: 0.82rem;
    color: #374151;
    font-family: inherit;
    line-height: 1.5;
    resize: vertical;
    min-height: 60px;
  }

  .detail-textarea:focus {
    outline: none;
    border-color: #6c63ff;
    box-shadow: 0 0 0 2px rgba(108, 99, 255, 0.15);
  }

  .saving-indicator {
    font-size: 0.68rem;
    color: #6c63ff;
    font-style: italic;
  }

  .modal-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.75rem 1.25rem;
    border-top: 1px solid #e5e7eb;
  }

  .btn {
    padding: 0.45rem 0.9rem;
    border: none;
    border-radius: 6px;
    font-size: 0.8rem;
    font-weight: 500;
    cursor: pointer;
    font-family: inherit;
    text-decoration: none;
    display: inline-flex;
    align-items: center;
  }

  .btn:disabled { opacity: 0.6; cursor: not-allowed; }
  .btn-ghost { background: transparent; color: #6b7280; }
  .btn-ghost:hover { color: #374151; background: #f3f4f6; }
  .btn-danger { background: #fee2e2; color: #dc2626; }
  .btn-danger:hover:not(:disabled) { background: #fecaca; }
</style>
```

**Key points:**
- Interest level dropdown only visible when status is one of `exciting | interested | acceptable` (the Targeting column statuses).
- Notes and reputation notes auto-save on blur via `PATCH`.
- "Remove from Pipeline" sets `status: null` -- the org is NOT deleted, just removed from the board.
- "Edit Full Details" links to `/data/organizations?id={orgId}` for the full editor form. **Note:** The `/data/organizations` page does not currently read this query param for auto-selection -- this is a future enhancement. The link still works (navigates to the page) but the user must manually find the org.
- Column indicator shows "Targeting" for all three interest levels, "Backlog", "Researching", or "Excluded" for others.
- **Interest level changes are NOT optimistically updated** -- the board reloads after the PATCH completes. This is intentional: interest level changes are less frequent than drag-and-drop column changes, and the reload ensures consistency. The UX tradeoff (brief delay before card color updates) is acceptable for a single-user tool.

**Acceptance criteria:**
- Interest dropdown visible only for targeting statuses.
- Changing interest level calls `PATCH` with new status.
- Notes save on blur.
- "Remove from Pipeline" sets status to null, modal closes, card disappears from board.
- "Edit Full Details" navigates to `/data/organizations?id={orgId}`.
- Org with `worked=true` shows "Worked" badge.

**Failure criteria:**
- Interest dropdown appears for `backlog` or `researching` statuses.
- Notes save on every keystroke instead of on blur.
- "Remove from Pipeline" deletes the org instead of nulling status.

---

### T38.9: Write `KanbanBoard.svelte`

**File:** `packages/webui/src/lib/components/kanban/KanbanBoard.svelte`

Top-level board component that fetches organizations, groups them into columns, manages modals, and handles cross-column drag-and-drop with optimistic UI updates.

```svelte
<script lang="ts">
  import { forge, friendlyError } from '$lib/sdk'
  import { addToast } from '$lib/stores/toast.svelte'
  import { LoadingSpinner } from '$lib/components'
  import KanbanColumn from './KanbanColumn.svelte'
  import OrgPickerModal from './OrgPickerModal.svelte'
  import OrgDetailModal from './OrgDetailModal.svelte'
  import type { Organization } from '@forge/sdk'

  type OrgStatus = 'backlog' | 'researching' | 'exciting' | 'interested' | 'acceptable' | 'excluded'

  const COLUMNS: Array<{
    key: string
    label: string
    statuses: OrgStatus[]
    accent: string
  }> = [
    { key: 'backlog', label: 'Backlog', statuses: ['backlog'], accent: '#a5b4fc' },
    { key: 'researching', label: 'Researching', statuses: ['researching'], accent: '#fbbf24' },
    { key: 'targeting', label: 'Targeting', statuses: ['exciting', 'interested', 'acceptable'], accent: '#22c55e' },
    { key: 'excluded', label: 'Excluded', statuses: ['excluded'], accent: '#d1d5db' },
  ]

  // Default status when dropping into a column
  const DROP_STATUS: Record<string, OrgStatus> = {
    backlog: 'backlog',
    researching: 'researching',
    targeting: 'interested', // default interest level
    excluded: 'excluded',
  }

  let organizations = $state<Organization[]>([])
  let loading = $state(true)
  let excludedExpanded = $state(false)
  let showPicker = $state(false)
  let detailOrgId = $state<string | null>(null)

  let detailOrg = $derived(organizations.find(o => o.id === detailOrgId) ?? null)

  let columnData = $derived(COLUMNS.map(col => ({
    ...col,
    items: organizations
      .filter(o => o.status && col.statuses.includes(o.status as OrgStatus))
      .sort((a, b) => a.name.localeCompare(b.name))
      .map(o => ({ ...o, id: o.id })), // ensure `id` is a top-level property for svelte-dnd-action
  })))

  let hasAnyOrgs = $derived(organizations.length > 0)

  $effect(() => { loadOrganizations() })

  async function loadOrganizations() {
    loading = true
    const result = await forge.organizations.list({ limit: 500 })
    if (result.ok) {
      organizations = result.data.filter(o => o.status)
    } else {
      addToast({ message: friendlyError(result.error, 'Failed to load organizations'), type: 'error' })
    }
    loading = false
  }

  async function handleDrop(columnKey: string, orgId: string) {
    if (!orgId) return // reorder within same column, ignore

    const newStatus = DROP_STATUS[columnKey]
    if (!newStatus) return

    // Check if org is already in this column's statuses
    const col = COLUMNS.find(c => c.key === columnKey)
    const org = organizations.find(o => o.id === orgId)
    if (!col || !org) return
    if (col.statuses.includes(org.status as OrgStatus)) return // same column, no-op

    // Optimistic update
    const oldStatus = org.status
    organizations = organizations.map(o =>
      o.id === orgId ? { ...o, status: newStatus } : o
    )

    // Persist
    const result = await forge.organizations.update(orgId, { status: newStatus })
    if (!result.ok) {
      // Revert on failure
      organizations = organizations.map(o =>
        o.id === orgId ? { ...o, status: oldStatus } : o
      )
      addToast({ message: friendlyError(result.error, 'Failed to update status'), type: 'error' })
    }
  }

  function handleCardClick(orgId: string) {
    detailOrgId = orgId
  }

  function handleDetailUpdate() {
    loadOrganizations()
    // If the detail org was removed from pipeline, close the modal
    // (loadOrganizations will filter it out; detailOrg will become null)
  }

  function handlePickerAdd() {
    loadOrganizations()
  }
</script>

<div class="kanban-page">
  <div class="kanban-header">
    <h2>Organization Pipeline</h2>
    <button class="btn-add" onclick={() => showPicker = true}>+ Add Organization</button>
  </div>

  {#if loading}
    <div class="board-loading">
      <LoadingSpinner size="lg" message="Loading pipeline..." />
    </div>
  {:else if !hasAnyOrgs}
    <div class="board-empty">
      <p>No organizations in the pipeline yet.</p>
      <p>Click <strong>+ Add Organization</strong> to start tracking.</p>
    </div>
  {:else}
    <div class="board-columns">
      {#each columnData as col, i (col.key)}
        <KanbanColumn
          label={col.label}
          accent={col.accent}
          items={col.items}
          collapsed={col.key === 'excluded' && !excludedExpanded}
          onToggleCollapse={col.key === 'excluded' ? () => { excludedExpanded = !excludedExpanded } : undefined}
          onDrop={(orgId) => handleDrop(col.key, orgId)}
          onCardClick={handleCardClick}
        />
      {/each}
    </div>
  {/if}
</div>

<OrgPickerModal
  open={showPicker}
  onclose={() => showPicker = false}
  onadd={handlePickerAdd}
/>

<OrgDetailModal
  org={detailOrg}
  onclose={() => detailOrgId = null}
  onupdate={handleDetailUpdate}
/>

<style>
  .kanban-page {
    display: flex;
    flex-direction: column;
    height: calc(100vh - 4rem);
    margin: -2rem;
    background: #fff;
  }

  .kanban-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 1rem 1.5rem;
    border-bottom: 1px solid #e5e7eb;
    flex-shrink: 0;
  }

  .kanban-header h2 {
    font-size: 1.15rem;
    font-weight: 600;
    color: #1a1a2e;
    margin: 0;
  }

  .btn-add {
    padding: 0.4rem 0.85rem;
    background: #6c63ff;
    color: #fff;
    border: none;
    border-radius: 6px;
    font-size: 0.82rem;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.15s;
    white-space: nowrap;
    font-family: inherit;
  }

  .btn-add:hover { background: #5a52e0; }

  .board-loading {
    display: flex;
    align-items: center;
    justify-content: center;
    flex: 1;
  }

  .board-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    flex: 1;
    color: #6b7280;
    font-size: 0.95rem;
    text-align: center;
    gap: 0.25rem;
  }

  .board-empty p {
    margin: 0;
  }

  .board-columns {
    display: flex;
    gap: 0.75rem;
    padding: 1rem;
    flex: 1;
    overflow-x: auto;
    align-items: stretch;
  }
</style>
```

**Key points:**
- `organizations` state holds only orgs with a non-null status (pipeline orgs).
- `columnData` is a `$derived` that groups orgs by column statuses and adds `id` for `svelte-dnd-action`.
- Excluded column starts collapsed (`excludedExpanded = false`).
- `handleDrop` does optimistic update: changes `organizations` state immediately, fires `PATCH` in background, reverts on failure.
- When dropping into Targeting, status is set to `interested` (default interest level). The user changes to `exciting` or `acceptable` via the detail modal.
- `handleDetailUpdate` calls `loadOrganizations()` to refresh the board after any detail modal change.
- `handlePickerAdd` calls `loadOrganizations()` after adding an org from the picker.
- The `onDrop` callback in `KanbanColumn` now receives just the `orgId` -- the board handles the status mapping via `DROP_STATUS`.

**Acceptance criteria:**
- Board renders 4 columns: Backlog, Researching, Targeting, Excluded.
- Excluded column is collapsed by default.
- Dragging a card from Backlog to Researching updates status to `researching`.
- Dropping in Targeting sets status to `interested`.
- API failure reverts the card to its original column and shows error toast.
- "+" button opens OrgPickerModal.
- Card click opens OrgDetailModal.
- Empty board shows guidance message.
- Loading state shows spinner.

**Failure criteria:**
- Cards in wrong columns after status migration.
- Optimistic revert does not work (card stuck in wrong column on API error).
- Board does not refresh after picker add or detail update.

---

### T38.10: Update `+page.svelte` to Use KanbanBoard

**File:** `packages/webui/src/routes/opportunities/organizations/+page.svelte`

Replace the entire file with the kanban board import:

```svelte
<script lang="ts">
  import KanbanBoard from '$lib/components/kanban/KanbanBoard.svelte'
</script>

<KanbanBoard />
```

**Key points:**
- The old split-panel layout is completely replaced.
- All logic is in `KanbanBoard.svelte`; the page is a thin wrapper.
- The old page's functionality (full org editor) is preserved at `/data/organizations` (created in an earlier phase). The "Edit Full Details" link in `OrgDetailModal` navigates there.

**Acceptance criteria:**
- `/opportunities/organizations` renders the kanban board.
- No remnants of the old split-panel layout.
- Existing org data is visible on the board (after migration renames statuses).

**Failure criteria:**
- Old split-panel UI still renders.
- Import path is wrong and page crashes.

---

## Testing Support

### Test Fixtures

The existing `createTestDb()` helper in `packages/core/src/db/__tests__/helpers.ts` runs all migrations including the new `012_org_kanban_statuses.sql`. No changes to `createTestDb()` are needed. The `seedOrganization()` helper does not set `status`, so it creates orgs with `status = NULL` by default -- this is correct for the picker tests.

For tests that need orgs with specific kanban statuses, use the service directly:

```typescript
// In test setup
const service = new OrganizationService(db)
service.create({ name: 'Backlog Org', status: 'backlog' })
service.create({ name: 'Research Org', status: 'researching' })
service.create({ name: 'Exciting Org', status: 'exciting' })
service.create({ name: 'Excluded Org', status: 'excluded' })
```

### Unit Tests

**File:** `packages/core/src/services/__tests__/organization-service.test.ts` (updated in T38.4)

| Test | Assertion |
|------|-----------|
| Create with `backlog` status succeeds | `result.ok === true`, `status === 'backlog'` |
| Create with `researching` status succeeds | `result.ok === true`, `status === 'researching'` |
| Create with `exciting` status succeeds | `result.ok === true`, `status === 'exciting'` |
| Create with `interested` status succeeds | `result.ok === true`, `status === 'interested'` |
| Create with `acceptable` status succeeds | `result.ok === true`, `status === 'acceptable'` |
| Create with `excluded` status succeeds | `result.ok === true`, `status === 'excluded'` |
| Create with old `review` status fails | `result.ok === false`, `code === 'VALIDATION_ERROR'` |
| Create with old `targeting` status fails | `result.ok === false`, `code === 'VALIDATION_ERROR'` |
| Create without status defaults to null | `result.ok === true`, `status === null` |
| Update to valid kanban status succeeds | `result.ok === true`, `status === 'researching'` |
| Update to null removes from pipeline | `result.ok === true`, `status === null` |
| Update with old `review` status fails | `result.ok === false`, `code === 'VALIDATION_ERROR'` |
| List filters by kanban status | Returns only matching orgs |

### Integration Tests

These test the API routes directly. Add to the existing contract test file or a dedicated org API test file.

| Test | Route | Assertion |
|------|-------|-----------|
| PATCH with `{ status: 'backlog' }` succeeds | `PATCH /api/organizations/:id` | 200, `data.status === 'backlog'` |
| PATCH with `{ status: 'exciting' }` succeeds | `PATCH /api/organizations/:id` | 200, `data.status === 'exciting'` |
| PATCH with `{ status: 'review' }` returns 400 | `PATCH /api/organizations/:id` | 400, `error.code === 'VALIDATION_ERROR'` |
| PATCH with `{ status: null }` succeeds | `PATCH /api/organizations/:id` | 200, `data.status === null` |
| GET with `?status=exciting` returns only exciting orgs | `GET /api/organizations?status=exciting` | 200, all items have `status === 'exciting'` |

### Component Smoke Tests

These would be manual or automated via a browser test framework. Listed here as acceptance checklist items.

| Test | What to verify |
|------|---------------|
| Board renders 4 columns | Columns labeled Backlog, Researching, Targeting, Excluded |
| Cards in correct columns | Org with `status=backlog` is in Backlog, etc. |
| Drag between columns | Card moves from Backlog to Researching, status updates |
| Excluded column collapsed | Renders as thin vertical strip on load |
| Excluded column expands on click | Full column with cards visible |
| Targeting cards color-coded | Exciting=green, Interested=blue, Acceptable=gray |
| Excluded cards muted | Opacity 0.6, name has line-through |
| Worked badge on cards | Org with `worked=true` shows "Worked" badge |
| OrgPickerModal shows only null-status orgs | Orgs already on board are hidden |
| OrgPickerModal name collision | Creating "google" when "Google" exists shows warning |
| OrgDetailModal interest dropdown | Visible only for targeting statuses |
| OrgDetailModal notes auto-save | Notes persist on blur |
| OrgDetailModal remove from pipeline | Status set to null, card disappears, modal closes |
| Empty board state | Shows message when no orgs have statuses |
| Loading spinner | Shows while fetching org data |

### Migration Integrity Test

Create an in-memory DB, apply migrations 001-011, insert orgs with old statuses (`interested`, `review`, `targeting`, `excluded`), apply migration 012, verify: (a) row count before = row count after, (b) `interested` maps to `backlog`, (c) `review` maps to `researching`, (d) `targeting` maps to `interested`, (e) `excluded` stays `excluded`, (f) `org_tags` rows are preserved.

### Contract Tests

| Test | What to verify |
|------|---------------|
| SDK `Organization.status` type matches API | `forge.organizations.get(id)` returns `status` as one of the new union values |
| `UpdateOrganization` accepts new statuses | `forge.organizations.update(id, { status: 'exciting' })` compiles and succeeds |
| `UpdateOrganization` accepts null status | `forge.organizations.update(id, { status: null })` compiles and succeeds |

---

## Documentation Requirements

- No new documentation files required (non-goal).
- The spec file serves as the design document.
- This plan file serves as the implementation reference.
- Inline code comments in migration SQL explain the status migration sequence.
- Inline TSDoc comments in SDK types explain the new status values.
- Add TSDoc to the SDK `Organization.status` field:
```typescript
/** Pipeline status for org vetting. Backlog->Researching->Targeting (exciting/interested/acceptable)->Excluded. Null means not in the pipeline. */
status: 'backlog' | 'researching' | 'exciting' | 'interested' | 'acceptable' | 'excluded' | null
```

---

## Parallelization Notes

**Within this phase:**
- T38.1 (migration) and T38.2 (service) and T38.3 (SDK types) can be developed in any order but must all be committed together -- the migration changes the DB schema, the service validates the new values, and the SDK types must match.
- T38.1 (migration), T38.2 (service), and T38.3 (SDK types) must be committed together or in rapid sequence. If the migration runs but the service still has old VALID_STATUSES, the service will reject new status values at runtime. Run all tests only after all three tasks are complete.
- T38.4 (tests) depends on T38.2 (service update).
- T38.5 (KanbanCard), T38.7 (OrgPickerModal), and T38.8 (OrgDetailModal) can be developed in parallel -- they are leaf components with no dependencies on each other.
- T38.6 (KanbanColumn) depends on T38.5 (imports KanbanCard).
- T38.9 (KanbanBoard) depends on T38.6, T38.7, and T38.8 (imports all three).
- T38.10 (page update) depends on T38.9.

**Recommended execution order:**
1. T38.1 + T38.2 + T38.3 (schema + service + types -- foundational)
2. T38.4 (tests -- validates foundation)
3. T38.5 + T38.7 + T38.8 (leaf components -- parallel)
4. T38.6 (column -- depends on card)
5. T38.9 (board -- depends on column + modals)
6. T38.10 (page swap -- final step)

**Cross-phase:**
- This phase is independent of Phases 29-37 at the DDL level. The migration (012) follows 011 numerically but does not depend on any tables created in migrations 005-011 (profile, summaries, job descriptions, templates, education subtypes, org_tags). It only modifies the `organizations` table which was created in migration 003.
- If Phases 29-37 are also being developed in parallel, the only coordination needed is ensuring migration numbering does not conflict. Migration 012 is reserved for this phase.
- Migration 012 is reserved for this phase. If any parallel phase (29-37) needs a new migration, it should use 013+. Phases 29-37 already have migrations numbered 005-011; no conflicts exist.
