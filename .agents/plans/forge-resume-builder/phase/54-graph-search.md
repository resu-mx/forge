# Phase 54: Graph Search (Spec H5)

**Status:** Planning
**Date:** 2026-04-03
**Spec:** [2026-04-03-graph-search.md](../refs/specs/2026-04-03-graph-search.md)
**Depends on:** Phase 48 (Generic GraphView), Phase 52 (Node Labels -- needs slugs from H3)
**Blocks:** Phase 55 (Graph Toolbar -- needs `getSigma()`/`getGraph()` exports)
**Parallelizable with:** Phase 51, Phase 53, Phase 56 -- creates new files only (`graph.search.ts`, `GraphSearchBar.svelte`), modifies `GraphView.svelte` (adds exported methods)

## Goal

Add a search bar with autocomplete that matches against node labels, slugs (from Phase 52/H3), and content snippets. Selecting a result animates the camera to center on the matching node and highlights it. Search is client-side only, operating over the in-memory graph data. Fuzzy matching (case-insensitive substring with scoring) keeps the interaction forgiving. The `GraphView` component exposes `getSigma()`, `getGraph()`, and `focusNode()` methods for search result navigation.

## Non-Goals

- Full-text search across database fields (search operates on loaded `GraphNode` data only)
- Search history
- Search across multiple graphs
- Regex or advanced query syntax
- Server-side search
- Debouncing (computation is trivially fast for <500 nodes)

## Context

Phase 52 generates slugs (`src:raytheon-pcfe`, `blt:built-ai`) and stores them as graphology node attributes. This phase builds a search index from the graphology `Graph` object, reading `fullLabel`, `slug`, and `searchContent` attributes. The search bar provides autocomplete results sorted by relevance score. Selecting a result calls `focusNode()` on the `GraphView` to animate the camera and highlight the node.

Phase 48's `GraphView` already has `getSigma()`, `getGraph()`, and `focusNode()` exported methods (added at the end of the Phase 48 plan). This phase adds the search logic and UI component that consumes those methods.

## Scope

| Spec section | Covered here? |
|-------------|---------------|
| 1. Search Index (searchable fields, index construction) | Yes |
| 2. Fuzzy Matching (scoring algorithm, search function) | Yes |
| 3. Search Bar Component (UI, keyboard navigation) | Yes |
| 4. Camera Animation on Select | Yes |
| 5. GraphView API additions (`getSigma`, `getGraph`, `focusNode`) | Yes (verify existing from Phase 48) |
| 6. Testing | Yes |

## Files to Create

| File | Description |
|------|-------------|
| `packages/webui/src/lib/components/graph/graph.search.ts` | `SearchEntry`, `SearchResult`, `buildSearchIndex()`, `scoreMatch()`, `searchNodes()` |
| `packages/webui/src/lib/components/graph/GraphSearchBar.svelte` | Search bar with autocomplete dropdown and keyboard navigation |
| `packages/webui/src/lib/components/graph/__tests__/graph-search.test.ts` | Unit tests for search index, scoring, and query (11 cases) |

## Files to Modify

| File | Description |
|------|-------------|
| `packages/webui/src/lib/components/graph/GraphView.svelte` | Verify/add `getSigma()`, `getGraph()`, `focusNode()` exports (may already exist from Phase 48) |

## Fallback Strategies

- **`buildSearchIndex` receives null graph:** Returns empty array. Search bar shows no results.
- **Node has no `slug` attribute:** Falls back to empty string for the slug field in the search entry. Slug matching returns 0 score but label/content matching still works.
- **`focusNode` called before Sigma init:** Returns `false`, no camera animation. The search bar can show a "node not found" state.
- **Very large graph (>500 nodes):** Linear scan is O(n) per query. For <500 nodes, this completes in <1ms. If performance becomes an issue, add a 100ms debounce on the input handler.
- **`sigma.getNodeDisplayData()` returns null:** `focusNode` returns `false`. The node may be hidden by a filter -- the search still selects it, but the camera does not animate to a hidden node.

---

## Tasks

### T54.1: Write Search Index and Matching Module

**File:** `packages/webui/src/lib/components/graph/graph.search.ts`

[CRITICAL] `buildSearchIndex` must accept the graphology `Graph` object and read slugs from node attributes (`graph.getNodeAttributes(nodeId).slug`). Slugs are computed during graph construction in Phase 52 and stored as graphology attributes -- they do NOT exist on the original `GraphNode[]` props.

[IMPORTANT] Scoring priorities: exact slug match (100) > label starts-with (80) > slug starts-with (70) > label contains (50) > slug contains (40) > content contains (20). This gives precise matches priority over fuzzy partial matches.

[MINOR] Empty queries return no results immediately (no index scan).

```typescript
import type Graph from 'graphology'

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
 * A search result with scoring.
 */
export interface SearchResult {
  nodeId: string
  label: string
  slug: string
  type: string
  content: string
  score: number
}

/**
 * Build a search index from the graphology Graph object.
 *
 * IMPORTANT: Reads slugs from node attributes (computed in Phase 52 graph
 * construction), NOT from the raw GraphNode[] props where slugs don't exist.
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
```

**Acceptance criteria:**
- `buildSearchIndex` reads `fullLabel`, `slug`, `searchContent`, `nodeType` from graphology attributes.
- `scoreMatch` returns correct scores for each match type.
- `scoreMatch` is case-insensitive.
- `searchNodes` returns results sorted by score descending.
- `searchNodes` returns empty array for empty/whitespace queries.
- `searchNodes` limits results to `maxResults`.

**Failure criteria:**
- `buildSearchIndex` reads from `GraphNode[]` props instead of graphology attributes.
- Scoring order is wrong (e.g., content match scores higher than label match).

---

### T54.2: Write `GraphSearchBar.svelte`

**File:** `packages/webui/src/lib/components/graph/GraphSearchBar.svelte`

[IMPORTANT] Keyboard navigation uses `focusedIndex` state to track which result is highlighted. `ArrowDown`/`ArrowUp` move the index, `Enter` selects the focused result, `Escape` closes the dropdown.

[MINOR] Each result row shows the slug in monospace, the label/content truncated to ~60 characters, and a type prefix indicator.

```svelte
<!--
  GraphSearchBar.svelte — Autocomplete search bar for graph nodes.
  Matches against node labels, slugs, and content snippets.
  Selecting a result fires onSelect with the node ID.
-->
<script lang="ts">
  import { buildSearchIndex, searchNodes } from './graph.search'
  import type { SearchResult } from './graph.search'
  import type Graph from 'graphology'

  interface GraphSearchBarProps {
    /** The graphology Graph to search over (reads slug attributes) */
    graph: Graph | null
    /** Callback when user selects a result */
    onSelect: (nodeId: string) => void
    /** Placeholder text */
    placeholder?: string
  }

  let {
    graph,
    onSelect,
    placeholder = 'Search graph nodes...',
  }: GraphSearchBarProps = $props()

  let query = $state('')
  let focusedIndex = $state(-1)
  let isOpen = $state(false)

  // Build search index reactively when graph changes
  let searchIndex = $derived(graph ? buildSearchIndex(graph) : [])

  // Compute results reactively when query changes
  let results: SearchResult[] = $derived(searchNodes(query, searchIndex))

  function handleInput(e: Event) {
    query = (e.target as HTMLInputElement).value
    focusedIndex = -1
    isOpen = query.trim().length > 0
  }

  function handleKeydown(e: KeyboardEvent) {
    if (!isOpen || results.length === 0) {
      if (e.key === 'Escape') {
        isOpen = false
        query = ''
      }
      return
    }

    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault()
        focusedIndex = Math.min(focusedIndex + 1, results.length - 1)
        break
      case 'ArrowUp':
        e.preventDefault()
        focusedIndex = Math.max(focusedIndex - 1, 0)
        break
      case 'Enter':
        e.preventDefault()
        if (focusedIndex >= 0 && focusedIndex < results.length) {
          selectResult(results[focusedIndex])
        }
        break
      case 'Escape':
        e.preventDefault()
        isOpen = false
        query = ''
        focusedIndex = -1
        break
    }
  }

  function selectResult(result: SearchResult) {
    onSelect(result.nodeId)
    query = ''
    isOpen = false
    focusedIndex = -1
  }

  function truncate(text: string, maxLen = 60): string {
    if (text.length <= maxLen) return text
    return text.slice(0, maxLen) + '...'
  }
</script>

<div class="search-bar-wrapper">
  <div class="search-input-wrapper">
    <span class="search-icon" aria-hidden="true">&#128269;</span>
    <input
      type="text"
      class="search-input"
      {placeholder}
      value={query}
      oninput={handleInput}
      onkeydown={handleKeydown}
      onfocus={() => { if (query.trim().length > 0) isOpen = true }}
      onblur={() => { setTimeout(() => { isOpen = false }, 150) }}
      role="combobox"
      aria-expanded={isOpen}
      aria-autocomplete="list"
      aria-controls="search-results"
    />
  </div>

  {#if isOpen && results.length > 0}
    <ul class="search-results" id="search-results" role="listbox">
      {#each results as result, i}
        <li
          class="search-result"
          class:focused={i === focusedIndex}
          role="option"
          aria-selected={i === focusedIndex}
          onmousedown={() => selectResult(result)}
          onmouseenter={() => { focusedIndex = i }}
        >
          <span class="result-slug">{result.slug}</span>
          <span class="result-label">{truncate(result.label)}</span>
          {#if result.content}
            <span class="result-content">{truncate(result.content)}</span>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .search-bar-wrapper {
    position: relative;
    width: 100%;
    max-width: 320px;
  }

  .search-input-wrapper {
    display: flex;
    align-items: center;
    border: 1px solid var(--color-border, #e5e7eb);
    border-radius: 6px;
    background: white;
    padding: 0 8px;
  }

  .search-icon {
    font-size: 14px;
    margin-right: 6px;
    color: var(--color-muted, #9ca3af);
  }

  .search-input {
    flex: 1;
    border: none;
    outline: none;
    padding: 6px 0;
    font-size: 0.85rem;
    background: transparent;
  }

  .search-results {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    margin: 4px 0 0;
    padding: 4px 0;
    list-style: none;
    background: white;
    border: 1px solid var(--color-border, #e5e7eb);
    border-radius: 6px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
    max-height: 300px;
    overflow-y: auto;
    z-index: 20;
  }

  .search-result {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 6px 12px;
    cursor: pointer;
  }

  .search-result:hover,
  .search-result.focused {
    background: var(--color-hover, #f3f4f6);
  }

  .result-slug {
    font-family: monospace;
    font-size: 0.8rem;
    color: var(--color-primary, #3b82f6);
  }

  .result-label {
    font-size: 0.85rem;
    color: var(--color-text, #374151);
  }

  .result-content {
    font-size: 0.75rem;
    color: var(--color-muted, #9ca3af);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
```

**Acceptance criteria:**
- Input field renders with placeholder text.
- Typing a query displays autocomplete results.
- Results show slug (monospace), label, and truncated content.
- ArrowDown/ArrowUp move focus through results.
- Enter selects the focused result and calls `onSelect`.
- Escape closes the dropdown and clears query.
- Clicking a result calls `onSelect`.
- Empty query shows no results.
- ARIA attributes for combobox accessibility.

**Failure criteria:**
- Dropdown remains open after selecting a result.
- Keyboard navigation wraps around (should clamp at boundaries).
- `onBlur` closes dropdown before `onmousedown` fires on result.

---

### T54.3: Verify GraphView Exported Methods

**File:** `packages/webui/src/lib/components/graph/GraphView.svelte`

[IMPORTANT] Phase 48 already adds `getSigma()`, `getGraph()`, and `focusNode()` to `GraphView.svelte`. This task verifies they exist and match the interface expected by the search bar and toolbar. If they are missing or differ, add them.

The expected interface:

```typescript
// Already in Phase 48's GraphView.svelte (lines ~699-726):
export function getSigma(): any { return sigmaInstance }
export function getGraph(): Graph | null { return graph }
export function focusNode(nodeId: string): void {
  if (!graph?.hasNode(nodeId) || !sigmaInstance) return
  selectedNodeId = nodeId
  const displayData = sigmaInstance.getNodeDisplayData(nodeId)
  if (displayData) {
    sigmaInstance.getCamera().animate(
      { x: displayData.x, y: displayData.y, ratio: 0.3 },
      { duration: 300 },
    )
  }
  const attrs = graph.getNodeAttributes(nodeId)
  onNodeClick?.(nodeId, attrs as GraphNode)
}
```

The spec (H5) defines a slightly different signature with optional `ratio` and `duration` params and a `boolean` return. If Phase 48 does not include these, add the enhanced version:

```typescript
/**
 * Animate camera to center on a specific node.
 * Returns false if the node doesn't exist or Sigma is not initialized.
 */
export function focusNode(
  nodeId: string,
  ratio = 0.3,
  duration = 400
): boolean {
  if (!sigmaInstance || !graph?.hasNode(nodeId)) return false
  selectedNodeId = nodeId
  const displayData = sigmaInstance.getNodeDisplayData(nodeId)
  if (!displayData) return false
  sigmaInstance.getCamera().animate(
    { x: displayData.x, y: displayData.y, ratio },
    { duration },
  )
  const attrs = graph.getNodeAttributes(nodeId)
  onNodeClick?.(nodeId, attrs as GraphNode)
  return true
}
```

**Acceptance criteria:**
- `getSigma()` returns the Sigma instance or `null`.
- `getGraph()` returns the graphology Graph or `null`.
- `focusNode(nodeId)` animates camera and selects the node.
- `focusNode` returns `false` for non-existent nodes.
- `focusNode` accepts optional `ratio` and `duration` parameters.

**Failure criteria:**
- Methods not exported (consumer cannot call them via `bind:this`).
- `focusNode` throws for non-existent nodes instead of returning `false`.

---

### T54.4: Write Search Unit Tests

**File:** `packages/webui/src/lib/components/graph/__tests__/graph-search.test.ts`

[IMPORTANT] Tests for `buildSearchIndex` use a mock graphology-like object since the actual `Graph` class requires a full import. The mock provides `forEachNode` that iterates over test data.

```typescript
import { describe, it, expect } from 'vitest'
import { buildSearchIndex, scoreMatch, searchNodes } from '../graph.search'
import type { SearchEntry } from '../graph.search'

// Mock graph object for buildSearchIndex tests
function createMockGraph(nodes: Record<string, Record<string, unknown>>) {
  return {
    forEachNode: (callback: (nodeId: string, attrs: Record<string, unknown>) => void) => {
      for (const [id, attrs] of Object.entries(nodes)) {
        callback(id, attrs)
      }
    },
  } as any
}

describe('buildSearchIndex', () => {
  it('builds entries from graphology node attributes', () => {
    const graph = createMockGraph({
      '1': {
        fullLabel: 'Raytheon PCFE Migration Project',
        slug: 'src:raytheon-pcfe',
        searchContent: 'Led cloud migration for radar systems',
        nodeType: 'source',
      },
    })

    const index = buildSearchIndex(graph)
    expect(index).toHaveLength(1)
    expect(index[0]).toEqual({
      nodeId: '1',
      label: 'Raytheon PCFE Migration Project',
      slug: 'src:raytheon-pcfe',
      content: 'Led cloud migration for radar systems',
      type: 'source',
    })
  })

  it('falls back to label when fullLabel is missing', () => {
    const graph = createMockGraph({
      '1': { label: 'Fallback Label', nodeType: 'bullet' },
    })

    const index = buildSearchIndex(graph)
    expect(index[0].label).toBe('Fallback Label')
  })

  it('handles missing attributes gracefully', () => {
    const graph = createMockGraph({
      '1': {},
    })

    const index = buildSearchIndex(graph)
    expect(index[0]).toEqual({
      nodeId: '1',
      label: '',
      slug: '',
      content: '',
      type: '',
    })
  })
})

describe('scoreMatch', () => {
  const entry: SearchEntry = {
    nodeId: '1',
    label: 'Raytheon PCFE Migration Project',
    slug: 'src:raytheon-pcfe',
    content: 'Led cloud migration for radar systems',
    type: 'source',
  }

  it('scores exact slug match highest', () => {
    expect(scoreMatch('src:raytheon-pcfe', entry)).toBe(100)
  })

  it('scores label starts-with highly', () => {
    expect(scoreMatch('Raytheon', entry)).toBe(80)
  })

  it('scores slug starts-with', () => {
    expect(scoreMatch('src:', entry)).toBe(70)
  })

  it('scores label contains', () => {
    expect(scoreMatch('PCFE', entry)).toBe(50)
  })

  it('scores content contains lowest', () => {
    expect(scoreMatch('radar', entry)).toBe(20)
  })

  it('returns 0 for no match', () => {
    expect(scoreMatch('kubernetes', entry)).toBe(0)
  })

  it('is case-insensitive', () => {
    expect(scoreMatch('raytheon', entry)).toBe(80)
    expect(scoreMatch('SRC:RAYTHEON-PCFE', entry)).toBe(100)
  })

  it('returns 0 for empty query', () => {
    expect(scoreMatch('', entry)).toBe(0)
    expect(scoreMatch('   ', entry)).toBe(0)
  })
})

describe('searchNodes', () => {
  const index: SearchEntry[] = [
    {
      nodeId: '1',
      label: 'Raytheon PCFE Migration Project',
      slug: 'src:raytheon-pcfe',
      content: 'Led cloud migration for radar systems',
      type: 'source',
    },
    {
      nodeId: '2',
      label: 'Built AI taxonomy pipeline',
      slug: 'blt:built-ai',
      content: 'Designed ML classification pipeline',
      type: 'bullet',
    },
    {
      nodeId: '3',
      label: 'DevSecOps Security Lead',
      slug: 'psp:devsecops-sec',
      content: 'Security-focused perspective',
      type: 'perspective',
    },
  ]

  it('returns results sorted by score descending', () => {
    const results = searchNodes('src', index)
    expect(results.length).toBeGreaterThan(0)
    expect(results[0].nodeId).toBe('1')  // slug starts with 'src'
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
    const results = searchNodes('security', index)
    expect(results[0].nodeId).toBe('3')  // label contains 'Security'
  })

  it('includes score in results', () => {
    const results = searchNodes('src:raytheon-pcfe', index)
    expect(results[0].score).toBe(100)
  })
})
```

**Acceptance criteria:**
- All 16 test cases pass.
- `buildSearchIndex` reads from graphology attributes correctly.
- Scoring priorities are verified (exact > starts-with > contains).
- Case insensitivity is verified.
- Edge cases (empty query, missing attributes) are covered.

**Failure criteria:**
- Any test fails, indicating search logic bug.

---

## Testing Support

| Test file | Test count | Type |
|-----------|-----------|------|
| `__tests__/graph-search.test.ts` | 16 | Unit |
| **Total** | **16** | |

**Run command:** `cd packages/webui && npx vitest run src/lib/components/graph/__tests__/graph-search.test.ts`

## Documentation Requirements

- Export `SearchEntry`, `SearchResult`, `buildSearchIndex`, `scoreMatch`, `searchNodes` from `graph.search.ts`.
- Document the `GraphView` exported methods (`getSigma`, `getGraph`, `focusNode`) for consumer usage with `bind:this`.
- No new user-facing docs (internal component module).

## Parallelization Notes

- T54.1 (new file `graph.search.ts`) is independent -- can start immediately.
- T54.2 (new file `GraphSearchBar.svelte`) depends on T54.1.
- T54.3 modifies `GraphView.svelte` -- verifies/enhances existing exports. Minimal conflict potential with other phases.
- T54.4 (tests) depends on T54.1.
- This phase depends on Phase 52 (needs slugs on graphology nodes). Cannot start until Phase 52's T52.2 (slug integration into graph construction) is complete.
- Phase 55 (toolbar) depends on this phase's `getSigma()`/`getGraph()` exports.
