# DragNDrop View: Add Bullet to Role

**Date:** 2026-03-30
**Status:** Design
**Builds on:** Resume Renderer (Phases 19-20), Provenance Tooltip (Phase 24)

## Purpose

Enable users to add a bullet point to a specific role within the DragNDrop resume view. Currently the "+ Add Entry" button at the bottom of each section opens a picker showing ALL approved perspectives. Users need to add bullets scoped to a specific role — showing only perspectives derived from that role's source.

## Current State

- `ExperienceSubheading` in the IR has `source_id: string | null` — identifies which source (role) the subheading came from
- The DragNDrop view has `onAddEntry?: (section: string) => void` callback — passes only the section name, no source context
- The resumes page's `openPicker(section)` loads ALL approved perspectives via `forge.perspectives.list({ status: 'approved', limit: 500 })`
- The picker modal has archetype and domain filters but no source/role filter
- Perspectives have `bullet_id` → bullets have `source_id` (via `bullet_sources` junction) — the chain exists but the picker doesn't use it

## Goals

1. Add a "+ Add" button per role subheading in the DragNDrop view
2. When clicked, open the perspective picker pre-filtered to perspectives from that role's source
3. Keep the existing section-level "+ Add Entry" button (for adding perspectives from any source)
4. Filter happens server-side via API query param for efficiency

## Non-Goals

- Creating new bullets or perspectives from the DnD view (that's the Derivation workflow)
- Changing the perspective picker modal's visual design
- Adding bullet-level drag between roles (complex, deferred)

---

## 1. API: Filter Perspectives by Source

### 1.1 Current State

`GET /api/perspectives` supports these filters: `bullet_id`, `target_archetype`, `domain`, `framing`, `status`.

There is no `source_id` filter — you can't ask "give me perspectives whose bullet belongs to this source."

### 1.2 Add `source_id` Filter

The perspectives list endpoint should accept `?source_id=X` which filters to perspectives whose bullet is linked to the given source via `bullet_sources`:

```sql
SELECT DISTINCT p.* FROM perspectives p
JOIN bullet_sources bs ON bs.bullet_id = p.bullet_id
WHERE bs.source_id = ?
```

> A bullet can belong to multiple sources via `bullet_sources`. Without DISTINCT, perspectives on multi-source bullets would appear once per source link, producing duplicates.

This requires updating:
- `PerspectiveFilter` type — add `source_id?: string`
- `PerspectiveRepository.list()` — add conditional JOIN when `source_id` filter is present
- SDK `PerspectiveFilter` type — add `source_id?: string`

**Implementation guidance for `PerspectiveRepository.list()`:**

The existing `PerspectiveRepository.list()` method builds a simple `FROM perspectives WHERE ...` query. Adding `source_id` requires a conditional JOIN — the `bullet_sources` table is only JOINed when `source_id` is present in the filter. Implementation pattern:

```typescript
let joinClause = ''
// ... existing condition building ...

if (filter.source_id !== undefined) {
  joinClause = 'JOIN bullet_sources bs ON bs.bullet_id = perspectives.bullet_id'
  conditions.push('bs.source_id = ?')
  params.push(filter.source_id)
}

// Count query:
const countRow = db.query(
  `SELECT COUNT(DISTINCT perspectives.id) AS total FROM perspectives ${joinClause} ${where}`
).get(...params)

// Data query:
const rows = db.query(
  `SELECT DISTINCT perspectives.* FROM perspectives ${joinClause} ${where} ORDER BY created_at DESC LIMIT ? OFFSET ?`
).all(...params, limit, offset)
```

> When `source_id` is present, use `DISTINCT` in both COUNT and SELECT to avoid duplicates from the JOIN.

#### Known Issues

**Pre-existing inconsistency:** The SDK's `PerspectiveFilter` uses `archetype` while the core types and route use `target_archetype`. The SDK sends `?archetype=X` but the route reads `?target_archetype=X`, meaning archetype filtering via the SDK is silently broken. This spec does not fix this pre-existing issue — it is out of scope. A future fix should reconcile the field name in either the SDK or the route.

### 1.3 Alternative: Client-Side Filtering

If adding a server-side filter is too heavy, the picker could load all approved perspectives and filter client-side by matching `perspective.bullet_id` against bullets whose `sources` array includes the target source ID. However, this requires the bullet data to be available in the picker context. Server-side filtering is cleaner.

**Recommendation:** Server-side filter via `?source_id=X`. It's a single JOIN addition to an existing query.

---

## 2. DragNDrop View: Per-Role Add Button

### 2.1 Updated Callback Signature

The `onAddEntry` callback currently takes `(section: string)`. It needs to also accept an optional source ID:

```typescript
onAddEntry?: (section: string, sourceId?: string, sourceLabel?: string) => void
```

> `sub.source_id` is `string | null` but the callback expects `string | undefined`. Use `?? undefined` to convert.

When the per-role "+" button is clicked: `onAddEntry('experience', subheading.source_id ?? undefined, subheading.title)`
When the section-level "+" button is clicked: `onAddEntry('experience')` (no source filter — shows all)

### 2.2 Per-Role Button Placement

Inside each `ExperienceSubheading`, after the bullet list, add a small "+ Add" button:

```svelte
{#each group.subheadings as sub (sub.id)}
  <div class="subheading">
    <div class="subheading-header">
      <span class="role-title">{sub.title}</span>
      <span class="date-range">{sub.date_range}</span>
    </div>
    <!-- bullet list (existing) -->
    <div class="bullet-list" ...>
      {#each dndBullets[sub.id] ?? [] as bullet (bullet.id)}
        ...
      {/each}
    </div>
    <!-- Per-role add button (new) -->
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
{/each}
```

### 2.3 Button Styling

Small, unobtrusive, appears below each role's bullet list:

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

---

## 3. Resumes Page: Picker with Source Filter

### 3.1 Updated `openPicker` Signature

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

### 3.2 Picker Modal Header

When `sourceId` is present, show which role the picker is filtered to:

```svelte
<h3>
  {#if pickerModal.sourceId}
    Add bullet — {pickerModal.sourceLabel ?? 'filtered by source'}
  {:else}
    Add Entry to {SECTION_LABELS[pickerModal.section]}
  {/if}
</h3>
```

### 3.3 pickerModal State Update

```typescript
let pickerModal = $state({ open: false, section: '', sourceId: null as string | null, sourceLabel: null as string | null })
```

### 3.4 Post-Add Refresh

After successfully adding an entry, the picker must refresh its perspective list using the same `sourceId` filter. The current code at `+page.svelte:372` re-fetches with a hardcoded `{ status: 'approved' }` filter, ignoring `sourceId`. Update `addEntry` to use `pickerModal.sourceId`:

```typescript
async function addEntry(perspectiveId: string) {
  // ... existing add logic ...
  if (result.ok) {
    // Refresh with the same source filter
    const refreshFilter: Record<string, unknown> = { status: 'approved', limit: 500 }
    if (pickerModal.sourceId) {
      refreshFilter.source_id = pickerModal.sourceId
    }
    const listResult = await forge.perspectives.list(refreshFilter)
    if (listResult.ok) {
      availablePerspectives = listResult.data
    }
    // Also refresh IR so DragNDrop view shows the new bullet
    await Promise.all([
      loadResumeDetail(selectedResumeId!),
      loadIR(selectedResumeId!),
    ])
  }
}
```

### 3.5 `closePicker` Must Reset `sourceId`

```typescript
function closePicker() {
  pickerModal = { open: false, section: '', sourceId: null, sourceLabel: null }
  availablePerspectives = []
}
```

---

## 4. Acceptance Criteria

### API
- [ ] `GET /api/perspectives?source_id=X` returns only perspectives whose bullet is linked to source X
- [ ] Filter works alongside existing filters (status, archetype, domain)
- [ ] Empty result when source has no perspectives

### DragNDrop View
- [ ] Each role subheading has a "+ Add from this role" button
- [ ] Button only appears when `source_id` is non-null
- [ ] Clicking the button calls `onAddEntry('experience', source_id ?? undefined, title)`
- [ ] Section-level "+ Add Entry" still works (no source filter)

### Picker
- [ ] When opened with `sourceId`, shows only perspectives from that source
- [ ] Picker header indicates source-filtered mode
- [ ] Archetype and domain filters still work within the source-filtered set
- [ ] Already-included perspectives are excluded from the picker list

### Duplicate Prevention
- [ ] `GET /api/perspectives?source_id=X` returns each perspective exactly once, even for multi-source bullets
- [ ] `closePicker` resets `sourceId` to null — reopening via section button shows all perspectives

### TypeScript
- [ ] `sub.source_id ?? undefined` used in onclick to satisfy `string | undefined` callback type
- [ ] Build passes without TypeScript errors

### IR Refresh
- [ ] After adding entry via source-filtered picker, DragNDrop view updates to show the new bullet

### Picker Header
- [ ] Source-filtered picker header shows role title (e.g., "Add bullet — Senior Engineer")

### Tests
- [ ] PerspectiveRepository: `list({ source_id })` returns correct filtered results
- [ ] PerspectiveRepository: `list({ source_id, status: 'approved' })` combines filters correctly
- [ ] Route: `GET /api/perspectives?source_id=X&status=approved` returns 200 with filtered data
- [ ] Repository: `list({ source_id })` with bullet linked to 2 sources returns perspective exactly once (DISTINCT)
- [ ] Repository: `list({ source_id })` for unknown source returns empty data, not error
- [ ] Repository: `list({ source_id, status: 'approved', target_archetype: 'X' })` combines all three filters
- [ ] Route: `GET /api/perspectives?source_id=X` with unknown X returns 200 with empty data
- [ ] Picker: open with sourceId, close, reopen without sourceId — full list loads (sourceId reset verified)

---

## 5. Dependencies & Parallelization

### Sequential
1. API filter (`source_id` on perspectives) — must be first
2. DragNDrop button + resumes page picker update — after API

### Parallel (after API filter)
- DragNDrop button (DragNDropView.svelte)
- Picker update (resumes/+page.svelte)

### Files to Modify
- `packages/core/src/types/index.ts` — add `source_id` to `PerspectiveFilter`
- `packages/core/src/db/repositories/perspective-repository.ts` — add `source_id` JOIN filter to `list()`
- `packages/core/src/routes/perspectives.ts` — pass `source_id` query param to service
- `packages/sdk/src/types.ts` — add `source_id` to `PerspectiveFilter`
- `packages/webui/src/lib/components/resume/DragNDropView.svelte` — per-role button, updated callback signature
- `packages/webui/src/routes/resumes/+page.svelte` — updated `openPicker`, `pickerModal` state, picker header

### No New Dependencies
Uses existing perspective list endpoint with an additional filter parameter.
