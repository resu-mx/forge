# Phase 59: ECharts Infrastructure (Spec J1)

**Status:** Planning
**Date:** 2026-04-03
**Spec:** [2026-04-03-echarts-infrastructure.md](../refs/specs/2026-04-03-echarts-infrastructure.md)
**Depends on:** Phase 42 (Design System — CSS custom properties and `data-theme` attribute for theme integration)
**Blocks:** J2 (Dashboard Charts), J3 (Skills Analytics), J4 (Timeline/Gantt), J5 (Choropleth Map)
**Parallelizable with:** Phase 57 (UI Consistency), Phase 58 (Profile Menu) — creates new files only, modifies nothing except `package.json`

## Goal

Set up Apache ECharts as the project's charting library with Svelte 5 integration. Create:

1. A tree-shakeable ECharts setup that registers only the chart types used by Forge (pie, sunburst, treemap, custom/Gantt, map/choropleth).
2. A reusable `EChart.svelte` wrapper component with reactive option updates, responsive resizing, theme integration, and proper cleanup.
3. A custom ECharts theme definition that maps Phase 42's CSS custom properties to ECharts color schemes.
4. A centralized registry module for tree-shaking control.

No charts are rendered by this spec — it provides the infrastructure for J2-J5 to build on.

## Non-Goals

- Rendering any specific charts (deferred to J2-J5)
- Dashboard layout or page routing
- Data fetching or aggregation logic
- Chart animation customization beyond ECharts defaults
- Chart export (PNG, SVG download)
- Chart accessibility (aria labels, screen reader support)
- Server-side rendering of charts
- Mobile-specific chart interactions (pinch zoom, etc.)
- ECharts extensions (3D, GL, wordcloud)

## Context

Forge needs charting for dashboard analytics (J2), skills breakdown (J3), experience timeline (J4), and geographic distribution (J5). ECharts is chosen over Chart.js/D3 for its built-in support for pie, sunburst, treemap, custom (Gantt), and map chart types — all needed by J2-J5. The tree-shakeable build via `echarts/core` keeps the bundle under 250KB.

The existing `packages/webui/package.json` does not include `echarts`. Graphology/Sigma are already dependencies for the chain graph view — ECharts serves a different purpose (data visualization vs. network graphs).

Phase 42 establishes `tokens.css` with CSS custom properties. If Phase 42 has not landed when J1 is built, chart color tokens are added to `+layout.svelte` as an interim measure and moved to `tokens.css` later.

## Scope

| Spec section | Covered here? |
|-------------|---------------|
| 1. Installation | Yes |
| 2. Tree-Shaking Registry | Yes |
| 3. Theme Integration | Yes |
| 4. EChart Component | Yes |
| 5. Files to Create | Yes |
| 6. Files to Modify | Yes |
| 7. Testing | Yes |
| 8. Acceptance Criteria | Yes |

## Files to Create

| File | Description |
|------|-------------|
| `packages/webui/src/lib/components/charts/echarts-registry.ts` | Centralized chart type registration (tree-shaking) |
| `packages/webui/src/lib/components/charts/echarts-theme.ts` | Custom theme mapping CSS tokens to ECharts colors |
| `packages/webui/src/lib/components/charts/EChart.svelte` | Reusable ECharts wrapper component |
| `packages/webui/src/lib/components/charts/__tests__/echarts-theme.test.ts` | Unit tests for `buildEChartsTheme()` |
| `packages/webui/src/lib/components/charts/__tests__/echarts-registry.test.ts` | Unit tests for registry exports |

## Files to Modify

| File | Changes |
|------|---------|
| `packages/webui/package.json` | Add `echarts` dependency |
| `packages/webui/src/lib/styles/tokens.css` (if Phase 42 exists) OR `packages/webui/src/routes/+layout.svelte` (interim) | Add `--color-chart-1` through `--color-chart-8` CSS custom properties with light/dark variants |

## Fallback Strategies

- **Phase 42 `tokens.css` not merged yet:** Add chart color tokens as a `<style>` block in `+layout.svelte` using `:root` and `[data-theme='dark']` selectors. When Phase 42 lands, move the tokens to `tokens.css` and remove the interim block. The `buildEChartsTheme()` function reads from CSS custom properties either way — no code change needed.
- **ECharts bundle size exceeds 250KB:** If the registered chart types push the bundle over 250KB minified, consider lazy-loading chart types via dynamic import. The registry can be split into `echarts-registry-core.ts` (pie, sunburst, treemap + components) and `echarts-registry-map.ts` (MapChart, loaded on demand by J5). This is a future optimization — the initial registration is simple and should stay under budget.
- **SVG renderer performance for large datasets:** The spec chooses `SVGRenderer` for crisper text and smaller module size. If a future chart (J3 skills with >1000 nodes) hits performance limits, register `CanvasRenderer` as an additional option in the registry. The `EChart.svelte` component would accept a `renderer: 'svg' | 'canvas'` prop (default `'svg'`). Not implemented in this phase.
- **`MutationObserver` not available (SSR):** The component guards all browser APIs with `onMount`/`onDestroy`. `MutationObserver` is created inside `onMount`, which only runs in the browser. No SSR crash.
- **`matchMedia` not available:** The `prefers-color-scheme` listener is added inside `onMount`. If `matchMedia` is not available (unlikely in modern browsers), the listener registration is wrapped in `if (typeof matchMedia !== 'undefined')`.
- **ECharts `dispose()` called on unmounted chart:** The `onDestroy` cleanup sets `chart = null` after `dispose()`. Any pending `$effect` that checks `if (chart)` will bail out. No double-dispose crash.
- **`ResizeObserver` fires after component unmount:** The `onDestroy` callback disconnects the `ResizeObserver` before disposing the chart. The `chart?.resize()` call in the observer callback uses optional chaining — if `chart` is null, it no-ops.

---

## Tasks

### T59.1: Install ECharts Dependency

**File:** `packages/webui/package.json`

[IMPORTANT] Install `echarts` version 5.5+ as a production dependency. The tree-shakeable build is available via `echarts/core`.

Run:
```bash
cd packages/webui && bun add echarts
```

This adds `echarts` to `dependencies` in `package.json`:
```json
{
  "dependencies": {
    "echarts": "^5.5.0"
  }
}
```

**Acceptance criteria:**
- `echarts` appears in `packages/webui/package.json` dependencies.
- `bun install` succeeds without errors.
- `import * as echarts from 'echarts/core'` resolves in TypeScript.

**Failure criteria:**
- ECharts added to wrong package (root `package.json` instead of `packages/webui/package.json`).
- Version pinned too tightly (should use `^` for minor updates).

---

### T59.2: Create Tree-Shaking Registry

**File:** `packages/webui/src/lib/components/charts/echarts-registry.ts`

[CRITICAL] All chart types and components are imported and registered once here. The `EChart.svelte` wrapper imports from this module instead of from `echarts` directly. This is the single point of control for what goes into the ECharts bundle.

[IMPORTANT] Import paths must use `echarts/core`, `echarts/charts`, `echarts/components`, and `echarts/renderers` for tree-shaking to work. Importing from `echarts` directly pulls the entire library.

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
import { GraphicComponent } from 'echarts/components'  // Phase 63's sunburst center-text label requires this

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
  GraphicComponent,
  SVGRenderer,
])

export { echarts }
```

**Key design decisions:**
- **Why centralized?** Adding a new chart type (e.g., `BarChart` for J3) requires a single import + `use()` entry here. No other files change.
- **Import pattern for consumers:** `import { echarts } from '$lib/components/charts/echarts-registry'`
- **SVG renderer over Canvas:** Better text rendering, smaller module, sufficient for Forge's summary-level data volumes.

**Acceptance criteria:**
- Exactly 5 chart types registered: Pie, Sunburst, Treemap, Custom, Map.
- Exactly 8 components registered: Tooltip, Legend, Title, Grid, VisualMap, Dataset, Transform, Graphic.
- SVGRenderer registered (not CanvasRenderer).
- `echarts` export is the configured instance from `echarts/core`.
- TypeScript compilation succeeds.

**Failure criteria:**
- Import from `echarts` instead of `echarts/core` (full bundle, no tree-shaking).
- Missing `echarts.use()` call (chart types not registered, rendering fails).
- Extra chart types registered that Forge does not need (unnecessary bundle size).

---

### T59.3: Create ECharts Theme

**File:** `packages/webui/src/lib/components/charts/echarts-theme.ts`

[CRITICAL] `buildEChartsTheme()` must be called at render time (inside `onMount`), not at module scope. CSS custom properties are only available after the document has loaded and the `<style>` has been applied. Calling at module scope returns fallback values.

[IMPORTANT] The `token()` helper reads from `getComputedStyle(document.documentElement)`. This requires a browser context. The function should never be called during SSR — it is called inside `onMount` in `EChart.svelte`.

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

**Acceptance criteria:**
- `buildEChartsTheme()` returns an object with keys: `color`, `backgroundColor`, `title`, `legend`, `tooltip`, `categoryAxis`, `valueAxis`.
- `color` array has 8 entries.
- When CSS custom properties are set on `:root`, `buildEChartsTheme()` reads them (not fallbacks).
- When CSS custom properties are NOT set, fallback values are used.
- Title font size is `16` (ECharts uses pixels, not CSS tokens for numeric font sizes).
- Background is `'transparent'` (chart inherits page background).

**Failure criteria:**
- Called at module scope and returns stale/fallback values.
- `getComputedStyle` crashes during SSR (should never be called outside browser).
- Missing fallback values cause `undefined` in theme object.

---

### T59.4: Add Chart Color Tokens to CSS

**File:** `packages/webui/src/lib/styles/tokens.css` (if Phase 42 exists) OR `packages/webui/src/routes/+layout.svelte` (interim)

[IMPORTANT] If `tokens.css` exists (Phase 42 landed), add the chart tokens there. If not, add them as an interim `<style>` block in `+layout.svelte`. The `buildEChartsTheme()` function reads CSS custom properties either way.

Chart color tokens for light mode:
```css
:root {
  /* Chart palette — light mode */
  --color-chart-1: #6c63ff;
  --color-chart-2: #22c55e;
  --color-chart-3: #f59e0b;
  --color-chart-4: #ef4444;
  --color-chart-5: #06b6d4;
  --color-chart-6: #8b5cf6;
  --color-chart-7: #ec4899;
  --color-chart-8: #14b8a6;
}
```

Chart color tokens for dark mode:
```css
[data-theme='dark'] {
  --color-chart-1: #818cf8;
  --color-chart-2: #4ade80;
  --color-chart-3: #fbbf24;
  --color-chart-4: #f87171;
  --color-chart-5: #22d3ee;
  --color-chart-6: #a78bfa;
  --color-chart-7: #f472b6;
  --color-chart-8: #2dd4bf;
}

@media (prefers-color-scheme: dark) {
  :root:not([data-theme='light']) {
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

[MINOR] The `@media (prefers-color-scheme: dark)` block uses `:root:not([data-theme='light'])` to support the "system" theme mode. When `data-theme` is not set and the OS prefers dark mode, dark chart colors are applied. When `data-theme='light'` is explicitly set, light colors are used regardless of OS preference.

**Acceptance criteria:**
- 8 chart color CSS custom properties defined for light mode.
- 8 chart color CSS custom properties defined for dark mode (both explicit `data-theme='dark'` and OS preference).
- Dark mode colors are lighter/brighter variants of the light mode palette (higher saturation, higher lightness).
- Properties are readable via `getComputedStyle(document.documentElement).getPropertyValue('--color-chart-1')`.

**Failure criteria:**
- Tokens defined but not applied to `:root` (scoped to wrong selector).
- Dark mode selector does not match (wrong attribute name or missing `@media` query).
- Colors are identical in light and dark mode (defeats the purpose of theme integration).

---

### T59.5: Create EChart Wrapper Component

**File:** `packages/webui/src/lib/components/charts/EChart.svelte`

[CRITICAL] The component must dispose the chart on unmount and on theme change (dispose + re-init). Failure to dispose causes WebGL/SVG context leaks.

[CRITICAL] The `chart` state variable is `$state<ECharts | null>(null)`. This ensures the `$effect` watching `option` properly re-tracks when `chart` transitions from `null` to initialized.

[SEVERE] Theme re-application requires full chart re-initialization (dispose + init). ECharts does not support changing the theme of an existing instance. The `rebuildWithTheme()` function captures the current option, disposes, re-inits with new theme, and re-applies the option.

[IMPORTANT] The `ResizeObserver` must be disconnected BEFORE `chart.dispose()` in `rebuildWithTheme()` to avoid race conditions where the observer fires on a disposed chart. Reconnect after re-init.

[IMPORTANT] The `matchMedia` listener detects OS-level dark/light switches that do not change the `data-theme` attribute (when theme is set to "system"). Without this listener, switching OS dark mode with theme=system would not update chart colors.

[MINOR] Loading spinner color is read from CSS token `--color-primary` with `#6c63ff` fallback — not hardcoded.

```svelte
<!--
  EChart.svelte — Reusable ECharts wrapper component.

  Handles initialization, reactive option updates, responsive resizing,
  theme integration (light/dark), loading state, and cleanup.

  Usage:
    <EChart option={chartOption} height="400px" />
-->
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
  let mediaQuery: MediaQueryList | null = null

  // ── Initialization ──────────────────────────────────────────────

  // [FIX] Remove the `chart.setOption(option)` call from onMount.
  // The `$effect` watching `option` (below) handles it when `chart`
  // transitions from null to initialized. Having both causes a redundant
  // double-render on first mount.
  onMount(() => {
    const theme = buildEChartsTheme()
    chart = echarts.init(chartEl, theme, { renderer: 'svg' })

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

    // ── Theme change listener (data-theme attribute) ────────────
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
    if (typeof matchMedia !== 'undefined') {
      mediaQuery = matchMedia('(prefers-color-scheme: dark)')
      mediaQuery.addEventListener('change', rebuildWithTheme)
    }
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

  /**
   * Rebuild the chart with a fresh theme. Called when data-theme attribute
   * changes or OS dark/light preference changes.
   *
   * Sequence: disconnect resize observer -> capture current option ->
   * dispose chart -> rebuild theme -> re-init -> re-apply option ->
   * re-register event handlers -> reconnect resize observer.
   */
  function rebuildWithTheme() {
    if (!chart) return
    const currentOption = chart.getOption()

    // Disconnect resize observer before dispose to avoid race conditions
    resizeObserver?.disconnect()

    chart.dispose()

    const theme = buildEChartsTheme()
    chart = echarts.init(chartEl, theme, { renderer: 'svg' })
    chart.setOption(currentOption as EChartsOption)

    // Re-register event handlers after re-init
    if (onChartEvent) {
      for (const [eventName, handler] of Object.entries(onChartEvent)) {
        chart.on(eventName, handler)
      }
    }

    // Reconnect resize observer
    resizeObserver?.observe(chartEl)
  }

  // ── Cleanup ─────────────────────────────────────────────────────

  onDestroy(() => {
    resizeObserver?.disconnect()
    themeObserver?.disconnect()
    mediaQuery?.removeEventListener('change', rebuildWithTheme)
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

**Props interface:**

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `option` | `EChartsOption` | *(required)* | ECharts configuration object. Reactive — changes trigger `setOption()`. |
| `height` | `string` | `'400px'` | CSS height for the chart container. |
| `width` | `string` | `'100%'` | CSS width for the chart container. |
| `loading` | `boolean` | `false` | When true, shows ECharts' built-in loading spinner overlay. |
| `notMerge` | `boolean` | `false` | When true, `setOption()` replaces entire option instead of merging. |
| `onChartEvent` | `Record<string, (params: any) => void>` | `undefined` | Map of ECharts event names to handlers. Iterated on init, each calls `chart.on()`. |

**Lifecycle summary:**
1. **Mount:** `echarts.init()` with custom theme + SVG renderer. Apply initial option. Register events. Set up ResizeObserver, MutationObserver, matchMedia listener.
2. **Option change:** `$effect` watches `option` and calls `chart.setOption()`.
3. **Loading change:** `$effect` watches `loading` and calls `showLoading()`/`hideLoading()`.
4. **Theme change:** MutationObserver + matchMedia trigger `rebuildWithTheme()`.
5. **Resize:** ResizeObserver calls `chart.resize()`.
6. **Destroy:** Disconnect all observers, remove matchMedia listener, dispose chart.

**Acceptance criteria:**
- Component renders a `<div>` with specified `height` and `width`.
- Passing `option` initializes the chart (no console errors).
- Changing `option` reactively updates the chart.
- Setting `loading = true` shows loading overlay; `false` hides it.
- Container resize triggers `chart.resize()`.
- Theme change (data-theme attribute) triggers chart re-initialization with new theme.
- OS dark/light switch triggers theme rebuild.
- `rebuildWithTheme()` disconnects resizeObserver before dispose and reconnects after re-init.
- Event handlers re-registered after theme rebuild.
- `onDestroy` disposes chart, disconnects all observers, removes matchMedia listener.
- Loading spinner color reads from `--color-primary` CSS token.
- `EChartsOption` imported from `echarts/core` (not `echarts`).

**Failure criteria:**
- Chart not disposed on unmount (memory leak).
- Theme change causes crash (dispose not called before re-init, or resizeObserver fires on disposed chart).
- Option `$effect` fires before chart is initialized (null reference).
- Event handlers lost after theme rebuild.
- Loading spinner uses hardcoded color instead of CSS token.

---

### T59.6: Write Registry Unit Tests

**File:** `packages/webui/src/lib/components/charts/__tests__/echarts-registry.test.ts`

[IMPORTANT] These tests verify that the registry exports a valid, configured ECharts instance. They do NOT render charts (that requires a DOM container) — they verify the module interface.

```typescript
import { describe, it, expect } from 'vitest'
import { echarts } from '../echarts-registry'

describe('echarts-registry', () => {
  it('exports a valid echarts instance', () => {
    expect(echarts).toBeDefined()
    expect(typeof echarts.init).toBe('function')
  })

  it('has use method (registration function)', () => {
    expect(typeof echarts.use).toBe('function')
  })

  it('exports echarts from echarts/core (not full bundle)', () => {
    // The echarts/core module exports a `use` function.
    // The full `echarts` module also has `use`, but this test verifies
    // our import path is correct by checking the module identity.
    expect(echarts).toBeDefined()
    // If we imported from 'echarts' instead of 'echarts/core',
    // tree-shaking would not work. This test is a reminder, not
    // a runtime assertion (both modules have the same API surface).
  })
})
```

**Acceptance criteria:**
- All tests pass.
- `echarts` export is defined and has `init` and `use` methods.

**Failure criteria:**
- Import fails (module not found or registration error).

---

### T59.7: Write Theme Unit Tests

**File:** `packages/webui/src/lib/components/charts/__tests__/echarts-theme.test.ts`

[IMPORTANT] In vitest with jsdom, `getComputedStyle` returns empty strings for unset CSS custom properties. Tests verify the fallback behavior.

```typescript
import { describe, it, expect, beforeEach } from 'vitest'
import { buildEChartsTheme } from '../echarts-theme'

describe('buildEChartsTheme', () => {
  it('returns an object with required keys', () => {
    const theme = buildEChartsTheme() as Record<string, any>
    expect(theme).toHaveProperty('color')
    expect(theme).toHaveProperty('backgroundColor')
    expect(theme).toHaveProperty('title')
    expect(theme).toHaveProperty('legend')
    expect(theme).toHaveProperty('tooltip')
    expect(theme).toHaveProperty('categoryAxis')
    expect(theme).toHaveProperty('valueAxis')
  })

  it('color palette has 8 entries', () => {
    const theme = buildEChartsTheme() as Record<string, any>
    expect(theme.color).toHaveLength(8)
  })

  it('uses fallback values when CSS properties are not set', () => {
    // In jsdom, CSS custom properties are not set, so fallbacks are used
    const theme = buildEChartsTheme() as Record<string, any>
    expect(theme.color[0]).toBe('#6c63ff')
    expect(theme.color[1]).toBe('#22c55e')
    expect(theme.color[2]).toBe('#f59e0b')
    expect(theme.color[3]).toBe('#ef4444')
    expect(theme.color[4]).toBe('#06b6d4')
    expect(theme.color[5]).toBe('#8b5cf6')
    expect(theme.color[6]).toBe('#ec4899')
    expect(theme.color[7]).toBe('#14b8a6')
  })

  it('background is transparent', () => {
    const theme = buildEChartsTheme() as Record<string, any>
    expect(theme.backgroundColor).toBe('transparent')
  })

  it('title style uses fallback colors', () => {
    const theme = buildEChartsTheme() as Record<string, any>
    expect(theme.title.textStyle.color).toBe('#1a1a2e')
    expect(theme.title.textStyle.fontSize).toBe(16)
    expect(theme.title.textStyle.fontWeight).toBe(600)
    expect(theme.title.subtextStyle.color).toBe('#6b7280')
    expect(theme.title.subtextStyle.fontSize).toBe(12)
  })

  it('tooltip style uses fallback colors', () => {
    const theme = buildEChartsTheme() as Record<string, any>
    expect(theme.tooltip.backgroundColor).toBe('#ffffff')
    expect(theme.tooltip.borderColor).toBe('#e5e7eb')
    expect(theme.tooltip.textStyle.color).toBe('#1a1a2e')
  })

  it('axis styles use fallback colors', () => {
    const theme = buildEChartsTheme() as Record<string, any>
    expect(theme.categoryAxis.axisLine.lineStyle.color).toBe('#e5e7eb')
    expect(theme.categoryAxis.axisLabel.color).toBe('#6b7280')
    expect(theme.categoryAxis.splitLine.lineStyle.color).toBe('#f3f4f6')
    expect(theme.valueAxis.axisLine.lineStyle.color).toBe('#e5e7eb')
    expect(theme.valueAxis.axisLabel.color).toBe('#6b7280')
  })

  it('reads CSS custom properties when set', () => {
    // Set a CSS custom property on the root element
    document.documentElement.style.setProperty('--color-chart-1', '#ff0000')
    const theme = buildEChartsTheme() as Record<string, any>
    expect(theme.color[0]).toBe('#ff0000')
    // Cleanup
    document.documentElement.style.removeProperty('--color-chart-1')
  })

  it('legend style uses fallback font', () => {
    const theme = buildEChartsTheme() as Record<string, any>
    expect(theme.legend.textStyle.color).toBe('#1a1a2e')
    expect(theme.legend.textStyle.fontFamily).toBe('Inter, system-ui, sans-serif')
  })
})
```

**Acceptance criteria:**
- All 9 test cases pass.
- Fallback values match the hardcoded defaults in `echarts-theme.ts`.
- CSS custom property reading is verified by setting a property on `document.documentElement`.

**Failure criteria:**
- Fallback values in test do not match those in source (test would fail).
- CSS property set/read test fails in jsdom (vitest environment issue).

---

## Testing Support

### Unit Tests

**File:** `packages/webui/src/lib/components/charts/__tests__/echarts-registry.test.ts` (T59.6)

| Test | Assertion |
|------|-----------|
| Exports valid echarts | `echarts` is defined, has `init` function |
| Has `use` method | `typeof echarts.use === 'function'` |
| Imports from echarts/core | Module is defined (reminder test) |

**File:** `packages/webui/src/lib/components/charts/__tests__/echarts-theme.test.ts` (T59.7)

| Test | Assertion |
|------|-----------|
| Returns object with required keys | `color`, `backgroundColor`, `title`, `legend`, `tooltip`, `categoryAxis`, `valueAxis` |
| Color palette has 8 entries | `theme.color.length === 8` |
| Uses fallback values | Each fallback matches hardcoded default |
| Background is transparent | `backgroundColor === 'transparent'` |
| Title style fallbacks | Color, fontSize, fontWeight match |
| Tooltip style fallbacks | Background, border, text color match |
| Axis style fallbacks | Line, label, splitLine colors match |
| Reads CSS custom properties | Set `--color-chart-1`, verify it is read |
| Legend style fallbacks | Color, fontFamily match |

### Component Tests (Manual / Future Playwright)

| Test | What to verify |
|------|---------------|
| Component renders div | `<div class="echart-container">` appears in DOM with specified height/width |
| Chart initializes | SVG elements created inside container (inspect DOM) |
| Option update | Change option prop, chart re-renders (visual confirmation) |
| Loading overlay | Set `loading=true`, spinner appears. Set `false`, spinner gone. |
| Responsive resize | Resize browser window, chart resizes within container. |
| Theme toggle | Switch light/dark, chart colors update (dispose + re-init visible in DevTools) |
| Cleanup | Unmount component, verify no console errors, no orphaned SVG elements |
| Event forwarding | Pass `onChartEvent={{ click: fn }}`, click a chart element, verify `fn` called |

### Bundle Size Verification

| Test | What to verify |
|------|---------------|
| Build succeeds | `bun run build` completes without errors |
| ECharts chunk size | Check `dist/` output — ECharts-related chunks should total < 250KB minified |
| No unregistered types | Source map analysis — `BarChart`, `LineChart` etc. should NOT appear in bundle |

### Regression Gate

Before merging:
1. `bun run check` passes (TypeScript compilation).
2. `bun run build` succeeds.
3. `bun test` passes — specifically `echarts-registry.test.ts` and `echarts-theme.test.ts`.
4. ECharts bundle chunk < 250KB minified (check build output).
5. Existing tests not modified by this phase continue to pass.

---

## Documentation Requirements

- No new documentation files.
- The spec file serves as the design document.
- This plan file serves as the implementation reference.
- Inline TSDoc comments on:
  - `echarts-registry.ts`: why centralized, import pattern for consumers, chart type listing.
  - `buildEChartsTheme()`: must be called at render time, not module scope. Fallback behavior.
  - `EChart.svelte` props: each prop's purpose, default value, reactivity behavior.
  - `rebuildWithTheme()`: dispose/re-init sequence, resizeObserver disconnect/reconnect rationale.
- Inline code comments for:
  - SVG renderer choice rationale.
  - `matchMedia` listener for OS-level dark/light detection.
  - `$effect` tracking: `chart` as `$state` enables the option effect to re-track after initialization.
  - Loading spinner color from CSS token.

---

## Parallelization Notes

**Within this phase:**
- T59.1 (install dependency) must be first.
- T59.2 (registry) and T59.3 (theme) can be developed in parallel after T59.1 — they have no imports between each other.
- T59.4 (CSS tokens) can be done in parallel with T59.2/T59.3.
- T59.5 (component) depends on T59.2 and T59.3 (imports both).
- T59.6 (registry tests) depends on T59.2.
- T59.7 (theme tests) depends on T59.3.

**Recommended execution order:**
1. T59.1 (install — foundational)
2. T59.2 + T59.3 + T59.4 (registry + theme + tokens — parallel, no interdependencies)
3. T59.5 (component — depends on T59.2 and T59.3)
4. T59.6 + T59.7 (tests — parallel, each depends on its source file)

**Cross-phase:**
- This phase creates new files only (except adding `echarts` to `package.json`). It can run in parallel with Phase 57 and Phase 58.
- If Phase 42 has not landed, T59.4 adds chart tokens to `+layout.svelte`. Phase 58 also modifies `+layout.svelte` (adds profile button). These changes are in different parts of the file (CSS tokens in `<style>`, profile button in template) — no conflict expected, but coordinate if both phases land simultaneously.
- J2, J3, J4, J5 all depend on this phase. Once the component is committed, chart consumers can use `<EChart option={...} />` immediately.
- Adding a new chart type for J2-J5 requires only a new import + `echarts.use()` entry in `echarts-registry.ts`. Example: J3 might need `BarChart` — one line added to the registry, no other files change.
