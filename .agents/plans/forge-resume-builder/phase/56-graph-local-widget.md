# Phase 56: Local Graph Widget (Spec H7)

**Status:** Planning
**Date:** 2026-04-03
**Spec:** [2026-04-03-graph-local-widget.md](../refs/specs/2026-04-03-graph-local-widget.md)
**Depends on:** Phase 48 (Generic GraphView)
**Blocks:** None
**Parallelizable with:** Phase 51, Phase 52, Phase 53, Phase 54, Phase 55 -- creates new files only (`graph.local.ts`, `LocalGraphWidget.svelte`), does not modify any existing files

## Goal

Add a small embeddable graph widget that shows a single node and its immediate neighbors (1-hop), providing at-a-glance relationship context in entity detail views without navigating to the full graph view. The widget reuses `GraphView` from Phase 48 with minimal configuration: no drag, no toolbar, no filters, no zoom, compact size (default 300x200px). A thin wrapper extracts the local subgraph, pre-positions nodes (center at origin, neighbors in a circle), and runs a one-shot layout.

## Non-Goals

- Multi-hop exploration (1-hop only, no expanding)
- Interactive editing within the widget (no drag, no node creation)
- Toolbar or filter controls (read-only mini-view)
- Live ForceAtlas2 simulation (one-shot layout only)
- Responsive auto-resizing (fixed size, parent can override via CSS)

## Context

Entity detail views (source detail, bullet modal, org detail) lack spatial context. Users see properties but not relationships. The widget fills this gap by showing a node's immediate neighborhood in a compact graph visualization. It wraps `GraphView` with pre-extracted subgraph data and a compact configuration that disables interactive features.

The widget is self-contained -- it does not modify `GraphView` or any other existing file. It creates two new files: `graph.local.ts` (subgraph extraction, pre-positioning, widget config) and `LocalGraphWidget.svelte` (the component).

## Scope

| Spec section | Covered here? |
|-------------|---------------|
| 1. Subgraph Extraction (algorithm, center emphasis) | Yes |
| 2. Layout (pre-positioning, widget config) | Yes |
| 3. Component `LocalGraphWidget.svelte` (props, implementation, click behavior, empty state) | Yes |
| 4. Use Cases (source detail, bullet modal, org detail) | Yes (examples in spec) |
| 5. Testing | Yes |

## Files to Create

| File | Description |
|------|-------------|
| `packages/webui/src/lib/components/graph/graph.local.ts` | `extractLocalSubgraph()`, `prepareLocalNodes()`, `prePositionLocalNodes()`, `LOCAL_WIDGET_CONFIG` |
| `packages/webui/src/lib/components/graph/LocalGraphWidget.svelte` | Small embeddable graph widget component |
| `packages/webui/src/lib/components/graph/__tests__/graph-local.test.ts` | Unit tests for subgraph extraction, node preparation, positioning (11 cases) |

## Files to Modify

None. This phase creates new files only.

## Fallback Strategies

- **`centerId` not found in `allNodes`:** `extractLocalSubgraph` returns `{ nodes: [], edges: [] }`. The widget renders "Node not found" message.
- **Center node has no edges:** `extractLocalSubgraph` returns `{ nodes: [centerNode], edges: [] }`. The widget renders "No connected entities" message.
- **`GraphView` fails to initialize in widget context:** The widget container shows the `GraphView` error state (inherited from Phase 48). No crash.
- **Sigma v3 does not have `enableZoom` config flag:** The `LOCAL_WIDGET_CONFIG` notes this. To disable zoom, the component passes `zoomingRatio: 1` to Sigma settings (makes scroll-wheel zoom a no-op). The `GraphView` should translate `enableZoom: false` into Sigma-native settings.
- **Widget rendered before graph data is available:** If `nodes` or `edges` are empty arrays, the widget shows the empty state. No crash.

---

## Tasks

### T56.1: Write Subgraph Extraction and Configuration Module

**File:** `packages/webui/src/lib/components/graph/graph.local.ts`

[IMPORTANT] `extractLocalSubgraph` only includes edges directly connected to the center node. Neighbor-to-neighbor edges are excluded to keep the widget clean and focused on the center node's relationships.

[IMPORTANT] `prepareLocalNodes` enlarges the center node to 1.5x default size and sets `forceLabel: true` so its label is always visible regardless of zoom threshold.

[MINOR] `prePositionLocalNodes` places the center at (50, 50) and distributes neighbors evenly in a circle with radius 30. This gives ForceAtlas2 a good starting point for convergence in few iterations.

```typescript
import type { GraphNode, GraphEdge, GraphConfig } from './graph.types'

/**
 * Extract a subgraph containing the center node and all nodes
 * directly connected by an edge (1-hop neighborhood).
 *
 * Returns new arrays of nodes and edges. The center node is included.
 * Edges between neighbors (not involving the center) are excluded.
 *
 * Returns empty arrays if centerId is not found in allNodes.
 */
export function extractLocalSubgraph(
  centerId: string,
  allNodes: GraphNode[],
  allEdges: GraphEdge[]
): { nodes: GraphNode[]; edges: GraphEdge[] } {
  // Find edges connected to the center node
  const connectedEdges = allEdges.filter(
    e => e.source === centerId || e.target === centerId
  )

  // Collect neighbor IDs
  const neighborIds = new Set<string>()
  for (const edge of connectedEdges) {
    if (edge.source === centerId) neighborIds.add(edge.target)
    if (edge.target === centerId) neighborIds.add(edge.source)
  }

  // Build node set: center + neighbors
  const nodeIds = new Set([centerId, ...neighborIds])
  const nodes = allNodes.filter(n => nodeIds.has(n.id))

  // If center node itself wasn't in allNodes, return empty
  if (!nodes.some(n => n.id === centerId)) {
    return { nodes: [], edges: [] }
  }

  // Only include edges that connect to the center (not neighbor-to-neighbor)
  return { nodes, edges: connectedEdges }
}

/**
 * Prepare nodes for the local widget, emphasizing the center node.
 * Center node gets 1.5x size and forceLabel: true.
 */
export function prepareLocalNodes(
  centerId: string,
  nodes: GraphNode[],
  config: GraphConfig
): GraphNode[] {
  return nodes.map(node => {
    if (node.id === centerId) {
      return {
        ...node,
        size: (node.size ?? config.nodeDefaults.size) * 1.5,
        forceLabel: true,
      }
    }
    return node
  })
}

/**
 * Pre-position nodes for the local widget layout.
 * Center at (50, 50), neighbors in a circle around it.
 * Gives ForceAtlas2 a good starting point for fast convergence.
 */
export function prePositionLocalNodes(
  centerId: string,
  nodes: GraphNode[]
): GraphNode[] {
  const neighborCount = nodes.filter(n => n.id !== centerId).length
  const angleStep = (2 * Math.PI) / Math.max(neighborCount, 1)
  const radius = 30

  const result: GraphNode[] = []
  let neighborIdx = 0

  for (const node of nodes) {
    if (node.id === centerId) {
      result.push({ ...node, x: 50, y: 50 })
    } else {
      result.push({
        ...node,
        x: 50 + radius * Math.cos(neighborIdx * angleStep),
        y: 50 + radius * Math.sin(neighborIdx * angleStep),
      })
      neighborIdx++
    }
  }

  return result
}

/**
 * Widget-specific config: no drag, no edge events, small iterations.
 * Consumers merge this with their own overrides via mergeConfig().
 *
 * NOTE: Sigma v3 does not have an `enableZoom` config flag. To disable
 * scroll-wheel zoom, the GraphView component should translate
 * `enableZoom: false` into Sigma-native settings (e.g., zoomingRatio: 1
 * or maxCameraRatio: 1, minCameraRatio: 1).
 *
 * IMPORTANT: Phase 48's `GraphView` must translate `enableZoom: false` into
 * Sigma-native settings (`zoomingRatio: 1` or `maxCameraRatio: minCameraRatio: 1`).
 * If Phase 48 does not implement this translation, LocalGraphWidget's zoom
 * disabling will silently not work -- the user will still be able to scroll-zoom
 * inside the mini widget.
 */
export const LOCAL_WIDGET_CONFIG: Partial<GraphConfig> = {
  layout: 'forceatlas2',
  forces: {
    gravity: 3,
    scalingRatio: 5,
    slowDown: 2,
    iterations: 30,
  },
  enableDrag: false,
  enableZoom: false,
  enableEdgeEvents: false,
  labelThreshold: 1e6,
  zIndex: false,
}
```

**Acceptance criteria:**
- `extractLocalSubgraph` includes center + 1-hop neighbors.
- `extractLocalSubgraph` excludes 2-hop nodes.
- `extractLocalSubgraph` excludes neighbor-to-neighbor edges.
- `extractLocalSubgraph` returns empty for non-existent centerId.
- `extractLocalSubgraph` returns center-only for isolated nodes.
- `prepareLocalNodes` enlarges center to 1.5x and sets `forceLabel`.
- `prepareLocalNodes` does not modify neighbor nodes.
- `prePositionLocalNodes` places center at (50, 50).
- `prePositionLocalNodes` distributes neighbors in a circle at radius 30.
- `LOCAL_WIDGET_CONFIG` disables drag, zoom, edge events, and z-index.

**Failure criteria:**
- 2-hop nodes included in subgraph.
- Neighbor-to-neighbor edges included.
- Center node not enlarged or missing `forceLabel`.

---

### T56.2: Write `LocalGraphWidget.svelte`

**File:** `packages/webui/src/lib/components/graph/LocalGraphWidget.svelte`

[IMPORTANT] The component extracts the subgraph, prepares nodes, pre-positions them, and passes everything to `GraphView`. All transformations are reactive via `$derived`.

[MINOR] Clicking the center node is a no-op. Clicking a neighbor fires `onNodeClick` with the neighbor's data.

[GAP] The spec shows `highlightNode={centerId}` on the `GraphView`. This triggers Phase 48's camera animation and selection highlighting on the center node, which provides the visual emphasis.

```svelte
<!--
  LocalGraphWidget.svelte — Small embeddable graph showing a node's
  1-hop neighborhood. Read-only mini-view for entity detail pages.
-->
<script lang="ts">
  import GraphView from './GraphView.svelte'
  import { mergeConfig } from './graph.config'
  import {
    extractLocalSubgraph,
    prepareLocalNodes,
    prePositionLocalNodes,
    LOCAL_WIDGET_CONFIG,
  } from './graph.local'
  import type { GraphNode, GraphEdge, GraphConfig } from './graph.types'

  interface LocalGraphWidgetProps {
    /** The ID of the center node to display */
    centerId: string
    /** All graph nodes (the widget extracts the subgraph) */
    nodes: GraphNode[]
    /** All graph edges (the widget extracts the subgraph) */
    edges: GraphEdge[]
    /** Optional config overrides (merged with LOCAL_WIDGET_CONFIG) */
    config?: Partial<GraphConfig>
    /** Width in pixels (default: 300) */
    width?: number
    /** Height in pixels (default: 200) */
    height?: number
    /** Callback when a neighbor node is clicked */
    onNodeClick?: (nodeId: string, data: GraphNode) => void
  }

  let {
    centerId,
    nodes,
    edges,
    config: configOverride = undefined,
    width = 300,
    height = 200,
    onNodeClick = undefined,
  }: LocalGraphWidgetProps = $props()

  // Extract 1-hop subgraph reactively
  let subgraph = $derived(extractLocalSubgraph(centerId, nodes, edges))

  // Merge widget defaults with consumer overrides
  let resolvedConfig = $derived(mergeConfig({
    ...LOCAL_WIDGET_CONFIG,
    ...configOverride,
  }))

  // Emphasize center node and pre-position for layout
  let preparedNodes = $derived(
    prePositionLocalNodes(
      centerId,
      prepareLocalNodes(centerId, subgraph.nodes, resolvedConfig)
    )
  )

  function handleNodeClick(nodeId: string, data: GraphNode) {
    if (nodeId === centerId) return  // clicking center is a no-op
    onNodeClick?.(nodeId, data)
  }
</script>

<div class="local-graph-widget" style:width="{width}px" style:height="{height}px">
  {#if subgraph.nodes.length === 0}
    <p class="widget-empty">Node not found</p>
  {:else if subgraph.edges.length === 0}
    <p class="widget-empty">No connected entities</p>
  {:else}
    <GraphView
      nodes={preparedNodes}
      edges={subgraph.edges}
      config={resolvedConfig}
      highlightNode={centerId}
      onNodeClick={handleNodeClick}
    />
  {/if}
</div>

<style>
  .local-graph-widget {
    position: relative;
    border: 1px solid var(--color-border, #e5e7eb);
    border-radius: 8px;
    overflow: hidden;
    background: var(--color-surface, #ffffff);
  }

  .widget-empty {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    margin: 0;
    color: var(--color-muted, #9ca3af);
    font-size: 0.875rem;
  }
</style>
```

**Acceptance criteria:**
- Widget renders at default size 300x200px.
- Widget size is configurable via `width` and `height` props.
- Subgraph is extracted reactively when `centerId`, `nodes`, or `edges` change.
- `GraphView` receives pre-positioned nodes and widget config.
- `highlightNode` is set to `centerId` for visual emphasis.
- Clicking a neighbor fires `onNodeClick` with neighbor data.
- Clicking the center node is a no-op.
- Non-existent `centerId` shows "Node not found".
- Center with no edges shows "No connected entities".
- Widget has border, rounded corners, overflow hidden.

**Failure criteria:**
- Widget does not extract subgraph (shows full graph).
- Clicking center node fires `onNodeClick` (should be no-op).
- Empty states not rendered for invalid/isolated center.

---

### T56.3: Write Subgraph and Positioning Unit Tests

**File:** `packages/webui/src/lib/components/graph/__tests__/graph-local.test.ts`

```typescript
import { describe, it, expect } from 'vitest'
import {
  extractLocalSubgraph,
  prepareLocalNodes,
  prePositionLocalNodes,
} from '../graph.local'
import type { GraphNode, GraphEdge } from '../graph.types'

const nodes: GraphNode[] = [
  { id: 'a', label: 'Center', type: 'source' },
  { id: 'b', label: 'Neighbor 1', type: 'bullet' },
  { id: 'c', label: 'Neighbor 2', type: 'bullet' },
  { id: 'd', label: 'Connected to E', type: 'perspective' },
  { id: 'e', label: 'Two-hop', type: 'perspective' },
  { id: 'f', label: 'Truly Isolated', type: 'source' },
]

const edges: GraphEdge[] = [
  { id: 'e1', source: 'a', target: 'b' },
  { id: 'e2', source: 'a', target: 'c' },
  { id: 'e3', source: 'b', target: 'e' },
  { id: 'e4', source: 'd', target: 'e' },
]

describe('extractLocalSubgraph', () => {
  it('includes center node and direct neighbors', () => {
    const { nodes: subNodes } = extractLocalSubgraph('a', nodes, edges)
    const ids = subNodes.map(n => n.id)
    expect(ids).toContain('a')
    expect(ids).toContain('b')
    expect(ids).toContain('c')
  })

  it('excludes nodes beyond 1-hop', () => {
    const { nodes: subNodes } = extractLocalSubgraph('a', nodes, edges)
    const ids = subNodes.map(n => n.id)
    expect(ids).not.toContain('d')
    expect(ids).not.toContain('e')
  })

  it('includes only edges connected to center', () => {
    const { edges: subEdges } = extractLocalSubgraph('a', nodes, edges)
    const edgeIds = subEdges.map(e => e.id)
    expect(edgeIds).toContain('e1')
    expect(edgeIds).toContain('e2')
    expect(edgeIds).not.toContain('e3')
    expect(edgeIds).not.toContain('e4')
  })

  it('handles center with no edges', () => {
    const { nodes: subNodes, edges: subEdges } = extractLocalSubgraph('f', nodes, edges)
    expect(subNodes.length).toBe(1)
    expect(subNodes[0].id).toBe('f')
    expect(subEdges.length).toBe(0)
  })

  it('handles non-existent centerId', () => {
    const { nodes: subNodes, edges: subEdges } = extractLocalSubgraph('z', nodes, edges)
    expect(subNodes.length).toBe(0)
    expect(subEdges.length).toBe(0)
  })
})

describe('prepareLocalNodes', () => {
  const config = { nodeDefaults: { size: 8, color: '#000' } } as any

  it('enlarges center node to 1.5x', () => {
    const prepared = prepareLocalNodes('a', nodes.slice(0, 3), config)
    const center = prepared.find(n => n.id === 'a')!
    expect(center.size).toBe(12)  // 8 * 1.5
  })

  it('does not enlarge neighbor nodes', () => {
    const prepared = prepareLocalNodes('a', nodes.slice(0, 3), config)
    const neighbor = prepared.find(n => n.id === 'b')!
    expect(neighbor.size).toBeUndefined()
  })

  it('sets forceLabel on center node', () => {
    const prepared = prepareLocalNodes('a', nodes.slice(0, 3), config)
    const center = prepared.find(n => n.id === 'a')!
    expect((center as any).forceLabel).toBe(true)
  })
})

describe('prePositionLocalNodes', () => {
  it('places center at (50, 50)', () => {
    const positioned = prePositionLocalNodes('a', nodes.slice(0, 3))
    const center = positioned.find(n => n.id === 'a')!
    expect(center.x).toBe(50)
    expect(center.y).toBe(50)
  })

  it('places neighbors in a circle around center at radius 30', () => {
    const positioned = prePositionLocalNodes('a', nodes.slice(0, 3))
    const neighbors = positioned.filter(n => n.id !== 'a')
    for (const n of neighbors) {
      expect(n.x).toBeDefined()
      expect(n.y).toBeDefined()
      const dist = Math.sqrt((n.x! - 50) ** 2 + (n.y! - 50) ** 2)
      expect(dist).toBeCloseTo(30, 0)
    }
  })

  it('handles single neighbor without errors', () => {
    const positioned = prePositionLocalNodes('a', [nodes[0], nodes[1]])
    expect(positioned.length).toBe(2)
    const neighbor = positioned.find(n => n.id === 'b')!
    expect(neighbor.x).toBeDefined()
    expect(neighbor.y).toBeDefined()
  })
})
```

**Acceptance criteria:**
- All 11 test cases pass.
- Subgraph extraction verified for 1-hop inclusion, 2-hop exclusion, edge filtering.
- Empty/non-existent center cases verified.
- Center node enlargement and positioning verified.
- Neighbor circular positioning verified at radius 30.

**Failure criteria:**
- Any test fails, indicating subgraph extraction or positioning bug.

---

## Testing Support

| Test file | Test count | Type |
|-----------|-----------|------|
| `__tests__/graph-local.test.ts` | 11 | Unit |
| **Total** | **11** | |

**Run command:** `cd packages/webui && npx vitest run src/lib/components/graph/__tests__/graph-local.test.ts`

## Documentation Requirements

- Export `extractLocalSubgraph`, `prepareLocalNodes`, `prePositionLocalNodes`, `LOCAL_WIDGET_CONFIG` from `graph.local.ts`.
- Consumer usage examples in JSDoc (source detail, bullet modal, org detail embedding).
- No new user-facing docs (internal component module).

## Parallelization Notes

- T56.1 (new file `graph.local.ts`) is independent -- can start immediately.
- T56.2 (new file `LocalGraphWidget.svelte`) depends on T56.1.
- T56.3 (tests) depends on T56.1 only.
- This entire phase modifies NO existing files. It is fully parallelizable with all other phases (51-55).
- The widget can be embedded by any entity detail view independently of the other graph phases.
