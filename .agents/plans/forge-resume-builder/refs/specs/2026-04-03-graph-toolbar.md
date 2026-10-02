# Graph Toolbar

**Date:** 2026-04-03
**Spec:** H6 (Graph Toolbar)
**Phase:** TBD (next available)
**Depends on:** H1 (Generic GraphView Component)

## Overview

The generic `GraphView` from H1 has no user-facing controls for zoom, layout, or viewport reset. Users must rely on scroll-wheel zoom and have no way to pause the ForceAtlas2 physics simulation, reset the viewport, or toggle fullscreen. This spec adds a floating toolbar overlay with buttons for these common graph interactions.

The toolbar is a separate Svelte component that sits on top of the `GraphView` container. It communicates with `GraphView` through the component's exported methods (`getSigma()`, `getGraph()`, `focusNode()`) and direct Sigma camera API calls.

## Non-Goals

- **Custom layout parameter tuning UI:** No sliders for gravity, scaling ratio, etc. Users pick from presets.
- **Export graph as image:** No PNG/SVG export button. Deferred to a future spec.
- **Edge-specific controls:** No edge visibility toggle or edge color picker.
- **Toolbar customization:** No drag-to-reorder buttons, no user preferences for which buttons appear.
- **Mobile-specific controls:** Touch gestures (pinch-to-zoom) are handled by Sigma natively. The toolbar targets mouse/keyboard users.

---

## 1. Toolbar Buttons

### 1.1 Button Set

| Button | Icon | Action | Keyboard Shortcut |
|--------|------|--------|-------------------|
| Zoom In | `+` | Zoom camera in by ratio step | `Ctrl+=` / `Cmd+=` |
| Zoom Out | `-` | Zoom camera out by ratio step | `Ctrl+-` / `Cmd+-` |
| Fit to Screen | `⊞` (fit icon) | Reset camera to show all nodes | `Ctrl+0` / `Cmd+0` |
| Toggle Layout | `▶`/`⏸` (play/pause) | Start/stop ForceAtlas2 simulation | `Space` (when graph focused) |
| Reset | `↺` (reset icon) | Reset zoom, center graph, restart layout | — |
| Fullscreen | `⛶` (expand icon) | Toggle fullscreen on the graph container | `F11` or `F` (when graph focused) |

### 1.2 Zoom Controls

Zoom uses the Sigma camera's `animatedZoom` and `animatedUnzoom` methods:

```typescript
const ZOOM_STEP = 0.5  // camera ratio multiplier per click

function zoomIn(sigma: Sigma) {
  const camera = sigma.getCamera()
  camera.animatedZoom({ duration: 200 })
}

function zoomOut(sigma: Sigma) {
  const camera = sigma.getCamera()
  camera.animatedUnzoom({ duration: 200 })
}

function fitToScreen(sigma: Sigma) {
  const camera = sigma.getCamera()
  camera.animatedReset({ duration: 300 })
}
```

### 1.3 Layout Toggle (ForceAtlas2)

The toolbar needs the ability to start and stop a live ForceAtlas2 simulation. H1 runs layout synchronously during graph construction (one-shot `forceAtlas2.assign()`). For interactive layout toggling, the component switches to the `graphology-layout-forceatlas2/worker` supervisor that runs the simulation in a Web Worker:

```typescript
import FA2Layout from 'graphology-layout-forceatlas2/worker'

let fa2Instance: FA2Layout | null = null
let layoutRunning = $state(false)

function toggleLayout(graph: Graph, config: GraphConfig) {
  if (layoutRunning && fa2Instance) {
    fa2Instance.stop()
    layoutRunning = false
    return
  }

  if (!fa2Instance) {
    fa2Instance = new FA2Layout(graph, {
      settings: {
        gravity: config.forces.gravity,
        scalingRatio: config.forces.scalingRatio,
        slowDown: config.forces.slowDown,
        barnesHutOptimize: config.forces.barnesHutOptimize
          ?? graph.order > (config.forces.barnesHutThreshold ?? 100),
      },
    })
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

// When the `graph` state changes in the parent, the toolbar's `fa2Instance`
// must be killed and re-created with the new graph. Without this, the
// fa2Instance holds a reference to the old graph and layout updates are lost.
$effect(() => {
  const currentGraph = graphView.getGraph()
  // Kill stale instance when graph reference changes
  if (fa2Instance) {
    fa2Instance.kill()
    fa2Instance = null
    layoutRunning = false
  }
})
```

The toolbar button shows `▶` (play) when layout is stopped and `⏸` (pause) when running.

> **Worker cleanup on unmount:** On component unmount (`onDestroy`), call `fa2Instance?.kill()` to stop the Web Worker and prevent leaks:
> ```typescript
> onDestroy(() => {
>   fa2Instance?.kill()
>   fa2Instance = null
> })
> ```

### 1.4 Reset Button

Reset performs three actions in sequence:

1. Stop the ForceAtlas2 simulation if running.
2. Re-run the one-shot layout (`forceAtlas2.assign()`) to reset node positions.
3. Reset the camera to show all nodes (`camera.animatedReset()`).

```typescript
async function resetGraph(sigma: Sigma, graph: Graph, config: GraphConfig) {
  stopLayout()

  // Re-run one-shot layout
  const forceAtlas2 = await import('graphology-layout-forceatlas2')
  forceAtlas2.default.assign(graph, {
    iterations: config.forces.iterations ?? 100,
    settings: {
      gravity: config.forces.gravity,
      scalingRatio: config.forces.scalingRatio,
      slowDown: config.forces.slowDown,
    },
  })

  sigma.refresh()
  sigma.getCamera().animatedReset({ duration: 300 })
}
```

### 1.5 Fullscreen Toggle

Uses the browser Fullscreen API on the graph container element:

```typescript
function toggleFullscreen(container: HTMLElement) {
  if (document.fullscreenElement) {
    document.exitFullscreen()
  } else {
    container.requestFullscreen()
  }
}
```

The component listens for the `fullscreenchange` event to update the button icon.

### 1.6 Layout Presets (Optional)

If H1's preset configs (`DENSE_GRAPH_CONFIG`, `TREE_GRAPH_CONFIG`, `SMALL_GRAPH_CONFIG`) are available, the toolbar can include a preset selector — a small dropdown or button group that switches the force parameters and re-runs layout:

```typescript
const PRESETS = [
  { label: 'Default', config: {} },
  { label: 'Dense', config: DENSE_GRAPH_CONFIG },
  { label: 'Tree', config: TREE_GRAPH_CONFIG },
  { label: 'Small', config: SMALL_GRAPH_CONFIG },
] as const
```

This is optional — the toolbar works without presets. The preset buttons appear only if the consumer passes a `showPresets: true` prop.

---

## 2. Component: `GraphToolbar.svelte`

### 2.1 Props

```typescript
interface GraphToolbarProps {
  /** Reference to the GraphView component (for getSigma/getGraph/focusNode) */
  graphView: GraphView
  /** The graph config (for layout preset parameters) */
  config: GraphConfig
  /** Position of the toolbar */
  position?: 'top-right' | 'top-left' | 'bottom-right' | 'bottom-left'
  /** Show layout preset buttons */
  showPresets?: boolean
  /** Show fullscreen toggle */
  showFullscreen?: boolean
}
```

### 2.2 UI Layout

The toolbar renders as a floating overlay inside the graph container:

```
+--------------------------------------------------+
|                                    [+][-][⊞]     |  <- zoom group
|                                    [▶][↺][⛶]    |  <- control group
|                                                   |
|              (graph canvas)                       |
|                                                   |
|                                                   |
+--------------------------------------------------+
```

Buttons are arranged in a vertical stack or horizontal row depending on the position. The default is vertical stack at `top-right`.

### 2.3 Styling

```css
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

.graph-toolbar.bottom-right {
  bottom: 8px;
  right: 8px;
}

/* ... other positions ... */

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

.toolbar-btn:hover {
  background: var(--color-hover, #f3f4f6);
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
```

### 2.4 Keyboard Shortcuts

Keyboard shortcuts are active when the graph container (or any element within it) has focus. The toolbar registers event listeners on the graph container element:

```typescript
function handleKeydown(e: KeyboardEvent) {
  const mod = e.metaKey || e.ctrlKey

  if (mod && e.key === '=') { e.preventDefault(); zoomIn(sigma) }
  if (mod && e.key === '-') { e.preventDefault(); zoomOut(sigma) }
  if (mod && e.key === '0') { e.preventDefault(); fitToScreen(sigma) }
  // Space only fires when the graph container has focus (check
  // document.activeElement is inside the graph container). Otherwise,
  // Space scrolls the page as normal.
  if (e.key === ' ' && !isInputFocused(e) && container?.contains(document.activeElement)) { e.preventDefault(); toggleLayout(graph, config) }
  if (e.key === 'f' && !isInputFocused(e)) { e.preventDefault(); toggleFullscreen(container) }
}

/**
 * Check if the keyboard event originates from an input/textarea
 * to avoid hijacking typing.
 */
function isInputFocused(e: KeyboardEvent): boolean {
  const tag = (e.target as HTMLElement)?.tagName
  return tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT'
}
```

---

## 3. Integration with GraphView

The toolbar needs access to the Sigma instance and graphology Graph. This is achieved through the exported methods added in H5:

```typescript
// GraphView.svelte exports (from H5 spec):
export function getSigma(): Sigma | null
export function getGraph(): Graph | null
export function focusNode(nodeId: string, ratio?: number, duration?: number): boolean
```

The toolbar also needs access to the graph container element for fullscreen. This is exposed via a new export:

```typescript
// Addition to GraphView.svelte:
export function getContainer(): HTMLElement | null { return container }
```

### 3.1 Consumer Usage

```svelte
<script>
  import GraphView from '$lib/components/graph/GraphView.svelte'
  import GraphToolbar from '$lib/components/graph/GraphToolbar.svelte'

  let graphView: GraphView
</script>

<div class="graph-wrapper" style="position: relative; width: 100%; height: 500px;">
  <GraphView bind:this={graphView} {nodes} {edges} {config} />
  <GraphToolbar {graphView} {config} position="top-right" />
</div>
```

---

## 4. Files to Create

| File | Description |
|------|-------------|
| `packages/webui/src/lib/components/graph/GraphToolbar.svelte` | Floating toolbar component with zoom, layout, reset, fullscreen controls |
| `packages/webui/src/lib/components/graph/graph.toolbar.ts` | Toolbar action functions (`zoomIn`, `zoomOut`, `fitToScreen`, `toggleLayout`, `resetGraph`, `toggleFullscreen`) |

## 5. Files to Modify

| File | Description |
|------|-------------|
| `packages/webui/src/lib/components/graph/GraphView.svelte` | Add `getContainer()` export; ensure `getSigma()` and `getGraph()` exports exist (may already exist from H5) |

---

## 6. Testing Approach

### 6.1 Unit Tests

**File:** `packages/webui/src/lib/components/graph/__tests__/graph-toolbar.test.ts`

```typescript
import { describe, it, expect, vi } from 'vitest'

describe('toolbar actions', () => {
  it('zoomIn calls camera.animatedZoom', () => {
    const camera = { animatedZoom: vi.fn() }
    const sigma = { getCamera: () => camera }
    zoomIn(sigma as any)
    expect(camera.animatedZoom).toHaveBeenCalledWith({ duration: 200 })
  })

  it('zoomOut calls camera.animatedUnzoom', () => {
    const camera = { animatedUnzoom: vi.fn() }
    const sigma = { getCamera: () => camera }
    zoomOut(sigma as any)
    expect(camera.animatedUnzoom).toHaveBeenCalledWith({ duration: 200 })
  })

  it('fitToScreen calls camera.animatedReset', () => {
    const camera = { animatedReset: vi.fn() }
    const sigma = { getCamera: () => camera }
    fitToScreen(sigma as any)
    expect(camera.animatedReset).toHaveBeenCalledWith({ duration: 300 })
  })

  it('toggleFullscreen enters fullscreen when not active', () => {
    const container = { requestFullscreen: vi.fn() }
    Object.defineProperty(document, 'fullscreenElement', { value: null, writable: true })
    toggleFullscreen(container as any)
    expect(container.requestFullscreen).toHaveBeenCalled()
  })

  it('toggleFullscreen exits fullscreen when active', () => {
    const exitFn = vi.fn()
    Object.defineProperty(document, 'fullscreenElement', { value: {}, writable: true })
    Object.defineProperty(document, 'exitFullscreen', { value: exitFn, writable: true })
    toggleFullscreen({} as any)
    expect(exitFn).toHaveBeenCalled()
  })
})
```

### 6.2 Keyboard Shortcut Tests

Test the `handleKeydown` function in isolation:

- `Ctrl+=` triggers zoom in.
- `Ctrl+-` triggers zoom out.
- `Ctrl+0` triggers fit to screen.
- `Space` toggles layout (when not in an input field).
- `Space` does NOT toggle layout when target is an `<input>`.
- `f` toggles fullscreen (when not in an input field).

### 6.3 Component Tests

**File:** `packages/webui/src/lib/components/graph/__tests__/GraphToolbar.test.ts`

- Renders all expected buttons (zoom in, zoom out, fit, toggle layout, reset, fullscreen).
- Layout toggle button shows play icon when stopped, pause icon when running.
- Fullscreen button is hidden when `showFullscreen` is false.
- Preset buttons are hidden when `showPresets` is false.
- Buttons are disabled when `graphView.getSigma()` returns null (graph not yet initialized).

---

## 7. Acceptance Criteria

### Zoom controls
- [ ] Zoom in button increases zoom level with animation (200ms)
- [ ] Zoom out button decreases zoom level with animation (200ms)
- [ ] Fit to screen resets camera to show all nodes with animation (300ms)
- [ ] Zoom controls work via keyboard shortcuts (`Ctrl/Cmd + =/-/0`)

### Layout toggle
- [ ] Play button starts ForceAtlas2 live simulation (nodes move)
- [ ] Pause button stops the simulation (nodes freeze)
- [ ] Button icon toggles between play and pause states
- [ ] Layout uses Web Worker (does not block UI thread)
- [ ] Space bar toggles layout when graph is focused
- [ ] Space bar does NOT toggle layout when an input field is focused

### Reset
- [ ] Reset stops the live simulation if running
- [ ] Reset re-runs one-shot layout to recompute node positions
- [ ] Reset resets camera to show all nodes
- [ ] Graph returns to its initial visual state after reset

### Fullscreen
- [ ] Fullscreen button enters fullscreen mode for the graph container
- [ ] Pressing fullscreen again (or Escape) exits fullscreen
- [ ] Graph resizes correctly within fullscreen
- [ ] Button icon updates to reflect fullscreen state

### Layout presets (optional)
- [ ] Preset buttons appear when `showPresets` is true
- [ ] Selecting a preset re-runs layout with the preset's force parameters
- [ ] Active preset is visually indicated

### Toolbar styling
- [ ] Toolbar renders as a floating overlay at the configured position
- [ ] Toolbar has semi-transparent background with backdrop blur
- [ ] Buttons have hover and active states
- [ ] Toolbar does not obscure the graph significantly (compact size)

### Integration
- [ ] Toolbar works with any `GraphView` instance (not coupled to specific data)
- [ ] Toolbar buttons are disabled when Sigma is not yet initialized
- [ ] Toolbar cleanup stops any running layout simulation on unmount

### Tests
- [ ] Toolbar action unit tests pass (5 cases)
- [ ] Keyboard shortcut tests pass (6 cases)
- [ ] Component rendering tests pass

---

## 8. Dependencies

- **Runtime:** `graphology-layout-forceatlas2` (already installed), `graphology-layout-forceatlas2/worker` (Web Worker supervisor — included in the same package)
- **Spec dependencies:** H1 (Generic GraphView Component), H5 (Graph Search -- for `getSigma()`, `getGraph()` exports)
- **Blocked by:** H1 and H5 (the toolbar requires `getSigma()` and `getGraph()` exports which are defined in H5)
- **Blocks:** No other specs directly
