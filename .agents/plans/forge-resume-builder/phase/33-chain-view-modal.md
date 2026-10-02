# Phase 33: Chain View Modal

**Status:** Planning
**Date:** 2026-03-31
**Spec:** [2026-03-30-chain-view-modal.md](../refs/specs/2026-03-30-chain-view-modal.md)
**Depends on:** Phase 32 (Nav Restructuring -- layout must be merged first)
**Blocks:** Nothing
**Parallelizable with:** Phases 29-31, 34-36 (after Phase 32)

## Goal

Convert the Chain View from a full-page route (`/chain`) to a reusable component that can render as either a full-screen modal overlay or inline page content. Create a global store to control modal visibility and highlight target, mount the modal at the layout level so it is accessible from any page, and replace the provenance `<a href>` links in `DragNDropView.svelte` with `onclick` handlers that open the modal without navigating away from the resume builder.

## Non-Goals

- Persistent Sigma.js instance across navigations (too much memory/WebGL state)
- Mini/embedded chain view within the resume builder (this is always full-screen overlay)
- Editing entities from within the modal (read-only graph inspection)
- Focus trapping within the modal (not implemented in this phase)

## Context

The Chain View is currently a standalone page at `/chain` (~1,160 lines in `+page.svelte`). The resume builder's provenance tooltips in `DragNDropView.svelte` link to `/chain?highlight=source-{id}`, which navigates away from the resume page and destroys all in-progress editing state. This phase extracts the chain view into a reusable component, adds a store for open/close state, and mounts the component conditionally in the root layout. The `/chain` route becomes a thin wrapper around the same component in inline mode.

The store uses a **getter-object pattern** (module-level `$state` runes with exported mutation functions, and a `chainViewState` object whose properties are getters). This is distinct from the toast store (`toast.svelte.ts`), which exposes raw functions (`addToast`, `getToasts`). The getter-object pattern allows consumers to access `.isOpen` and `.highlightNode` as reactive values without calling functions.

The file extension must be `.svelte.ts` because the store uses `$state` runes.

## Scope

| Spec section | Covered here? |
|-------------|---------------|
| 1. Store (`chain-view.svelte.ts`) | Yes |
| 2. Component (`ChainViewModal.svelte`) | Yes |
| 3. Layout mount | Yes |
| 4. DragNDropView link updates | Yes |
| 5. Standalone `/chain` route refactor | Yes |
| 6. Modal styling | Yes |

## Files to Create

| File | Description |
|------|-------------|
| `packages/webui/src/lib/stores/chain-view.svelte.ts` | Global store for modal open/close state and highlight target |
| `packages/webui/src/lib/components/ChainViewModal.svelte` | Extracted chain view component (modal + inline modes) |

## Files to Modify

| File | Change |
|------|--------|
| `packages/webui/src/routes/chain/+page.svelte` | Replace ~1,160 lines with ~20-line thin wrapper |
| `packages/webui/src/routes/+layout.svelte` | Import + conditionally mount `ChainViewModal` |
| `packages/webui/src/lib/components/resume/DragNDropView.svelte` | Replace 3 `<a href="/chain?highlight=...">` with `<button onclick={openChainView(...)}>`; update `.tooltip-link` styles |

## Fallback Strategies

| Risk | Fallback |
|------|----------|
| Sigma WebGL context leaks after repeated open/close cycles | The `{#if chainViewState.isOpen}` conditional mount destroys the entire component tree on close; Sigma `kill()` is called in both `$effect` return and `onDestroy`. If leaks persist, add a manual WebGL context loss trigger via `canvas.getContext('webgl')?.getExtension('WEBGL_lose_context')?.loseContext()` |
| Body scroll lock not restored after unexpected unmount | The `$effect` cleanup returns `document.body.style.overflow = ''`. If the component is removed without cleanup (e.g., SvelteKit navigation), add a global `beforeNavigate` guard that resets overflow |
| Phase 32 layout structure differs from expected | The modal mount is a single `{#if}` block appended after `<ToastContainer />` -- minimal coupling to layout structure. Adjust mount point as needed |
| `page` import from `$app/state` not available in component context | Fall back to `$app/stores` page store (`$page.url.searchParams`). The spec prefers `$app/state` for Svelte 5 but both work |

---

## Tasks

### T33.1: Create `chain-view.svelte.ts` Store

**File to create:** `packages/webui/src/lib/stores/chain-view.svelte.ts`

**Goal:** Global reactive store for chain view modal state using the getter-object pattern.

```typescript
// packages/webui/src/lib/stores/chain-view.svelte.ts

let isOpen = $state(false)
let highlightNode = $state<string | null>(null)

/** Open the chain view modal, optionally highlighting a specific graph node. */
export function openChainView(nodeKey?: string) {
  highlightNode = nodeKey ?? null
  isOpen = true
}

/** Close the chain view modal and clear the highlight target. */
export function closeChainView() {
  isOpen = false
  highlightNode = null
}

/**
 * Reactive state object for chain view modal.
 * Properties are getters that read $state runes reactively.
 *
 * Usage:
 *   import { chainViewState } from '$lib/stores/chain-view.svelte'
 *   // In template: {#if chainViewState.isOpen}
 *   // In script: chainViewState.highlightNode
 */
export const chainViewState = {
  get isOpen() { return isOpen },
  get highlightNode() { return highlightNode },
}
```

**Key design decisions:**
- `.svelte.ts` extension is required because the file uses `$state` runes.
- Getter-object pattern (NOT the function-export pattern used by `toast.svelte.ts`). The toast store exposes `getToasts()` as a function; this store exposes `chainViewState.isOpen` as a reactive getter. The difference matters because the layout template needs to reactively track `isOpen` in an `{#if}` block -- a getter on an imported object is reactive in Svelte 5 rune mode, while a bare function call is not.
- `closeChainView` nulls out `highlightNode` to prevent stale highlight state on next open.

**Acceptance criteria:**
- [ ] `openChainView()` sets `isOpen` to `true`, `highlightNode` to `null`
- [ ] `openChainView('source-abc')` sets `isOpen` to `true`, `highlightNode` to `'source-abc'`
- [ ] `closeChainView()` sets `isOpen` to `false`, `highlightNode` to `null`
- [ ] `chainViewState.isOpen` and `chainViewState.highlightNode` return current values reactively
- [ ] File extension is `.svelte.ts`

**Failure criteria:**
- File uses `.ts` extension (runes will not compile)
- Store uses a class or `$state.frozen` instead of module-level `$state`
- `chainViewState` properties are plain values instead of getters (breaks reactivity)

---

### T33.2: Create `ChainViewModal.svelte` Component

**File to create:** `packages/webui/src/lib/components/ChainViewModal.svelte`

**Goal:** Extract all chain view logic from `/chain/+page.svelte` into a reusable component that supports both modal and inline rendering modes.

#### T33.2.1: Props interface

```typescript
let {
  highlightNode = null,
  isModal = true,
  onClose,
}: {
  highlightNode?: string | null
  isModal?: boolean
  onClose?: () => void
} = $props()
```

- `highlightNode`: graph node key to highlight on open (e.g., `"source-550e8400-..."`)
- `isModal`: when `true`, renders with modal chrome (backdrop, close button, escape handler). When `false`, renders inline (for `/chain` route).
- `onClose`: callback invoked when the modal is dismissed.

#### T33.2.2: Script block -- extracted from `+page.svelte`

Move the entire `<script>` block from the current `+page.svelte` (lines 1-474) into the component, with these modifications:

**Imports -- add:**
```typescript
import { page } from '$app/state'
import { onDestroy } from 'svelte'
import { closeChainView } from '$lib/stores/chain-view.svelte'
```

**Close handler:**
```typescript
function close() {
  if (onClose) onClose()
  else closeChainView()
}
```

**Keydown handler (Escape gated on `isModal`):**
```typescript
function handleKeydown(e: KeyboardEvent) {
  if (isModal && e.key === 'Escape') close()
}
```

**Backdrop click handler:**
```typescript
function handleBackdropClick() {
  close()
}
```

**Highlight handling -- replace `window.location.search` (line 390) with unified path:**

Replace the current highlight logic (lines 387-416) with:

```typescript
// After Sigma init completes and sigmaInstance is assigned:
const nodeToHighlight = isModal ? highlightNode : page.url.searchParams.get('highlight')
if (nodeToHighlight && graph!.hasNode(nodeToHighlight)) {
  // Enlarge and mark the highlighted node
  const attrs = graph!.getNodeAttributes(nodeToHighlight)
  graph!.setNodeAttribute(nodeToHighlight, 'size', (attrs.size ?? 8) * 2)
  graph!.setNodeAttribute(nodeToHighlight, 'highlighted', true)

  // Select and center via existing helper
  selectNodeInGraph(nodeToHighlight)
}
```

Note: `selectNodeInGraph` already handles setting `selectedNodeId`, `selectedNodeData`, clearing `selectedEdgeData`, and animating the camera. The highlight logic above adds the size enlargement and `highlighted` attribute that `selectNodeInGraph` does not do. The order matters: set attributes first, then call `selectNodeInGraph` (which reads attributes and animates camera).

**UUID round-trip example:** source_id `550e8400-e29b-41d4-a716-446655440000` -> node key `source-550e8400-e29b-41d4-a716-446655440000` -> `openChainView('source-550e8400-e29b-41d4-a716-446655440000')` -> `graph.hasNode('source-550e8400-e29b-41d4-a716-446655440000')` returns true. Do NOT split on '-' -- UUIDs contain hyphens.

#### T33.2.3: Sigma lifecycle -- dual cleanup

**Retry button lifecycle:** The Retry button calls `buildGraph()` directly (not via `$effect`). `buildGraph()` updates the `graph` state variable, which triggers the Sigma `$effect` to re-run. The full lifecycle is: Retry click -> reset `sigmaInstance` to null -> `buildGraph()` -> `graph` state updates -> `$effect` re-runs -> new Sigma instance created. The `$effect` return cleanup fires when the effect re-runs, killing the old Sigma instance (if any).

**Critical: Retry must reset `sigmaInstance` before calling `buildGraph()`.** The `$effect` guard (`if (sigmaInstance) return`) will prevent re-initialization if `sigmaInstance` is still set. The Retry handler must include: `sigmaInstance?.kill(); sigmaInstance = null;` before `buildGraph()`. Without this, clicking Retry after a previous successful render will silently do nothing.

**Primary cleanup via `$effect` return:**

The existing `$effect` for Sigma init (lines 257-428) already returns a cleanup function. Keep this. It handles cleanup on re-runs (e.g., when the Retry button calls `buildGraph()` again):

```typescript
// Initialize Sigma when container and graph are ready
$effect(() => {
  if (!browser || !container || !graph || sigmaInstance) return

  async function initSigma() {
    // ... existing Sigma initialization code ...

    sigmaInstance = instance

    // Highlight handling (unified modal/standalone)
    const nodeToHighlight = isModal ? highlightNode : page.url.searchParams.get('highlight')
    if (nodeToHighlight && graph!.hasNode(nodeToHighlight)) {
      const attrs = graph!.getNodeAttributes(nodeToHighlight)
      graph!.setNodeAttribute(nodeToHighlight, 'size', (attrs.size ?? 8) * 2)
      graph!.setNodeAttribute(nodeToHighlight, 'highlighted', true)
      selectNodeInGraph(nodeToHighlight)
    }
  }

  initSigma()

  // Primary cleanup: handles re-runs (Retry button)
  return () => {
    if (sigmaInstance) {
      sigmaInstance.kill()
      sigmaInstance = null
    }
  }
})
```

**Backup cleanup via `onDestroy`:**

```typescript
// Belt-and-suspenders: onDestroy handles final component teardown
onDestroy(() => {
  if (sigmaInstance) {
    sigmaInstance.kill()
    sigmaInstance = null
  }
})
```

#### T33.2.4: Body scroll lock

`isModal` is treated as an immutable prop -- it must not change after mount. The body scroll lock `$effect` assumes `isModal` is constant for the component's lifetime.

```typescript
$effect(() => {
  if (isModal) {
    document.body.style.overflow = 'hidden'
    return () => { document.body.style.overflow = '' }
  }
})
```

#### T33.2.5: Template -- modal chrome wrapping

The template wraps the graph content in modal chrome when `isModal === true`.

**Focus for keyboard events:** The backdrop div has `tabindex="-1"` but must receive focus for keyboard events to work. Add a mount effect that focuses the backdrop: `$effect(() => { if (isModal) backdropRef?.focus() })` using `bind:this={backdropRef}`. Without this, Escape key dismissal will not work because the backdrop never receives focus and the `onkeydown` handler will never fire.

**`aria-labelledby` scope:** The `id='chain-modal-title'` exists only inside the `{#if isModal}` branch. In standalone mode (`isModal={false}`), no element has this ID. The `aria-labelledby` reference is scoped to modal chrome only -- do not add this ID to the standalone page's heading.

**DO NOT implement this duplicate version** -- see the snippet refactor below for the actual implementation. This expanded form is shown only to illustrate the full markup structure before deduplication:

```svelte
{#if isModal}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div
    class="chain-modal-backdrop"
    bind:this={backdropRef}
    onclick={handleBackdropClick}
    onkeydown={handleKeydown}
    tabindex="-1"
  >
    <div
      class="chain-modal-content"
      role="dialog"
      aria-modal="true"
      aria-labelledby="chain-modal-title"
      onclick={(e) => e.stopPropagation()}
    >
      <div class="chain-modal-header">
        <h2 id="chain-modal-title">Chain View</h2>
        <button class="modal-close-btn" onclick={close}>Close</button>
      </div>

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
          <option value="draft">Draft</option>
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
        <div class="legend">
          <span class="legend-item"><span class="legend-dot" style:background={NODE_COLORS.source}></span> Source</span>
          <span class="legend-item"><span class="legend-dot" style:background={NODE_COLORS.bullet}></span> Bullet</span>
          <span class="legend-item"><span class="legend-dot" style:background={NODE_COLORS.perspective}></span> Perspective</span>
          <span class="legend-item"><span class="legend-dot" style:background={NODE_COLORS.resume_entry}></span> Entry</span>
          <span class="legend-divider">|</span>
          <span class="legend-item"><span class="legend-line" style:background="#94a3b8"></span> Matching</span>
          <span class="legend-item"><span class="legend-line" style:background="#ef4444"></span> Drifted</span>
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
      {:else if nodeCount === 0}
        <EmptyState
          title="No chain data"
          description="Create sources and derive bullets to see the provenance graph."
        />
      {:else}
        <div class="graph-layout">
          <div class="graph-container" bind:this={container}></div>

          {#if selectedNodeData}
            <div class="detail-panel">
              <!-- ... existing detail panel markup (lines 558-674 of current +page.svelte) ... -->
              <div class="detail-header">
                <span class="node-type-icon" style:background={NODE_COLORS[selectedNodeData.type]}>
                  {selectedNodeData.type[0].toUpperCase()}
                </span>
                <h3 class="detail-title">{selectedNodeData.label}</h3>
                <button class="close-btn" onclick={() => { selectedNodeId = null; selectedNodeData = null }}>
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
              </div>
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
                  <span class="edge-detail-icon" style:background={NODE_COLORS[String(selectedEdgeData.sourceType)]}>
                    {String(selectedEdgeData.sourceType).charAt(0).toUpperCase()}
                  </span>
                  <div>
                    <strong>{selectedEdgeData.sourceLabel}</strong>
                    <StatusBadge status={String(selectedEdgeData.sourceStatus)} />
                  </div>
                </div>
                <div class="edge-detail-arrow">&rarr;</div>
                <div class="edge-detail-row">
                  <span class="edge-detail-icon" style:background={NODE_COLORS[String(selectedEdgeData.targetType)]}>
                    {String(selectedEdgeData.targetType).charAt(0).toUpperCase()}
                  </span>
                  <div>
                    <strong>{selectedEdgeData.targetLabel}</strong>
                    <StatusBadge status={String(selectedEdgeData.targetStatus)} />
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
        </div>

        {#if edgeTooltip}
          <div class="edge-tooltip" style="left: {edgeTooltipPosition.x}px; top: {edgeTooltipPosition.y}px;">
            <div class="edge-tooltip-row">
              <span class="edge-node-type" style:background={NODE_COLORS[String(edgeTooltip.sourceType)]}>
                {String(edgeTooltip.sourceType).charAt(0).toUpperCase()}
              </span>
              <span class="edge-tooltip-label">{truncate(String(edgeTooltip.sourceLabel), 40)}</span>
              <span class="edge-arrow">&rarr;</span>
              <span class="edge-node-type" style:background={NODE_COLORS[String(edgeTooltip.targetType)]}>
                {String(edgeTooltip.targetType).charAt(0).toUpperCase()}
              </span>
              <span class="edge-tooltip-label">{truncate(String(edgeTooltip.targetLabel), 40)}</span>
            </div>
            <div class="edge-tooltip-meta">
              {#if edgeTooltip.isPrimary}<span class="primary-badge">Primary</span>{/if}
              {#if edgeTooltip.drifted}<span class="drift-badge">Snapshot drifted</span>
              {:else}<span class="match-badge">Snapshot matches</span>{/if}
            </div>
          </div>
        {/if}
      {/if}
    </div>
  </div>
{:else}
  <!-- Inline mode: same content without backdrop/modal chrome -->
  <div class="controls">
    <!-- ... identical controls markup ... -->
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
      <option value="draft">Draft</option>
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

  <div class="stats-bar">
    <!-- ... identical stats markup ... -->
    <span class="stat"><strong>{nodeCount}</strong> nodes</span>
    <span class="stat"><strong>{edgeCount}</strong> edges</span>
    {#if driftedCount > 0}
      <span class="stat drift-stat"><strong>{driftedCount}</strong> drifted</span>
    {/if}
    <div class="legend">
      <span class="legend-item"><span class="legend-dot" style:background={NODE_COLORS.source}></span> Source</span>
      <span class="legend-item"><span class="legend-dot" style:background={NODE_COLORS.bullet}></span> Bullet</span>
      <span class="legend-item"><span class="legend-dot" style:background={NODE_COLORS.perspective}></span> Perspective</span>
      <span class="legend-item"><span class="legend-dot" style:background={NODE_COLORS.resume_entry}></span> Entry</span>
      <span class="legend-divider">|</span>
      <span class="legend-item"><span class="legend-line" style:background="#94a3b8"></span> Matching</span>
      <span class="legend-item"><span class="legend-line" style:background="#ef4444"></span> Drifted</span>
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
  {:else if nodeCount === 0}
    <EmptyState
      title="No chain data"
      description="Create sources and derive bullets to see the provenance graph."
    />
  {:else}
    <div class="graph-layout">
      <div class="graph-container" bind:this={container}></div>
      <!-- ... identical detail panel + edge tooltip markup ... -->
      {#if selectedNodeData}
        <!-- ... same detail panel as modal mode ... -->
      {:else if selectedEdgeData}
        <!-- ... same edge detail panel as modal mode ... -->
      {/if}
    </div>
    {#if edgeTooltip}
      <!-- ... same edge tooltip as modal mode ... -->
    {/if}
  {/if}
{/if}
```

**Implementation note:** To avoid duplicating the controls, stats, loading, graph-layout, detail panel, and edge tooltip markup between the two `{#if}` branches, use Svelte snippet blocks (`{#snippet}` / `{@render}`) to define the shared content once. The only difference between the branches is the outer wrapper (backdrop + modal chrome vs bare div). Concretely:

```svelte
{#snippet chainContent()}
  <!-- Controls -->
  <div class="controls">
    <!-- ... all controls ... -->
  </div>

  <!-- Stats bar -->
  <div class="stats-bar">
    <!-- ... all stats ... -->
  </div>

  {#if loading}
    <!-- ... loading state ... -->
  {:else if error}
    <!-- ... error state ... -->
  {:else if nodeCount === 0}
    <!-- ... empty state ... -->
  {:else}
    <div class="graph-layout">
      <!-- ... graph + detail panel ... -->
    </div>
    {#if edgeTooltip}
      <!-- ... edge tooltip ... -->
    {/if}
  {/if}
{/snippet}

{#if isModal}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div
    class="chain-modal-backdrop"
    onclick={handleBackdropClick}
    onkeydown={handleKeydown}
    tabindex="-1"
  >
    <div
      class="chain-modal-content"
      role="dialog"
      aria-modal="true"
      aria-labelledby="chain-modal-title"
      onclick={(e) => e.stopPropagation()}
    >
      <div class="chain-modal-header">
        <h2 id="chain-modal-title">Chain View</h2>
        <button class="modal-close-btn" onclick={close}>Close</button>
      </div>
      {@render chainContent()}
    </div>
  </div>
{:else}
  {@render chainContent()}
{/if}
```

This eliminates all markup duplication.

#### T33.2.6: Styles

All styles from the current `+page.svelte` (lines 742-1163) move into the component. Add the modal-specific styles:

```css
/* Modal chrome */
.chain-modal-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  z-index: 9999;
}

.chain-modal-content {
  position: absolute;
  inset: 2rem;
  background: #fff;
  border-radius: 12px;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
}

.chain-modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.75rem 1.25rem;
  border-bottom: 1px solid #e5e7eb;
  background: #f9fafb;
  flex-shrink: 0;
}

.chain-modal-header h2 {
  font-size: 1.1rem;
  font-weight: 600;
  color: #1a1a2e;
}

.modal-close-btn {
  padding: 0.35rem 0.75rem;
  background: none;
  border: 1px solid #d1d5db;
  border-radius: 4px;
  font-size: 0.8rem;
  color: #6b7280;
  cursor: pointer;
  font-family: inherit;
  transition: background 0.1s;
}

.modal-close-btn:hover {
  background: #f3f4f6;
}
```

**Existing styles to modify for modal context:**

The `.graph-layout` height must adapt to the container. In standalone mode it uses `calc(100vh - 260px)`. In modal mode, it should flex to fill available space:

```css
.graph-layout {
  display: flex;
  gap: 0;
  flex: 1;
  min-height: 0;  /* allow flex shrink */
  border: 1px solid #e5e7eb;
  border-radius: 8px;
  overflow: hidden;
  background: #fff;
}
```

When `isModal === false`, the component is rendered inside `.chain-page` which has no flex context, so `flex: 1` has no effect and the layout needs an explicit height. Add a conditional class:

```svelte
<div class="graph-layout" class:graph-layout-standalone={!isModal}>
```

```css
.graph-layout-standalone {
  height: calc(100vh - 260px);
}
```

**Acceptance criteria:**
- [ ] All script logic from `+page.svelte` (lines 1-474) is in the component
- [ ] All markup from `+page.svelte` (lines 476-740) is in the component (via snippets)
- [ ] All styles from `+page.svelte` (lines 742-1163) are in the component
- [ ] `isModal=true` renders with backdrop, close button, `role="dialog"` on content div
- [ ] `isModal=false` renders inline without modal chrome
- [ ] `role="dialog"` is on `.chain-modal-content`, NOT on `.chain-modal-backdrop`
- [ ] Escape key calls `close()` only when `isModal === true`
- [ ] Backdrop click calls `close()`
- [ ] `close()` calls `onClose` if provided, else `closeChainView()`
- [ ] Highlight uses `selectNodeInGraph` (NOT a fictional `selectAndCenterNode`)
- [ ] If `highlightNode` references a node that does not exist in the graph (e.g., deleted entity), the modal opens normally with no highlight and no error
- [ ] Highlight logic uses `page.url.searchParams.get('highlight')` (NOT `window.location.search`)
- [ ] Sigma cleanup in both `$effect` return (primary) and `onDestroy` (backup)
- [ ] Body scroll locked (`overflow: hidden`) when `isModal` is true
- [ ] Body scroll restored in `$effect` cleanup
- [ ] Snippet blocks used to avoid duplicating controls/stats/graph/detail markup
- [ ] `.graph-layout` height adapts correctly in both modal and standalone modes

**Failure criteria:**
- `role="dialog"` placed on the backdrop element (a11y violation)
- Escape handler not gated on `isModal` (would mutate store state in standalone mode)
- Uses `window.location.search` instead of `page.url.searchParams`
- Uses `selectAndCenterNode` or other non-existent function
- Only uses `onDestroy` for Sigma cleanup (leaks on Retry re-runs)
- Only uses `$effect` return for Sigma cleanup (may not fire on abrupt unmount)
- Markup duplicated between `{#if isModal}` branches instead of using snippets
- `.chain-modal-content` has `role="dialog"` on the backdrop div

---

### T33.3: Replace `/chain/+page.svelte` with Thin Wrapper

**File to modify:** `packages/webui/src/routes/chain/+page.svelte`

**Goal:** Replace the entire 1,160-line file with a ~20-line wrapper that renders `ChainViewModal` in inline mode.

```svelte
<script lang="ts">
  import ChainViewModal from '$lib/components/ChainViewModal.svelte'
</script>

<div class="chain-page">
  <h1 class="page-title">Chain View</h1>
  <p class="page-description">Interactive provenance graph: Source &rarr; Bullet &rarr; Perspective</p>
  <ChainViewModal isModal={false} />
</div>

<style>
  .chain-page {
    max-width: 1400px;
  }

  .page-title {
    font-size: 1.5rem;
    font-weight: 700;
    color: #1a1a2e;
    margin-bottom: 0.25rem;
  }

  .page-description {
    font-size: 0.85rem;
    color: #6b7280;
    margin-bottom: 1.5rem;
  }
</style>
```

When `isModal={false}`:
- No backdrop, close button, or escape handler
- `?highlight=` query param still works (the component reads from `page.url.searchParams`)
- No `onClose` prop needed (close button is not rendered)

**Acceptance criteria:**
- [ ] File is ~20 lines (down from ~1,160)
- [ ] `ChainViewModal` is imported and rendered with `isModal={false}`
- [ ] No `onClose` prop passed
- [ ] Page title and description remain
- [ ] `.chain-page` max-width preserved
- [ ] Direct URL `/chain` still works
- [ ] `/chain?highlight=source-{id}` still highlights the correct node

**Failure criteria:**
- Any chain view logic (Sigma, graph building, state) remains in this file
- `isModal={true}` used (would render a modal inside a page)

---

### T33.4: Mount Modal in `+layout.svelte`

**File to modify:** `packages/webui/src/routes/+layout.svelte`

**Goal:** Import and conditionally mount `ChainViewModal` at the layout level so it is available from any page.

**Hard prerequisite:** Phase 32 (Nav Restructuring) must be merged first. The layout file is restructured by Phase 32. Applying this change to the pre-Phase-32 layout would create merge conflicts.

**Changes to the current layout:**

Add imports in `<script>`:

```svelte
<script>
  import { page } from '$app/state'
  import { ToastContainer } from '$lib/components'
  import ChainViewModal from '$lib/components/ChainViewModal.svelte'
  import { chainViewState, closeChainView } from '$lib/stores/chain-view.svelte'

  // ... existing navItems, props, etc. from Phase 32 layout ...

  let { children } = $props()
</script>
```

Add the conditional mount after `<ToastContainer />`:

```svelte
<div class="app">
  <nav class="sidebar"><!-- nav content from Phase 32 --></nav>
  <main class="content">
    {@render children()}
  </main>
</div>

<ToastContainer />

{#if chainViewState.isOpen}
  <ChainViewModal
    highlightNode={chainViewState.highlightNode}
    isModal={true}
    onClose={closeChainView}
  />
{/if}
```

**Key design decisions:**
- `{#if chainViewState.isOpen}` conditional rendering means zero overhead when modal is closed. No Sigma instance, no graph data, no WebGL context.
- `onClose={closeChainView}` ensures the store is updated when the modal is dismissed. The `ChainViewModal.close()` function calls `onClose()` if provided, which calls `closeChainView()`, which sets `isOpen = false`, which unmounts the component via the `{#if}` block.
- The modal mount is placed after `<ToastContainer />` and outside the `.app` div so it renders on top of everything.

**Acceptance criteria:**
- [ ] `ChainViewModal` imported from `$lib/components/ChainViewModal.svelte`
- [ ] `chainViewState` and `closeChainView` imported from `$lib/stores/chain-view.svelte`
- [ ] Modal conditionally rendered with `{#if chainViewState.isOpen}`
- [ ] `highlightNode`, `isModal={true}`, and `onClose={closeChainView}` props passed
- [ ] Modal mount is after `<ToastContainer />` and outside `.app`
- [ ] No changes to existing layout structure or nav

**Failure criteria:**
- Modal mounted inside `.app` (could be clipped by overflow)
- Missing `onClose` prop (modal would call `closeChainView()` directly, which works but bypasses the explicit prop pattern)
- `chainViewState.isOpen` accessed without importing from `.svelte.ts` (would not be reactive)

---

### T33.5: Update DragNDropView Provenance Links

**File to modify:** `packages/webui/src/lib/components/resume/DragNDropView.svelte`

**Goal:** Replace the three `<a href="/chain?highlight=...">` links in the provenance tooltip with `<button onclick>` handlers that open the chain view modal.

#### T33.5.1: Add import

Add to the existing `<script>` block:

```typescript
import { openChainView } from '$lib/stores/chain-view.svelte'
```

#### T33.5.2: Replace links with buttons

**Before (lines 430-449):**

```svelte
<div class="tooltip-row">
  <strong>Source:</strong>
  <span class="tooltip-label">{tooltipEntry.source_chain.source_title}</span>
  <a
    class="tooltip-link"
    href="/chain?highlight=source-{tooltipEntry.source_chain.source_id}"
  >&rarr;</a>
</div>
<div class="tooltip-row">
  <strong>Bullet:</strong>
  <span class="tooltip-label">{tooltipEntry.source_chain.bullet_preview}</span>
  <a
    class="tooltip-link"
    href="/chain?highlight=bullet-{tooltipEntry.source_chain.bullet_id}"
  >&rarr;</a>
</div>
<div class="tooltip-row">
  <strong>Perspective:</strong>
  <span class="tooltip-label">{tooltipEntry.source_chain.perspective_preview}</span>
  <a
    class="tooltip-link"
    href="/chain?highlight=perspective-{tooltipEntry.source_chain.perspective_id}"
  >&rarr;</a>
</div>
```

**After:**

The `onclick` closures must guard against null `tooltipEntry` (which can be nulled by the 150ms dismiss timer between render and click). Capture `tooltipEntry` locally in each handler:

```svelte
<div class="tooltip-row">
  <strong>Source:</strong>
  <span class="tooltip-label">{tooltipEntry.source_chain.source_title}</span>
  <button
    class="tooltip-link"
    onclick={() => { const t = tooltipEntry; if (t?.source_chain) openChainView(`source-${t.source_chain.source_id}`) }}
  >&rarr;</button>
</div>
<div class="tooltip-row">
  <strong>Bullet:</strong>
  <span class="tooltip-label">{tooltipEntry.source_chain.bullet_preview}</span>
  <button
    class="tooltip-link"
    onclick={() => { const t = tooltipEntry; if (t?.source_chain) openChainView(`bullet-${t.source_chain.bullet_id}`) }}
  >&rarr;</button>
</div>
<div class="tooltip-row">
  <strong>Perspective:</strong>
  <span class="tooltip-label">{tooltipEntry.source_chain.perspective_preview}</span>
  <button
    class="tooltip-link"
    onclick={() => { const t = tooltipEntry; if (t?.source_chain) openChainView(`perspective-${t.source_chain.perspective_id}`) }}
  >&rarr;</button>
</div>
```

#### T33.5.3: Update `.tooltip-link` styles

**Before:**

```css
.tooltip-link {
  color: #6c63ff;
  text-decoration: none;
  margin-left: 0.35rem;
  flex-shrink: 0;
}

.tooltip-link:hover {
  text-decoration: underline;
}
```

**After:**

```css
.tooltip-link {
  color: #6c63ff;
  background: none;
  border: none;
  padding: 0;
  margin-left: 0.35rem;
  flex-shrink: 0;
  cursor: pointer;
  font-size: inherit;
  font-family: inherit;
  line-height: inherit;
}

.tooltip-link:hover {
  text-decoration: underline;
}
```

The style update removes anchor-specific defaults (links have built-in pointer cursor, buttons do not), adds `cursor: pointer`, and resets button defaults (`background: none`, `border: none`, `padding: 0`).

**Acceptance criteria:**
- [ ] `openChainView` imported from `$lib/stores/chain-view.svelte`
- [ ] All three `<a>` tags replaced with `<button>` tags
- [ ] Each button's `onclick` calls `openChainView(...)` with the correct node key
- [ ] Node key format preserved: `source-{id}`, `bullet-{id}`, `perspective-{id}`
- [ ] `.tooltip-link` styles updated with button resets
- [ ] Clicking a provenance arrow opens the chain view modal without navigating away
- [ ] Resume page editing state is preserved after opening/closing the modal

**Failure criteria:**
- Links remain as `<a>` tags (would navigate away, destroying editing state)
- `onclick` uses `goto()` or any navigation
- Node key format changed (e.g., missing the `source-` / `bullet-` / `perspective-` prefix)
- `openChainView` imported from wrong file (e.g., bare `.ts` instead of `.svelte.ts`)

---

## Testing Support

### Test kinds

| Kind | Tests | Rationale |
|------|-------|-----------|
| Component (unit) | Store behavior | Verify open/close/highlight state transitions |
| Smoke (manual) | Modal open/close from provenance tooltip | Verify full integration path: click arrow in DnD view -> modal opens -> graph renders -> highlighted node centered |
| Smoke (manual) | Standalone `/chain` route | Verify route still works after extraction |
| Smoke (manual) | Modal dismiss methods | Escape key, backdrop click, close button |
| Smoke (manual) | Body scroll lock | Scroll lock applied on open, removed on close |
| Smoke (manual) | Repeated open/close | Open/close 5+ times, verify no WebGL context leaks (check browser console for warnings) |
| Visual (manual) | Modal appearance | Verify backdrop opacity, content positioning, border-radius, header styling |

### Store unit tests

**File:** `packages/webui/src/lib/stores/__tests__/chain-view.test.ts`

These tests verify the store's state management logic. Note: Svelte runes (`$state`) require a Svelte compilation context, so these tests may need to be run through vitest with the Svelte plugin configured, or the store logic can be tested via a thin wrapper.

**Vitest configuration requirement:** Store unit tests require Svelte compilation context for `$state` runes. Verify that `packages/webui/vitest.config.ts` includes the Svelte plugin. If no vitest config exists, these tests must be deferred until one is configured, or test the store logic via integration tests (opening/closing the modal in the browser).

```typescript
import { describe, test, expect } from 'vitest'
// Import may need adjustment based on vitest Svelte plugin configuration
import { openChainView, closeChainView, chainViewState } from '$lib/stores/chain-view.svelte'

describe('chain-view store', () => {
  test('initial state is closed with no highlight', () => {
    expect(chainViewState.isOpen).toBe(false)
    expect(chainViewState.highlightNode).toBeNull()
  })

  test('openChainView() opens with null highlight', () => {
    openChainView()
    expect(chainViewState.isOpen).toBe(true)
    expect(chainViewState.highlightNode).toBeNull()
    closeChainView() // cleanup
  })

  test('openChainView(nodeKey) opens with specific highlight', () => {
    openChainView('source-abc-123')
    expect(chainViewState.isOpen).toBe(true)
    expect(chainViewState.highlightNode).toBe('source-abc-123')
    closeChainView() // cleanup
  })

  test('closeChainView() resets both isOpen and highlightNode', () => {
    openChainView('bullet-xyz')
    closeChainView()
    expect(chainViewState.isOpen).toBe(false)
    expect(chainViewState.highlightNode).toBeNull()
  })

  test('re-open with different highlight replaces previous', () => {
    openChainView('source-aaa')
    openChainView('perspective-bbb')
    expect(chainViewState.highlightNode).toBe('perspective-bbb')
    closeChainView()
  })
})
```

### Smoke test checklist (manual)

- [ ] **Provenance tooltip -> modal:** Navigate to a resume's DragNDrop view. Hover over an entry to see the provenance tooltip. Click the arrow button next to "Source". Verify: modal opens, graph renders, source node is enlarged and centered. Close modal. Verify: resume page state is unchanged.
- [ ] **All three link types:** Repeat for "Bullet" and "Perspective" arrows. Verify each opens the modal with the correct node highlighted.
- [ ] **Escape dismissal:** Open modal via provenance link. Press Escape. Verify: modal closes, store state is `isOpen=false, highlightNode=null`.
- [ ] **Backdrop click dismissal:** Open modal. Click the semi-transparent backdrop outside the white content area. Verify: modal closes.
- [ ] **Close button dismissal:** Open modal. Click the "Close" button in the modal header. Verify: modal closes.
- [ ] **Standalone `/chain` route:** Navigate directly to `/chain`. Verify: graph renders inline (no modal chrome, no backdrop, no close button).
- [ ] **Standalone with highlight:** Navigate to `/chain?highlight=source-{some-id}`. Verify: the specified node is enlarged and centered.
- [ ] **Body scroll lock:** Open modal. Attempt to scroll the page behind the modal. Verify: page does not scroll. Close modal. Verify: page scrolls normally.
- [ ] **Repeated open/close:** Open and close the modal 5+ times with different highlight targets. Verify: no console warnings about WebGL context limits, each open shows the correct highlighted node, previous highlight is cleared.
- [ ] **Retry after error:** In the modal, if graph loading fails, click "Retry". Verify: graph reloads successfully, Sigma instance is recreated (old one cleaned up).
- [ ] **Escape key in standalone mode:** Navigate to `/chain`. Press Escape. Verify: nothing happens (no store mutation, no navigation).
- [ ] **Standalone visual test:** Verify standalone mode (`isModal={false}`): graph fills the page container, no backdrop visible, no close button rendered.

---

## Documentation Requirements

- [ ] Add JSDoc to `openChainView`, `closeChainView`, and `chainViewState` in the store file
- [ ] Add component-level doc comment in `ChainViewModal.svelte` explaining the dual-mode (modal/inline) pattern
- [ ] Document the props interface (`highlightNode`, `isModal`, `onClose`) with inline comments
- [ ] Add a comment in `+layout.svelte` at the modal mount point explaining why it is placed outside `.app`

---

## Parallelization Notes

**Internal task parallelization:** T33.1 (store) has no dependencies and can be implemented first. T33.2 (component) depends on T33.1 (imports the store). T33.3 (page wrapper), T33.4 (layout mount), and T33.5 (DnD link updates) all depend on T33.1 and T33.2. T33.3, T33.4, and T33.5 are independent of each other and can be done in parallel after T33.2.

**Recommended implementation order:**
1. T33.1 (store)
2. T33.2 (component extraction -- largest task, ~90% of the work)
3. T33.3, T33.4, T33.5 (in any order or parallel)

**External parallelization:** Phase 33 can run in parallel with Phases 29-31 and 34-36, as long as Phase 32 (Nav Restructuring) has been merged. Phase 33 touches the layout file, but only appends a conditional block after `<ToastContainer />` -- it does not modify existing layout structure, so merge conflicts with other phases are unlikely.

---

## Acceptance Criteria

- [ ] Chain view modal opens from provenance tooltip links in DragNDropView
- [ ] Resume page editing state is preserved after opening/closing the modal
- [ ] Highlighted node is enlarged (`size * 2`), marked (`highlighted: true`), selected, and camera-centered
- [ ] If `highlightNode` references a node that does not exist in the graph (e.g., deleted entity), the modal opens normally with no highlight and no error
- [ ] Escape key, backdrop click, and close button all dismiss the modal
- [ ] Escape key does NOT trigger close in standalone mode (`isModal=false`)
- [ ] `role="dialog"` is on `.chain-modal-content`, NOT on `.chain-modal-backdrop`
- [ ] Body scroll is locked when modal is open; restored when modal closes
- [ ] Standalone `/chain` route still works (inline rendering, no modal chrome)
- [ ] `/chain?highlight=source-{id}` still works for direct linking
- [ ] No WebGL context leaks after opening/closing the modal multiple times
- [ ] Store state is `isOpen=false, highlightNode=null` after close
- [ ] Open modal, close, reopen with different highlight -- new node highlighted, old one cleared
- [ ] Retry button in error state successfully renders the graph (Sigma re-created)
- [ ] `+page.svelte` reduced from ~1,160 lines to ~20 lines
- [ ] Store file uses `.svelte.ts` extension
- [ ] Store uses getter-object pattern (NOT function-export like toast store)

## Estimated Effort

| Task | Lines changed (est.) |
|------|---------------------|
| T33.1 Store | ~30 (new file) |
| T33.2 Component extraction | ~1,200 (new file, moved from +page.svelte) |
| T33.3 Page wrapper | ~1,140 deleted, ~20 written |
| T33.4 Layout mount | ~10 added |
| T33.5 DragNDropView updates | ~30 changed |
| Tests | ~60 (new file) |
| **Total** | **~1,350 net** |
