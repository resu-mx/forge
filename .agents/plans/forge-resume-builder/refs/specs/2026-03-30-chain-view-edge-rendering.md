# Chain View: Edge Rendering & Interaction

**Date:** 2026-03-30
**Status:** Design
**Builds on:** Chain View (Phase 15), Sigma.js graph visualization

## Purpose

Improve edge visibility and interaction in the Chain View graph. Currently edges are created in the graphology data model with arrow types, drift coloring (gray vs red), and primary/non-primary sizing, but Sigma.js needs explicit edge renderer programs to display arrows, labels, and interactive hover states. This spec defines how edges should render, what information they convey, and how users interact with them.

## Current State

The graph already creates edges with these attributes (in `chain/+page.svelte`):

```typescript
// Source -> Bullet edge
g.addEdge(sourceNodeKey, `bullet-${bullet.id}`, {
  size: src.is_primary ? 2 : 1,
  color: snapshotMatch ? '#94a3b8' : '#ef4444',  // gray vs red
  type: 'arrow',
  drifted: !snapshotMatch,
  isPrimary: src.is_primary,
})

// Bullet -> Perspective edge
g.addEdge(bulletNodeKey, `perspective-${perspective.id}`, {
  size: 1,
  color: snapshotMatch ? '#94a3b8' : '#ef4444',
  type: 'arrow',
  drifted: !snapshotMatch,
})
```

Sigma.js is configured with `defaultEdgeType: 'arrow'` and `enableEdgeEvents: true` (lines 201, 200). Sigma.js v3 registers `EdgeArrowProgram` by default for the `arrow` edge type. The current code already sets `defaultEdgeType: 'arrow'`, so arrows render without additional configuration. Adding `edgeProgramClasses: { arrow: EdgeArrowProgram }` explicitly is optional but makes the dependency visible.

## Goals

1. Render edges as visible arrows showing derivation direction (source -> bullet -> perspective)
2. Visual distinction between primary and non-primary source associations
3. Drift indication on edges (solid gray = snapshot matches, red = drifted)
4. Edge hover: show tooltip with relationship info (primary? drifted? snapshot status)
5. Edge click: select both connected nodes in the detail panel
6. Node detail panels show related entities (bullet shows perspectives, source shows bullets, perspective shows parent bullet)

## Non-Goals

- Edge labels (too cluttered at scale)
- Animated edge flows
- Edge editing (adding/removing associations from the graph view)
- Resume entry nodes in the graph (future enhancement)

---

## 1. Edge Rendering Programs

### 1.1 Install Sigma Edge Programs

Sigma.js v3 bundles `EdgeArrowProgram` in the `sigma/rendering` module. No additional package needed.

```typescript
import { EdgeArrowProgram } from 'sigma/rendering'
```

> **Note:** Sigma.js v3 registers `EdgeArrowProgram` by default for the `arrow` edge type. The current code already sets `defaultEdgeType: 'arrow'`, so arrows render without additional configuration. Adding `edgeProgramClasses: { arrow: EdgeArrowProgram }` explicitly is optional but makes the dependency visible.

### 1.2 Register Edge Programs in Sigma Constructor

Update the Sigma initialization in `chain/+page.svelte`:

```typescript
const instance = new Sigma(graph!, container!, {
  renderEdgeLabels: false,
  enableEdgeEvents: true,
  defaultNodeType: 'circle',
  defaultEdgeType: 'arrow',
  zIndex: true,  // Required for nodeReducer/edgeReducer zIndex values to take effect
  edgeProgramClasses: {
    arrow: EdgeArrowProgram,
  },
  labelRenderedSizeThreshold: 6,
  // nodeReducer and edgeReducer defined below in section 4.2
})
```

> **Important:** `zIndex: true` is required for `nodeReducer`/`edgeReducer` zIndex values to take effect. Without this, Sigma skips z-ordering entirely and the selection dimming effect will not layer correctly.

### 1.3 Edge Visual Attributes

Edges already have the correct attributes in the graphology model. Sigma reads:
- `size` -- line thickness (1 for non-primary/perspective edges, 2 for primary source edges)
- `color` -- `#94a3b8` (slate gray) for matching snapshots, `#ef4444` (red) for drifted
- `type` -- `'arrow'` (renders with arrowhead at target node)

**Additional visual encoding:**

| Edge Type | Color | Size | Arrow | Dash |
|-----------|-------|------|-------|------|
| Primary source -> bullet (matching) | `#94a3b8` (gray) | 2 | Yes | Solid |
| Secondary source -> bullet (matching) | `#94a3b8` (gray) | 1 | Yes | Solid |
| Primary source -> bullet (drifted) | `#ef4444` (red) | 2 | Yes | Solid |
| Bullet -> perspective (matching) | `#94a3b8` (gray) | 1 | Yes | Solid |
| Bullet -> perspective (drifted) | `#ef4444` (red) | 1 | Yes | Solid |

Note: Sigma.js v3's `EdgeArrowProgram` does not support dashed lines natively. Dashed edges would require a custom WebGL program. For MVP, use color alone (red = drifted) to indicate drift. Dashed lines are a future enhancement.

### 1.4 Edge Visibility on Filtered Graphs

When nodes are hidden by filters (source type, status, search), edges connected to hidden nodes must also be hidden. The `edgeReducer` must check endpoint visibility:

```typescript
edgeReducer: (edge, data) => {
  const [src, tgt] = graph!.extremities(edge)
  const srcAttrs = graph!.getNodeAttributes(src)
  const tgtAttrs = graph!.getNodeAttributes(tgt)

  // Hide edges with hidden endpoints
  if (srcAttrs.hidden || tgtAttrs.hidden) {
    return { ...data, hidden: true }
  }

  // Node selection dimming (existing logic)
  if (selectedNodeId) {
    if (src === selectedNodeId || tgt === selectedNodeId) {
      return { ...data, size: (data.size ?? 1) + 1, zIndex: 1 }
    }
    return { ...data, color: '#f3f4f6', size: 0.5, zIndex: 0 }
  }

  // Edge hover highlight
  if (hoveredEdge === edge) {
    return { ...data, size: (data.size ?? 1) + 1 }
  }

  return data
}
```

This single reducer handles edge hiding (filtered graphs), selection dimming, and hover highlighting. The filter-hiding check must come first so hidden-endpoint edges are never rendered regardless of selection state.

---

## 2. Edge Hover Interaction

### 2.1 Edge Hover Tooltip

When the user hovers over an edge, show a lightweight tooltip with:

```
Source -> Bullet (primary)
Snapshot: matches
```

or for drifted edges:

```
Bullet -> Perspective
Snapshot: drifted -- content has changed since derivation
```

When hovering an edge, the edge should be visually thickened (size + 1) in addition to showing the tooltip. Use `hoveredEdge` state and extend the `edgeReducer` (see section 1.4).

### 2.2 State Declarations

```typescript
interface SelectedEdgeData {
  sourceLabel: string
  sourceType: string
  sourceId: string
  sourceStatus: string
  targetLabel: string
  targetType: string
  targetId: string
  targetStatus: string
  isPrimary: boolean
  drifted: boolean
}

let selectedEdgeData = $state<SelectedEdgeData | null>(null)
let edgeTooltip = $state<{
  sourceLabel: string
  sourceType: string
  targetLabel: string
  targetType: string
  isPrimary: boolean
  drifted: boolean
} | null>(null)
let edgeTooltipPosition = $state({ x: 0, y: 0 })
let hoveredEdge = $state<string | null>(null)
```

### 2.3 Implementation

Sigma.js v3 supports edge events via `enterEdge` and `leaveEdge`:

```typescript
instance.on('enterEdge', ({ edge }: { edge: string }) => {
  const attrs = graph!.getEdgeAttributes(edge)
  const [sourceNode, targetNode] = graph!.extremities(edge)
  const sourceAttrs = graph!.getNodeAttributes(sourceNode)
  const targetAttrs = graph!.getNodeAttributes(targetNode)

  hoveredEdge = edge

  edgeTooltip = {
    sourceLabel: sourceAttrs.label,
    sourceType: sourceAttrs.entityType,
    targetLabel: targetAttrs.label,
    targetType: targetAttrs.entityType,
    isPrimary: attrs.isPrimary ?? false,
    drifted: attrs.drifted ?? false,
  }

  // Position tooltip near the edge midpoint using page-absolute coordinates
  const canvasRect = container!.getBoundingClientRect()
  const sourceCoords = instance.graphToViewport({
    x: sourceAttrs.x, y: sourceAttrs.y
  })
  const targetCoords = instance.graphToViewport({
    x: targetAttrs.x, y: targetAttrs.y
  })
  edgeTooltipPosition = {
    x: canvasRect.left + (sourceCoords.x + targetCoords.x) / 2,
    y: canvasRect.top + (sourceCoords.y + targetCoords.y) / 2,
  }
})

instance.on('leaveEdge', () => {
  hoveredEdge = null
  edgeTooltip = null
})
```

> **Note on positioning:** `graphToViewport` returns coordinates relative to the canvas element. Adding `canvasRect.left`/`canvasRect.top` converts them to page-absolute coordinates suitable for `position: fixed`. The tooltip template must be placed OUTSIDE the `.graph-layout` container to avoid `overflow: hidden` clipping.

### 2.4 Tooltip Template

The tooltip must be rendered OUTSIDE `.graph-layout` (which has `overflow: hidden`) to avoid clipping:

```svelte
{:else if nodeCount === 0}
  <EmptyState ... />
{:else}
  <div class="graph-layout">
    <!-- graph container and detail panel -->
  </div>

  <!-- Edge tooltip: OUTSIDE .graph-layout to avoid overflow: hidden clipping -->
  {#if edgeTooltip}
    <div class="edge-tooltip" style="left: {edgeTooltipPosition.x}px; top: {edgeTooltipPosition.y}px;">
      <div class="edge-tooltip-row">
        <span class="edge-node-type" style:background={NODE_COLORS[edgeTooltip.sourceType]}>
          {edgeTooltip.sourceType[0].toUpperCase()}
        </span>
        <span class="edge-tooltip-label">{truncate(edgeTooltip.sourceLabel, 40)}</span>
        <span class="edge-arrow">&rarr;</span>
        <span class="edge-node-type" style:background={NODE_COLORS[edgeTooltip.targetType]}>
          {edgeTooltip.targetType[0].toUpperCase()}
        </span>
        <span class="edge-tooltip-label">{truncate(edgeTooltip.targetLabel, 40)}</span>
      </div>
      <div class="edge-tooltip-meta">
        {#if edgeTooltip.isPrimary}
          <span class="primary-badge">Primary</span>
        {/if}
        {#if edgeTooltip.drifted}
          <span class="drift-badge">Snapshot drifted</span>
        {:else}
          <span class="match-badge">Snapshot matches</span>
        {/if}
      </div>
    </div>
  {/if}
{/if}
```

### 2.5 Tooltip Styling

```css
.edge-tooltip {
  position: fixed;
  background: #1a1a2e;
  color: #e0e0e0;
  padding: 0.5rem 0.75rem;
  border-radius: 6px;
  font-size: 0.8rem;
  pointer-events: none;
  z-index: 100;
  max-width: 400px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  transform: translate(-50%, -100%);
  margin-top: -8px;
}

.edge-tooltip-row {
  display: flex;
  align-items: center;
  gap: 0.35rem;
}

.edge-node-type {
  width: 18px;
  height: 18px;
  border-radius: 50%;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  font-size: 0.65rem;
  font-weight: 700;
  color: white;
  flex-shrink: 0;
}

.edge-arrow { color: #6b7280; font-size: 0.9rem; }

.edge-tooltip-meta {
  display: flex;
  gap: 0.5rem;
  margin-top: 0.35rem;
  font-size: 0.75rem;
}

.primary-badge {
  background: #dbeafe;
  color: #1e40af;
  padding: 0.1em 0.4em;
  border-radius: 3px;
  font-weight: 600;
}

.drift-badge { color: #ef4444; }
.match-badge { color: #22c55e; }
```

> **Performance note:** Edge events use mousemove-based picking (Sigma renders to an offscreen color buffer on each mousemove). With <500 edges this is imperceptible. At scale, consider disabling `enableEdgeEvents` and using node-only interactions.

---

## 3. Edge Click Interaction

### 3.1 Behavior

When a user clicks an edge, the detail panel shows both connected nodes:

```
Edge: Source -> Bullet
From: Principal Cloud Forensics Engineer (source, approved)
To:   Led 4-engineer team migrating... (bullet, approved)

Relationship:
  Primary: Yes
  Snapshot: Drifted
    Snapshot: "Led a team of 4 engineers to migrate..."
    Current:  "Led a senior team of 5 engineers to migrate..."
```

### 3.2 Implementation

```typescript
instance.on('clickEdge', ({ edge }: { edge: string }) => {
  const attrs = graph!.getEdgeAttributes(edge)
  const [sourceNode, targetNode] = graph!.extremities(edge)
  const sourceAttrs = graph!.getNodeAttributes(sourceNode)
  const targetAttrs = graph!.getNodeAttributes(targetNode)

  selectedNodeId = null       // deselect any node highlight
  selectedNodeData = null     // MUST clear node detail panel too
  selectedEdgeData = {
    sourceLabel: sourceAttrs.label,
    sourceType: sourceAttrs.entityType,
    sourceId: sourceAttrs.entityId,
    sourceStatus: sourceAttrs.status,
    targetLabel: targetAttrs.label,
    targetType: targetAttrs.entityType,
    targetId: targetAttrs.entityId,
    targetStatus: targetAttrs.status,
    isPrimary: attrs.isPrimary ?? false,
    drifted: attrs.drifted ?? false,
  }
})
```

> **Important:** `selectedNodeData` must be cleared alongside `selectedNodeId`. If only `selectedNodeId` is nulled, the `{#if selectedNodeData}` template branch remains truthy and the node detail panel stays visible instead of showing the edge detail panel.

### 3.3 clickStage Clears Edge Selection

The existing `clickStage` handler must also clear edge selection:

```typescript
instance.on('clickStage', () => {
  selectedNodeId = null
  selectedNodeData = null
  selectedEdgeData = null  // clear edge selection too
})
```

### 3.4 Detail Panel for Edges

The detail panel template uses `{#if selectedNodeData}` / `{:else if selectedEdgeData}` branching so that only one panel type shows at a time:

```svelte
{#if selectedNodeData}
  <div class="detail-panel">
    <!-- existing node detail panel (see section 5 for enhancements) -->
  </div>
{:else if selectedEdgeData}
  <div class="detail-panel">
    <div class="detail-header">
      <h3 class="detail-title">Edge Detail</h3>
      <button class="close-btn" onclick={() => { selectedEdgeData = null }}>Close</button>
    </div>
    <div class="detail-body">
      <div class="edge-detail-row">
        <span class="edge-detail-icon" style:background={NODE_COLORS[selectedEdgeData.sourceType]}>
          {selectedEdgeData.sourceType[0].toUpperCase()}
        </span>
        <div>
          <strong>{selectedEdgeData.sourceLabel}</strong>
          <StatusBadge status={selectedEdgeData.sourceStatus} />
        </div>
      </div>
      <div class="edge-detail-arrow">&rarr;</div>
      <div class="edge-detail-row">
        <span class="edge-detail-icon" style:background={NODE_COLORS[selectedEdgeData.targetType]}>
          {selectedEdgeData.targetType[0].toUpperCase()}
        </span>
        <div>
          <strong>{selectedEdgeData.targetLabel}</strong>
          <StatusBadge status={selectedEdgeData.targetStatus} />
        </div>
      </div>
      <div class="edge-detail-meta">
        {#if selectedEdgeData.isPrimary}<span class="primary-badge">Primary</span>{/if}
        {#if selectedEdgeData.drifted}<span class="drift-badge">Drifted</span>
        {:else}<span class="match-badge">Matches</span>{/if}
      </div>
    </div>
  </div>
{/if}
```

---

## 4. Edge Highlighting on Node Selection

### 4.1 Behavior

When a node is selected (clicked), its connected edges should be visually emphasized:
- Connected edges: full opacity, slightly thicker
- Non-connected edges: reduced opacity (0.15)
- Connected neighbor nodes: full opacity
- Non-connected nodes: reduced opacity (0.15)

This creates a "focus" effect that shows the selected node's neighborhood.

### 4.2 Implementation

Use Sigma's `nodeReducer` and `edgeReducer`:

```typescript
const instance = new Sigma(graph!, container!, {
  // ... existing config ...
  zIndex: true,  // Required for zIndex values in reducers to take effect
  nodeReducer: (node, data) => {
    if (!selectedNodeId) return data
    if (node === selectedNodeId || graph!.hasEdge(selectedNodeId, node) || graph!.hasEdge(node, selectedNodeId)) {
      return { ...data, zIndex: 1 }
    }
    return { ...data, color: '#e5e7eb', label: '', zIndex: 0 }
  },
  edgeReducer: (edge, data) => {
    const [src, tgt] = graph!.extremities(edge)
    const srcAttrs = graph!.getNodeAttributes(src)
    const tgtAttrs = graph!.getNodeAttributes(tgt)

    // Hide edges with hidden endpoints (filtered graphs)
    if (srcAttrs.hidden || tgtAttrs.hidden) {
      return { ...data, hidden: true }
    }

    // Node selection dimming
    if (selectedNodeId) {
      if (src === selectedNodeId || tgt === selectedNodeId) {
        return { ...data, size: (data.size ?? 1) + 1, zIndex: 1 }
      }
      return { ...data, color: '#f3f4f6', size: 0.5, zIndex: 0 }
    }

    // Edge hover highlight
    if (hoveredEdge === edge) {
      return { ...data, size: (data.size ?? 1) + 1 }
    }

    return data
  },
})
```

Note: `nodeReducer` and `edgeReducer` run on every render frame. They must be fast (no async, no DOM access). The `selectedNodeId` reference should be read from a closure or module scope, not from a Svelte `$state` directly inside the reducer (Sigma callbacks are not reactive). Update the reducer reference when `selectedNodeId` changes by calling `sigmaInstance.refresh()`.

### 4.3 Refresh on Selection Change

```typescript
$effect(() => {
  const _sel = selectedNodeId       // track node selection changes
  const _edge = selectedEdgeData    // track edge selection changes
  const _hover = hoveredEdge        // track edge hover changes
  sigmaInstance?.refresh()
})
```

---

## 5. Bullet Detail: Related Perspectives

When a **bullet node** is selected in the graph, the detail panel should show not only the bullet's own content but also an expandable list of all perspectives derived from it.

### 5.1 Data

The perspectives are already loaded during graph building (`perspectivesRes.data`). Filter by `bullet_id` matching the selected bullet's `entityId`.

`perspectives` must be stored at component scope (not just inside `buildGraph`) so the derived can access it:

```typescript
// At component scope, alongside other state
let perspectives = $state<Perspective[]>([])

// Inside buildGraph(), after fetching:
perspectives = perspectivesRes.data
```

Similarly for bullets and sources:

```typescript
let allBullets = $state<Bullet[]>([])
let allSources = $state<Source[]>([])

// Inside buildGraph():
allBullets = bulletsRes.data
allSources = sourcesRes.data
```

### 5.2 Derived State

```typescript
let relatedPerspectives = $derived(
  selectedNodeData?.type === 'bullet'
    ? perspectives.filter(p => p.bullet_id === selectedNodeData.entityId)
    : []
)

let relatedBullets = $derived(
  selectedNodeData?.type === 'source'
    ? allBullets.filter(b =>
        b.sources?.some(s => s.id === selectedNodeData.id)
      )
    : []
)

let parentBullet = $derived(
  selectedNodeData?.type === 'perspective'
    ? allBullets.find(b =>
        perspectives.find(p => p.id === selectedNodeData.id)?.bullet_id === b.id
      ) ?? null
    : null
)
```

### 5.3 Detail Panel Enhancement

When `selectedNodeData.type === 'bullet'`, the detail panel shows:
1. Bullet content (existing)
2. Bullet metadata: status, domain (existing)
3. **Perspectives section** (new): list of all perspectives for this bullet

```svelte
{#if selectedNodeData.type === 'bullet'}
  <div class="related-perspectives">
    <h4 class="related-header">
      Perspectives ({relatedPerspectives.length})
    </h4>
    {#each relatedPerspectives as persp}
      <button class="perspective-card"
        onclick={() => selectNodeInGraph(`perspective-${persp.id}`)}
      >
        <div class="persp-content">{truncate(persp.content, 80)}</div>
        <div class="persp-meta">
          {#if persp.target_archetype}
            <span class="archetype-tag">{persp.target_archetype}</span>
          {/if}
          {#if persp.domain}
            <span class="domain-tag">{persp.domain}</span>
          {/if}
          <StatusBadge status={persp.status} />
        </div>
      </button>
    {/each}
  </div>
{/if}
```

When `selectedNodeData.type === 'source'`, show related bullets:

```svelte
{#if selectedNodeData.type === 'source'}
  <div class="related-bullets">
    <h4 class="related-header">
      Bullets ({relatedBullets.length})
    </h4>
    {#each relatedBullets as bullet}
      <button class="perspective-card"
        onclick={() => selectNodeInGraph(`bullet-${bullet.id}`)}
      >
        <div class="persp-content">{truncate(bullet.content, 80)}</div>
        <div class="persp-meta">
          {#if bullet.domain}
            <span class="domain-tag">{bullet.domain}</span>
          {/if}
          <StatusBadge status={bullet.status} />
        </div>
      </button>
    {/each}
  </div>
{/if}
```

When `selectedNodeData.type === 'perspective'`, show parent bullet:

```svelte
{#if selectedNodeData.type === 'perspective' && parentBullet}
  <div class="related-parent">
    <h4 class="related-header">Parent Bullet</h4>
    <button class="perspective-card"
      onclick={() => selectNodeInGraph(`bullet-${parentBullet.id}`)}
    >
      <div class="persp-content">{truncate(parentBullet.content, 80)}</div>
      <div class="persp-meta">
        {#if parentBullet.domain}
          <span class="domain-tag">{parentBullet.domain}</span>
        {/if}
        <StatusBadge status={parentBullet.status} />
      </div>
    </button>
  </div>
{/if}
```

### 5.4 `selectNodeInGraph` Function

Centers the camera on a node and selects it:

```typescript
function selectNodeInGraph(nodeKey: string) {
  if (!graph?.hasNode(nodeKey) || !sigmaInstance) return
  const attrs = graph.getNodeAttributes(nodeKey)
  selectedNodeId = nodeKey
  selectedNodeData = {
    id: attrs.entityId,
    label: attrs.label,
    type: attrs.entityType,
    content: attrs.content,
    status: attrs.status,
    sourceType: attrs.sourceType,
    archetype: attrs.archetype,
    domain: attrs.domain,
  }
  const displayData = sigmaInstance.getNodeDisplayData(nodeKey)
  if (displayData) {
    sigmaInstance.getCamera().animate(
      { x: displayData.x, y: displayData.y, ratio: 0.3 },
      { duration: 300 }
    )
  }
}
```

---

## 6. Edge Legend Enhancement

### 6.1 Updated Legend

Add edge legend items to the existing stats bar:

```svelte
<div class="legend">
  <!-- Existing node legend -->
  <span class="legend-item">
    <span class="legend-dot" style:background={NODE_COLORS.source}></span> Source
  </span>
  <!-- ... existing ... -->

  <!-- Edge legend -->
  <span class="legend-divider">|</span>
  <span class="legend-item">
    <span class="legend-line" style:background="#94a3b8"></span> Matching
  </span>
  <span class="legend-item">
    <span class="legend-line" style:background="#ef4444"></span> Drifted
  </span>
</div>
```

CSS for edge legend:
```css
.legend-line {
  display: inline-block;
  width: 16px;
  height: 2px;
  vertical-align: middle;
  margin-right: 4px;
}
.legend-divider {
  color: #d1d5db;
  margin: 0 0.25rem;
}
```

---

## 7. Acceptance Criteria

### Edge Rendering
- [ ] Edges render as arrows (arrowhead at target node)
- [ ] `EdgeArrowProgram` registered in Sigma constructor (explicitly, for visibility)
- [ ] `zIndex: true` enabled in Sigma config
- [ ] Primary source edges are thicker (size 2) than non-primary (size 1)
- [ ] Matching snapshot edges are gray (`#94a3b8`)
- [ ] Drifted edges are red (`#ef4444`)
- [ ] Arrow direction shows derivation flow: source -> bullet -> perspective
- [ ] Edges to hidden nodes are also hidden when filters are applied

### Edge Hover
- [ ] Hovering over an edge shows tooltip at edge midpoint
- [ ] Edge hover tooltip not clipped by graph container (rendered outside `.graph-layout`)
- [ ] Hovered edge is visually thickened (size + 1)
- [ ] Tooltip shows source and target node labels (not UUIDs)
- [ ] Tooltip shows "Primary" badge for primary source edges
- [ ] Tooltip shows snapshot status (matches or drifted)
- [ ] Tooltip disappears on mouse leave

### Edge Click
- [ ] Clicking an edge opens the detail panel with both connected nodes
- [ ] Detail panel shows From/To labels, types, statuses
- [ ] Detail panel shows relationship info (primary, drift status)
- [ ] Node selection highlight cleared when clicking an edge (`selectedNodeData = null`)
- [ ] Clicking background clears edge selection (`selectedEdgeData = null`)

### Node Selection Highlighting
- [ ] Selecting a node dims non-connected nodes and edges (opacity 0.15)
- [ ] Connected edges become slightly thicker
- [ ] Connected neighbor nodes remain at full opacity
- [ ] Deselecting (click background) restores all nodes and edges

### Node Detail: Related Entities
- [ ] Bullet detail panel shows list of related perspectives
- [ ] Each perspective shows content preview, archetype, domain, status
- [ ] Clicking a perspective in the list navigates to it in the graph (camera centers + selects)
- [ ] Source detail panel shows related bullets
- [ ] Perspective detail panel shows parent bullet

### Legend
- [ ] Edge legend shows gray line = Matching, red line = Drifted

### Tests
- [ ] Graph with drifted edge: edge renders in red
- [ ] Graph with primary and non-primary edges: primary is thicker
- [ ] Edge hover: tooltip appears with correct source/target labels
- [ ] Node selection: non-connected nodes are dimmed
- [ ] Node deselection: all nodes restored
- [ ] Filter + edge visibility: filter to show only "approved" nodes, verify edges to hidden nodes also hidden
- [ ] Click edge then click background: verify edge panel closes
- [ ] Click edge then click node: verify edge panel closes, node panel opens
- [ ] Multiple sources to one bullet: verify both edges independently hoverable

---

## 8. Dependencies & Parallelization

### Sequential
1. Register `EdgeArrowProgram` -- must be first (edges won't render as arrows without it)
2. Edge hover + click handlers -- depend on edges being visible

### Parallel (after arrow program registered)
- Edge hover tooltip
- Edge click detail panel
- Node selection highlighting
- Bullet detail: related perspectives
- Legend enhancement

### Files to Modify
- `packages/webui/src/routes/chain/+page.svelte` -- all changes are in this single file:
  - Import `EdgeArrowProgram` from `sigma/rendering`
  - Add `edgeProgramClasses` and `zIndex: true` to Sigma constructor
  - Add `edgeReducer` with filter-hiding, selection dimming, and hover highlight
  - Add `nodeReducer` for selection dimming
  - Add `enterEdge`/`leaveEdge` handlers
  - Add `clickEdge` handler (clears both `selectedNodeId` and `selectedNodeData`)
  - Update `clickStage` handler to also clear `selectedEdgeData`
  - Add `$effect` to refresh Sigma on selection/hover change
  - Add edge tooltip template OUTSIDE `.graph-layout` + CSS
  - Add edge detail panel branch (`{:else if selectedEdgeData}`)
  - Add `SelectedEdgeData` interface and state declarations
  - Add `hoveredEdge` state
  - Promote `perspectives`, `allBullets`, `allSources` to component scope
  - Add `relatedPerspectives`, `relatedBullets`, `parentBullet` derived state
  - Add `selectNodeInGraph` function
  - Add related-entity sections to node detail panel
  - Update legend with edge indicators

### No New Dependencies
`EdgeArrowProgram` is bundled with `sigma` v3. No additional packages needed.

---

## 9. Known Limitations

- **Dashed edges:** Sigma.js v3's `EdgeArrowProgram` does not support dashed lines. Drift is indicated by color only (red). A custom WebGL edge program would be needed for dashes -- deferred.
- **Edge label text:** Not displayed (too cluttered with many edges). Tooltip on hover provides the information instead.
- **Self-loops:** Not possible in the data model (a source can't derive from itself), so no handling needed.
- **Overlapping edges:** When multiple sources connect to the same bullet, edges may overlap. ForceAtlas2 layout minimizes this but cannot eliminate it. The hover tooltip helps distinguish overlapping edges.
- **Edge event performance:** Edge events use mousemove-based picking (Sigma renders to an offscreen color buffer on each mousemove). With <500 edges this is imperceptible. At scale, consider disabling `enableEdgeEvents` and using node-only interactions.
