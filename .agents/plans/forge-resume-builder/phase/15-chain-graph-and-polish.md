# Phase 15: Chain Graph + Integration Polish

**Goal:** Replace the tree-based Chain View with an interactive Sigma.js graph visualization, add drift detection banners to entity views, write E2E and contract tests for the schema evolution, and validate all acceptance criteria.

**Non-Goals:** Sankey diagram secondary mode (deferred — toggle stub only). Drag-and-drop resume entry reordering (deferred). Real-time drift notifications (single-user app, on-demand refresh is sufficient).

**Depends on:** Phase 14 (UI views ready), Phase 13 (v1 data imported and verified)
**Blocks:** Nothing (final phase)

**Reference:** `docs/superpowers/specs/2026-03-29-forge-schema-evolution-design.md` sections 2 (Chain View), 8 (Acceptance Criteria)

---

## Task 15.1: Install Sigma.js + graphology

**Goal:** Add graph visualization dependencies to the WebUI package.

**Steps:**

1. Install packages:
   ```bash
   cd packages/webui && bun add sigma graphology graphology-layout-forceatlas2 graphology-types
   ```

2. Verify Sigma.js works with Vite/SvelteKit by creating a minimal smoke test:
   ```typescript
   // packages/webui/src/lib/graph/smoke.test.ts
   import Graph from 'graphology'
   import { describe, it, expect } from 'vitest'

   describe('graphology smoke test', () => {
     it('creates a graph with nodes and edges', () => {
       const graph = new Graph()
       graph.addNode('a', { label: 'Node A', x: 0, y: 0, size: 10, color: '#6c63ff' })
       graph.addNode('b', { label: 'Node B', x: 1, y: 1, size: 10, color: '#3b82f6' })
       graph.addEdge('a', 'b', { type: 'arrow' })

       expect(graph.order).toBe(2)
       expect(graph.size).toBe(1)
       expect(graph.hasEdge('a', 'b')).toBe(true)
     })
   })
   ```

3. Create type declarations if needed for Sigma.js WebGL renderer with SvelteKit:
   ```typescript
   // packages/webui/src/lib/graph/types.ts
   export interface ChainNode {
     id: string
     label: string
     type: 'source' | 'bullet' | 'perspective' | 'resume_entry'
     content: string
     status: string
     sourceType?: string  // for source nodes: 'role' | 'project' | 'education' | 'clearance' | 'general'
     archetype?: string   // for perspective nodes
     domain?: string      // for perspective/bullet nodes
   }

   export interface ChainEdge {
     source: string
     target: string
     drifted: boolean     // true if snapshot does not match current content
     isPrimary: boolean   // true if this is the primary source link
   }

   export const NODE_COLORS: Record<ChainNode['type'], string> = {
     source: '#6c63ff',      // purple
     bullet: '#3b82f6',      // blue
     perspective: '#10b981', // green
     resume_entry: '#f59e0b', // amber
   }
   ```

**Acceptance Criteria:**
- [ ] `sigma`, `graphology`, `graphology-layout-forceatlas2`, `graphology-types` installed in `packages/webui/package.json`
- [ ] Smoke test passes: graphology creates nodes and edges
- [ ] Type definitions for chain graph nodes and edges

**Testing:**
- Unit: graphology smoke test creates graph with correct order and size

---

## Task 15.2: Chain View Graph

**Goal:** Replace the tree-based chain visualization with an interactive Sigma.js WebGL graph.

**File:** Rewrite `packages/webui/src/routes/chain/+page.svelte`

**Architecture:**

The graph is built client-side from API data. The component:
1. Fetches all sources, bullets (with sources array), perspectives, and resume entries
2. Builds a `graphology` directed graph
3. Runs ForceAtlas2 layout for positioning
4. Renders with Sigma.js
5. Handles interactions (click, hover, search, filter)

**State:**

```typescript
import Graph from 'graphology'
import Sigma from 'sigma'
import forceAtlas2 from 'graphology-layout-forceatlas2'
import { forge, friendlyError } from '$lib/sdk'
import { addToast } from '$lib/stores/toast.svelte'
import { LoadingSpinner, EmptyState, StatusBadge } from '$lib/components'
import { NODE_COLORS, type ChainNode, type ChainEdge } from '$lib/graph/types'
import type { Source, Bullet, Perspective } from '@forge/sdk'

let container = $state<HTMLDivElement | null>(null)
let sigmaInstance = $state<Sigma | null>(null)
let graph = $state<Graph | null>(null)
let loading = $state(true)
let error = $state<string | null>(null)

// Interaction state
let selectedNodeId = $state<string | null>(null)
let selectedNodeData = $state<ChainNode | null>(null)
let searchQuery = $state('')
let sourceTypeFilter = $state('all')
let statusFilter = $state('all')
let archetypeFilter = $state('all')

// Stats
let nodeCount = $state(0)
let edgeCount = $state(0)
let driftedCount = $state(0)
```

**Graph building:**

```typescript
async function buildGraph() {
  loading = true
  error = null

  try {
    const [sourcesRes, bulletsRes, perspectivesRes] = await Promise.all([
      forge.sources.list({ limit: 1000 }),
      forge.bullets.list({ limit: 1000 }),
      forge.perspectives.list({ limit: 1000 }),
    ])

    if (!sourcesRes.ok || !bulletsRes.ok || !perspectivesRes.ok) {
      error = 'Failed to load chain data'
      loading = false
      return
    }

    const sources = sourcesRes.data
    const bullets = bulletsRes.data
    const perspectives = perspectivesRes.data

    const g = new Graph({ type: 'directed', multi: false })

    // Add source nodes
    for (const source of sources) {
      g.addNode(`source-${source.id}`, {
        label: source.title,
        x: Math.random() * 100,
        y: Math.random() * 100,
        size: 12,
        color: NODE_COLORS.source,
        type: 'source',
        entityId: source.id,
        entityType: 'source',
        content: source.description,
        status: source.status,
        sourceType: source.source_type,
      })
    }

    // Add bullet nodes and edges to sources
    let drifted = 0
    for (const bullet of bullets) {
      g.addNode(`bullet-${bullet.id}`, {
        label: truncate(bullet.content, 60),
        x: Math.random() * 100,
        y: Math.random() * 100,
        size: 8,
        color: NODE_COLORS.bullet,
        type: 'bullet',
        entityId: bullet.id,
        entityType: 'bullet',
        content: bullet.content,
        status: bullet.status,
        domain: bullet.domain,
      })

      // Edges from source(s) to bullet
      for (const src of bullet.sources ?? []) {
        const sourceNode = sources.find(s => s.id === src.id)
        if (!sourceNode) continue

        const snapshotMatch = bullet.source_content_snapshot === sourceNode.description
        if (!snapshotMatch) drifted++

        g.addEdge(`source-${src.id}`, `bullet-${bullet.id}`, {
          size: src.is_primary ? 2 : 1,
          color: snapshotMatch ? '#94a3b8' : '#ef4444',
          type: 'arrow',
          drifted: !snapshotMatch,
          isPrimary: src.is_primary,
          // Dashed style for drifted edges
          style: snapshotMatch ? 'solid' : 'dashed',
        })
      }
    }

    // Add perspective nodes and edges to bullets
    for (const perspective of perspectives) {
      g.addNode(`perspective-${perspective.id}`, {
        label: truncate(perspective.content, 60),
        x: Math.random() * 100,
        y: Math.random() * 100,
        size: 6,
        color: NODE_COLORS.perspective,
        type: 'perspective',
        entityId: perspective.id,
        entityType: 'perspective',
        content: perspective.content,
        status: perspective.status,
        archetype: perspective.target_archetype,
        domain: perspective.domain,
      })

      const parentBullet = bullets.find(b => b.id === perspective.bullet_id)
      if (parentBullet) {
        const snapshotMatch = perspective.bullet_content_snapshot === parentBullet.content
        if (!snapshotMatch) drifted++

        g.addEdge(`bullet-${perspective.bullet_id}`, `perspective-${perspective.id}`, {
          size: 1,
          color: snapshotMatch ? '#94a3b8' : '#ef4444',
          type: 'arrow',
          drifted: !snapshotMatch,
          style: snapshotMatch ? 'solid' : 'dashed',
        })
      }
    }

    // TODO: Add resume_entry nodes when endpoint available
    // Resume entries would connect perspective -> resume_entry

    // Run ForceAtlas2 layout
    forceAtlas2.assign(g, {
      iterations: 100,
      settings: {
        gravity: 1,
        scalingRatio: 10,
        barnesHutOptimize: g.order > 100,
      },
    })

    graph = g
    nodeCount = g.order
    edgeCount = g.size
    driftedCount = drifted
  } catch (e) {
    error = e instanceof Error ? e.message : 'Failed to build graph'
  } finally {
    loading = false
  }
}
```

**Sigma.js rendering (Svelte 5 lifecycle):**

```typescript
$effect(() => {
  buildGraph()
})

$effect(() => {
  if (container && graph && !sigmaInstance) {
    sigmaInstance = new Sigma(graph, container, {
      renderEdgeLabels: false,
      enableEdgeEvents: true,
      defaultNodeType: 'circle',
      defaultEdgeType: 'arrow',
      labelRenderedSizeThreshold: 6,
    })

    // Click handler
    sigmaInstance.on('clickNode', ({ node }) => {
      const attrs = graph!.getNodeAttributes(node)
      selectedNodeId = node
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
    })

    // Hover handler for edge tooltips
    sigmaInstance.on('enterEdge', ({ edge }) => {
      const attrs = graph!.getEdgeAttributes(edge)
      if (attrs.drifted) {
        // Could show a tooltip — for MVP, visual red color is sufficient
      }
    })

    // Click on background to deselect
    sigmaInstance.on('clickStage', () => {
      selectedNodeId = null
      selectedNodeData = null
    })
  }

  // Cleanup on unmount
  return () => {
    if (sigmaInstance) {
      sigmaInstance.kill()
      sigmaInstance = null
    }
  }
})
```

**Search and filter:**

```typescript
$effect(() => {
  if (!graph || !sigmaInstance) return

  graph.forEachNode((node, attrs) => {
    let visible = true

    // Source type filter
    if (sourceTypeFilter !== 'all' && attrs.entityType === 'source') {
      visible = attrs.sourceType === sourceTypeFilter
    }

    // Status filter
    if (statusFilter !== 'all') {
      visible = visible && attrs.status === statusFilter
    }

    // Archetype filter (perspectives only)
    if (archetypeFilter !== 'all' && attrs.entityType === 'perspective') {
      visible = visible && attrs.archetype === archetypeFilter
    }

    // Search
    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase()
      visible = visible && (
        attrs.content?.toLowerCase().includes(q) ||
        attrs.label?.toLowerCase().includes(q)
      )
    }

    graph!.setNodeAttribute(node, 'hidden', !visible)
  })

  sigmaInstance.refresh()
})
```

**Template:**

```svelte
<div class="chain-page">
  <h1 class="page-title">Chain View</h1>
  <p class="page-description">Interactive provenance graph: Source &rarr; Bullet &rarr; Perspective</p>

  <!-- Controls -->
  <div class="controls">
    <input
      type="text"
      class="search-input"
      placeholder="Search nodes by content..."
      bind:value={searchQuery}
    />

    <select class="filter-select" bind:value={sourceTypeFilter}>
      <option value="all">All source types</option>
      <option value="role">Roles</option>
      <option value="project">Projects</option>
      <option value="education">Education</option>
      <option value="clearance">Clearances</option>
      <option value="general">General</option>
    </select>

    <select class="filter-select" bind:value={statusFilter}>
      <option value="all">All statuses</option>
      <option value="pending_review">Pending</option>
      <option value="approved">Approved</option>
      <option value="rejected">Rejected</option>
    </select>

    <select class="filter-select" bind:value={archetypeFilter}>
      <option value="all">All archetypes</option>
      <option value="agentic-ai">Agentic AI</option>
      <option value="infrastructure">Infrastructure</option>
      <option value="security-engineer">Security Engineer</option>
      <option value="solutions-architect">Solutions Architect</option>
      <option value="public-sector">Public Sector</option>
      <option value="hft">HFT</option>
    </select>
  </div>

  <!-- Stats bar -->
  <div class="stats-bar">
    <span class="stat"><strong>{nodeCount}</strong> nodes</span>
    <span class="stat"><strong>{edgeCount}</strong> edges</span>
    {#if driftedCount > 0}
      <span class="stat drift-stat"><strong>{driftedCount}</strong> drifted</span>
    {/if}
    <!-- Legend -->
    <div class="legend">
      <span class="legend-item"><span class="legend-dot" style:background={NODE_COLORS.source}></span> Source</span>
      <span class="legend-item"><span class="legend-dot" style:background={NODE_COLORS.bullet}></span> Bullet</span>
      <span class="legend-item"><span class="legend-dot" style:background={NODE_COLORS.perspective}></span> Perspective</span>
      <span class="legend-item"><span class="legend-dot" style:background={NODE_COLORS.resume_entry}></span> Entry</span>
    </div>
  </div>

  {#if loading}
    <div class="loading-container">
      <LoadingSpinner size="lg" message="Building chain graph..." />
    </div>
  {:else if error}
    <div class="error-banner">
      <p>{error}</p>
      <button class="retry-btn" onclick={buildGraph}>Retry</button>
    </div>
  {:else}
    <div class="graph-layout">
      <!-- Graph container -->
      <div class="graph-container" bind:this={container}></div>

      <!-- Detail panel (shown when node selected) -->
      {#if selectedNodeData}
        <div class="detail-panel">
          <div class="detail-header">
            <span class="node-icon" style:background={NODE_COLORS[selectedNodeData.type]}>
              {selectedNodeData.type[0].toUpperCase()}
            </span>
            <h3 class="detail-title">{selectedNodeData.label}</h3>
            <button class="btn btn-sm btn-ghost" onclick={() => { selectedNodeId = null; selectedNodeData = null }}>
              Close
            </button>
          </div>

          <div class="detail-body">
            <div class="detail-field">
              <span class="field-label">Type</span>
              <span class="field-value">{selectedNodeData.type}</span>
            </div>

            <div class="detail-field">
              <span class="field-label">Status</span>
              <StatusBadge status={selectedNodeData.status} />
            </div>

            {#if selectedNodeData.sourceType}
              <div class="detail-field">
                <span class="field-label">Source Type</span>
                <span class="field-value">{selectedNodeData.sourceType}</span>
              </div>
            {/if}

            {#if selectedNodeData.archetype}
              <div class="detail-field">
                <span class="field-label">Archetype</span>
                <span class="field-value">{selectedNodeData.archetype}</span>
              </div>
            {/if}

            {#if selectedNodeData.domain}
              <div class="detail-field">
                <span class="field-label">Domain</span>
                <span class="field-value">{selectedNodeData.domain}</span>
              </div>
            {/if}

            <div class="detail-field">
              <span class="field-label">Content</span>
              <p class="field-content">{selectedNodeData.content}</p>
            </div>

            <div class="detail-field">
              <span class="field-label">ID</span>
              <span class="field-value mono">{selectedNodeData.id}</span>
            </div>
          </div>
        </div>
      {/if}
    </div>
  {/if}
</div>
```

**Styles:**

```css
.graph-layout {
  display: flex;
  gap: 0;
  height: calc(100vh - 260px);
  border: 1px solid #e5e7eb;
  border-radius: 8px;
  overflow: hidden;
  background: #fff;
}

.graph-container {
  flex: 1;
  min-width: 0;
}

.detail-panel {
  width: 320px;
  border-left: 1px solid #e5e7eb;
  overflow-y: auto;
  flex-shrink: 0;
}

.detail-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 1rem;
  border-bottom: 1px solid #e5e7eb;
  background: #f9fafb;
}

.node-icon {
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

.detail-title {
  flex: 1;
  font-size: 0.9rem;
  font-weight: 600;
  color: #1a1a1a;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.detail-body {
  padding: 1rem;
}

.detail-field {
  margin-bottom: 1rem;
}

.field-label {
  display: block;
  font-size: 0.7rem;
  font-weight: 600;
  color: #6b7280;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  margin-bottom: 0.25rem;
}

.field-value {
  font-size: 0.85rem;
  color: #1a1a1a;
}

.field-content {
  font-size: 0.85rem;
  color: #374151;
  line-height: 1.5;
  white-space: pre-wrap;
  word-break: break-word;
}

.mono {
  font-family: 'SF Mono', Menlo, monospace;
  font-size: 0.8rem;
}

/* Controls */
.controls {
  display: flex;
  gap: 0.75rem;
  margin-bottom: 1rem;
  flex-wrap: wrap;
}

.search-input {
  flex: 1;
  min-width: 200px;
  padding: 0.45rem 0.75rem;
  border: 1px solid #d1d5db;
  border-radius: 6px;
  font-size: 0.85rem;
}

.search-input:focus {
  outline: none;
  border-color: #6c63ff;
  box-shadow: 0 0 0 2px rgba(108, 99, 255, 0.15);
}

.filter-select {
  padding: 0.45rem 0.75rem;
  border: 1px solid #d1d5db;
  border-radius: 6px;
  font-size: 0.85rem;
  background: #fff;
  cursor: pointer;
}

/* Stats bar */
.stats-bar {
  display: flex;
  align-items: center;
  gap: 1.25rem;
  margin-bottom: 0.75rem;
  font-size: 0.8rem;
  color: #6b7280;
}

.drift-stat {
  color: #ef4444;
}

.legend {
  display: flex;
  gap: 0.75rem;
  margin-left: auto;
}

.legend-item {
  display: flex;
  align-items: center;
  gap: 0.3rem;
  font-size: 0.75rem;
}

.legend-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
}
```

**Acceptance Criteria:**
- [ ] Graph renders with all sources, bullets, and perspectives as nodes
- [ ] Node colors: Source=purple, Bullet=blue, Perspective=green, ResumeEntry=amber
- [ ] Directed edges show derivation flow (source -> bullet -> perspective)
- [ ] Drifted edges shown in red (vs gray for matching)
- [ ] Click node opens detail panel with content, status, type, domain
- [ ] Click background closes detail panel
- [ ] Search filters nodes by content
- [ ] Filter by source_type, status, archetype
- [ ] Stats bar shows node count, edge count, drifted count
- [ ] Legend shows color mapping for node types
- [ ] ForceAtlas2 layout positions nodes in readable arrangement
- [ ] Graph handles 100+ nodes without performance issues

**Testing:**
- Unit: graphology graph construction produces correct node/edge counts
- Component: Detail panel renders with mock node data
- Smoke: Chain View page loads and renders graph container
- Visual: Graph renders with imported v1 data (73 bullets, 13 sources, etc.)

---

## Task 15.3: Drift Detection Banner Component

**Goal:** Create a reusable banner component that shows when a bullet or perspective has a stale snapshot, with actions to view diff, re-derive, or dismiss.

**File:** Create `packages/webui/src/lib/components/DriftBanner.svelte`

**Props:**

```typescript
interface DriftBannerProps {
  snapshotContent: string    // what was captured at derivation time
  currentContent: string     // what the parent entity currently says
  entityType: 'bullet' | 'perspective'  // for messaging
  onRederive?: () => void    // optional callback for re-derive action
  onDismiss?: () => void     // optional callback for dismiss action
}
```

**Component:**

```svelte
<script lang="ts">
  let {
    snapshotContent,
    currentContent,
    entityType,
    onRederive,
    onDismiss,
  } = $props<{
    snapshotContent: string
    currentContent: string
    entityType: 'bullet' | 'perspective'
    onRederive?: () => void
    onDismiss?: () => void
  }>()

  let showDiff = $state(false)
  let isDrifted = $derived(snapshotContent !== currentContent)
</script>

{#if isDrifted}
  <div class="drift-banner">
    <div class="drift-message">
      <span class="drift-icon">&#x26A0;</span>
      <span>
        {entityType === 'bullet'
          ? 'Source content has changed since this bullet was derived.'
          : 'Bullet content has changed since this perspective was derived.'}
      </span>
    </div>
    <div class="drift-actions">
      <button class="drift-btn" onclick={() => showDiff = !showDiff}>
        {showDiff ? 'Hide diff' : 'View diff'}
      </button>
      {#if onRederive}
        <button class="drift-btn drift-rederive" onclick={onRederive}>
          Re-derive
        </button>
      {/if}
      {#if onDismiss}
        <button class="drift-btn drift-dismiss" onclick={onDismiss}>
          Dismiss
        </button>
      {/if}
    </div>
  </div>

  {#if showDiff}
    <div class="drift-diff">
      <div class="diff-columns">
        <div class="diff-column snapshot-column">
          <div class="diff-column-header">Snapshot (at derivation time)</div>
          <div class="diff-column-body">{snapshotContent}</div>
        </div>
        <div class="diff-column current-column">
          <div class="diff-column-header">Current content</div>
          <div class="diff-column-body">{currentContent}</div>
        </div>
      </div>
    </div>
  {/if}
{/if}

<style>
  .drift-banner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    padding: 0.75rem 1rem;
    background: #fffbeb;
    border: 1px solid #fde68a;
    border-radius: 6px;
    margin-bottom: 0.75rem;
  }

  .drift-message {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.85rem;
    color: #92400e;
  }

  .drift-icon {
    font-size: 1rem;
    flex-shrink: 0;
  }

  .drift-actions {
    display: flex;
    gap: 0.5rem;
    flex-shrink: 0;
  }

  .drift-btn {
    padding: 0.3rem 0.65rem;
    border: 1px solid #fcd34d;
    border-radius: 4px;
    background: #fef3c7;
    color: #92400e;
    font-size: 0.75rem;
    font-weight: 500;
    cursor: pointer;
    font-family: inherit;
    transition: background 0.1s;
  }

  .drift-btn:hover {
    background: #fde68a;
  }

  .drift-rederive {
    background: #6c63ff;
    border-color: #6c63ff;
    color: #fff;
  }

  .drift-rederive:hover {
    background: #5a52e0;
  }

  .drift-dismiss {
    background: transparent;
    border-color: #d1d5db;
    color: #6b7280;
  }

  .drift-dismiss:hover {
    background: #f3f4f6;
  }

  .drift-diff {
    border: 1px solid #e5e7eb;
    border-radius: 6px;
    overflow: hidden;
    margin-bottom: 0.75rem;
  }

  .diff-columns {
    display: grid;
    grid-template-columns: 1fr 1fr;
  }

  .diff-column {
    min-width: 0;
  }

  .snapshot-column {
    border-right: 1px solid #e5e7eb;
  }

  .diff-column-header {
    padding: 0.4rem 0.75rem;
    font-size: 0.7rem;
    font-weight: 600;
    color: #6b7280;
    text-transform: uppercase;
    letter-spacing: 0.03em;
    border-bottom: 1px solid #e5e7eb;
  }

  .snapshot-column .diff-column-header {
    background: #fffbeb;
  }

  .current-column .diff-column-header {
    background: #f0fdf4;
  }

  .diff-column-body {
    padding: 0.75rem;
    font-size: 0.8rem;
    line-height: 1.5;
    color: #374151;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .snapshot-column .diff-column-body {
    background: #fffef5;
  }

  .current-column .diff-column-body {
    background: #f7fef9;
  }
</style>
```

**Register in component index:**

```typescript
// packages/webui/src/lib/components/index.ts
export { default as DriftBanner } from './DriftBanner.svelte'
```

**Usage in Bullets view (Task 14.3) — include DriftBanner in bullet detail:**

```svelte
<!-- Inside bullet card, after content -->
{#if contentType === 'bullet' && item.sources?.length > 0}
  {@const primarySource = item.sources.find(s => s.is_primary)}
  {#if primarySource && item.source_content_snapshot}
    <DriftBanner
      snapshotContent={item.source_content_snapshot}
      currentContent={primarySource.description ?? ''}
      entityType="bullet"
      onRederive={() => rederiveBullet(item.id)}
    />
  {/if}
{/if}
```

**Note:** Full drift data (the source's current description) may require an additional API call or embedding in the bullet response. The exact approach depends on how the API returns bullet data in Phase 12. If the sources array only contains `{id, title, is_primary}`, we need either:
- A separate `forge.integrity.drift()` call to get drifted entities, or
- An expanded bullet response that includes `source_description` for the primary source

For MVP, use the integrity drift endpoint to identify drifted bullets and show the banner only on those.

**Acceptance Criteria:**
- [ ] Banner shows only when `snapshotContent !== currentContent`
- [ ] Banner hidden when content matches (no false positives)
- [ ] "View diff" toggles side-by-side comparison panel
- [ ] "Re-derive" button calls `onRederive` callback
- [ ] "Dismiss" button calls `onDismiss` callback
- [ ] Component exported from `$lib/components/index.ts`
- [ ] Banner used in Bullets view for drifted bullets
- [ ] Banner used in perspective detail for drifted perspectives

**Testing:**
- Component: Banner renders when snapshot differs from current
- Component: Banner hidden when snapshot matches current
- Component: Diff panel toggles on button click
- Component: Callbacks fire on Re-derive and Dismiss clicks

---

## Task 15.4: E2E Tests for Schema Evolution

**Goal:** Write end-to-end tests covering the new schema features and data flows.

**File:** `packages/core/src/__tests__/e2e/schema-evolution.test.ts` (or `tests/e2e/`)

**Test scenarios:**

### E2E-SE1: Full Derivation Chain with New Schema

```typescript
import { describe, it, expect, beforeAll, afterAll } from 'bun:test'
// Test setup: fresh database with 002 migration applied

describe('E2E: Schema Evolution — Full Chain', () => {
  it('creates org, role source, derives bullets, approves, derives perspectives, adds to resume as entries', async () => {
    // 1. Create organization
    const org = await api.post('/api/organizations', {
      name: 'Acme Corp',
      org_type: 'company',
      industry: 'Technology',
      worked: true,
      employment_type: 'civilian',
    })
    expect(org.status).toBe(201)
    expect(org.body.name).toBe('Acme Corp')

    // 2. Create role source linked to organization
    const source = await api.post('/api/sources', {
      title: 'Senior Engineer',
      description: 'Led cloud migration for 50+ services...',
      source_type: 'role',
      role: {
        organization_id: org.body.id,
        start_date: '2023-01-15',
        end_date: '2024-06-30',
        is_current: false,
        work_arrangement: 'remote',
      },
    })
    expect(source.status).toBe(201)
    expect(source.body.source_type).toBe('role')
    expect(source.body.role.organization_id).toBe(org.body.id)

    // 3. Verify source GET returns extension data
    const sourceGet = await api.get(`/api/sources/${source.body.id}`)
    expect(sourceGet.body.role).toBeDefined()
    expect(sourceGet.body.role.start_date).toBe('2023-01-15')

    // 4. Derive bullets from source
    const derivation = await api.post(`/api/sources/${source.body.id}/derive-bullets`)
    expect(derivation.status).toBe(200)
    const bulletIds = derivation.body.map((b: any) => b.id)
    expect(bulletIds.length).toBeGreaterThan(0)

    // 5. Verify bullets have sources array (not source_id)
    const bulletGet = await api.get(`/api/bullets/${bulletIds[0]}`)
    expect(bulletGet.body.source_id).toBeUndefined()
    expect(bulletGet.body.sources).toBeInstanceOf(Array)
    expect(bulletGet.body.sources[0].is_primary).toBe(true)
    expect(bulletGet.body.sources[0].id).toBe(source.body.id)

    // 6. Approve first bullet
    const approved = await api.post(`/api/bullets/${bulletIds[0]}/approve`)
    expect(approved.body.status).toBe('approved')

    // 7. Derive perspective from approved bullet
    const perspective = await api.post(`/api/bullets/${bulletIds[0]}/derive-perspectives`, {
      archetype: 'infrastructure',
      domain: 'systems_engineering',
      framing: 'accomplishment',
    })
    expect(perspective.status).toBe(200)
    const perspId = perspective.body[0]?.id ?? perspective.body.id

    // 8. Approve perspective
    await api.post(`/api/perspectives/${perspId}/approve`)

    // 9. Create resume
    const resume = await api.post('/api/resumes', {
      name: 'Acme - Senior Engineer',
      target_role: 'Senior Engineer',
      target_employer: 'Acme Corp',
      archetype: 'infrastructure',
    })
    expect(resume.status).toBe(201)

    // 10. Add perspective as resume entry
    const entry = await api.post(`/api/resumes/${resume.body.id}/entries`, {
      perspective_id: perspId,
      section: 'work_history',
      position: 0,
    })
    expect(entry.status).toBe(201)
    expect(entry.body.content).toBeNull()  // reference mode
    expect(entry.body.perspective_id).toBe(perspId)

    // 11. Verify resume GET returns entries
    const resumeDetail = await api.get(`/api/resumes/${resume.body.id}`)
    expect(resumeDetail.body.sections.work_history).toHaveLength(1)
    expect(resumeDetail.body.sections.work_history[0].perspective_id).toBe(perspId)
  })
})
```

### E2E-SE2: Content Drift Detection with Junction Model

```typescript
describe('E2E: Schema Evolution — Content Drift', () => {
  it('detects drift when source content changes after bullet derivation', async () => {
    // 1. Create source and derive bullets
    const source = await api.post('/api/sources', {
      title: 'Drift Test Source',
      description: 'Original description of the project.',
      source_type: 'general',
    })
    const derivation = await api.post(`/api/sources/${source.body.id}/derive-bullets`)
    const bulletId = derivation.body[0].id

    // 2. Verify snapshot matches
    const bullet = await api.get(`/api/bullets/${bulletId}`)
    expect(bullet.body.source_content_snapshot).toBe('Original description of the project.')

    // 3. Update source description
    await api.patch(`/api/sources/${source.body.id}`, {
      description: 'Updated description with new details.',
    })

    // 4. Check integrity drift endpoint
    const drift = await api.get('/api/integrity/drift')
    expect(drift.body.some((d: any) => d.entity_id === bulletId && d.entity_type === 'bullet')).toBe(true)

    // 5. Verify chain integrity shows divergence
    const chain = await api.get(`/api/perspectives/${perspId}/chain`)
    expect(chain.body.source_snapshot_matches).toBe(false)
  })
})
```

### E2E-SE3: Import v1 Data + Chain Integrity

```typescript
describe('E2E: Schema Evolution — v1 Import', () => {
  it('imports v1 data and verifies chain integrity', async () => {
    // Requires v1 test fixture database at known path
    // 1. Run import
    const importResult = await cli('forge import v1 fixtures/v1-test.db')
    expect(importResult.exitCode).toBe(0)

    // 2. Verify organization count
    const orgs = await api.get('/api/organizations')
    expect(orgs.body.length).toBeGreaterThanOrEqual(18)

    // 3. Verify source count (roles + education + clearance)
    const sources = await api.get('/api/sources?limit=500')
    expect(sources.body.length).toBeGreaterThanOrEqual(36) // 13 roles + 22 education + 1 clearance

    // 4. Verify bullets imported with approved status
    const bullets = await api.get('/api/bullets?limit=500')
    const approvedBullets = bullets.body.filter((b: any) => b.status === 'approved')
    expect(approvedBullets.length).toBeGreaterThanOrEqual(73)

    // 5. Verify bullet_sources junction populated
    for (const bullet of bullets.body.slice(0, 5)) {
      expect(bullet.sources).toBeDefined()
      expect(bullet.sources.length).toBeGreaterThan(0)
    }

    // 6. Verify skills imported
    const skills = await api.get('/api/skills?limit=500')
    expect(skills.body.length).toBeGreaterThanOrEqual(62) // 60 skills + 2 languages

    // 7. Verify languages imported as skills
    const languageSkills = skills.body.filter((s: any) => s.category === 'language')
    expect(languageSkills.length).toBe(2)

    // 8. Verify resumes imported with entries
    const resumes = await api.get('/api/resumes')
    expect(resumes.body.length).toBeGreaterThanOrEqual(3)

    // 9. Re-run import is idempotent
    const reImport = await cli('forge import v1 fixtures/v1-test.db')
    expect(reImport.exitCode).toBe(0)
    const orgsAfter = await api.get('/api/organizations')
    expect(orgsAfter.body.length).toBe(orgs.body.length) // no duplicates
  })
})
```

### E2E-SE4: Copy-on-Write Resume Entries

```typescript
describe('E2E: Schema Evolution — Copy-on-Write', () => {
  it('supports reference mode, clone mode, and reset', async () => {
    // Setup: create perspective and resume with entry
    // ... (create source, derive bullet, approve, derive perspective, approve, create resume) ...

    // 1. Add entry in reference mode
    const entry = await api.post(`/api/resumes/${resumeId}/entries`, {
      perspective_id: perspId,
      section: 'work_history',
      position: 0,
    })
    expect(entry.body.content).toBeNull() // reference mode

    // 2. Clone: edit entry content
    const cloned = await api.patch(`/api/resumes/${resumeId}/entries/${entry.body.id}`, {
      content: 'Customized bullet text for this specific resume.',
    })
    expect(cloned.body.content).toBe('Customized bullet text for this specific resume.')
    expect(cloned.body.perspective_content_snapshot).toBeDefined() // snapshot captured

    // 3. Reset to reference mode
    const reset = await api.patch(`/api/resumes/${resumeId}/entries/${entry.body.id}`, {
      content: null,
    })
    expect(reset.body.content).toBeNull() // back to reference mode
  })
})
```

### E2E-SE5: Notes with Entity Linking

```typescript
describe('E2E: Schema Evolution — User Notes', () => {
  it('creates notes with entity links and supports two-way search', async () => {
    // 1. Create a note
    const note = await api.post('/api/notes', {
      title: 'Interview prep notes',
      content: 'Remember to emphasize the Kubernetes migration project.',
    })
    expect(note.status).toBe(201)

    // 2. Link to a source
    await api.post(`/api/notes/${note.body.id}/references`, {
      entity_type: 'source',
      entity_id: sourceId,
    })

    // 3. Verify note has reference
    const noteDetail = await api.get(`/api/notes/${note.body.id}`)
    expect(noteDetail.body.references).toHaveLength(1)
    expect(noteDetail.body.references[0].entity_type).toBe('source')

    // 4. Search notes by content
    const searchResult = await api.get('/api/notes?search=Kubernetes')
    expect(searchResult.body.some((n: any) => n.id === note.body.id)).toBe(true)

    // 5. Unlink entity
    await api.delete(`/api/notes/${note.body.id}/references/source/${sourceId}`)
    const afterUnlink = await api.get(`/api/notes/${note.body.id}`)
    expect(afterUnlink.body.references).toHaveLength(0)
  })
})
```

**Acceptance Criteria:**
- [ ] E2E-SE1: Full chain with new schema (org -> role source -> bullets -> perspectives -> resume entries)
- [ ] E2E-SE2: Content drift detection after source edit
- [ ] E2E-SE3: v1 import with idempotency verification
- [ ] E2E-SE4: Copy-on-write reference/clone/reset cycle
- [ ] E2E-SE5: Notes with entity linking and search
- [ ] All tests use fresh database per scenario
- [ ] All tests clean up after themselves

**Testing:**
- All scenarios are themselves tests. Run with `bun test --filter e2e`.

---

## Task 15.5: Contract Tests Update

**Goal:** Update contract tests to verify new API response shapes match SDK types.

**File:** `packages/core/src/routes/__tests__/contracts.test.ts`

**New contract test cases:**

```typescript
describe('Contract Tests: Schema Evolution', () => {
  describe('Source with extension data', () => {
    it('GET /api/sources/:id returns source_type and role extension', async () => {
      // Create role source, then GET
      const res = await api.get(`/api/sources/${roleSourceId}`)
      expect(res.status).toBe(200)

      // Verify shape matches SDK Source type
      expect(res.body).toHaveProperty('id')
      expect(res.body).toHaveProperty('title')
      expect(res.body).toHaveProperty('description')
      expect(res.body).toHaveProperty('source_type', 'role')
      expect(res.body).toHaveProperty('status')
      expect(res.body).toHaveProperty('notes')
      expect(res.body).toHaveProperty('role')
      expect(res.body.role).toHaveProperty('organization_id')
      expect(res.body.role).toHaveProperty('start_date')
      expect(res.body.role).toHaveProperty('end_date')
      expect(res.body.role).toHaveProperty('is_current')

      // Verify old fields removed
      expect(res.body).not.toHaveProperty('employer_id')
      expect(res.body).not.toHaveProperty('project_id')
    })

    it('GET /api/sources/:id returns education extension for education source', async () => {
      const res = await api.get(`/api/sources/${eduSourceId}`)
      expect(res.body.source_type).toBe('education')
      expect(res.body.education).toHaveProperty('education_type')
      expect(res.body.education).toHaveProperty('institution')
      expect(res.body.education).toHaveProperty('field')
    })
  })

  describe('Bullet with sources array', () => {
    it('GET /api/bullets/:id returns sources array instead of source_id', async () => {
      const res = await api.get(`/api/bullets/${bulletId}`)
      expect(res.status).toBe(200)

      // Verify new shape
      expect(res.body).not.toHaveProperty('source_id')
      expect(res.body).toHaveProperty('sources')
      expect(Array.isArray(res.body.sources)).toBe(true)
      expect(res.body.sources[0]).toHaveProperty('id')
      expect(res.body.sources[0]).toHaveProperty('title')
      expect(res.body.sources[0]).toHaveProperty('is_primary')
      expect(res.body).toHaveProperty('domain')
      expect(res.body).toHaveProperty('notes')
    })
  })

  describe('Resume with entries', () => {
    it('GET /api/resumes/:id returns sections with ResumeEntry objects', async () => {
      const res = await api.get(`/api/resumes/${resumeId}`)
      expect(res.status).toBe(200)

      expect(res.body).toHaveProperty('sections')
      const section = res.body.sections.work_history?.[0]
      if (section) {
        expect(section).toHaveProperty('id')
        expect(section).toHaveProperty('resume_id')
        expect(section).toHaveProperty('perspective_id')
        expect(section).toHaveProperty('content') // null or string
        expect(section).toHaveProperty('perspective_content_snapshot')
        expect(section).toHaveProperty('section')
        expect(section).toHaveProperty('position')
        expect(section).toHaveProperty('notes')
      }
    })
  })

  describe('Organization', () => {
    it('GET /api/organizations/:id returns full organization shape', async () => {
      const res = await api.get(`/api/organizations/${orgId}`)
      expect(res.status).toBe(200)

      expect(res.body).toHaveProperty('id')
      expect(res.body).toHaveProperty('name')
      expect(res.body).toHaveProperty('org_type')
      expect(res.body).toHaveProperty('industry')
      expect(res.body).toHaveProperty('worked')
      expect(res.body).toHaveProperty('employment_type')
      expect(res.body).toHaveProperty('location')
      expect(res.body).toHaveProperty('notes')
    })
  })

  describe('User Note', () => {
    it('GET /api/notes/:id returns note with references array', async () => {
      const res = await api.get(`/api/notes/${noteId}`)
      expect(res.status).toBe(200)

      expect(res.body).toHaveProperty('id')
      expect(res.body).toHaveProperty('title')
      expect(res.body).toHaveProperty('content')
      expect(res.body).toHaveProperty('references')
      expect(Array.isArray(res.body.references)).toBe(true)
    })
  })
})
```

**Acceptance Criteria:**
- [ ] Contract test for source with role extension
- [ ] Contract test for source with education extension
- [ ] Contract test for bullet with sources array (no source_id)
- [ ] Contract test for resume with entries (not perspectives)
- [ ] Contract test for organization shape
- [ ] Contract test for user note with references
- [ ] All contract tests pass against running API

**Testing:**
- All cases are themselves tests. Run with `bun test --filter contracts`.

---

## Task 15.6: Acceptance Criteria Walkthrough

**Goal:** Systematically verify every acceptance criterion from the schema evolution spec.

**File:** Create/update `docs/src/mvp/acceptance-report.md` (or `docs/src/mvp/acceptance-report-v2.md`)

**Steps:**

1. Extract all acceptance criteria from `docs/superpowers/specs/2026-03-29-forge-schema-evolution-design.md` section 8
2. For each criterion, verify by:
   - Running a command or test
   - Inspecting the database schema
   - Making an API call
   - Checking the UI
3. Mark each as PASS, FAIL, or DEFERRED with explanation
4. Document any deviations from the spec

**Criteria categories to verify:**
- Schema (20 items): tables, columns, constraints, migrations
- Data Migration (17 items): all entity types imported correctly
- Services (8 items): DerivationService, AuditService, ResumeService, etc.
- API (9 items): source CRUD, bullet response, organization CRUD, etc.
- SDK (5 items): types, resources
- UI (12 items): tabbed sources, unified bullets, graph, etc.
- Tests (9 items): round-trip, many-to-many, copy-on-write, etc.

**Report format:**

```markdown
# Schema Evolution — Acceptance Report

**Date:** [date]
**Spec:** 2026-03-29-forge-schema-evolution-design.md
**Status:** [X of Y criteria passed]

## Schema

| # | Criterion | Status | Notes |
|---|-----------|--------|-------|
| S1 | organizations table with UUID PKs, STRICT | PASS | Verified via .schema |
| S2 | employers table dropped | PASS | |
| ... | ... | ... | ... |

## Data Migration

| # | Criterion | Status | Notes |
|---|-----------|--------|-------|
| D1 | 18 organizations imported | PASS | SELECT COUNT(*) = 18 |
| ... | ... | ... | ... |

...
```

**Acceptance Criteria:**
- [ ] Every acceptance criterion from spec section 8 is addressed
- [ ] Report has PASS/FAIL/DEFERRED for each item
- [ ] Any DEFERRED items have clear rationale
- [ ] Report committed to docs

**Testing:**
- This task is itself a validation exercise, not a code change.

---

## Task 15.7: Documentation Updates

**Goal:** Update documentation to reflect the schema evolution changes.

**Files to update:**
- `docs/src/data/models/entity-types.md` — add organizations, user_notes, note_references, resume_entries, source extensions
- `docs/src/data/models/schema.md` — update ERD or table descriptions for polymorphic sources, bullet_sources junction, resume_entries
- `docs/src/api/routes.md` — add organization routes, note routes, resume entry routes, integrity drift endpoint; update source/bullet response shapes
- `docs/src/sdk/client.md` — add OrganizationsResource, NotesResource, updated ResumesResource
- `docs/src/webui/views.md` — rewrite with new view descriptions (tabbed sources, unified bullets, organizations, skills, archetypes, notes, logs, chain graph)
- `docs/src/webui/components.md` — add DriftBanner component

**New documentation:**
- Migration guide section: "Upgrading from v1" — document the `forge import v1` command, what gets imported, what doesn't, idempotency guarantees
- Schema evolution overview: summarize what changed and why (polymorphic sources, junction-only bullets, resume entries, copy-on-write)

**Acceptance Criteria:**
- [ ] Entity types doc covers all new tables and their relationships
- [ ] API routes doc lists all new/modified/removed endpoints
- [ ] SDK client doc covers new resource classes and methods
- [ ] WebUI views doc describes all 10 navigation items
- [ ] DriftBanner component documented with props and usage
- [ ] Migration guide explains v1 import process
- [ ] No stale references to `employer_id`, `source_id` on bullets, `resume_perspectives`

**Testing:**
- Review: All doc files reference current schema/API, not deprecated structures

---

## Parallelization

```
Task 15.1 (install sigma) ──> Task 15.2 (chain graph)
                                          |
Task 15.3 (drift banner) ────────────────┤ (independent of graph)
                                          |
Task 15.4 (E2E tests) ──────────────────┤ (independent, can start immediately)
Task 15.5 (contract tests) ─────────────┤
                                          |
Task 15.6 (acceptance walkthrough) ──────┤ (after 15.4 + 15.5 pass)
Task 15.7 (docs) ───────────────────────┘ (independent, can start immediately)
```

**Dependency chain:**
- T15.1 must complete before T15.2 (graph depends on installed libraries)
- T15.6 should run after T15.4 and T15.5 (walkthrough validates test results)
- Everything else is independent

**Priority order:**
1. T15.1 + T15.3 (quick, unblock other work)
2. T15.4 + T15.5 (tests validate everything)
3. T15.2 (graph visualization — largest task)
4. T15.6 (acceptance walkthrough)
5. T15.7 (docs — can be done last)
