# Generic Kanban Interfaces

**Date:** 2026-04-03
**Spec:** D (Generic Kanban)
**Phase:** TBD (next available)
**Builds on:** Phase 38 (Org Kanban Board -- existing `KanbanBoard`, `KanbanColumn`, `KanbanCard` components)
**Dependencies:** None (foundational reusable component)
**Blocks:** Spec E3 (JD Kanban Pipeline uses GenericKanban)

## Overview

The existing org kanban (Phase 38) hardcodes organization-specific logic into `KanbanBoard.svelte`, `KanbanColumn.svelte`, and `KanbanCard.svelte`. This spec extracts a reusable `GenericKanban` component that accepts column definitions, a card renderer snippet, items, and callbacks as props. The same component then powers four kanban views: organizations, bullets, sources (roles/projects/education), and resumes. Each page that uses a kanban also offers a toggle to switch between "Board" and "List" views, with the preference persisted in localStorage.

To make the kanban columns consistent across entity types, this spec introduces a **unified status model**. All three new entity types (bullets, sources, resumes) adopt the 5-status set: `draft | in_review | approved | rejected | archived`. This requires schema migrations to rename and expand CHECK constraints.

## Non-Goals

- Drag-and-drop card reordering within a column (no position field)
- Column reordering or custom/user-defined columns
- Batch operations (multi-select, bulk status change)
- Keyboard-based drag-and-drop (Tab/Space/Arrow card navigation)
- Swimlanes or sub-grouping within columns
- Card position persistence (cards are always sorted by a deterministic key per entity)
- Inline card editing (use detail modals)
- GenericKanban for organizations -- the org kanban has a fundamentally different status model (`backlog | researching | exciting | interested | acceptable | excluded`) that does not map to the unified 5-status set. The org kanban board (Phase 38) remains a standalone implementation. If future refactoring aligns the org board to GenericKanban, that is out of scope for this spec.
- GenericKanban for job descriptions -- JDs have an 8-status pipeline (`interested | analyzing | applied | interviewing | offered | rejected | withdrawn | closed`) that is domain-specific. Spec E3 handles the JD kanban as a separate concern, but it may optionally use GenericKanban if column grouping maps well.
- Modifying the org kanban implementation (Phase 38 components remain untouched)
- Mobile/responsive kanban layout
- Real-time sync or multi-user support

---

## 1. Unified Status Model

### 1.1 Target Statuses

All three entity types converge on a single status enum for kanban column mapping:

```
draft | in_review | approved | rejected | archived
```

### 1.2 Current vs. Target Per Entity

| Entity | Current Statuses | Changes Required |
|--------|-----------------|-----------------|
| Bullets | `draft`, `pending_review`, `approved`, `rejected` | Rename `pending_review` -> `in_review`; add `archived` |
| Sources | `draft`, `approved`, `deriving` | Add `in_review`, `rejected`, `archived`; keep `deriving` as a transient lock status (not a kanban column) |
| Resumes | `draft`, `final` | Replace with `draft`, `in_review`, `approved`, `rejected`, `archived`; migrate `final` -> `approved` |
| Perspectives | `draft`, `pending_review`, `approved`, `rejected` | Rename `pending_review` -> `in_review`; add `archived` (same as bullets) |

### 1.3 Sources: `deriving` as Transient Status

The `deriving` status is a machine lock that prevents concurrent AI derivation. It is NOT a kanban column. Sources in `deriving` state appear in whichever column they were in before the lock was acquired (typically `approved`). The kanban treats `deriving` as equivalent to `approved` for column placement:

```typescript
// In the sources column config, `approved` column includes `deriving`
{ key: 'approved', label: 'Approved', statuses: ['approved', 'deriving'], accent: '#22c55e' }
```

Cards in `deriving` status should not be draggable — show a lock/spinner cursor. The `onDrop` handler should reject transitions from `deriving`.

### 1.4 Status Transition Rules (Updated)

**Bullets:**
- `draft` -> `in_review`, `approved`
- `in_review` -> `approved`, `rejected`
- `approved` -> `archived`
- `rejected` -> `in_review` (reopen)
- `archived` -> `draft` (unarchive)

**Sources:**
- `draft` -> `in_review`, `approved`
- `in_review` -> `approved`, `rejected`
- `approved` -> `deriving` (machine lock), `archived`
- `rejected` -> `draft` (reopen)
- `archived` -> `draft` (unarchive)
- `deriving` -> `approved` (completion), previous status (failure)

**Resumes:**
- `draft` -> `in_review`
- `in_review` -> `approved`, `rejected`
- `approved` -> `archived`
- `rejected` -> `draft` (reopen)
- `archived` -> `draft` (unarchive)

**Perspectives:**
- Same as bullets.

> **Note:** Sources use `rejected → draft` (human-curated content goes back to editing). Bullets/perspectives use `rejected → in_review` (AI-generated content goes back for re-review). This asymmetry is intentional.

### 1.5 Impact on Existing Rules

The existing derivation chain rules (from `refs/taxonomy/statuses.md`) remain enforced:
- Only `approved` sources can have bullets derived (DerivationService)
- Only `approved` bullets can have perspectives derived (DerivationService)
- Only `approved` perspectives can be added to resumes (ResumeService)
- `rejected` -> `approved` is still NOT allowed (must go through `in_review` first)

The `archived` status is a soft-delete. Archived entities:
- Cannot be used as derivation inputs
- Cannot be added to resumes
- Are excluded from the review queue
- Are visible on the kanban in a collapsed "Archived" column
- Can be unarchived (returns to `draft`)

---

## 2. GenericKanban Component Interface

### 2.1 Component: `GenericKanban.svelte`

> **Note:** Use `<script lang='ts' generics='T extends { id: string }'>` on the GenericKanban component for Svelte 5 generic support.

```typescript
// Props interface
interface GenericKanbanProps<T extends { id: string }> {
  /** Column definitions: key, label, which statuses map to this column, accent color */
  columns: ColumnDef[]
  /** All items to display on the board */
  items: T[]
  /** Called when an item is dropped into a new column. Returns the new status string. */
  onDrop: (itemId: string, newStatus: string) => Promise<void>
  /** Whether the board is in a loading state */
  loading?: boolean
  /** Message shown when loading */
  loadingMessage?: string
  /** Message shown when no items exist */
  emptyMessage?: string
  /** Filter bar content -- rendered above the board via a Svelte snippet */
  filterBar?: Snippet
  /** Card content renderer -- receives the item and renders card body via a Svelte snippet */
  cardContent: Snippet<[T]>
  /** Optional: which column key should be collapsed by default */
  defaultCollapsed?: string
  /** Sort function for items within a column. Defaults to alphabetical by first string field. */
  sortItems?: (a: T, b: T) => number
}

interface ColumnDef {
  /** Unique key for this column (used as drop target identifier) */
  key: string
  /** Display label */
  label: string
  /** Status values that map to this column */
  statuses: string[]
  /** Default status to set when dropping into this column (first entry in statuses[] if omitted) */
  dropStatus?: string
  /** Accent color (CSS color value) for column header border */
  accent: string
}
```

### 2.2 Usage Pattern

```svelte
<GenericKanban
  columns={BULLET_COLUMNS}
  items={bullets}
  onDrop={handleBulletDrop}
  loading={isLoading}
  emptyMessage="No bullets yet. Create sources and derive bullets to populate this board."
  defaultCollapsed="archived"
  sortItems={(a, b) => a.content.localeCompare(b.content)}
>
  {#snippet filterBar()}
    <BulletFilterBar bind:filters onchange={applyFilters} />
  {/snippet}

  {#snippet cardContent(bullet)}
    <BulletKanbanCard {bullet} onclick={() => openDetail(bullet.id)} />
  {/snippet}
</GenericKanban>
```

### 2.3 Drag-and-Drop

- Uses `svelte-dnd-action` with Svelte 5 syntax (`onconsider`/`onfinalize`)
- Card identity uses item's `id` field (required by `svelte-dnd-action`)
- **Optimistic UI:** Local state updates immediately on drop. If the API call (via `onDrop`) throws or rejects, the component reverts local state and shows an error toast
- **Cross-column drops only:** Intra-column reorder is a no-op (no position field in schema). Cards snap back to sorted order on next render.
- `cursor: grab` on hover, `cursor: grabbing` while dragging
- Drop target highlight uses column accent color: `outline: 2px dashed ${accent}`

### 2.4 Internal State Management

GenericKanban maintains a local copy of items per column (same pattern as the existing `KanbanColumn.svelte`). When props change (e.g., after filter application), the local state syncs via `$effect`.

```typescript
let columnState = $state<Map<string, T[]>>(new Map())

$effect(() => {
  const grouped = new Map<string, T[]>()
  for (const col of columns) {
    const colItems = items
      .filter(item => col.statuses.includes(getStatus(item)))
      .sort(sortItems)
    grouped.set(col.key, colItems)
  }
  columnState = grouped
})
```

The component derives the status field from items using a convention: it reads `item.status`. All entity types in Forge have a `status` field at the top level.

---

## 3. Column Definitions Per Entity

### 3.1 Bullets Kanban

```typescript
const BULLET_COLUMNS: ColumnDef[] = [
  { key: 'draft', label: 'Draft', statuses: ['draft'], accent: '#a5b4fc' },
  { key: 'in_review', label: 'In Review', statuses: ['in_review'], accent: '#fbbf24' },
  { key: 'approved', label: 'Approved', statuses: ['approved'], accent: '#22c55e' },
  { key: 'rejected', label: 'Rejected', statuses: ['rejected'], accent: '#ef4444' },
  { key: 'archived', label: 'Archived', statuses: ['archived'], accent: '#d1d5db' },
]
```

**Card content:** Bullet content (first ~80 chars), domain badge, technology pills, source title (muted). Rejected cards show `opacity: 0.7`. Archived cards show `opacity: 0.5` with muted text.

**Default sort:** Alphabetical by `content`.

**Default collapsed:** `archived`.

### 3.2 Sources Kanban

```typescript
const SOURCE_COLUMNS: ColumnDef[] = [
  { key: 'draft', label: 'Draft', statuses: ['draft'], accent: '#a5b4fc' },
  { key: 'in_review', label: 'In Review', statuses: ['in_review'], accent: '#fbbf24' },
  { key: 'approved', label: 'Approved', statuses: ['approved', 'deriving'], dropStatus: 'approved', accent: '#22c55e' },
  { key: 'rejected', label: 'Rejected', statuses: ['rejected'], accent: '#ef4444' },
  { key: 'archived', label: 'Archived', statuses: ['archived'], accent: '#d1d5db' },
]
```

**Card content:** Source title, source_type badge (role/project/education/clearance/general), organization name (if linked), date range (start--end), bullet count badge. Deriving sources show a spinner icon overlay.

**Default sort:** By `source_type` (role first, then project, education, clearance, general), then alphabetical by `title`.

**Default collapsed:** `archived`.

### 3.3 Resumes Kanban

```typescript
const RESUME_COLUMNS: ColumnDef[] = [
  { key: 'draft', label: 'Draft', statuses: ['draft'], accent: '#a5b4fc' },
  { key: 'in_review', label: 'In Review', statuses: ['in_review'], accent: '#fbbf24' },
  { key: 'approved', label: 'Approved', statuses: ['approved'], accent: '#22c55e' },
  { key: 'rejected', label: 'Rejected', statuses: ['rejected'], accent: '#ef4444' },
  { key: 'archived', label: 'Archived', statuses: ['archived'], accent: '#d1d5db' },
]
```

**Card content:** Resume name, target_role, target_employer, archetype badge, section count. Approved resumes show a green check icon.

**Default sort:** Alphabetical by `name`.

**Default collapsed:** `archived`.

### 3.4 Perspectives Kanban

Perspectives share the same column definitions as bullets. Use `BULLET_COLUMNS` (or define a shared `UNIFIED_COLUMNS` constant).

**Card content:** Perspective content (first ~80 chars), archetype badge, domain badge, framing badge. Same opacity rules as bullets.

**Default sort:** By `target_archetype`, then by `content`.

**Default collapsed:** `archived`.

---

## 4. Schema Migrations

### 4.1 Migration: `017_unified_kanban_statuses.sql`

This migration expands CHECK constraints on `bullets`, `sources`, `resumes`, and `perspectives` tables. Since SQLite cannot ALTER CHECK constraints, each table uses the table-rebuild pattern (CREATE new -> INSERT INTO new FROM old -> DROP old -> RENAME new).

```sql
-- Unified Kanban Statuses
-- Migration: 017_unified_kanban_statuses
-- Expands status CHECK constraints for bullets, sources, resumes, perspectives.
-- Renames pending_review -> in_review in bullets and perspectives.
-- Replaces final -> approved in resumes.
-- Adds in_review, rejected, archived to sources.
-- Adds archived to bullets and perspectives.

PRAGMA foreign_keys = OFF;

-- NOTE: Verify whether `source_id` still exists on the `bullets` table before
-- the rebuild. If it does, include it in `bullets_new` CREATE TABLE and the
-- INSERT SELECT. If `bullet_sources` junction has fully replaced it, omit it.
-- Run `PRAGMA table_info(bullets)` to check.

-- ── Bullets ──────────────────────────────────────────────────────────

CREATE TABLE bullets_new (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  content TEXT NOT NULL,
  source_content_snapshot TEXT NOT NULL,
  metrics TEXT,
  status TEXT NOT NULL DEFAULT 'in_review' CHECK(status IN (
    'draft', 'in_review', 'approved', 'rejected', 'archived'
  )),
  rejection_reason TEXT,
  prompt_log_id TEXT REFERENCES prompt_logs(id) ON DELETE SET NULL,
  approved_at TEXT,
  approved_by TEXT,
  notes TEXT,
  domain TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

INSERT INTO bullets_new (id, content, source_content_snapshot, metrics, status,
  rejection_reason, prompt_log_id, approved_at, approved_by, notes, domain, created_at)
SELECT id, content, source_content_snapshot, metrics,
  CASE status
    WHEN 'pending_review' THEN 'in_review'
    ELSE status
  END,
  rejection_reason, prompt_log_id, approved_at, approved_by, notes, domain, created_at
FROM bullets;

DROP TABLE bullets;
ALTER TABLE bullets_new RENAME TO bullets;

DROP INDEX IF EXISTS idx_bullets_status;
CREATE INDEX idx_bullets_status ON bullets(status);

-- ── Perspectives ─────────────────────────────────────────────────────

CREATE TABLE perspectives_new (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  bullet_id TEXT NOT NULL REFERENCES bullets(id) ON DELETE RESTRICT,
  content TEXT NOT NULL,
  bullet_content_snapshot TEXT NOT NULL,
  target_archetype TEXT,
  domain TEXT,
  framing TEXT NOT NULL CHECK(framing IN ('accomplishment', 'responsibility', 'context')),
  status TEXT NOT NULL DEFAULT 'in_review' CHECK(status IN (
    'draft', 'in_review', 'approved', 'rejected', 'archived'
  )),
  rejection_reason TEXT,
  prompt_log_id TEXT REFERENCES prompt_logs(id) ON DELETE SET NULL,
  approved_at TEXT,
  approved_by TEXT,
  notes TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

INSERT INTO perspectives_new (id, bullet_id, content, bullet_content_snapshot,
  target_archetype, domain, framing, status, rejection_reason, prompt_log_id,
  approved_at, approved_by, notes, created_at)
SELECT id, bullet_id, content, bullet_content_snapshot,
  target_archetype, domain, framing,
  CASE status
    WHEN 'pending_review' THEN 'in_review'
    ELSE status
  END,
  rejection_reason, prompt_log_id, approved_at, approved_by, notes, created_at
FROM perspectives;

DROP TABLE perspectives;
ALTER TABLE perspectives_new RENAME TO perspectives;

DROP INDEX IF EXISTS idx_perspectives_bullet;
CREATE INDEX idx_perspectives_bullet ON perspectives(bullet_id);
DROP INDEX IF EXISTS idx_perspectives_status;
CREATE INDEX idx_perspectives_status ON perspectives(status);
DROP INDEX IF EXISTS idx_perspectives_archetype;
CREATE INDEX idx_perspectives_archetype ON perspectives(target_archetype);
DROP INDEX IF EXISTS idx_perspectives_domain;
CREATE INDEX idx_perspectives_domain ON perspectives(domain);

-- ── Sources ──────────────────────────────────────────────────────────

CREATE TABLE sources_new (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  title TEXT NOT NULL,
  description TEXT NOT NULL,
  source_type TEXT NOT NULL DEFAULT 'general'
    CHECK (source_type IN ('role', 'project', 'education', 'clearance', 'general')),
  start_date TEXT,
  end_date TEXT,
  status TEXT NOT NULL DEFAULT 'draft' CHECK(status IN (
    'draft', 'in_review', 'approved', 'rejected', 'archived', 'deriving'
  )),
  updated_by TEXT NOT NULL DEFAULT 'human' CHECK(updated_by IN ('human', 'ai')),
  last_derived_at TEXT,
  notes TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

INSERT INTO sources_new (id, title, description, source_type, start_date, end_date,
  status, updated_by, last_derived_at, notes, created_at, updated_at)
SELECT id, title, description, source_type, start_date, end_date,
  status, updated_by, last_derived_at, notes, created_at, updated_at
FROM sources;

DROP TABLE sources;
ALTER TABLE sources_new RENAME TO sources;

DROP INDEX IF EXISTS idx_sources_status;
CREATE INDEX idx_sources_status ON sources(status);
DROP INDEX IF EXISTS idx_sources_source_type;
CREATE INDEX idx_sources_source_type ON sources(source_type);

-- ── Resumes ──────────────────────────────────────────────────────────

CREATE TABLE resumes_new (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  name TEXT NOT NULL,
  target_role TEXT NOT NULL,
  target_employer TEXT NOT NULL,
  archetype TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'draft' CHECK(status IN (
    'draft', 'in_review', 'approved', 'rejected', 'archived'
  )),
  notes TEXT,
  header TEXT,
  summary_id TEXT,
  markdown_override TEXT,
  markdown_override_updated_at TEXT,
  latex_override TEXT,
  latex_override_updated_at TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

INSERT INTO resumes_new (id, name, target_role, target_employer, archetype, status,
  notes, header, summary_id, markdown_override, markdown_override_updated_at,
  latex_override, latex_override_updated_at, created_at, updated_at)
SELECT id, name, target_role, target_employer, archetype,
  CASE status
    WHEN 'final' THEN 'approved'
    ELSE status
  END,
  notes, header, summary_id, markdown_override, markdown_override_updated_at,
  latex_override, latex_override_updated_at, created_at, updated_at
FROM resumes;

DROP TABLE resumes;
ALTER TABLE resumes_new RENAME TO resumes;

-- ── Rebuild junction tables referencing rebuilt tables ────────────────

-- bullet_sources: references bullets(id). Since we dropped and recreated
-- bullets, we need to verify FKs still hold. With PRAGMA foreign_keys OFF,
-- existing junction rows still reference valid IDs (same UUIDs preserved).

-- bullet_technologies: references bullets(id) ON DELETE CASCADE.
-- Same IDs preserved; no action needed.

-- bullet_skills: references bullets(id) ON DELETE CASCADE.
-- Same IDs preserved; no action needed.

-- perspective_skills: references perspectives(id) ON DELETE CASCADE.
-- Same IDs preserved; no action needed.

-- resume_perspectives: references resumes(id) ON DELETE CASCADE,
-- perspectives(id) ON DELETE RESTRICT. Same IDs preserved; no action needed.

-- resume_sections: references resumes(id) ON DELETE CASCADE.
-- Same IDs preserved; no action needed.

-- source_roles, source_projects, source_education, source_clearances:
-- reference sources(id) ON DELETE CASCADE. Same IDs preserved; no action needed.

-- source_skills: references sources(id) ON DELETE CASCADE.
-- Same IDs preserved; no action needed.

PRAGMA foreign_keys = ON;

INSERT INTO _migrations (name) VALUES ('017_unified_kanban_statuses');
```

### 4.2 Data Migration Notes

- **Bullets/Perspectives:** `pending_review` -> `in_review`. No data loss.
- **Resumes:** `final` -> `approved`. Semantically equivalent -- a "final" resume is an approved one.
- **Sources:** No data remapping. Existing `draft`, `approved`, `deriving` values are all valid in the new constraint. The new statuses (`in_review`, `rejected`, `archived`) have no existing data.

### 4.3 Junction Table Safety

All four table rebuilds preserve IDs. With `PRAGMA foreign_keys = OFF`, the DROP + RENAME cycle does not trigger cascade deletes. Junction tables (`bullet_sources`, `bullet_technologies`, `bullet_skills`, `perspective_skills`, `resume_perspectives`, `resume_sections`, `resume_skills`, `source_roles`, `source_projects`, `source_education`, `source_clearances`, `source_skills`) continue to reference valid IDs after the rebuild. This is the same pattern used in migrations 002, 007, and 012.

---

## 5. View Toggle Design

### 5.1 Toggle Button

Each page that uses GenericKanban includes a view toggle in the page header:

```svelte
<div class="page-header">
  <h2>Bullets</h2>
  <div class="view-toggle">
    <button
      class:active={viewMode === 'list'}
      onclick={() => setViewMode('list')}
    >List</button>
    <button
      class:active={viewMode === 'board'}
      onclick={() => setViewMode('board')}
    >Board</button>
  </div>
</div>
```

### 5.2 Persistence

View mode preference is stored in localStorage per entity:

```typescript
const STORAGE_KEY_PREFIX = 'forge:viewMode:'

function getViewMode(entity: string): 'list' | 'board' {
  try {
    const stored = localStorage.getItem(`${STORAGE_KEY_PREFIX}${entity}`)
    return stored === 'board' ? 'board' : 'list' // default to list
  } catch {
    return 'list'
  }
}

function setViewMode(entity: string, mode: 'list' | 'board') {
  try {
    localStorage.setItem(`${STORAGE_KEY_PREFIX}${entity}`, mode)
  } catch {
    // localStorage unavailable, silently ignore
  }
}
```

Keys: `forge:viewMode:bullets`, `forge:viewMode:sources`, `forge:viewMode:resumes`, `forge:viewMode:perspectives`.

### 5.3 Default View

All entities default to `list` view. The list view is the existing split-panel or table layout already built for each entity. The board view is the new GenericKanban. The user can switch at will; the toggle updates a reactive `$state` variable that conditionally renders one or the other.

### 5.4 View Toggle Component

Extract a reusable `ViewToggle.svelte` component:

```typescript
// Props
let { mode, onchange }: {
  mode: 'list' | 'board'
  onchange: (mode: 'list' | 'board') => void
} = $props()
```

Styling: segmented control (two buttons joined), active button has filled background (`var(--color-primary)`), inactive is outlined. Matches the existing `btn-add` color scheme.

---

## 6. Filter Bar Design

The filter bar is rendered above the kanban board (or list view -- filters apply to both). Each entity type defines its own filter bar component. The filter bar is passed to GenericKanban via the `filterBar` snippet prop.

### 6.1 Bullet Filters

| Filter | Type | Source |
|--------|------|--------|
| Source | Dropdown (source title) | `forge.sources.list()` |
| Domain | Dropdown (domain name) | `forge.domains.list()` |
| Skill | Dropdown (skill name) | `forge.skills.list()` |
| Search | Text input (content search) | Client-side filter on `content` |

**Component:** `BulletFilterBar.svelte`

### 6.2 Source Filters

| Filter | Type | Source |
|--------|------|--------|
| Organization | Dropdown (org name) | `forge.organizations.list()` |
| Source Type | Dropdown (`role | project | education | clearance | general`) | Static enum |
| Skill | Dropdown (skill name) | `forge.skills.list()` |
| Search | Text input (title search) | Client-side filter on `title` |

**Component:** `SourceFilterBar.svelte`

### 6.3 Resume Filters

| Filter | Type | Source |
|--------|------|--------|
| Archetype | Dropdown (archetype name) | `forge.archetypes.list()` |
| Target Employer | Dropdown (org name) | `forge.organizations.list()` -- filtered to orgs appearing in `resumes.target_employer` |
| Search | Text input (name/role search) | Client-side filter on `name` and `target_role` |

**Component:** `ResumeFilterBar.svelte`

### 6.4 Perspective Filters

| Filter | Type | Source |
|--------|------|--------|
| Archetype | Dropdown (archetype name) | `forge.archetypes.list()` |
| Domain | Dropdown (domain name) | `forge.domains.list()` |
| Framing | Dropdown (`accomplishment | responsibility | context`) | Static enum |
| Search | Text input (content search) | Client-side filter on `content` |

**Component:** `PerspectiveFilterBar.svelte`

### 6.5 Filtering Strategy

Filters are applied **client-side** after fetching all items. This is consistent with the existing org kanban pattern (fetch all with `limit: 500`, filter in the browser). For entities with potentially large datasets (bullets, perspectives), consider adding server-side filtering in a future iteration -- but for a single-user tool with ~100-500 items per entity, client-side is sufficient.

Filters are combined with AND logic. Clearing a filter removes it from the active set.

---

## 7. Files to Create

| File | Purpose |
|------|---------|
| `packages/core/src/db/migrations/017_unified_kanban_statuses.sql` | Table rebuild migration for all 4 entity status changes |
| `packages/webui/src/lib/components/kanban/GenericKanban.svelte` | Reusable kanban board component |
| `packages/webui/src/lib/components/kanban/GenericKanbanColumn.svelte` | Reusable kanban column. Accepts `column: ColumnDef`, `items: T[]`, `cardContent: Snippet<[T]>`, `onDrop: (itemId: string) => void` |
| `packages/webui/src/lib/components/ViewToggle.svelte` | Reusable list/board toggle button |
| `packages/webui/src/lib/components/kanban/BulletKanbanCard.svelte` | Bullet card content for kanban |
| `packages/webui/src/lib/components/kanban/SourceKanbanCard.svelte` | Source card content for kanban |
| `packages/webui/src/lib/components/kanban/ResumeKanbanCard.svelte` | Resume card content for kanban |
| `packages/webui/src/lib/components/kanban/PerspectiveKanbanCard.svelte` | Perspective card content for kanban |
| `packages/webui/src/lib/components/filters/BulletFilterBar.svelte` | Bullet filter controls |
| `packages/webui/src/lib/components/filters/SourceFilterBar.svelte` | Source filter controls |
| `packages/webui/src/lib/components/filters/ResumeFilterBar.svelte` | Resume filter controls |
| `packages/webui/src/lib/components/filters/PerspectiveFilterBar.svelte` | Perspective filter controls |
| `packages/webui/src/lib/stores/viewMode.svelte.ts` | localStorage-backed view mode store |

## 8. Files to Modify

| File | Change |
|------|--------|
| `packages/core/src/types/index.ts` | Update `BulletStatus`, `SourceStatus`, `ResumeStatus`, `PerspectiveStatus` union types; add `UnifiedKanbanStatus` type; add `archived` to all four |
| `packages/sdk/src/types.ts` | Mirror status type changes from core |
| `packages/core/src/db/repositories/bullet-repository.ts` | Accept `in_review` and `archived`; reject `pending_review` |
| `packages/core/src/db/repositories/source-repository.ts` | Accept `in_review`, `rejected`, `archived` |
| `packages/core/src/db/repositories/resume-repository.ts` | Accept `in_review`, `approved`, `rejected`, `archived`; reject `final` |
| `packages/core/src/db/repositories/perspective-repository.ts` | Accept `in_review` and `archived`; reject `pending_review` |
| `packages/core/src/services/derivation-service.ts` | Update status checks: `approved` sources/bullets still required; reject `archived` as derivation input |
| `packages/core/src/services/resume-service.ts` | Update status checks: reject `archived` perspectives from being added to resumes |
| `packages/core/src/routes/bullets.ts` | Update status validation in PATCH handler |
| `packages/core/src/routes/sources.ts` | Update status validation in PATCH handler |
| `packages/core/src/routes/resumes.ts` | Update status validation in PATCH handler |
| `packages/core/src/routes/perspectives.ts` | Update status validation in PATCH handler |
| `packages/webui/src/routes/data/bullets/+page.svelte` | Add view toggle, integrate GenericKanban in board mode |
| `packages/webui/src/routes/data/sources/+page.ts` (or `SourcesView.svelte`) | Add view toggle, integrate GenericKanban in board mode |
| `packages/webui/src/routes/resumes/+page.svelte` | Add view toggle, integrate GenericKanban in board mode |
| `packages/webui/src/routes/data/perspectives/+page.svelte` | Add view toggle, integrate GenericKanban in board mode for perspectives |
| `packages/webui/src/routes/+page.svelte` (review queue) | Update pending count labels from `pending_review` to `in_review` |
| `packages/core/src/constants/index.ts` | Add `UNIFIED_KANBAN_STATUSES` constant; update per-entity status arrays |
| `packages/webui/src/lib/components/StatusBadge.svelte` | Add styling for `in_review` and `archived` statuses. Note: these StatusBadge entries are added in this spec's implementation, not Spec A. Spec A only tokenizes existing entries. |

---

## 9. Testing

### 9.1 Migration Tests

- Apply migration 017 on a database with existing data
- Verify bullets with `pending_review` are migrated to `in_review`
- Verify perspectives with `pending_review` are migrated to `in_review`
- Verify resumes with `final` are migrated to `approved`
- Verify sources retain their original statuses (`draft`, `approved`, `deriving`)
- Verify all junction tables remain intact after the 4-table rebuild
- Verify new status values (`archived`, `in_review` for sources, `rejected` for sources) can be inserted

### 9.2 Repository/Service Tests

- Bullet repository: `in_review` accepted, `pending_review` rejected, `archived` accepted
- Source repository: `in_review` accepted, `rejected` accepted, `archived` accepted
- Resume repository: `in_review` accepted, `approved` accepted, `rejected` accepted, `archived` accepted, `final` rejected
- Perspective repository: `in_review` accepted, `pending_review` rejected, `archived` accepted
- DerivationService: rejects `archived` sources as derivation input
- DerivationService: rejects `archived` bullets as derivation input
- ResumeService: rejects `archived` perspectives from being added to resumes

### 9.3 API Tests

- `PATCH /api/bullets/:id { status: 'in_review' }` succeeds
- `PATCH /api/bullets/:id { status: 'pending_review' }` returns 400
- `PATCH /api/bullets/:id { status: 'archived' }` succeeds
- `PATCH /api/sources/:id { status: 'in_review' }` succeeds
- `PATCH /api/sources/:id { status: 'archived' }` succeeds
- `PATCH /api/resumes/:id { status: 'approved' }` succeeds
- `PATCH /api/resumes/:id { status: 'final' }` returns 400

### 9.4 Component Smoke Tests

- GenericKanban renders 5 columns with correct labels
- Cards appear in correct columns based on status
- Archived column is collapsed by default
- Clicking collapsed Archived expands it
- Dropping a card into a new column calls `onDrop` with correct status
- Optimistic update: card moves immediately; reverts on API failure
- Filter bar renders above the board
- Filters reduce visible cards (client-side)
- ViewToggle renders two buttons, active state matches current mode
- Clicking ViewToggle updates localStorage and swaps the view
- Empty board shows `emptyMessage`
- Loading state shows spinner

### 9.5 Integration Tests (Per Entity Page)

- Bullets page: toggle to Board, verify GenericKanban renders with bullet columns
- Bullets page: drag bullet from Draft to In Review, verify API call
- Sources page: toggle to Board, verify sources grouped correctly (deriving items in Approved column)
- Resumes page: toggle to Board, verify all 5 columns present
- View mode persists across page navigation (leave bullets, come back, same mode)

---

## 10. Acceptance Criteria

1. `GenericKanban.svelte` is a reusable component that accepts column definitions, items, card renderer snippet, and drop callback as props
2. Migration 017 expands status CHECK constraints for bullets, perspectives, sources, and resumes; renames `pending_review` -> `in_review`; migrates `final` -> `approved`
3. All four entity types use the unified status set: `draft | in_review | approved | rejected | archived` (sources additionally keep `deriving`)
4. `/data/bullets`, `/data/sources` (or `SourcesView`), and `/resumes` pages each offer a List/Board view toggle
5. Board view renders GenericKanban with entity-appropriate columns, card content, and filters
6. List view renders the existing split-panel/table layout (no regression)
7. View mode preference persists in localStorage per entity
8. Archived column is collapsed by default on all boards; click to expand
9. Drag-and-drop between columns updates entity status via API with optimistic UI and revert on failure
10. Filter bar above the board filters visible cards client-side
11. `StatusBadge.svelte` renders correct styling for `in_review` and `archived` statuses
12. Existing org kanban (Phase 38) continues to work unchanged
13. Derivation chain rules enforced: archived entities cannot be used as derivation inputs or added to resumes
14. Review queue dashboard reflects renamed status (`in_review` instead of `pending_review`)
