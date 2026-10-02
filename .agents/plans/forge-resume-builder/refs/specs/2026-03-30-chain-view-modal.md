# Chain View as Modal

**Date:** 2026-03-30
**Status:** Design
**Builds on:** Chain View Edge Rendering (Phase 25), Provenance Tooltip Enhancement (Phase 24), Resume Sections as Entities (Phases 27-28)

## Purpose

Convert the Chain View from a full-page route (`/chain`) to a modal overlay that can be triggered from anywhere without losing context. The primary motivation is the resume builder's provenance tooltips: clicking a provenance link in `DragNDropView.svelte` currently navigates away from the resume page to `/chain?highlight=source-{id}`, destroying all in-progress editing state. The chain view should open as an overlay instead, keeping the resume page live underneath.

## Goals

1. Extract chain view logic from `/chain/+page.svelte` into a reusable `ChainViewModal.svelte` component
2. Create a global `chainViewStore` that controls modal visibility and highlight target
3. Mount the modal in `+layout.svelte` so it is available from any page
4. Replace provenance `<a href>` links in `DragNDropView.svelte` with `onclick` calls to `openChainView()`
5. Preserve the `/chain` route as a standalone page (for direct access and bookmarking)

## Non-Goals

- Persistent Sigma.js instance across navigations (too much memory/WebGL state)
- Mini/embedded chain view within the resume builder (this is always full-screen overlay)
- Editing entities from within the modal (read-only graph inspection)

---

## 1. Store: `chain-view.svelte.ts`

**File:** `packages/webui/src/lib/stores/chain-view.svelte.ts`

The store uses a getter-object pattern for reactive state exposure -- module-level `$state` runes with exported mutation functions, and a `chainViewState` object whose properties are getters that read the runes reactively. This allows consumers to import `chainViewState` and access `.isOpen` / `.highlightNode` as reactive values without needing to call functions.

```typescript
// packages/webui/src/lib/stores/chain-view.svelte.ts
let isOpen = $state(false)
let highlightNode = $state<string | null>(null)

export function openChainView(nodeKey?: string) {
  highlightNode = nodeKey ?? null
  isOpen = true
}

export function closeChainView() {
  isOpen = false
  highlightNode = null
}

// Export as reactive getters — callers access .isOpen and .highlightNode reactively
export const chainViewState = {
  get isOpen() { return isOpen },
  get highlightNode() { return highlightNode },
}
```

**Usage from any component:**
```typescript
import { openChainView } from '$lib/stores/chain-view.svelte'

// From provenance tooltip:
openChainView(`source-${sourceId}`)

// From any other context:
openChainView()  // opens without highlighting a specific node
```

## 2. Component: `ChainViewModal.svelte`

**File:** `packages/webui/src/lib/components/ChainViewModal.svelte`

### 2.1 Extraction from `+page.svelte`

The current `/chain/+page.svelte` contains approximately 470 lines of script + 700 lines of markup/style. The modal component extracts everything except the page-level wrapper markup (`<h1>`, `<p>` description).

**What moves into the component:**
- All Sigma.js state (`container`, `sigmaInstance`, `graph`, `loading`, `error`)
- Interaction state (`selectedNodeId`, `selectedNodeData`, `searchQuery`, filters)
- Edge interaction state (`selectedEdgeData`, `hoveredEdge`, `edgeTooltip`, `edgeTooltipPosition`)
- Related entity lookups (`allSources`, `allBullets`, `allPerspectives`, derived state)
- `buildGraph()` function
- Sigma initialization `$effect`
- Filter/search `$effect`
- All markup: controls bar, stats bar, graph container, detail panel, edge tooltip
- All styles

**What the component receives as props:**
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

- `highlightNode`: the graph node key to highlight on open (e.g., `"source-550e8400-..."`)
- `isModal`: when `true`, component renders with modal chrome (backdrop, close button, escape key handler). When `false`, renders as inline content (for the `/chain` route).
- `onClose`: callback invoked when the modal is dismissed. In the close handler: `if (onClose) onClose(); else closeChainView();`. This allows the parent (layout) to control cleanup via the store. When `isModal={false}` (standalone page), the close button is hidden. The `onClose` prop is only called in modal mode.

### 2.2 Highlight handling

The current `+page.svelte` reads `?highlight=` from the URL query params after Sigma initializes. The modal variant receives `highlightNode` as a prop instead. The component unifies both paths:

> **Migration note:** Line 390 of the current `+page.svelte` uses `window.location.search` to read the highlight param. This must be replaced with `page.url.searchParams.get('highlight')` from `$app/state` (imported below). Do not use `window.location` in SvelteKit components.

```typescript
// After Sigma init completes:
// In standalone mode, use SvelteKit's page state (not window.location):
import { page } from '$app/state'

const nodeToHighlight = isModal ? highlightNode : page.url.searchParams.get('highlight')
if (nodeToHighlight && graph!.hasNode(nodeToHighlight)) {
  selectNodeInGraph(nodeToHighlight)
  // selectNodeInGraph must be extended to also set `size: currentSize * 2`
  // and `highlighted: true` on the target node, matching the current inline
  // highlight logic at +page.svelte lines 392-394.
}
```

### 2.3 Modal chrome

When `isModal === true`, the component wraps the graph in a full-screen overlay:

```svelte
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
        <button class="close-btn" onclick={close}>Close</button>
      </div>
      <!-- graph content here -->
    </div>
  </div>
{:else}
  <!-- inline mode: same content without backdrop -->
{/if}
```

**Close handler:**
```typescript
function close() {
  if (onClose) onClose()
  else closeChainView()
}
```

**Keydown handler:**
```typescript
function handleKeydown(e: KeyboardEvent) {
  if (isModal && e.key === 'Escape') close()
}
```

The Escape handler is gated on `isModal` so that in standalone mode (`isModal={false}`), pressing Escape does not call `close()` -> `closeChainView()`, which would mutate store state unnecessarily.

**Key behaviors:**
- Escape key dismisses the modal (only when `isModal === true`)
- Clicking the semi-transparent backdrop dismisses the modal (calls `close()`)
- Close button in the header dismisses the modal (calls `close()`)
- Body scroll is locked while modal is open (`document.body.style.overflow = 'hidden'`)

### 2.4 Lifecycle

When `ChainViewModal` is mounted via `{#if chainViewState.isOpen}`, the component is created fresh each open and destroyed on close. Sigma cleanup uses two complementary mechanisms:

1. The `$effect` return handles cleanup on re-runs (e.g., Retry after error). When `buildGraph()` is called again, the effect's cleanup function fires first, tearing down the previous Sigma instance before creating a new one.
2. `onDestroy` handles final component teardown as a belt-and-suspenders backup.

```typescript
import { onDestroy } from 'svelte'

// Primary cleanup: $effect return handles re-runs (Retry button)
$effect(() => {
  buildGraph()
  // ... Sigma initialization ...

  return () => {
    if (sigmaInstance) {
      sigmaInstance.kill()
      sigmaInstance = null
    }
  }
})

// Belt-and-suspenders: onDestroy handles final component teardown
onDestroy(() => {
  if (sigmaInstance) {
    sigmaInstance.kill()
    sigmaInstance = null
  }
})
```

> **Note:** The `$effect` return handles cleanup on re-runs (e.g., Retry after error). `onDestroy` handles final component teardown. Using only `onDestroy` would leak the old Sigma instance if `buildGraph()` is called again within the same component lifecycle.

The `$effect` in the modal sets `document.body.style.overflow = 'hidden'` on mount and restores to `''` in the cleanup return:

```typescript
$effect(() => {
  if (isModal) {
    document.body.style.overflow = 'hidden'
    return () => { document.body.style.overflow = '' }
  }
})
```

## 3. Layout Mount

**File:** `packages/webui/src/routes/+layout.svelte`

**Depends on:** Spec 1 (Nav Restructuring -- the layout file is restructured by Spec 1; this spec's modal mount must be applied to the new layout).

> **Hard prerequisite:** Do not implement Spec 5 until Spec 1 (Nav Restructuring) is merged. The `+layout.svelte` mount point in Section 3 below assumes the Spec 1 layout structure (sidebar nav with grouped accordion sections). Applying the modal mount to the pre-Spec-1 layout would create a merge conflict when Spec 1 lands.

The modal is mounted at the layout level so it is available from any page:

```svelte
<script>
  import { page } from '$app/state'
  import { ToastContainer } from '$lib/components'
  import ChainViewModal from '$lib/components/ChainViewModal.svelte'
  import { chainViewState, closeChainView } from '$lib/stores/chain-view.svelte'

  // Layout structure from Spec 1 (Nav restructuring) — see that spec for the full sidebar
  // navigation model with grouped accordion sections.

  let { children } = $props()
</script>

<div class="app">
  <nav class="sidebar"><!-- Grouped accordion nav from Spec 1 --></nav>
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

**Conditional rendering:** The `ChainViewModal` is only mounted when `isOpen === true`. This means the Sigma.js instance, graph data, and WebGL context are only allocated when the modal is visible. When closed, the entire component unmounts and all resources are freed.

## 4. DragNDropView Provenance Link Updates

**File:** `packages/webui/src/lib/components/resume/DragNDropView.svelte`

Replace the three `<a href="/chain?highlight=...">` links in the provenance tooltip with `onclick` handlers:

**Before (lines 430-449):**
```svelte
<a class="tooltip-link" href="/chain?highlight=source-{tooltipEntry.source_chain.source_id}">&rarr;</a>
<!-- ... -->
<a class="tooltip-link" href="/chain?highlight=bullet-{tooltipEntry.source_chain.bullet_id}">&rarr;</a>
<!-- ... -->
<a class="tooltip-link" href="/chain?highlight=perspective-{tooltipEntry.source_chain.perspective_id}">&rarr;</a>
```

**After:**
```svelte
<script lang="ts">
  import { openChainView } from '$lib/stores/chain-view.svelte'
</script>

<button class="tooltip-link"
  onclick={() => openChainView(`source-${tooltipEntry.source_chain.source_id}`)}
>&rarr;</button>

<button class="tooltip-link"
  onclick={() => openChainView(`bullet-${tooltipEntry.source_chain.bullet_id}`)}
>&rarr;</button>

<button class="tooltip-link"
  onclick={() => openChainView(`perspective-${tooltipEntry.source_chain.perspective_id}`)}
>&rarr;</button>
```

The `<a>` tags become `<button>` tags because they no longer navigate. The `.tooltip-link` style is updated to remove anchor-specific defaults and add `cursor: pointer`.

## 5. Standalone `/chain` Route

**File:** `packages/webui/src/routes/chain/+page.svelte`

The page becomes a thin wrapper around the shared component:

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
  .chain-page { max-width: 1400px; }
  .page-title { font-size: 1.5rem; font-weight: 700; color: #1a1a2e; margin-bottom: 0.25rem; }
  .page-description { font-size: 0.85rem; color: #6b7280; margin-bottom: 1.5rem; }
</style>
```

When `isModal={false}`, the component renders inline without a backdrop or close button. The `?highlight=` query param still works for direct linking/bookmarking.

## 6. Modal Styling

```css
.chain-modal-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  z-index: 9999;
}

.chain-modal-content {
  position: absolute;
  inset: 2rem;           /* 2rem margin on all sides */
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
```

The modal content area uses `inset: 2rem` for a full-screen-with-margin effect. The graph container inside flexes to fill the remaining space.

## Key Decisions

| Decision | Rationale |
|----------|-----------|
| Full-screen overlay, not small popup | Sigma.js graph needs space for pan/zoom to be usable |
| Sigma instance created on open, destroyed on close | WebGL contexts are limited per page; keeping one alive while hidden wastes GPU memory and can hit browser limits |
| `highlightNode` as prop, not URL param | Modal does not change the URL; the `/chain?highlight=` route still works for direct access |
| `{#if chainViewState.isOpen}` conditional rendering | Ensures zero overhead when modal is not visible -- no Sigma, no graph, no WebGL context |
| Body scroll lock | Prevents background page from scrolling while the graph overlay is active |
| Backdrop click + Escape to dismiss | Standard modal UX pattern |

## Files to Create

| File | Description |
|------|-------------|
| `packages/webui/src/lib/stores/chain-view.svelte.ts` | Global store for modal open/close state |
| `packages/webui/src/lib/components/ChainViewModal.svelte` | Extracted chain view component (modal + inline) |

## Files to Modify

| File | Change |
|------|--------|
| `packages/webui/src/routes/chain/+page.svelte` | Replace 1100+ lines with thin wrapper (~20 lines) |
| `packages/webui/src/routes/+layout.svelte` | Import + mount `ChainViewModal` conditionally |
| `packages/webui/src/lib/components/resume/DragNDropView.svelte` | Replace `<a href="/chain?highlight=...">` with `onclick={openChainView(...)}` |

## Known Limitations

1. **Focus trapping is not implemented.** The modal backdrop captures Escape key but Tab does not cycle within the modal content.

## Testing

- Verify provenance tooltip links open the modal without leaving the resume page
- Verify the highlighted node is centered and enlarged in the modal graph
- Verify Escape key and backdrop click close the modal
- Verify the `/chain` standalone page still works with `?highlight=` query params
- Verify no WebGL context leaks after opening/closing the modal multiple times
- Verify body scroll is locked when modal is open and restored when closed
- Open modal, close, reopen with different highlight -- verify new node highlighted, old cleared
- Verify that clicking Retry after a load failure successfully renders the graph
- Store state is `isOpen=false, highlightNode=null` after close
