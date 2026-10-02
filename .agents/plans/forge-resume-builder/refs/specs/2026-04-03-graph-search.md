# Graph Search

**Date:** 2026-04-03
**Spec:** H5 (Graph Search)
**Phase:** TBD (next available)
**Depends on:** H1 (Generic GraphView Component), H3 (Node Labels & Display)

## Overview

Navigating a large graph visually is slow. Users need to jump directly to a known entity — "where is the Raytheon source?" or "find the DevSecOps perspective." This spec adds a search bar with autocomplete that matches against node labels, slugs (from H3), and content snippets. Selecting a result animates the camera to center on the matching node and highlights it.

The search is client-side only, operating over the in-memory `GraphNode[]` array. No API calls. Fuzzy matching keeps the interaction forgiving — typing "rayth" matches "Raytheon PCFE Migration" and "src:raytheon-pcfe".

## Non-Goals

- **Full-text search across all database fields:** Search operates only on fields present in the `GraphNode` objects already loaded into the graph.
- **Search history:** No persistence of previous searches.
- **Search across multiple graphs:** Search operates on the currently rendered graph only.
- **Regex or advanced query syntax:** Simple substring/fuzzy match only.
- **Server-side search:** All matching happens in the browser over the loaded node set.

---

## 1. Search Index

### 1.1 Searchable Fields

Each node contributes three searchable text fields:

1. **label** — The full original label (e.g., "Raytheon PCFE Migration Project")
2. **slug** — The auto-generated shortname from H3 (e.g., "src:raytheon-pcfe")
3. **content** — An optional text snippet from the node's metadata (e.g., a bullet's text, a source's description). Passed via the `GraphNode` index signature as `searchContent`.

### 1.2 Index Construction

The search index is built from the current `nodes` array. It is rebuilt whenever nodes change. No external library is needed — the node count is small enough (<500) for linear scan with early exit.

```typescript
/**
 * A searchable entry in the graph search index.
 */
export interface SearchEntry {
  nodeId: string
  label: string
  slug: string
  content: string
  type: string
}

/**
 * Build a search index from the graphology Graph object.
 *
 * IMPORTANT: `buildSearchIndex` must accept the graphology `Graph` object and
 * read slugs from node attributes: slugs are computed during graph construction
 * (H3) and stored as graphology attributes, NOT on the original `GraphNode[]`
 * props. Reading from the raw props would yield undefined slugs.
 */
export function buildSearchIndex(graph: Graph): SearchEntry[] {
  const entries: SearchEntry[] = []
  graph.forEachNode((nodeId, attrs) => {
    entries.push({
      nodeId,
      label: (attrs.fullLabel as string) ?? (attrs.label as string) ?? '',
      slug: (attrs.slug as string) ?? '',
      content: (attrs.searchContent as string) ?? '',
      type: (attrs.nodeType as string) ?? (attrs.type as string) ?? '',
    })
  })
  return entries
}
```

---

## 2. Fuzzy Matching

### 2.1 Algorithm

Case-insensitive substring matching with a simple scoring system:

```typescript
/**
 * Match a query against a search entry.
 * Returns a score (0 = no match, higher = better match).
 *
 * Scoring:
 * - Exact match on slug: 100
 * - Starts-with on label: 80
 * - Starts-with on slug: 70
 * - Contains in label: 50
 * - Contains in slug: 40
 * - Contains in content: 20
 * - No match: 0
 */
export function scoreMatch(query: string, entry: SearchEntry): number {
  const q = query.toLowerCase().trim()
  if (q.length === 0) return 0

  const label = entry.label.toLowerCase()
  const slug = entry.slug.toLowerCase()
  const content = entry.content.toLowerCase()

  if (slug === q) return 100
  if (label.startsWith(q)) return 80
  if (slug.startsWith(q)) return 70
  if (label.includes(q)) return 50
  if (slug.includes(q)) return 40
  if (content.includes(q)) return 20
  return 0
}

/**
 * Search the index for matches, returning results sorted by score (descending).
 * Limits results to `maxResults`.
 */
export function searchNodes(
  query: string,
  index: SearchEntry[],
  maxResults = 10
): SearchResult[] {
  if (query.trim().length === 0) return []

  return index
    .map(entry => ({ entry, score: scoreMatch(query, entry) }))
    .filter(r => r.score > 0)
    .sort((a, b) => b.score - a.score)
    .slice(0, maxResults)
    .map(r => ({
      nodeId: r.entry.nodeId,
      label: r.entry.label,
      slug: r.entry.slug,
      type: r.entry.type,
      content: r.entry.content,
      score: r.score,
    }))
}

export interface SearchResult {
  nodeId: string
  label: string
  slug: string
  type: string
  content: string
  score: number
}
```

---

## 3. Search Bar Component

### 3.1 Component: `GraphSearchBar.svelte`

```typescript
interface GraphSearchBarProps {
  /** The graph nodes to search over */
  nodes: GraphNode[]
  /** Callback when user selects a result */
  onSelect: (nodeId: string) => void
  /** Placeholder text */
  placeholder?: string
}
```

### 3.2 UI Layout

```
+-------------------------------------------+
| [🔍] Search graph nodes...        |  <- input field
+-------------------------------------------+
| 📄 src:raytheon-pcfe                      |  <- autocomplete result
|    Raytheon PCFE Migration Project         |
+-------------------------------------------+
| 📝 blt:built-ai                           |
|    Built AI taxonomy pipeline for...       |
+-------------------------------------------+
| 🎯 psp:devsecops-sec                     |
|    DevSecOps Security Lead perspective     |
+-------------------------------------------+
```

Each result row shows:
- A type icon (derived from `TYPE_PREFIXES` in H3 or a simple emoji/icon map)
- The slug (primary identifier, monospace styling)
- The label or truncated content (secondary line, smaller text)

### 3.3 Keyboard Navigation

| Key | Action |
|-----|--------|
| `ArrowDown` | Move focus to next result |
| `ArrowUp` | Move focus to previous result |
| `Enter` | Select the focused result (triggers `onSelect`) |
| `Escape` | Close the autocomplete dropdown, clear focus |
| Any character | Update query, refresh results |

Focus management:
- The input field captures keyboard events.
- `ArrowDown`/`ArrowUp` move a `focusedIndex` state variable.
- The focused result is visually highlighted (background color change).
- `Enter` calls `onSelect(results[focusedIndex].nodeId)`.
- Clicking a result also calls `onSelect`.

### 3.4 Debouncing

Search runs on every keystroke (no debounce). With <500 nodes and simple string matching, the computation is trivially fast (<1ms). If performance becomes an issue with larger datasets, a 100ms debounce can be added.

---

## 4. Camera Animation on Select

When the user selects a search result, the graph view animates the camera to center on the selected node:

```typescript
// Consumer-side handler:
function handleSearchSelect(nodeId: string) {
  // Set selectedNodeId to trigger highlighting (H1 reducer)
  selectedNodeId = nodeId

  // Animate camera to the selected node
  if (sigmaInstance && graph?.hasNode(nodeId)) {
    const displayData = sigmaInstance.getNodeDisplayData(nodeId)
    if (displayData) {
      sigmaInstance.getCamera().animate(
        { x: displayData.x, y: displayData.y, ratio: 0.3 },
        { duration: 400 },
      )
    }
  }
}
```

This requires the consumer to have access to the Sigma instance and graph. The `GraphView` component exposes these via a `getContext` pattern or a bindable ref.

> **Svelte 5 component instance typing:** Svelte 5 component instance exports may not have auto-generated TypeScript types. Define an explicit interface for consumers:
> ```typescript
> interface GraphViewAPI {
>   focusNode(id: string): void
>   getSigma(): Sigma | null
>   getGraph(): Graph | null
> }
> ```
> Consumers type the `bind:this` ref accordingly: `let graphView: GraphViewAPI`.

```typescript
// Addition to GraphView.svelte:
// Expose sigma and graph for external control (search, toolbar)
export function getSigma(): Sigma | null { return sigmaInstance }
export function getGraph(): Graph | null { return graph }

/**
 * Animate camera to center on a specific node.
 * Returns false if the node doesn't exist.
 */
export function focusNode(nodeId: string, ratio = 0.3, duration = 400): boolean {
  if (!sigmaInstance || !graph?.hasNode(nodeId)) return false
  const displayData = sigmaInstance.getNodeDisplayData(nodeId)
  if (!displayData) return false
  sigmaInstance.getCamera().animate(
    { x: displayData.x, y: displayData.y, ratio },
    { duration },
  )
  return true
}
```

---

## 5. Component Interface

### 5.1 New Exports

```typescript
// graph.search.ts
export interface SearchEntry
export interface SearchResult
export function buildSearchIndex(graph: Graph): SearchEntry[]
export function scoreMatch(query: string, entry: SearchEntry): number
export function searchNodes(query: string, index: SearchEntry[], maxResults?: number): SearchResult[]
```

### 5.2 GraphView Additions

```typescript
// New exported methods on GraphView.svelte
export function getSigma(): Sigma | null
export function getGraph(): Graph | null
export function focusNode(nodeId: string, ratio?: number, duration?: number): boolean
```

---

## 6. Files to Create

| File | Description |
|------|-------------|
| `packages/webui/src/lib/components/graph/graph.search.ts` | Search index, scoring, and query functions |
| `packages/webui/src/lib/components/graph/GraphSearchBar.svelte` | Search bar with autocomplete dropdown and keyboard navigation |

## 7. Files to Modify

| File | Description |
|------|-------------|
| `packages/webui/src/lib/components/graph/GraphView.svelte` | Add `getSigma()`, `getGraph()`, `focusNode()` exported methods |

---

## 8. Testing Approach

### 8.1 Unit Tests

**File:** `packages/webui/src/lib/components/graph/__tests__/graph-search.test.ts`

```typescript
import { describe, it, expect } from 'vitest'
import { buildSearchIndex, scoreMatch, searchNodes } from '../graph.search'
import type { GraphNode } from '../graph.types'

const testNodes: GraphNode[] = [
  { id: '1', label: 'Raytheon PCFE Migration Project', type: 'source', slug: 'src:raytheon-pcfe', searchContent: 'Led cloud migration for radar systems' },
  { id: '2', label: 'Built AI taxonomy pipeline', type: 'bullet', slug: 'blt:built-ai', searchContent: 'Designed ML classification pipeline' },
  { id: '3', label: 'DevSecOps Security Lead', type: 'perspective', slug: 'psp:devsecops-sec', searchContent: 'Security-focused perspective' },
]

describe('scoreMatch', () => {
  const index = buildSearchIndex(testNodes)

  it('scores exact slug match highest', () => {
    expect(scoreMatch('src:raytheon-pcfe', index[0])).toBe(100)
  })

  it('scores label starts-with highly', () => {
    expect(scoreMatch('Raytheon', index[0])).toBe(80)
  })

  it('scores slug starts-with', () => {
    expect(scoreMatch('src:', index[0])).toBe(70)
  })

  it('scores label contains', () => {
    expect(scoreMatch('PCFE', index[0])).toBe(50)
  })

  it('scores content contains lowest', () => {
    expect(scoreMatch('radar', index[0])).toBe(20)
  })

  it('returns 0 for no match', () => {
    expect(scoreMatch('kubernetes', index[0])).toBe(0)
  })

  it('is case-insensitive', () => {
    expect(scoreMatch('raytheon', index[0])).toBe(80)
    expect(scoreMatch('SRC:RAYTHEON-PCFE', index[0])).toBe(100)
  })
})

describe('searchNodes', () => {
  const index = buildSearchIndex(testNodes)

  it('returns results sorted by score descending', () => {
    const results = searchNodes('src', index)
    // 'src:raytheon-pcfe' starts with 'src' -> score 70
    // Others don't match 'src'
    expect(results.length).toBeGreaterThan(0)
    expect(results[0].nodeId).toBe('1')
  })

  it('returns empty array for empty query', () => {
    expect(searchNodes('', index)).toEqual([])
    expect(searchNodes('   ', index)).toEqual([])
  })

  it('limits results to maxResults', () => {
    const results = searchNodes('e', index, 2)
    expect(results.length).toBeLessThanOrEqual(2)
  })

  it('returns multiple matches sorted by relevance', () => {
    // 'security' appears in node 3's label and content
    const results = searchNodes('security', index)
    expect(results[0].nodeId).toBe('3')  // label contains
  })
})
```

### 8.2 Component Tests

**File:** `packages/webui/src/lib/components/graph/__tests__/GraphSearchBar.test.ts`

- Renders an input field with placeholder text.
- Typing a query displays autocomplete results.
- Results show type icon, slug, and truncated label/content.
- Arrow keys move focus through results.
- Enter key triggers `onSelect` with the focused result's `nodeId`.
- Escape key closes the dropdown.
- Clicking a result triggers `onSelect`.
- Empty query shows no results.

---

## 9. Acceptance Criteria

### Search index
- [ ] Search index is built from the graphology `Graph` object (not the raw `GraphNode[]` props)
- [ ] Index includes `label`, `slug`, and `searchContent` fields for each node
- [ ] Index is rebuilt reactively via `$derived`: `let searchIndex = $derived(graph ? buildSearchIndex(graph) : [])` -- this re-runs whenever `graph` state changes

### Fuzzy matching
- [ ] Exact slug match scores highest (100)
- [ ] Label starts-with scores 80
- [ ] Slug starts-with scores 70
- [ ] Label contains scores 50
- [ ] Content contains scores 20
- [ ] No match returns 0
- [ ] Matching is case-insensitive
- [ ] Empty query returns no results

### Autocomplete UI
- [ ] Search bar renders above or beside the graph
- [ ] Typing displays a dropdown of matching results (max 10)
- [ ] Each result shows type icon + slug + label/content snippet
- [ ] Content is truncated to ~60 characters with ellipsis

### Keyboard navigation
- [ ] ArrowDown moves focus to next result
- [ ] ArrowUp moves focus to previous result
- [ ] Enter selects the focused result
- [ ] Escape closes the dropdown
- [ ] Focused result has visual highlight (background color)

### Camera animation
- [ ] Selecting a result animates the camera to center on the node
- [ ] Camera zoom ratio is 0.3 (close-up) after animation
- [ ] Animation duration is 400ms
- [ ] Selected node is highlighted (H1 selection system)

### GraphView API
- [ ] `focusNode(nodeId)` animates camera to the specified node
- [ ] `focusNode` returns `false` for non-existent nodes
- [ ] `getSigma()` returns the Sigma instance (or null before init)
- [ ] `getGraph()` returns the graphology Graph (or null before init)

### Tests
- [ ] `scoreMatch` unit tests pass (7 cases)
- [ ] `searchNodes` unit tests pass (4 cases)
- [ ] Search bar component tests pass

---

## 10. Dependencies

- **Runtime:** None beyond H1's dependencies
- **Spec dependencies:**
  - H1 (Generic GraphView Component) — search results trigger camera animation and selection
  - H3 (Node Labels & Display) — slug generation provides the `slug` field for search matching
- **Blocked by:** H1 and H3
- **Blocks:** No other specs directly
