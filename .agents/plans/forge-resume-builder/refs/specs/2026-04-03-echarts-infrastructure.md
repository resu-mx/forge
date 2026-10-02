# ECharts Infrastructure

**Date:** 2026-04-03
**Spec:** J1 (ECharts Infrastructure)
**Phase:** TBD (next available)
**Builds on:** None (foundational charting library setup)
**Dependencies:** Spec A (Design System — CSS custom properties and `data-theme` attribute for theme integration)
**Blocks:** J2, J3, J4, J5

## Overview

Set up Apache ECharts as the project's charting library with Svelte 5 integration. This spec creates:

1. A tree-shakeable ECharts setup that registers only the chart types used by Forge (pie, sunburst, treemap, custom/Gantt, map/choropleth)
2. A reusable `EChart.svelte` wrapper component with reactive option updates, responsive resizing, theme integration, and proper cleanup
3. A custom ECharts theme definition that maps Spec A's CSS custom properties to ECharts color schemes
4. A centralized registry module for tree-shaking control

No charts are rendered by this spec — it provides the infrastructure for Specs J2-J5 to build on.

## Non-Goals

- Rendering any specific charts (deferred to J2-J5)
- Dashboard layout or page routing
- Data fetching or aggregation logic
- Chart animation customization beyond ECharts defaults
- Chart export (PNG, SVG download)
- Chart accessibility (aria labels, screen reader support) — deferred
- Server-side rendering of charts
- Mobile-specific chart interactions (pinch zoom, etc.)
- ECharts extensions (3D, GL, wordcloud)

---

## 1. Installation

### 1.1 Package

Install `echarts` as a dependency of the webui package:

```bash
cd packages/webui && bun add echarts
```

**Version:** 5.5+ (current stable). The ESM tree-shakeable build is available via `echarts/core`.

### 1.2 Bundle Size Considerations

A full `import * as echarts from 'echarts'` pulls in ~1MB (minified). The tree-shakeable approach via `echarts/core` + individual component imports keeps the bundle under 200KB for the chart types Forge needs:

| Component | Import Path | Approx. Size |
|-----------|------------|-------------|
| Core | `echarts/core` | ~40KB |
| Pie chart | `echarts/charts` → `PieChart` | ~15KB |
| Sunburst chart | `echarts/charts` → `SunburstChart` | ~20KB |
| Treemap chart | `echarts/charts` → `TreemapChart` | ~25KB |
| Custom chart (Gantt) | `echarts/charts` → `CustomChart` | ~10KB |
| Map chart (choropleth) | `echarts/charts` → `MapChart` | ~30KB |
| Tooltip | `echarts/components` → `TooltipComponent` | ~15KB |
| Legend | `echarts/components` → `LegendComponent` | ~10KB |
| Title | `echarts/components` → `TitleComponent` | ~5KB |
| Grid | `echarts/components` → `GridComponent` | ~10KB |
| VisualMap | `echarts/components` → `VisualMapComponent` | ~15KB |
| SVG Renderer | `echarts/renderers` → `SVGRenderer` | ~20KB |

**Total estimate:** ~215KB minified, ~70KB gzipped. This is well within the SaaS target for a charting library.

### 1.3 Renderer Choice

Use `SVGRenderer` instead of the default `CanvasRenderer`:
- Better for small-to-medium datasets (Forge's charts are summaries, not big data)
- Crisper text rendering
- Better print support
- Smaller renderer module
- Better accessibility (DOM elements can be inspected)

For charts with >1000 data points, `CanvasRenderer` can be registered as a fallback in a future spec.

---

## 2. Tree-Shaking Registry

### 2.1 File: `packages/webui/src/lib/components/charts/echarts-registry.ts`

Centralized chart type registration. All chart types and components are imported and registered once here. The `EChart.svelte` wrapper imports from this module instead of from `echarts` directly.

```typescript
import * as echarts from 'echarts/core'

// ── Chart types ──────────────────────────────────────────────────
import { PieChart } from 'echarts/charts'
import { SunburstChart } from 'echarts/charts'
import { TreemapChart } from 'echarts/charts'
import { CustomChart } from 'echarts/charts'
import { MapChart } from 'echarts/charts'

// ── Components ───────────────────────────────────────────────────
import { TooltipComponent } from 'echarts/components'
import { LegendComponent } from 'echarts/components'
import { TitleComponent } from 'echarts/components'
import { GridComponent } from 'echarts/components'
import { VisualMapComponent } from 'echarts/components'
import { DatasetComponent } from 'echarts/components'
import { TransformComponent } from 'echarts/components'

// ── Renderer ─────────────────────────────────────────────────────
import { SVGRenderer } from 'echarts/renderers'

// ── Register all ─────────────────────────────────────────────────
echarts.use([
  PieChart,
  SunburstChart,
  TreemapChart,
  CustomChart,
  MapChart,
  TooltipComponent,
  LegendComponent,
  TitleComponent,
  GridComponent,
  VisualMapComponent,
  DatasetComponent,
  TransformComponent,
  SVGRenderer,
])

export { echarts }
```

**Why centralized?** If a future spec needs a new chart type (e.g., BarChart for J3), it adds a single import + `use()` entry here. No other files change. This also ensures tree-shaking works correctly — unused chart types from `echarts/charts` are not bundled.

**Import pattern for consumers:** Chart pages import `echarts` from this module:
```typescript
import { echarts } from '$lib/components/charts/echarts-registry'
```

---

## 3. Theme Integration

### 3.1 File: `packages/webui/src/lib/components/charts/echarts-theme.ts`

A custom ECharts theme definition that reads CSS custom properties from Spec A's `tokens.css` and builds an ECharts theme object. This ensures charts match the app's light/dark theme.

```typescript
/**
 * Build an ECharts theme object from CSS custom property values.
 * Call this function at render time (not module scope) so it reads
 * the current computed styles from the document root.
 */
export function buildEChartsTheme(): object {
  const root = getComputedStyle(document.documentElement)

  function token(name: string, fallback: string): string {
    return root.getPropertyValue(name).trim() || fallback
  }

  return {
    // ── Color palette ────────────────────────────────────────────
    color: [
      token('--color-chart-1', '#6c63ff'),
      token('--color-chart-2', '#22c55e'),
      token('--color-chart-3', '#f59e0b'),
      token('--color-chart-4', '#ef4444'),
      token('--color-chart-5', '#06b6d4'),
      token('--color-chart-6', '#8b5cf6'),
      token('--color-chart-7', '#ec4899'),
      token('--color-chart-8', '#14b8a6'),
    ],

    // ── Background ───────────────────────────────────────────────
    backgroundColor: 'transparent',

    // ── Title ────────────────────────────────────────────────────
    title: {
      textStyle: {
        color: token('--text-primary', '#1a1a2e'),
        fontSize: 16,
        fontWeight: 600,
        fontFamily: token('--font-sans', 'Inter, system-ui, sans-serif'),
      },
      subtextStyle: {
        color: token('--text-secondary', '#6b7280'),
        fontSize: 12,
        fontFamily: token('--font-sans', 'Inter, system-ui, sans-serif'),
      },
    },

    // ── Legend ────────────────────────────────────────────────────
    legend: {
      textStyle: {
        color: token('--text-primary', '#1a1a2e'),
        fontFamily: token('--font-sans', 'Inter, system-ui, sans-serif'),
      },
    },

    // ── Tooltip ──────────────────────────────────────────────────
    tooltip: {
      backgroundColor: token('--color-surface', '#ffffff'),
      borderColor: token('--border-primary', '#e5e7eb'),
      textStyle: {
        color: token('--text-primary', '#1a1a2e'),
        fontFamily: token('--font-sans', 'Inter, system-ui, sans-serif'),
      },
    },

    // ── Category Axis ────────────────────────────────────────────
    categoryAxis: {
      axisLine: { lineStyle: { color: token('--border-primary', '#e5e7eb') } },
      axisLabel: { color: token('--text-secondary', '#6b7280') },
      splitLine: { lineStyle: { color: token('--border-subtle', '#f3f4f6') } },
    },

    // ── Value Axis ───────────────────────────────────────────────
    valueAxis: {
      axisLine: { lineStyle: { color: token('--border-primary', '#e5e7eb') } },
      axisLabel: { color: token('--text-secondary', '#6b7280') },
      splitLine: { lineStyle: { color: token('--border-subtle', '#f3f4f6') } },
    },
  }
}
```

### 3.2 Chart Token CSS Variables

Spec A's `tokens.css` must include chart-specific color tokens. If Spec A has not yet landed, the `buildEChartsTheme()` function uses fallback values (the second argument to `token()`). When Spec A lands, the following variables should be added to `tokens.css`:

```css
:root {
  /* Chart palette */
  --color-chart-1: #6c63ff;
  --color-chart-2: #22c55e;
  --color-chart-3: #f59e0b;
  --color-chart-4: #ef4444;
  --color-chart-5: #06b6d4;
  --color-chart-6: #8b5cf6;
  --color-chart-7: #ec4899;
  --color-chart-8: #14b8a6;
}

@media (prefers-color-scheme: dark) {
  :root {
    --color-chart-1: #818cf8;
    --color-chart-2: #4ade80;
    --color-chart-3: #fbbf24;
    --color-chart-4: #f87171;
    --color-chart-5: #22d3ee;
    --color-chart-6: #a78bfa;
    --color-chart-7: #f472b6;
    --color-chart-8: #2dd4bf;
  }
}
```

**Integration note:** If Spec A has not been implemented when J1 is built, the chart tokens are added directly to `+layout.svelte` as an interim measure. When Spec A lands, they move to `tokens.css`.

### 3.3 Theme Re-application on Theme Change

When the user toggles between light and dark themes (via Spec A's `data-theme` attribute), the EChart component must re-read CSS tokens and re-apply the theme. This is handled in the `EChart.svelte` component by watching the `data-theme` attribute via `MutationObserver` (see Section 4).

---

## 4. EChart Component

### 4.1 File: `packages/webui/src/lib/components/charts/EChart.svelte`

Reusable wrapper component that handles initialization, reactive updates, responsive resizing, theme integration, and cleanup.

```svelte
<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { echarts } from './echarts-registry'
  import { buildEChartsTheme } from './echarts-theme'
  import type { EChartsOption, ECharts } from 'echarts/core'

  let {
    option,
    height = '400px',
    width = '100%',
    loading = false,
    notMerge = false,
    onChartEvent,
  }: {
    option: EChartsOption
    height?: string
    width?: string
    loading?: boolean
    notMerge?: boolean
    onChartEvent?: Record<string, (params: any) => void>
  } = $props()

  let chartEl: HTMLDivElement
  let chart = $state<ECharts | null>(null)
  let resizeObserver: ResizeObserver | null = null
  let themeObserver: MutationObserver | null = null

  // ── Initialization ──────────────────────────────────────────────

  onMount(() => {
    const theme = buildEChartsTheme()
    chart = echarts.init(chartEl, theme, { renderer: 'svg' })
    chart.setOption(option)

    // ── Event forwarding ──────────────────────────────────────
    if (onChartEvent) {
      for (const [eventName, handler] of Object.entries(onChartEvent)) {
        chart.on(eventName, handler)
      }
    }

    if (loading) {
      chart.showLoading('default', {
        text: '',
        color: getComputedStyle(chartEl).getPropertyValue('--color-primary').trim() || '#6c63ff',
        maskColor: 'rgba(255, 255, 255, 0.7)',
      })
    }

    // ── Responsive resize ───────────────────────────────────────
    resizeObserver = new ResizeObserver(() => {
      chart?.resize()
    })
    resizeObserver.observe(chartEl)

    // ── Theme change listener ───────────────────────────────────
    themeObserver = new MutationObserver((mutations) => {
      for (const mutation of mutations) {
        if (
          mutation.type === 'attributes' &&
          mutation.attributeName === 'data-theme'
        ) {
          rebuildWithTheme()
          break
        }
      }
    })
    themeObserver.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['data-theme'],
    })

    // ── OS-level dark/light mode detection ────────────────────
    matchMedia('(prefers-color-scheme: dark)').addEventListener('change', rebuildWithTheme)
  })

  // ── Reactive option updates ─────────────────────────────────────

  $effect(() => {
    if (chart) {
      chart.setOption(option, notMerge)
    }
  })

  // ── Loading state ───────────────────────────────────────────────

  $effect(() => {
    if (!chart) return
    if (loading) {
      chart.showLoading('default', {
        text: '',
        color: getComputedStyle(chartEl).getPropertyValue('--color-primary').trim() || '#6c63ff',
        maskColor: 'rgba(255, 255, 255, 0.7)',
      })
    } else {
      chart.hideLoading()
    }
  })

  // ── Theme rebuild ───────────────────────────────────────────────

  function rebuildWithTheme() {
    if (!chart) return
    const currentOption = chart.getOption()
    resizeObserver?.disconnect()
    chart.dispose()
    const theme = buildEChartsTheme()
    chart = echarts.init(chartEl, theme, { renderer: 'svg' })
    chart.setOption(currentOption as EChartsOption)
    resizeObserver?.observe(chartEl)

    // Re-register event handlers after re-init
    if (onChartEvent) {
      for (const [eventName, handler] of Object.entries(onChartEvent)) {
        chart.on(eventName, handler)
      }
    }
  }

  // ── Cleanup ─────────────────────────────────────────────────────

  onDestroy(() => {
    resizeObserver?.disconnect()
    themeObserver?.disconnect()
    chart?.dispose()
    chart = null
  })
</script>

<div
  bind:this={chartEl}
  class="echart-container"
  style:height
  style:width
></div>

<style>
  .echart-container {
    min-height: 200px;
  }
</style>
```

### 4.2 Props Interface

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `option` | `EChartsOption` | *(required)* | The ECharts configuration object. Reactive — changes trigger `setOption()`. |
| `height` | `string` | `'400px'` | CSS height value for the chart container. |
| `width` | `string` | `'100%'` | CSS width value for the chart container. |
| `loading` | `boolean` | `false` | When true, shows ECharts' built-in loading spinner overlay. |
| `notMerge` | `boolean` | `false` | When true, `setOption()` replaces the entire option instead of merging. Use for charts that change structure (e.g., switching between pie and sunburst). |
| `onChartEvent` | `Record<string, (params: any) => void>` | `undefined` | Map of ECharts event names to handler functions. In `onMount`, the component iterates entries and calls `chart.on(eventName, handler)` for each. This allows J2/J3/J4/J5 to listen for chart events without modifying J1's component. |

### 4.3 Lifecycle

1. **Mount:** `echarts.init()` with custom theme + SVG renderer. Apply initial `option`. Register `onChartEvent` handlers if provided. Set up `ResizeObserver` and `MutationObserver`. Add `matchMedia('(prefers-color-scheme: dark)')` listener for OS-level dark/light mode detection.
2. **Option change:** `$effect` watches `option` prop and calls `chart.setOption()` reactively. Note: `chart` is declared as `$state<ECharts | null>(null)` so this effect properly re-tracks when `chart` transitions from `null` to initialized.
3. **Loading change:** `$effect` watches `loading` prop and calls `showLoading()` / `hideLoading()`. Loading spinner color is read from `--color-primary` CSS token with `#6c63ff` fallback.
4. **Theme change:** `MutationObserver` on `document.documentElement` watches `data-theme` attribute, plus `matchMedia` listener detects OS-level dark/light switches. On change, `rebuildWithTheme()` disconnects `resizeObserver`, disposes the chart, rebuilds theme, re-initializes, re-applies the current option, re-registers event handlers, and reconnects the `resizeObserver`.
5. **Resize:** `ResizeObserver` on the container div calls `chart.resize()` whenever the container dimensions change (e.g., sidebar toggle, window resize, split-panel drag).
6. **Destroy:** `onDestroy` disconnects both observers and calls `chart.dispose()` to free memory.

### 4.4 Event Forwarding

The `onChartEvent` prop accepts a `Record<string, (params: any) => void>` mapping ECharts event names to handler functions. During `onMount`, the component iterates the entries and calls `chart.on(eventName, handler)` for each. This allows consuming components (J2, J3, J4, J5) to handle chart events (e.g., click, mouseover) without modifying `EChart.svelte`. ECharts' `dispose()` automatically removes all event listeners, so no explicit `off()` is needed in `onDestroy`.

### 4.5 SVG Renderer + lineDash Note

SVG renderer applies `lineDash` to the entire shape border, not just one edge. For J4's dashed-right-edge effect on active Gantt bars, use a two-shape group (solid `rect` + dashed `line`) in the custom `renderItem`, or accept a fully dashed border.

### 4.6 Type Import Path

Import `EChartsOption` from `echarts/core` (not `echarts`) to match the tree-shaken module surface. This ensures TypeScript sees the correct types for the registered chart/component subset.

---

## 5. Files to Create

| File | Purpose |
|------|---------|
| `packages/webui/src/lib/components/charts/EChart.svelte` | Reusable ECharts wrapper component |
| `packages/webui/src/lib/components/charts/echarts-registry.ts` | Centralized chart type registration (tree-shaking) |
| `packages/webui/src/lib/components/charts/echarts-theme.ts` | Custom theme mapping CSS tokens to ECharts colors |

## 6. Files to Modify

| File | Change |
|------|--------|
| `packages/webui/package.json` | Add `echarts` dependency |
| `packages/webui/src/lib/styles/tokens.css` (if Spec A exists) OR `packages/webui/src/routes/+layout.svelte` (interim) | Add `--color-chart-1` through `--color-chart-8` CSS custom properties with light/dark variants |

---

## 7. Testing

### 7.1 Registry Tests

- Importing `echarts` from `echarts-registry.ts` provides a valid ECharts instance
- `echarts.init()` succeeds with SVG renderer
- Attempting to create a chart type not registered (e.g., `BarChart`) does not break — it simply renders nothing (graceful degradation)
- Registered chart types (`PieChart`, `SunburstChart`, `TreemapChart`, `CustomChart`, `MapChart`) can be used in `setOption()`

### 7.2 Theme Tests

- `buildEChartsTheme()` returns an object with `color`, `title`, `legend`, `tooltip`, `categoryAxis`, `valueAxis` keys
- `color` array has 8 entries
- When CSS custom properties are set on `:root`, `buildEChartsTheme()` reads them (not hardcoded fallbacks)
- When CSS custom properties are NOT set, fallback values are used

### 7.3 Component Tests

- `EChart.svelte` renders a `<div>` with the specified `height` and `width`
- Passing an `option` prop initializes the chart (no errors in console)
- Changing `option` prop reactively updates the chart (calls `setOption`)
- Setting `loading = true` shows the loading overlay; `false` hides it
- Component cleanup: after unmount, `chart.dispose()` is called (no memory leaks)
- Container resize triggers `chart.resize()` (verify via mock `ResizeObserver`)
- Theme change (setting `data-theme` attribute) triggers chart re-initialization with new theme

### 7.4 Bundle Size Verification

- Build the webui with `bun run build` and check that the `echarts`-related chunk is under 250KB (minified)
- Verify that unregistered chart types (e.g., `BarChart`, `LineChart`) are NOT in the bundle via source map analysis

---

## 8. Acceptance Criteria

1. `echarts` is installed as a dependency in `packages/webui/package.json`
2. `echarts-registry.ts` registers exactly 5 chart types (Pie, Sunburst, Treemap, Custom, Map) + 7 components (Tooltip, Legend, Title, Grid, VisualMap, Dataset, Transform) + SVG renderer
3. `echarts-registry.ts` exports a configured `echarts` instance that consumers import instead of importing `echarts` directly
4. `echarts-theme.ts` exports `buildEChartsTheme()` that reads CSS custom properties with sensible fallbacks
5. Theme includes color palette (8 colors), title style, legend style, tooltip style, and axis styles
6. Light and dark theme variants are supported via CSS custom properties (`--color-chart-*`)
7. `EChart.svelte` accepts `option`, `height`, `width`, `loading`, `notMerge`, and `onChartEvent` props
8. `EChart.svelte` initializes the chart on mount with SVG renderer and custom theme; `chart` is `$state` so the options `$effect` re-tracks when chart transitions from null to initialized
9. `EChart.svelte` reactively updates when `option` prop changes
10. `EChart.svelte` handles `loading` prop to show/hide loading overlay
11. `EChart.svelte` uses `ResizeObserver` to resize the chart when the container changes
12. `EChart.svelte` uses `MutationObserver` on `document.documentElement[data-theme]` to re-apply theme on theme toggle
13. `EChart.svelte` uses `matchMedia('(prefers-color-scheme: dark)')` listener to detect OS-level dark/light switches without a `data-theme` attribute change
14. `rebuildWithTheme` disconnects `resizeObserver` before `chart.dispose()` and reconnects after re-init to avoid ResizeObserver race conditions
15. `EChart.svelte` calls `chart.dispose()` and disconnects all observers on destroy
16. `onChartEvent` prop iterates entries and calls `chart.on(eventName, handler)` for each, enabling J2-J5 to handle events without modifying J1's component
17. Loading spinner color is read from CSS token `--color-primary` with `#6c63ff` fallback (not hardcoded)
18. `EChartsOption` is imported from `echarts/core` (not `echarts`) to match the tree-shaken module surface
19. Tree-shakeable build keeps the ECharts bundle under 250KB minified
20. No charts are rendered by this spec — it is infrastructure only
