# Phase 25: Chain View Edge Rendering & Interaction

**Goal:** Add edge arrows, hover tooltips, click detail panels, node selection highlighting (dimming non-connected elements), and related-entity detail panels to the Chain View graph. All changes are in a single file: `packages/webui/src/routes/chain/+page.svelte`.

**Non-Goals:** Edge labels (too cluttered). Animated edge flows. Edge editing (adding/removing associations). Dashed edge lines (requires custom WebGL program). Resume entry nodes in the graph. Snapshot diff text display in edge detail panel (deferred -- show boolean drift indicator only).

**Depends on:** Nothing (can start immediately)
**Blocks:** Phase 24 T24.4 (modifies the same file)

**Internal task parallelization:** All tasks modify `packages/webui/src/routes/chain/+page.svelte`. They should be implemented as a single combined effort or applied sequentially. The task breakdown is for logical organization, not parallel execution. T25.1 must complete first (edges must render before interaction is added). T25.7 depends on all prior tasks.

**Tech Stack:** TypeScript, Svelte 5 runes, Sigma.js v3, Graphology

**Reference:** `refs/specs/2026-03-30-chain-view-edge-rendering.md`

**Architecture:**
- All changes in `packages/webui/src/routes/chain/+page.svelte`
- `EdgeArrowProgram` from `sigma/rendering` (bundled with sigma v3, no new dependencies)
- `nodeReducer` and `edgeReducer` for visual state management (dimming, hiding, hover)
- Edge tooltip rendered OUTSIDE `.graph-layout` to avoid `overflow: hidden` clipping
- Data promoted from local scope to component scope for derived state access

**Fallback Strategies:**
- If `EdgeArrowProgram` import fails (bundler issue), `defaultEdgeType: 'arrow'` may still work since Sigma v3 registers it by default -- verify and remove explicit import if so
- If edge event performance degrades with many edges (>500), disable `enableEdgeEvents` and rely on node-only interactions
- If `graphToViewport` returns coordinates that are off-screen (edge midpoint outside viewport), clamp tooltip to viewport bounds

---

## Context

The Chain View graph (`chain/+page.svelte`) currently creates edges in the graphology data model with arrow types, drift coloring (gray `#94a3b8` vs red `#ef4444`), and primary/non-primary sizing. Sigma.js v3 registers `EdgeArrowProgram` by default for the `arrow` edge type, so arrows may already render. However:

1. **No explicit arrow program registration** -- making the dependency invisible
2. **No `zIndex: true`** -- required for `nodeReducer`/`edgeReducer` z-ordering to take effect
3. **No edge interaction** -- no hover tooltip, no click handler, no edge detail panel
4. **No node selection highlighting** -- clicking a node doesn't dim non-connected elements
5. **No related entity display** -- bullet detail panel doesn't show perspectives, source detail doesn't show bullets
6. **No edge legend** -- only node types in the legend

The graph already creates edges with correct attributes:
- `size`: 2 for primary source edges, 1 for others
- `color`: `#94a3b8` (slate gray) for matching snapshots, `#ef4444` (red) for drifted
- `type`: `'arrow'`
- `drifted`: boolean
- `isPrimary`: boolean (source->bullet edges only)

---

## Tasks

### Task 25.1: Register EdgeArrowProgram + zIndex

**File to modify:** `packages/webui/src/routes/chain/+page.svelte`

**Goal:** Explicitly register `EdgeArrowProgram` in the Sigma constructor and enable `zIndex` for reducer-based z-ordering.

#### Steps

- [ ] **Add import** at the top of the `<script>` block (after existing imports):

```typescript
import { EdgeArrowProgram } from 'sigma/rendering'
```

Note: This is a dynamic import context -- the Sigma import is already `await import('sigma')`. The `EdgeArrowProgram` import can be a static import since it's only used in the browser-only `initSigma()` function guarded by the `if (!browser)` check. If the bundler tree-shakes it incorrectly, move it inside `initSigma()` as a dynamic import.

- [ ] **Update the Sigma constructor** (inside `initSigma()`) to add `edgeProgramClasses` and `zIndex`:

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
  // nodeReducer and edgeReducer added in T25.2
})
```

#### Acceptance Criteria
- [ ] `EdgeArrowProgram` imported from `sigma/rendering`
- [ ] `edgeProgramClasses: { arrow: EdgeArrowProgram }` in Sigma config
- [ ] `zIndex: true` in Sigma config
- [ ] Build passes: `cd packages/webui && bun run build`
- [ ] Edges render as arrows with arrowheads at target nodes
- [ ] Arrow direction shows derivation flow: source -> bullet -> perspective

---

### Task 25.2: Add nodeReducer + edgeReducer

**File to modify:** `packages/webui/src/routes/chain/+page.svelte`

**Goal:** Add `nodeReducer` and `edgeReducer` to Sigma for selection dimming, edge hover highlighting, and filter-based edge hiding. Add `hoveredEdge` state. Add `$effect` to refresh Sigma on selection/hover changes.

#### Steps

- [ ] **Declare all state variables needed by reducers and effects upfront** (in the State section, after existing state). The full `SelectedEdgeData` interface and click handler logic are defined in T25.4, but the `$effect` in this task references `selectedEdgeData`, `hoveredEdge`, `edgeTooltip`, and `edgeTooltipPosition`. These variables must exist before the reducers and effect can compile:

```typescript
// Declare all state variables needed by reducers and effects upfront
// (Full SelectedEdgeData interface and click handler logic defined in T25.4)
let selectedEdgeData = $state<Record<string, unknown> | null>(null)
let hoveredEdge = $state<string | null>(null)
let edgeTooltip = $state<Record<string, unknown> | null>(null)
let edgeTooltipPosition = $state({ x: 0, y: 0 })
```

- [ ] **Add `nodeReducer` to the Sigma constructor:**

```typescript
nodeReducer: (node, data) => {
  if (!selectedNodeId) return data
  if (
    node === selectedNodeId ||
    graph!.hasEdge(selectedNodeId, node) ||
    graph!.hasEdge(node, selectedNodeId)
  ) {
    return { ...data, zIndex: 1 }
  }
  return { ...data, color: '#e5e7eb', label: '', zIndex: 0 }
},
```

This dims non-connected nodes when a node is selected: connected neighbors retain full styling, non-connected nodes get a light gray color and hidden labels.

- [ ] **Add `edgeReducer` to the Sigma constructor:**

```typescript
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
```

The reducer handles three concerns in priority order:
1. **Filter hiding**: edges to hidden nodes are also hidden
2. **Selection dimming**: connected edges are thickened, non-connected are dimmed
3. **Hover highlight**: hovered edge is thickened

- [ ] **Add `$effect` to refresh Sigma** on selection/hover state changes (after the Sigma init effect):

```typescript
$effect(() => {
  const _sel = selectedNodeId       // track node selection changes
  const _edge = selectedEdgeData    // track edge selection changes
  const _hover = hoveredEdge        // track edge hover changes
  sigmaInstance?.refresh()
})
```

Note: `nodeReducer` and `edgeReducer` run on every render frame. They must be fast (no async, no DOM access). The `selectedNodeId` and `hoveredEdge` references are read from Svelte `$state` which is accessible in the closure. The `$effect` triggers `sigmaInstance.refresh()` to re-run the reducers when these states change.

#### Acceptance Criteria
- [ ] `hoveredEdge` state declared
- [ ] `nodeReducer` dims non-connected nodes when a node is selected
- [ ] `nodeReducer` preserves full styling for selected node and its neighbors
- [ ] `edgeReducer` hides edges with hidden endpoints (filter support)
- [ ] `edgeReducer` dims non-connected edges when a node is selected
- [ ] `edgeReducer` thickens connected edges when a node is selected
- [ ] `edgeReducer` thickens hovered edge
- [ ] `$effect` refreshes Sigma on selection/hover changes
- [ ] Clicking background restores all nodes and edges to normal

---

### Task 25.3: Edge hover tooltip

**File to modify:** `packages/webui/src/routes/chain/+page.svelte`

**Goal:** Add `enterEdge`/`leaveEdge` event handlers that show a tooltip at the edge midpoint with source/target labels, primary badge, and drift status.

#### Steps

- [ ] **Add tooltip state declarations** (in the State section):

```typescript
let edgeTooltip = $state<{
  sourceLabel: string
  sourceType: string
  targetLabel: string
  targetType: string
  isPrimary: boolean
  drifted: boolean
} | null>(null)
let edgeTooltipPosition = $state({ x: 0, y: 0 })
```

- [ ] **Add `enterEdge` handler** (inside `initSigma()`, after the click handlers):

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

  // Position tooltip near the edge midpoint using page-absolute coordinates.
  // graphToViewport returns coords relative to the canvas element.
  // Adding canvasRect offsets converts to page-absolute for position: fixed.
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
```

- [ ] **Add `leaveEdge` handler:**

```typescript
instance.on('leaveEdge', () => {
  hoveredEdge = null
  edgeTooltip = null
})
```

- [ ] **Add tooltip template** -- render OUTSIDE `.graph-layout` to avoid `overflow: hidden` clipping. Place it after the closing `</div>` of `.graph-layout` but before the closing `{/if}` of the `{:else}` branch:

```svelte
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

- [ ] **Add tooltip CSS** (in the `<style>` block):

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

.edge-tooltip-label {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 140px;
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

#### Acceptance Criteria
- [ ] `enterEdge` handler sets `hoveredEdge`, `edgeTooltip`, and `edgeTooltipPosition`
- [ ] `leaveEdge` handler clears `hoveredEdge` and `edgeTooltip`
- [ ] Tooltip positioned at edge midpoint using `graphToViewport()` + canvas rect offsets
- [ ] Tooltip rendered OUTSIDE `.graph-layout` (not clipped by `overflow: hidden`)
- [ ] Tooltip shows source and target node labels (not UUIDs)
- [ ] Tooltip shows colored type badges (S/B/P)
- [ ] Tooltip shows "Primary" badge for primary source edges
- [ ] Tooltip shows snapshot status (matches/drifted)
- [ ] Tooltip has `pointer-events: none` (non-interactive)
- [ ] Hovered edge is visually thickened (via `edgeReducer`)
- [ ] Tooltip disappears on mouse leave

---

### Task 25.4: Edge click detail panel

**File to modify:** `packages/webui/src/routes/chain/+page.svelte`

**Goal:** Add a `clickEdge` handler that populates an edge detail panel, and update `clickStage` to clear edge selection.

#### Steps

- [ ] **Add `SelectedEdgeData` interface and state** (in the State section):

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
```

- [ ] **Add `clickEdge` handler** (inside `initSigma()`, after existing click handlers):

```typescript
instance.on('clickEdge', ({ edge }: { edge: string }) => {
  const attrs = graph!.getEdgeAttributes(edge)
  const [sourceNode, targetNode] = graph!.extremities(edge)
  const sourceAttrs = graph!.getNodeAttributes(sourceNode)
  const targetAttrs = graph!.getNodeAttributes(targetNode)

  // Clear node selection -- MUST clear both selectedNodeId and selectedNodeData
  // If only selectedNodeId is nulled, the {#if selectedNodeData} template branch
  // remains truthy and the node detail panel stays visible.
  selectedNodeId = null
  selectedNodeData = null

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

- [ ] **Update `clickStage` handler** to also clear edge selection:

```typescript
instance.on('clickStage', () => {
  selectedNodeId = null
  selectedNodeData = null
  selectedEdgeData = null  // clear edge selection too
})
```

- [ ] **Add edge detail panel template** -- add `{:else if selectedEdgeData}` branch after the existing `{#if selectedNodeData}` block, inside `.graph-layout`:

```svelte
{#if selectedNodeData}
  <div class="detail-panel">
    <!-- existing node detail panel -->
  </div>
{:else if selectedEdgeData}
  <div class="detail-panel">
    <div class="detail-header">
      <h3 class="detail-title">Edge Detail</h3>
      <button class="close-btn" onclick={() => { selectedEdgeData = null }}>
        Close
      </button>
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
        {#if selectedEdgeData.isPrimary}
          <span class="primary-badge">Primary</span>
        {/if}
        {#if selectedEdgeData.drifted}
          <span class="drift-badge">Drifted</span>
        {:else}
          <span class="match-badge">Matches</span>
        {/if}
      </div>
    </div>
  </div>
{/if}
```

- [ ] **Add edge detail CSS** (in the `<style>` block):

```css
.edge-detail-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin-bottom: 0.5rem;
}

.edge-detail-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 1.75rem;
  height: 1.75rem;
  border-radius: 5px;
  font-size: 0.75rem;
  font-weight: 700;
  color: #fff;
  flex-shrink: 0;
}

.edge-detail-arrow {
  text-align: center;
  font-size: 1.2rem;
  color: #6b7280;
  margin: 0.25rem 0;
}

.edge-detail-meta {
  display: flex;
  gap: 0.5rem;
  margin-top: 0.75rem;
  padding-top: 0.75rem;
  border-top: 1px solid #e5e7eb;
}
```

**Deferred: snapshot diff text in edge detail panel.** The spec shows snapshot vs current text comparison for drifted edges. This requires storing `source_content_snapshot` and `bullet_content_snapshot` on edge attributes during `buildGraph()`. For MVP, the edge detail panel shows only the `drifted: boolean` flag with a colored badge. The full diff text is a future enhancement -- the data is available on the SDK response but not currently stored on edge attributes.

#### Acceptance Criteria
- [ ] `SelectedEdgeData` interface defined
- [ ] `selectedEdgeData` state declared
- [ ] `clickEdge` handler clears BOTH `selectedNodeId` and `selectedNodeData`, sets `selectedEdgeData`
- [ ] `clickStage` handler also clears `selectedEdgeData`
- [ ] Edge detail panel shows source and target labels with type badges and status
- [ ] Edge detail panel shows relationship info (primary, drift status)
- [ ] Only one detail panel visible at a time (node or edge, never both)
- [ ] Clicking background clears edge selection

---

### Task 25.5: Related entities in node detail panel

**File to modify:** `packages/webui/src/routes/chain/+page.svelte`

**Goal:** Promote `perspectives`, `allBullets`, `allSources` to component scope. Add derived state for related entities. Add related-entity sections to the node detail panel. Add `selectNodeInGraph` function for navigating to related entities.

#### Steps

- [ ] **Promote data arrays to component scope** (in the State section):

```typescript
let perspectives = $state<Perspective[]>([])
let allBullets = $state<Bullet[]>([])
let allSources = $state<Source[]>([])
```

- [ ] **Assign in `buildGraph()`** -- after fetching data, assign to component-scope state:

```typescript
// Inside buildGraph(), after:
// const sources = sourcesRes.data
// const bullets = bulletsRes.data
// const perspectives = perspectivesRes.data

// Promote to component scope for derived state access
allSources = sources
allBullets = bullets
perspectives = perspectivesRes.data
```

Note: the local `const sources`, `const bullets`, `const perspectives` remain for use within `buildGraph()`. The component-scope `allSources`, `allBullets`, `perspectives` are assigned from them.

Wait -- `perspectives` is already the local variable name. To avoid shadowing, rename the component-scope variable or the local variable. Recommended: keep the component-scope as `perspectives` and rename the local:

```typescript
const sourcesData = sourcesRes.data
const bulletsData = bulletsRes.data
const perspectivesData = perspectivesRes.data

// Promote to component scope
allSources = sourcesData
allBullets = bulletsData
perspectives = perspectivesData

// Use sourcesData, bulletsData, perspectivesData in the rest of buildGraph()
```

**All references to rename in `buildGraph()` scope:**
- Line 63: `const sources = sourcesRes.data` -> `const sourcesData = sourcesRes.data`
- Line 64: `const bullets = bulletsRes.data` -> `const bulletsData = bulletsRes.data`
- Line 65: `const perspectives = perspectivesRes.data` -> `const perspectivesData = perspectivesRes.data`
- Line 70: `for (const source of sources)` -> `for (const source of sourcesData)`
- Line 88: `for (const bullet of bullets)` -> `for (const bullet of bulletsData)`
- Line 108: `const sourceNode = sources.find(...)` -> `const sourceNode = sourcesData.find(...)`
- Line 125: `for (const perspective of perspectives)` -> `for (const perspective of perspectivesData)`
- Line 143: `const parentBullet = bullets.find(...)` -> `const parentBullet = bulletsData.find(...)`

**After renaming, add component-scope assignments:**
```typescript
allSources = sourcesData
allBullets = bulletsData
perspectives = perspectivesData
```

Use find-and-replace within the `buildGraph()` function body only. Do NOT rename the component-scope `$state` variables.

- [ ] **Add derived state** (after the state declarations):

```typescript
let relatedPerspectives = $derived(
  selectedNodeData?.type === 'bullet'
    ? perspectives.filter(p => p.bullet_id === selectedNodeData!.id)
    : []
)

let relatedBullets = $derived(
  selectedNodeData?.type === 'source'
    ? allBullets.filter(b =>
        b.sources?.some(s => s.id === selectedNodeData!.id)
      )
    : []
)

let parentBullet = $derived(
  selectedNodeData?.type === 'perspective'
    ? allBullets.find(b =>
        perspectives.find(p => p.id === selectedNodeData!.id)?.bullet_id === b.id
      ) ?? null
    : null
)
```

- [ ] **Add `selectNodeInGraph` function** (in the Helpers section):

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
  selectedEdgeData = null  // clear edge selection when navigating to a node
  const displayData = sigmaInstance.getNodeDisplayData(nodeKey)
  if (displayData) {
    sigmaInstance.getCamera().animate(
      { x: displayData.x, y: displayData.y, ratio: 0.3 },
      { duration: 300 }
    )
  }
}
```

- [ ] **Add related-entity sections to the node detail panel** -- after the existing detail fields (ID field), before the closing `</div>` of `.detail-body`:

For bullet nodes -- show related perspectives:

```svelte
{#if selectedNodeData.type === 'bullet'}
  <div class="related-section">
    <h4 class="related-header">
      Perspectives ({relatedPerspectives.length})
    </h4>
    {#each relatedPerspectives as persp}
      <button class="related-card"
        onclick={() => selectNodeInGraph(`perspective-${persp.id}`)}
      >
        <div class="related-content">{truncate(persp.content, 80)}</div>
        <div class="related-meta">
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

For source nodes -- show related bullets:

```svelte
{#if selectedNodeData.type === 'source'}
  <div class="related-section">
    <h4 class="related-header">
      Bullets ({relatedBullets.length})
    </h4>
    {#each relatedBullets as bullet}
      <button class="related-card"
        onclick={() => selectNodeInGraph(`bullet-${bullet.id}`)}
      >
        <div class="related-content">{truncate(bullet.content, 80)}</div>
        <div class="related-meta">
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

For perspective nodes -- show parent bullet:

```svelte
{#if selectedNodeData.type === 'perspective' && parentBullet}
  <div class="related-section">
    <h4 class="related-header">Parent Bullet</h4>
    <button class="related-card"
      onclick={() => selectNodeInGraph(`bullet-${parentBullet.id}`)}
    >
      <div class="related-content">{truncate(parentBullet.content, 80)}</div>
      <div class="related-meta">
        {#if parentBullet.domain}
          <span class="domain-tag">{parentBullet.domain}</span>
        {/if}
        <StatusBadge status={parentBullet.status} />
      </div>
    </button>
  </div>
{/if}
```

- [ ] **Add related-entity CSS** (in the `<style>` block):

```css
.related-section {
  margin-top: 1rem;
  padding-top: 1rem;
  border-top: 1px solid #e5e7eb;
}

.related-header {
  font-size: 0.75rem;
  font-weight: 600;
  color: #6b7280;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  margin-bottom: 0.5rem;
}

.related-card {
  display: block;
  width: 100%;
  text-align: left;
  padding: 0.5rem;
  margin-bottom: 0.35rem;
  border: 1px solid #e5e7eb;
  border-radius: 6px;
  background: #fff;
  cursor: pointer;
  font-family: inherit;
  transition: background 0.1s, border-color 0.1s;
}

.related-card:hover {
  background: #f9fafb;
  border-color: #6c63ff;
}

.related-content {
  font-size: 0.8rem;
  color: #374151;
  line-height: 1.4;
  margin-bottom: 0.25rem;
}

.related-meta {
  display: flex;
  gap: 0.35rem;
  flex-wrap: wrap;
  align-items: center;
}

.archetype-tag, .domain-tag {
  display: inline-block;
  padding: 0.1em 0.35em;
  font-size: 0.65rem;
  font-weight: 600;
  border-radius: 3px;
  background: #f3f4f6;
  color: #374151;
}

.archetype-tag {
  background: #ede9fe;
  color: #5b21b6;
}

.domain-tag {
  background: #ecfdf5;
  color: #065f46;
}
```

#### Acceptance Criteria
- [ ] `perspectives`, `allBullets`, `allSources` promoted to component scope
- [ ] `buildGraph()` assigns fetched data to component-scope state
- [ ] `relatedPerspectives` derived: perspectives filtered by `bullet_id` matching selected bullet's `entityId`
- [ ] `relatedBullets` derived: bullets filtered by sources containing selected source's `id`
- [ ] `parentBullet` derived: bullet whose `id` matches the selected perspective's `bullet_id`
- [ ] `selectNodeInGraph` centers camera and selects the target node
- [ ] Bullet detail panel shows list of related perspectives with archetype, domain, status
- [ ] Source detail panel shows list of related bullets with domain, status
- [ ] Perspective detail panel shows parent bullet
- [ ] Clicking a related entity navigates to it in the graph (camera centers + node selects)

---

### Task 25.6: Legend enhancement

**File to modify:** `packages/webui/src/routes/chain/+page.svelte`

**Goal:** Add edge legend items to the existing stats bar legend.

#### Steps

- [ ] **Add edge legend items** after the existing node legend items (inside the `.legend` div):

```svelte
<div class="legend">
  <!-- Existing node legend -->
  <span class="legend-item"><span class="legend-dot" style:background={NODE_COLORS.source}></span> Source</span>
  <span class="legend-item"><span class="legend-dot" style:background={NODE_COLORS.bullet}></span> Bullet</span>
  <span class="legend-item"><span class="legend-dot" style:background={NODE_COLORS.perspective}></span> Perspective</span>
  <span class="legend-item"><span class="legend-dot" style:background={NODE_COLORS.resume_entry}></span> Entry</span>

  <!-- Edge legend -->
  <span class="legend-divider">|</span>
  <span class="legend-item"><span class="legend-line" style:background="#94a3b8"></span> Matching</span>
  <span class="legend-item"><span class="legend-line" style:background="#ef4444"></span> Drifted</span>
</div>
```

- [ ] **Add edge legend CSS**:

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

#### Acceptance Criteria
- [ ] Legend shows gray line = Matching, red line = Drifted
- [ ] Edge legend separated from node legend by a divider

---

### Task 25.7: Tests + verification

**Files:** None created or modified. This is a verification task.

**Goal:** Verify that all edge rendering and interaction features work correctly via build check and manual testing.

#### Steps

- [ ] **Build check:**

```bash
cd packages/webui && bun run build
```

If build fails, fix TypeScript or import errors.

- [ ] **Manual smoke tests:**

| Test | Steps | Expected |
|------|-------|----------|
| Arrows render | Open Chain View, observe edges | Arrowheads visible at target nodes |
| Primary vs non-primary | Find a bullet with multiple sources | Primary edge is thicker (size 2 vs 1) |
| Drift coloring | Find a drifted edge (if any) | Drifted edge is red, matching is gray |
| Edge hover | Hover over an edge | Tooltip appears at edge midpoint |
| Tooltip content | Read tooltip text | Shows source/target labels, primary badge, drift status |
| Tooltip not clipped | Hover edge near graph boundary | Tooltip not cut off by `.graph-layout` overflow |
| Hovered edge thickened | Hover an edge | Edge visually thicker while hovered |
| Edge click | Click an edge | Edge detail panel opens with From/To info |
| Edge click clears node | Select a node, then click an edge | Node panel closes, edge panel opens |
| Background click clears edge | Click edge, then click background | Edge panel closes |
| Node selection dimming | Click a node | Non-connected nodes/edges dimmed |
| Node deselection | Click background after selecting | All nodes/edges restored |
| Filter + edge hiding | Filter to "approved" status | Edges to hidden nodes also hidden |
| Bullet related perspectives | Click a bullet node | Detail panel shows list of related perspectives |
| Source related bullets | Click a source node | Detail panel shows list of related bullets |
| Perspective parent bullet | Click a perspective node | Detail panel shows parent bullet |
| Navigate to related | Click a perspective in bullet detail | Camera centers on perspective, detail panel updates |
| Edge legend | Check legend bar | Gray line = Matching, red line = Drifted visible |
| Multiple sources to one bullet | Find bullet with 2+ sources | Both edges independently hoverable |

#### Acceptance Criteria
- [ ] Build passes: `cd packages/webui && bun run build`
- [ ] All manual smoke tests pass
- [ ] No console errors during edge interaction
- [ ] Performance acceptable with current data size

---

### Task 25.8: Documentation

**Files:** None

**Goal:** No new ADRs or documentation files needed. This phase is a UI enhancement to an existing view.

#### Acceptance Criteria
- [ ] Code comments on `edgeReducer` explaining the three-concern priority order (filter hiding > selection dimming > hover highlight)
- [ ] Code comments on `graphToViewport` usage for tooltip positioning
- [ ] Code comments on why `selectedNodeData` must be cleared alongside `selectedNodeId` in `clickEdge`
- [ ] Code comments on variable renaming in `buildGraph()` to avoid shadowing component-scope state
- [ ] Update `PLAN.md` status line to mark this phase as complete

---

## Testing Requirements

| Category | Test | Location |
|----------|------|----------|
| Build | WebUI builds without errors | `cd packages/webui && bun run build` |
| Smoke | Edges render as arrows with correct colors | Manual |
| Smoke | Primary edges thicker than non-primary | Manual |
| Smoke | Edge hover shows tooltip with source/target labels | Manual |
| Smoke | Tooltip not clipped by overflow:hidden | Manual |
| Smoke | Edge click opens edge detail panel | Manual |
| Smoke | Node selection dims non-connected elements | Manual |
| Smoke | Background click clears all selections | Manual |
| Smoke | Bullet detail shows related perspectives | Manual |
| Smoke | Source detail shows related bullets | Manual |
| Smoke | Perspective detail shows parent bullet | Manual |
| Smoke | Clicking related entity navigates in graph | Manual |
| Smoke | Edge legend shows Matching/Drifted | Manual |
| Smoke | Filter hides edges to hidden nodes | Manual |
| Regression | Click edge then click node: edge panel closes | Manual |
| Regression | Click edge then click background: edge panel closes | Manual |
| Regression | Multiple sources to one bullet: edges independently hoverable | Manual |

No unit tests for this phase. All features are Sigma.js/DOM interactions that require a browser environment. If a Svelte component test harness with canvas support is added in a future phase, add tests for:
- `nodeReducer` returns dimmed attributes for non-connected nodes
- `edgeReducer` returns hidden for edges with hidden endpoints
- `selectNodeInGraph` updates `selectedNodeId` and `selectedNodeData`

---

## Documentation Requirements

- [ ] Inline code comments as specified in T25.8
- [ ] No new documentation files needed
- [ ] No new ADRs needed
