# Local Graph Widget

**Date:** 2026-04-03
**Spec:** H7 (Local Graph Widget)
**Phase:** TBD (next available)
**Depends on:** H1 (Generic GraphView Component)

## Overview

Entity detail views (source detail, bullet modal, org detail) lack spatial context — users can see an entity's properties but not how it relates to neighbors in the graph. This spec adds a small embeddable graph widget that shows a single node and its immediate neighbors (1-hop), providing at-a-glance relationship context without navigating to the full graph view.

The widget reuses the `GraphView` component from H1 with a minimal configuration: no drag, no toolbar, no filters, compact size. It is a thin wrapper that accepts a `centerId` prop, fetches or receives the relevant subgraph data, and renders the local neighborhood.

## Non-Goals

- **Multi-hop exploration:** The widget shows 1-hop neighbors only. No expanding the neighborhood by clicking.
- **Interactive editing within the widget:** No drag, no node creation, no edge editing.
- **Toolbar or filter controls:** The widget is a read-only mini-view. Full interaction is deferred to the main graph view.
- **Live simulation:** No ForceAtlas2 animation. A one-shot layout positions nodes once on render.
- **Responsive resizing:** The widget has a fixed default size. The parent can override via CSS, but the widget does not auto-resize to fill available space.

---

## 1. Subgraph Extraction

### 1.1 Algorithm

Given a `centerId` and the full graph data (`GraphNode[]`, `GraphEdge[]`), extract the 1-hop neighborhood:

```typescript
/**
 * Extract a subgraph containing the center node and all nodes
 * directly connected by an edge (1-hop neighborhood).
 *
 * Returns a new set of nodes and edges. The center node is
 * included in the result. Edges between neighbors (not involving
 * the center) are excluded to keep the widget clean.
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

  // Only include edges that connect to the center (not neighbor-to-neighbor)
  return { nodes, edges: connectedEdges }
}
```

### 1.2 Center Node Emphasis

The center node is visually emphasized:
- Larger size (1.5x the default node size)
- Full color (from config colorMap)
- Label always visible (force-displayed, not dependent on zoom threshold)

Neighbor nodes use the standard size and color from config.

```typescript
/**
 * Prepare nodes for the local widget, emphasizing the center node.
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
```

---

## 2. Layout

### 2.1 Strategy

For a small subgraph (typically 2-15 nodes), a one-shot ForceAtlas2 with few iterations produces a clean layout. The center node is placed at (0, 0) and neighbors radiate outward.

```typescript
/**
 * Widget-specific config: no drag, no edge events, small iterations.
 */
export const LOCAL_WIDGET_CONFIG: Partial<GraphConfig> = {
  layout: 'forceatlas2',
  forces: {
    gravity: 3,        // strong gravity to keep neighbors close
    scalingRatio: 5,   // low scaling to prevent spread
    slowDown: 2,
    iterations: 30,    // few iterations — small graph converges fast
  },
  enableDrag: false,
  enableZoom: false,     // no scroll-wheel zoom on the mini widget
  // NOTE: Sigma v3 does not have an `enableZoom` config flag. To disable zoom,
  // set `maxCameraRatio: 1, minCameraRatio: 1` in the Sigma constructor options
  // to lock the camera ratio, or use `zoomingRatio: 1` to make zoom a no-op.
  // The GraphView component should translate `enableZoom: false` into these
  // Sigma-native settings.
  enableEdgeEvents: false,
  labelThreshold: 1e6,  // hide labels by default (center uses forceLabel)
  // NOTE: Avoid `Infinity` — use `1e6` or `Number.MAX_SAFE_INTEGER` instead.
  zIndex: false,
}
```

### 2.2 Pre-positioning

Before running ForceAtlas2, the center node is placed at (50, 50) and neighbors are arranged in a circle around it. This gives the force layout a good starting point and ensures convergence in few iterations:

```typescript
/**
 * Pre-position nodes for the local widget layout.
 * Center at (50, 50), neighbors in a circle around it.
 */
export function prePositionLocalNodes(
  centerId: string,
  nodes: GraphNode[]
): GraphNode[] {
  // Use forEach with its index instead of `neighbors.indexOf(node)` which
  // can return -1 when object identity differs (e.g., after mapping/cloning).
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
```

---

## 3. Component: `LocalGraphWidget.svelte`

### 3.1 Props

```typescript
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
```

### 3.2 Implementation

The component computes the local subgraph reactively and renders a `GraphView`:

```svelte
<script lang="ts">
  import GraphView from './GraphView.svelte'
  import { mergeConfig } from './graph.config'
  import { extractLocalSubgraph, prepareLocalNodes, prePositionLocalNodes, LOCAL_WIDGET_CONFIG } from './graph.local'
  import type { GraphNode, GraphEdge, GraphConfig } from './graph.types'

  let {
    centerId,
    nodes,
    edges,
    config: configOverride = undefined,
    width = 300,
    height = 200,
    onNodeClick = undefined,
  }: LocalGraphWidgetProps = $props()

  // Extract 1-hop subgraph
  let subgraph = $derived(extractLocalSubgraph(centerId, nodes, edges))

  // Emphasize center node and pre-position
  let preparedNodes = $derived(
    prePositionLocalNodes(
      centerId,
      prepareLocalNodes(centerId, subgraph.nodes, mergeConfig(configOverride))
    )
  )

  // Merge widget defaults with consumer overrides
  let resolvedConfig = $derived({
    ...LOCAL_WIDGET_CONFIG,
    ...configOverride,
  })

  function handleNodeClick(nodeId: string, data: GraphNode) {
    if (nodeId === centerId) return  // clicking center does nothing
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

### 3.3 Click Behavior

When a user clicks a neighbor node in the widget:

1. The `onNodeClick` callback fires with the neighbor's ID and data.
2. The consumer decides what to do — typically one of:
   - Navigate to that entity's detail page.
   - Open the full graph view centered on that node.
   - Open a modal for that entity.

Clicking the center node is a no-op (the user is already viewing it).

### 3.4 Empty State

If `centerId` does not exist in `nodes`, or if the center node has no edges, the widget renders a minimal state:

- **No matching node:** Render a small message: "Node not found."
- **No neighbors:** Render just the center node with a message: "No connected entities."

---

## 4. Use Cases

### 4.1 Source Detail Page

Embed the widget in the source detail view to show which bullets and organizations are connected to a source:

```svelte
<LocalGraphWidget
  centerId={source.id}
  nodes={allGraphNodes}
  edges={allGraphEdges}
  config={{ colorMap: NODE_COLORS }}
  onNodeClick={(id) => goto(`/entities/${id}`)}
  width={350}
  height={220}
/>
```

### 4.2 Bullet Modal

Embed in the bullet detail modal to show the source it derives from and perspectives it feeds:

```svelte
<LocalGraphWidget
  centerId={bullet.id}
  nodes={allGraphNodes}
  edges={allGraphEdges}
  config={{ colorMap: NODE_COLORS }}
  onNodeClick={(id, data) => {
    if (data.type === 'source') goto(`/sources/${id}`)
    if (data.type === 'perspective') goto(`/perspectives/${id}`)
  }}
/>
```

### 4.3 Organization Detail

Show connected sources (employment history) and job descriptions (opportunities):

```svelte
<LocalGraphWidget
  centerId={org.id}
  nodes={allGraphNodes}
  edges={allGraphEdges}
  config={{ colorMap: NODE_COLORS }}
  onNodeClick={(id) => goto(`/entities/${id}`)}
  width={400}
  height={250}
/>
```

---

## 5. Files to Create

| File | Description |
|------|-------------|
| `packages/webui/src/lib/components/graph/graph.local.ts` | `extractLocalSubgraph`, `prepareLocalNodes`, `prePositionLocalNodes`, `LOCAL_WIDGET_CONFIG` |
| `packages/webui/src/lib/components/graph/LocalGraphWidget.svelte` | Small embeddable graph widget component |

## 6. Files to Modify

None. The widget consumes the existing `GraphView` component without modifying it.

---

## 7. Testing Approach

### 7.1 Unit Tests

**File:** `packages/webui/src/lib/components/graph/__tests__/graph-local.test.ts`

```typescript
import { describe, it, expect } from 'vitest'
import { extractLocalSubgraph, prepareLocalNodes, prePositionLocalNodes } from '../graph.local'
import type { GraphNode, GraphEdge } from '../graph.types'

const nodes: GraphNode[] = [
  { id: 'a', label: 'Center', type: 'source' },
  { id: 'b', label: 'Neighbor 1', type: 'bullet' },
  { id: 'c', label: 'Neighbor 2', type: 'bullet' },
  { id: 'd', label: 'Connected to E', type: 'perspective' },
  { id: 'e', label: 'Two-hop', type: 'perspective' },
  { id: 'f', label: 'Truly Isolated', type: 'source' },  // no edges at all
]

const edges: GraphEdge[] = [
  { id: 'e1', source: 'a', target: 'b' },
  { id: 'e2', source: 'a', target: 'c' },
  { id: 'e3', source: 'b', target: 'e' },  // two-hop from 'a'
  { id: 'e4', source: 'd', target: 'e' },  // d-e connection, no connection to 'a'
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
    expect(edgeIds).not.toContain('e3')  // b->e, not connected to center
    expect(edgeIds).not.toContain('e4')
  })

  it('handles center with no edges', () => {
    // Node 'f' is truly isolated (no edges), unlike 'd' which has edge e4.
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
  it('enlarges center node', () => {
    const config = { nodeDefaults: { size: 8, color: '#000' } } as any
    const prepared = prepareLocalNodes('a', nodes.slice(0, 3), config)
    const center = prepared.find(n => n.id === 'a')!
    expect(center.size).toBe(12)  // 8 * 1.5
  })

  it('does not enlarge neighbor nodes', () => {
    const config = { nodeDefaults: { size: 8, color: '#000' } } as any
    const prepared = prepareLocalNodes('a', nodes.slice(0, 3), config)
    const neighbor = prepared.find(n => n.id === 'b')!
    expect(neighbor.size).toBeUndefined()  // unchanged
  })

  it('sets forceLabel on center node', () => {
    const config = { nodeDefaults: { size: 8, color: '#000' } } as any
    const prepared = prepareLocalNodes('a', nodes.slice(0, 3), config)
    const center = prepared.find(n => n.id === 'a')!
    expect(center.forceLabel).toBe(true)
  })
})

describe('prePositionLocalNodes', () => {
  it('places center at (50, 50)', () => {
    const positioned = prePositionLocalNodes('a', nodes.slice(0, 3))
    const center = positioned.find(n => n.id === 'a')!
    expect(center.x).toBe(50)
    expect(center.y).toBe(50)
  })

  it('places neighbors in a circle around center', () => {
    const positioned = prePositionLocalNodes('a', nodes.slice(0, 3))
    const neighbors = positioned.filter(n => n.id !== 'a')
    for (const n of neighbors) {
      expect(n.x).toBeDefined()
      expect(n.y).toBeDefined()
      // Distance from center should be approximately the radius (30)
      const dist = Math.sqrt((n.x! - 50) ** 2 + (n.y! - 50) ** 2)
      expect(dist).toBeCloseTo(30, 0)
    }
  })

  it('handles single neighbor', () => {
    const positioned = prePositionLocalNodes('a', [nodes[0], nodes[1]])
    expect(positioned.length).toBe(2)
  })
})
```

### 7.2 Component Tests

**File:** `packages/webui/src/lib/components/graph/__tests__/LocalGraphWidget.test.ts`

- Renders a container at the specified width and height.
- Passes the extracted subgraph to `GraphView`.
- Clicking a neighbor fires `onNodeClick` with the neighbor's ID.
- Clicking the center node does not fire `onNodeClick`.
- Renders "Node not found" when centerId is invalid.
- Renders just the center when no edges exist.

---

## 8. Acceptance Criteria

### Subgraph extraction
- [ ] Given centerId, includes the center node and all 1-hop neighbors
- [ ] Excludes nodes beyond 1-hop
- [ ] Includes only edges directly connected to the center node
- [ ] Excludes neighbor-to-neighbor edges (keeps the widget clean)
- [ ] Returns empty result for non-existent centerId
- [ ] Returns center-only result for isolated nodes (no edges)

### Center node emphasis
- [ ] Center node renders at 1.5x default size
- [ ] Center node label is always visible (forceLabel)
- [ ] Neighbor nodes use standard size

### Layout
- [ ] Nodes are pre-positioned (center at origin, neighbors in circle)
- [ ] One-shot ForceAtlas2 runs with 30 iterations
- [ ] No live simulation (no physics animation after initial render)

### Widget rendering
- [ ] Widget renders at default size 300x200px
- [ ] Widget size is configurable via `width` and `height` props
- [ ] Widget has a border and rounded corners
- [ ] No toolbar, no filters, no zoom controls inside the widget

### Interaction
- [ ] Clicking a neighbor node fires `onNodeClick` with that node's ID and data
- [ ] Clicking the center node is a no-op
- [ ] No drag support (nodes are fixed after layout)
- [ ] No scroll-wheel zoom

### Empty states
- [ ] Non-existent centerId shows "Node not found" message
- [ ] Center with no edges shows only the center node

### Tests
- [ ] `extractLocalSubgraph` unit tests pass (5 cases)
- [ ] `prepareLocalNodes` unit tests pass (3 cases)
- [ ] `prePositionLocalNodes` unit tests pass (3 cases)
- [ ] Widget component tests pass

---

## 9. Dependencies

- **Runtime:** None beyond H1's dependencies
- **Spec dependencies:** H1 (Generic GraphView Component) — the widget wraps `GraphView`
- **Blocked by:** H1
- **Blocks:** No other specs directly. The widget can be embedded by any entity detail view independently.
