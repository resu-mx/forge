# Phase 51: Edge Rendering (Spec H2)

**Status:** Planning
**Date:** 2026-04-03
**Spec:** [2026-04-03-graph-edge-rendering.md](../refs/specs/2026-04-03-graph-edge-rendering.md)
**Depends on:** Phase 48 (Generic GraphView)
**Blocks:** None directly (H4 filters and H5 search benefit from visible edge differentiation)
**Parallelizable with:** Phase 52, Phase 53, Phase 56 -- no file conflicts

## Goal

Add edge visual differentiation to the `GraphView` component: type-based coloring via `edgeColorMap`, weight-based thickness via `edgeSizeFromWeight()`, edge-hover node highlighting (endpoint nodes stay visible while non-endpoints dim), and a `getEdgeMidpoint()` utility for consumer-side tooltip positioning. Users can read the graph's edge structure at a glance without clicking individual edges.

## Non-Goals

- Edge editing or creation UI
- Animated edge flows or particle effects
- Edge labels rendered on the WebGL canvas (tooltip on hover only)
- Custom WebGL edge programs (dashed, curved)
- Edge type filtering (handled by H4)

## Context

Phase 48's `GraphView.svelte` renders all edges identically -- same color, same thickness, simple arrow type. The `edgeReducer` handles selection and hover highlighting, but does not differentiate edges by type or weight. The `GraphConfig` already declares `edgeColorMap` and `GraphEdge` already has `weight` and `type` fields -- this phase wires them into the rendering pipeline.

H1's `edgeReducer` already handles `hoveredEdge` for the edge itself (size bump, dimming other edges). This phase extends the `nodeReducer` to highlight endpoint nodes when an edge is hovered, and adds `edgeSizeFromWeight()` to the graph construction loop.

## Scope

| Spec section | Covered here? |
|-------------|---------------|
| 1. Edge Type Styling (color by type, weight-based thickness, arrow styles) | Yes |
| 2. Edge Hover Highlight (endpoint node highlighting, tooltip midpoint) | Yes |
| 3. Component Interface Changes (no new props needed) | Yes |
| 4. New exports (`edgeSizeFromWeight`, `getEdgeMidpoint`) | Yes |
| 5. Testing | Yes |

## Files to Create

| File | Description |
|------|-------------|
| `packages/webui/src/lib/components/graph/graph.utils.ts` | `getEdgeMidpoint()` utility for tooltip positioning |
| `packages/webui/src/lib/components/graph/__tests__/graph-edge-rendering.test.ts` | Unit tests for `edgeSizeFromWeight` |
| `packages/webui/src/lib/components/graph/__tests__/graph-utils.test.ts` | Unit tests for `getEdgeMidpoint` |

## Files to Modify

| File | Description |
|------|-------------|
| `packages/webui/src/lib/components/graph/graph.constants.ts` | Add `MIN_EDGE_SIZE`, `MAX_EDGE_SIZE`, `DEFAULT_EDGE_SIZE`, `edgeSizeFromWeight()` |
| `packages/webui/src/lib/components/graph/GraphView.svelte` | Use `edgeSizeFromWeight()` in edge construction; extend `nodeReducer` for edge-hover endpoint highlighting |

## Fallback Strategies

- **`edgeSizeFromWeight` receives NaN or negative weight:** The `Math.max(MIN_EDGE_SIZE, ...)` clamp ensures a minimum visible size. `NaN` comparisons return `false` in `Math.max`/`Math.min`, so `edgeSizeFromWeight(NaN)` falls through to `DEFAULT_EDGE_SIZE` via the `undefined` check. Add explicit `isNaN` guard for safety.
- **`getEdgeMidpoint` called before Sigma init:** Returns `null` when `sigma.getNodeDisplayData()` returns falsy. Consumer tooltip code must handle `null`.
- **`graph.extremities()` fails on invalid edge ID:** Wrap in try/catch inside the reducer; return unmodified data on error.

---

## Tasks

### T51.1: Add Edge Size Constants and Utility

**File:** `packages/webui/src/lib/components/graph/graph.constants.ts`

[IMPORTANT] `edgeSizeFromWeight` must handle `NaN`, negative values, and `0` gracefully. All return clamped values within `[MIN_EDGE_SIZE, MAX_EDGE_SIZE]`.

```typescript
// --- Add to graph.constants.ts ---

/**
 * Minimum edge rendering size. Prevents invisible edges.
 */
export const MIN_EDGE_SIZE = 0.5

/**
 * Maximum edge rendering size. Prevents oversized edges.
 */
export const MAX_EDGE_SIZE = 5

/**
 * Default edge size when no weight is provided.
 */
export const DEFAULT_EDGE_SIZE = 1

/**
 * Compute edge display size from weight.
 * Clamps to [MIN_EDGE_SIZE, MAX_EDGE_SIZE] to prevent invisible or oversized edges.
 * Returns DEFAULT_EDGE_SIZE for undefined or NaN weights.
 */
export function edgeSizeFromWeight(weight: number | undefined): number {
  if (weight === undefined || isNaN(weight)) return DEFAULT_EDGE_SIZE
  return Math.max(MIN_EDGE_SIZE, Math.min(MAX_EDGE_SIZE, weight))
}
```

**Acceptance criteria:**
- `edgeSizeFromWeight(undefined)` returns `DEFAULT_EDGE_SIZE` (1).
- `edgeSizeFromWeight(0.1)` returns `MIN_EDGE_SIZE` (0.5).
- `edgeSizeFromWeight(100)` returns `MAX_EDGE_SIZE` (5).
- `edgeSizeFromWeight(2)` returns `2`.
- `edgeSizeFromWeight(NaN)` returns `DEFAULT_EDGE_SIZE`.
- `edgeSizeFromWeight(0)` returns `MIN_EDGE_SIZE`.

**Failure criteria:**
- `NaN` input causes unexpected return value or crash.

---

### T51.2: Write `getEdgeMidpoint` Utility

**File:** `packages/webui/src/lib/components/graph/graph.utils.ts`

[IMPORTANT] Uses `sigma.graphToViewport()` to convert graph-space coordinates to screen pixels. Without this conversion, tooltip positions would be wrong because display data coordinates are in graph space, not screen pixels.

```typescript
import type Sigma from 'sigma'
import type Graph from 'graphology'

/**
 * Get the midpoint of an edge in viewport (screen) coordinates.
 * Useful for positioning tooltips near hovered edges.
 *
 * Returns null if either endpoint node has no display data (e.g., hidden node).
 */
export function getEdgeMidpoint(
  sigma: Sigma,
  graph: Graph,
  edgeId: string
): { x: number; y: number } | null {
  try {
    const [src, tgt] = graph.extremities(edgeId)
    const srcDisplay = sigma.getNodeDisplayData(src)
    const tgtDisplay = sigma.getNodeDisplayData(tgt)
    if (!srcDisplay || !tgtDisplay) return null

    // Compute midpoint in graph space, then convert to viewport (screen) pixels.
    const midGraph = {
      x: (srcDisplay.x + tgtDisplay.x) / 2,
      y: (srcDisplay.y + tgtDisplay.y) / 2,
    }
    return sigma.graphToViewport(midGraph)
  } catch {
    return null
  }
}
```

**Acceptance criteria:**
- Returns `{ x, y }` when both endpoints have display data.
- Returns `null` when either endpoint has no display data.
- Returns `null` for invalid edge IDs (try/catch handles `extremities` error).
- Uses `sigma.graphToViewport()` for correct screen-space coordinates.

**Failure criteria:**
- Returns graph-space coordinates instead of viewport coordinates (wrong tooltip position).
- Throws on invalid edge ID instead of returning `null`.

---

### T51.3: Extend Graph Construction for Weight-Based Size

**File:** `packages/webui/src/lib/components/graph/GraphView.svelte`

[CRITICAL] The edge construction loop in the `$effect` must use `edgeSizeFromWeight(edge.weight)` instead of `edge.weight ?? _config.edgeDefaults.size`. This ensures clamping and default handling.

Modify the edge-adding section of the graph-building `$effect`:

```typescript
// --- In GraphView.svelte, inside the buildAndAssign() function ---
// Replace:
//   size: edge.weight ?? _config.edgeDefaults.size,
// With:
import { edgeSizeFromWeight } from './graph.constants'

// In the edge loop:
for (const edge of _edges) {
  if (!g.hasNode(edge.source) || !g.hasNode(edge.target)) continue
  g.addEdge(edge.source, edge.target, {
    ...edge,
    size: edgeSizeFromWeight(edge.weight),  // <-- changed
    color: edge.color
      ?? (edge.type ? _config.edgeColorMap?.[edge.type] : undefined)
      ?? _config.edgeDefaults.color,
    type: _config.edgeDefaults.type,
    edgeType: edge.type,
  })
}
```

**Acceptance criteria:**
- Edges with `weight: 3` render with `size: 3`.
- Edges without `weight` render with `size: 1` (DEFAULT_EDGE_SIZE).
- Edges with `weight: 100` are clamped to `size: 5`.
- `edgeSizeFromWeight` import does not break SSR (pure function, no browser APIs).

**Failure criteria:**
- `edgeSizeFromWeight` is not called during edge construction (weight has no visual effect).

---

### T51.4: Extend `nodeReducer` for Edge-Hover Endpoint Highlighting

**File:** `packages/webui/src/lib/components/graph/GraphView.svelte`

[IMPORTANT] Edge-hover highlighting does NOT dim the currently selected node. The `selectedNodeId` check takes priority. The `hoveredEdge` branch only executes when `selectedNodeId` is null.

[MINOR] The `graph.extremities()` call in the reducer uses the `graph!` non-null assertion because the reducer only runs when Sigma is initialized, which requires `graph` to be non-null.

Extend the `nodeReducer` in the Sigma initialization block:

```typescript
// --- In GraphView.svelte, inside initSigma() ---
// Replace the existing nodeReducer with:
nodeReducer: (node: string, data: Record<string, any>) => {
  // Selection highlighting takes priority
  if (selectedNodeId) {
    if (
      node === selectedNodeId
      || graph!.hasEdge(selectedNodeId, node)
      || graph!.hasEdge(node, selectedNodeId)
    ) {
      return { ...data, zIndex: Z_FOREGROUND }
    }
    return { ...data, color: DIM_NODE_COLOR, label: '', zIndex: Z_BACKGROUND }
  }

  // Edge hover: highlight endpoint nodes
  if (hoveredEdge && graph) {
    try {
      const [src, tgt] = graph.extremities(hoveredEdge)
      if (node === src || node === tgt) {
        return { ...data, zIndex: Z_FOREGROUND }
      }
      return { ...data, color: DIM_NODE_COLOR, label: '', zIndex: Z_BACKGROUND }
    } catch {
      // Invalid edge ID — return unmodified data
      return data
    }
  }

  return data
},
```

**Acceptance criteria:**
- Hovering an edge highlights its two endpoint nodes (z-index foreground).
- Hovering an edge dims all non-endpoint nodes (DIM_NODE_COLOR, hidden labels).
- Selection highlighting takes priority over edge hover.
- Moving cursor away from edge restores normal rendering.
- Invalid `hoveredEdge` value does not crash the reducer.

**Failure criteria:**
- Edge hover dims the selected node (priority violation).
- `graph.extremities()` throws and crashes the reducer.

---

### T51.5: Write `edgeSizeFromWeight` Unit Tests

**File:** `packages/webui/src/lib/components/graph/__tests__/graph-edge-rendering.test.ts`

```typescript
import { describe, it, expect } from 'vitest'
import {
  edgeSizeFromWeight,
  MIN_EDGE_SIZE,
  MAX_EDGE_SIZE,
  DEFAULT_EDGE_SIZE,
} from '../graph.constants'

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

  it('returns DEFAULT_EDGE_SIZE for NaN', () => {
    expect(edgeSizeFromWeight(NaN)).toBe(DEFAULT_EDGE_SIZE)
  })

  it('clamps zero to MIN_EDGE_SIZE', () => {
    expect(edgeSizeFromWeight(0)).toBe(MIN_EDGE_SIZE)
  })

  it('clamps negative values to MIN_EDGE_SIZE', () => {
    expect(edgeSizeFromWeight(-5)).toBe(MIN_EDGE_SIZE)
  })

  it('returns exact boundary values', () => {
    expect(edgeSizeFromWeight(MIN_EDGE_SIZE)).toBe(MIN_EDGE_SIZE)
    expect(edgeSizeFromWeight(MAX_EDGE_SIZE)).toBe(MAX_EDGE_SIZE)
  })
})

describe('edge size constants', () => {
  it('MIN_EDGE_SIZE is 0.5', () => {
    expect(MIN_EDGE_SIZE).toBe(0.5)
  })

  it('MAX_EDGE_SIZE is 5', () => {
    expect(MAX_EDGE_SIZE).toBe(5)
  })

  it('DEFAULT_EDGE_SIZE is 1', () => {
    expect(DEFAULT_EDGE_SIZE).toBe(1)
  })

  it('MIN < DEFAULT < MAX', () => {
    expect(MIN_EDGE_SIZE).toBeLessThan(DEFAULT_EDGE_SIZE)
    expect(DEFAULT_EDGE_SIZE).toBeLessThan(MAX_EDGE_SIZE)
  })
})
```

**Acceptance criteria:**
- All 12 test cases pass.
- Edge cases (NaN, 0, negative, boundaries) are covered.

---

### T51.6: Write `getEdgeMidpoint` Unit Tests

**File:** `packages/webui/src/lib/components/graph/__tests__/graph-utils.test.ts`

```typescript
import { describe, it, expect, vi } from 'vitest'
import { getEdgeMidpoint } from '../graph.utils'

function createMockSigma(
  nodeDisplayData: Record<string, { x: number; y: number } | undefined>
) {
  return {
    getNodeDisplayData: vi.fn((nodeId: string) => nodeDisplayData[nodeId] ?? undefined),
    graphToViewport: vi.fn(({ x, y }: { x: number; y: number }) => ({
      x: x * 2,  // simple transform for testing
      y: y * 2,
    })),
  } as any
}

function createMockGraph(edges: Record<string, [string, string]>) {
  return {
    extremities: vi.fn((edgeId: string) => {
      const result = edges[edgeId]
      if (!result) throw new Error(`Edge not found: ${edgeId}`)
      return result
    }),
  } as any
}

describe('getEdgeMidpoint', () => {
  it('returns viewport midpoint when both nodes have display data', () => {
    const sigma = createMockSigma({
      nodeA: { x: 10, y: 20 },
      nodeB: { x: 30, y: 40 },
    })
    const graph = createMockGraph({ edge1: ['nodeA', 'nodeB'] })

    const result = getEdgeMidpoint(sigma, graph, 'edge1')

    expect(result).toEqual({ x: 40, y: 60 })  // midpoint (20,30) * 2
    expect(sigma.graphToViewport).toHaveBeenCalledWith({ x: 20, y: 30 })
  })

  it('returns null when source node has no display data', () => {
    const sigma = createMockSigma({
      nodeA: undefined,
      nodeB: { x: 30, y: 40 },
    })
    const graph = createMockGraph({ edge1: ['nodeA', 'nodeB'] })

    expect(getEdgeMidpoint(sigma, graph, 'edge1')).toBeNull()
  })

  it('returns null when target node has no display data', () => {
    const sigma = createMockSigma({
      nodeA: { x: 10, y: 20 },
      nodeB: undefined,
    })
    const graph = createMockGraph({ edge1: ['nodeA', 'nodeB'] })

    expect(getEdgeMidpoint(sigma, graph, 'edge1')).toBeNull()
  })

  it('returns null for non-existent edge ID', () => {
    const sigma = createMockSigma({})
    const graph = createMockGraph({})

    expect(getEdgeMidpoint(sigma, graph, 'nonexistent')).toBeNull()
  })
})
```

**Acceptance criteria:**
- All 4 test cases pass.
- `graphToViewport` is verified to be called with graph-space midpoint.
- Null returns are verified for missing display data and invalid edges.

---

## Testing Support

| Test file | Test count | Type |
|-----------|-----------|------|
| `__tests__/graph-edge-rendering.test.ts` | 12 | Unit |
| `__tests__/graph-utils.test.ts` | 4 | Unit |
| **Total** | **16** | |

**Run command:** `cd packages/webui && npx vitest run src/lib/components/graph/__tests__/graph-edge-rendering.test.ts src/lib/components/graph/__tests__/graph-utils.test.ts`

## Documentation Requirements

- Add `edgeSizeFromWeight`, `MIN_EDGE_SIZE`, `MAX_EDGE_SIZE`, `DEFAULT_EDGE_SIZE` to the `graph.constants.ts` module exports section in any existing barrel file.
- Add `getEdgeMidpoint` to the `graph.utils.ts` module exports.
- No new user-facing docs (internal component enhancement).

## Parallelization Notes

- T51.1 and T51.2 are independent (different files) -- can run in parallel.
- T51.3 and T51.4 both modify `GraphView.svelte` -- must run sequentially.
- T51.5 and T51.6 are independent test files -- can run in parallel.
- This entire phase is parallelizable with Phase 52, 53, and 56 (no shared file modifications beyond `GraphView.svelte` where changes are in distinct sections).
