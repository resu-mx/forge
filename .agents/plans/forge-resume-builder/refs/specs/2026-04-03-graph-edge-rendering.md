# Edge Rendering

**Date:** 2026-04-03
**Spec:** H2 (Edge Rendering)
**Phase:** TBD (next available)
**Depends on:** H1 (Generic GraphView Component)

## Overview

The generic `GraphView` from H1 renders all edges identically — same color, same thickness, simple arrow type. Real graph data has semantic variety: derivation edges (source -> bullet), skill-link edges (bullet -> skill), org-link edges (org -> source), each carrying different weight and meaning. This spec adds edge visual differentiation so users can read the graph's structure at a glance without clicking individual edges.

The implementation configures edge appearance through the existing `GraphConfig` surface (H1's `edgeColorMap`, `edgeDefaults`, and `GraphEdge.weight`), adds an edge hover highlight system that illuminates connected nodes, and introduces directional arrow rendering that shows derivation flow.

## Non-Goals

- **Edge editing:** No UI to create, delete, or modify edges. Edges reflect the data model.
- **Edge creation UI:** No drag-to-connect interaction.
- **Animated edge flows:** No particle animations or pulsing effects along edges.
- **Edge labels (rendered on canvas):** Edge labels are handled via tooltip on hover, not rendered inline on the WebGL canvas. Inline labels cause unreadable clutter at scale.
- **Custom WebGL edge programs:** Uses Sigma's built-in `arrow` and `line` programs. Dashed or curved edges are future work.

---

## 1. Edge Type Styling

### 1.1 Color by Type

Consumers provide an `edgeColorMap` in their `GraphConfig` that maps edge `type` strings to colors. The color resolution priority (established in H1) is:

```
edge.color (explicit per-edge) > config.edgeColorMap[edge.type] > config.edgeDefaults.color
```

**Recommended color palette for the Forge domain:**

```typescript
// Example consumer config (chain view, org view, etc.)
const EDGE_COLORS: Record<string, string> = {
  derivation: '#6c63ff',   // purple — source-to-bullet lineage
  skill_link: '#10b981',   // green — bullet-to-skill association
  org_link: '#f59e0b',     // amber — org-to-source employment
  perspective: '#3b82f6',  // blue — bullet-to-perspective tailoring
  resume_entry: '#8b5cf6', // violet — perspective-to-resume inclusion
}
```

This palette is consumer-owned, not baked into the generic component. The component only reads `edgeColorMap` from config.

### 1.2 Weight-Based Thickness

Edges with a `weight` property render with proportional thickness. The mapping is:

```typescript
/**
 * Compute edge display size from weight.
 * Clamps to [MIN_EDGE_SIZE, MAX_EDGE_SIZE] to prevent invisible or oversized edges.
 */
export const MIN_EDGE_SIZE = 0.5
export const MAX_EDGE_SIZE = 5
export const DEFAULT_EDGE_SIZE = 1

export function edgeSizeFromWeight(weight: number | undefined): number {
  if (weight === undefined) return DEFAULT_EDGE_SIZE
  return Math.max(MIN_EDGE_SIZE, Math.min(MAX_EDGE_SIZE, weight))
}
```

This function is called during graph construction (H1's `$effect` that builds the graphology `Graph`). The `size` attribute on each graphology edge is set to `edgeSizeFromWeight(edge.weight)`.

### 1.3 Arrow Styles

All directed edges use Sigma's built-in `arrow` edge type (already the default in H1's `edgeDefaults.type: 'arrow'`). The arrow head indicates derivation direction:

- **Source -> Bullet:** arrow points from source node to bullet node
- **Bullet -> Perspective:** arrow points from bullet to perspective
- **Org -> Source:** arrow points from org to source (employment relationship)

No configuration is needed beyond ensuring `edge.source` and `edge.target` are set correctly by the consumer. The arrow head renders at the `target` end automatically.

For undirected associations (e.g., skill-to-bullet), consumers set `edgeDefaults.type: 'line'` in their config or set `type: 'line'` on individual edges.

---

## 2. Edge Hover Highlight

### 2.1 Behavior

When the user hovers over an edge:

1. The hovered edge thickens by `HIGHLIGHT_EDGE_SIZE_BUMP` (defined in H1's constants).
2. The two endpoint nodes of the hovered edge receive a subtle highlight (size increase or border glow).
3. All other edges dim to `DIM_EDGE_COLOR` / `DIM_EDGE_SIZE`.
4. All non-endpoint nodes dim to `DIM_NODE_COLOR`.

When the user moves the cursor away, the graph returns to its normal state.

### 2.2 Implementation

This extends H1's existing `edgeReducer` and `nodeReducer`. H1 already handles `hoveredEdge` state for the edge itself. This spec adds the connected-node highlight:

```typescript
// In nodeReducer (extending H1's existing reducer):
nodeReducer: (node: string, data: Record<string, any>) => {
  // ... existing selectedNodeId logic from H1 ...

  // Edge hover: highlight endpoint nodes
  if (hoveredEdge && graph) {
    const [src, tgt] = graph.extremities(hoveredEdge)
    if (node === src || node === tgt) {
      return { ...data, zIndex: Z_FOREGROUND }
    }
    return { ...data, color: DIM_NODE_COLOR, label: '', zIndex: Z_BACKGROUND }
  }

  return data
},
```

The `edgeReducer` from H1 already handles the edge-side highlighting. The node reducer change is the only addition.

> **Edge-hover vs selection priority:** Edge-hover highlighting does NOT dim the currently selected node. The `nodeReducer` checks `selectedNodeId` first -- if a node is selected, its appearance takes priority over edge-hover dimming. The edge-hover branch only executes when `selectedNodeId` is null.

### 2.3 Tooltip on Edge Hover

When an edge is hovered, the consumer can render a tooltip showing the edge's metadata. The `onEdgeHover` callback (from H1) fires with the edge ID, and the consumer can look up the edge data and position a tooltip.

The generic component exposes a helper to get viewport coordinates for a tooltip near the hovered edge:

```typescript
/**
 * Get the midpoint of an edge in viewport (screen) coordinates.
 * Useful for positioning tooltips near hovered edges.
 */
export function getEdgeMidpoint(
  sigma: Sigma,
  graph: Graph,
  edgeId: string
): { x: number; y: number } | null {
  const [src, tgt] = graph.extremities(edgeId)
  const srcDisplay = sigma.getNodeDisplayData(src)
  const tgtDisplay = sigma.getNodeDisplayData(tgt)
  if (!srcDisplay || !tgtDisplay) return null
  // Compute midpoint in graph space, then convert to viewport (screen) pixels.
  // Using graph-space values directly would produce wrong tooltip positions
  // because display data coordinates are in graph space, not screen pixels.
  const midGraph = {
    x: (srcDisplay.x + tgtDisplay.x) / 2,
    y: (srcDisplay.y + tgtDisplay.y) / 2,
  }
  return sigma.graphToViewport(midGraph)
}
```

This is a pure utility function, not a component. Consumers call it in their `onEdgeHover` handler to position tooltips.

---

## 3. Component Interface Changes

### 3.1 New Props on GraphViewProps

No new props are needed. The existing `GraphConfig.edgeColorMap`, `GraphEdge.weight`, `GraphEdge.type`, and event callbacks from H1 are sufficient. The edge rendering enhancements are internal to the component's reducers and graph construction.

### 3.2 New Exports

```typescript
// graph.constants.ts additions
export const MIN_EDGE_SIZE = 0.5
export const MAX_EDGE_SIZE = 5
export const DEFAULT_EDGE_SIZE = 1
export function edgeSizeFromWeight(weight: number | undefined): number

// graph.utils.ts (new file)
export function getEdgeMidpoint(
  sigma: Sigma,
  graph: Graph,
  edgeId: string
): { x: number; y: number } | null
```

---

## 4. Files to Create

| File | Description |
|------|-------------|
| `packages/webui/src/lib/components/graph/graph.utils.ts` | Edge midpoint utility, future graph helper functions |

## 5. Files to Modify

| File | Description |
|------|-------------|
| `packages/webui/src/lib/components/graph/graph.constants.ts` | Add `MIN_EDGE_SIZE`, `MAX_EDGE_SIZE`, `DEFAULT_EDGE_SIZE`, `edgeSizeFromWeight()` |
| `packages/webui/src/lib/components/graph/GraphView.svelte` | Extend `nodeReducer` to handle `hoveredEdge` endpoint highlighting; use `edgeSizeFromWeight()` during graph construction |

---

## 6. Testing Approach

### 6.1 Unit Tests

**File:** `packages/webui/src/lib/components/graph/__tests__/graph-edge-rendering.test.ts`

```typescript
import { describe, it, expect } from 'vitest'
import { edgeSizeFromWeight, MIN_EDGE_SIZE, MAX_EDGE_SIZE, DEFAULT_EDGE_SIZE } from '../graph.constants'

describe('edgeSizeFromWeight', () => {
  it('returns DEFAULT_EDGE_SIZE for undefined weight', () => {
    expect(edgeSizeFromWeight(undefined)).toBe(DEFAULT_EDGE_SIZE)
  })

  it('clamps to MIN_EDGE_SIZE for very small weights', () => {
    expect(edgeSizeFromWeight(0.1)).toBe(MIN_EDGE_SIZE)
  })

  it('clamps to MAX_EDGE_SIZE for very large weights', () => {
    expect(edgeSizeFromWeight(100)).toBe(MAX_EDGE_SIZE)
  })

  it('passes through weights within range', () => {
    expect(edgeSizeFromWeight(2)).toBe(2)
    expect(edgeSizeFromWeight(3.5)).toBe(3.5)
  })
})
```

### 6.2 Integration Tests

**File:** `packages/webui/src/lib/components/graph/__tests__/graph-edge-hover.test.ts`

Test the reducer logic in isolation (without Sigma):

- Given `hoveredEdge` is set to an edge ID, verify `nodeReducer` returns `Z_FOREGROUND` for endpoint nodes and `DIM_NODE_COLOR` for all others.
- Given `hoveredEdge` is null, verify `nodeReducer` returns unmodified data.

### 6.3 Utility Tests

**File:** `packages/webui/src/lib/components/graph/__tests__/graph-utils.test.ts`

Test `getEdgeMidpoint` with mocked Sigma/Graph objects:

- Returns midpoint coordinates when both nodes have display data.
- Returns `null` when a node has no display data (edge with hidden node).

---

## 7. Acceptance Criteria

### Edge type coloring
- [ ] Edges with a `type` property render in the color specified by `config.edgeColorMap[edge.type]`
- [ ] Edges without a `type` fall back to `config.edgeDefaults.color`
- [ ] Edges with an explicit `color` property override the type-based color

### Weight-based thickness
- [ ] Edges with `weight: 3` render thicker than edges with `weight: 1`
- [ ] Weight is clamped to `[MIN_EDGE_SIZE, MAX_EDGE_SIZE]` range
- [ ] Edges without a `weight` property render at `DEFAULT_EDGE_SIZE`

### Arrow rendering
- [ ] Directed edges render with an arrowhead at the target node
- [ ] Arrow direction matches the `source -> target` relationship in the data
- [ ] Undirected edges (type: 'line') render without arrowheads

### Edge hover highlight
- [ ] Hovering an edge thickens it by `HIGHLIGHT_EDGE_SIZE_BUMP`
- [ ] Hovering an edge highlights its two endpoint nodes (kept at full opacity, foreground z-index)
- [ ] Hovering an edge dims all non-endpoint nodes to `DIM_NODE_COLOR`
- [ ] Hovering an edge dims all non-hovered edges to `DIM_EDGE_COLOR`
- [ ] Moving cursor away from edge restores normal rendering

### Edge midpoint utility
- [ ] `getEdgeMidpoint()` returns the midpoint in viewport coordinates
- [ ] `getEdgeMidpoint()` returns `null` for edges with hidden endpoints

### Tests
- [ ] `edgeSizeFromWeight` unit tests pass (4 cases)
- [ ] Reducer logic tests pass for edge hover node highlighting
- [ ] `getEdgeMidpoint` utility tests pass

---

## 8. Dependencies

- **Runtime:** `sigma`, `graphology` (already installed via H1)
- **Spec dependencies:** H1 (Generic GraphView Component) — this spec extends H1's reducers and constants
- **Blocked by:** Nothing beyond H1
- **Blocks:** No other specs directly, but H4 (Graph Filters) and H5 (Graph Search) benefit from visible edge differentiation
