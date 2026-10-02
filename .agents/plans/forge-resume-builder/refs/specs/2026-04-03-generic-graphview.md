# Generic GraphView Component

**Date:** 2026-04-03
**Spec:** H1 (Generic GraphView)
**Phase:** TBD (next available)
**Builds on:** Chain View (Phase 15), Chain View Edge Rendering (Phase 25), Chain View Modal (Phase 33)

## Overview

The current chain view (`ChainViewModal.svelte`, ~350 lines) tightly couples Sigma.js/graphology initialization, ForceAtlas2 layout, node/edge reducers, event wiring, and cleanup lifecycle to the derivation chain data model. Every future graph visualization (skill relationships, org-to-JD-to-resume mapping, source-to-bullet sub-graphs) would duplicate this boilerplate.

This spec extracts a generic `GraphView.svelte` component that accepts arbitrary nodes and edges, manages the Sigma.js WebGL lifecycle, runs configurable force-directed layout, and exposes interaction callbacks. The derivation chain view becomes a consumer that maps its domain data into the generic `GraphNode[]` / `GraphEdge[]` interface. Future consumers (H2-H7) plug in their own data and config without touching Sigma internals.

**Reference architecture:** The user's blog project implements this pattern in React with `GraphView.tsx` (core renderer), `GraphControllers.tsx` (drag, layout, theme, filter), `graph.types.ts`, `graph.constants.ts`, and `graph.shared.ts`. This spec adapts that architecture to Svelte 5 runes, replacing React hooks/refs with `$state`/`$effect`/`$derived` and React context with Svelte module-scope stores.

## Non-Goals

- **Filters, search, toolbar:** Deferred to H4 (GraphToolbar). The generic component renders nodes and edges and fires callbacks. Filtering logic lives in the consumer or a future toolbar wrapper.
- **Edge labels or animated flows:** Same as current chain view — too cluttered at scale.
- **Multiple simultaneous instances:** One `GraphView` per page/modal. No shared WebGL context pool.
- **Custom WebGL node/edge programs:** Use Sigma's built-in `circle` nodes and `arrow` edges. Custom programs (dashed edges, image nodes) are future work.
- **Dark mode:** The component reads CSS custom properties for theme awareness, but a full dark mode color scheme is out of scope here. Theme integration is limited to reading existing custom properties so the component adapts automatically when a theme system is added.
- **Accessibility:** Graph visualizations are inherently visual. Screen reader support for graph data is a separate concern. The component provides keyboard-navigable controls (zoom buttons) but does not attempt to make the WebGL canvas itself accessible.

---

## 1. Type Definitions

**File:** `packages/webui/src/lib/components/graph/graph.types.ts`

```typescript
/**
 * Generic graph node. The `type` field drives color/shape mapping via the
 * config's colorMap. Custom metadata can be attached via the index signature
 * and accessed in onNodeClick callbacks.
 */
export interface GraphNode {
  id: string
  label: string
  type: string
  size?: number
  color?: string
  x?: number
  y?: number
  [key: string]: unknown
}

/**
 * Generic graph edge. The `type` field drives styling via the config's
 * edgeColorMap (if provided). Weight influences edge thickness.
 */
export interface GraphEdge {
  id: string
  source: string
  target: string
  label?: string
  weight?: number
  type?: string
  color?: string
  [key: string]: unknown
}

/**
 * Force-directed layout parameters for ForceAtlas2.
 */
export interface ForceConfig {
  gravity: number
  scalingRatio: number
  slowDown: number
  barnesHutOptimize?: boolean  // auto-enabled when node count > barnesHutThreshold
  barnesHutThreshold?: number  // default: 100
  iterations?: number          // default: 100
}

/**
 * Full configuration for a GraphView instance.
 */
export interface GraphConfig {
  layout: 'forceatlas2' | 'circular' | 'random'
  forces: ForceConfig
  nodeDefaults: { size: number; color: string }
  edgeDefaults: { color: string; size: number; type: 'arrow' | 'line' }
  colorMap?: Record<string, string>     // node type -> color
  edgeColorMap?: Record<string, string> // edge type -> color
  labelThreshold: number                // zoom level to show labels
  enableDrag: boolean
  enableZoom: boolean
  enableEdgeEvents: boolean
  zIndex: boolean
}

/**
 * Props for the GraphView component.
 */
export interface GraphViewProps {
  nodes: GraphNode[]
  edges: GraphEdge[]
  config?: Partial<GraphConfig>
  highlightNode?: string | null
  onNodeClick?: (nodeId: string, data: GraphNode) => void
  onNodeHover?: (nodeId: string | null, data: GraphNode | null) => void
  onEdgeClick?: (edgeId: string, data: GraphEdge) => void
  onEdgeHover?: (edgeId: string | null) => void
  onStageClick?: () => void
}
```

### Relationship to Existing Types

The existing `ChainNode` and `ChainEdge` types in `$lib/graph/types.ts` remain unchanged. The chain view consumer maps `ChainNode` to `GraphNode` and `ChainEdge` to `GraphEdge`. The existing `NODE_COLORS` map becomes the `colorMap` in the chain view's `GraphConfig`.

---

## 2. Default Configuration

**File:** `packages/webui/src/lib/components/graph/graph.config.ts`

```typescript
import type { GraphConfig } from './graph.types'

export const DEFAULT_GRAPH_CONFIG: GraphConfig = {
  layout: 'forceatlas2',
  forces: {
    gravity: 1,
    scalingRatio: 10,
    slowDown: 1,
    barnesHutOptimize: false,  // auto-enabled per barnesHutThreshold
    barnesHutThreshold: 100,
    iterations: 100,
  },
  nodeDefaults: { size: 8, color: '#6b7280' },
  edgeDefaults: { color: '#94a3b8', size: 1, type: 'arrow' },
  colorMap: {},
  edgeColorMap: {},
  labelThreshold: 6,
  enableDrag: true,
  enableZoom: true,
  enableEdgeEvents: true,
  zIndex: true,
}

/**
 * Merge partial user config with defaults. Nested objects (forces, nodeDefaults,
 * edgeDefaults) are shallow-merged independently so that providing
 * `{ forces: { gravity: 2 } }` does not discard `scalingRatio` and `slowDown`.
 */
export function mergeConfig(partial?: Partial<GraphConfig>): GraphConfig {
  if (!partial) return { ...DEFAULT_GRAPH_CONFIG }

  return {
    ...DEFAULT_GRAPH_CONFIG,
    ...partial,
    forces: { ...DEFAULT_GRAPH_CONFIG.forces, ...partial.forces },
    nodeDefaults: { ...DEFAULT_GRAPH_CONFIG.nodeDefaults, ...partial.nodeDefaults },
    edgeDefaults: { ...DEFAULT_GRAPH_CONFIG.edgeDefaults, ...partial.edgeDefaults },
    colorMap: { ...DEFAULT_GRAPH_CONFIG.colorMap, ...partial.colorMap },
    edgeColorMap: { ...DEFAULT_GRAPH_CONFIG.edgeColorMap, ...partial.edgeColorMap },
  }
}
```

### Preset Configs

The config file also exports named presets for common graph shapes:

```typescript
/** Dense graphs: stronger gravity to prevent sprawl */
export const DENSE_GRAPH_CONFIG: Partial<GraphConfig> = {
  forces: { gravity: 3, scalingRatio: 5, slowDown: 2 },
}

/** Sparse/tree-like graphs: weaker gravity, higher scaling to spread branches */
export const TREE_GRAPH_CONFIG: Partial<GraphConfig> = {
  forces: { gravity: 0.5, scalingRatio: 20, slowDown: 1 },
}

/** Small graphs (<20 nodes): no Barnes-Hut, fewer iterations */
export const SMALL_GRAPH_CONFIG: Partial<GraphConfig> = {
  forces: { gravity: 1, scalingRatio: 10, slowDown: 1, iterations: 50 },
}
```

---

## 3. Constants

**File:** `packages/webui/src/lib/components/graph/graph.constants.ts`

Constants are browser-only (they reference CSS custom properties or compute values from the DOM). The file guards against SSR with `typeof window !== 'undefined'` checks.

```typescript
/**
 * Dimmed color for non-selected nodes/edges during selection highlighting.
 * Uses a light gray that works on both light backgrounds. When theme support
 * lands, this should read from a CSS custom property.
 */
export const DIM_NODE_COLOR = '#e5e7eb'
export const DIM_EDGE_COLOR = '#f3f4f6'
export const DIM_EDGE_SIZE = 0.5

/**
 * Selection highlight: connected edges get +1 size bump.
 */
export const HIGHLIGHT_EDGE_SIZE_BUMP = 1

/**
 * Z-index layers for selection highlighting.
 */
export const Z_FOREGROUND = 1
export const Z_BACKGROUND = 0

/**
 * Resolve a CSS custom property value from the document root.
 * Returns the fallback if running in SSR or the property is not set.
 *
 * Usage: resolveThemeColor('--graph-node-default', '#6b7280')
 */
export function resolveThemeColor(property: string, fallback: string): string {
  if (typeof window === 'undefined') return fallback
  const value = getComputedStyle(document.documentElement).getPropertyValue(property).trim()
  return value || fallback
}
```

### Theme Integration

The component does not hardcode colors directly. Node colors come from `config.colorMap` (provided by the consumer), edge colors from `config.edgeColorMap`, and defaults from `config.nodeDefaults.color` / `config.edgeDefaults.color`. Consumers can call `resolveThemeColor()` when building their config to read CSS custom properties set by a future design system:

```typescript
// Example consumer usage (chain view):
const config: Partial<GraphConfig> = {
  colorMap: {
    source: resolveThemeColor('--chain-source-color', '#6c63ff'),
    bullet: resolveThemeColor('--chain-bullet-color', '#3b82f6'),
    perspective: resolveThemeColor('--chain-perspective-color', '#10b981'),
  },
}
```

---

## 4. Component: `GraphView.svelte`

**File:** `packages/webui/src/lib/components/graph/GraphView.svelte`

### 4.1 Props

```typescript
let {
  nodes,
  edges,
  config: configOverride = undefined,
  highlightNode = null,
  onNodeClick = undefined,
  onNodeHover = undefined,
  onEdgeClick = undefined,
  onEdgeHover = undefined,
  onStageClick = undefined,
}: GraphViewProps = $props()
```

### 4.2 Internal State

```typescript
let container = $state<HTMLDivElement | null>(null)
let sigmaInstance: any = $state(null)
let graph = $state<Graph | null>(null)
let selectedNodeId = $state<string | null>(null)
let hoveredEdge = $state<string | null>(null)

// Merged config (recomputed when configOverride changes)
let resolvedConfig = $derived(mergeConfig(configOverride))
```

### 4.3 Graph Building

The component builds a graphology `Graph` from the `nodes` and `edges` props. This runs reactively whenever the data changes.

```typescript
$effect(() => {
  const g = new Graph({ type: 'directed', multi: false })

  for (const node of nodes) {
    g.addNode(node.id, {
      // Spread node FIRST so explicit Sigma attributes below take precedence.
      // The `type` field on `GraphNode` is the entity classification
      // (source/bullet/perspective). Sigma's node program type is stored
      // separately as `nodeType` to avoid collision — if `...node` came after,
      // `node.type` would overwrite the Sigma program type ('circle').
      ...node,
      label: node.label ?? node.id,
      x: node.x ?? Math.random() * 100,
      y: node.y ?? Math.random() * 100,
      size: node.size ?? resolvedConfig.nodeDefaults.size,
      color: node.color
        ?? resolvedConfig.colorMap?.[node.type]
        ?? resolvedConfig.nodeDefaults.color,
      nodeType: node.type,  // entity classification stored separately
      type: 'circle',       // Sigma node program type
    })
  }

  for (const edge of edges) {
    if (!g.hasNode(edge.source) || !g.hasNode(edge.target)) continue
    g.addEdge(edge.source, edge.target, {
      size: edge.weight ?? resolvedConfig.edgeDefaults.size,
      color: edge.color
        ?? (edge.type ? resolvedConfig.edgeColorMap?.[edge.type] : undefined)
        ?? resolvedConfig.edgeDefaults.color,
      type: resolvedConfig.edgeDefaults.type,
      ...edge,
    })
  }

  // Run layout BEFORE assigning to `graph` state. Do not set `graph = g`
  // until layout completes — this ensures Sigma only initializes after
  // layout is done. Use the local variable `g` throughout.
  if (g.order > 0) {
    await runLayout(g, resolvedConfig)
  }

  graph = g
})
```

### 4.4 Layout Execution

Layout runs synchronously after graph construction. ForceAtlas2 is dynamically imported to avoid SSR issues.

```typescript
async function runLayout(g: Graph, config: GraphConfig) {
  if (config.layout === 'forceatlas2') {
    const forceAtlas2 = await import('graphology-layout-forceatlas2')
    forceAtlas2.default.assign(g, {
      iterations: config.forces.iterations ?? 100,
      settings: {
        gravity: config.forces.gravity,
        scalingRatio: config.forces.scalingRatio,
        slowDown: config.forces.slowDown,
        barnesHutOptimize:
          config.forces.barnesHutOptimize
          ?? g.order > (config.forces.barnesHutThreshold ?? 100),
      },
    })
  } else if (config.layout === 'circular') {
    // If the `circular` layout option is used, `graphology-layout` must be
    // installed. Add a runtime check to fall back gracefully.
    try {
      const circular = await import('graphology-layout/circular')
      circular.default.assign(g)
    } catch {
      console.warn('graphology-layout not installed; falling back to random')
    }
  }
  // 'random' layout: nodes already have random positions from construction
}
```

> **Note:** `runLayout` is `async` because layout libraries are dynamically imported. The graph building `$effect` calls `runLayout` and then sets `graph = g`. Since Svelte effects are synchronous, the layout `await` completes before Sigma initialization (which depends on `graph` being non-null). If layout is truly expensive (>1000 nodes), a future enhancement could show a loading indicator during layout computation.

### 4.5 Sigma Initialization

Sigma is initialized when the container DOM element and graph are both ready. This mirrors the existing `ChainViewModal` dual-init pattern.

```typescript
$effect(() => {
  if (!browser || !container || !graph) return

  // When `graph` state changes (new data), the old Sigma instance must be
  // destroyed and re-created. This ensures reducers always reference the
  // current graph. Do NOT guard with `if (sigmaInstance) return`.
  if (sigmaInstance) {
    sigmaInstance.kill()
    sigmaInstance = null
  }

  async function initSigma() {
    const { default: Sigma } = await import('sigma')

    const instance = new Sigma(graph!, container!, {
      renderEdgeLabels: false,
      enableEdgeEvents: resolvedConfig.enableEdgeEvents,
      defaultNodeType: 'circle',
      defaultEdgeType: resolvedConfig.edgeDefaults.type,
      zIndex: resolvedConfig.zIndex,
      labelRenderedSizeThreshold: resolvedConfig.labelThreshold,

      nodeReducer: (node: string, data: Record<string, any>) => {
        if (!selectedNodeId) return data
        if (
          node === selectedNodeId
          || graph!.hasEdge(selectedNodeId, node)
          || graph!.hasEdge(node, selectedNodeId)
        ) {
          return { ...data, zIndex: Z_FOREGROUND }
        }
        return { ...data, color: DIM_NODE_COLOR, label: '', zIndex: Z_BACKGROUND }
      },

      edgeReducer: (edge: string, data: Record<string, any>) => {
        const [src, tgt] = graph!.extremities(edge)
        const srcAttrs = graph!.getNodeAttributes(src)
        const tgtAttrs = graph!.getNodeAttributes(tgt)

        if (srcAttrs.hidden || tgtAttrs.hidden) {
          return { ...data, hidden: true }
        }

        if (selectedNodeId) {
          if (src === selectedNodeId || tgt === selectedNodeId) {
            return {
              ...data,
              size: (data.size ?? 1) + HIGHLIGHT_EDGE_SIZE_BUMP,
              zIndex: Z_FOREGROUND,
            }
          }
          return { ...data, color: DIM_EDGE_COLOR, size: DIM_EDGE_SIZE, zIndex: Z_BACKGROUND }
        }

        if (hoveredEdge === edge) {
          return { ...data, size: (data.size ?? 1) + HIGHLIGHT_EDGE_SIZE_BUMP }
        }

        return data
      },
    })

    // ----- Event handlers -----

    instance.on('clickNode', ({ node }: { node: string }) => {
      selectedNodeId = node
      const attrs = graph!.getNodeAttributes(node)
      onNodeClick?.(node, attrs as GraphNode)
    })

    instance.on('enterNode', ({ node }: { node: string }) => {
      const attrs = graph!.getNodeAttributes(node)
      onNodeHover?.(node, attrs as GraphNode)
    })

    instance.on('leaveNode', () => {
      onNodeHover?.(null, null)
    })

    if (resolvedConfig.enableEdgeEvents) {
      instance.on('clickEdge', ({ edge }: { edge: string }) => {
        selectedNodeId = null
        const attrs = graph!.getEdgeAttributes(edge)
        onEdgeClick?.(edge, attrs as GraphEdge)
      })

      instance.on('enterEdge', ({ edge }: { edge: string }) => {
        hoveredEdge = edge
        onEdgeHover?.(edge)
      })

      instance.on('leaveEdge', () => {
        hoveredEdge = null
        onEdgeHover?.(null)
      })
    }

    instance.on('clickStage', () => {
      selectedNodeId = null
      onStageClick?.()
    })

    // ----- Highlight on open -----

    if (highlightNode && graph!.hasNode(highlightNode)) {
      selectedNodeId = highlightNode
      const displayData = instance.getNodeDisplayData(highlightNode)
      if (displayData) {
        instance.getCamera().animate(
          { x: displayData.x, y: displayData.y, ratio: 0.3 },
          { duration: 300 },
        )
      }
      const attrs = graph!.getNodeAttributes(highlightNode)
      onNodeClick?.(highlightNode, attrs as GraphNode)
    }

    sigmaInstance = instance
  }

  initSigma()

  return () => {
    if (sigmaInstance) {
      sigmaInstance.kill()
      sigmaInstance = null
    }
  }
})
```

### 4.6 Refresh on State Changes

Sigma's reducers read `selectedNodeId` and `hoveredEdge` from closure scope. When these change, Sigma must re-render to apply the visual updates:

```typescript
$effect(() => {
  // Track reactive values to trigger refresh
  const _sel = selectedNodeId
  const _hover = hoveredEdge
  sigmaInstance?.refresh()
})
```

### 4.7 Cleanup Lifecycle

Matches the Phase 33 dual-cleanup pattern:

1. **`$effect` return** (primary): handles cleanup on re-runs. When nodes/edges change, the graph-building effect runs cleanup before rebuilding. When the Sigma init effect re-runs, it kills the old instance first.
2. **`onDestroy`** (backup): handles abrupt component unmount (e.g., modal close, navigation).

```typescript
onDestroy(() => {
  if (sigmaInstance) {
    sigmaInstance.kill()
    sigmaInstance = null
  }
})
```

### 4.8 Responsive Container

The component renders a `<div>` that fills its parent container. The parent is responsible for sizing.

```svelte
<div class="graph-view" bind:this={container}></div>

<style>
  .graph-view {
    width: 100%;
    height: 100%;
    min-height: 300px;
    position: relative;
  }
</style>
```

The parent sets the dimensions. For the chain view modal, the parent is a flex container that stretches the graph to fill available space. For inline usage, the parent sets an explicit height.

### 4.9 Drag Support

When `config.enableDrag` is `true`, the component enables Sigma's built-in node dragging. The drag controller updates node positions in the graphology model so that ForceAtlas2's gravity does not fight the user's placement.

```typescript
// Inside initSigma(), after instance creation:
if (resolvedConfig.enableDrag) {
  let draggedNode: string | null = null
  let isDragging = false

  instance.on('downNode', ({ node }: { node: string }) => {
    isDragging = true
    draggedNode = node
    graph!.setNodeAttribute(node, 'fixed', true)
  })

  // NOTE: Verify Sigma v3 mouse captor event names. The existing
  // ChainViewModal does not use drag. Test `getMouseCaptor().on('mousemove', ...)`
  // vs `'mousemovebody'` against the installed Sigma version.
  instance.getMouseCaptor().on('mousemovebody', (e: any) => {
    if (!isDragging || !draggedNode) return
    const pos = instance.viewportToGraph(e)
    graph!.setNodeAttribute(draggedNode, 'x', pos.x)
    graph!.setNodeAttribute(draggedNode, 'y', pos.y)
  })

  instance.getMouseCaptor().on('mouseup', () => {
    if (draggedNode) {
      graph!.removeNodeAttribute(draggedNode, 'fixed')
    }
    isDragging = false
    draggedNode = null
  })
}
```

---

## 5. Consumer Migration: Chain View

After `GraphView.svelte` exists, `ChainViewModal.svelte` is refactored to become a consumer. The migration is not part of this spec's implementation scope (it is tracked separately) but the approach is documented here to validate the interface.

### Before (current)

`ChainViewModal.svelte` directly imports `graphology`, `sigma`, `graphology-layout-forceatlas2`, creates a `Graph`, wires event handlers, manages `selectedNodeId`, `hoveredEdge`, node/edge reducers, and cleanup lifecycle. Approximately 350 lines of graph infrastructure mixed with chain-specific data fetching and detail panel rendering.

### After (target)

```svelte
<script lang="ts">
  import GraphView from '$lib/components/graph/GraphView.svelte'
  import type { GraphNode, GraphEdge, GraphConfig } from '$lib/components/graph/graph.types'
  import { NODE_COLORS } from '$lib/graph/types'
  // ... chain-specific state, data fetching, detail panels ...

  // Map chain data to generic graph data
  let graphNodes = $derived<GraphNode[]>(buildChainNodes(allSources, allBullets, allPerspectives))
  let graphEdges = $derived<GraphEdge[]>(buildChainEdges(allBullets, allPerspectives, allSources))

  const chainConfig: Partial<GraphConfig> = {
    colorMap: NODE_COLORS,
    edgeColorMap: { matching: '#94a3b8', drifted: '#ef4444' },
    forces: { gravity: 1, scalingRatio: 10, slowDown: 1 },
  }

  function handleNodeClick(nodeId: string, data: GraphNode) {
    selectedNodeData = { /* map from GraphNode to ChainNode */ }
    selectedEdgeData = null
  }
</script>

<!-- Chain-specific UI: controls, stats, filters -->
<div class="graph-container">
  <GraphView
    nodes={graphNodes}
    edges={graphEdges}
    config={chainConfig}
    {highlightNode}
    onNodeClick={handleNodeClick}
    onEdgeClick={handleEdgeClick}
    onStageClick={handleStageClick}
  />
</div>
<!-- Chain-specific detail panels, edge tooltips, etc. -->
```

The chain view retains ownership of:
- Data fetching (`forge.sources.list()`, etc.)
- Domain-specific data mapping (`buildChainNodes`, `buildChainEdges`)
- Detail panels (node detail, edge detail, related entities)
- Edge tooltip positioning and content
- Filter controls and search

The generic `GraphView` owns:
- Sigma.js lifecycle (init, kill, refresh)
- Graphology graph construction from props
- ForceAtlas2 layout
- Node/edge reducers for selection highlighting
- Drag support
- Event delegation to callbacks

---

## 6. Files to Create

| File | Description |
|------|-------------|
| `packages/webui/src/lib/components/graph/GraphView.svelte` | Generic Sigma.js graph renderer component |
| `packages/webui/src/lib/components/graph/graph.types.ts` | `GraphNode`, `GraphEdge`, `GraphConfig`, `GraphViewProps` type definitions |
| `packages/webui/src/lib/components/graph/graph.config.ts` | Default config, `mergeConfig()` utility, preset configs |
| `packages/webui/src/lib/components/graph/graph.constants.ts` | Dim colors, z-index constants, `resolveThemeColor()` |

## 7. Files to Modify

None. This spec is foundational — it creates new files only. Consumers (chain view, future graph views) are modified in their own specs. The existing `$lib/graph/types.ts` (`ChainNode`, `ChainEdge`, `NODE_COLORS`) remains unchanged.

---

## 8. Testing Approach

### 8.1 Unit Tests

**File:** `packages/webui/src/lib/components/graph/__tests__/graph-config.test.ts`

```typescript
import { describe, it, expect } from 'vitest'
import { mergeConfig, DEFAULT_GRAPH_CONFIG } from '../graph.config'

describe('mergeConfig', () => {
  it('returns defaults when no override provided', () => {
    const config = mergeConfig()
    expect(config).toEqual(DEFAULT_GRAPH_CONFIG)
  })

  it('shallow-merges nested forces without losing defaults', () => {
    const config = mergeConfig({ forces: { gravity: 5 } })
    expect(config.forces.gravity).toBe(5)
    expect(config.forces.scalingRatio).toBe(10)  // preserved from default
    expect(config.forces.slowDown).toBe(1)        // preserved from default
  })

  it('merges colorMap additively', () => {
    const config = mergeConfig({ colorMap: { source: '#ff0000' } })
    expect(config.colorMap).toEqual({ source: '#ff0000' })
  })

  it('overrides top-level scalars', () => {
    const config = mergeConfig({ layout: 'circular', labelThreshold: 10 })
    expect(config.layout).toBe('circular')
    expect(config.labelThreshold).toBe(10)
    expect(config.enableDrag).toBe(true)  // unchanged default
  })
})
```

### 8.2 Graph Building Tests

**File:** `packages/webui/src/lib/components/graph/__tests__/graph-types.test.ts`

Type-level tests ensuring the interfaces compile correctly and satisfy expected shapes. Vitest `expectTypeOf` or simple runtime checks:

```typescript
import { describe, it, expect } from 'vitest'
import type { GraphNode, GraphEdge } from '../graph.types'

describe('GraphNode', () => {
  it('accepts minimal node', () => {
    const node: GraphNode = { id: '1', label: 'Test', type: 'source' }
    expect(node.id).toBe('1')
  })

  it('accepts node with custom metadata', () => {
    const node: GraphNode = {
      id: '1', label: 'Test', type: 'source',
      entityId: 'abc', status: 'approved',
    }
    expect(node.entityId).toBe('abc')
  })
})

describe('GraphEdge', () => {
  it('accepts minimal edge', () => {
    const edge: GraphEdge = { id: 'e1', source: '1', target: '2' }
    expect(edge.source).toBe('1')
  })
})
```

### 8.3 Component Tests

Sigma.js requires a WebGL context, which is not available in jsdom/happy-dom. Component tests for `GraphView.svelte` use one of two strategies:

1. **Mock Sigma:** Replace `import('sigma')` with a mock that records constructor calls and event registrations. Verify that the component calls `new Sigma(graph, container, config)` with the expected config, registers the expected events, and calls `kill()` on cleanup.

2. **Browser-based tests (Playwright):** Mount the component in a real browser and verify visual output. Deferred to integration testing in a later spec.

For this spec, Strategy 1 is implemented:

```typescript
// packages/webui/src/lib/components/graph/__tests__/GraphView.test.ts
import { describe, it, expect, vi } from 'vitest'

describe('GraphView', () => {
  it('creates a graphology Graph with provided nodes and edges', async () => {
    // Test the graph-building logic extracted into a testable function
    // (The actual Sigma rendering is mocked)
  })

  it('applies colorMap to node colors', () => {
    // Verify node color resolution: explicit > colorMap > default
  })

  it('skips edges with missing source/target nodes', () => {
    // Edge referencing non-existent node should be silently dropped
  })
})
```

### 8.4 Smoke Test

Extend the existing graphology smoke test at `packages/webui/src/lib/graph/smoke.test.ts` with a case that uses the new `GraphNode` / `GraphEdge` types to build a graphology graph, validating the data flow from generic types to graphology.

---

## 9. Acceptance Criteria

### Component creation
- [ ] `GraphView.svelte` renders a `<div>` that fills its parent container
- [ ] Component accepts `nodes`, `edges`, `config`, `highlightNode`, and callback props
- [ ] Component creates a graphology `Graph` from `nodes` and `edges` arrays
- [ ] Nodes with missing `x`/`y` receive random positions
- [ ] Edges referencing non-existent nodes are silently skipped (no crash)

### Layout
- [ ] ForceAtlas2 runs with the provided force configuration
- [ ] `barnesHutOptimize` auto-enables when node count exceeds `barnesHutThreshold`
- [ ] `circular` layout assigns circular positions
- [ ] `random` layout uses the initial random positions (no additional computation)

### Rendering
- [ ] Sigma.js initializes with WebGL rendering (browser-only, guarded by `$app/environment`)
- [ ] Node colors resolve in priority order: `node.color` > `config.colorMap[node.type]` > `config.nodeDefaults.color`
- [ ] Edge colors resolve in priority order: `edge.color` > `config.edgeColorMap[edge.type]` > `config.edgeDefaults.color`
- [ ] Labels appear/hide based on `config.labelThreshold` zoom level
- [ ] `zIndex: true` is passed to Sigma when `config.zIndex` is true

### Selection highlighting
- [ ] Clicking a node dims non-connected nodes (color → `DIM_NODE_COLOR`, label removed)
- [ ] Connected edges become thicker by `HIGHLIGHT_EDGE_SIZE_BUMP`
- [ ] Non-connected edges dim to `DIM_EDGE_COLOR` / `DIM_EDGE_SIZE`
- [ ] Clicking the stage (background) clears selection
- [ ] Edge hover thickens the hovered edge

### Event callbacks
- [ ] `onNodeClick` fires with `(nodeId, nodeData)` when a node is clicked
- [ ] `onNodeHover` fires with `(nodeId, nodeData)` on enter and `(null, null)` on leave
- [ ] `onEdgeClick` fires with `(edgeId, edgeData)` when an edge is clicked
- [ ] `onEdgeHover` fires with `(edgeId)` on enter and `(null)` on leave
- [ ] `onStageClick` fires when background is clicked
- [ ] Edge events only fire when `config.enableEdgeEvents` is true

### Highlight on open
- [ ] When `highlightNode` prop is provided and the node exists, camera animates to it
- [ ] The highlighted node is selected (triggers `onNodeClick`)

### Drag support
- [ ] When `config.enableDrag` is true, nodes can be dragged to new positions
- [ ] Dragged node positions update in the graphology model
- [ ] Mouse up releases the drag

### Lifecycle
- [ ] `$effect` return kills Sigma instance on re-run (data change)
- [ ] `onDestroy` kills Sigma instance on component unmount
- [ ] No WebGL context leaks after mounting/unmounting the component 5 times

### Configuration
- [ ] `mergeConfig()` shallow-merges nested objects without losing unspecified defaults
- [ ] Preset configs (`DENSE_GRAPH_CONFIG`, `TREE_GRAPH_CONFIG`, `SMALL_GRAPH_CONFIG`) are importable

### SSR safety
- [ ] Component renders correctly in SvelteKit SPA mode with no SSR errors. Dynamic imports for Sigma and graphology prevent server-side evaluation.

### Tests
- [ ] `mergeConfig` unit tests pass (4 cases: no override, nested merge, additive colorMap, scalar override)
- [ ] Type-level tests compile and pass for `GraphNode` and `GraphEdge`
- [ ] Graphology smoke test extended to use `GraphNode` / `GraphEdge` types

---

## 10. Dependencies

- **Runtime:** `sigma`, `graphology`, `graphology-layout-forceatlas2`, `graphology-types` (already installed in `packages/webui`)
- **Optional (for circular layout):** `graphology-layout` (must be installed if circular layout is used)
- **Spec dependencies:** None. This is foundational.
- **Blocks:** H2-H7 (all graph sub-specs):
  - H2: GraphToolbar (filter/search controls wrapping GraphView)
  - H3: Skill Relationship Graph (consumer)
  - H4: Org-JD-Resume Graph (consumer)
  - H5: Source-Bullet Sub-Graph (consumer)
  - H6: Graph Search + Filter Controllers
  - H7: Graph Export (PNG/SVG)
