# Graph Filters

**Date:** 2026-04-03
**Spec:** H4 (Graph Filters)
**Phase:** TBD (next available)
**Depends on:** H1 (Generic GraphView Component)

## Overview

A graph with 50+ nodes across multiple entity types is overwhelming without filtering. This spec adds a filter panel that lets users toggle visibility by node type, status, domain, and archetype. Filtered-out nodes are dimmed (semi-transparent) rather than removed, preserving the graph's overall topology while drawing attention to the subset of interest.

The filter system uses Sigma.js's `nodeReducer` / `edgeReducer` pattern (already established in H1 for selection highlighting) and a `dimColorsRef` approach where filtered-out nodes receive muted colors instead of being hidden from the graph entirely. Filter state is persisted in URL query parameters so users can share or bookmark filtered views.

## Non-Goals

- **Saved filter presets:** No system for naming and saving filter combinations. Users bookmark the URL.
- **Filter by edge type:** Edges are filtered implicitly — an edge is dimmed when both its endpoints are dimmed.
- **Advanced filter expressions:** No AND/OR logic, no range filters, no custom predicates. Checkbox-based only.
- **Filter animation:** No animated transitions when filters change. Instant visual update.

---

## 1. Filter State

### 1.1 Filter Model

```typescript
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
 * Default filter state: everything visible.
 */
export function createDefaultFilterState(): GraphFilterState {
  return {
    nodeTypes: new Set(),
    statuses: new Set(),
    domains: new Set(),
    archetypes: new Set(),
  }
}
```

### 1.2 Filter Logic

A node passes the filter if it matches all active filter categories (AND across categories, OR within each category):

```typescript
/**
 * Determine if a node passes the current filter state.
 * Returns true if the node should be visible (full opacity).
 * Returns false if the node should be dimmed.
 */
export function nodePassesFilter(
  node: GraphNode,
  filter: GraphFilterState
): boolean {
  // If a category has active values, the node must match one of them
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
```

Nodes access `status`, `domain`, and `archetype` via the `GraphNode` index signature (`[key: string]: unknown`). Consumers are responsible for including these fields when constructing `GraphNode[]`.

---

## 2. Sigma Integration (Attribute-Based Filtering)

### 2.1 Approach: Attribute-Based, Not Reducer-Based

Instead of reading `filterState` inside `nodeReducer` closures (which capture stale references due to Sigma creating the reducer once at init time), use a separate `$effect` that iterates all nodes and sets the `hidden` attribute directly on the graphology graph. The `nodeReducer` then reads `data.hidden` and dims accordingly. This matches the existing `ChainViewModal` pattern and avoids stale closures.

```typescript
// Separate $effect that reacts to filterState changes and marks nodes hidden/visible:
$effect(() => {
  if (!graph) return
  graph.forEachNode((nodeId, attrs) => {
    const passes = nodePassesFilter(attrs as GraphNode, filterState)
    graph.setNodeAttribute(nodeId, 'hidden', !passes)
  })
  sigmaInstance?.refresh()
})
```

### 2.2 Node Reducer Reads `hidden` Attribute

The `nodeReducer` from H1 checks `data.hidden` (set by the `$effect` above). Selection highlighting takes priority -- if a node is both filtered-out and selected, the selection keeps it visible:

```typescript
nodeReducer: (node: string, data: Record<string, any>) => {
  // Selection highlighting takes priority over filtering
  if (selectedNodeId) {
    // ... existing H1 selection logic (unchanged) ...
  }

  // Filter dimming via attribute (set by $effect, not read from closure)
  if (data.hidden) {
    return {
      ...data,
      color: DIM_NODE_COLOR,
      label: '',
      zIndex: Z_BACKGROUND,
    }
  }

  return data
}
```

### 2.3 Edge Reducer Extension

Edges are dimmed when both endpoints are hidden. The `edgeReducer` reads the `hidden` attribute from node data (already set by the `$effect`):

```typescript
edgeReducer: (edge: string, data: Record<string, any>) => {
  const [src, tgt] = graph.extremities(edge)
  const srcAttrs = graph.getNodeAttributes(src)
  const tgtAttrs = graph.getNodeAttributes(tgt)

  if (srcAttrs.hidden && tgtAttrs.hidden) {
    return {
      ...data,
      color: DIM_EDGE_COLOR,
      size: DIM_EDGE_SIZE,
      zIndex: Z_BACKGROUND,
    }
  }

  // ... existing H1 selection/hover logic ...
  return data
}
```

---

## 3. Filter UI Panel

### 3.1 Component: `GraphFilterPanel.svelte`

A collapsible sidebar panel that renders filter controls. It sits beside (or overlays) the `GraphView` component.

```typescript
// Props
interface GraphFilterPanelProps {
  /** Current filter state (bindable) */
  filterState: GraphFilterState
  /** Available values for each filter category, derived from graph data */
  availableTypes: string[]
  availableStatuses: string[]
  availableDomains: string[]
  availableArchetypes: string[]
  /** Callback when filter changes */
  onFilterChange: (newState: GraphFilterState) => void
}
```

### 3.2 UI Layout

```
+---------------------------+
| Filters            [X]    |  <- collapsible header
+---------------------------+
| Node Type                 |
| [x] Source                |
| [x] Bullet               |
| [x] Perspective          |
| [ ] Resume Entry          |
+---------------------------+
| Status                    |
| [x] Draft                 |
| [x] Approved             |
| [ ] Rejected             |
+---------------------------+
| Domain                    |
| [v] Select domain...     |  <- dropdown (too many for checkboxes)
+---------------------------+
| Archetype                 |
| [v] Select archetype...  |  <- dropdown
+---------------------------+
| [Clear All Filters]       |
+---------------------------+
```

- **Node Type and Status:** Checkbox groups (small fixed set of values).
- **Domain and Archetype:** Dropdown selects (potentially many values).
- **Clear All Filters:** Resets to `createDefaultFilterState()`.

> **Svelte 5 Set reactivity:** Svelte 5 does not track mutations to `Set` objects. Always reassign the Set on changes: `filter.nodeTypes = new Set([...filter.nodeTypes, newType])` -- never use `.add()` or `.delete()` in place, as those mutations will not trigger reactivity.

### 3.3 Available Values Extraction

The consumer derives available filter values from the graph data:

```typescript
// Consumer-side:
let availableTypes = $derived([...new Set(nodes.map(n => n.type))])
let availableStatuses = $derived([...new Set(
  nodes.map(n => n.status as string).filter(Boolean)
)])
let availableDomains = $derived([...new Set(
  nodes.map(n => n.domain as string).filter(Boolean)
)])
let availableArchetypes = $derived([...new Set(
  nodes.map(n => n.archetype as string).filter(Boolean)
)])
```

Filter categories with no available values are hidden from the panel (e.g., if no nodes have an `archetype` field, the archetype dropdown is not rendered).

> **Single-select dropdowns:** Domain and archetype dropdowns are single-select `<select>` elements. To filter by multiple values, the user applies one filter at a time (each selection replaces the previous). Multi-select is deferred to a future enhancement.

---

## 4. URL Query Parameter Persistence

### 4.1 Serialization

Filter state is serialized into URL query parameters so users can share filtered views:

```
?types=source,bullet&status=draft,approved&domain=security
```

```typescript
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

### 4.2 SvelteKit Integration

The graph page reads initial filter state from `$page.url.searchParams` and updates the URL (without navigation) when filters change:

```typescript
import { goto } from '$app/navigation'
import { page } from '$app/state'

let filterState = $state(searchParamsToFilter(page.url.searchParams))

function handleFilterChange(newState: GraphFilterState) {
  filterState = newState
  const params = filterToSearchParams(newState)
  const url = new URL(page.url)
  url.search = params.toString()
  goto(url.toString(), { replaceState: true, noScroll: true })
}
```

---

## 5. Component Interface

### 5.1 New Exports

```typescript
// graph.filters.ts
export interface GraphFilterState
export function createDefaultFilterState(): GraphFilterState
export function nodePassesFilter(node: GraphNode, filter: GraphFilterState): boolean
export function filterToSearchParams(filter: GraphFilterState): URLSearchParams
export function searchParamsToFilter(params: URLSearchParams): GraphFilterState
```

### 5.2 GraphView Changes

`GraphView.svelte` does not own filter state. The consumer passes filter-aware reducers or the component accepts an optional `filterState` prop that is integrated into its built-in reducers. The latter approach is chosen for simplicity:

```typescript
// Addition to GraphViewProps (H1 types)
export interface GraphViewProps {
  // ... existing props ...
  filterState?: GraphFilterState
}
```

When `filterState` is provided, the component's `nodeReducer` and `edgeReducer` apply filter dimming as described in Section 2.

---

## 6. Files to Create

| File | Description |
|------|-------------|
| `packages/webui/src/lib/components/graph/graph.filters.ts` | `GraphFilterState`, `nodePassesFilter`, `filterToSearchParams`, `searchParamsToFilter` |
| `packages/webui/src/lib/components/graph/GraphFilterPanel.svelte` | Filter UI panel component (checkboxes, dropdowns, clear button) |

## 7. Files to Modify

| File | Description |
|------|-------------|
| `packages/webui/src/lib/components/graph/graph.types.ts` | Add optional `filterState` to `GraphViewProps` |
| `packages/webui/src/lib/components/graph/GraphView.svelte` | Integrate filter state into `nodeReducer` and `edgeReducer` |

---

## 8. Testing Approach

### 8.1 Unit Tests

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
})
```

### 8.2 Component Tests

**File:** `packages/webui/src/lib/components/graph/__tests__/GraphFilterPanel.test.ts`

- Renders checkbox groups for node types and statuses.
- Checking a checkbox adds the value to the filter state.
- Unchecking removes it.
- "Clear All Filters" resets to default state.
- Dropdown renders for domains and archetypes.
- Categories with no available values are hidden.

---

## 9. Acceptance Criteria

### Filter logic
- [ ] Default filter state (all empty sets) shows all nodes at full opacity
- [ ] Setting `nodeTypes` to `['source']` dims all non-source nodes
- [ ] Setting `statuses` to `['draft', 'approved']` dims nodes with other statuses
- [ ] Filter categories use AND logic (node must pass all active categories)
- [ ] Within a category, values use OR logic (node matches any active value)
- [ ] Nodes without a `status`/`domain`/`archetype` field are dimmed when that category has active values

### Dim rendering
- [ ] Filtered-out nodes render in `DIM_NODE_COLOR` with hidden labels
- [ ] Edges between two filtered-out nodes render in `DIM_EDGE_COLOR`
- [ ] Edges with at least one passing endpoint remain at full opacity
- [ ] Selection highlighting takes precedence over filter dimming

### Filter panel UI
- [ ] Panel is collapsible (toggle button)
- [ ] Node type and status categories render as checkbox groups
- [ ] Domain and archetype categories render as dropdown selects
- [ ] "Clear All Filters" button resets all categories
- [ ] Categories with no available values are hidden from the panel

### URL persistence
- [ ] Filter state is serialized to URL query params on change
- [ ] URL updates use `replaceState` (no browser history entry per filter toggle)
- [ ] Page load reads initial filter state from URL
- [ ] Empty filter state produces no query params

### Tests
- [ ] `nodePassesFilter` unit tests pass (5 cases)
- [ ] URL serialization round-trip tests pass (3 cases)
- [ ] Filter panel component tests pass

---

## 10. Dependencies

- **Runtime:** None beyond H1's dependencies
- **Spec dependencies:** H1 (Generic GraphView Component)
- **Blocked by:** Nothing beyond H1
- **Blocks:** No other specs directly, but works well with H5 (Graph Search) and H6 (Graph Toolbar)
