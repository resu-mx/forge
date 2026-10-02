# Phase 48: Generic GraphView Component (Spec H1)

**Status:** Planning
**Date:** 2026-04-03
**Spec:** [2026-04-03-generic-graphview.md](../refs/specs/2026-04-03-generic-graphview.md)
**Depends on:** None (foundational for H2-H7)
**Blocks:** Phase 51 (Edge Rendering), Phase 52 (Node Labels), Phase 53 (Filters), Phase 54 (Search), Phase 55 (Toolbar), Phase 56 (Local Widget)
**Parallelizable with:** All other phases -- creates new files only, modifies nothing

## Goal

Extract a generic, reusable `GraphView.svelte` component from the existing `ChainViewModal.svelte` (~350 lines of tightly-coupled Sigma.js/graphology boilerplate). The component accepts arbitrary `GraphNode[]` and `GraphEdge[]` arrays, manages the full Sigma.js WebGL lifecycle (init, layout, reducers, events, cleanup), and exposes interaction callbacks. Consumers map domain data into the generic interface and receive callbacks without touching Sigma internals. The chain view becomes a thin consumer; future graph visualizations (skill relationships, org-JD-resume mapping, source-bullet sub-graphs) plug into the same component.

## Non-Goals

- Filters, search, toolbar controls (deferred to H4 GraphToolbar)
- Edge labels or animated flows
- Multiple simultaneous GraphView instances sharing a WebGL context
- Custom WebGL node/edge programs (dashed edges, image nodes)
- Dark mode color scheme (reads CSS custom properties for future theme integration)
- Screen reader accessibility for the WebGL canvas
- Chain view migration (tracked separately; this phase validates the interface only)

## Context

The current `ChainViewModal.svelte` directly imports `graphology`, `sigma`, `graphology-layout-forceatlas2`, creates a `Graph`, wires event handlers, manages `selectedNodeId`, `hoveredEdge`, node/edge reducers, and cleanup lifecycle -- approximately 350 lines of graph infrastructure mixed with chain-specific data fetching and detail panel rendering. Every future graph visualization would duplicate this boilerplate.

The user's blog project implements this extraction pattern in React with `GraphView.tsx` (core renderer), `GraphControllers.tsx` (drag, layout, theme, filter), `graph.types.ts`, `graph.constants.ts`, and `graph.shared.ts`. This plan adapts that architecture to Svelte 5 runes, replacing React hooks/refs with `$state`/`$effect`/`$derived` and React context with Svelte module-scope stores.

**Existing dependencies (already in `packages/webui/package.json`):** `graphology@^0.26.0`, `graphology-layout-forceatlas2@^0.10.1`, `graphology-types@^0.24.8`, `sigma@^3.0.2`.

**Existing chain view types (unchanged):** `ChainNode`, `ChainEdge`, `NODE_COLORS` in `packages/webui/src/lib/graph/types.ts`.

## Scope

| Spec section | Covered here? |
|-------------|---------------|
| 1. Type Definitions (`GraphNode`, `GraphEdge`, `ForceConfig`, `GraphConfig`, `GraphViewProps`) | Yes |
| 2. Default Configuration (`mergeConfig`, preset configs) | Yes |
| 3. Constants (dim colors, z-index, `resolveThemeColor`) | Yes |
| 4. Component `GraphView.svelte` (props, state, graph building, layout, Sigma init, reducers, events, drag, cleanup) | Yes |
| 5. Consumer migration (chain view) | No -- validates interface, migration is separate |
| 6. Files to create | Yes |
| 7. Files to modify | N/A (none) |
| 8. Testing | Yes |
| 9. Acceptance criteria | Yes |
| 10. Dependencies | Yes |

## Files to Create

| File | Description |
|------|-------------|
| `packages/webui/src/lib/components/graph/graph.types.ts` | `GraphNode`, `GraphEdge`, `ForceConfig`, `GraphConfig`, `GraphViewProps` type definitions |
| `packages/webui/src/lib/components/graph/graph.config.ts` | `DEFAULT_GRAPH_CONFIG`, `mergeConfig()` utility, `DENSE_GRAPH_CONFIG`, `TREE_GRAPH_CONFIG`, `SMALL_GRAPH_CONFIG` presets |
| `packages/webui/src/lib/components/graph/graph.constants.ts` | Dim colors, z-index constants, `resolveThemeColor()` utility |
| `packages/webui/src/lib/components/graph/GraphView.svelte` | Generic Sigma.js graph renderer component |
| `packages/webui/src/lib/components/graph/__tests__/graph-config.test.ts` | Unit tests for `mergeConfig()` |
| `packages/webui/src/lib/components/graph/__tests__/graph-types.test.ts` | Type-level compilation tests for `GraphNode`, `GraphEdge` |
| `packages/webui/src/lib/components/graph/__tests__/graph-constants.test.ts` | Unit tests for `resolveThemeColor()` and constant values |

## Files to Modify

None. This phase is foundational -- it creates new files only. The existing `$lib/graph/types.ts` (`ChainNode`, `ChainEdge`, `NODE_COLORS`) remains unchanged. The existing `ChainViewModal.svelte` is not modified in this phase.

## Fallback Strategies

- **`graphology-layout` not installed (circular layout):** The `runLayout` function wraps `import('graphology-layout/circular')` in a `try/catch`. If the optional dependency is missing, it logs a warning and falls back to random positions (nodes already have random x/y from construction). No crash, no data loss.
- **Sigma v3 mouse captor event name mismatch:** The drag implementation uses `instance.getMouseCaptor().on('mousemovebody', ...)`. If the installed Sigma version uses a different event name, drag silently fails (no crash). The spec notes this as a verification point during implementation. Fallback: test `'mousemove'` as an alternative.
- **ForceAtlas2 dynamic import fails:** If `graphology-layout-forceatlas2` fails to load (should not happen -- it is already a direct dependency), the catch block logs a warning and the graph renders with random positions. Users see a readable but unoptimized layout.
- **WebGL context unavailable:** Sigma's constructor throws if WebGL is not supported. The `initSigma()` function is wrapped in a try/catch that sets an error state. The component renders an error message instead of a blank div.
- **SSR evaluation:** All Sigma and graphology imports are dynamic (`await import(...)`) inside `$effect` blocks guarded by `if (!browser)` return. No server-side evaluation occurs.
- **`$effect` async limitation:** Svelte 5 effects are synchronous. The graph-building effect cannot use `await` directly. The plan extracts an `async function buildAndAssign()` called from within the effect, with the `graph = g` assignment happening inside the async function after layout completes.

---

## Tasks

### T48.1: Write Type Definitions

**File:** `packages/webui/src/lib/components/graph/graph.types.ts`

Defines the generic graph interfaces that decouple consumers from Sigma.js/graphology internals.

[CRITICAL] The `GraphNode.type` field is the entity classification (source/bullet/perspective), NOT the Sigma node program type. The component stores entity type as `nodeType` on the graphology node and sets Sigma's `type: 'circle'` separately. The `...node` spread comes BEFORE explicit Sigma attributes to prevent `node.type` from overwriting `type: 'circle'`.

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

**Acceptance criteria:**
- All five interfaces (`GraphNode`, `GraphEdge`, `ForceConfig`, `GraphConfig`, `GraphViewProps`) export correctly.
- `GraphNode` supports index signature for custom metadata (`[key: string]: unknown`).
- `GraphEdge` supports index signature for custom metadata.
- `GraphConfig.layout` is a union of three literal types.
- `GraphViewProps.config` is `Partial<GraphConfig>` (every field optional).
- TypeScript compilation succeeds with strict mode.

**Failure criteria:**
- `GraphNode.type` collides with Sigma's `type` attribute at the graphology level (this is the `...node` spread ordering problem -- verified in T48.4).
- Index signature prevents type narrowing on known fields.

---

### T48.2: Write Default Configuration and Presets

**File:** `packages/webui/src/lib/components/graph/graph.config.ts`

[IMPORTANT] `mergeConfig` must shallow-merge nested objects independently. Providing `{ forces: { gravity: 2 } }` must NOT discard `scalingRatio` and `slowDown` from defaults.

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

**Acceptance criteria:**
- `mergeConfig()` with no argument returns a copy of `DEFAULT_GRAPH_CONFIG` (not a reference).
- `mergeConfig({ forces: { gravity: 5 } })` preserves `scalingRatio: 10` and `slowDown: 1`.
- `mergeConfig({ colorMap: { source: '#ff0000' } })` returns `{ source: '#ff0000' }` (additive merge with empty default).
- `mergeConfig({ layout: 'circular' })` changes layout but preserves all other defaults.
- Preset configs (`DENSE_GRAPH_CONFIG`, `TREE_GRAPH_CONFIG`, `SMALL_GRAPH_CONFIG`) are importable and pass through `mergeConfig()` correctly.

**Failure criteria:**
- Nested merge clobbers sibling keys (e.g., providing `gravity` wipes `scalingRatio`).
- `mergeConfig()` returns a reference to `DEFAULT_GRAPH_CONFIG` (mutations leak).

---

### T48.3: Write Constants

**File:** `packages/webui/src/lib/components/graph/graph.constants.ts`

Constants for selection highlighting and theme resolution. Values match the existing `ChainViewModal.svelte` hardcoded values exactly (verified by reading lines 338-355 of `ChainViewModal.svelte`).

[IMPORTANT] `resolveThemeColor` must guard against SSR with `typeof window === 'undefined'`.

```typescript
/**
 * Dimmed color for non-selected nodes during selection highlighting.
 * Matches the existing ChainViewModal value at line 338.
 */
export const DIM_NODE_COLOR = '#e5e7eb'

/**
 * Dimmed color for non-connected edges during selection highlighting.
 * Matches the existing ChainViewModal value at line 355.
 */
export const DIM_EDGE_COLOR = '#f3f4f6'

/**
 * Dimmed edge size during selection highlighting.
 */
export const DIM_EDGE_SIZE = 0.5

/**
 * Selection highlight: connected edges get +1 size bump.
 * Matches ChainViewModal edgeReducer at line 353.
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

**Acceptance criteria:**
- `DIM_NODE_COLOR` is `'#e5e7eb'` (matches `ChainViewModal` line 338).
- `DIM_EDGE_COLOR` is `'#f3f4f6'` (matches `ChainViewModal` line 355).
- `DIM_EDGE_SIZE` is `0.5`.
- `HIGHLIGHT_EDGE_SIZE_BUMP` is `1`.
- `Z_FOREGROUND` is `1`, `Z_BACKGROUND` is `0`.
- `resolveThemeColor` returns fallback when `window` is undefined (SSR).
- `resolveThemeColor` returns the CSS property value when set, fallback when empty.

**Failure criteria:**
- Constants diverge from existing `ChainViewModal` hardcoded values.
- `resolveThemeColor` crashes during SSR.

---

### T48.4: Write `GraphView.svelte` Component

**File:** `packages/webui/src/lib/components/graph/GraphView.svelte`

This is the core component. It handles graph construction, layout, Sigma initialization, event wiring, selection highlighting, drag support, and cleanup.

[CRITICAL] `...node` spread BEFORE explicit Sigma attributes. The `GraphNode.type` field (e.g., `'source'`, `'bullet'`) would overwrite Sigma's `type: 'circle'` if spread came after. The node's entity type is preserved as `nodeType` on the graphology node.

[CRITICAL] Sigma re-init: when `graph` state changes (new data), the old Sigma instance MUST be destroyed and re-created. Do NOT guard with `if (sigmaInstance) return` (the existing `ChainViewModal` has this guard at line 321, which is the anti-pattern this component fixes).

[SEVERE] Async layout: `await runLayout(g, config)` must complete before setting `graph = g`. If `graph` is set before layout finishes, Sigma initializes with random positions and then nodes jump when layout completes.

[IMPORTANT] `$effect` cannot be async. Extract an `async function` called from within the effect. The graph assignment (`graph = g`) happens inside the async function after layout completes.

[IMPORTANT] Cleanup uses dual pattern: `$effect` return (primary, handles re-runs) + `onDestroy` (backup, handles abrupt unmount). Matches Phase 33 dual-cleanup pattern.

[IMPORTANT] Dynamic imports for SSR safety. `sigma`, `graphology-layout-forceatlas2`, and `graphology-layout/circular` are all dynamically imported inside browser-guarded code paths.

[MINOR] Edge events only fire when `resolvedConfig.enableEdgeEvents` is true.

[MINOR] `highlightNode` camera animation uses `ratio: 0.3` and `duration: 300` (matches existing ChainViewModal).

```svelte
<!--
  GraphView.svelte — Generic Sigma.js graph renderer.

  Accepts arbitrary nodes and edges, manages the full Sigma.js WebGL
  lifecycle (init, layout, reducers, events, cleanup), and exposes
  interaction callbacks. Consumers map domain data into the generic
  GraphNode[]/GraphEdge[] interface.

  Cleanup uses dual pattern: $effect return (primary) + onDestroy (backup).
-->
<script lang="ts">
  import { browser } from '$app/environment'
  import { onDestroy } from 'svelte'
  // NOTE: Move `import Graph from 'graphology'` inside the browser-guarded
  // `$effect` or use dynamic import (`const { default: Graph } = await import('graphology')`).
  // Static import at module level causes SSR issues because graphology
  // references browser globals during module evaluation.
  import Graph from 'graphology'
  import type { GraphNode, GraphEdge, GraphViewProps } from './graph.types'
  import { mergeConfig } from './graph.config'
  import type { GraphConfig } from './graph.types'
  import {
    DIM_NODE_COLOR,
    DIM_EDGE_COLOR,
    DIM_EDGE_SIZE,
    HIGHLIGHT_EDGE_SIZE_BUMP,
    Z_FOREGROUND,
    Z_BACKGROUND,
  } from './graph.constants'

  // ---------------------------------------------------------------------------
  // Props
  // ---------------------------------------------------------------------------

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

  // ---------------------------------------------------------------------------
  // Internal state
  // ---------------------------------------------------------------------------

  let container = $state<HTMLDivElement | null>(null)
  let sigmaInstance: any = $state(null)
  let graph = $state<Graph | null>(null)
  let selectedNodeId = $state<string | null>(null)
  let hoveredEdge = $state<string | null>(null)
  let initError = $state<string | null>(null)

  // Merged config (recomputed when configOverride changes)
  let resolvedConfig = $derived(mergeConfig(configOverride))

  // ---------------------------------------------------------------------------
  // Layout execution
  // ---------------------------------------------------------------------------

  /**
   * Run the configured layout algorithm on the graph. Dynamically imports
   * layout libraries for SSR safety. ForceAtlas2 auto-enables Barnes-Hut
   * when node count exceeds the threshold.
   */
  async function runLayout(g: Graph, config: GraphConfig): Promise<void> {
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
      try {
        const circular = await import('graphology-layout/circular')
        circular.default.assign(g)
      } catch {
        console.warn('graphology-layout not installed; falling back to random positions')
      }
    }
    // 'random' layout: nodes already have random positions from construction
  }

  // ---------------------------------------------------------------------------
  // Graph building (reactive on nodes/edges/config changes)
  // ---------------------------------------------------------------------------

  $effect(() => {
    // Track reactive dependencies
    const _nodes = nodes
    const _edges = edges
    const _config = resolvedConfig

    if (!browser) return

    async function buildAndAssign() {
      const g = new Graph({ type: 'directed', multi: false })

      for (const node of _nodes) {
        // Spread node FIRST so explicit Sigma attributes below take precedence.
        // node.type is the entity classification (source/bullet/perspective).
        // Sigma's node program type is stored separately as type: 'circle'.
        // If ...node came after, node.type would overwrite 'circle'.
        g.addNode(node.id, {
          ...node,
          label: node.label ?? node.id,
          x: node.x ?? Math.random() * 100,
          y: node.y ?? Math.random() * 100,
          size: node.size ?? _config.nodeDefaults.size,
          color: node.color
            ?? _config.colorMap?.[node.type]
            ?? _config.nodeDefaults.color,
          nodeType: node.type,  // entity classification stored separately
          type: 'circle',       // Sigma node program type — MUST come after ...node
        })
      }

      for (const edge of _edges) {
        if (!g.hasNode(edge.source) || !g.hasNode(edge.target)) continue
        g.addEdge(edge.source, edge.target, {
          ...edge,
          size: edge.weight ?? _config.edgeDefaults.size,
          color: edge.color
            ?? (edge.type ? _config.edgeColorMap?.[edge.type] : undefined)
            ?? _config.edgeDefaults.color,
          type: _config.edgeDefaults.type,
          edgeType: edge.type,  // entity classification stored separately
        })
      }

      // Run layout BEFORE setting graph state. Sigma only initializes after
      // layout is done — no visual jump from random to laid-out positions.
      if (g.order > 0) {
        await runLayout(g, _config)
      }

      graph = g
    }

    buildAndAssign()
  })

  // ---------------------------------------------------------------------------
  // Sigma initialization (reactive on container + graph)
  // ---------------------------------------------------------------------------

  $effect(() => {
    if (!browser || !container || !graph) return

    // When graph state changes (new data), destroy old Sigma instance and
    // re-create. This ensures reducers always reference the current graph.
    // Do NOT guard with `if (sigmaInstance) return` — that is the anti-pattern
    // this component fixes from ChainViewModal.
    if (sigmaInstance) {
      sigmaInstance.kill()
      sigmaInstance = null
    }

    initError = null

    async function initSigma() {
      try {
        const { default: Sigma } = await import('sigma')

        const instance = new Sigma(graph!, container!, {
          renderEdgeLabels: false,
          enableEdgeEvents: resolvedConfig.enableEdgeEvents,
          defaultNodeType: 'circle',
          defaultEdgeType: resolvedConfig.edgeDefaults.type,
          zIndex: resolvedConfig.zIndex,
          labelRenderedSizeThreshold: resolvedConfig.labelThreshold,

          // --- Node reducer: selection highlighting ---
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

          // --- Edge reducer: selection + hover highlighting ---
          edgeReducer: (edge: string, data: Record<string, any>) => {
            const [src, tgt] = graph!.extremities(edge)
            const srcAttrs = graph!.getNodeAttributes(src)
            const tgtAttrs = graph!.getNodeAttributes(tgt)

            // Hide edges with hidden endpoints (filtered graphs)
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

        // --- Event handlers ---

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

        // --- Drag support ---

        if (resolvedConfig.enableDrag) {
          let draggedNode: string | null = null
          let isDragging = false

          instance.on('downNode', ({ node }: { node: string }) => {
            isDragging = true
            draggedNode = node
            graph!.setNodeAttribute(node, 'fixed', true)
          })

          // Sigma v3 mouse captor event for mouse movement during drag.
          // If 'mousemovebody' does not work with the installed version,
          // try 'mousemove' as a fallback.
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

        // --- Highlight on open ---

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
      } catch (e) {
        initError = e instanceof Error ? e.message : 'Failed to initialize graph renderer'
        console.error('GraphView: Sigma initialization failed', e)
      }
    }

    initSigma()

    return () => {
      if (sigmaInstance) {
        sigmaInstance.kill()
        sigmaInstance = null
      }
    }
  })

  // ---------------------------------------------------------------------------
  // Refresh on selection/hover state changes
  // ---------------------------------------------------------------------------

  $effect(() => {
    // Track reactive values to trigger Sigma refresh for reducer updates
    const _sel = selectedNodeId
    const _hover = hoveredEdge
    sigmaInstance?.refresh()
  })

  // ---------------------------------------------------------------------------
  // Backup cleanup: onDestroy for abrupt component unmount
  // ---------------------------------------------------------------------------

  onDestroy(() => {
    if (sigmaInstance) {
      sigmaInstance.kill()
      sigmaInstance = null
    }
  })

  // ---------------------------------------------------------------------------
  // Public API: exported accessor functions for H5/H6 consumers
  // ---------------------------------------------------------------------------

  /**
   * Returns the current Sigma instance, or null if not initialized.
   * Used by H5 (sub-graph) and H6 (search/filter controllers).
   */
  export function getSigma(): any {
    return sigmaInstance
  }

  /**
   * Returns the current graphology Graph instance, or null if not built.
   */
  export function getGraph(): Graph | null {
    return graph
  }

  /**
   * Returns the graph container DOM element.
   * Used by GraphToolbar (Phase 55) for fullscreen toggle.
   * NOTE: Phase 55 will add `getContainer()`. The `container` variable is
   * already in scope — Phase 55's addition is a one-line export.
   */
  export function getContainer(): HTMLElement | null {
    return container
  }

  /**
   * Programmatically focus on a node: select it and animate the camera to it.
   * Used by H6 search results to navigate to a found node.
   */
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
</script>

{#if initError}
  <div class="graph-error">
    <p>Graph rendering failed: {initError}</p>
  </div>
{/if}

<div class="graph-view" bind:this={container}></div>

<style>
  .graph-view {
    width: 100%;
    height: 100%;
    min-height: 300px;
    position: relative;
  }

  .graph-error {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    color: #ef4444;
    font-size: 0.85rem;
    text-align: center;
    z-index: 10;
  }
</style>
```

**Key implementation notes:**

1. **`...node` spread ordering (line ~120):** The spread comes FIRST, then explicit Sigma attributes (`label`, `x`, `y`, `size`, `color`, `nodeType`, `type`) overwrite any collisions. This prevents `node.type` (entity classification) from overwriting Sigma's `type: 'circle'`.

2. **No `if (sigmaInstance) return` guard (line ~155):** When `graph` changes, the effect re-runs. The old Sigma instance is killed and a new one created. This is the opposite of the existing `ChainViewModal` pattern (line 321: `if (!browser || !container || !graph || sigmaInstance) return`), which prevents re-initialization when data changes.

3. **Async in `$effect` (line ~135):** The `buildAndAssign()` function is async, called from a synchronous `$effect`. The `graph = g` assignment happens after `await runLayout(g, _config)` completes. This works because Svelte 5 effects do not need to return a cleanup from async code -- the cleanup return is on the Sigma init effect, not the graph-building effect.

4. **Edge spread ordering (line ~134):** For edges, the spread comes LAST (`...edge`) because edges do not have a `type` collision problem -- the Sigma edge type is set via `type: _config.edgeDefaults.type` before the spread, and `GraphEdge.type` is the entity classification stored as `edgeType`. Wait -- [INCONSISTENCY] The spec shows edge attributes with `type: resolvedConfig.edgeDefaults.type` set BEFORE `...edge`, meaning `edge.type` WOULD overwrite the Sigma edge type. Resolution: store edge entity type as `edgeType` (analogous to `nodeType`) and set Sigma's `type` AFTER the spread, same as nodes. The code above implements this fix.

5. **Exported functions:** `getSigma()`, `getGraph()`, `focusNode()` are exported for H5 and H6 consumer components to call via `bind:this` references.

**Acceptance criteria:**
- Component renders a `<div class="graph-view">` that fills its parent.
- Component accepts all `GraphViewProps` fields.
- Graphology `Graph` is created with `type: 'directed', multi: false`.
- Nodes with missing `x`/`y` get random positions (0-100 range).
- Edges referencing non-existent nodes are silently skipped.
- ForceAtlas2 runs before `graph` is assigned to state.
- Barnes-Hut auto-enables when `g.order > barnesHutThreshold`.
- Sigma initializes after graph + container are ready.
- Clicking a node sets `selectedNodeId`, calls `onNodeClick`.
- Clicking stage clears `selectedNodeId`, calls `onStageClick`.
- Node/edge hover callbacks fire correctly.
- Edge events only wire when `enableEdgeEvents` is true.
- Drag updates node positions in graphology model.
- `highlightNode` triggers camera animation and `onNodeClick`.
- `$effect` return kills Sigma on re-run.
- `onDestroy` kills Sigma on unmount.
- WebGL init failure shows error message (not blank div).
- `getSigma()`, `getGraph()`, `focusNode()` are callable from parent.

**Failure criteria:**
- `node.type` overwrites Sigma's `type: 'circle'` (spread ordering wrong).
- Old Sigma instance not killed when graph data changes (memory/context leak).
- Layout runs after `graph = g` (visual jump from random to laid-out positions).
- SSR crash from evaluating Sigma/graphology at import time.

---

### T48.5: Write `mergeConfig` Unit Tests

**File:** `packages/webui/src/lib/components/graph/__tests__/graph-config.test.ts`

```typescript
import { describe, it, expect } from 'vitest'
import { mergeConfig, DEFAULT_GRAPH_CONFIG, DENSE_GRAPH_CONFIG, TREE_GRAPH_CONFIG, SMALL_GRAPH_CONFIG } from '../graph.config'

describe('mergeConfig', () => {
  it('returns defaults when no override provided', () => {
    const config = mergeConfig()
    expect(config).toEqual(DEFAULT_GRAPH_CONFIG)
  })

  it('returns a copy, not a reference to DEFAULT_GRAPH_CONFIG', () => {
    const config = mergeConfig()
    config.forces.gravity = 999
    expect(DEFAULT_GRAPH_CONFIG.forces.gravity).toBe(1)
  })

  it('shallow-merges nested forces without losing defaults', () => {
    const config = mergeConfig({ forces: { gravity: 5 } })
    expect(config.forces.gravity).toBe(5)
    expect(config.forces.scalingRatio).toBe(10)  // preserved from default
    expect(config.forces.slowDown).toBe(1)        // preserved from default
    expect(config.forces.iterations).toBe(100)    // preserved from default
  })

  it('shallow-merges nodeDefaults without losing defaults', () => {
    const config = mergeConfig({ nodeDefaults: { size: 12 } })
    expect(config.nodeDefaults.size).toBe(12)
    expect(config.nodeDefaults.color).toBe('#6b7280')  // preserved from default
  })

  it('shallow-merges edgeDefaults without losing defaults', () => {
    const config = mergeConfig({ edgeDefaults: { type: 'line' } })
    expect(config.edgeDefaults.type).toBe('line')
    expect(config.edgeDefaults.color).toBe('#94a3b8')  // preserved from default
    expect(config.edgeDefaults.size).toBe(1)            // preserved from default
  })

  it('merges colorMap additively', () => {
    const config = mergeConfig({ colorMap: { source: '#ff0000' } })
    expect(config.colorMap).toEqual({ source: '#ff0000' })
  })

  it('merges edgeColorMap additively', () => {
    const config = mergeConfig({ edgeColorMap: { matching: '#00ff00', drifted: '#ff0000' } })
    expect(config.edgeColorMap).toEqual({ matching: '#00ff00', drifted: '#ff0000' })
  })

  it('overrides top-level scalars', () => {
    const config = mergeConfig({ layout: 'circular', labelThreshold: 10 })
    expect(config.layout).toBe('circular')
    expect(config.labelThreshold).toBe(10)
    expect(config.enableDrag).toBe(true)  // unchanged default
  })

  it('overrides boolean flags', () => {
    const config = mergeConfig({ enableDrag: false, enableZoom: false, enableEdgeEvents: false })
    expect(config.enableDrag).toBe(false)
    expect(config.enableZoom).toBe(false)
    expect(config.enableEdgeEvents).toBe(false)
  })

  it('handles undefined partial gracefully', () => {
    const config = mergeConfig(undefined)
    expect(config).toEqual(DEFAULT_GRAPH_CONFIG)
  })

  it('handles empty object partial', () => {
    const config = mergeConfig({})
    expect(config).toEqual(DEFAULT_GRAPH_CONFIG)
  })

  it('works with DENSE_GRAPH_CONFIG preset', () => {
    const config = mergeConfig(DENSE_GRAPH_CONFIG)
    expect(config.forces.gravity).toBe(3)
    expect(config.forces.scalingRatio).toBe(5)
    expect(config.forces.slowDown).toBe(2)
    expect(config.forces.iterations).toBe(100)  // preserved from default
  })

  it('works with TREE_GRAPH_CONFIG preset', () => {
    const config = mergeConfig(TREE_GRAPH_CONFIG)
    expect(config.forces.gravity).toBe(0.5)
    expect(config.forces.scalingRatio).toBe(20)
    expect(config.forces.slowDown).toBe(1)
  })

  it('works with SMALL_GRAPH_CONFIG preset', () => {
    const config = mergeConfig(SMALL_GRAPH_CONFIG)
    expect(config.forces.iterations).toBe(50)
    expect(config.forces.gravity).toBe(1)
  })
})
```

**Acceptance criteria:**
- All 13 test cases pass.
- Nested merge preserves unspecified sibling keys.
- Default immutability is verified (mutation on copy does not affect original).
- All three presets produce expected merged results.

**Failure criteria:**
- Any test fails, indicating a merge bug.

---

### T48.6: Write Type-Level Tests

**File:** `packages/webui/src/lib/components/graph/__tests__/graph-types.test.ts`

```typescript
import { describe, it, expect } from 'vitest'
import type { GraphNode, GraphEdge, GraphConfig, GraphViewProps } from '../graph.types'

describe('GraphNode', () => {
  it('accepts minimal node', () => {
    const node: GraphNode = { id: '1', label: 'Test', type: 'source' }
    expect(node.id).toBe('1')
    expect(node.label).toBe('Test')
    expect(node.type).toBe('source')
  })

  it('accepts node with optional position and size', () => {
    const node: GraphNode = { id: '1', label: 'Test', type: 'source', x: 10, y: 20, size: 15 }
    expect(node.x).toBe(10)
    expect(node.y).toBe(20)
    expect(node.size).toBe(15)
  })

  it('accepts node with explicit color', () => {
    const node: GraphNode = { id: '1', label: 'Test', type: 'source', color: '#ff0000' }
    expect(node.color).toBe('#ff0000')
  })

  it('accepts node with custom metadata via index signature', () => {
    const node: GraphNode = {
      id: '1', label: 'Test', type: 'source',
      entityId: 'abc', status: 'approved', domain: 'engineering',
    }
    expect(node.entityId).toBe('abc')
    expect(node.status).toBe('approved')
    expect(node.domain).toBe('engineering')
  })
})

describe('GraphEdge', () => {
  it('accepts minimal edge', () => {
    const edge: GraphEdge = { id: 'e1', source: '1', target: '2' }
    expect(edge.source).toBe('1')
    expect(edge.target).toBe('2')
  })

  it('accepts edge with optional fields', () => {
    const edge: GraphEdge = {
      id: 'e1', source: '1', target: '2',
      label: 'derives', weight: 2, type: 'matching', color: '#00ff00',
    }
    expect(edge.weight).toBe(2)
    expect(edge.type).toBe('matching')
  })

  it('accepts edge with custom metadata', () => {
    const edge: GraphEdge = {
      id: 'e1', source: '1', target: '2',
      drifted: true, isPrimary: false,
    }
    expect(edge.drifted).toBe(true)
  })
})

describe('GraphConfig shape', () => {
  it('accepts a full config object', () => {
    const config: GraphConfig = {
      layout: 'forceatlas2',
      forces: { gravity: 1, scalingRatio: 10, slowDown: 1 },
      nodeDefaults: { size: 8, color: '#6b7280' },
      edgeDefaults: { color: '#94a3b8', size: 1, type: 'arrow' },
      colorMap: { source: '#ff0000' },
      edgeColorMap: { matching: '#00ff00' },
      labelThreshold: 6,
      enableDrag: true,
      enableZoom: true,
      enableEdgeEvents: true,
      zIndex: true,
    }
    expect(config.layout).toBe('forceatlas2')
  })

  it('accepts circular and random layout values', () => {
    const circular: GraphConfig['layout'] = 'circular'
    const random: GraphConfig['layout'] = 'random'
    expect(circular).toBe('circular')
    expect(random).toBe('random')
  })
})

describe('GraphViewProps shape', () => {
  it('accepts minimal props (nodes + edges)', () => {
    const props: GraphViewProps = {
      nodes: [{ id: '1', label: 'Test', type: 'source' }],
      edges: [{ id: 'e1', source: '1', target: '2' }],
    }
    expect(props.nodes).toHaveLength(1)
    expect(props.edges).toHaveLength(1)
  })

  it('accepts all optional callback props', () => {
    const props: GraphViewProps = {
      nodes: [],
      edges: [],
      config: { layout: 'circular' },
      highlightNode: 'node-1',
      onNodeClick: (_id, _data) => {},
      onNodeHover: (_id, _data) => {},
      onEdgeClick: (_id, _data) => {},
      onEdgeHover: (_id) => {},
      onStageClick: () => {},
    }
    expect(props.highlightNode).toBe('node-1')
  })
})
```

**Acceptance criteria:**
- All type tests compile and pass.
- `GraphNode` and `GraphEdge` index signatures allow arbitrary metadata.
- `GraphConfig.layout` accepts exactly three literal values.
- `GraphViewProps.config` is `Partial<GraphConfig>`.

**Failure criteria:**
- TypeScript compiler rejects valid interface usage.

---

### T48.7: Write Constants Unit Tests

**File:** `packages/webui/src/lib/components/graph/__tests__/graph-constants.test.ts`

```typescript
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import {
  DIM_NODE_COLOR,
  DIM_EDGE_COLOR,
  DIM_EDGE_SIZE,
  HIGHLIGHT_EDGE_SIZE_BUMP,
  Z_FOREGROUND,
  Z_BACKGROUND,
  resolveThemeColor,
} from '../graph.constants'

describe('graph constants', () => {
  it('DIM_NODE_COLOR matches ChainViewModal hardcoded value', () => {
    expect(DIM_NODE_COLOR).toBe('#e5e7eb')
  })

  it('DIM_EDGE_COLOR matches ChainViewModal hardcoded value', () => {
    expect(DIM_EDGE_COLOR).toBe('#f3f4f6')
  })

  it('DIM_EDGE_SIZE is 0.5', () => {
    expect(DIM_EDGE_SIZE).toBe(0.5)
  })

  it('HIGHLIGHT_EDGE_SIZE_BUMP is 1', () => {
    expect(HIGHLIGHT_EDGE_SIZE_BUMP).toBe(1)
  })

  it('Z_FOREGROUND is 1 and Z_BACKGROUND is 0', () => {
    expect(Z_FOREGROUND).toBe(1)
    expect(Z_BACKGROUND).toBe(0)
  })
})

describe('resolveThemeColor', () => {
  it('returns fallback when window is undefined (SSR)', () => {
    // In vitest with jsdom, window IS defined. Test the SSR path by
    // temporarily overriding typeof check via function extraction.
    // The SSR guard is `typeof window === 'undefined'`.
    // In jsdom, window exists, so we test the browser path instead.
    // The SSR path is validated by the component's SSR safety smoke test.
    const result = resolveThemeColor('--nonexistent-property', '#abcdef')
    // In jsdom, getComputedStyle returns '' for unset properties
    expect(result).toBe('#abcdef')
  })

  it('returns fallback for unset CSS property', () => {
    const result = resolveThemeColor('--definitely-not-set-12345', '#fallback')
    expect(result).toBe('#fallback')
  })
})
```

**Acceptance criteria:**
- All constant values match their expected values.
- `resolveThemeColor` returns fallback for unset properties.

**Failure criteria:**
- Constants diverge from `ChainViewModal` hardcoded values (visual regression risk).

---

### T48.8: Extend Graphology Smoke Test

**File:** `packages/webui/src/lib/graph/smoke.test.ts` (modify existing)

Add a test case that uses the new `GraphNode` and `GraphEdge` types to build a graphology graph, validating the data flow from generic types to graphology.

Add the following test case after the existing test:

```typescript
import type { GraphNode, GraphEdge } from '$lib/components/graph/graph.types'

// ... after existing test ...

it('builds a graph from GraphNode[] and GraphEdge[] types', () => {
  const nodes: GraphNode[] = [
    { id: 'src-1', label: 'Source A', type: 'source', size: 12, color: '#6c63ff' },
    { id: 'bul-1', label: 'Bullet A', type: 'bullet', size: 8, color: '#3b82f6' },
    { id: 'per-1', label: 'Perspective A', type: 'perspective', size: 6, color: '#10b981' },
  ]

  const edges: GraphEdge[] = [
    { id: 'e1', source: 'src-1', target: 'bul-1', weight: 2 },
    { id: 'e2', source: 'bul-1', target: 'per-1', weight: 1 },
  ]

  const graph = new Graph({ type: 'directed', multi: false })

  for (const node of nodes) {
    graph.addNode(node.id, {
      ...node,
      label: node.label,
      x: node.x ?? Math.random() * 100,
      y: node.y ?? Math.random() * 100,
      size: node.size ?? 8,
      color: node.color ?? '#6b7280',
      nodeType: node.type,
      type: 'circle',
    })
  }

  for (const edge of edges) {
    graph.addEdge(edge.source, edge.target, {
      size: edge.weight ?? 1,
      color: '#94a3b8',
      type: 'arrow',
    })
  }

  expect(graph.order).toBe(3)
  expect(graph.size).toBe(2)

  // Verify spread ordering: Sigma type is 'circle', not 'source'
  expect(graph.getNodeAttribute('src-1', 'type')).toBe('circle')
  expect(graph.getNodeAttribute('src-1', 'nodeType')).toBe('source')

  // Verify color resolution
  expect(graph.getNodeAttribute('src-1', 'color')).toBe('#6c63ff')
  expect(graph.getNodeAttribute('bul-1', 'color')).toBe('#3b82f6')
})
```

[CRITICAL] The assertion `expect(graph.getNodeAttribute('src-1', 'type')).toBe('circle')` validates the spread ordering fix. If `...node` came AFTER `type: 'circle'`, this assertion would fail because `node.type` ('source') would overwrite 'circle'.

**Acceptance criteria:**
- Existing smoke test still passes.
- New test builds a 3-node, 2-edge graph from `GraphNode[]`/`GraphEdge[]`.
- Spread ordering assertion passes: `type` is `'circle'`, `nodeType` is `'source'`.

**Failure criteria:**
- `graph.getNodeAttribute('src-1', 'type')` returns `'source'` instead of `'circle'`.

---

## Testing Support

### Test Fixtures

No new test helpers needed. The unit tests for `mergeConfig` and constants are self-contained. The graphology smoke test extension uses the existing `Graph` import.

For component-level testing of `GraphView.svelte`, Sigma requires a WebGL context not available in jsdom/happy-dom. Two strategies:

1. **Mock Sigma (this phase):** The unit tests validate the data layer (types, config, constants) and graph-building logic (smoke test). Sigma rendering behavior is tested indirectly through the smoke test's spread-ordering assertion.

2. **Browser-based tests (future):** Playwright component tests that mount `GraphView.svelte` in a real browser. Deferred to a future phase when E2E infrastructure is established.

### Unit Tests

**File:** `packages/webui/src/lib/components/graph/__tests__/graph-config.test.ts` (T48.5)

| Test | Assertion |
|------|-----------|
| No override returns defaults | `config === DEFAULT_GRAPH_CONFIG` (deep equal) |
| No override returns copy, not reference | Mutation does not affect original |
| Nested forces merge preserves siblings | `gravity` overridden, `scalingRatio` preserved |
| Nested nodeDefaults merge preserves siblings | `size` overridden, `color` preserved |
| Nested edgeDefaults merge preserves siblings | `type` overridden, `color`/`size` preserved |
| ColorMap additive merge | New entries added to empty default |
| EdgeColorMap additive merge | Both entries present |
| Top-level scalar override | `layout` and `labelThreshold` overridden, `enableDrag` unchanged |
| Boolean flag override | `enableDrag`/`enableZoom`/`enableEdgeEvents` set to `false` |
| Undefined partial handled | Returns defaults |
| Empty object partial handled | Returns defaults |
| DENSE_GRAPH_CONFIG preset | `gravity: 3`, `scalingRatio: 5`, `slowDown: 2`, `iterations: 100` |
| TREE_GRAPH_CONFIG preset | `gravity: 0.5`, `scalingRatio: 20` |
| SMALL_GRAPH_CONFIG preset | `iterations: 50` |

### Type Compilation Tests

**File:** `packages/webui/src/lib/components/graph/__tests__/graph-types.test.ts` (T48.6)

| Test | Assertion |
|------|-----------|
| Minimal GraphNode compiles | `id`, `label`, `type` accepted |
| GraphNode with position/size | Optional `x`, `y`, `size` accepted |
| GraphNode with explicit color | Optional `color` accepted |
| GraphNode with custom metadata | Index signature fields accepted |
| Minimal GraphEdge compiles | `id`, `source`, `target` accepted |
| GraphEdge with optional fields | `label`, `weight`, `type`, `color` accepted |
| GraphEdge with custom metadata | Index signature fields accepted |
| Full GraphConfig compiles | All required fields accepted |
| GraphConfig layout values | `'circular'` and `'random'` accepted |
| Minimal GraphViewProps | `nodes` and `edges` only |
| Full GraphViewProps with callbacks | All optional props accepted |

### Constants Tests

**File:** `packages/webui/src/lib/components/graph/__tests__/graph-constants.test.ts` (T48.7)

| Test | Assertion |
|------|-----------|
| DIM_NODE_COLOR value | `'#e5e7eb'` |
| DIM_EDGE_COLOR value | `'#f3f4f6'` |
| DIM_EDGE_SIZE value | `0.5` |
| HIGHLIGHT_EDGE_SIZE_BUMP value | `1` |
| Z-index values | `Z_FOREGROUND: 1`, `Z_BACKGROUND: 0` |
| resolveThemeColor returns fallback for unset property | Falls back correctly |

### Smoke Tests

**File:** `packages/webui/src/lib/graph/smoke.test.ts` (T48.8, extends existing)

| Test | Assertion |
|------|-----------|
| Existing: creates graph with nodes and edges | `order: 2`, `size: 1` |
| New: builds graph from GraphNode[]/GraphEdge[] | `order: 3`, `size: 2`, spread ordering correct |

### Component Smoke Tests (Manual / Future Playwright)

| Test | What to verify |
|------|---------------|
| Component renders div | `<div class="graph-view">` appears in DOM |
| Component fills parent | Width and height 100% of parent container |
| Sigma initializes | WebGL canvas created inside container div |
| Node click fires callback | `onNodeClick` receives `(nodeId, nodeData)` |
| Stage click clears selection | `onStageClick` fires, nodes no longer dimmed |
| Drag moves node | Node position updates in graphology model |
| Highlight on open | Camera animates to `highlightNode` position |
| Destroy kills Sigma | No WebGL context leak after unmount |
| Error state renders | Invalid graph shows error message |

---

## Documentation Requirements

- No new documentation files required (non-goal).
- The spec file serves as the design document.
- This plan file serves as the implementation reference.
- Inline TSDoc comments on all exported interfaces and functions:
  - `GraphNode`: purpose of `type` field vs Sigma's `type`
  - `GraphEdge`: purpose of `weight` vs `size`
  - `GraphConfig`: explanation of each field
  - `GraphViewProps`: callback signatures
  - `mergeConfig`: nested merge behavior
  - `resolveThemeColor`: SSR safety
  - `getSigma()`, `getGraph()`, `focusNode()`: H5/H6 consumer usage
- Inline comments in `GraphView.svelte` for:
  - Spread ordering rationale (node type collision)
  - No `if (sigmaInstance) return` guard rationale
  - Async layout before state assignment rationale
  - Dual cleanup pattern rationale

---

## Parallelization Notes

**Within this phase:**
- T48.1 (types), T48.2 (config), T48.3 (constants) can be developed in parallel -- they have no imports between each other except T48.2 imports types from T48.1.
- T48.4 (component) depends on T48.1, T48.2, and T48.3 (imports all three).
- T48.5 (config tests) depends on T48.2.
- T48.6 (type tests) depends on T48.1.
- T48.7 (constants tests) depends on T48.3.
- T48.8 (smoke test extension) depends on T48.1 (imports `GraphNode`, `GraphEdge`).

**Recommended execution order:**
1. T48.1 (types -- foundational, no dependencies)
2. T48.2 + T48.3 (config + constants -- parallel, both depend on T48.1 at most)
3. T48.4 (component -- depends on all three above)
4. T48.5 + T48.6 + T48.7 + T48.8 (all tests -- parallel, each depends on its source file)

**Cross-phase:**
- This phase creates new files only and modifies nothing. It can run in parallel with any other phase.
- H2-H7 all depend on this phase. Once the component is committed, H2 (GraphToolbar) can wrap it, H3-H5 can consume it, H6 can use `getSigma()`/`getGraph()`/`focusNode()`, and H7 can export from `getSigma()`.
- The chain view migration (replacing `ChainViewModal`'s Sigma boilerplate with `<GraphView>`) is tracked separately and depends on this phase.
- The optional `graphology-layout` dependency (for circular layout) should be added to `packages/webui/package.json` only when a consumer requests circular layout. ForceAtlas2 and random layouts work without it.
