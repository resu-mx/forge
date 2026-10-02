# Phase 26: DragNDrop Per-Role Bullet Addition

**Goal:** Enable adding bullets scoped to a specific role in the DragNDrop resume view, via a server-side `source_id` filter on the perspectives endpoint.

**Non-Goals:** Creating new bullets/perspectives from the DnD view (that is the Derivation workflow). Changing the picker modal's visual design. Adding bullet-level drag between roles. Fixing the pre-existing `archetype` vs `target_archetype` SDK/route inconsistency.

**Depends on:** Phase 24 (provenance tooltip -- enriched `source_chain` with `source_id` on subheadings)
**Blocks:** Nothing

**Internal task parallelization:** Not parallelizable internally -- T26.1 (API) must land before T26.2 and T26.3 (UI). T26.2 and T26.3 are parallelizable with each other (different files). T26.4 depends on all prior tasks. T26.5 can run in parallel with T26.4.

**Tech Stack:** TypeScript, Svelte 5 runes, SvelteKit, SQLite (via `bun:sqlite`), Hono, Bun test

**Reference:** `refs/specs/2026-03-30-dnd-role-bullet-add.md`

**Architecture:**
- Type changes in `packages/core/src/types/index.ts` and `packages/sdk/src/types.ts` (must stay in sync)
- Repository query change in `packages/core/src/db/repositories/perspective-repository.ts` (conditional JOIN when `source_id` is present)
- Route passthrough in `packages/core/src/routes/perspectives.ts`
- SDK plumbing is automatic -- `PerspectivesResource.list()` uses `toParams()` which serializes any filter key to a query param
- DragNDrop button in `packages/webui/src/lib/components/resume/DragNDropView.svelte`
- Picker state + filter in `packages/webui/src/routes/resumes/+page.svelte`

**Fallback Strategies:**
- If the `bullet_sources` JOIN causes a performance regression on large datasets, add an index: `CREATE INDEX IF NOT EXISTS idx_bullet_sources_source_id ON bullet_sources(source_id)` (should already have FK index, but verify)
- If source-filtered picker returns zero results (all perspectives already on resume), show a clear "No more perspectives from this role" message rather than the generic empty state
- If `sub.source_id` is null for legacy data, the per-role button simply does not render (guarded by `{#if onAddEntry && sub.source_id}`)

---

## Context

The DragNDrop resume view currently has a single "+ Add Entry" button at the bottom of each section. This opens a picker showing ALL approved perspectives regardless of which role they came from. Users need to add bullets scoped to a specific role -- e.g., "show me only the perspectives derived from my Principal Cloud Forensics Engineer role."

The data chain already exists: `ExperienceSubheading` has `source_id: string | null`, perspectives have `bullet_id`, and bullets link to sources via the `bullet_sources` junction table. The missing piece is a `source_id` query filter on the perspectives list endpoint and a per-role "+" button in the DnD view that passes the source context through.

### Current Flow
1. User clicks "+ Add Entry" on a section
2. `DragNDropView` calls `onAddEntry('experience')`
3. `+page.svelte` calls `openPicker('experience')` which fetches ALL approved perspectives
4. Picker shows the full list with archetype/domain client-side filters

### Target Flow
1. User clicks "+ Add from this role" on a specific subheading
2. `DragNDropView` calls `onAddEntry('experience', sourceId, roleTitle)`
3. `+page.svelte` calls `openPicker('experience', sourceId, roleTitle)` which fetches perspectives filtered by `source_id`
4. Picker shows only perspectives from that role, with header indicating the filter
5. Section-level "+ Add Entry" button still works (no source filter, shows all)

---

## Goals

1. Add `source_id` filter to `PerspectiveFilter` type (core + SDK)
2. Update `PerspectiveRepository.list()` with conditional JOIN on `bullet_sources`
3. Update the perspectives route to pass `source_id` query param
4. Add per-role "+ Add from this role" button in DragNDropView
5. Update picker modal to support source-filtered mode with header and post-add refresh

---

## Tasks

### Task 26.1: Add `source_id` filter to PerspectiveRepository + types

**Files to modify:**
- `packages/core/src/types/index.ts`
- `packages/sdk/src/types.ts`
- `packages/core/src/db/repositories/perspective-repository.ts`
- `packages/core/src/routes/perspectives.ts`

**Files to create:**
- None (tests added to existing test file)

**Goal:** Add `source_id` as a filter parameter that JOINs through `bullet_sources` to find perspectives whose bullet belongs to the given source.

#### Steps

- [ ] **Add `source_id` to `PerspectiveFilter` in `packages/core/src/types/index.ts`** (line ~466, after `status`):

```typescript
/** Filter options for listing perspectives. */
export interface PerspectiveFilter {
  bullet_id?: string
  target_archetype?: string
  domain?: string
  framing?: Framing
  status?: PerspectiveStatus
  source_id?: string
}
```

- [ ] **Add `source_id` to `PerspectiveFilter` in `packages/sdk/src/types.ts`** (line ~721, after `status`):

```typescript
export interface PerspectiveFilter {
  bullet_id?: string
  archetype?: string
  domain?: string
  framing?: string
  status?: string
  source_id?: string
}
```

> Note: The SDK uses `archetype` while core uses `target_archetype`. This is a pre-existing inconsistency (see spec Known Issues). Do not fix it in this phase.

- [ ] **Update `PerspectiveRepository.list()` in `packages/core/src/db/repositories/perspective-repository.ts`** (lines 185-231).

The current method builds a simple `FROM perspectives WHERE ...` query. When `source_id` is present, we need a conditional JOIN on `bullet_sources` and `DISTINCT` in both COUNT and SELECT to avoid duplicates from multi-source bullets.

Replace the existing `list` method body:

```typescript
  list(
    db: Database,
    filter: PerspectiveFilter = {},
    offset = 0,
    limit = 50,
  ): PaginatedResult<Perspective> {
    const conditions: string[] = []
    const params: unknown[] = []
    let joinClause = ''

    if (filter.bullet_id !== undefined) {
      conditions.push('perspectives.bullet_id = ?')
      params.push(filter.bullet_id)
    }
    if (filter.target_archetype !== undefined) {
      conditions.push('perspectives.target_archetype = ?')
      params.push(filter.target_archetype)
    }
    if (filter.domain !== undefined) {
      conditions.push('perspectives.domain = ?')
      params.push(filter.domain)
    }
    if (filter.framing !== undefined) {
      conditions.push('perspectives.framing = ?')
      params.push(filter.framing)
    }
    if (filter.status !== undefined) {
      conditions.push('perspectives.status = ?')
      params.push(filter.status)
    }
    if (filter.source_id !== undefined) {
      joinClause = 'JOIN bullet_sources bs ON bs.bullet_id = perspectives.bullet_id'
      conditions.push('bs.source_id = ?')
      params.push(filter.source_id)
    }

    const where = conditions.length > 0
      ? 'WHERE ' + conditions.join(' AND ')
      : ''

    // When JOIN is active, use COUNT(DISTINCT) and SELECT DISTINCT to avoid
    // duplicates from multi-source bullets. When no JOIN, plain COUNT(*) is simpler.
    const countExpr = joinClause ? 'COUNT(DISTINCT perspectives.id)' : 'COUNT(*)'
    const selectPrefix = joinClause ? 'SELECT DISTINCT perspectives.*' : 'SELECT perspectives.*'

    const countRow = db.query(
      `SELECT ${countExpr} AS total FROM perspectives ${joinClause} ${where}`
    ).get(...params) as { total: number }

    const rows = db.query(
      `${selectPrefix} FROM perspectives ${joinClause} ${where} ORDER BY perspectives.created_at DESC LIMIT ? OFFSET ?`
    ).all(...params, limit, offset) as PerspectiveRow[]

    return {
      data: rows.map(rowToPerspective),
      total: countRow.total,
    }
  },
```

Key changes from current implementation:
1. Added `joinClause` variable (empty string by default, populated when `source_id` filter is present)
2. Added `source_id` filter condition block that sets the JOIN and adds `bs.source_id = ?`
3. Table-qualified all column references (e.g., `perspectives.bullet_id` instead of `bullet_id`) so they are unambiguous when the JOIN is active
4. Added `DISTINCT` conditionally (only when JOIN is present) to both COUNT and SELECT
5. `ORDER BY` uses `perspectives.created_at` (table-qualified)

- [ ] **Update `packages/core/src/routes/perspectives.ts`** GET handler (line ~20, after the `status` check) to read the `source_id` query param:

```typescript
    if (c.req.query('status')) filter.status = c.req.query('status')!
    if (c.req.query('source_id')) filter.source_id = c.req.query('source_id')!
```

> The SDK's `PerspectivesResource.list()` uses a generic `toParams()` that serializes any filter object property to a query param. Adding `source_id` to `PerspectiveFilter` is sufficient -- no SDK resource code changes needed.

#### Acceptance Criteria
- [ ] `PerspectiveFilter` has `source_id?: string` in both core and SDK types
- [ ] `PerspectiveRepository.list({ source_id: 'X' })` returns only perspectives whose bullet is linked to source X via `bullet_sources`
- [ ] `PerspectiveRepository.list({ source_id: 'X', status: 'approved' })` combines both filters correctly
- [ ] Multi-source bullets: a perspective on a bullet linked to 2 sources returns exactly once (DISTINCT)
- [ ] Unknown source_id returns `{ data: [], total: 0 }` -- not an error
- [ ] Route: `GET /api/perspectives?source_id=X&status=approved` returns 200 with filtered data
- [ ] All existing perspective list tests still pass (column qualification does not break them)
- [ ] Build passes: `cd packages/core && bun run build` (if applicable) or type-check passes

---

### Task 26.2: Update DragNDropView -- per-role button + callback signature

**File to modify:** `packages/webui/src/lib/components/resume/DragNDropView.svelte`

**Goal:** Add a "+ Add from this role" button inside each experience subheading (after the bullet list) and update the `onAddEntry` callback signature to pass the source context.

#### Steps

- [ ] **Update `onAddEntry` prop type** (line 29) from `(section: string) => void` to `(section: string, sourceId?: string, sourceLabel?: string) => void`:

```typescript
  let {
    ir,
    resumeId,
    onUpdate,
    onAddEntry,
  }: {
    ir: ResumeDocument
    resumeId: string
    onUpdate: () => Promise<void>
    onAddEntry?: (section: string, sourceId?: string, sourceLabel?: string) => void
  } = $props()
```

- [ ] **Add per-role "+ Add from this role" button** after the bullet-list div, before the subheading closing div.

Find the `</div>` that closes `class="bullet-list"` and the `</div>` that closes `class="subheading"`. Add the button between them:

```svelte
                  </div>
                  <!-- Per-role add button -->
                  {#if onAddEntry && sub.source_id}
                    <button
                      class="btn btn-xs btn-add-role"
                      onclick={() => onAddEntry('experience', sub.source_id ?? undefined, sub.title)}
                      title="Add a bullet from {sub.title}"
                    >
                      + Add from this role
                    </button>
                  {/if}
                </div>
```

The `sub.source_id ?? undefined` conversion is required because `sub.source_id` is `string | null` but the callback parameter is `string | undefined`.

- [ ] **Add `.btn-add-role` CSS** in the `<style>` block:

```css
  .btn-add-role {
    margin-top: 0.25rem;
    padding: 0.2rem 0.5rem;
    font-size: 0.7rem;
    color: #9ca3af;
    background: transparent;
    border: 1px dashed #e5e7eb;
    border-radius: 4px;
    cursor: pointer;
    font-family: inherit;
  }
  .btn-add-role:hover {
    color: #6c63ff;
    border-color: #6c63ff;
  }
```

- [ ] **Keep existing section-level "+ Add Entry" button unchanged** (line ~313). It still calls `onAddEntry(section.type)` with no source filter.

#### Acceptance Criteria
- [ ] Each experience role subheading has a "+ Add from this role" button below its bullet list
- [ ] Button only appears when both `onAddEntry` is provided AND `sub.source_id` is non-null
- [ ] Clicking the button calls `onAddEntry('experience', sub.source_id ?? undefined, sub.title)`
- [ ] Section-level "+ Add Entry" button still calls `onAddEntry(section.type)` with no extra args
- [ ] Build passes: `cd packages/webui && bun run build`
- [ ] No TypeScript errors related to `string | null` vs `string | undefined`

---

### Task 26.3: Update resumes page -- picker with source filter

**File to modify:** `packages/webui/src/routes/resumes/+page.svelte`

**Goal:** Update the picker modal state, `openPicker`, `addEntry`, and `closePicker` functions to support source-filtered mode, and update the picker header to show which role is being filtered.

> **Note:** "Already-included perspectives excluded from picker" is handled by the existing `filteredPickerPerspectives` derived state -- it already filters out perspectives whose IDs appear in `resumeDetail.sections`. No new code needed for that.

#### Steps

- [ ] **Update `pickerModal` state** (line 70) to include `sourceId` and `sourceLabel`:

```typescript
  let pickerModal = $state({ open: false, section: '', sourceId: null as string | null, sourceLabel: null as string | null })
```

- [ ] **Update `openPicker` function signature and body** (lines 401-418) to accept optional `sourceId` and `sourceLabel`, and pass `source_id` in the API filter:

```typescript
  async function openPicker(section: string, sourceId?: string, sourceLabel?: string) {
    pickerModal = { open: true, section, sourceId: sourceId ?? null, sourceLabel: sourceLabel ?? null }
    pickerArchetypeFilter = ''
    pickerDomainFilter = ''
    pickerLoading = true
    try {
      const filter: Record<string, unknown> = { status: 'approved', limit: 500 }
      if (sourceId) {
        filter.source_id = sourceId
      }
      const result = await forge.perspectives.list(filter)
      if (result.ok) {
        availablePerspectives = result.data
      } else {
        addToast({ message: friendlyError(result.error), type: 'error' })
      }
    } catch (e) {
      addToast({ message: 'Failed to load perspectives', type: 'error' })
    } finally {
      pickerLoading = false
    }
  }
```

- [ ] **Update `addEntry` post-add refresh** (lines 367-375) to use `pickerModal.sourceId` when re-fetching the perspectives list, and also refresh the IR so the DnD view shows the new bullet:

```typescript
  async function addEntry(perspectiveId: string) {
    if (!selectedResumeId || !pickerModal.section) return
    const section = pickerModal.section
    const currentSection = resumeDetail?.sections[section] ?? []
    const position = currentSection.length

    try {
      const result = await forge.resumes.addEntry(selectedResumeId, {
        perspective_id: perspectiveId,
        section,
        position,
      })
      if (result.ok) {
        addToast({ message: 'Entry added', type: 'success' })
        // Refresh with the same source filter
        const refreshFilter: Record<string, unknown> = { status: 'approved', limit: 500 }
        if (pickerModal.sourceId) {
          refreshFilter.source_id = pickerModal.sourceId
        }
        const listResult = await forge.perspectives.list(refreshFilter)
        if (listResult.ok) {
          availablePerspectives = listResult.data
        }
        // Refresh resume detail and IR so DnD view updates
        await Promise.all([
          loadResumeDetail(selectedResumeId!),
          loadGapAnalysis(selectedResumeId!),
          loadIR(selectedResumeId!),
        ])
      } else {
        addToast({ message: friendlyError(result.error, 'Failed to add entry'), type: 'error' })
      }
    } catch (e) {
      addToast({ message: 'Failed to add entry', type: 'error' })
    }
  }
```

> Key changes from current: (1) refresh filter includes `source_id` when source-filtered, (2) `loadIR` is called alongside `loadResumeDetail` and `loadGapAnalysis` so the DnD view reflects the new bullet immediately. Note: `loadGapAnalysis` is added to `Promise.all` as an improvement over the spec's sequential example -- running all three refreshes in parallel is faster.

- [ ] **Update `closePicker`** (lines 420-423) to reset `sourceId` and `sourceLabel`:

```typescript
  function closePicker() {
    pickerModal = { open: false, section: '', sourceId: null, sourceLabel: null }
    availablePerspectives = []
  }
```

- [ ] **Update picker modal header** (line 735) to show the role title when source-filtered:

```svelte
        <h3>
          {#if pickerModal.sourceId}
            Add bullet &mdash; {pickerModal.sourceLabel ?? 'filtered by source'}
          {:else}
            Add Entry to {SECTION_LABELS[pickerModal.section]}
          {/if}
        </h3>
```

- [ ] **Update `onAddEntry` prop on `DragNDropView`** (line 644) -- no change needed here since `openPicker`'s new signature `(section: string, sourceId?: string, sourceLabel?: string)` is compatible with the new `onAddEntry` callback type `(section: string, sourceId?: string, sourceLabel?: string) => void`. TypeScript will accept `openPicker` as the callback because extra optional params are compatible.

#### Acceptance Criteria
- [ ] `pickerModal` state includes `sourceId` and `sourceLabel` fields
- [ ] When opened with `sourceId`, API call includes `source_id` in the filter
- [ ] Picker header shows "Add bullet -- {roleTitle}" when source-filtered
- [ ] Picker header shows "Add Entry to {sectionLabel}" when not source-filtered
- [ ] After adding an entry, refresh uses the same `sourceId` filter
- [ ] After adding an entry, `loadIR` is called so DnD view updates
- [ ] `closePicker` resets `sourceId` and `sourceLabel` to null
- [ ] Reopening via the section-level button after closing a source-filtered picker shows all perspectives (sourceId properly reset)
- [ ] Build passes: `cd packages/webui && bun run build`

---

### Task 26.4: Tests + verification

**Files to modify:**
- `packages/core/src/db/repositories/__tests__/perspective-repository.test.ts`

**Goal:** Add 6 repository unit tests for the `source_id` filter on `PerspectiveRepository.list()`. Route-level tests are manual smoke verification only.

#### Test fixture setup

The existing test file already imports `createTestDb`, `seedSource`, `seedBullet` from the test helpers. Each test in the new `describe` block needs:

1. **Two sources** -- to verify filtering returns only the targeted source's perspectives
2. **Two bullets** -- one linked to source A, one linked to source B (via `bullet_sources`)
3. **Two perspectives** -- one per bullet
4. Optionally, a bullet linked to BOTH sources (for the DISTINCT test)

```typescript
  // ── list with source_id filter ──────────────────────────────────────

  describe('list with source_id filter', () => {
    let sourceIdA: string
    let sourceIdB: string
    let bulletIdA: string
    let bulletIdB: string

    beforeEach(() => {
      // Source A: "Principal Cloud Forensics Engineer"
      sourceIdA = seedSource(db, { title: 'Principal Cloud Forensics Engineer' })
      // Source B: "Staff SRE"
      sourceIdB = seedSource(db, { title: 'Staff SRE' })
      // Bullet A linked to source A
      bulletIdA = seedBullet(db, [{ id: sourceIdA }], { content: 'Migrated ELK to OpenSearch' })
      // Bullet B linked to source B
      bulletIdB = seedBullet(db, [{ id: sourceIdB }], { content: 'Built SLO framework' })
    })
```

> Note: The outer `beforeEach` already creates `db`, `sourceId`, and `bulletId`. The inner `beforeEach` adds additional sources and bullets for source_id filter tests. The outer `sourceId` and `bulletId` are not used in these tests.

#### Test cases

```typescript
    test('filters perspectives by source_id', () => {
      PerspectiveRepository.create(db, makeInput({
        bullet_id: bulletIdA,
        content: 'Perspective from source A',
      }))
      PerspectiveRepository.create(db, makeInput({
        bullet_id: bulletIdB,
        content: 'Perspective from source B',
      }))

      const result = PerspectiveRepository.list(db, { source_id: sourceIdA })
      expect(result.data).toHaveLength(1)
      expect(result.total).toBe(1)
      expect(result.data[0].content).toBe('Perspective from source A')
    })

    test('combines source_id with status filter', () => {
      PerspectiveRepository.create(db, makeInput({
        bullet_id: bulletIdA,
        content: 'Approved from A',
        status: 'approved',
      }))
      PerspectiveRepository.create(db, makeInput({
        bullet_id: bulletIdA,
        content: 'Draft from A',
        status: 'draft',
      }))
      PerspectiveRepository.create(db, makeInput({
        bullet_id: bulletIdB,
        content: 'Approved from B',
        status: 'approved',
      }))

      const result = PerspectiveRepository.list(db, {
        source_id: sourceIdA,
        status: 'approved',
      })
      expect(result.data).toHaveLength(1)
      expect(result.total).toBe(1)
      expect(result.data[0].content).toBe('Approved from A')
    })

    test('combines source_id with status and target_archetype filters', () => {
      PerspectiveRepository.create(db, makeInput({
        bullet_id: bulletIdA,
        content: 'AI from A',
        status: 'approved',
        target_archetype: 'agentic-ai',
      }))
      PerspectiveRepository.create(db, makeInput({
        bullet_id: bulletIdA,
        content: 'Infra from A',
        status: 'approved',
        target_archetype: 'infrastructure',
      }))

      const result = PerspectiveRepository.list(db, {
        source_id: sourceIdA,
        status: 'approved',
        target_archetype: 'agentic-ai',
      })
      expect(result.data).toHaveLength(1)
      expect(result.data[0].content).toBe('AI from A')
    })

    test('multi-source bullet returns perspective exactly once (DISTINCT)', () => {
      // Create a bullet linked to BOTH sources
      const multiBulletId = seedBullet(db, [
        { id: sourceIdA, isPrimary: true },
        { id: sourceIdB, isPrimary: false },
      ], { content: 'Multi-source bullet' })

      PerspectiveRepository.create(db, makeInput({
        bullet_id: multiBulletId,
        content: 'Perspective on multi-source bullet',
      }))

      // Query by source A -- should get exactly 1, not duplicated
      const resultA = PerspectiveRepository.list(db, { source_id: sourceIdA })
      expect(resultA.data).toHaveLength(1)
      expect(resultA.total).toBe(1)

      // Query by source B -- should also get exactly 1
      const resultB = PerspectiveRepository.list(db, { source_id: sourceIdB })
      expect(resultB.data).toHaveLength(1)
      expect(resultB.total).toBe(1)
    })

    test('unknown source_id returns empty data, not error', () => {
      PerspectiveRepository.create(db, makeInput({
        bullet_id: bulletIdA,
        content: 'Some perspective',
      }))

      const result = PerspectiveRepository.list(db, { source_id: crypto.randomUUID() })
      expect(result.data).toHaveLength(0)
      expect(result.total).toBe(0)
    })

    test('pagination with source_id filter returns correct total and page', () => {
      PerspectiveRepository.create(db, makeInput({
        bullet_id: bulletIdA,
        content: 'First from A',
      }))
      PerspectiveRepository.create(db, makeInput({
        bullet_id: bulletIdA,
        content: 'Second from A',
      }))
      PerspectiveRepository.create(db, makeInput({
        bullet_id: bulletIdB,
        content: 'From B (excluded)',
      }))

      const result = PerspectiveRepository.list(db, { source_id: sourceIdA }, 1, 1)
      expect(result.data).toHaveLength(1)
      expect(result.total).toBe(2)
    })

    test('no source_id filter returns all perspectives (regression check)', () => {
      PerspectiveRepository.create(db, makeInput({
        bullet_id: bulletIdA,
        content: 'From A',
      }))
      PerspectiveRepository.create(db, makeInput({
        bullet_id: bulletIdB,
        content: 'From B',
      }))

      const result = PerspectiveRepository.list(db)
      expect(result.data).toHaveLength(2)
      expect(result.total).toBe(2)
    })
  })
```

#### Route-level verification (manual or integration test)

If an integration/route test file exists (`packages/core/src/routes/__tests__/perspectives.test.ts`), add:

```typescript
    test('GET /api/perspectives?source_id=X&status=approved returns filtered data', async () => {
      // Seed source, bullet, perspective as above
      const res = await app.request(`/api/perspectives?source_id=${sourceIdA}&status=approved`)
      expect(res.status).toBe(200)
      const body = await res.json()
      expect(body.data).toHaveLength(1)
    })

    test('GET /api/perspectives?source_id=unknown returns 200 with empty data', async () => {
      const res = await app.request(`/api/perspectives?source_id=${crypto.randomUUID()}`)
      expect(res.status).toBe(200)
      const body = await res.json()
      expect(body.data).toHaveLength(0)
    })
```

#### Run commands

```bash
# Repository + service tests
bun test packages/core/src/db/repositories/__tests__/perspective-repository.test.ts

# All core tests (regression)
bun test packages/core

# SDK tests (type check)
bun test packages/sdk

# WebUI build (type check for Svelte changes)
cd packages/webui && bun run build
```

#### Acceptance Criteria
- [ ] 6 new repository test cases pass
- [ ] All existing perspective tests still pass (no regressions from column qualification)
- [ ] WebUI builds without TypeScript errors
- [ ] SDK tests pass

---

### Task 26.5: Documentation

**Files to modify:**
- `packages/core/src/db/repositories/perspective-repository.ts` (JSDoc)

**Goal:** Add inline documentation for the updated `list()` method and note the pre-existing inconsistency.

#### Steps

- [ ] **Add JSDoc to `list()` method** explaining the conditional JOIN:

```typescript
  /**
   * List perspectives with optional filters and pagination.
   *
   * When `filter.source_id` is provided, a JOIN on `bullet_sources` is added
   * to find perspectives whose bullet is linked to the given source. DISTINCT
   * is used in this case to avoid duplicates when a bullet belongs to multiple
   * sources via the `bullet_sources` junction table.
   *
   * @note The SDK's PerspectiveFilter uses `archetype` while the core type and
   * route use `target_archetype`. This is a pre-existing inconsistency where
   * the SDK sends `?archetype=X` but the route reads `?target_archetype=X`,
   * meaning archetype filtering via the SDK is silently broken. This method
   * uses `target_archetype` consistently with the core type definition.
   */
```

#### Acceptance Criteria
- [ ] JSDoc on `list()` method explains the conditional JOIN and DISTINCT logic
- [ ] Pre-existing `archetype` vs `target_archetype` inconsistency is documented as a known issue

---

## Summary of All Files Changed

| File | Change |
|------|--------|
| `packages/core/src/types/index.ts` | Add `source_id?: string` to `PerspectiveFilter` |
| `packages/sdk/src/types.ts` | Add `source_id?: string` to `PerspectiveFilter` |
| `packages/core/src/db/repositories/perspective-repository.ts` | Conditional JOIN + DISTINCT in `list()`, JSDoc |
| `packages/core/src/routes/perspectives.ts` | Read `source_id` query param |
| `packages/webui/src/lib/components/resume/DragNDropView.svelte` | Updated callback signature, per-role button, CSS |
| `packages/webui/src/routes/resumes/+page.svelte` | `pickerModal` state, `openPicker`, `addEntry`, `closePicker`, header |
| `packages/core/src/db/repositories/__tests__/perspective-repository.test.ts` | 6 new test cases |

## No New Dependencies

All changes use existing libraries and database tables. The `bullet_sources` junction table already exists. The SDK's `toParams()` utility automatically serializes new filter properties.
