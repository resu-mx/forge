# Phase 55: Graph Toolbar (Spec H6)

**Status:** Planning
**Date:** 2026-04-03
**Spec:** [2026-04-03-graph-toolbar.md](../refs/specs/2026-04-03-graph-toolbar.md)
**Depends on:** Phase 48 (Generic GraphView), Phase 54 (Graph Search -- needs `getSigma()`/`getGraph()` exports)
**Blocks:** None
**Parallelizable with:** Phase 51, Phase 52, Phase 53, Phase 56 -- creates new files only (`GraphToolbar.svelte`, `graph.toolbar.ts`), modifies `GraphView.svelte` (adds `getContainer()` export)

## Goal

Add a floating toolbar overlay with buttons for zoom in/out, fit-to-screen, ForceAtlas2 layout toggle (play/pause using Web Worker), graph reset, and fullscreen toggle. The toolbar communicates with `GraphView` through exported methods (`getSigma()`, `getGraph()`, `getContainer()`). Keyboard shortcuts provide power-user access to all toolbar actions.

## Non-Goals

- Custom layout parameter tuning UI (no sliders)
- Export graph as image (future spec)
- Edge-specific controls
- Toolbar button customization or drag-to-reorder
- Mobile-specific controls (touch gestures handled by Sigma natively)

## Context

Phase 48's `GraphView` has no user-facing controls beyond mouse scroll-wheel zoom and drag. Users cannot pause the ForceAtlas2 simulation (it runs as a one-shot at construction, but interactive toggling requires the `graphology-layout-forceatlas2/worker` supervisor), reset the viewport, or toggle fullscreen.

The toolbar needs the Sigma instance and graphology Graph from `GraphView` via `getSigma()` and `getGraph()` (defined in Phase 48, verified in Phase 54). It also needs the container element for fullscreen, which requires a new `getContainer()` export on `GraphView`.

## Scope

| Spec section | Covered here? |
|-------------|---------------|
| 1. Toolbar Buttons (zoom, fit, layout toggle, reset, fullscreen) | Yes |
| 2. Component `GraphToolbar.svelte` (props, UI, styling) | Yes |
| 3. Keyboard Shortcuts | Yes |
| 4. Integration with GraphView (`getContainer()` export) | Yes |
| 5. Layout Presets (optional) | Yes |
| 6. Testing | Yes |

## Files to Create

| File | Description |
|------|-------------|
| `packages/webui/src/lib/components/graph/graph.toolbar.ts` | Toolbar action functions: `zoomIn`, `zoomOut`, `fitToScreen`, `toggleFullscreen`, `isInputFocused` |
| `packages/webui/src/lib/components/graph/GraphToolbar.svelte` | Floating toolbar component with buttons, keyboard shortcuts, layout toggle |
| `packages/webui/src/lib/components/graph/__tests__/graph-toolbar.test.ts` | Unit tests for toolbar actions and keyboard handling (11 cases) |

## Files to Modify

| File | Description |
|------|-------------|
| `packages/webui/src/lib/components/graph/GraphView.svelte` | Add `getContainer()` export |

## Fallback Strategies

- **`graphology-layout-forceatlas2/worker` import fails:** The `toggleLayout` function wraps the FA2Layout import in a try/catch. If the worker cannot be instantiated, log a warning and disable the layout toggle button (show it as grayed out). The one-shot layout from Phase 48 still works.
- **Fullscreen API not available:** Some browsers or embedded contexts do not support `document.fullscreenElement`. Guard with `if (!document.fullscreenEnabled)` and hide the fullscreen button.
- **`getSigma()` returns null before init:** All toolbar action functions guard against null Sigma. Buttons are disabled when Sigma is not initialized.
- **Web Worker cleanup leak:** `onDestroy` kills the FA2 worker instance. If the component unmounts before layout starts, the null check prevents calling `.kill()` on undefined.
- **Keyboard shortcut conflicts:** Shortcuts only fire when the graph container has focus or when using Ctrl/Cmd modifier keys. The `isInputFocused` guard prevents hijacking input/textarea typing.

---

## Tasks

### T55.1: Write Toolbar Action Functions

**File:** `packages/webui/src/lib/components/graph/graph.toolbar.ts`

[IMPORTANT] All action functions accept a Sigma instance as their first argument and guard against null. They are pure functions (no component state) so they can be tested in isolation.

[MINOR] `isInputFocused` checks the event target's tag name to avoid hijacking keyboard input in text fields.

```typescript
import type Sigma from 'sigma'

/**
 * Zoom the camera in with animation.
 */
export function zoomIn(sigma: Sigma): void {
  sigma.getCamera().animatedZoom({ duration: 200 })
}

/**
 * Zoom the camera out with animation.
 */
export function zoomOut(sigma: Sigma): void {
  sigma.getCamera().animatedUnzoom({ duration: 200 })
}

/**
 * Reset the camera to show all nodes.
 */
export function fitToScreen(sigma: Sigma): void {
  sigma.getCamera().animatedReset({ duration: 300 })
}

/**
 * Toggle fullscreen on the graph container element.
 * No-op if Fullscreen API is not available.
 */
export function toggleFullscreen(container: HTMLElement): void {
  if (!document.fullscreenEnabled) return
  if (document.fullscreenElement) {
    document.exitFullscreen()
  } else {
    container.requestFullscreen()
  }
}

/**
 * Check if the keyboard event originates from an input/textarea/select
 * to avoid hijacking typing.
 */
export function isInputFocused(e: KeyboardEvent): boolean {
  const tag = (e.target as HTMLElement)?.tagName
  return tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT'
}
```

**Acceptance criteria:**
- `zoomIn` calls `camera.animatedZoom` with `{ duration: 200 }`.
- `zoomOut` calls `camera.animatedUnzoom` with `{ duration: 200 }`.
- `fitToScreen` calls `camera.animatedReset` with `{ duration: 300 }`.
- `toggleFullscreen` enters fullscreen when not active, exits when active.
- `toggleFullscreen` is a no-op when Fullscreen API is unavailable.
- `isInputFocused` returns `true` for INPUT, TEXTAREA, SELECT tags.

**Failure criteria:**
- Functions throw when Sigma is null (should be guarded by caller).
- Fullscreen toggle crashes when API is unavailable.

---

### T55.2: Write `GraphToolbar.svelte`

**File:** `packages/webui/src/lib/components/graph/GraphToolbar.svelte`

[CRITICAL] The ForceAtlas2 worker instance (`FA2Layout`) must be killed on component unmount and when the graph reference changes. Without cleanup, the Web Worker runs indefinitely and leaks memory.

[IMPORTANT] The layout toggle uses `graphology-layout-forceatlas2/worker` for interactive simulation (continuous, runs in a Web Worker). This is distinct from Phase 48's one-shot `forceAtlas2.assign()` which runs synchronously during graph construction.

[IMPORTANT] When the graph object changes (new data loaded), the FA2 worker instance must be killed and re-created because it holds a reference to the old graph.

[ANTI-PATTERN] Do not store the FA2 instance in a Svelte `$state` variable -- it is a mutable external object and should be stored in a plain `let` variable. Only `layoutRunning` needs to be reactive.

```svelte
<!--
  GraphToolbar.svelte — Floating toolbar overlay for graph interactions.
  Provides zoom, layout toggle, reset, and fullscreen controls.
-->
<script lang="ts">
  import { onDestroy } from 'svelte'
  import { zoomIn, zoomOut, fitToScreen, toggleFullscreen, isInputFocused } from './graph.toolbar'
  import type { GraphConfig } from './graph.types'
  import type Graph from 'graphology'

  interface GraphToolbarProps {
    /** Reference to the GraphView component */
    graphView: any
    /** The graph config (for layout parameters) */
    config: GraphConfig
    /**
     * The graphology Graph instance, passed as a direct prop from the parent.
     * CRITICAL: The `$effect` tracking `graphView?.getGraph?.()` will not detect
     * inner state changes. Instead, accept `graph` as a direct prop. Svelte 5
     * reactivity tracks prop changes, so when the parent rebuilds the graph and
     * passes a new reference, this component reacts correctly.
     */
    graph?: Graph | null
    /** Position of the toolbar */
    position?: 'top-right' | 'top-left' | 'bottom-right' | 'bottom-left'
    /** Show layout preset buttons */
    showPresets?: boolean
    /** Show fullscreen toggle */
    showFullscreen?: boolean
  }

  let {
    graphView,
    config,
    graph = null,
    position = 'top-right',
    showPresets = false,
    showFullscreen = true,
  }: GraphToolbarProps = $props()

  // Layout state — NOT $state for the FA2 instance (mutable external object)
  let fa2Instance: any = null
  let layoutRunning = $state(false)
  let isFullscreen = $state(false)

  // Track graph changes to kill stale FA2 instances.
  // Uses the `graph` prop (direct reference) instead of `graphView?.getGraph?.()`.
  // Svelte 5 reactivity tracks prop changes, so this $effect runs when the
  // parent passes a new Graph reference.
  $effect(() => {
    const _graph = graph  // track reactive dependency on the prop
    // Kill stale instance when graph reference changes
    if (fa2Instance) {
      fa2Instance.kill()
      fa2Instance = null
      layoutRunning = false
    }
  })

  // Listen for fullscreen changes
  $effect(() => {
    function handleFullscreenChange() {
      isFullscreen = !!document.fullscreenElement
    }
    document.addEventListener('fullscreenchange', handleFullscreenChange)
    return () => document.removeEventListener('fullscreenchange', handleFullscreenChange)
  })

  async function toggleLayout() {
    if (!graph) return

    if (layoutRunning && fa2Instance) {
      fa2Instance.stop()
      layoutRunning = false
      return
    }

    if (!fa2Instance) {
      try {
        const { default: FA2Layout } = await import('graphology-layout-forceatlas2/worker')
        fa2Instance = new FA2Layout(graph, {
          settings: {
            gravity: config.forces.gravity,
            scalingRatio: config.forces.scalingRatio,
            slowDown: config.forces.slowDown,
            barnesHutOptimize: config.forces.barnesHutOptimize
              ?? graph.order > (config.forces.barnesHutThreshold ?? 100),
          },
        })
      } catch (e) {
        console.warn('GraphToolbar: Failed to initialize FA2 worker', e)
        return
      }
    }

    fa2Instance.start()
    layoutRunning = true
  }

  function stopLayout() {
    if (fa2Instance) {
      fa2Instance.stop()
      fa2Instance.kill()
      fa2Instance = null
      layoutRunning = false
    }
  }

  async function resetGraph() {
    const sigma = graphView?.getSigma?.()
    if (!sigma || !graph) return

    stopLayout()

    // Re-run one-shot layout
    try {
      const forceAtlas2 = await import('graphology-layout-forceatlas2')
      forceAtlas2.default.assign(graph, {
        iterations: config.forces.iterations ?? 100,
        settings: {
          gravity: config.forces.gravity,
          scalingRatio: config.forces.scalingRatio,
          slowDown: config.forces.slowDown,
        },
      })
    } catch (e) {
      console.warn('GraphToolbar: Failed to re-run layout', e)
    }

    sigma.refresh()
    sigma.getCamera().animatedReset({ duration: 300 })
  }

  function handleKeydown(e: KeyboardEvent) {
    const sigma = graphView?.getSigma?.()
    if (!sigma) return

    const mod = e.metaKey || e.ctrlKey

    if (mod && e.key === '=') {
      e.preventDefault()
      zoomIn(sigma)
    }
    if (mod && e.key === '-') {
      e.preventDefault()
      zoomOut(sigma)
    }
    if (mod && e.key === '0') {
      e.preventDefault()
      fitToScreen(sigma)
    }
    if (
      e.key === ' '
      && !isInputFocused(e)
      && graphView?.getContainer?.()?.contains(document.activeElement)
    ) {
      e.preventDefault()
      toggleLayout()
    }
    if ((e.key === 'f' || e.key === 'F11') && !isInputFocused(e)) {
      const container = graphView?.getContainer?.()
      if (container) {
        e.preventDefault()
        toggleFullscreen(container)
      }
    }
  }

  // Register keyboard listener on mount
  $effect(() => {
    document.addEventListener('keydown', handleKeydown)
    return () => document.removeEventListener('keydown', handleKeydown)
  })

  // Cleanup FA2 worker on unmount
  onDestroy(() => {
    if (fa2Instance) {
      fa2Instance.kill()
      fa2Instance = null
    }
  })

  let sigmaReady = $derived(!!graphView?.getSigma?.())
</script>

<div class="graph-toolbar {position}">
  <div class="toolbar-group">
    <button
      class="toolbar-btn"
      title="Zoom In (Ctrl/Cmd + =)"
      disabled={!sigmaReady}
      onclick={() => { const s = graphView?.getSigma?.(); if (s) zoomIn(s) }}
    >+</button>
    <button
      class="toolbar-btn"
      title="Zoom Out (Ctrl/Cmd + -)"
      disabled={!sigmaReady}
      onclick={() => { const s = graphView?.getSigma?.(); if (s) zoomOut(s) }}
    >-</button>
    <button
      class="toolbar-btn"
      title="Fit to Screen (Ctrl/Cmd + 0)"
      disabled={!sigmaReady}
      onclick={() => { const s = graphView?.getSigma?.(); if (s) fitToScreen(s) }}
    >&#8862;</button>
  </div>

  <div class="toolbar-separator"></div>

  <div class="toolbar-group">
    <button
      class="toolbar-btn"
      class:active={layoutRunning}
      title={layoutRunning ? 'Pause Layout (Space)' : 'Start Layout (Space)'}
      disabled={!sigmaReady}
      onclick={toggleLayout}
    >{layoutRunning ? '&#9208;' : '&#9654;'}</button>
    <button
      class="toolbar-btn"
      title="Reset Graph"
      disabled={!sigmaReady}
      onclick={resetGraph}
    >&#8634;</button>
    {#if showFullscreen}
      <button
        class="toolbar-btn"
        class:active={isFullscreen}
        title={isFullscreen ? 'Exit Fullscreen (F)' : 'Fullscreen (F)'}
        onclick={() => { const c = graphView?.getContainer?.(); if (c) toggleFullscreen(c) }}
      >&#9974;</button>
    {/if}
  </div>
</div>

<style>
  .graph-toolbar {
    position: absolute;
    z-index: 10;
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 4px;
    background: rgba(255, 255, 255, 0.9);
    border-radius: 8px;
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.1);
    backdrop-filter: blur(4px);
  }

  .graph-toolbar.top-right {
    top: 8px;
    right: 8px;
  }

  .graph-toolbar.top-left {
    top: 8px;
    left: 8px;
  }

  .graph-toolbar.bottom-right {
    bottom: 8px;
    right: 8px;
  }

  .graph-toolbar.bottom-left {
    bottom: 8px;
    left: 8px;
  }

  .toolbar-group {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .toolbar-btn {
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--color-border, #e5e7eb);
    border-radius: 6px;
    background: white;
    cursor: pointer;
    font-size: 16px;
    transition: background-color 0.15s;
  }

  .toolbar-btn:hover:not(:disabled) {
    background: var(--color-hover, #f3f4f6);
  }

  .toolbar-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .toolbar-btn.active {
    background: var(--color-primary-light, #dbeafe);
    border-color: var(--color-primary, #3b82f6);
  }

  .toolbar-separator {
    width: 24px;
    height: 1px;
    background: var(--color-border, #e5e7eb);
    margin: 2px auto;
  }
</style>
```

**Acceptance criteria:**
- Toolbar renders at the configured position (default `top-right`).
- Zoom in/out/fit buttons call the corresponding action functions.
- Layout toggle starts/stops the FA2 Web Worker simulation.
- Layout button icon toggles between play and pause.
- Reset stops simulation, re-runs one-shot layout, resets camera.
- Fullscreen button enters/exits fullscreen on the graph container.
- Buttons are disabled when `getSigma()` returns null.
- Fullscreen button is hidden when `showFullscreen` is false.
- FA2 worker is killed on unmount and on graph change.

**Failure criteria:**
- FA2 worker not killed on unmount (memory leak).
- FA2 instance holds reference to stale graph after graph change.
- Keyboard shortcuts fire when input fields are focused.

---

### T55.3: Add `getContainer()` Export to GraphView

**File:** `packages/webui/src/lib/components/graph/GraphView.svelte`

[MINOR] Simple one-line addition. The `container` variable is already defined as `$state<HTMLDivElement | null>(null)` and bound via `bind:this`.

```typescript
// --- Add to the exported methods section of GraphView.svelte ---

/**
 * Returns the graph container DOM element.
 * Used by GraphToolbar for fullscreen toggle.
 */
export function getContainer(): HTMLElement | null {
  return container
}
```

**Acceptance criteria:**
- `getContainer()` returns the `<div class="graph-view">` element.
- Returns `null` before the component mounts.

**Failure criteria:**
- Returns a different element or undefined.

---

### T55.4: Write Toolbar Action Unit Tests

**File:** `packages/webui/src/lib/components/graph/__tests__/graph-toolbar.test.ts`

```typescript
import { describe, it, expect, vi } from 'vitest'
import { zoomIn, zoomOut, fitToScreen, toggleFullscreen, isInputFocused } from '../graph.toolbar'

describe('zoomIn', () => {
  it('calls camera.animatedZoom with duration 200', () => {
    const camera = { animatedZoom: vi.fn() }
    const sigma = { getCamera: () => camera } as any
    zoomIn(sigma)
    expect(camera.animatedZoom).toHaveBeenCalledWith({ duration: 200 })
  })
})

describe('zoomOut', () => {
  it('calls camera.animatedUnzoom with duration 200', () => {
    const camera = { animatedUnzoom: vi.fn() }
    const sigma = { getCamera: () => camera } as any
    zoomOut(sigma)
    expect(camera.animatedUnzoom).toHaveBeenCalledWith({ duration: 200 })
  })
})

describe('fitToScreen', () => {
  it('calls camera.animatedReset with duration 300', () => {
    const camera = { animatedReset: vi.fn() }
    const sigma = { getCamera: () => camera } as any
    fitToScreen(sigma)
    expect(camera.animatedReset).toHaveBeenCalledWith({ duration: 300 })
  })
})

describe('toggleFullscreen', () => {
  it('enters fullscreen when not active', () => {
    const container = { requestFullscreen: vi.fn() } as any
    Object.defineProperty(document, 'fullscreenEnabled', { value: true, writable: true, configurable: true })
    Object.defineProperty(document, 'fullscreenElement', { value: null, writable: true, configurable: true })
    toggleFullscreen(container)
    expect(container.requestFullscreen).toHaveBeenCalled()
  })

  it('exits fullscreen when active', () => {
    const exitFn = vi.fn().mockResolvedValue(undefined)
    Object.defineProperty(document, 'fullscreenEnabled', { value: true, writable: true, configurable: true })
    Object.defineProperty(document, 'fullscreenElement', { value: {}, writable: true, configurable: true })
    Object.defineProperty(document, 'exitFullscreen', { value: exitFn, writable: true, configurable: true })
    toggleFullscreen({} as any)
    expect(exitFn).toHaveBeenCalled()
  })

  it('is a no-op when fullscreen API is unavailable', () => {
    const container = { requestFullscreen: vi.fn() } as any
    Object.defineProperty(document, 'fullscreenEnabled', { value: false, writable: true, configurable: true })
    toggleFullscreen(container)
    expect(container.requestFullscreen).not.toHaveBeenCalled()
  })
})

describe('isInputFocused', () => {
  it('returns true for INPUT elements', () => {
    const event = { target: { tagName: 'INPUT' } } as any
    expect(isInputFocused(event)).toBe(true)
  })

  it('returns true for TEXTAREA elements', () => {
    const event = { target: { tagName: 'TEXTAREA' } } as any
    expect(isInputFocused(event)).toBe(true)
  })

  it('returns true for SELECT elements', () => {
    const event = { target: { tagName: 'SELECT' } } as any
    expect(isInputFocused(event)).toBe(true)
  })

  it('returns false for DIV elements', () => {
    const event = { target: { tagName: 'DIV' } } as any
    expect(isInputFocused(event)).toBe(false)
  })

  it('returns false for BUTTON elements', () => {
    const event = { target: { tagName: 'BUTTON' } } as any
    expect(isInputFocused(event)).toBe(false)
  })
})
```

**Acceptance criteria:**
- All 11 test cases pass.
- Camera animation methods are verified with correct durations.
- Fullscreen toggle logic is verified for both enter and exit states.
- Fullscreen API unavailability is handled gracefully.
- `isInputFocused` correctly identifies interactive form elements.

**Failure criteria:**
- Any test fails, indicating action function bug.

---

## Testing Support

| Test file | Test count | Type |
|-----------|-----------|------|
| `__tests__/graph-toolbar.test.ts` | 11 | Unit |
| **Total** | **11** | |

**Run command:** `cd packages/webui && npx vitest run src/lib/components/graph/__tests__/graph-toolbar.test.ts`

## Documentation Requirements

- Export `zoomIn`, `zoomOut`, `fitToScreen`, `toggleFullscreen`, `isInputFocused` from `graph.toolbar.ts`.
- Document keyboard shortcuts in component JSDoc.
- Consumer usage pattern: `<GraphToolbar {graphView} {config} position="top-right" />` alongside `<GraphView bind:this={graphView}>`.
- No new user-facing docs (internal component module).

## Parallelization Notes

- T55.1 (new file `graph.toolbar.ts`) is independent -- can start immediately.
- T55.2 (new file `GraphToolbar.svelte`) depends on T55.1.
- T55.3 modifies `GraphView.svelte` -- one-line addition, minimal conflict potential.
- T55.4 (tests) depends on T55.1 only.
- This phase depends on Phase 54 for `getSigma()`/`getGraph()` exports on `GraphView`. If Phase 48 already includes these (it does), this phase can technically start before Phase 54, but the toolbar should be tested against the final API from Phase 54.
- The FA2 worker import (`graphology-layout-forceatlas2/worker`) is from the same package already installed -- no new dependency needed.
