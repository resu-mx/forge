# Phase 43: Generic Kanban Interfaces

**Status:** Planning
**Date:** 2026-04-03
**Spec:** [2026-04-03-generic-kanban.md](../refs/specs/2026-04-03-generic-kanban.md)
**Depends on:** None (foundational, but benefits from Phase 42 design tokens if available)
**Blocks:** Spec E3 (JD Kanban Pipeline uses GenericKanban)
**Parallelizable with:** Phase 42 (Design System) at the code level; Phase 38 (Org Kanban) must be complete first since this phase builds on the patterns established there

## Goal

Extract a reusable `GenericKanban` component from the Phase 38 org kanban pattern, then use it to build kanban boards for four entity types: bullets, sources, resumes, and perspectives. Introduce a unified 5-status model (`draft | in_review | approved | rejected | archived`) across all four entities via a schema migration that renames `pending_review` to `in_review`, replaces `final` with `approved`, and adds missing statuses. Each entity page gets a list/board view toggle persisted in localStorage. The org kanban (Phase 38) remains a standalone implementation -- its 6-status model does not map to the unified set.

## Non-Goals

- Drag-and-drop card reordering within a column (no position field).
- Column reordering or custom/user-defined columns.
- Batch operations (multi-select, bulk status change).
- Keyboard-based drag-and-drop (Tab/Space/Arrow card navigation).
- Swimlanes or sub-grouping within columns.
- Card position persistence (cards always sorted by a deterministic key per entity).
- Inline card editing (use detail modals).
- GenericKanban for organizations (different 6-status model; Phase 38 standalone).
- GenericKanban for job descriptions (8-status pipeline; Spec E3 handles separately).
- Modifying the Phase 38 org kanban implementation.
- Mobile/responsive kanban layout.
- Real-time sync or multi-user support.

## Context

The Phase 38 org kanban hardcodes organization-specific logic into `KanbanBoard.svelte`, `KanbanColumn.svelte`, and `KanbanCard.svelte`. These components work well for organizations but cannot be reused for other entity types without significant modification. The four entity types (bullets, sources, resumes, perspectives) each have different status sets today:

- **Bullets:** `draft`, `pending_review`, `approved`, `rejected`
- **Sources:** `draft`, `approved`, `deriving`
- **Resumes:** `draft`, `final`
- **Perspectives:** `draft`, `pending_review`, `approved`, `rejected`

This spec converges them on a unified status set while preserving domain-specific semantics (e.g., sources keep `deriving` as a transient lock, mapped to the `approved` column).

The migration (017) uses the table-rebuild pattern consistent with migrations 002, 007, and 012. The GenericKanban component uses Svelte 5 generics (`<script lang='ts' generics='T extends { id: string }'>`) and `svelte-dnd-action` for drag-and-drop, matching the patterns already proven in the org kanban.

## Scope

| Spec section | Covered here? |
|-------------|---------------|
| 1. Unified Status Model (statuses, transitions, impact on rules) | Yes |
| 2. GenericKanban Component Interface (props, usage, drag-and-drop, state management) | Yes |
| 3. Column Definitions Per Entity (bullets, sources, resumes, perspectives) | Yes |
| 4. Schema Migrations (017_unified_kanban_statuses.sql) | Yes |
| 5. View Toggle Design (toggle button, persistence, component) | Yes |
| 6. Filter Bar Design (per-entity filters, strategy) | Yes |
| 7. Files to Create | Yes |
| 8. Files to Modify | Yes |
| 9. Testing | Yes |
| 10. Acceptance Criteria | Yes |

## Files to Create

| File | Description |
|------|-------------|
| `packages/core/src/db/migrations/017_unified_kanban_statuses.sql` | Table rebuild migration: expand/rename status CHECK constraints for bullets, sources, resumes, perspectives |
| `packages/webui/src/lib/components/kanban/GenericKanban.svelte` | Reusable kanban board component with Svelte 5 generics, column definitions, card renderer snippet, drag-and-drop |
| `packages/webui/src/lib/components/kanban/GenericKanbanColumn.svelte` | Reusable kanban column with `svelte-dnd-action` drop zone, collapse support |
| `packages/webui/src/lib/components/ViewToggle.svelte` | Reusable list/board segmented toggle button |
| `packages/webui/src/lib/components/kanban/BulletKanbanCard.svelte` | Bullet card content for kanban (content preview, domain badge, tech pills, source title) |
| `packages/webui/src/lib/components/kanban/SourceKanbanCard.svelte` | Source card content for kanban (title, type badge, org name, date range, bullet count) |
| `packages/webui/src/lib/components/kanban/ResumeKanbanCard.svelte` | Resume card content for kanban (name, target role/employer, archetype badge, section count) |
| `packages/webui/src/lib/components/kanban/PerspectiveKanbanCard.svelte` | Perspective card content for kanban (content preview, archetype/domain/framing badges) |
| `packages/webui/src/lib/components/filters/BulletFilterBar.svelte` | Bullet filter controls (source, domain, skill, search) |
| `packages/webui/src/lib/components/filters/SourceFilterBar.svelte` | Source filter controls (org, source type, skill, search) |
| `packages/webui/src/lib/components/filters/ResumeFilterBar.svelte` | Resume filter controls (archetype, target employer, search) |
| `packages/webui/src/lib/components/filters/PerspectiveFilterBar.svelte` | Perspective filter controls (archetype, domain, framing, search) |
| `packages/webui/src/lib/stores/viewMode.svelte.ts` | localStorage-backed view mode store (list/board per entity) |

## Files to Modify

| File | Change |
|------|--------|
| `packages/core/src/types/index.ts` | Update `BulletStatus`, `SourceStatus`, `ResumeStatus`, `PerspectiveStatus` union types; add `UnifiedKanbanStatus` type; add `archived` to all four |
| `packages/sdk/src/types.ts` | Mirror status type changes from core |
| `packages/core/src/db/repositories/bullet-repository.ts` | Accept `in_review` and `archived`; reject `pending_review` |
| `packages/core/src/db/repositories/source-repository.ts` | Accept `in_review`, `rejected`, `archived` |
| `packages/core/src/db/repositories/resume-repository.ts` | Accept `in_review`, `approved`, `rejected`, `archived`; reject `final` |
| `packages/core/src/db/repositories/perspective-repository.ts` | Accept `in_review` and `archived`; reject `pending_review` |
| `packages/core/src/services/derivation-service.ts` | Update status checks: reject `archived` as derivation input |
| `packages/core/src/services/resume-service.ts` | Update status checks: reject `archived` perspectives from being added to resumes |
| `packages/core/src/routes/bullets.ts` | Update status validation in PATCH handler |
| `packages/core/src/routes/sources.ts` | Update status validation in PATCH handler |
| `packages/core/src/routes/resumes.ts` | Update status validation in PATCH handler |
| `packages/core/src/routes/perspectives.ts` | Update status validation in PATCH handler |
| `packages/webui/src/routes/data/bullets/+page.svelte` | Add view toggle, integrate GenericKanban in board mode |
| `packages/webui/src/routes/data/sources/+page.svelte` (or `SourcesView.svelte`) | Add view toggle, integrate GenericKanban in board mode |
| `packages/webui/src/routes/resumes/+page.svelte` | Add view toggle, integrate GenericKanban in board mode |
| `packages/webui/src/routes/data/perspectives/+page.svelte` | Add view toggle, integrate GenericKanban in board mode |
| `packages/webui/src/routes/+page.svelte` (review queue) | Update pending count labels from `pending_review` to `in_review` |
| `packages/core/src/constants/index.ts` | Add `UNIFIED_KANBAN_STATUSES` constant; update per-entity status arrays |
| `packages/webui/src/lib/components/StatusBadge.svelte` | Add styling for `in_review` and `archived` statuses |

## Fallback Strategies

- **`svelte-dnd-action` Svelte 5 compatibility:** The project uses `svelte-dnd-action` v0.9.69 with `onconsider`/`onfinalize` event handlers, proven working in `DragNDropView.svelte` and the Phase 38 kanban. If issues arise with generic components, fall back to `use:dndzone` with Svelte 4 event directive syntax.
- **Svelte 5 generics support:** `<script lang='ts' generics='T extends { id: string }'>` is Svelte 5 syntax. If the project's Svelte version does not support this, fall back to using `any` with runtime type assertions and JSDoc type comments.
- **PRAGMA foreign_keys in transactions:** The migration runner wraps each migration in `BEGIN`/`COMMIT`. SQLite silently ignores `PRAGMA foreign_keys = OFF` inside an active transaction. The PRAGMA calls in the migration SQL are defensive only. This is consistent with how migrations 002, 007, and 012 handle table rebuilds.
- **Junction table safety during rebuild:** All four table rebuilds preserve IDs. With `PRAGMA foreign_keys = OFF`, the DROP + RENAME cycle does not trigger cascade deletes. Junction tables continue to reference valid IDs after the rebuild.
- **Empty board:** If no items exist for an entity, the board shows the `emptyMessage` prop content instead of empty columns.
- **API failure on drag:** Optimistic UI reverts the card to its original column and shows an error toast via the existing toast system.
- **Large item lists:** The board fetches with `limit: 500`. For a single-user app with ~100-500 items per entity, client-side filtering is sufficient.
- **Sources `deriving` status and drag:** Cards in `deriving` status must not be draggable. If `svelte-dnd-action` does not support per-item drag disabling, add a check in the `onDrop` handler that rejects transitions from `deriving` and show a toast explaining why.

---

## Tasks

### T43.1: Write Migration `017_unified_kanban_statuses.sql` [CRITICAL]

**File:** `packages/core/src/db/migrations/017_unified_kanban_statuses.sql`

Expands CHECK constraints on `bullets`, `sources`, `resumes`, and `perspectives` tables. Uses the table-rebuild pattern (SQLite cannot ALTER CHECK constraints). Status remapping:
- Bullets/Perspectives: `pending_review` -> `in_review`; add `archived`
- Resumes: `final` -> `approved`; add `in_review`, `rejected`, `archived`
- Sources: add `in_review`, `rejected`, `archived` (keep `deriving`)

**Important pre-check [INCONSISTENCY]:** Before writing the final migration, run `PRAGMA table_info(bullets)` to verify whether `source_id` still exists on the `bullets` table. If `bullet_sources` junction has fully replaced it, omit `source_id` from `bullets_new`. The spec's CREATE TABLE for `bullets_new` omits `source_id`, indicating the junction table is the current foreign key mechanism.

```sql
-- Unified Kanban Statuses
-- Migration: 017_unified_kanban_statuses
-- Expands status CHECK constraints for bullets, sources, resumes, perspectives.
-- Renames pending_review -> in_review in bullets and perspectives.
-- Replaces final -> approved in resumes.
-- Adds in_review, rejected, archived to sources.
-- Adds archived to bullets and perspectives.

PRAGMA foreign_keys = OFF;

-- NOTE: The migration runner wraps each migration in BEGIN/COMMIT.
-- PRAGMA foreign_keys = OFF is silently ignored inside an active transaction.
-- The PRAGMA calls are defensive only -- the actual FK protection comes from
-- the runner's transaction ensuring all statements execute atomically.
-- This is consistent with how migrations 002, 007, and 012 handle table rebuilds.

-- NOTE: Verify whether `source_id` still exists on the `bullets` table before
-- the rebuild. If it does, include it in `bullets_new` CREATE TABLE and the
-- INSERT SELECT. If `bullet_sources` junction has fully replaced it, omit it.
-- Run `PRAGMA table_info(bullets)` to check.

-- == Bullets =================================================================

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

-- == Perspectives ============================================================

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

-- == Sources =================================================================

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

-- == Resumes =================================================================

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

-- == Rebuild junction table FKs ==============================================

-- bullet_sources: references bullets(id). Same UUIDs preserved; no action needed.
-- bullet_technologies: references bullets(id) ON DELETE CASCADE. Same IDs preserved.
-- bullet_skills: references bullets(id) ON DELETE CASCADE. Same IDs preserved.
-- perspective_skills: references perspectives(id) ON DELETE CASCADE. Same IDs preserved.
-- resume_perspectives: references resumes(id) ON DELETE CASCADE,
--   perspectives(id) ON DELETE RESTRICT. Same IDs preserved.
-- resume_sections: references resumes(id) ON DELETE CASCADE. Same IDs preserved.
-- source_roles, source_projects, source_education, source_clearances:
--   reference sources(id) ON DELETE CASCADE. Same IDs preserved.
-- source_skills: references sources(id) ON DELETE CASCADE. Same IDs preserved.

PRAGMA foreign_keys = ON;

INSERT INTO _migrations (name) VALUES ('017_unified_kanban_statuses');
```

**Key points:**
- Status remapping is done inline via `CASE WHEN` in the SELECT during INSERT, consistent with migration 012.
- `SELECT *` is avoided: explicit column list prevents breakage if columns are added between migrations.
- Bullets/perspectives: `pending_review` -> `in_review`. No data loss.
- Resumes: `final` -> `approved`. Semantically equivalent.
- Sources: no data remapping. Existing `draft`, `approved`, `deriving` values are all valid. New statuses (`in_review`, `rejected`, `archived`) have no existing data.
- All junction tables preserve their references because UUIDs are unchanged.
- [CRITICAL] The `bullets_new` table must match the current schema exactly. Verify column list with `PRAGMA table_info(bullets)` before running.

**Acceptance criteria:**
- After migration, `SELECT status FROM bullets WHERE status = 'pending_review'` returns 0 rows.
- After migration, `SELECT status FROM perspectives WHERE status = 'pending_review'` returns 0 rows.
- After migration, `SELECT status FROM resumes WHERE status = 'final'` returns 0 rows.
- Sources retain their original statuses (`draft`, `approved`, `deriving`).
- New status values (`archived`, `in_review` for sources, `rejected` for sources) can be inserted.
- Row count before = row count after for all four tables.
- All junction tables remain intact (no orphaned rows).
- `_migrations` table contains `017_unified_kanban_statuses`.

**Failure criteria:**
- Migration fails with SQL error.
- Rows lost during table rebuild (count mismatch).
- Junction table FK violations after migration.
- Old status values (`pending_review`, `final`) still pass CHECK constraints.

---

### T43.2: Update Core Types [CRITICAL]

**File:** `packages/core/src/types/index.ts`

Update the status union types and add the unified kanban status type.

```typescript
// Unified kanban status set (used by bullets, sources, resumes, perspectives)
export type UnifiedKanbanStatus = 'draft' | 'in_review' | 'approved' | 'rejected' | 'archived'

// Per-entity status types
export type BulletStatus = 'draft' | 'in_review' | 'approved' | 'rejected' | 'archived'
export type SourceStatus = 'draft' | 'in_review' | 'approved' | 'rejected' | 'archived' | 'deriving'
export type ResumeStatus = 'draft' | 'in_review' | 'approved' | 'rejected' | 'archived'
export type PerspectiveStatus = 'draft' | 'in_review' | 'approved' | 'rejected' | 'archived'
```

**Key points [IMPORTANT]:**
- `SourceStatus` includes `deriving` as a transient machine lock status, not a kanban column.
- `pending_review` is removed from `BulletStatus` and `PerspectiveStatus`.
- `final` is removed from `ResumeStatus`.
- The `UnifiedKanbanStatus` type is the intersection -- the 5 statuses that map to kanban columns.

**Acceptance criteria:**
- All four entity status types updated to the new values.
- `UnifiedKanbanStatus` type exported.
- TypeScript compiler accepts `'in_review'` for all entity status types.
- TypeScript compiler rejects `'pending_review'` for `BulletStatus`.
- TypeScript compiler rejects `'final'` for `ResumeStatus`.

**Failure criteria:**
- Old status values still in union types.
- Missing `deriving` from `SourceStatus`.

---

### T43.3: Update SDK Types [CRITICAL]

**File:** `packages/sdk/src/types.ts`

Mirror the status type changes from core. Update all places where bullet, source, resume, and perspective status types are defined.

**Changes:**
- `Bullet.status`: `'draft' | 'in_review' | 'approved' | 'rejected' | 'archived'`
- `Source.status`: `'draft' | 'in_review' | 'approved' | 'rejected' | 'archived' | 'deriving'`
- `Resume.status`: `'draft' | 'in_review' | 'approved' | 'rejected' | 'archived'`
- `Perspective.status`: `'draft' | 'in_review' | 'approved' | 'rejected' | 'archived'`
- Update corresponding `Create*` and `Update*` types to match.
- Add exported `UnifiedKanbanStatus` type.

**Acceptance criteria:**
- SDK types match core types exactly.
- `CreateBullet`, `UpdateBullet`, etc. accept the new status values.
- Old values (`pending_review`, `final`) rejected by TypeScript.

**Failure criteria:**
- Type mismatch between SDK and core.
- Missing `deriving` in source status.

---

### T43.4: Update Constants [IMPORTANT]

**File:** `packages/core/src/constants/index.ts`

Add the unified kanban statuses constant and update per-entity status arrays.

```typescript
/** Unified kanban column statuses (5-status model). */
export const UNIFIED_KANBAN_STATUSES = ['draft', 'in_review', 'approved', 'rejected', 'archived'] as const

/** Valid bullet statuses. */
export const BULLET_STATUSES = ['draft', 'in_review', 'approved', 'rejected', 'archived'] as const

/** Valid source statuses (includes transient 'deriving' lock). */
export const SOURCE_STATUSES = ['draft', 'in_review', 'approved', 'rejected', 'archived', 'deriving'] as const

/** Valid resume statuses. */
export const RESUME_STATUSES = ['draft', 'in_review', 'approved', 'rejected', 'archived'] as const

/** Valid perspective statuses. */
export const PERSPECTIVE_STATUSES = ['draft', 'in_review', 'approved', 'rejected', 'archived'] as const
```

**Acceptance criteria:**
- All constant arrays contain the correct status values.
- `UNIFIED_KANBAN_STATUSES` excludes `deriving`.
- `SOURCE_STATUSES` includes `deriving`.
- Old values (`pending_review`, `final`) not present.

**Failure criteria:**
- Constants out of sync with types.

---

### T43.5: Update Repository Status Validation (4 files) [CRITICAL]

**Files:**
- `packages/core/src/db/repositories/bullet-repository.ts`
- `packages/core/src/db/repositories/source-repository.ts`
- `packages/core/src/db/repositories/resume-repository.ts`
- `packages/core/src/db/repositories/perspective-repository.ts`

**Bullet repository:**
- Replace `VALID_STATUSES` (or equivalent) to accept: `draft`, `in_review`, `approved`, `rejected`, `archived`.
- Reject `pending_review`.

**Source repository:**
- Update to accept: `draft`, `in_review`, `approved`, `rejected`, `archived`, `deriving`.
- Previously only accepted: `draft`, `approved`, `deriving`.

**Resume repository:**
- Update to accept: `draft`, `in_review`, `approved`, `rejected`, `archived`.
- Reject `final`.
- Previously only accepted: `draft`, `final`.

**Perspective repository:**
- Same as bullet repository: accept `draft`, `in_review`, `approved`, `rejected`, `archived`.
- Reject `pending_review`.

**Acceptance criteria:**
- Each repository's status validation accepts all new status values.
- Each repository rejects the old removed status values.
- `source-repository.ts` accepts `deriving` as a valid status.

**Failure criteria:**
- Old status values pass validation.
- New status values rejected.
- `deriving` not accepted for sources.

---

### T43.6: Update Service Layer (2 files) [IMPORTANT]

**Files:**
- `packages/core/src/services/derivation-service.ts`
- `packages/core/src/services/resume-service.ts`

**DerivationService changes:**
- Add check: reject `archived` sources as derivation input. Only `approved` sources can have bullets derived.
- Add check: reject `archived` bullets as derivation input. Only `approved` bullets can have perspectives derived.
- The existing checks for `approved`-only derivation should already cover this (archived != approved), but add an explicit error message for `archived` inputs: "Cannot derive from archived source/bullet. Unarchive it first."

**ResumeService changes:**
- Add check: reject `archived` perspectives from being added to resumes. Only `approved` perspectives can be added.
- The existing check should cover this, but add an explicit error message.

**Acceptance criteria:**
- `derivationService.deriveBullets(archivedSourceId)` returns error with message mentioning "archived".
- `derivationService.derivePerspectives(archivedBulletId)` returns error with message mentioning "archived".
- `resumeService.addPerspective(resumeId, archivedPerspectiveId)` returns error.

**Failure criteria:**
- Archived entities silently pass derivation/addition checks.

---

### T43.7: Update API Route Handlers (4 files) [IMPORTANT]

**Files:**
- `packages/core/src/routes/bullets.ts`
- `packages/core/src/routes/sources.ts`
- `packages/core/src/routes/resumes.ts`
- `packages/core/src/routes/perspectives.ts`

Update the status validation in each PATCH handler to accept the new status values and reject old ones.

**Per route:**
- Import the appropriate status constant from `constants/index.ts`.
- In the PATCH handler, validate that `status` (if provided) is in the allowed set.
- Return 400 with `VALIDATION_ERROR` for invalid statuses.

**Acceptance criteria:**
- `PATCH /api/bullets/:id { status: 'in_review' }` returns 200.
- `PATCH /api/bullets/:id { status: 'pending_review' }` returns 400.
- `PATCH /api/bullets/:id { status: 'archived' }` returns 200.
- `PATCH /api/sources/:id { status: 'in_review' }` returns 200.
- `PATCH /api/sources/:id { status: 'archived' }` returns 200.
- `PATCH /api/resumes/:id { status: 'approved' }` returns 200.
- `PATCH /api/resumes/:id { status: 'final' }` returns 400.
- `PATCH /api/perspectives/:id { status: 'in_review' }` returns 200.
- `PATCH /api/perspectives/:id { status: 'pending_review' }` returns 400.

**Failure criteria:**
- Old status values accepted by API.
- New status values rejected.

---

### T43.8: Write `GenericKanban.svelte` [CRITICAL]

**File:** `packages/webui/src/lib/components/kanban/GenericKanban.svelte`

The core reusable kanban component. Uses Svelte 5 generics for type safety. Accepts column definitions, items, card renderer snippet, and drop callback as props. Manages local state per column, synced from props via `$effect`. Supports filter bar snippet, loading state, empty state, and default-collapsed columns.

```svelte
<script lang="ts" generics="T extends { id: string; status: string }">
  import type { Snippet } from 'svelte'
  import GenericKanbanColumn from './GenericKanbanColumn.svelte'
  import LoadingSpinner from '../LoadingSpinner.svelte'

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

  let {
    columns,
    items,
    onDrop,
    loading = false,
    loadingMessage = 'Loading...',
    emptyMessage = 'No items yet.',
    filterBar,
    cardContent,
    defaultCollapsed = '',
    sortItems = (a: T, b: T) => {
      // Default sort: alphabetical by first string field after id and status
      const aVal = Object.values(a).find((v, i) => i > 1 && typeof v === 'string') as string ?? ''
      const bVal = Object.values(b).find((v, i) => i > 1 && typeof v === 'string') as string ?? ''
      return aVal.localeCompare(bVal)
    },
  }: {
    columns: ColumnDef[]
    items: T[]
    onDrop: (itemId: string, newStatus: string) => Promise<void>
    loading?: boolean
    loadingMessage?: string
    emptyMessage?: string
    filterBar?: Snippet
    cardContent: Snippet<[T]>
    defaultCollapsed?: string
    sortItems?: (a: T, b: T) => number
  } = $props()

  // Track which columns are collapsed
  let collapsedColumns = $state<Record<string, boolean>>({})

  // Initialize collapsed state for defaultCollapsed column
  $effect(() => {
    if (defaultCollapsed && !(defaultCollapsed in collapsedColumns)) {
      collapsedColumns[defaultCollapsed] = true
    }
  })

  // Group items into columns, synced from props
  let columnState = $state<Map<string, T[]>>(new Map())

  $effect(() => {
    const grouped = new Map<string, T[]>()
    for (const col of columns) {
      const colItems = items
        .filter(item => col.statuses.includes(item.status))
        .sort(sortItems)
      grouped.set(col.key, colItems)
    }
    columnState = grouped
  })

  function toggleCollapse(key: string) {
    collapsedColumns[key] = !collapsedColumns[key]
  }

  async function handleDrop(columnKey: string, itemId: string) {
    const col = columns.find(c => c.key === columnKey)
    if (!col) return
    const newStatus = col.dropStatus ?? col.statuses[0]

    // Check if the item is already in this column (intra-column reorder)
    const currentItems = columnState.get(columnKey) ?? []
    if (currentItems.some(i => i.id === itemId)) return // no-op

    // Optimistic update: move item to new column locally
    const item = items.find(i => i.id === itemId)
    if (!item) return

    const previousStatus = item.status
    // Temporarily update status for local display
    ;(item as any).status = newStatus

    // Re-group to reflect the change
    const grouped = new Map<string, T[]>()
    for (const c of columns) {
      const cItems = items
        .filter(i => c.statuses.includes(i.status))
        .sort(sortItems)
      grouped.set(c.key, cItems)
    }
    columnState = grouped

    try {
      await onDrop(itemId, newStatus)
    } catch {
      // Revert on failure
      ;(item as any).status = previousStatus
      const reverted = new Map<string, T[]>()
      for (const c of columns) {
        const cItems = items
          .filter(i => c.statuses.includes(i.status))
          .sort(sortItems)
        reverted.set(c.key, cItems)
      }
      columnState = reverted
    }
  }
</script>

{#if loading}
  <div class="board-loading">
    <LoadingSpinner message={loadingMessage} />
  </div>
{:else if items.length === 0}
  <div class="board-empty">
    <p>{emptyMessage}</p>
  </div>
{:else}
  {#if filterBar}
    <div class="board-filter-bar">
      {@render filterBar()}
    </div>
  {/if}

  <div class="board-columns">
    {#each columns as col (col.key)}
      <GenericKanbanColumn
        column={col}
        items={columnState.get(col.key) ?? []}
        {cardContent}
        collapsed={collapsedColumns[col.key] ?? false}
        onToggleCollapse={() => toggleCollapse(col.key)}
        onDrop={(itemId) => handleDrop(col.key, itemId)}
      />
    {/each}
  </div>
{/if}

<style>
  .board-loading {
    display: flex;
    justify-content: center;
    align-items: center;
    padding: var(--space-12, 3rem);
  }

  .board-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: var(--space-12, 3rem);
    color: var(--text-muted, #6b7280);
    font-size: var(--text-base, 0.875rem);
    text-align: center;
    gap: var(--space-1, 0.25rem);
  }

  .board-empty p {
    margin: 0;
  }

  .board-filter-bar {
    padding: var(--space-3, 0.75rem) var(--space-4, 1rem);
    border-bottom: 1px solid var(--color-border, #e5e7eb);
  }

  .board-columns {
    display: flex;
    gap: var(--space-3, 0.75rem);
    padding: var(--space-4, 1rem);
    flex: 1;
    overflow-x: auto;
    align-items: stretch;
  }
</style>
```

**Key points:**
- Uses Svelte 5 generics: `generics="T extends { id: string; status: string }"`.
- The `cardContent` snippet receives `[T]` -- the individual item to render.
- `filterBar` is an optional snippet rendered above the board.
- Optimistic UI: local state updates immediately on drop; reverts on failure.
- Intra-column reorder is a no-op (no position field in schema).
- The component derives status from `item.status` -- all Forge entities have this field.
- [ANTI-PATTERN] The fallback `sortItems` function uses `Object.values()` indexing which is fragile. Each entity page should provide its own sort function.

**Acceptance criteria:**
- Component renders columns from `columns` prop.
- Items appear in correct columns based on `status` field matching `col.statuses`.
- `defaultCollapsed` column is collapsed on initial render.
- Dropping a card into a new column calls `onDrop(itemId, newStatus)`.
- On `onDrop` rejection, card reverts to original column.
- Loading state shows spinner.
- Empty state shows `emptyMessage`.
- Filter bar renders above columns when provided.

**Failure criteria:**
- Svelte 5 generics syntax not supported (compilation error).
- Cards in wrong columns.
- Optimistic revert fails.
- Collapsed column not working.

---

### T43.9: Write `GenericKanbanColumn.svelte` [CRITICAL]

**File:** `packages/webui/src/lib/components/kanban/GenericKanbanColumn.svelte`

A single kanban column with `svelte-dnd-action` drop zone, header with label and count badge, and optional collapsed state. Reuses the visual pattern from the Phase 38 `KanbanColumn.svelte` but accepts generic items and a card content snippet.

```svelte
<script lang="ts" generics="T extends { id: string }">
  import type { Snippet } from 'svelte'
  import { dndzone } from 'svelte-dnd-action'

  interface ColumnDef {
    key: string
    label: string
    statuses: string[]
    dropStatus?: string
    accent: string
  }

  let {
    column,
    items,
    cardContent,
    collapsed = false,
    onToggleCollapse,
    onDrop,
  }: {
    column: ColumnDef
    items: (T & { id: string })[]
    cardContent: Snippet<[T]>
    collapsed?: boolean
    onToggleCollapse?: () => void
    onDrop: (itemId: string) => void
  } = $props()

  let localItems = $state<(T & { id: string })[]>([])

  $effect(() => {
    localItems = [...items]
  })

  function handleConsider(e: CustomEvent) {
    localItems = e.detail.items
  }

  function handleFinalize(e: CustomEvent) {
    localItems = e.detail.items
    // Identify which items are new to this column
    const originalIds = new Set(items.map(i => i.id))
    const newItems = localItems.filter(i => !originalIds.has(i.id))
    for (const item of newItems) {
      onDrop(item.id)
    }
  }

  const flipDurationMs = 200
</script>

{#if collapsed}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div
    class="column-collapsed"
    style:border-top-color={column.accent}
    onclick={onToggleCollapse}
    role="button"
    tabindex="0"
    onkeydown={(e) => { if (e.key === 'Enter') onToggleCollapse?.() }}
  >
    <span class="collapsed-label">{column.label}</span>
    <span class="collapsed-count">{items.length}</span>
  </div>
{:else}
  <div class="column" style:border-top-color={column.accent}>
    <div class="column-header">
      <h3 class="column-label">{column.label}</h3>
      <span class="column-count">{localItems.length}</span>
      {#if onToggleCollapse}
        <button class="collapse-btn" onclick={onToggleCollapse} title="Collapse">
          &#x2715;
        </button>
      {/if}
    </div>

    <div
      class="column-body"
      use:dndzone={{ items: localItems, flipDurationMs, dropTargetStyle: { outline: `2px dashed ${column.accent}` } }}
      onconsider={handleConsider}
      onfinalize={handleFinalize}
    >
      {#each localItems as item (item.id)}
        <div class="kanban-card-wrapper">
          {@render cardContent(item)}
        </div>
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
    background: var(--color-surface-raised, #f9fafb);
    border-radius: var(--radius-md, 6px);
    overflow: hidden;
  }

  .column-header {
    display: flex;
    align-items: center;
    gap: var(--space-2, 0.4rem);
    padding: var(--space-3, 0.6rem) var(--space-3, 0.75rem);
    background: var(--color-surface, #fff);
    border-bottom: 1px solid var(--color-border, #e5e7eb);
  }

  .column-label {
    font-size: var(--text-sm, 0.8rem);
    font-weight: var(--font-bold, 700);
    color: var(--text-primary, #1a1a2e);
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
    padding: 0 var(--space-1, 0.3rem);
    background: var(--color-border, #e5e7eb);
    color: var(--text-secondary, #374151);
    border-radius: var(--radius-full, 999px);
    font-size: var(--text-xs, 0.7rem);
    font-weight: var(--font-semibold, 600);
  }

  .collapse-btn {
    background: none;
    border: none;
    color: var(--text-faint, #9ca3af);
    cursor: pointer;
    font-size: var(--text-xs, 0.7rem);
    padding: 0.15rem;
    border-radius: var(--radius-sm, 3px);
    line-height: 1;
  }

  .collapse-btn:hover {
    color: var(--text-secondary, #374151);
    background: var(--color-surface-sunken, #f3f4f6);
  }

  .column-body {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-2, 0.5rem);
    min-height: 60px;
  }

  .kanban-card-wrapper {
    margin-bottom: var(--space-1, 0.35rem);
  }

  .column-collapsed {
    width: 48px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-2, 0.5rem);
    background: var(--color-surface-sunken, #f3f4f6);
    border-top: 3px solid;
    border-radius: var(--radius-md, 6px);
    cursor: pointer;
    transition: background 0.12s;
    padding: var(--space-4, 1rem) 0;
    min-height: 200px;
  }

  .column-collapsed:hover {
    background: var(--color-border, #e5e7eb);
  }

  .collapsed-label {
    writing-mode: vertical-rl;
    text-orientation: mixed;
    font-size: var(--text-sm, 0.75rem);
    font-weight: var(--font-semibold, 600);
    color: var(--text-muted, #6b7280);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .collapsed-count {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 1.4rem;
    height: 1.4rem;
    background: var(--color-border-strong, #d1d5db);
    color: var(--text-secondary, #374151);
    border-radius: var(--radius-full, 999px);
    font-size: var(--text-xs, 0.65rem);
    font-weight: var(--font-semibold, 600);
  }
</style>
```

**Key points:**
- Mirrors the Phase 38 `KanbanColumn.svelte` visual design but uses generic types and a snippet-based card renderer.
- Uses Svelte 5 generics: `generics="T extends { id: string }"`.
- `localItems` is a mutable copy synced from `items` via `$effect` -- `svelte-dnd-action` requires a mutable array.
- Uses `onconsider`/`onfinalize` event handlers (Svelte 5 syntax), matching proven patterns.
- `dropTargetStyle` uses the column's accent color.
- Collapsed mode renders as a thin vertical strip with rotated text and count badge.
- CSS values use token vars with fallbacks for graceful degradation if Phase 42 is not yet applied.

**Acceptance criteria:**
- Column renders header with label, count badge, and accent color top border.
- Cards appear sorted inside the column via snippet rendering.
- Dragging a card from another column into this column fires `onDrop(itemId)`.
- Collapsed column renders as thin vertical strip with rotated label and count.
- Clicking collapsed column expands it.

**Failure criteria:**
- `svelte-dnd-action` throws errors about incompatible event syntax.
- Cards disappear during drag-and-drop.
- Collapsed column does not respond to click.
- Snippet rendering fails.

---

### T43.10: Write `ViewToggle.svelte` [IMPORTANT]

**File:** `packages/webui/src/lib/components/ViewToggle.svelte`

Reusable segmented control for switching between list and board views.

```svelte
<script lang="ts">
  let { mode, onchange }: {
    mode: 'list' | 'board'
    onchange: (mode: 'list' | 'board') => void
  } = $props()
</script>

<div class="view-toggle">
  <button
    class="toggle-btn"
    class:active={mode === 'list'}
    onclick={() => onchange('list')}
  >
    List
  </button>
  <button
    class="toggle-btn"
    class:active={mode === 'board'}
    onclick={() => onchange('board')}
  >
    Board
  </button>
</div>

<style>
  .view-toggle {
    display: inline-flex;
    border: 1px solid var(--color-border-strong, #d1d5db);
    border-radius: var(--radius-md, 6px);
    overflow: hidden;
  }

  .toggle-btn {
    padding: var(--space-1, 0.25rem) var(--space-3, 0.75rem);
    border: none;
    background: var(--color-surface, #ffffff);
    color: var(--text-muted, #6b7280);
    font-size: var(--text-sm, 0.8rem);
    font-weight: var(--font-medium, 500);
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }

  .toggle-btn:not(:last-child) {
    border-right: 1px solid var(--color-border-strong, #d1d5db);
  }

  .toggle-btn.active {
    background: var(--color-primary, #6c63ff);
    color: var(--text-inverse, #ffffff);
  }

  .toggle-btn:hover:not(.active) {
    background: var(--color-ghost, #f3f4f6);
  }
</style>
```

**Acceptance criteria:**
- Two buttons rendered ("List", "Board") in a segmented control.
- Active button has primary background with inverse text.
- Inactive button has surface background with muted text.
- Clicking a button calls `onchange` with the corresponding mode.

**Failure criteria:**
- Buttons not joined (gap between them).
- Active state not visually distinct.

---

### T43.11: Write View Mode Store [IMPORTANT]

**File:** `packages/webui/src/lib/stores/viewMode.svelte.ts`

localStorage-backed store for view mode preference per entity.

```typescript
import { browser } from '$app/environment'

const STORAGE_KEY_PREFIX = 'forge:viewMode:'

export type ViewMode = 'list' | 'board'

export function getViewMode(entity: string): ViewMode {
  if (!browser) return 'list'
  try {
    const stored = localStorage.getItem(`${STORAGE_KEY_PREFIX}${entity}`)
    return stored === 'board' ? 'board' : 'list' // default to list
  } catch {
    return 'list'
  }
}

export function setViewMode(entity: string, mode: ViewMode) {
  if (!browser) return
  try {
    localStorage.setItem(`${STORAGE_KEY_PREFIX}${entity}`, mode)
  } catch {
    // localStorage unavailable, silently ignore
  }
}
```

**Key points:**
- Default is `list` (existing behavior).
- Keys: `forge:viewMode:bullets`, `forge:viewMode:sources`, `forge:viewMode:resumes`, `forge:viewMode:perspectives`.
- SSR-safe: all localStorage access gated behind `browser` check.
- Wrapped in try/catch for environments where localStorage is unavailable.

**Acceptance criteria:**
- `getViewMode('bullets')` returns `'list'` by default.
- After `setViewMode('bullets', 'board')`, `getViewMode('bullets')` returns `'board'`.
- No errors during SSR.

**Failure criteria:**
- SSR crashes on `localStorage` access.
- Default is not `'list'`.

---

### T43.12: Write Entity Kanban Card Components (4 files) [IMPORTANT]

**Files:**
- `packages/webui/src/lib/components/kanban/BulletKanbanCard.svelte`
- `packages/webui/src/lib/components/kanban/SourceKanbanCard.svelte`
- `packages/webui/src/lib/components/kanban/ResumeKanbanCard.svelte`
- `packages/webui/src/lib/components/kanban/PerspectiveKanbanCard.svelte`

**BulletKanbanCard.svelte:**

```svelte
<script lang="ts">
  import type { Bullet } from '@forge/sdk'

  let { bullet, onclick }: {
    bullet: Bullet
    onclick: () => void
  } = $props()

  let contentPreview = $derived(
    bullet.content.length > 80 ? bullet.content.slice(0, 80) + '...' : bullet.content
  )
  let isRejected = $derived(bullet.status === 'rejected')
  let isArchived = $derived(bullet.status === 'archived')
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<div
  class="kanban-card"
  class:rejected={isRejected}
  class:archived={isArchived}
  onclick={onclick}
  role="button"
  tabindex="0"
  onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onclick() } }}
>
  <p class="card-content">{contentPreview}</p>

  {#if bullet.domain}
    <span class="domain-badge">{bullet.domain}</span>
  {/if}

  {#if bullet.technologies && bullet.technologies.length > 0}
    <div class="tech-pills">
      {#each bullet.technologies.slice(0, 3) as tech}
        <span class="pill">{tech}</span>
      {/each}
      {#if bullet.technologies.length > 3}
        <span class="pill pill-neutral">+{bullet.technologies.length - 3}</span>
      {/if}
    </div>
  {/if}

  {#if bullet.source_title}
    <span class="source-ref">{bullet.source_title}</span>
  {/if}
</div>

<style>
  .kanban-card {
    padding: var(--space-2, 0.5rem) var(--space-3, 0.75rem);
    border: 1px solid var(--color-border, #e5e7eb);
    border-radius: var(--radius-md, 6px);
    cursor: grab;
    background: var(--color-surface, #ffffff);
    transition: box-shadow 0.12s, opacity 0.12s;
  }

  .kanban-card:hover {
    box-shadow: var(--shadow-sm, 0 1px 3px rgba(0, 0, 0, 0.08));
  }

  .kanban-card:active {
    cursor: grabbing;
  }

  .kanban-card.rejected {
    opacity: 0.7;
  }

  .kanban-card.archived {
    opacity: 0.5;
    color: var(--text-muted, #6b7280);
  }

  .card-content {
    font-size: var(--text-sm, 0.8rem);
    color: var(--text-primary, #1a1a2e);
    line-height: var(--leading-normal, 1.5);
    margin: 0 0 var(--space-1, 0.25rem) 0;
  }

  .domain-badge {
    display: inline-block;
    padding: 0.05em 0.3em;
    background: var(--color-info-subtle, #eff6ff);
    color: var(--color-info-text, #1e40af);
    border-radius: var(--radius-sm, 3px);
    font-size: var(--text-xs, 0.65rem);
    font-weight: var(--font-medium, 500);
    margin-bottom: var(--space-1, 0.25rem);
  }

  .tech-pills {
    display: flex;
    flex-wrap: wrap;
    gap: 0.2rem;
    margin-bottom: var(--space-1, 0.2rem);
  }

  .source-ref {
    display: block;
    font-size: var(--text-xs, 0.65rem);
    color: var(--text-faint, #9ca3af);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
```

**SourceKanbanCard.svelte:**

```svelte
<script lang="ts">
  import type { Source } from '@forge/sdk'

  let { source, onclick }: {
    source: Source & { bullet_count?: number; organization_name?: string }
    onclick: () => void
  } = $props()

  const TYPE_LABELS: Record<string, string> = {
    role: 'Role',
    project: 'Project',
    education: 'Education',
    clearance: 'Clearance',
    general: 'General',
  }

  let typeLabel = $derived(TYPE_LABELS[source.source_type] ?? source.source_type)
  let isDeriving = $derived(source.status === 'deriving')
  let isArchived = $derived(source.status === 'archived')
  let isRejected = $derived(source.status === 'rejected')
  let dateRange = $derived(() => {
    if (!source.start_date) return ''
    const start = source.start_date.slice(0, 7) // YYYY-MM
    const end = source.end_date ? source.end_date.slice(0, 7) : 'Present'
    return `${start} -- ${end}`
  })
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<div
  class="kanban-card"
  class:deriving={isDeriving}
  class:archived={isArchived}
  class:rejected={isRejected}
  onclick={onclick}
  role="button"
  tabindex="0"
  onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onclick() } }}
>
  <div class="card-header">
    <span class="card-title">{source.title}</span>
    <span class="type-badge">{typeLabel}</span>
  </div>

  {#if source.organization_name}
    <span class="org-name">{source.organization_name}</span>
  {/if}

  {#if dateRange()}
    <span class="date-range">{dateRange()}</span>
  {/if}

  <div class="card-footer">
    {#if source.bullet_count !== undefined}
      <span class="bullet-count">{source.bullet_count} bullets</span>
    {/if}
    {#if isDeriving}
      <span class="deriving-indicator" title="Derivation in progress">&#x21BB;</span>
    {/if}
  </div>
</div>

<style>
  .kanban-card {
    padding: var(--space-2, 0.5rem) var(--space-3, 0.75rem);
    border: 1px solid var(--color-border, #e5e7eb);
    border-radius: var(--radius-md, 6px);
    cursor: grab;
    background: var(--color-surface, #ffffff);
    transition: box-shadow 0.12s, opacity 0.12s;
  }

  .kanban-card:hover {
    box-shadow: var(--shadow-sm, 0 1px 3px rgba(0, 0, 0, 0.08));
  }

  .kanban-card:active {
    cursor: grabbing;
  }

  .kanban-card.deriving {
    cursor: not-allowed;
    border-left: 3px solid var(--color-info, #3b82f6);
  }

  .kanban-card.archived {
    opacity: 0.5;
    color: var(--text-muted, #6b7280);
  }

  .kanban-card.rejected {
    opacity: 0.7;
  }

  .card-header {
    display: flex;
    align-items: center;
    gap: var(--space-2, 0.4rem);
    margin-bottom: var(--space-1, 0.25rem);
  }

  .card-title {
    font-size: var(--text-sm, 0.82rem);
    font-weight: var(--font-semibold, 600);
    color: var(--text-primary, #1a1a2e);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    min-width: 0;
  }

  .type-badge {
    display: inline-block;
    padding: 0.05em 0.3em;
    background: var(--color-tag-bg, #e0e7ff);
    color: var(--color-tag-text, #3730a3);
    border-radius: var(--radius-sm, 3px);
    font-size: var(--text-xs, 0.6rem);
    font-weight: var(--font-medium, 500);
    flex-shrink: 0;
  }

  .org-name {
    display: block;
    font-size: var(--text-xs, 0.7rem);
    color: var(--text-muted, #6b7280);
    margin-bottom: var(--space-1, 0.15rem);
  }

  .date-range {
    display: block;
    font-size: var(--text-xs, 0.65rem);
    color: var(--text-faint, #9ca3af);
    margin-bottom: var(--space-1, 0.15rem);
  }

  .card-footer {
    display: flex;
    align-items: center;
    gap: var(--space-2, 0.4rem);
  }

  .bullet-count {
    font-size: var(--text-xs, 0.65rem);
    color: var(--text-faint, #9ca3af);
  }

  .deriving-indicator {
    font-size: var(--text-sm, 0.8rem);
    color: var(--color-info, #3b82f6);
    animation: spin 1.5s linear infinite;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }
</style>
```

**ResumeKanbanCard.svelte:**

```svelte
<script lang="ts">
  import type { Resume } from '@forge/sdk'

  let { resume, onclick }: {
    resume: Resume & { section_count?: number }
    onclick: () => void
  } = $props()

  let isApproved = $derived(resume.status === 'approved')
  let isArchived = $derived(resume.status === 'archived')
  let isRejected = $derived(resume.status === 'rejected')
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<div
  class="kanban-card"
  class:archived={isArchived}
  class:rejected={isRejected}
  onclick={onclick}
  role="button"
  tabindex="0"
  onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onclick() } }}
>
  <div class="card-header">
    <span class="card-title">{resume.name}</span>
    {#if isApproved}
      <span class="approved-check" title="Approved">&#x2713;</span>
    {/if}
  </div>
  <span class="target-role">{resume.target_role}</span>
  <span class="target-employer">{resume.target_employer}</span>
  <div class="card-meta">
    <span class="archetype-badge">{resume.archetype}</span>
    {#if resume.section_count !== undefined}
      <span class="section-count">{resume.section_count} sections</span>
    {/if}
  </div>
</div>

<style>
  .kanban-card {
    padding: var(--space-2, 0.5rem) var(--space-3, 0.75rem);
    border: 1px solid var(--color-border, #e5e7eb);
    border-radius: var(--radius-md, 6px);
    cursor: grab;
    background: var(--color-surface, #ffffff);
    transition: box-shadow 0.12s, opacity 0.12s;
  }

  .kanban-card:hover {
    box-shadow: var(--shadow-sm, 0 1px 3px rgba(0, 0, 0, 0.08));
  }

  .kanban-card:active {
    cursor: grabbing;
  }

  .kanban-card.archived {
    opacity: 0.5;
    color: var(--text-muted, #6b7280);
  }

  .kanban-card.rejected {
    opacity: 0.7;
  }

  .card-header {
    display: flex;
    align-items: center;
    gap: var(--space-2, 0.4rem);
    margin-bottom: var(--space-1, 0.15rem);
  }

  .card-title {
    font-size: var(--text-sm, 0.82rem);
    font-weight: var(--font-semibold, 600);
    color: var(--text-primary, #1a1a2e);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    min-width: 0;
  }

  .approved-check {
    color: var(--color-success, #22c55e);
    font-size: var(--text-sm, 0.8rem);
    font-weight: var(--font-bold, 700);
  }

  .target-role {
    display: block;
    font-size: var(--text-xs, 0.75rem);
    color: var(--text-secondary, #374151);
    margin-bottom: 0.1rem;
  }

  .target-employer {
    display: block;
    font-size: var(--text-xs, 0.7rem);
    color: var(--text-muted, #6b7280);
    margin-bottom: var(--space-1, 0.25rem);
  }

  .card-meta {
    display: flex;
    gap: var(--space-2, 0.4rem);
    align-items: center;
  }

  .archetype-badge {
    display: inline-block;
    padding: 0.05em 0.3em;
    background: var(--color-tag-bg, #e0e7ff);
    color: var(--color-tag-text, #3730a3);
    border-radius: var(--radius-sm, 3px);
    font-size: var(--text-xs, 0.6rem);
    font-weight: var(--font-medium, 500);
  }

  .section-count {
    font-size: var(--text-xs, 0.65rem);
    color: var(--text-faint, #9ca3af);
  }
</style>
```

**PerspectiveKanbanCard.svelte:**

```svelte
<script lang="ts">
  import type { Perspective } from '@forge/sdk'

  let { perspective, onclick }: {
    perspective: Perspective
    onclick: () => void
  } = $props()

  let contentPreview = $derived(
    perspective.content.length > 80 ? perspective.content.slice(0, 80) + '...' : perspective.content
  )
  let isRejected = $derived(perspective.status === 'rejected')
  let isArchived = $derived(perspective.status === 'archived')
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<div
  class="kanban-card"
  class:rejected={isRejected}
  class:archived={isArchived}
  onclick={onclick}
  role="button"
  tabindex="0"
  onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onclick() } }}
>
  <p class="card-content">{contentPreview}</p>

  <div class="badge-row">
    {#if perspective.target_archetype}
      <span class="pill">{perspective.target_archetype}</span>
    {/if}
    {#if perspective.domain}
      <span class="pill pill-neutral">{perspective.domain}</span>
    {/if}
    <span class="framing-badge">{perspective.framing}</span>
  </div>
</div>

<style>
  .kanban-card {
    padding: var(--space-2, 0.5rem) var(--space-3, 0.75rem);
    border: 1px solid var(--color-border, #e5e7eb);
    border-radius: var(--radius-md, 6px);
    cursor: grab;
    background: var(--color-surface, #ffffff);
    transition: box-shadow 0.12s, opacity 0.12s;
  }

  .kanban-card:hover {
    box-shadow: var(--shadow-sm, 0 1px 3px rgba(0, 0, 0, 0.08));
  }

  .kanban-card:active {
    cursor: grabbing;
  }

  .kanban-card.rejected {
    opacity: 0.7;
  }

  .kanban-card.archived {
    opacity: 0.5;
    color: var(--text-muted, #6b7280);
  }

  .card-content {
    font-size: var(--text-sm, 0.8rem);
    color: var(--text-primary, #1a1a2e);
    line-height: var(--leading-normal, 1.5);
    margin: 0 0 var(--space-1, 0.25rem) 0;
  }

  .badge-row {
    display: flex;
    flex-wrap: wrap;
    gap: 0.2rem;
  }

  .framing-badge {
    display: inline-block;
    padding: 0.05em 0.3em;
    background: var(--color-success-subtle, #f0fdf4);
    color: var(--color-success-text, #065f46);
    border-radius: var(--radius-sm, 3px);
    font-size: var(--text-xs, 0.6rem);
    font-weight: var(--font-medium, 500);
    text-transform: capitalize;
  }
</style>
```

**Acceptance criteria (all 4 cards):**
- Card renders correct content preview (truncated at ~80 chars for text-based cards).
- Domain/type/archetype badges visible.
- Rejected cards at `opacity: 0.7`.
- Archived cards at `opacity: 0.5` with muted text.
- Source cards in `deriving` status show spinner icon and non-grab cursor.
- Resume cards with `approved` status show green check.
- `cursor: grab` on hover, `cursor: grabbing` on active.
- Card click calls `onclick`.

**Failure criteria:**
- Content preview not truncated.
- Badges missing or incorrectly colored.
- Opacity rules not applied.
- Click handler not firing.

---

### T43.13: Write Filter Bar Components (4 files) [IMPORTANT]

**Files:**
- `packages/webui/src/lib/components/filters/BulletFilterBar.svelte`
- `packages/webui/src/lib/components/filters/SourceFilterBar.svelte`
- `packages/webui/src/lib/components/filters/ResumeFilterBar.svelte`
- `packages/webui/src/lib/components/filters/PerspectiveFilterBar.svelte`

Each filter bar is a horizontal row of dropdown selects and a search text input. Filters are applied client-side with AND logic. Each component accepts `bind:filters` (a reactive state object) and emits `onchange` when any filter changes.

**BulletFilterBar.svelte (representative example):**

```svelte
<script lang="ts">
  import { forge } from '$lib/sdk'

  let { filters, onchange }: {
    filters: {
      source?: string
      domain?: string
      skill?: string
      search?: string
    }
    onchange: () => void
  } = $props()

  let sources = $state<{ id: string; title: string }[]>([])
  let domains = $state<string[]>([])
  let skills = $state<string[]>([])

  $effect(() => {
    loadFilterOptions()
  })

  async function loadFilterOptions() {
    const [srcResult, domResult, skillResult] = await Promise.all([
      forge.sources.list({ limit: 500 }),
      forge.domains ? forge.domains.list() : Promise.resolve({ ok: true, data: [] }),
      forge.skills ? forge.skills.list({ limit: 500 }) : Promise.resolve({ ok: true, data: [] }),
    ])
    if (srcResult.ok) sources = srcResult.data.map(s => ({ id: s.id, title: s.title }))
    if (domResult.ok) domains = [...new Set(domResult.data.map((d: any) => d.name ?? d))]
    if (skillResult.ok) skills = [...new Set(skillResult.data.map((s: any) => s.name ?? s))]
  }

  function handleChange() {
    onchange()
  }
</script>

<div class="filter-bar">
  <select
    class="field-select"
    bind:value={filters.source}
    onchange={handleChange}
  >
    <option value="">All Sources</option>
    {#each sources as src}
      <option value={src.id}>{src.title}</option>
    {/each}
  </select>

  <select
    class="field-select"
    bind:value={filters.domain}
    onchange={handleChange}
  >
    <option value="">All Domains</option>
    {#each domains as domain}
      <option value={domain}>{domain}</option>
    {/each}
  </select>

  <select
    class="field-select"
    bind:value={filters.skill}
    onchange={handleChange}
  >
    <option value="">All Skills</option>
    {#each skills as skill}
      <option value={skill}>{skill}</option>
    {/each}
  </select>

  <input
    type="text"
    class="field-input"
    placeholder="Search bullets..."
    bind:value={filters.search}
    oninput={handleChange}
  />
</div>

<style>
  .filter-bar {
    display: flex;
    gap: var(--space-2, 0.5rem);
    flex-wrap: wrap;
    align-items: center;
  }

  .filter-bar .field-select,
  .filter-bar .field-input {
    font-size: var(--text-sm, 0.8rem);
    padding: var(--space-1, 0.25rem) var(--space-2, 0.5rem);
  }

  .filter-bar .field-input {
    min-width: 150px;
  }
</style>
```

The other three filter bars follow the same pattern with entity-specific filter options:

- **SourceFilterBar:** Organization dropdown, Source Type dropdown (static enum: role/project/education/clearance/general), Skill dropdown, title search.
- **ResumeFilterBar:** Archetype dropdown, Target Employer dropdown, name/role search.
- **PerspectiveFilterBar:** Archetype dropdown, Domain dropdown, Framing dropdown (static enum: accomplishment/responsibility/context), content search.

**Acceptance criteria:**
- Each filter bar renders dropdowns and search input.
- Selecting a filter calls `onchange`.
- Filters can be cleared (select "All" option).
- Dropdowns populated from API on mount.

**Failure criteria:**
- Dropdowns empty.
- Filter changes not triggering parent re-filter.
- AND logic not applied.

---

### T43.14: Integrate Board View into Entity Pages (4 files) [CRITICAL]

**Files:**
- `packages/webui/src/routes/data/bullets/+page.svelte`
- `packages/webui/src/routes/data/sources/+page.svelte` (or wherever SourcesView is used)
- `packages/webui/src/routes/resumes/+page.svelte`
- `packages/webui/src/routes/data/perspectives/+page.svelte`

Each page adds:
1. Import `ViewToggle`, `GenericKanban`, entity-specific card component, filter bar, and column definitions.
2. A `viewMode` state initialized from `getViewMode(entity)`.
3. A toggle handler that calls `setViewMode(entity, mode)`.
4. Conditional rendering: `{#if viewMode === 'board'}` shows GenericKanban, `{:else}` shows existing list view.

**Example integration for bullets page:**

```svelte
<script lang="ts">
  import { forge, friendlyError } from '$lib/sdk'
  import { addToast } from '$lib/stores/toast.svelte'
  import ViewToggle from '$lib/components/ViewToggle.svelte'
  import GenericKanban from '$lib/components/kanban/GenericKanban.svelte'
  import BulletKanbanCard from '$lib/components/kanban/BulletKanbanCard.svelte'
  import BulletFilterBar from '$lib/components/filters/BulletFilterBar.svelte'
  import { getViewMode, setViewMode } from '$lib/stores/viewMode.svelte'
  import type { Bullet } from '@forge/sdk'

  // ... existing page logic ...

  let viewMode = $state<'list' | 'board'>(getViewMode('bullets'))

  function handleViewChange(mode: 'list' | 'board') {
    viewMode = mode
    setViewMode('bullets', mode)
  }

  // Column definitions for bullets kanban
  const BULLET_COLUMNS = [
    { key: 'draft', label: 'Draft', statuses: ['draft'], accent: '#a5b4fc' },
    { key: 'in_review', label: 'In Review', statuses: ['in_review'], accent: '#fbbf24' },
    { key: 'approved', label: 'Approved', statuses: ['approved'], accent: '#22c55e' },
    { key: 'rejected', label: 'Rejected', statuses: ['rejected'], accent: '#ef4444' },
    { key: 'archived', label: 'Archived', statuses: ['archived'], accent: '#d1d5db' },
  ]

  // Filter state
  let filters = $state<{ source?: string; domain?: string; skill?: string; search?: string }>({})
  let filteredBullets = $derived(() => {
    let result = bullets // existing bullets array from page logic
    if (filters.source) result = result.filter(b => b.source_id === filters.source)
    if (filters.domain) result = result.filter(b => b.domain === filters.domain)
    if (filters.skill) result = result.filter(b =>
      b.skills?.some(s => s === filters.skill))
    if (filters.search) {
      const q = filters.search.toLowerCase()
      result = result.filter(b => b.content.toLowerCase().includes(q))
    }
    return result
  })

  async function handleBulletDrop(itemId: string, newStatus: string) {
    const result = await forge.bullets.update(itemId, { status: newStatus })
    if (!result.ok) {
      addToast({ type: 'error', message: friendlyError(result.error) })
      throw new Error('Status update failed') // triggers optimistic revert
    }
    addToast({ type: 'success', message: `Bullet moved to ${newStatus.replace('_', ' ')}` })
  }
</script>

<div class="page-header">
  <h1>Bullets</h1>
  <ViewToggle mode={viewMode} onchange={handleViewChange} />
</div>

{#if viewMode === 'board'}
  <GenericKanban
    columns={BULLET_COLUMNS}
    items={filteredBullets()}
    onDrop={handleBulletDrop}
    loading={isLoading}
    emptyMessage="No bullets yet. Create sources and derive bullets to populate this board."
    defaultCollapsed="archived"
    sortItems={(a, b) => a.content.localeCompare(b.content)}
  >
    {#snippet filterBar()}
      <BulletFilterBar bind:filters onchange={() => {}} />
    {/snippet}

    {#snippet cardContent(bullet)}
      <BulletKanbanCard {bullet} onclick={() => openDetail(bullet.id)} />
    {/snippet}
  </GenericKanban>
{:else}
  <!-- Existing list view unchanged -->
  <!-- ... -->
{/if}
```

The same pattern applies to sources (with `SOURCE_COLUMNS` including `deriving` in the approved column), resumes (with `RESUME_COLUMNS`), and perspectives (with `BULLET_COLUMNS` reused or `UNIFIED_COLUMNS` shared).

**Key points [CRITICAL]:**
- The existing list view must remain completely unchanged. The `{:else}` branch renders the exact same markup as before.
- Column accent colors are hardcoded hex values that match the kanban status semantic colors (not theme tokens, because they are per-column accent overrides).
- The `onDrop` handler calls the SDK `update()` method and throws on failure to trigger optimistic revert.
- Filters apply to both list and board views.

**Acceptance criteria:**
- Each page renders a `ViewToggle` in the header.
- Default view is `list` (existing behavior preserved).
- Switching to `board` renders GenericKanban with correct columns.
- View mode persists in localStorage across page navigation.
- Drag-and-drop between columns calls the correct SDK update method.
- Filter bar above board filters visible cards.
- List view renders exactly as before (no regression).
- Sources kanban maps `deriving` items into the Approved column.

**Failure criteria:**
- List view broken or modified.
- View mode not persisting.
- Wrong columns for any entity.
- Drop handler not calling API.

---

### T43.15: Update `StatusBadge.svelte` [MINOR]

**File:** `packages/webui/src/lib/components/StatusBadge.svelte`

Add styling for `in_review` and `archived` statuses (if not already done in Phase 42).

Add to `colorMap`:
```typescript
in_review: '#f59e0b',   // or var(--color-warning) if Phase 42 is applied
archived: '#9ca3af',    // or var(--text-faint)
```

Add to `labelMap`:
```typescript
in_review: 'In Review',
archived: 'Archived',
```

**Acceptance criteria:**
- `in_review` status renders with warning/amber color and "In Review" label.
- `archived` status renders with faint/gray color and "Archived" label.

**Failure criteria:**
- Unknown status falls through to default gray.

---

### T43.16: Update Review Queue Dashboard [MINOR]

**File:** `packages/webui/src/routes/+page.svelte`

Update pending count labels from `pending_review` to `in_review`. Any query or filter that counts items with `pending_review` status must be updated to `in_review`.

**Acceptance criteria:**
- Dashboard shows correct "In Review" count.
- No references to `pending_review` remain on the dashboard.

**Failure criteria:**
- Counts show 0 because the query still filters on `pending_review`.

---

## Testing Support

### Test Fixtures

The existing `createTestDb()` helper in `packages/core/src/db/__tests__/helpers.ts` runs all migrations including the new `017_unified_kanban_statuses.sql`. No changes to `createTestDb()` are needed.

For tests that need entities with specific statuses, use the repository/service directly:

```typescript
// In test setup
const bulletRepo = new BulletRepository(db)
bulletRepo.create({ content: 'Draft bullet', status: 'draft', ... })
bulletRepo.create({ content: 'In review bullet', status: 'in_review', ... })
bulletRepo.create({ content: 'Archived bullet', status: 'archived', ... })

const sourceRepo = new SourceRepository(db)
sourceRepo.create({ title: 'Deriving source', status: 'deriving', ... })
```

### Unit Tests

**Migration integrity tests:**

| Test | Kind | Assertion |
|------|------|-----------|
| Bullets `pending_review` migrated to `in_review` | Unit | `SELECT count(*) FROM bullets WHERE status = 'pending_review'` returns 0 |
| Perspectives `pending_review` migrated to `in_review` | Unit | Same as above for perspectives |
| Resumes `final` migrated to `approved` | Unit | `SELECT count(*) FROM resumes WHERE status = 'final'` returns 0 |
| Sources retain original statuses | Unit | Existing `draft`/`approved`/`deriving` values preserved |
| Row count preserved (all 4 tables) | Unit | Count before = count after |
| Junction tables intact | Unit | `bullet_sources`, `bullet_technologies`, `perspective_skills`, `resume_perspectives`, `resume_sections`, `source_roles` etc. row counts preserved |
| New statuses insertable | Unit | `INSERT INTO bullets (..., status) VALUES (..., 'archived')` succeeds |
| Old statuses rejected | Unit | `INSERT INTO bullets (..., status) VALUES (..., 'pending_review')` fails CHECK constraint |

**Repository/service tests:**

| Test | Kind | Assertion |
|------|------|-----------|
| Bullet repo: `in_review` accepted | Unit | `create({ ..., status: 'in_review' })` succeeds |
| Bullet repo: `pending_review` rejected | Unit | Returns `VALIDATION_ERROR` |
| Bullet repo: `archived` accepted | Unit | `create({ ..., status: 'archived' })` succeeds |
| Source repo: `in_review` accepted | Unit | Succeeds |
| Source repo: `rejected` accepted | Unit | Succeeds |
| Source repo: `archived` accepted | Unit | Succeeds |
| Resume repo: `in_review` accepted | Unit | Succeeds |
| Resume repo: `approved` accepted | Unit | Succeeds |
| Resume repo: `final` rejected | Unit | Returns `VALIDATION_ERROR` |
| Perspective repo: `in_review` accepted | Unit | Succeeds |
| Perspective repo: `pending_review` rejected | Unit | Returns `VALIDATION_ERROR` |
| DerivationService: rejects archived sources | Unit | Returns error mentioning "archived" |
| DerivationService: rejects archived bullets | Unit | Returns error mentioning "archived" |
| ResumeService: rejects archived perspectives | Unit | Returns error mentioning "archived" |

### Integration Tests (API)

| Test | Kind | Route | Assertion |
|------|------|-------|-----------|
| PATCH bullet to `in_review` | Integration | `PATCH /api/bullets/:id` | 200, `status === 'in_review'` |
| PATCH bullet to `pending_review` | Integration | `PATCH /api/bullets/:id` | 400, `VALIDATION_ERROR` |
| PATCH bullet to `archived` | Integration | `PATCH /api/bullets/:id` | 200, `status === 'archived'` |
| PATCH source to `in_review` | Integration | `PATCH /api/sources/:id` | 200 |
| PATCH source to `archived` | Integration | `PATCH /api/sources/:id` | 200 |
| PATCH resume to `approved` | Integration | `PATCH /api/resumes/:id` | 200 |
| PATCH resume to `final` | Integration | `PATCH /api/resumes/:id` | 400 |
| PATCH perspective to `in_review` | Integration | `PATCH /api/perspectives/:id` | 200 |
| PATCH perspective to `pending_review` | Integration | `PATCH /api/perspectives/:id` | 400 |

### Component Smoke Tests

| Test | Kind | What to verify |
|------|------|---------------|
| GenericKanban renders 5 columns | Component | Columns labeled Draft, In Review, Approved, Rejected, Archived |
| Cards in correct columns | Component | Item with `status='draft'` in Draft column |
| Archived column collapsed by default | Component | Renders as thin vertical strip |
| Archived column expands on click | Component | Full column with cards visible |
| Drop calls onDrop with correct status | Component | Dropping into In Review calls `onDrop(id, 'in_review')` |
| Optimistic revert on failure | Component | Card returns to original column |
| Filter bar renders above board | Component | Filter bar visible |
| Filters reduce visible cards | Component | Client-side AND filtering works |
| ViewToggle renders two buttons | Component | "List" and "Board" buttons visible |
| ViewToggle active state | Component | Active button has primary background |
| ViewToggle persists to localStorage | Smoke | Mode survives page reload |
| Empty board shows message | Component | `emptyMessage` displayed when no items |
| Loading shows spinner | Component | Spinner visible during load |
| Sources: deriving in Approved column | Component | `deriving` status items appear in Approved column |
| Sources: deriving cards not draggable | Component | `cursor: not-allowed` on deriving cards |

### Contract Tests

| Test | Kind | What to verify |
|------|------|---------------|
| SDK `Bullet.status` type matches API | Contract | `forge.bullets.get(id)` returns `status` as new union value |
| SDK `Source.status` includes `deriving` | Contract | TypeScript accepts `deriving` |
| SDK `Resume.status` excludes `final` | Contract | TypeScript rejects `final` |

---

## Documentation Requirements

- No new documentation files required (non-goal).
- The spec file serves as the design document.
- This plan file serves as the implementation reference.
- Inline SQL comments in migration 017 explain the status remapping.
- TSDoc comments on the `UnifiedKanbanStatus` type:
  ```typescript
  /** Unified 5-status model for kanban boards. Used by bullets, sources (excluding deriving), resumes, perspectives. */
  export type UnifiedKanbanStatus = 'draft' | 'in_review' | 'approved' | 'rejected' | 'archived'
  ```
- TSDoc on column definition constants documenting the entity-specific semantics (e.g., sources `approved` column includes `deriving`).
- Inline comments in GenericKanban explaining the optimistic update pattern and intra-column reorder no-op.

---

## Parallelization Notes

**Within this phase:**
- T43.1 (migration), T43.2 (core types), T43.3 (SDK types), T43.4 (constants) must be committed together -- they are tightly coupled. The migration changes the DB schema, types and constants must match.
- T43.5 (repository updates) depends on T43.2 and T43.4 (types and constants).
- T43.6 (service updates) depends on T43.5 (repositories).
- T43.7 (route handlers) depends on T43.4 (constants).
- T43.8 (GenericKanban) and T43.9 (GenericKanbanColumn) can be developed in parallel with T43.1-T43.7 (no DB dependency for UI components).
- T43.10 (ViewToggle) and T43.11 (viewMode store) are independent of all other tasks. T43.11 creates `viewMode.svelte.ts` -- a prerequisite for the view toggle on all entity pages.
- T43.12 (card components) depends on T43.3 (SDK types for imports).
- T43.13 (filter bars) depends on T43.12 (card components used in preview).
- T43.14 (page integration) depends on T43.8, T43.9, T43.10, T43.11, T43.12, T43.13 (imports all). T43.14 integrates GenericKanban into the 4 entity pages (bullets, sources, resumes, perspectives).
- T43.15 (StatusBadge) is independent.
- T43.16 (dashboard) depends on T43.1 (migration, status rename).

**Recommended execution order:**
1. T43.1 + T43.2 + T43.3 + T43.4 (schema + types + constants -- foundational, commit together)
2. T43.5 + T43.7 (repository + route updates -- parallel)
3. T43.6 (service updates -- depends on repos)
4. T43.8 + T43.9 + T43.10 + T43.11 (GenericKanban + column + toggle + store -- parallel, no DB deps)
5. T43.12 (card components)
6. T43.13 (filter bars)
7. T43.14 (page integration -- final step)
8. T43.15 + T43.16 (StatusBadge + dashboard -- independent, can parallel with 4-7)

**Cross-phase:**
- This phase is independent of Phase 42 (Design System) at the functionality level, but benefits from it aesthetically (token vars vs hardcoded hex). The kanban card components use CSS `var()` with fallback values so they work with or without Phase 42.
- The existing org kanban (Phase 38) is untouched by this phase.
- Spec E3 (JD Kanban Pipeline) depends on this phase for the GenericKanban component.
- Migration 017 is reserved for this phase. Phase 47 (Clearance) uses 018.
- StatusBadge changes must apply AFTER Phase 42 tokenizes existing entries.
