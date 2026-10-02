# Forge Design System: Data Visualization

**Date:** 2026-04-04
**Doc:** 6 of 6 (Design System Series)
**Status:** Reference specification
**Depends on:** Doc 1 (Foundation), Doc 4 (Content Patterns -- LoadingSpinner, EmptyState)

This document specifies every data visualization component in the Forge UI: ECharts wrappers, dashboard metric containers, the Sigma.js provenance graph, and the document render viewport. It defines component APIs, chart theming integration, dashboard layout patterns, and loading/error states.

---

## 1. Component Summary

| Component | Layer | Description | Location |
|-----------|-------|-------------|----------|
| **EChart** | Atom | Reusable ECharts wrapper with reactive options, resize, and theme integration | `charts/EChart.svelte` |
| **MetricContainer** | Component | Dashboard stat card showing a number + label | Dashboard `+page.svelte` (inline) |
| **MetricCalloutContainer** | Component | Action-oriented card with accent border and optional CTA | Dashboard `+page.svelte` (inline) |
| **ChainViewModal** | View | Sigma.js WebGL graph renderer for Source/Bullet/Perspective provenance | `components/ChainViewModal.svelte` |
| **RenderViewport** | Atom | Embedded renderer for document preview (Markdown, LaTeX, PDF) | Future: `components/RenderViewport.svelte` |

Chart-specific wrappers (SkillsSunburst, SkillsTreemap, BulletsTreemap, DomainsTreemap, ApplicationGantt, RoleChoropleth, CompensationBulletGraph, JDSkillRadar) are **components** that compose the EChart atom with domain-specific data fetching and option building.

---

## 2. GraphViewport (EChart -- ECharts Wrapper)

### 2.1 Purpose

`EChart.svelte` is the single reusable container for all ECharts chart instances in Forge. It handles initialization, reactive option updates, responsive resizing, light/dark theme integration via design tokens, loading state overlay, event forwarding, and cleanup.

Every chart in Forge renders through this component. Domain-specific chart components (SkillsSunburst, ApplicationGantt, etc.) build an `EChartsOption` object and pass it as a prop.

### 2.2 Component API

```typescript
interface EChartProps {
  /** ECharts configuration object. Reactive -- changes trigger setOption(). */
  option: EChartsOption

  /** CSS height for the chart container. Default: '400px' */
  height?: string

  /** CSS width for the chart container. Default: '100%' */
  width?: string

  /** When true, shows ECharts' built-in loading spinner overlay. Default: false */
  loading?: boolean

  /** When true, setOption() replaces entire option instead of merging. Default: false */
  notMerge?: boolean

  /** Map of ECharts event names to handlers. Each calls chart.on(). */
  onChartEvent?: Record<string, (params: any) => void>
}
```

### 2.3 Usage

```svelte
<EChart
  option={sunburstOption}
  height="450px"
  loading={false}
  notMerge={true}
  onChartEvent={{ click: handleSunburstClick }}
/>
```

### 2.4 Lifecycle

1. **Initialization (`onMount`):** Reads CSS custom properties via `buildEChartsTheme()`, calls `echarts.init(el, theme, { renderer: 'svg' })`, registers event handlers from `onChartEvent`, sets up `ResizeObserver` and `MutationObserver`.
2. **Reactive option updates (`$effect`):** When `option` changes, calls `chart.setOption(option, notMerge)`.
3. **Loading state (`$effect`):** Toggles `showLoading` / `hideLoading` based on `loading` prop. Spinner color reads from `--color-primary` token.
4. **Theme change:** `MutationObserver` watches `data-theme` attribute on `<html>`. `matchMedia('(prefers-color-scheme: dark)')` detects OS-level switches. Both trigger `rebuildWithTheme()` which disposes the chart, rebuilds the theme from tokens, re-initializes, and re-applies the current option.
5. **Cleanup (`onDestroy`):** Disconnects observers, removes media query listener, disposes chart instance.

### 2.5 Renderer Choice

Forge uses the **SVG renderer** exclusively. Rationale:
- Crisper text rendering (important for chart labels).
- Smaller bundle (no Canvas polyfills).
- Sufficient for Forge's data volumes (hundreds of data points, not thousands).

Registered in `echarts-registry.ts` via tree-shakeable imports.

### 2.6 Chart Type Registry

All chart types and ECharts components are registered centrally in `charts/echarts-registry.ts`. Adding a new chart type requires one import + one `use()` entry. No other files change.

Currently registered:
- **Charts:** Pie, Sunburst, Treemap, Custom (Gantt), Map (Choropleth), Radar, Bar
- **Components:** Tooltip, Legend, Title, Grid, VisualMap, Dataset, Transform, Graphic, Radar, MarkArea, MarkLine, DataZoom
- **Renderer:** SVG

```typescript
// Consumer import pattern:
import { echarts } from '$lib/components/charts/echarts-registry'
```

### 2.7 CSS

```css
.echart-container {
  min-height: 200px;
}
```

The container is styled with `style:height` and `style:width` bound to props. No hardcoded dimensions. The `min-height` prevents invisible charts when data loads slowly.

---

## 3. Chart Theming

### 3.1 Theme Builder

`echarts-theme.ts` exports `buildEChartsTheme()` which reads CSS custom properties from `document.documentElement` at render time (inside `onMount`, never at module scope). Every value has a fallback for graceful degradation.

### 3.2 Token-to-Theme Mapping

| Theme Property | CSS Token | Fallback | Notes |
|---------------|-----------|----------|-------|
| **Color palette** `color[0..7]` | `--color-chart-1` through `--color-chart-8` | `#6c63ff`, `#22c55e`, `#f59e0b`, `#ef4444`, `#06b6d4`, `#8b5cf6`, `#ec4899`, `#14b8a6` | Auto-assigned to series in order |
| **Background** | (none) | `transparent` | Charts float on card backgrounds |
| **Title text** | `--text-primary` | `#1a1a2e` | |
| **Title font** | `--font-sans` | `Inter, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif` | Must match the actual `--font-sans` token value in tokens.css |
| **Subtitle text** | `--text-secondary` | `#6b7280` | |
| **Legend text** | `--text-primary` | `#1a1a2e` | |
| **Tooltip background** | `--color-surface` | `#ffffff` | |
| **Tooltip border** | `--color-border` | `#e5e7eb` | (not `--border-primary` -- that token does not exist) |
| **Tooltip text** | `--text-primary` | `#1a1a2e` | |
| **Axis lines** | `--color-border` | `#e5e7eb` | Category and value axes |
| **Axis labels** | `--text-secondary` | `#6b7280` | |
| **Grid/split lines** | `--color-surface-sunken` | `#f3f4f6` | (not `--border-subtle` -- that token does not exist; using surface-sunken as the nearest semantic match) |

### 3.3 Dark Mode

The chart palette has dedicated dark-mode values defined in `tokens.css`:

```css
/* Light mode */
:root {
  --color-chart-1: #6c63ff;
  --color-chart-2: #22c55e;
  /* ... */
}

/* Dark mode */
html[data-theme="dark"] {
  --color-chart-1: #818cf8;
  --color-chart-2: #4ade80;
  /* ... */
}
```

Dark-mode colors are higher-saturation variants that maintain contrast against dark surface backgrounds. Theme switching triggers `rebuildWithTheme()` in EChart.svelte, which re-reads all tokens and re-initializes the chart.

### 3.4 Resolving Tokens for ECharts Options

ECharts cannot interpret CSS `var()` expressions. When chart option builders need token colors (e.g., for custom series `itemStyle`, `markLine`, or `graphic` elements), they must resolve tokens at render time:

```typescript
import { resolveTokenColor } from './skills-chart-utils'

// Inside onMount or an event handler -- never at module scope
const primaryColor = resolveTokenColor('--color-primary', '#6c63ff')
```

```typescript
export function resolveTokenColor(token: string, fallback: string): string {
  return getComputedStyle(document.documentElement)
    .getPropertyValue(token).trim() || fallback
}
```

### 3.5 Domain Color Maps

Domain-specific coloring (e.g., skill categories mapped to chart palette slots) is built via `buildDomainColors()` which resolves tokens at render time:

```typescript
export function buildDomainColors(): Record<string, string> {
  return {
    security:       resolveTokenColor('--color-chart-1', '#6c63ff'),
    cloud:          resolveTokenColor('--color-chart-2', '#22c55e'),
    ai_ml:          resolveTokenColor('--color-chart-3', '#f59e0b'),
    infrastructure: resolveTokenColor('--color-chart-4', '#ef4444'),
    data:           resolveTokenColor('--color-chart-5', '#06b6d4'),
    general:        resolveTokenColor('--color-chart-6', '#8b5cf6'),
    devops:         resolveTokenColor('--color-chart-7', '#ec4899'),
    unassigned:     resolveTokenColor('--color-chart-8', '#14b8a6'),
  }
}
```

---

## 4. MetricContainer (Quick Stats)

### 4.1 Purpose

A dashboard card displaying a single metric: a large number with a descriptive label. Used in the "Quick Stats" section of the dashboard to show counts for Sources, Bullets, Perspectives, Organizations, and Resumes.

### 4.2 Current Implementation

MetricContainer is currently implemented as inline CSS classes in the dashboard `+page.svelte` rather than a standalone component. This is acceptable because the pattern is used only on the dashboard. If it appears in 3+ locations, it should be extracted.

### 4.3 Component API (Extractable)

```typescript
interface MetricContainerProps {
  /** The metric value to display prominently. */
  value: number | string

  /** Descriptive label below the value. */
  label: string

  /** Optional accent color for the value. Default: --color-primary */
  color?: string

  /** Optional icon or slot for visual accent. */
  icon?: Snippet
}
```

### 4.4 HTML Structure

```svelte
<div class="stat-card">
  {#if icon}{@render icon()}{/if}
  <div class="stat-number" style:color={color}>{value}</div>
  <div class="stat-label">{label}</div>
</div>
```

### 4.5 CSS

```css
.stat-card {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  padding: 1.25rem;
  text-align: center;
}

.stat-number {
  font-size: 2rem;
  font-weight: var(--font-bold);
  color: var(--color-primary);
  line-height: 1;
  margin-bottom: 0.25rem;
}

.stat-label {
  font-size: var(--text-sm);
  color: var(--text-muted);
  font-weight: var(--font-medium);
}
```

### 4.6 Layout

Stats render in a responsive grid:

```css
.stats-grid {
  display: grid;
  grid-template-columns: repeat(5, 1fr);
  gap: 1rem;
}

/* Responsive: 3-column on medium, 2-column on small */
@media (max-width: 768px) {
  .stats-grid {
    grid-template-columns: repeat(3, 1fr);
  }
}

@media (max-width: 480px) {
  .stats-grid {
    grid-template-columns: repeat(2, 1fr);
  }
}
```

---

## 5. MetricCalloutContainer (Alert/Action Cards)

### 5.1 Purpose

Dashboard cards that highlight items requiring user action. They differ from MetricContainer in three ways: (1) an accent-colored left border indicates severity, (2) supplementary text explains what needs attention, and (3) the entire card is clickable or contains an action link.

Two callout types exist on the dashboard:
- **Pending Review** (warning accent): Pending Bullets and Pending Perspectives counts with a link to the review page.
- **Integrity Alerts** (danger accent): Drifted entity count with a link to Chain View.

### 5.2 Component API (Extractable)

```typescript
interface MetricCalloutContainerProps {
  /** Large number to display. */
  count: number

  /** Title label for the callout. */
  label: string

  /** Supplementary hint text. */
  hint?: string

  /** Severity determines accent border color. Default: 'warning' */
  severity?: 'info' | 'warning' | 'danger'

  /** Navigation URL. If provided, the card renders as an <a> element. */
  href?: string

  /** Click handler (alternative to href for programmatic navigation). */
  onclick?: () => void
}
```

### 5.3 Severity-to-Token Mapping

| Severity | Border token | Count color |
|----------|-------------|-------------|
| `info` | `--color-info` | `--color-info` |
| `warning` | `--color-warning` | `--text-primary` |
| `danger` | `--color-danger` | `--color-danger` |

### 5.4 CSS -- Pending Review Card

```css
.pending-card {
  display: block;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-left: 4px solid var(--color-warning);
  border-radius: var(--radius-lg);
  padding: 1.5rem;
  text-decoration: none;
  color: inherit;
  transition: box-shadow 0.15s, border-color 0.15s;
  cursor: pointer;
}

.pending-card:hover {
  box-shadow: var(--shadow-sm);
  border-left-color: var(--color-warning-text);
}

.pending-count {
  font-size: 2.25rem;
  font-weight: var(--font-bold);
  color: var(--text-primary);
  line-height: 1;
  margin-bottom: 0.25rem;
}

.pending-label {
  font-size: var(--text-base);
  font-weight: var(--font-semibold);
  color: var(--text-secondary);
  margin-bottom: 0.5rem;
}

.pending-hint {
  font-size: var(--text-sm);
  color: var(--text-faint);
}
```

### 5.5 CSS -- Alert Card (Danger Variant)

```css
.alert-card {
  display: block;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-left: 4px solid var(--color-danger);
  border-radius: var(--radius-lg);
  padding: 1.5rem;
  text-decoration: none;
  color: inherit;
  transition: box-shadow 0.15s;
  cursor: pointer;
}

.alert-card:hover {
  box-shadow: var(--shadow-sm);
}

.alert-count {
  font-size: 2.25rem;
  font-weight: var(--font-bold);
  color: var(--color-danger);
  line-height: 1;
  margin-bottom: 0.25rem;
}

.alert-label {
  font-size: var(--text-base);
  font-weight: var(--font-semibold);
  color: var(--text-secondary);
  margin-bottom: 0.5rem;
}

.alert-hint {
  font-size: var(--text-sm);
  color: var(--text-faint);
}
```

### 5.6 OK State (No Alerts)

When no items require action, the callout section shows a positive confirmation:

```css
.integrity-ok {
  background: var(--color-success-subtle);
  border: 1px solid var(--color-success);
  border-radius: var(--radius-lg);
  padding: 1.25rem 1.5rem;
  color: var(--color-success-text);
  font-size: var(--text-base);
}
```

### 5.7 Layout

Pending review cards use a 2-column grid:

```css
.pending-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 1rem;
}
```

---

## 6. EdgeNodeGraphViewport (Sigma.js Chain View)

### 6.1 Purpose

`ChainViewModal.svelte` renders an interactive node-edge provenance graph using Sigma.js over WebGL. The graph visualizes the Forge data chain: **Source -> Bullet -> Perspective** (with future support for Resume Entry nodes). It supports dual rendering modes: full-screen modal and standalone inline page.

### 6.2 Component API

```typescript
interface ChainViewModalProps {
  /** Graph node key to highlight on open (e.g., "source-550e8400-..."). */
  highlightNode?: string | null

  /** When true, renders with modal chrome (backdrop, close button, escape key).
      When false, renders inline without modal wrapping. Default: true */
  isModal?: boolean

  /** Callback invoked when the modal is dismissed. */
  onClose?: () => void
}
```

### 6.3 Node Types

Each entity type has a distinct color for visual identification:

```typescript
export const NODE_COLORS: Record<ChainNode['type'], string> = {
  source:       '#6c63ff',  // purple
  bullet:       '#3b82f6',  // blue
  perspective:  '#10b981',  // green
  resume_entry: '#f59e0b',  // amber
}
```

```typescript
export interface ChainNode {
  id: string
  label: string
  type: 'source' | 'bullet' | 'perspective' | 'resume_entry'
  content: string
  status: string
  sourceType?: string  // for source nodes
  archetype?: string   // for perspective nodes
  domain?: string      // for perspective/bullet nodes
}
```

Node sizes vary by type: sources are largest (12px), bullets medium (8px), perspectives smallest (6px). This hierarchy visually communicates the derivation chain.

### 6.4 Edge Rendering

Edges are directed arrows connecting parent to child nodes. Two visual states exist:

| Edge State | Color | Width | Meaning |
|-----------|-------|-------|---------|
| Matching | `#94a3b8` (slate) | size: 1 (Sigma units, not CSS px) (size: 2 for primary) | Snapshot matches current content |
| Drifted | `#ef4444` (red) | size: 1 (Sigma units) | Snapshot no longer matches; integrity alert |

Primary source links (the canonical source for a bullet) render at size 2 (Sigma units). Non-primary links render at size 1. These are Sigma.js edge size values, not CSS pixels -- Sigma scales them based on zoom level.

### 6.5 Graph Layout

The graph uses **ForceAtlas2** layout via `graphology-layout-forceatlas2`:

```typescript
forceAtlas2.default.assign(g, {
  iterations: 100,
  settings: {
    gravity: 1,
    scalingRatio: 10,
    barnesHutOptimize: g.order > 100,
  },
})
```

Barnes-Hut optimization is enabled when the graph exceeds 100 nodes for performance.

### 6.6 Interaction Model

**Node click:** Selects the node, opens the detail panel, centers the camera on the node with animation. All other nodes dim (color fades to `#e5e7eb`, labels hidden). Connected edges highlight (thicker width, higher z-index).

**Edge click:** Selects the edge, shows source/target details and drift status in the detail panel.

**Edge hover:** Shows a floating tooltip at the edge midpoint with source/target labels and drift status.

**Stage click:** Deselects all nodes and edges.

**Camera:** `animate({ x, y, ratio: 0.3 }, { duration: 300 })` for smooth focus transitions.

### 6.7 Controls Toolbar

The controls bar provides:

1. **Search input:** Filters nodes by content/label text match (case-insensitive).
2. **Source type filter:** Dropdown: All / Roles / Projects / Education / Clearances / General.
3. **Status filter:** Dropdown: All / Draft / In Review / Approved / Rejected / Archived.
4. **Archetype filter:** Dropdown: All / Agentic AI / Infrastructure / Security Engineer / Solutions Architect / Public Sector / HFT.

Filters set `hidden` attribute on non-matching nodes; edges with hidden endpoints are also hidden via `edgeReducer`. Sigma refreshes after each filter change.

### 6.8 Controls CSS

```css
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
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-md);
  font-size: 0.85rem;
  font-family: inherit;
  color: var(--text-primary);
  background: var(--color-surface);
}

.search-input:focus {
  outline: none;
  border-color: var(--color-border-focus);
  box-shadow: 0 0 0 2px var(--color-primary-subtle);
}

.filter-select {
  padding: 0.45rem 0.75rem;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-md);
  font-size: 0.85rem;
  background: var(--color-surface);
  cursor: pointer;
  font-family: inherit;
  color: var(--text-primary);
}
```

### 6.9 Stats Bar

Displays node count, edge count, and drifted edge count. Includes a color-coded legend for node types and edge states.

```css
.stats-bar {
  display: flex;
  align-items: center;
  gap: 1.25rem;
  margin-bottom: 0.75rem;
  font-size: 0.8rem;
  color: var(--text-muted);
}

.drift-stat {
  color: var(--color-danger);
}

.legend {
  display: flex;
  gap: 0.75rem;
  margin-left: auto;
}

.legend-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
}
```

### 6.10 Graph Container Layout

```css
.graph-layout {
  display: flex;
  gap: 0;
  flex: 1;
  min-height: 0;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  overflow: hidden;
  background: var(--color-surface);
}

/* Standalone (non-modal) mode fills available viewport height */
.graph-layout-standalone {
  height: calc(100vh - 260px);
}

.graph-container {
  flex: 1;
  min-width: 0;
}
```

### 6.11 Detail Panel

When a node is selected, a 320px detail panel slides in from the right edge of the graph layout:

```css
.detail-panel {
  width: 320px;
  border-left: 1px solid var(--color-border);
  overflow-y: auto;
  flex-shrink: 0;
}

.detail-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 1rem;
  border-bottom: 1px solid var(--color-border);
  background: var(--color-surface-raised);
}

.node-type-icon {
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
```

The detail panel shows:
- Node type icon (colored square with initial letter)
- Label, type, status (via StatusBadge)
- Optional fields: source type, archetype, domain
- Full content text
- Entity ID (monospaced)
- Related entities section (bullets for sources, perspectives for bullets, parent bullet for perspectives) with clickable cards that navigate the graph

### 6.12 Edge Tooltip

The edge tooltip floats at the midpoint between the source and target node viewport coordinates. It renders outside `.graph-layout` to avoid `overflow: hidden` clipping:

```css
.edge-tooltip {
  position: fixed;
  z-index: var(--z-dropdown);
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  padding: 0.5rem 0.75rem;
  box-shadow: var(--shadow-md);
  pointer-events: none;
  transform: translate(-50%, -100%);
  margin-top: -8px;
}
```

### 6.13 Modal Mode

When `isModal={true}`, the component wraps in a backdrop overlay:

```css
.chain-modal-backdrop {
  position: fixed;
  inset: 0;
  z-index: var(--z-modal);
  background: var(--color-overlay);
  display: flex;
  flex-direction: column;
}

.chain-modal-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  margin: 1rem;
  background: var(--color-bg);
  border-radius: var(--radius-lg);
  overflow: hidden;
}
```

Modal features: body scroll lock (`overflow: hidden` on `<body>`), focus trap on backdrop, escape key dismissal, backdrop click dismissal with `stopPropagation` on content.

### 6.14 Sigma.js Configuration

```typescript
const instance = new Sigma(graph, container, {
  renderEdgeLabels: false,
  enableEdgeEvents: true,
  defaultNodeType: 'circle',
  defaultEdgeType: 'arrow',
  zIndex: true,
  labelRenderedSizeThreshold: 6,
  nodeReducer: (node, data) => { /* selection dimming logic */ },
  edgeReducer: (edge, data) => { /* filtering + selection logic */ },
})
```

Key settings:
- **`enableEdgeEvents: true`**: Required for hover/click on edges.
- **`zIndex: true`**: Enables z-ordering so selected nodes render on top.
- **`labelRenderedSizeThreshold: 6`**: Labels only render when node is large enough in current zoom.
- **Reducers:** Applied on every render frame. `nodeReducer` dims unselected nodes. `edgeReducer` hides edges with hidden endpoints and dims unrelated edges.

### 6.15 Cleanup

Dual cleanup strategy:
1. **`$effect` return:** Primary cleanup. Handles re-runs (e.g., Retry button triggers `buildGraph()` again).
2. **`onDestroy`:** Backup cleanup. Handles abrupt component unmount.

Both call `sigmaInstance.kill()` which releases WebGL resources.

---

## 7. RenderViewport

> **Status:** Design placeholder -- library decisions TBD. The rendering pipeline (remark/rehype vs. markdown-it, highlight.js vs. CodeMirror, pdf.js vs. iframe) has not been finalized. The API and CSS below are target specifications that may change based on library selection.

### 7.1 Purpose

An embedded renderer for document preview. Supports three output formats: rendered Markdown (GFM), syntax-highlighted LaTeX source, and embedded PDF viewing. Used in the resume editor "Source" tab and future note preview contexts.

### 7.2 Component API

```typescript
interface RenderViewportProps {
  /** The document content to render. */
  content: string

  /** Output format. Determines which renderer is used. */
  format: 'markdown' | 'latex' | 'pdf'

  /** CSS height for the viewport. Default: '500px' */
  height?: string

  /** When true, shows a loading overlay. Default: false */
  loading?: boolean
}
```

### 7.3 Rendering Strategy by Format

| Format | Renderer | Notes |
|--------|----------|-------|
| `markdown` | GFM-compatible pipeline (remark/rehype or similar) | Renders to sanitized HTML. Supports tables, task lists, code blocks with syntax highlighting. |
| `latex` | Syntax-highlighted source view (highlight.js or CodeMirror) | Read-only code display. Does not compile LaTeX -- shows source with TeX keyword highlighting. |
| `pdf` | Embedded viewer (`<iframe>` with data URL or pdf.js) | `content` is a base64-encoded PDF or a blob URL. |

### 7.4 CSS

```css
.render-viewport {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  overflow: auto;
}

.render-viewport--markdown {
  padding: 1.5rem;
  font-family: var(--font-sans);
  font-size: var(--text-base);
  color: var(--text-primary);
  line-height: var(--leading-normal);
}

.render-viewport--latex {
  padding: 0;
  font-family: var(--font-mono);
  font-size: var(--text-sm);
}

.render-viewport--pdf {
  padding: 0;
}

.render-viewport--pdf iframe {
  width: 100%;
  height: 100%;
  border: none;
}
```

### 7.5 Markdown Typography

Rendered Markdown inherits design tokens for consistent typography:

```css
.render-viewport--markdown h1 { font-size: var(--text-2xl); font-weight: var(--font-bold); margin: 1.5rem 0 0.75rem; }
.render-viewport--markdown h2 { font-size: var(--text-xl); font-weight: var(--font-semibold); margin: 1.25rem 0 0.5rem; }
.render-viewport--markdown h3 { font-size: var(--text-lg); font-weight: var(--font-semibold); margin: 1rem 0 0.5rem; }
.render-viewport--markdown p { margin: 0.5rem 0; }
.render-viewport--markdown code {
  font-family: var(--font-mono);
  background: var(--color-surface-sunken);
  padding: 0.15rem 0.35rem;
  border-radius: var(--radius-sm);
  font-size: 0.9em;
}
.render-viewport--markdown pre {
  background: var(--color-surface-sunken);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  padding: 1rem;
  overflow-x: auto;
}
.render-viewport--markdown table {
  width: 100%;
  border-collapse: collapse;
  margin: 0.75rem 0;
}
.render-viewport--markdown th,
.render-viewport--markdown td {
  border: 1px solid var(--color-border);
  padding: 0.5rem 0.75rem;
  text-align: left;
}
.render-viewport--markdown th {
  background: var(--color-surface-raised);
  font-weight: var(--font-semibold);
}
```

---

## 8. Chart Component Catalog

Each domain-specific chart component follows the same pattern: fetch data in `onMount`, build an `EChartsOption` via a utility module, and pass it to `<EChart>`. The utility module is independently testable (pure functions, no DOM dependency).

### 8.1 Pattern

```
ComponentName.svelte          # Svelte component: data fetch, state, EChart composition
component-name-utils.ts       # Pure functions: data transformation + option building
__tests__/component-name-utils.test.ts  # Unit tests for the utility module
```

### 8.2 Catalog

| Component | Chart Type | Utility Module | Data Source | Self-Wrapping | Key Features |
|-----------|-----------|----------------|-------------|---------------|--------------|
| **SkillsSunburst** | Sunburst + Pie | `skills-chart-utils.ts` | Skills + Bullets + Perspectives | Yes (.chart-card) | Drill-down from category to pie; domain-archetype nested pie |
| **SkillsTreemap** | Treemap | `treemap-utils.ts` | Skills + Bullets | Yes (.chart-card) | Category grouping; zoom-to-node; breadcrumb |
| **BulletsTreemap** | Treemap | `treemap-utils.ts` | Bullets + Perspectives | Yes (.chart-card) | Grouped by primary source; sized by perspective count |
| **DomainsTreemap** | Treemap | `treemap-utils.ts` | Perspectives | Yes (.chart-card) | Single-level; sized by perspective count |
| **ApplicationGantt** | Custom (renderItem) | `gantt-utils.ts` | Job Descriptions | No | Horizontal bars; status-colored; dashed right edge for active; Y-axis dataZoom |
| **RoleChoropleth** | Map | `choropleth-utils.ts` | Job Descriptions + GeoJSON | No | US state map; continuous indigo visualMap; click-to-select state; remote/unknown badges |
| **JDSkillRadar** | Radar / Bar (fallback) | `jd-radar-utils.ts` | JD Skills + User Skills + Bullets | No | Spider chart with fallback to bar when < 3 categories; gap list |
| **CompensationBulletGraph** | Bar (stacked) | `compensation-utils.ts` | JD salary + Profile expectations | No | Bullet graph with qualitative bands (markArea) and reference lines (markLine) |

### 8.3 Font Size Divergences

`JDSkillRadar` and `CompensationBulletGraph` use `--font-size-sm`, `--font-size-xs`, and `--font-size-base` in their utility modules, but these tokens do not exist in `tokens.css`. The correct tokens are `--text-sm`, `--text-xs`, and `--text-base`. These divergences should be fixed in the respective `-utils.ts` files. Note that ECharts option builders resolve font sizes as raw pixel values (e.g., `12`, `14`) rather than CSS `var()` references, so the mapping happens in the utility module, not in CSS.

### 8.4 Data Fetching Rule

All chart components use `onMount()` for data fetching, **not** `$effect()`. Rationale: data fetching writes to reactive state (`$state`), which `$effect` would re-track, causing infinite reactive loops. The only exception is `JDSkillRadar`, which uses `$effect` keyed on `jdId` to reload when the selected JD changes -- this is safe because `jdId` is a prop, not internal state.

### 8.5 Utility Module Contract

Every `*-utils.ts` module exports:

1. **TypeScript interfaces** for its data structures.
2. **A data transformation function** (`buildXyzData(...)`) that takes SDK entity arrays and returns chart-ready data.
3. **An option builder function** (`buildXyzOption(...)`) that takes chart-ready data and returns an `EChartsOption`.
4. **Optionally**, a mock data generator for development/testing.

These functions are pure (no side effects, no DOM access). They can be tested with `vitest` in Node without browser context.

---

## 9. Dashboard Layout Pattern

### 9.1 Page Structure

The dashboard (`/+page.svelte`) is the primary composition surface for visualization components. It follows this section order:

1. **Pending Review** -- MetricCalloutContainers (warning accent, 2-column grid)
2. **Integrity Alerts** -- MetricCalloutContainer (danger accent) or success confirmation
3. **Quick Stats** -- MetricContainers (5-column grid)
4. **Skill & Domain Distribution** -- SkillsSunburst (sunburst + domain-archetype pie)
5. **Treemap Views** -- SkillsTreemap + BulletsTreemap + DomainsTreemap (vertical stack)
6. **Application Timeline** -- ApplicationGantt (in chart-card wrapper)
7. **Opportunity Map** -- RoleChoropleth (in chart-card wrapper)

### 9.2 Section Pattern

Each dashboard section follows a consistent pattern:

```svelte
<section class="section">
  <h2 class="section-title">Section Name</h2>
  <!-- Section content: grids, chart cards, callouts -->
</section>
```

```css
.section {
  margin-bottom: 2rem;
}

.section-title {
  font-size: var(--text-sm);
  font-weight: var(--font-semibold);
  color: var(--text-secondary);
  margin-bottom: 0.75rem;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}
```

### 9.3 Chart Card Wrapper

Charts that are not self-wrapping use a `.chart-card` container:

```css
.chart-card {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  padding: 1rem;
}
```

Some chart components (SkillsSunburst, treemaps) include their own `.chart-card` wrapper internally. The dashboard should not double-wrap.

### 9.4 Dashboard Container

```css
.dashboard {
  max-width: 900px;
}
```

No explicit padding -- the `ContentArea` container from the layout provides `2rem` padding (see Doc 2).

### 9.5 Page-Level Loading and Error States

The dashboard uses a single top-level loading/error state for the initial API calls. Individual chart components manage their own loading states independently.

```svelte
{#if loading}
  <div class="loading-container">
    <LoadingSpinner size="lg" message="Loading dashboard..." />
  </div>
{:else if error}
  <div class="error-banner">
    <p>{error}</p>
    <button class="retry-btn" onclick={loadData}>Retry</button>
  </div>
{:else}
  <!-- Dashboard sections -->
{/if}
```

---

## 10. Loading States

### 10.1 Three-Tier Loading Strategy

Forge uses three tiers of loading indication depending on context:

| Tier | Component | When Used |
|------|-----------|-----------|
| **Page-level spinner** | `LoadingSpinner` (size="lg") | Initial page data fetch (dashboard, chain view) |
| **Chart-level loading** | ECharts built-in `showLoading()` | Chart data is fetching while the page shell is already visible |
| **Inline text** | `<div class="loading-text">` | Chart components that render their own loading message |

### 10.2 LoadingSpinner Integration

```svelte
<LoadingSpinner size="lg" message="Loading dashboard..." />
```

The spinner uses `--color-primary` for its animation color. It centers in its container:

```css
.loading-container {
  display: flex;
  justify-content: center;
  padding: 4rem 0;
}
```

### 10.3 ECharts Loading Overlay

EChart.svelte manages the built-in loading spinner:

```typescript
chart.showLoading('default', {
  text: '',
  color: getComputedStyle(chartEl)
    .getPropertyValue('--color-primary').trim() || '#6c63ff',
  maskColor: 'rgba(255, 255, 255, 0.7)',
})
```

- Text is empty (no "Loading..." label inside the chart).
- Spinner color reads from `--color-primary` token.
- Mask is semi-transparent white (`rgba(255, 255, 255, 0.7)`). **Dark mode note:** This mask color does not adapt to dark mode. In dark mode, the white mask creates a visible flash. A future fix should resolve `--color-surface` at render time and use it with opacity for a theme-aware mask: `maskColor: resolveTokenColor('--color-surface', '#ffffff') + 'b3'` (where `b3` is ~70% opacity hex).

### 10.4 Chart Component Loading Pattern

Each chart component handles its own loading state independently of the dashboard:

```svelte
{#if loading}
  <div class="chart-loading">Loading chart data...</div>
{:else if error}
  <div class="chart-error">{error}</div>
{:else if items.length === 0}
  <div class="chart-empty">
    No data yet. Create items to populate this chart.
  </div>
{:else}
  <EChart option={chartOption} ... />
{/if}
```

```css
.chart-loading,
.chart-empty {
  text-align: center;
  padding: 2rem 1rem;
  color: var(--text-muted);
  font-size: var(--text-sm);
}

.chart-error {
  text-align: center;
  padding: 2rem 1rem;
  color: var(--color-danger-text);
  font-size: var(--text-sm);
}
```

### 10.5 Error State with Retry

Error banners provide a retry button for recoverable failures:

```css
.error-banner {
  background: var(--color-danger-subtle);
  border: 1px solid var(--color-danger);
  border-radius: var(--radius-lg);
  padding: 1.25rem 1.5rem;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  color: var(--color-danger-text);
  font-size: var(--text-base);
}

.retry-btn {
  padding: 0.4rem 1rem;
  background: var(--color-danger);
  color: var(--text-inverse);
  border: none;
  border-radius: var(--radius-md);
  font-size: var(--text-sm);
  font-weight: var(--font-medium);
  cursor: pointer;
  white-space: nowrap;
  transition: background 0.15s;
}

.retry-btn:hover {
  background: var(--color-danger-hover);
}
```

### 10.6 Empty State

Chart components use `EmptyState` for zero-data scenarios:

```svelte
<EmptyState
  title="No skill data"
  description="Add skills to sources and derive bullets to see distributions."
/>
```

Or simpler inline text for chart-specific empty messages:

```svelte
<div class="chart-empty">
  No job descriptions yet. Create one to start tracking your application timeline.
</div>
```

---

## 11. Composition Examples

### 11.1 Adding a New ECharts Visualization

To add a new chart type (e.g., a bar chart for monthly application counts):

**Step 1: Register the chart type** (if not already registered):
```typescript
// echarts-registry.ts -- BarChart is already registered
```

**Step 2: Create the utility module:**
```typescript
// charts/monthly-apps-utils.ts
import type { EChartsOption } from 'echarts/core'

export interface MonthlyAppData {
  month: string
  count: number
}

export function buildMonthlyAppsData(jds: JDLike[]): MonthlyAppData[] {
  // Group by month, count
}

export function buildMonthlyAppsOption(data: MonthlyAppData[]): EChartsOption {
  return {
    tooltip: { trigger: 'axis' },
    xAxis: { type: 'category', data: data.map(d => d.month) },
    yAxis: { type: 'value' },
    series: [{ type: 'bar', data: data.map(d => d.count) }],
  }
}
```

**Step 3: Create the Svelte component:**
```svelte
<!-- charts/MonthlyApps.svelte -->
<script lang="ts">
  import { onMount } from 'svelte'
  import { forge } from '$lib/sdk'
  import EChart from './EChart.svelte'
  import { buildMonthlyAppsData, buildMonthlyAppsOption } from './monthly-apps-utils'

  let loading = $state(true)
  let data = $state([])

  onMount(() => { loadData() })

  async function loadData() {
    loading = true
    const result = await forge.jobDescriptions.list({ limit: 500 })
    if (result.ok) data = buildMonthlyAppsData(result.data)
    loading = false
  }

  let chartOption = $derived(
    data.length > 0 ? buildMonthlyAppsOption(data) : null
  )
</script>

{#if loading}
  <div class="chart-loading">Loading...</div>
{:else if chartOption}
  <EChart option={chartOption} height="300px" />
{:else}
  <div class="chart-empty">No data available.</div>
{/if}
```

**Step 4: Add to dashboard:**
```svelte
<section class="section">
  <h2 class="section-title">Monthly Applications</h2>
  <div class="chart-card">
    <MonthlyApps />
  </div>
</section>
```

### 11.2 Theming a Custom Chart Element

When a chart option builder needs to use token colors for non-series elements (graphic overlays, custom annotations):

```typescript
// WRONG -- module scope, tokens not available yet
const bgColor = resolveTokenColor('--color-surface', '#fff')

// CORRECT -- called inside onMount or derived computation
function buildOptionAtRenderTime(): EChartsOption {
  const labelColor = resolveTokenColor('--text-primary', '#1a1a2e')
  return {
    graphic: [{
      type: 'text',
      left: 'center',
      top: '50%',
      style: {
        text: '42\nskills',
        textAlign: 'center',
        fontSize: 18,
        fontWeight: 700,
        fill: labelColor,
      },
    }],
    // ...
  }
}
```

### 11.3 Chart with Event Forwarding

```svelte
<EChart
  option={chartOption}
  height="400px"
  notMerge={true}
  onChartEvent={{
    click: (params) => {
      if (params.componentType === 'series') {
        goto(`/detail/${params.data.id}`)
      }
    },
    mouseover: (params) => {
      highlightedItem = params.dataIndex
    },
  }}
/>
```

### 11.4 Dashboard with Responsive Stats

```svelte
<section class="section">
  <h2 class="section-title">Quick Stats</h2>
  <div class="stats-grid">
    <div class="stat-card">
      <div class="stat-number">{totalSources}</div>
      <div class="stat-label">Sources</div>
    </div>
    <div class="stat-card">
      <div class="stat-number">{totalBullets}</div>
      <div class="stat-label">Bullets</div>
    </div>
    <div class="stat-card">
      <div class="stat-number">{totalPerspectives}</div>
      <div class="stat-label">Perspectives</div>
    </div>
    <div class="stat-card">
      <div class="stat-number">{totalOrganizations}</div>
      <div class="stat-label">Organizations</div>
    </div>
    <div class="stat-card">
      <div class="stat-number">{totalResumes}</div>
      <div class="stat-label">Resumes</div>
    </div>
  </div>
</section>
```

---

## 12. Accessibility

### 12.1 Chart Accessibility

ECharts SVG renderer produces DOM elements that can be targeted by screen readers, though the experience is limited. Mitigations:

- **Tooltips** provide textual detail for every data point on hover/focus.
- **Legends** are keyboard-navigable within ECharts.
- **Alternative text:** Chart components should include a visually hidden summary when the data is simple enough (e.g., "5 sources, 23 bullets, 41 perspectives").

### 12.2 Graph Accessibility

Sigma.js renders to WebGL canvas, which is inherently inaccessible. Mitigations:

- **Stats bar** provides textual summary (node count, edge count, drift count).
- **Search input** allows finding nodes by text.
- **Detail panel** provides full node information in accessible HTML.
- **Keyboard support:** Escape to close modal, tab to controls.

### 12.3 Modal Accessibility

ChainViewModal in modal mode:
- `role="dialog"` and `aria-modal="true"` on the content container.
- `aria-labelledby="chain-modal-title"` references the heading.
- Focus is moved to the backdrop on open.
- Escape key dismisses.
- Backdrop click dismisses.
- Body scroll is locked.

---

## 13. Performance

### 13.1 ECharts Bundle Size

Tree-shaking via `echarts/core` + individual chart/component imports keeps the bundle to the chart types actually used. The SVG renderer is smaller than Canvas.

### 13.2 Sigma.js

- Dynamic import (`import('sigma')`) ensures Sigma.js is only loaded when the chain view is opened.
- `graphology-layout-forceatlas2` is also dynamically imported.
- Barnes-Hut optimization is enabled for graphs > 100 nodes.
- WebGL resources are released on cleanup via `sigmaInstance.kill()`.

### 13.3 GeoJSON

The US states GeoJSON (`/geo/us-states.json`) is fetched on demand and cached via a module-level flag (`mapLoaded`). `echarts.registerMap('USA', geoJson)` is a global operation -- once registered, it persists across mount/unmount cycles.

### 13.4 Data Volume Limits

Chart components fetch with generous limits (`limit: 500` for JDs, `limit: 2000` for bullets, `limit: 5000` for perspectives). These limits are adequate for a single-user resume builder. If data volumes grow significantly, pagination or server-side aggregation would be needed.

---

## 14. Token Gaps for Visualization

The following tokens are not yet defined but would improve theming consistency:

| Gap | Current Workaround | Recommendation |
|-----|-------------------|----------------|
| Node colors for graph types | Hardcoded hex in `NODE_COLORS` | Add `--color-graph-source`, `--color-graph-bullet`, `--color-graph-perspective`, `--color-graph-entry` |
| Edge colors | Hardcoded `#94a3b8` (matching) and `#ef4444` (drifted) | Reuse `--color-border` for matching, `--color-danger` for drifted |
| Status colors for Gantt | Hardcoded hex in `STATUS_COLORS` | Add `--color-pipeline-{status}` tokens |
| Choropleth map colors | Hardcoded indigo gradient `['#e0e7ff', '#818cf8', '#4f46e5', '#312e81']` | Add `--color-map-range-{1..4}` tokens |
| Selection dimming color | Hardcoded `#e5e7eb` / `#f3f4f6` | Reuse `--color-border` / `--color-surface-sunken` |

These gaps are documented for future token expansion. The current hardcoded values are acceptable during the migration period (see ADR-004 in Doc 1).

---

## 15. Known Bugs

### 15.1 Pending-Card `href` Bug

On the dashboard, both pending review cards (Pending Bullets and Pending Perspectives) link to the bullets page. The Pending Perspectives card should link to the perspectives page instead. The `href` on the perspectives pending-card should be updated to point to the correct route.

---

## Acceptance Criteria

1. Charts render correctly in both light and dark mode. Theme switching triggers chart re-initialization with fresh token colors.
2. Loading overlay uses `--color-primary` for spinner color, resolved at render time via `getComputedStyle`.
3. RenderViewport renders GFM-compatible Markdown with design-token-based typography.
4. All chart components handle loading, error, and empty states independently with appropriate visual feedback.
5. EChart SVG renderer is used exclusively -- no Canvas renderer in the bundle.
6. Chart utility modules are pure functions testable with Vitest in Node (no DOM dependency).
7. Node colors in the graph use hardcoded hex values documented as a token gap (future: `--color-graph-*` tokens).
8. Sigma.js is dynamically imported to avoid bundle size impact on non-graph pages.
9. Dashboard pending-card href bug is documented as a known issue.
10. Font-size divergences in JDSkillRadar and CompensationBulletGraph utility modules are documented for remediation.
