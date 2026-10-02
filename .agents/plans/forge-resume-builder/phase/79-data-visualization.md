# Phase 79: Data Visualization Implementation Plan

> **For agentic workers:** Use superpowers:subagent-driven-development

**Goal:** Fix chart theming token references, standardize font-size tokens in chart components, fix dark mode loading overlay, add RenderViewport placeholder, fix dashboard pending-card href bug, add self-wrapping documentation to chart catalog, add acceptance tests.

**Depends on:** Phase 75 (layout tokens)

**Tech Stack:** Svelte 5 (runes mode), CSS design tokens, ECharts (via echarts-registry), bun:test

**Key files:**
- ECharts theme: `packages/webui/src/lib/components/charts/echarts-theme.ts`
- ECharts wrapper: `packages/webui/src/lib/components/charts/EChart.svelte`
- JD Skill Radar: `packages/webui/src/lib/components/charts/JDSkillRadar.svelte`
- Compensation Graph: `packages/webui/src/lib/components/charts/CompensationBulletGraph.svelte`
- Dashboard: `packages/webui/src/routes/+page.svelte`
- Component barrel: `packages/webui/src/lib/components/index.ts`
- Tests: `packages/webui/src/__tests__/data-visualization.test.ts` (new)

**Non-goals:**
- Don't implement full RenderViewport (library decisions TBD -- remark/marked, highlight.js/CodeMirror, pdf.js)
- Don't refactor chart component architecture
- Don't add new chart types
- Don't change Sigma.js chain view (it's working)
- Don't extract MetricContainer/MetricCalloutContainer to standalone components (only used on dashboard)

**Internal task parallelization:**
- Tasks 79.1-79.5 are all independent (different files)
- Task 79.6 is independent (new file)
- Task 79.7 depends on all previous tasks (tests validate the fixes)

---

## Task 79.1: Fix echarts-theme.ts token references

**Files to modify:**
- `packages/webui/src/lib/components/charts/echarts-theme.ts`

**Satisfies acceptance criteria:** #1 (Charts render correctly in both light and dark mode)

### Problem

The theme builder references two CSS custom properties that do not exist in tokens.css:
- `--border-primary` (used for tooltip border, axis lines) -- should be `--color-border`
- `--border-subtle` (used for split/grid lines) -- should be `--color-surface-sunken`

Because these tokens don't exist, `getComputedStyle` returns empty strings and the `token()` helper falls through to the hardcoded light-mode fallback values. In dark mode, this means axis lines and tooltip borders render with light-mode colors against dark backgrounds.

Additionally, the `--font-sans` fallback is `'Inter, system-ui, sans-serif'` but the actual token value in tokens.css is `'-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif'`. The fallback should match.

### Implementation

Replace the full contents of `packages/webui/src/lib/components/charts/echarts-theme.ts`:

```typescript
/**
 * Build an ECharts theme object from CSS custom property values.
 *
 * IMPORTANT: Call this function at render time (inside `onMount`), not at
 * module scope. CSS custom properties are only available after the document
 * has loaded and the `<style>` has been applied. Calling at module scope
 * returns fallback values.
 *
 * The function reads CSS custom properties from `document.documentElement`
 * via `getComputedStyle`. This requires a browser context — it must never
 * be called during SSR.
 *
 * Fallback values are provided for every token so the theme degrades
 * gracefully if a CSS custom property is not defined.
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
        fontFamily: token('--font-sans', "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif"),
      },
      subtextStyle: {
        color: token('--text-secondary', '#6b7280'),
        fontSize: 12,
        fontFamily: token('--font-sans', "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif"),
      },
    },

    // ── Legend ────────────────────────────────────────────────────
    legend: {
      textStyle: {
        color: token('--text-primary', '#1a1a2e'),
        fontFamily: token('--font-sans', "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif"),
      },
    },

    // ── Tooltip ──────────────────────────────────────────────────
    tooltip: {
      backgroundColor: token('--color-surface', '#ffffff'),
      borderColor: token('--color-border', '#e5e7eb'),
      textStyle: {
        color: token('--text-primary', '#1a1a2e'),
        fontFamily: token('--font-sans', "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif"),
      },
    },

    // ── Category Axis ────────────────────────────────────────────
    categoryAxis: {
      axisLine: { lineStyle: { color: token('--color-border', '#e5e7eb') } },
      axisLabel: { color: token('--text-secondary', '#6b7280') },
      splitLine: { lineStyle: { color: token('--color-surface-sunken', '#f3f4f6') } },
    },

    // ── Value Axis ───────────────────────────────────────────────
    valueAxis: {
      axisLine: { lineStyle: { color: token('--color-border', '#e5e7eb') } },
      axisLabel: { color: token('--text-secondary', '#6b7280') },
      splitLine: { lineStyle: { color: token('--color-surface-sunken', '#f3f4f6') } },
    },
  }
}
```

### What changed

| Line | Before | After | Why |
|------|--------|-------|-----|
| Tooltip borderColor | `--border-primary` | `--color-border` | `--border-primary` does not exist in tokens.css |
| categoryAxis axisLine | `--border-primary` | `--color-border` | Same -- nonexistent token |
| valueAxis axisLine | `--border-primary` | `--color-border` | Same |
| categoryAxis splitLine | `--border-subtle` | `--color-surface-sunken` | `--border-subtle` does not exist; `--color-surface-sunken` is the semantic match per spec |
| valueAxis splitLine | `--border-subtle` | `--color-surface-sunken` | Same |
| All `--font-sans` fallbacks | `'Inter, system-ui, sans-serif'` | `"-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif"` | Match actual `--font-sans` value from tokens.css |

---

## Task 79.2: Fix chart component font-size token divergence

**Files to modify:**
- `packages/webui/src/lib/components/charts/JDSkillRadar.svelte`
- `packages/webui/src/lib/components/charts/CompensationBulletGraph.svelte`

**Satisfies acceptance criteria:** #10 (Font-size divergences remediated)

### Problem

Both components use `--font-size-sm`, `--font-size-xs`, and `--font-size-base` which are not defined in tokens.css. The actual design tokens are `--text-sm`, `--text-xs`, and `--text-base`. Because the wrong tokens are referenced, the components fall through to their hardcoded fallback values instead of inheriting the design system sizes.

### Implementation -- JDSkillRadar.svelte

In `packages/webui/src/lib/components/charts/JDSkillRadar.svelte`, make the following replacements in the `<style>` block:

Replace all occurrences of `var(--font-size-sm, 0.875rem)` with `var(--text-sm)`.

Replace all occurrences of `var(--font-size-xs, 0.75rem)` with `var(--text-xs)`.

Replace all occurrences of `var(--font-size-base, 1rem)` with `var(--text-base)`.

Complete `<style>` block for JDSkillRadar.svelte:

```svelte
<style>
  .radar-loading,
  .radar-error,
  .radar-empty {
    padding: 1.5rem;
    text-align: center;
    font-size: var(--text-sm);
    color: var(--text-muted);
    border: 1px dashed var(--color-border);
    border-radius: 0.5rem;
    margin: 0.75rem 0;
  }

  .radar-error {
    color: var(--color-danger);
    border-color: var(--color-danger);
  }

  .radar-container {
    margin: 0.75rem 0;
  }

  .radar-header {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    margin-bottom: 0.5rem;
  }

  .radar-header h3 {
    font-size: var(--text-base);
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
  }

  .radar-summary {
    font-size: var(--text-sm);
    color: var(--text-secondary);
  }

  .gap-list {
    margin-top: 1rem;
    padding: 0.75rem;
    background: var(--color-surface-raised, var(--color-surface));
    border: 1px solid var(--color-border);
    border-radius: 0.5rem;
  }

  .gap-list h4 {
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--text-primary);
    margin: 0 0 0.5rem 0;
  }

  .gap-category {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
  }

  .gap-category:last-child {
    margin-bottom: 0;
  }

  .gap-category-name {
    font-size: var(--text-xs);
    font-weight: 600;
    color: var(--text-secondary);
    min-width: 100px;
    text-transform: capitalize;
  }

  .gap-skills {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
  }

  .gap-pill {
    display: inline-block;
    padding: 0.15rem 0.5rem;
    font-size: var(--text-xs);
    background: var(--color-danger-subtle, rgba(239, 68, 68, 0.1));
    color: var(--color-danger);
    border-radius: 1rem;
  }
</style>
```

**Specific replacements (8 total):**

| Location | Before | After |
|----------|--------|-------|
| `.radar-loading/error/empty` font-size | `var(--font-size-sm, 0.875rem)` | `var(--text-sm)` |
| `.radar-header h3` font-size | `var(--font-size-base, 1rem)` | `var(--text-base)` |
| `.radar-summary` font-size | `var(--font-size-sm, 0.875rem)` | `var(--text-sm)` |
| `.gap-list h4` font-size | `var(--font-size-sm, 0.875rem)` | `var(--text-sm)` |
| `.gap-category-name` font-size | `var(--font-size-xs, 0.75rem)` | `var(--text-xs)` |
| `.gap-pill` font-size | `var(--font-size-xs, 0.75rem)` | `var(--text-xs)` |

### Implementation -- CompensationBulletGraph.svelte

In `packages/webui/src/lib/components/charts/CompensationBulletGraph.svelte`, make the following replacements in the `<style>` block:

Replace all occurrences of `var(--font-size-sm, 0.875rem)` with `var(--text-sm)`.

Replace all occurrences of `var(--font-size-xs, 0.75rem)` with `var(--text-xs)`.

Complete `<style>` block for CompensationBulletGraph.svelte:

```svelte
<style>
  .comp-loading,
  .comp-empty {
    padding: 1rem;
    text-align: center;
    font-size: var(--text-sm);
    color: var(--text-muted);
    border: 1px dashed var(--color-border);
    border-radius: 0.5rem;
    margin: 0.75rem 0;
  }

  .comp-container {
    margin: 0.75rem 0;
  }

  .comp-hint {
    font-size: var(--text-xs);
    color: var(--text-muted);
    text-align: center;
    margin-top: 0.5rem;
  }

  .comp-hint a {
    color: var(--color-info);
    text-decoration: none;
  }

  .comp-hint a:hover {
    text-decoration: underline;
  }
</style>
```

**Specific replacements (2 total):**

| Location | Before | After |
|----------|--------|-------|
| `.comp-loading/empty` font-size | `var(--font-size-sm, 0.875rem)` | `var(--text-sm)` |
| `.comp-hint` font-size | `var(--font-size-xs, 0.75rem)` | `var(--text-xs)` |

---

## Task 79.3: Fix loading overlay dark mode

**Files to modify:**
- `packages/webui/src/lib/components/charts/EChart.svelte`

**Satisfies acceptance criteria:** #2 (Loading overlay uses token-resolved color)

### Problem

`EChart.svelte` hardcodes `maskColor: 'rgba(255, 255, 255, 0.7)'` in two places (the `onMount` loading check and the `$effect` loading watcher). In dark mode, this produces a white flash over dark-background charts. The mask should use a token-resolved surface color with alpha transparency.

### Implementation

In `packages/webui/src/lib/components/charts/EChart.svelte`, replace the hardcoded mask color in both `showLoading` calls with a dynamically resolved value from the `--color-surface` token.

Replace the `onMount` loading block (lines 62-68):

```typescript
    if (loading) {
      const surface = getComputedStyle(chartEl).getPropertyValue('--color-surface').trim() || '#ffffff'
      chart.showLoading('default', {
        text: '',
        color: getComputedStyle(chartEl).getPropertyValue('--color-primary').trim() || '#6c63ff',
        maskColor: surface.startsWith('#')
          ? surface + 'b3' // ~70% opacity via hex alpha
          : surface.startsWith('rgb(')
            ? surface.replace('rgb(', 'rgba(').replace(')', ', 0.7)')
            : 'rgba(255, 255, 255, 0.7)',
      })
    }
```

Replace the `$effect` loading block (lines 112-123):

```typescript
  $effect(() => {
    if (!chart) return
    if (loading) {
      const surface = getComputedStyle(chartEl).getPropertyValue('--color-surface').trim() || '#ffffff'
      chart.showLoading('default', {
        text: '',
        color: getComputedStyle(chartEl).getPropertyValue('--color-primary').trim() || '#6c63ff',
        maskColor: surface.startsWith('#')
          ? surface + 'b3' // ~70% opacity via hex alpha
          : surface.startsWith('rgb(')
            ? surface.replace('rgb(', 'rgba(').replace(')', ', 0.7)')
            : 'rgba(255, 255, 255, 0.7)',
      })
    } else {
      chart.hideLoading()
    }
  })
```

To reduce duplication, extract a helper function before the `onMount`:

```typescript
  /** Resolve a loading mask color from the --color-surface token with ~70% opacity. */
  function resolveMaskColor(el: HTMLElement): string {
    const surface = getComputedStyle(el).getPropertyValue('--color-surface').trim() || '#ffffff'
    if (surface.startsWith('#')) {
      return surface + 'b3' // hex alpha for ~70% opacity
    }
    if (surface.startsWith('rgb(')) {
      return surface.replace('rgb(', 'rgba(').replace(')', ', 0.7)')
    }
    return 'rgba(255, 255, 255, 0.7)'
  }
```

Then both call sites simplify to:

```typescript
    chart.showLoading('default', {
      text: '',
      color: getComputedStyle(chartEl).getPropertyValue('--color-primary').trim() || '#6c63ff',
      maskColor: resolveMaskColor(chartEl),
    })
```

### Full updated EChart.svelte

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
    /** ECharts configuration object. Reactive — changes trigger setOption(). */
    option,
    /** CSS height for the chart container. */
    height = '400px',
    /** CSS width for the chart container. */
    width = '100%',
    /** When true, shows ECharts' built-in loading spinner overlay. */
    loading = false,
    /** When true, setOption() replaces entire option instead of merging. */
    notMerge = false,
    /** Map of ECharts event names to handlers. Each calls chart.on(). */
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
  /** Chart instance as $state so $effect can track null -> initialized transition. */
  let chart = $state<ECharts | null>(null)
  let resizeObserver: ResizeObserver | null = null
  let themeObserver: MutationObserver | null = null
  let mediaQuery: MediaQueryList | null = null

  // ── Helpers ─────────────────────────────────────────────────────

  /** Resolve a loading mask color from the --color-surface token with ~70% opacity. */
  function resolveMaskColor(el: HTMLElement): string {
    const surface = getComputedStyle(el).getPropertyValue('--color-surface').trim() || '#ffffff'
    if (surface.startsWith('#')) {
      return surface + 'b3' // hex alpha for ~70% opacity
    }
    if (surface.startsWith('rgb(')) {
      return surface.replace('rgb(', 'rgba(').replace(')', ', 0.7)')
    }
    return 'rgba(255, 255, 255, 0.7)'
  }

  // ── Initialization ──────────────────────────────────────────────

  // The $effect watching `option` (below) handles the initial setOption
  // when `chart` transitions from null to initialized. No redundant
  // setOption call needed in onMount.
  onMount(() => {
    const theme = buildEChartsTheme()
    chart = echarts.init(chartEl, theme, { renderer: 'svg' })

    // ── Event forwarding ──────────────────────────────────────
    if (onChartEvent) {
      for (const [eventName, handler] of Object.entries(onChartEvent)) {
        chart.on(eventName, handler)
      }
    }

    // Loading spinner color from CSS token, not hardcoded
    if (loading) {
      chart.showLoading('default', {
        text: '',
        color: getComputedStyle(chartEl).getPropertyValue('--color-primary').trim() || '#6c63ff',
        maskColor: resolveMaskColor(chartEl),
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
    // matchMedia listener detects OS-level dark/light switches that
    // do not change the data-theme attribute (when theme is set to "system").
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
        maskColor: resolveMaskColor(chartEl),
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
   *
   * The resize observer is disconnected before dispose to avoid race
   * conditions where the observer fires on a disposed chart.
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

---

## Task 79.4: Add self-wrapping documentation to chart catalog

**Files to modify:**
- `packages/webui/src/lib/components/charts/SkillsSunburst.svelte` (add comment)
- `packages/webui/src/lib/components/charts/SkillsTreemap.svelte` (add comment)
- `packages/webui/src/lib/components/charts/BulletsTreemap.svelte` (add comment)
- `packages/webui/src/lib/components/charts/DomainsTreemap.svelte` (add comment)
- `packages/webui/src/lib/components/charts/ApplicationGantt.svelte` (add comment)
- `packages/webui/src/lib/components/charts/RoleChoropleth.svelte` (add comment)
- `packages/webui/src/lib/components/charts/JDSkillRadar.svelte` (add comment)
- `packages/webui/src/lib/components/charts/CompensationBulletGraph.svelte` (add comment)

**Satisfies acceptance criteria:** Documentation only (no functional changes)

### Problem

Some chart components include their own `.chart-card` wrapper div (with background, border, border-radius, padding) while others do not. The dashboard `+page.svelte` wraps some charts in an external `.chart-card` div. This inconsistency is confusing for consumers who don't know whether to add their own wrapper.

### Self-wrapping status

| Component | Self-wraps with `.chart-card`? |
|-----------|-------------------------------|
| SkillsSunburst | YES -- internal `.chart-card` wrapper |
| SkillsTreemap | YES -- internal `.chart-card` wrapper |
| BulletsTreemap | YES -- internal `.chart-card` wrapper |
| DomainsTreemap | YES -- internal `.chart-card` wrapper |
| ApplicationGantt | NO -- consumer must wrap |
| RoleChoropleth | NO -- consumer must wrap |
| JDSkillRadar | NO -- consumer must wrap |
| CompensationBulletGraph | NO -- consumer must wrap |

### Implementation

Add a line to the top-of-file comment block in each component. This is documentation only -- no code changes.

For self-wrapping components (SkillsSunburst, SkillsTreemap, BulletsTreemap, DomainsTreemap), add to the existing comment block:

```
  @layout Self-wrapping — includes its own .chart-card container. Do NOT wrap externally.
```

For non-wrapping components (ApplicationGantt, RoleChoropleth, JDSkillRadar, CompensationBulletGraph), add:

```
  @layout No wrapper — consumer must provide .chart-card or equivalent container.
```

**Example -- SkillsSunburst.svelte** (first lines):

```svelte
<!--
  SkillsSunburst.svelte — Combined skills sunburst + domain-archetype breakout charts.

  The skills sunburst shows every skill grouped by category with usage-based sizing
  and supports drill-down to single-category pie views. The domain-archetype nested
  pie shows how perspectives distribute across domains and archetypes.

  Data fetching uses onMount (NOT $effect) to avoid infinite reactive loops.

  @layout Self-wrapping — includes its own .chart-card container. Do NOT wrap externally.
-->
```

**Example -- ApplicationGantt.svelte** (first lines):

```svelte
<!--
  ApplicationGantt.svelte — Gantt-style chart showing JD application timelines.

  Each row is a JD with a horizontal bar from created_at to updated_at (or "now"
  for active JDs), colored by pipeline status. Active JDs have a dashed right edge.
  Supports Y-axis scrolling via dataZoom when > 10 JDs exist.
  Clicking a bar navigates to the JD detail page.

  @layout No wrapper — consumer must provide .chart-card or equivalent container.
-->
```

Apply the same pattern to all 8 chart components.

---

## Task 79.5: Fix pending-card href bug

**Files to modify:**
- `packages/webui/src/routes/+page.svelte`

**Satisfies acceptance criteria:** #9 (Dashboard pending-card href bug fixed)

### Problem

In the dashboard `+page.svelte`, both pending review cards link to `/data/sources?tab=bullets`. The "Pending Perspectives" card should link to `/data/sources?tab=perspectives`.

### Implementation

In `packages/webui/src/routes/+page.svelte`, find the second pending card (lines 123-127):

```svelte
          <a href="/data/sources?tab=bullets" class="pending-card">
            <div class="pending-count">{pendingPerspectives}</div>
            <div class="pending-label">Pending Perspectives</div>
            <div class="pending-hint">Click to review</div>
          </a>
```

Replace with:

```svelte
          <a href="/data/sources?tab=perspectives" class="pending-card">
            <div class="pending-count">{pendingPerspectives}</div>
            <div class="pending-label">Pending Perspectives</div>
            <div class="pending-hint">Click to review</div>
          </a>
```

The only change is `?tab=bullets` to `?tab=perspectives` on line 123.

---

## Task 79.6: Mark RenderViewport as design placeholder

**Files to create:**
- `packages/webui/src/lib/components/RenderViewport.svelte`

**Files to modify:**
- `packages/webui/src/lib/components/index.ts` (add export)

**Satisfies acceptance criteria:** #3 (RenderViewport exists as placeholder)

### Implementation

Create `packages/webui/src/lib/components/RenderViewport.svelte`:

```svelte
<!--
  RenderViewport.svelte — Placeholder for embedded document preview.

  Placeholder — library decisions TBD:
    Markdown: remark or marked
    Code highlighting: highlight.js or CodeMirror
    PDF: pdf.js

  This component will eventually render inline previews of Markdown, LaTeX,
  and PDF content. For now it renders a placeholder message indicating the
  format and that the feature is coming soon.

  @layout No wrapper — consumer must provide container.
-->
<script lang="ts">
  let {
    /** The document format to preview. */
    format = 'markdown',
  }: {
    format?: 'markdown' | 'latex' | 'pdf' | string
  } = $props()
</script>

<div class="render-viewport-placeholder">
  <div class="placeholder-icon">
    {#if format === 'markdown'}
      M&darr;
    {:else if format === 'latex'}
      &Lscr;
    {:else if format === 'pdf'}
      PDF
    {:else}
      &hellip;
    {/if}
  </div>
  <p class="placeholder-message">Render preview coming soon. Format: {format}</p>
</div>

<style>
  .render-viewport-placeholder {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.75rem;
    padding: var(--space-8, 2rem);
    border: 1px dashed var(--color-border);
    border-radius: var(--radius-lg);
    background: var(--color-surface-sunken);
    min-height: 200px;
  }

  .placeholder-icon {
    font-size: var(--text-2xl);
    font-weight: var(--font-bold);
    color: var(--text-faint);
    user-select: none;
  }

  .placeholder-message {
    font-size: var(--text-sm);
    color: var(--text-faint);
    font-style: italic;
    margin: 0;
  }
</style>
```

Add to `packages/webui/src/lib/components/index.ts`:

```typescript
export { default as RenderViewport } from './RenderViewport.svelte'
```

Append this line after the existing `TagsList` export.

---

## Task 79.7: Write acceptance tests

**Files to create:**
- `packages/webui/src/__tests__/data-visualization.test.ts`

**Depends on:** Tasks 79.1-79.6

**Satisfies acceptance criteria:** All (validates each fix)

### Implementation

Create `packages/webui/src/__tests__/data-visualization.test.ts`:

```typescript
import { describe, test, expect } from 'bun:test'
import { readFileSync, existsSync, readdirSync } from 'fs'
import { join } from 'path'

const COMPONENTS = join(import.meta.dir, '..', 'lib', 'components')
const CHARTS = join(COMPONENTS, 'charts')
const ROUTES = join(import.meta.dir, '..', 'routes')

function read(path: string): string {
  return readFileSync(path, 'utf-8')
}

/** Recursively find all .svelte files under a directory */
function findSvelteFiles(dir: string): string[] {
  const results: string[] = []
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const fullPath = join(dir, entry.name)
    if (entry.isDirectory()) {
      results.push(...findSvelteFiles(fullPath))
    } else if (entry.name.endsWith('.svelte')) {
      results.push(fullPath)
    }
  }
  return results
}

describe('Data Visualization', () => {
  // AC #1: Charts render in both light and dark mode —
  // echarts-theme.ts uses only existing tokens (no --border-primary/--border-subtle)
  describe('echarts-theme.ts token references', () => {
    const theme = read(join(CHARTS, 'echarts-theme.ts'))

    test('does not reference --border-primary (nonexistent token)', () => {
      expect(theme).not.toContain('--border-primary')
    })

    test('does not reference --border-subtle (nonexistent token)', () => {
      expect(theme).not.toContain('--border-subtle')
    })

    test('uses --color-border for tooltip and axis lines', () => {
      // Should appear at least 3 times: tooltip borderColor + categoryAxis + valueAxis
      const matches = theme.match(/--color-border/g)
      expect(matches).not.toBeNull()
      expect(matches!.length).toBeGreaterThanOrEqual(3)
    })

    test('uses --color-surface-sunken for split/grid lines', () => {
      // Should appear at least 2 times: categoryAxis + valueAxis splitLine
      const matches = theme.match(/--color-surface-sunken/g)
      expect(matches).not.toBeNull()
      expect(matches!.length).toBeGreaterThanOrEqual(2)
    })

    test('font fallback matches tokens.css --font-sans stack (not Inter)', () => {
      expect(theme).not.toContain("'Inter, system-ui, sans-serif'")
      expect(theme).toContain('-apple-system')
      expect(theme).toContain('BlinkMacSystemFont')
    })
  })

  // AC #2: Loading overlay uses token-resolved color, not hardcoded white
  describe('EChart.svelte loading overlay', () => {
    const echart = read(join(CHARTS, 'EChart.svelte'))

    test('does not hardcode white maskColor', () => {
      // The literal 'rgba(255, 255, 255, 0.7)' should only appear as a
      // last-resort fallback inside the resolveMaskColor helper, not as
      // a direct value in showLoading calls.
      const showLoadingBlocks = echart.split('showLoading')
      // Each showLoading call should use resolveMaskColor, not a literal
      for (let i = 1; i < showLoadingBlocks.length; i++) {
        const block = showLoadingBlocks[i].slice(0, 200)
        expect(block).toContain('resolveMaskColor')
      }
    })

    test('has resolveMaskColor helper that reads --color-surface', () => {
      expect(echart).toContain('resolveMaskColor')
      expect(echart).toContain('--color-surface')
    })
  })

  // AC #10: All chart font sizes use --text-* tokens, not --font-size-*
  describe('Chart component font-size tokens', () => {
    const chartFiles = findSvelteFiles(CHARTS)

    test('no chart component uses --font-size- (should use --text-*)', () => {
      const violations: { file: string; lines: string[] }[] = []

      for (const file of chartFiles) {
        const content = read(file)
        const lines = content.split('\n')
        const badLines: string[] = []

        for (let i = 0; i < lines.length; i++) {
          if (lines[i].includes('--font-size-')) {
            badLines.push(`L${i + 1}: ${lines[i].trim()}`)
          }
        }

        if (badLines.length > 0) {
          const name = file.split('/').pop()!
          violations.push({ file: name, lines: badLines })
        }
      }

      expect(violations).toEqual([])
    })
  })

  // AC #3 (placeholder): RenderViewport placeholder exists with format prop
  describe('RenderViewport placeholder', () => {
    test('RenderViewport.svelte exists', () => {
      expect(existsSync(join(COMPONENTS, 'RenderViewport.svelte'))).toBe(true)
    })

    test('accepts format prop', () => {
      const content = read(join(COMPONENTS, 'RenderViewport.svelte'))
      expect(content).toContain('format')
    })

    test('renders placeholder message', () => {
      const content = read(join(COMPONENTS, 'RenderViewport.svelte'))
      expect(content).toContain('Render preview coming soon')
    })

    test('is exported from component barrel', () => {
      const barrel = read(join(COMPONENTS, 'index.ts'))
      expect(barrel).toContain('RenderViewport')
    })
  })

  // AC #9: Dashboard pending-card links are correct
  describe('Dashboard pending card links', () => {
    const dashboard = read(join(ROUTES, '+page.svelte'))

    test('perspectives card links to tab=perspectives (not tab=bullets)', () => {
      // Find the pending card that contains "pendingPerspectives"
      const perspectivesCardMatch = dashboard.match(
        /<a[^>]*href="([^"]*)"[^>]*class="pending-card"[^>]*>[\s\S]*?pendingPerspectives[\s\S]*?<\/a>/
      )
      expect(perspectivesCardMatch).not.toBeNull()
      expect(perspectivesCardMatch![1]).toContain('tab=perspectives')
      expect(perspectivesCardMatch![1]).not.toContain('tab=bullets')
    })

    test('bullets card still links to tab=bullets', () => {
      const bulletsCardMatch = dashboard.match(
        /<a[^>]*href="([^"]*)"[^>]*class="pending-card"[^>]*>[\s\S]*?pendingBullets[\s\S]*?<\/a>/
      )
      expect(bulletsCardMatch).not.toBeNull()
      expect(bulletsCardMatch![1]).toContain('tab=bullets')
    })
  })

  // Chart components document self-wrapping (informational check)
  describe('Chart self-wrapping documentation', () => {
    const selfWrapping = [
      'SkillsSunburst.svelte',
      'SkillsTreemap.svelte',
      'BulletsTreemap.svelte',
      'DomainsTreemap.svelte',
    ]
    const noWrapper = [
      'ApplicationGantt.svelte',
      'RoleChoropleth.svelte',
      'JDSkillRadar.svelte',
      'CompensationBulletGraph.svelte',
    ]

    for (const name of selfWrapping) {
      test(`${name} documents self-wrapping`, () => {
        const content = read(join(CHARTS, name))
        expect(content).toContain('@layout Self-wrapping')
      })
    }

    for (const name of noWrapper) {
      test(`${name} documents no-wrapper requirement`, () => {
        const content = read(join(CHARTS, name))
        expect(content).toContain('@layout No wrapper')
      })
    }
  })
})
```

### Test-to-acceptance-criteria mapping

| Spec AC | Test | What it validates |
|---------|------|-------------------|
| #1 (light/dark mode) | `echarts-theme.ts token references` (4 tests) | No references to nonexistent `--border-primary`/`--border-subtle`; correct tokens used instead |
| #2 (loading overlay) | `EChart.svelte loading overlay` (2 tests) | `maskColor` uses `resolveMaskColor()` helper, not hardcoded white |
| #3 (RenderViewport) | `RenderViewport placeholder` (4 tests) | File exists, has `format` prop, shows placeholder message, exported from barrel |
| #9 (pending-card href) | `Dashboard pending card links` (2 tests) | Perspectives card links to `tab=perspectives`, bullets card still links to `tab=bullets` |
| #10 (font-size tokens) | `Chart component font-size tokens` (1 test) | Zero `--font-size-` references across all chart `.svelte` files |
| Documentation | `Chart self-wrapping documentation` (8 tests) | Every chart component has `@layout` annotation documenting wrapper behavior |
