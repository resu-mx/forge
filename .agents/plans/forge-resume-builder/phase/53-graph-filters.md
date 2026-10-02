# Phase 53: Graph Filters (Spec H4)

**Status:** Planning
**Date:** 2026-04-03
**Spec:** [2026-04-03-graph-filters.md](../refs/specs/2026-04-03-graph-filters.md)
**Depends on:** Phase 48 (Generic GraphView)
**Blocks:** None directly (works well with Phase 54 search and Phase 55 toolbar)
**Parallelizable with:** Phase 51, Phase 52, Phase 56 -- creates new files, modifies `GraphView.svelte` in distinct sections (filter `$effect` and `filterState` prop), modifies `graph.types.ts` (adds optional prop)

## Goal

Add a filter panel that lets users toggle node visibility by type, status, domain, and archetype. Filtered-out nodes are dimmed (not removed) to preserve graph topology. Filter state is persisted in URL query parameters for shareable/bookmarkable filtered views. The filter system integrates with the existing `nodeReducer`/`edgeReducer` pattern via an attribute-based approach that avoids stale Sigma reducer closures.

## Non-Goals

- Saved filter presets (users bookmark the URL)
- Filter by edge type (edges dim implicitly via endpoints)
- Advanced filter expressions (AND/OR logic, ranges, custom predicates)
- Filter animation (instant visual update)
- Multi-select dropdowns for domain/archetype (single-select only)

## Context

Phase 48's `GraphView` shows all nodes at full opacity. Dense graphs with 50+ nodes across multiple entity types need filtering. The spec uses an attribute-based approach: a separate `$effect` iterates all nodes and sets a `hidden` attribute directly on the graphology graph when filter state changes. The `nodeReducer` and `edgeReducer` read `data.hidden` and dim accordingly. This avoids the stale closure problem where Sigma captures reducer functions at init time and never sees updated filter state.

## Scope

| Spec section | Covered here? |
|-------------|---------------|
| 1. Filter State (model, logic, default state) | Yes |
| 2. Sigma Integration (attribute-based filtering, reducer extensions) | Yes |
| 3. Filter UI Panel (`GraphFilterPanel.svelte`) | Yes |
| 4. URL Query Parameter Persistence | Yes |
| 5. Component Interface (`filterState` prop on `GraphViewProps`) | Yes |
| 6. Testing | Yes |

## Files to Create

| File | Description |
|------|-------------|
| `packages/webui/src/lib/components/graph/graph.filters.ts` | `GraphFilterState`, `createDefaultFilterState()`, `nodePassesFilter()`, `filterToSearchParams()`, `searchParamsToFilter()` |
| `packages/webui/src/lib/components/graph/GraphFilterPanel.svelte` | Filter UI panel with checkboxes, dropdowns, clear button |
| `packages/webui/src/lib/components/graph/__tests__/graph-filters.test.ts` | Unit tests for filter logic and URL serialization (8 cases) |

## Files to Modify

| File | Description |
|------|-------------|
| `packages/webui/src/lib/components/graph/graph.types.ts` | Add optional `filterState` to `GraphViewProps` |
| `packages/webui/src/lib/components/graph/GraphView.svelte` | Add filter `$effect` that marks nodes hidden; integrate `data.hidden` into existing reducers |

## Fallback Strategies

- **Node missing `status`/`domain`/`archetype` field:** `nodePassesFilter` reads these via the `GraphNode` index signature. If the field is `undefined`, the node fails the filter when that category has active values. This is intentional -- nodes without metadata are excluded from filtered views.
- **Svelte 5 Set reactivity:** The filter panel always creates new `Set` objects on change (`new Set([...old, newValue])`), never mutates in place. This ensures Svelte 5 detects the change.
- **URL params with invalid values:** `searchParamsToFilter` passes values through without validation. Unknown values in the Set simply never match any nodes, effectively filtering everything out. No crash.
- **Empty graph (no nodes):** `createDefaultFilterState()` returns all-empty Sets (show all). The filter panel hides categories with no available values.

---

## Tasks

### T53.1: Write Filter State Module

**File:** `packages/webui/src/lib/components/graph/graph.filters.ts`

[IMPORTANT] Filter logic uses AND across categories, OR within each category. A node passes if it matches at least one active value in every category that has active values.

[IMPORTANT] Empty set means "show all" for that category. This is the key semantic -- an empty set is NOT "show none."

```typescript
import type { GraphNode } from './graph.types'

/**
 * Filter state for the graph. Each field is a set of active (visible) values.
 * A node passes the filter if it matches at least one active value in each
 * category that has any values set.
 *
 * Empty set = "show all" for that category (no filtering).
 */
export interface GraphFilterState {
  /** Visible node types. Empty = show all types. */
  nodeTypes: Set<string>
  /** Visible statuses. Empty = show all statuses. */
  statuses: Set<string>
  /** Visible domains. Empty = show all domains. */
  domains: Set<string>
  /** Visible archetypes. Empty = show all archetypes. */
  archetypes: Set<string>
}

/**
 * Default filter state: everything visible (all empty sets).
 */
export function createDefaultFilterState(): GraphFilterState {
  return {
    nodeTypes: new Set(),
    statuses: new Set(),
    domains: new Set(),
    archetypes: new Set(),
  }
}

/**
 * Determine if a node passes the current filter state.
 * Returns true if the node should be visible (full opacity).
 * Returns false if the node should be dimmed.
 *
 * Logic: AND across categories, OR within each category.
 * Empty category = pass (no filtering for that dimension).
 */
export function nodePassesFilter(
  node: GraphNode,
  filter: GraphFilterState
): boolean {
  if (filter.nodeTypes.size > 0 && !filter.nodeTypes.has(node.type)) {
    return false
  }
  if (filter.statuses.size > 0 && !filter.statuses.has(node.status as string)) {
    return false
  }
  if (filter.domains.size > 0 && !filter.domains.has(node.domain as string)) {
    return false
  }
  if (filter.archetypes.size > 0 && !filter.archetypes.has(node.archetype as string)) {
    return false
  }
  return true
}

/**
 * Serialize filter state to URL search params.
 * Empty sets are omitted (no filter = no param).
 */
export function filterToSearchParams(filter: GraphFilterState): URLSearchParams {
  const params = new URLSearchParams()
  if (filter.nodeTypes.size > 0) params.set('types', [...filter.nodeTypes].join(','))
  if (filter.statuses.size > 0) params.set('status', [...filter.statuses].join(','))
  if (filter.domains.size > 0) params.set('domain', [...filter.domains].join(','))
  if (filter.archetypes.size > 0) params.set('archetype', [...filter.archetypes].join(','))
  return params
}

/**
 * Deserialize URL search params to filter state.
 * Missing params default to empty sets (show all).
 */
export function searchParamsToFilter(params: URLSearchParams): GraphFilterState {
  return {
    nodeTypes: paramToSet(params.get('types')),
    statuses: paramToSet(params.get('status')),
    domains: paramToSet(params.get('domain')),
    archetypes: paramToSet(params.get('archetype')),
  }
}

function paramToSet(value: string | null): Set<string> {
  if (!value) return new Set()
  return new Set(value.split(',').map(s => s.trim()).filter(Boolean))
}
```

**Acceptance criteria:**
- `createDefaultFilterState()` returns all empty Sets.
- `nodePassesFilter` returns `true` when all filter Sets are empty.
- `nodePassesFilter` returns `false` when a node's type is not in `nodeTypes`.
- AND logic: a node must pass ALL active categories.
- OR logic: within a category, matching any value passes.
- `filterToSearchParams` omits empty categories.
- `searchParamsToFilter` round-trips with `filterToSearchParams`.

**Failure criteria:**
- Empty set treated as "show none" instead of "show all."
- OR logic within category treated as AND.

---

### T53.2: Add `filterState` Prop to `GraphViewProps`

**File:** `packages/webui/src/lib/components/graph/graph.types.ts`

[MINOR] The import is type-only to avoid circular dependencies.

```typescript
// --- Add import at top of graph.types.ts ---
import type { GraphFilterState } from './graph.filters'

// --- Add to GraphViewProps interface ---
export interface GraphViewProps {
  // ... existing props ...
  /** Optional filter state. When provided, nodes failing the filter are dimmed. */
  filterState?: GraphFilterState
}
```

**Acceptance criteria:**
- `filterState` is an optional prop on `GraphViewProps`.
- Type import is type-only (`import type`).
- Existing props are unchanged.

**Failure criteria:**
- Circular dependency between `graph.types.ts` and `graph.filters.ts`.

---

### T53.3: Integrate Filter State into `GraphView.svelte`

**File:** `packages/webui/src/lib/components/graph/GraphView.svelte`

[CRITICAL] Use attribute-based filtering (separate `$effect`), NOT filter logic inside the `nodeReducer` closure. Sigma captures reducer functions at init time, so reading `filterState` from a closure inside `nodeReducer` would read stale values. Instead, a separate `$effect` reacts to `filterState` changes and sets the `hidden` attribute on graphology nodes directly.

[IMPORTANT] Selection highlighting takes priority over filter dimming. If a node is both filtered-out and selected, the selection keeps it visible.

[GAP] The existing `edgeReducer` in Phase 48 already has `srcAttrs.hidden || tgtAttrs.hidden` handling (line ~542-545 of the Phase 48 plan). The filter `$effect` sets `hidden` on nodes, and the existing edge reducer reads it. No additional edge reducer changes are needed.

Add the `filterState` prop and a filter `$effect`:

```typescript
// --- Add to props destructuring ---
import { nodePassesFilter } from './graph.filters'
import type { GraphFilterState } from './graph.filters'

let {
  // ... existing props ...
  filterState = undefined,
}: GraphViewProps = $props()

// --- Add a new $effect for filter application ---

/**
 * Filter $effect: reacts to filterState changes, sets hidden attribute
 * on graphology nodes. The nodeReducer reads data.hidden and dims accordingly.
 * This avoids stale closures — the reducer does not read filterState directly.
 */
$effect(() => {
  if (!graph) return

  // When filterState is undefined (not provided or cleared), iterate all
  // nodes and set hidden = false to clear previously-applied filters.
  if (!filterState) {
    graph.forEachNode((nodeId) => {
      graph!.setNodeAttribute(nodeId, 'hidden', false)
    })
    sigmaInstance?.refresh()
    return
  }

  const _filter = filterState  // track reactive dependency

  // CRITICAL: Pass `{ ...attrs, type: attrs.nodeType ?? attrs.type }` to
  // `nodePassesFilter`. The graphology `type` attribute is `'circle'` (Sigma
  // node program), not the entity type. The entity classification is stored
  // as `nodeType` on graphology nodes (set in Phase 48's node construction).
  graph.forEachNode((nodeId, attrs) => {
    const passes = nodePassesFilter(
      { ...attrs, type: attrs.nodeType ?? attrs.type } as GraphNode,
      _filter,
    )
    graph!.setNodeAttribute(nodeId, 'hidden', !passes)
  })

  sigmaInstance?.refresh()
})
```

The existing `nodeReducer` already handles `data.hidden` via the selection priority check. After the selection branch, add the filter dimming check:

```typescript
// --- In the nodeReducer (after selection and edge-hover branches) ---
// Filter dimming via attribute (set by $effect above)
if (data.hidden) {
  return {
    ...data,
    color: DIM_NODE_COLOR,
    label: '',
    zIndex: Z_BACKGROUND,
  }
}

return data
```

**Acceptance criteria:**
- Filter `$effect` runs when `filterState` changes.
- Nodes failing the filter get `hidden: true` attribute on graphology.
- `nodeReducer` reads `data.hidden` and dims with `DIM_NODE_COLOR`.
- Selection takes priority over filter dimming (selected + filtered node stays visible).
- `sigmaInstance.refresh()` is called after filter changes.
- When `filterState` is `undefined` (not provided), the `$effect` returns early (no filtering).

**Failure criteria:**
- Filter state read from closure inside `nodeReducer` (stale values).
- Filtered-out selected node gets dimmed (priority violation).
- No `sigma.refresh()` after attribute changes (visual not updated).

---

### T53.4: Write `GraphFilterPanel.svelte`

**File:** `packages/webui/src/lib/components/graph/GraphFilterPanel.svelte`

[IMPORTANT] Svelte 5 does not track mutations to `Set` objects. Always reassign: `filter.nodeTypes = new Set([...filter.nodeTypes, newType])`. Never use `.add()` or `.delete()` in place.

[MINOR] Categories with no available values are hidden from the panel (e.g., if no nodes have an `archetype` field, the archetype section is not rendered).

```svelte
<!--
  GraphFilterPanel.svelte — Collapsible filter sidebar for graph views.
  Renders checkbox groups for node types and statuses, dropdowns for
  domains and archetypes. Persists filter state via callback.
-->
<script lang="ts">
  import type { GraphFilterState } from './graph.filters'
  import { createDefaultFilterState } from './graph.filters'

  interface GraphFilterPanelProps {
    filterState: GraphFilterState
    availableTypes: string[]
    availableStatuses: string[]
    availableDomains: string[]
    availableArchetypes: string[]
    onFilterChange: (newState: GraphFilterState) => void
  }

  let {
    filterState,
    availableTypes,
    availableStatuses,
    availableDomains,
    availableArchetypes,
    onFilterChange,
  }: GraphFilterPanelProps = $props()

  let collapsed = $state(false)

  function toggleType(type: string) {
    const next = new Set(filterState.nodeTypes)
    if (next.has(type)) {
      next.delete(type)
    } else {
      next.add(type)
    }
    onFilterChange({ ...filterState, nodeTypes: next })
  }

  function toggleStatus(status: string) {
    const next = new Set(filterState.statuses)
    if (next.has(status)) {
      next.delete(status)
    } else {
      next.add(status)
    }
    onFilterChange({ ...filterState, statuses: next })
  }

  function selectDomain(event: Event) {
    const value = (event.target as HTMLSelectElement).value
    if (value) {
      onFilterChange({ ...filterState, domains: new Set([value]) })
    } else {
      onFilterChange({ ...filterState, domains: new Set() })
    }
  }

  function selectArchetype(event: Event) {
    const value = (event.target as HTMLSelectElement).value
    if (value) {
      onFilterChange({ ...filterState, archetypes: new Set([value]) })
    } else {
      onFilterChange({ ...filterState, archetypes: new Set() })
    }
  }

  function clearAll() {
    onFilterChange(createDefaultFilterState())
  }

  let hasActiveFilters = $derived(
    filterState.nodeTypes.size > 0
    || filterState.statuses.size > 0
    || filterState.domains.size > 0
    || filterState.archetypes.size > 0
  )
</script>

<div class="filter-panel" class:collapsed>
  <button class="filter-header" onclick={() => collapsed = !collapsed}>
    <span>Filters</span>
    <span class="toggle-icon">{collapsed ? '+' : '-'}</span>
  </button>

  {#if !collapsed}
    <div class="filter-body">
      {#if availableTypes.length > 0}
        <fieldset class="filter-group">
          <legend>Node Type</legend>
          {#each availableTypes as type}
            <label class="filter-checkbox">
              <input
                type="checkbox"
                checked={filterState.nodeTypes.size === 0 || filterState.nodeTypes.has(type)}
                onchange={() => toggleType(type)}
              />
              {type}
            </label>
          {/each}
        </fieldset>
      {/if}

      {#if availableStatuses.length > 0}
        <fieldset class="filter-group">
          <legend>Status</legend>
          {#each availableStatuses as status}
            <label class="filter-checkbox">
              <input
                type="checkbox"
                checked={filterState.statuses.size === 0 || filterState.statuses.has(status)}
                onchange={() => toggleStatus(status)}
              />
              {status}
            </label>
          {/each}
        </fieldset>
      {/if}

      {#if availableDomains.length > 0}
        <fieldset class="filter-group">
          <legend>Domain</legend>
          <select onchange={selectDomain}>
            <option value="">All domains</option>
            {#each availableDomains as domain}
              <option
                value={domain}
                selected={filterState.domains.has(domain)}
              >{domain}</option>
            {/each}
          </select>
        </fieldset>
      {/if}

      {#if availableArchetypes.length > 0}
        <fieldset class="filter-group">
          <legend>Archetype</legend>
          <select onchange={selectArchetype}>
            <option value="">All archetypes</option>
            {#each availableArchetypes as archetype}
              <option
                value={archetype}
                selected={filterState.archetypes.has(archetype)}
              >{archetype}</option>
            {/each}
          </select>
        </fieldset>
      {/if}

      {#if hasActiveFilters}
        <button class="clear-btn" onclick={clearAll}>
          Clear All Filters
        </button>
      {/if}
    </div>
  {/if}
</div>

<style>
  .filter-panel {
    background: rgba(255, 255, 255, 0.95);
    border: 1px solid var(--color-border, #e5e7eb);
    border-radius: 8px;
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.1);
    backdrop-filter: blur(4px);
    min-width: 180px;
    font-size: 0.85rem;
  }

  .filter-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
    padding: 8px 12px;
    border: none;
    background: none;
    cursor: pointer;
    font-weight: 600;
    font-size: 0.85rem;
  }

  .toggle-icon {
    font-size: 1rem;
    color: var(--color-muted, #9ca3af);
  }

  .filter-body {
    padding: 4px 12px 12px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .filter-group {
    border: none;
    padding: 0;
    margin: 0;
  }

  .filter-group legend {
    font-weight: 500;
    font-size: 0.8rem;
    color: var(--color-muted, #6b7280);
    margin-bottom: 4px;
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }

  .filter-checkbox {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 2px 0;
    cursor: pointer;
  }

  .filter-checkbox input {
    margin: 0;
  }

  .filter-group select {
    width: 100%;
    padding: 4px 8px;
    border: 1px solid var(--color-border, #e5e7eb);
    border-radius: 4px;
    font-size: 0.85rem;
    background: white;
  }

  .clear-btn {
    padding: 6px 12px;
    border: 1px solid var(--color-border, #e5e7eb);
    border-radius: 4px;
    background: white;
    cursor: pointer;
    font-size: 0.8rem;
    color: var(--color-muted, #6b7280);
    transition: background-color 0.15s;
  }

  .clear-btn:hover {
    background: var(--color-hover, #f3f4f6);
  }
</style>
```

**Acceptance criteria:**
- Panel renders checkbox groups for node types and statuses.
- Panel renders dropdown selects for domains and archetypes.
- Checking a checkbox toggles the value in the filter state.
- "Clear All Filters" resets to `createDefaultFilterState()`.
- Categories with no available values are hidden.
- "Clear All Filters" button only appears when filters are active.
- Panel is collapsible (toggle button).

**Failure criteria:**
- Set mutations in place (`.add()`/`.delete()`) without reassignment (no reactivity).
- Filter changes not propagated via `onFilterChange` callback.

---

### T53.5: Write Filter Logic and URL Serialization Unit Tests

**File:** `packages/webui/src/lib/components/graph/__tests__/graph-filters.test.ts`

```typescript
import { describe, it, expect } from 'vitest'
import {
  createDefaultFilterState,
  nodePassesFilter,
  filterToSearchParams,
  searchParamsToFilter,
} from '../graph.filters'
import type { GraphNode } from '../graph.types'

describe('nodePassesFilter', () => {
  const node: GraphNode = {
    id: '1', label: 'Test', type: 'source',
    status: 'draft', domain: 'security', archetype: 'DevSecOps',
  }

  it('passes all filters when filter state is empty (default)', () => {
    const filter = createDefaultFilterState()
    expect(nodePassesFilter(node, filter)).toBe(true)
  })

  it('filters by node type', () => {
    const filter = createDefaultFilterState()
    filter.nodeTypes = new Set(['bullet'])
    expect(nodePassesFilter(node, filter)).toBe(false)
  })

  it('passes when node type is in the active set', () => {
    const filter = createDefaultFilterState()
    filter.nodeTypes = new Set(['source', 'bullet'])
    expect(nodePassesFilter(node, filter)).toBe(true)
  })

  it('applies AND logic across categories', () => {
    const filter = createDefaultFilterState()
    filter.nodeTypes = new Set(['source'])
    filter.statuses = new Set(['approved'])
    expect(nodePassesFilter(node, filter)).toBe(false)  // status mismatch
  })

  it('ignores categories with empty sets', () => {
    const filter = createDefaultFilterState()
    filter.nodeTypes = new Set(['source'])
    filter.statuses = new Set()  // empty = show all
    expect(nodePassesFilter(node, filter)).toBe(true)
  })

  it('fails node without status field when status filter is active', () => {
    const nodeNoStatus: GraphNode = { id: '2', label: 'No Status', type: 'bullet' }
    const filter = createDefaultFilterState()
    filter.statuses = new Set(['draft'])
    expect(nodePassesFilter(nodeNoStatus, filter)).toBe(false)
  })

  it('filters by domain', () => {
    const filter = createDefaultFilterState()
    filter.domains = new Set(['cloud'])
    expect(nodePassesFilter(node, filter)).toBe(false)  // node is 'security'
  })

  it('filters by archetype', () => {
    const filter = createDefaultFilterState()
    filter.archetypes = new Set(['DevSecOps'])
    expect(nodePassesFilter(node, filter)).toBe(true)
  })
})

describe('createDefaultFilterState', () => {
  it('returns all empty sets', () => {
    const state = createDefaultFilterState()
    expect(state.nodeTypes.size).toBe(0)
    expect(state.statuses.size).toBe(0)
    expect(state.domains.size).toBe(0)
    expect(state.archetypes.size).toBe(0)
  })

  it('returns a new object each call', () => {
    const a = createDefaultFilterState()
    const b = createDefaultFilterState()
    expect(a).not.toBe(b)
    expect(a.nodeTypes).not.toBe(b.nodeTypes)
  })
})

describe('URL serialization', () => {
  it('round-trips filter state through URL params', () => {
    const filter = createDefaultFilterState()
    filter.nodeTypes = new Set(['source', 'bullet'])
    filter.statuses = new Set(['draft'])

    const params = filterToSearchParams(filter)
    const restored = searchParamsToFilter(params)

    expect(restored.nodeTypes).toEqual(new Set(['source', 'bullet']))
    expect(restored.statuses).toEqual(new Set(['draft']))
    expect(restored.domains.size).toBe(0)
    expect(restored.archetypes.size).toBe(0)
  })

  it('omits empty categories from URL', () => {
    const filter = createDefaultFilterState()
    const params = filterToSearchParams(filter)
    expect(params.toString()).toBe('')
  })

  it('handles single value', () => {
    const filter = createDefaultFilterState()
    filter.domains = new Set(['security'])
    const params = filterToSearchParams(filter)
    expect(params.get('domain')).toBe('security')
  })

  it('handles whitespace in param values', () => {
    const params = new URLSearchParams('types= source , bullet ')
    const filter = searchParamsToFilter(params)
    expect(filter.nodeTypes).toEqual(new Set(['source', 'bullet']))
  })

  it('handles empty param value', () => {
    const params = new URLSearchParams('types=')
    const filter = searchParamsToFilter(params)
    expect(filter.nodeTypes.size).toBe(0)
  })
})
```

**Acceptance criteria:**
- All 15 test cases pass.
- Filter logic, default state, and URL serialization are verified.
- Edge cases (missing fields, whitespace, empty params) are covered.

**Failure criteria:**
- Any test fails, indicating filter logic or serialization bug.

---

## Testing Support

| Test file | Test count | Type |
|-----------|-----------|------|
| `__tests__/graph-filters.test.ts` | 15 | Unit |
| **Total** | **15** | |

**Run command:** `cd packages/webui && npx vitest run src/lib/components/graph/__tests__/graph-filters.test.ts`

## Documentation Requirements

- Export all public functions and the `GraphFilterState` interface from `graph.filters.ts`.
- Consumer usage example for URL integration with SvelteKit's `$page.url.searchParams` and `goto()`.
- No new user-facing docs (internal component module).

## Parallelization Notes

- T53.1 (new file `graph.filters.ts`) is independent -- can start immediately.
- T53.2 modifies `graph.types.ts` -- minor change, no conflict with other phases.
- T53.3 modifies `GraphView.svelte` -- adds a new `$effect` block (does not modify existing reducer code, just adds `data.hidden` check). Mergeable with Phase 51/52 changes.
- T53.4 (new file `GraphFilterPanel.svelte`) is independent.
- T53.5 (tests) depends on T53.1 only.
- This phase is parallelizable with Phases 51, 52, and 56. The `graph.types.ts` modification (T53.2) is a one-line addition that is unlikely to conflict.
