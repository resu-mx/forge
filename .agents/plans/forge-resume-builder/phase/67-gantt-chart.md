# Phase 67: Application Gantt Chart (Spec J4)

**Status:** Planning
**Date:** 2026-04-03
**Spec:** [2026-04-03-gantt-chart.md](../refs/specs/2026-04-03-gantt-chart.md)
**Depends on:** Phase 59 (ECharts Infrastructure — `EChart.svelte` wrapper, `echarts-registry.ts` with `CustomChart` registered), Phase 61 (JD Pipeline — `JobDescriptionStatus` with 9 pipeline statuses)
**Blocks:** None
**Parallelizable with:** Phase 65, Phase 66, Phase 68 -- creates new files, only modifies dashboard `+page.svelte`

## Goal

Add a Gantt-style chart to the dashboard showing JD application timelines. Each row represents a job description with a horizontal bar spanning `created_at` to `updated_at` (or "now" for active JDs), colored by current pipeline status. Active JDs have a dashed right edge (two-shape group: solid rect + dashed line) indicating the bar is still growing. The chart supports Y-axis scrolling via `dataZoom` (with `filterMode: 'weakFilter'`) when more than 10 JDs exist, and clicking a bar navigates to the JD detail page.

## Non-Goals

- Full status history tracking (per-stage bar segments -- deferred, schema documented in spec Section 6)
- Per-stage duration analytics or metrics
- Deadline tracking or SLA alerts
- Calendar integration
- Drag-and-drop status changes from the chart
- Custom zoom/pan controls (ECharts native scroll zoom is sufficient)
- Grouping by organization
- Mobile-specific interactions, chart export, accessibility

## Context

The MVP Gantt renders a single bar per JD because the current schema only stores the current status and `created_at`/`updated_at` timestamps -- not a history of status changes. The full timeline breakdown (bar segments per stage) requires a `jd_status_history` table documented in spec Section 6 but deferred to a future phase.

The chart uses ECharts `custom` series type (already registered by Phase 59) with `renderItem` for full control over bar rendering. The two-shape group technique (solid rect + dashed line on right edge) is required because ECharts SVG renderer applies `lineDash` to the entire shape border, not just one edge.

Status colors reuse the accent colors from Phase 61's pipeline column definitions.

## Scope

| Spec section | Covered here? |
|-------------|---------------|
| 1. MVP Chart Design (layout, mockup, status colors, active/terminal, click, sorting, scrolling) | Yes |
| 2. Data Loading (SDK call, transformation to `GanttItem[]`) | Yes |
| 3. ECharts Configuration (option builder, custom renderItem, legend) | Yes |
| 4. Component (`ApplicationGantt.svelte`, styling, dynamic height) | Yes |
| 5. Dashboard Integration (placement, import) | Yes |
| 6. Future Status History Table | No (informational only, documented in spec) |
| 7. Mock Data | Yes (development utility) |
| 8-9. Files to create/modify | Yes |
| 10. Testing | Yes |
| 11. Acceptance criteria | Yes |

## Files to Create

| File | Description |
|------|-------------|
| `packages/webui/src/lib/components/charts/gantt-utils.ts` | `buildGanttItems`, `buildGanttOption`, `STATUS_COLORS`, `TERMINAL_STATUSES`, `generateMockGanttItems` |
| `packages/webui/src/lib/components/charts/ApplicationGantt.svelte` | Gantt chart component with loading, error, empty, and chart states; includes HTML legend |
| `packages/webui/src/lib/components/charts/__tests__/gantt-utils.test.ts` | Unit tests for data transformation and option building (17 cases) |

## Files to Modify

| File | Change |
|------|--------|
| `packages/webui/src/routes/+page.svelte` | Import `ApplicationGantt` and add "Application Timeline" section inside the `{:else}` block |

No changes to `echarts-registry.ts` -- `CustomChart` is already registered by Phase 59.

## Fallback Strategies

- **Zero JDs:** Component shows a clear empty state ("No job descriptions yet. Create one to start tracking your application timeline."). No chart rendered.
- **All JDs created on the same date:** `minTime === maxTime`, so `padding` falls back to `86400000` (1 day) to avoid a zero-width time axis.
- **JD with no organization name:** The Y-axis label shows only the JD title (no second line). The `orgName` check in category label construction handles this.
- **JD title longer than 30 characters:** Truncated to 30 chars with `...` suffix. The Y-axis `axisLabel` also applies `overflow: 'truncate'` at 150px width as a secondary guard.
- **More than 10 JDs:** Y-axis `dataZoom` activates with `filterMode: 'weakFilter'` (not `'none'`). `weakFilter` correctly handles custom series data where the X-axis range and Y-axis category range both need to be evaluated. Using `'none'` would disable filtering entirely and could cause rendering artifacts.
- **ECharts `renderItem` with dashed right edge:** Uses a two-shape group (solid rect + dashed line) instead of a single rect with `lineDash`. ECharts SVG renderer applies `lineDash` to the entire rect border, not just the right edge. The group approach draws a solid-bordered rect body, then overlays a dashed vertical line on the right edge only.
- **API failure:** The component sets an error state and shows an error message. No crash.
- **Custom series tooltip:** Uses `params.value[0]` (the explicit category index from the data array) for item lookup, not `params.dataIndex`. In custom series, `params.dataIndex` can differ from the logical category index when `dataZoom` filtering is active.
- **Multiline Y-axis labels:** Multiline Y-axis labels (`title\norgName`) combined with `overflow: 'truncate'` have inconsistent behavior across ECharts renderers. If truncation fails visually, truncate `orgName` to 20 chars in `buildGanttItems` before combining.

---

## Tasks

### T67.1: Write Gantt Data Transformation and Option Builder

**File:** `packages/webui/src/lib/components/charts/gantt-utils.ts`

[IMPORTANT] Status colors reuse the accent colors from Phase 61's pipeline definitions. The `STATUS_COLORS` map is exported for use by both the option builder and the HTML legend.

[CRITICAL] Gantt two-shape group: The `renderItem` function returns a `{ type: 'group', children: [...] }` containing a solid rect for the bar body and (for active items only) a dashed vertical line on the right edge. Do NOT use `lineDash` on the rect itself -- ECharts applies it to all four edges.

[CRITICAL] Gantt tooltip: Use `params.value[0]` (the explicit category index stored in the data array) for item lookup, not `params.dataIndex`. When `dataZoom` filtering is active, `params.dataIndex` may not correspond to the visible item index.

[CRITICAL] Gantt dataZoom: Use `filterMode: 'weakFilter'`, not `'none'`. `weakFilter` correctly evaluates both X and Y dimensions for custom series data, while `'none'` disables all filtering and can cause rendering artifacts with large datasets.

```typescript
export interface GanttItem {
  id: string
  title: string
  orgName: string | null
  status: string
  startDate: Date
  endDate: Date      // updated_at for terminal, now for active
  isActive: boolean  // true if status is non-terminal
  color: string
}

export const STATUS_COLORS: Record<string, string> = {
  discovered:   '#a5b4fc',  // light purple
  analyzing:    '#60a5fa',  // blue
  applying:     '#fbbf24',  // amber
  applied:      '#818cf8',  // indigo
  interviewing: '#a78bfa',  // purple
  offered:      '#22c55e',  // green
  rejected:     '#f87171',  // red
  withdrawn:    '#fb923c',  // orange
  closed:       '#d1d5db',  // dark gray
}

export const TERMINAL_STATUSES = new Set(['rejected', 'withdrawn', 'closed'])

/**
 * Transform JD list into Gantt chart items.
 * Sorts by created_at descending (newest first).
 */
export function buildGanttItems(jds: JobDescriptionWithOrg[]): GanttItem[] {
  const now = new Date()

  return jds
    .map(jd => {
      const isActive = !TERMINAL_STATUSES.has(jd.status)
      return {
        id: jd.id,
        title: jd.title.length > 30 ? jd.title.slice(0, 30) + '...' : jd.title,
        orgName: jd.organization_name ?? null,
        status: jd.status,
        startDate: new Date(jd.created_at),
        endDate: isActive ? now : new Date(jd.updated_at),
        isActive,
        color: STATUS_COLORS[jd.status] ?? '#d1d5db',
      }
    })
    .sort((a, b) => b.startDate.getTime() - a.startDate.getTime())
}

/**
 * Build ECharts option for the Gantt chart.
 * Uses custom series with renderItem for two-shape group bars.
 */
export function buildGanttOption(items: GanttItem[]): EChartsOption {
  const categories = items.map(item => {
    const label = item.orgName
      ? `${item.title}\n${item.orgName}`
      : item.title
    return label
  })

  // Compute time range
  const allDates = items.flatMap(i => [i.startDate, i.endDate])
  const minTime = Math.min(...allDates.map(d => d.getTime()))
  const maxTime = Math.max(...allDates.map(d => d.getTime()))
  const padding = (maxTime - minTime) * 0.05 || 86400000 // 5% padding or 1 day

  return {
    tooltip: {
      trigger: 'item',
      formatter: (params: any) => {
        // Use params.value[0] (the explicit category index from the data array)
        // instead of params.dataIndex for robust item lookup in custom series.
        const item = items[params.value[0]]
        if (!item) return ''
        const start = item.startDate.toLocaleDateString()
        const end = item.isActive ? 'ongoing' : item.endDate.toLocaleDateString()
        const days = Math.ceil(
          (item.endDate.getTime() - item.startDate.getTime()) / 86400000
        )
        return [
          `<strong>${item.title}</strong>`,
          item.orgName ? item.orgName : '',
          `Status: ${item.status}`,
          `${start} — ${end} (${days} days)`,
        ].filter(Boolean).join('<br/>')
      },
    },
    grid: {
      left: '20%',
      right: '5%',
      top: '5%',
      bottom: items.length > 10 ? '15%' : '10%',
    },
    xAxis: {
      type: 'time',
      min: minTime - padding,
      max: maxTime + padding,
      axisLabel: {
        fontSize: 10,
      },
    },
    yAxis: {
      type: 'category',
      data: categories,
      inverse: true,   // newest at top
      axisLabel: {
        fontSize: 10,
        width: 150,
        overflow: 'truncate',
      },
    },
    dataZoom: items.length > 10
      ? [
          {
            type: 'slider',
            yAxisIndex: 0,
            right: 0,
            width: 20,
            startValue: 0,
            endValue: 9,
            filterMode: 'weakFilter',
          },
        ]
      : [],
    series: [
      {
        type: 'custom',
        renderItem: (params: any, api: any) => {
          const categoryIndex = api.value(0)
          const start = api.coord([api.value(1), categoryIndex])
          const end = api.coord([api.value(2), categoryIndex])
          const height = api.size([0, 1])[1] * 0.6
          const item = items[categoryIndex]
          const barWidth = Math.max(end[0] - start[0], 4) // min 4px width

          // Two-shape group: solid rect for bar body + dashed line on right edge.
          // ECharts SVG renderer applies lineDash to the entire shape border,
          // not just one edge, so a single rect with lineDash won't work.
          const children: any[] = [
            {
              type: 'rect',
              shape: {
                x: start[0],
                y: start[1] - height / 2,
                width: barWidth,
                height: height,
                r: 3,
              },
              style: {
                fill: item?.color ?? '#d1d5db',
              },
            },
          ]

          // Dashed right edge for active items
          if (item?.isActive) {
            children.push({
              type: 'line',
              shape: {
                x1: start[0] + barWidth,
                y1: start[1] - height / 2,
                x2: start[0] + barWidth,
                y2: start[1] + height / 2,
              },
              style: {
                stroke: item.color,
                lineWidth: 2,
                lineDash: [4, 4],
              },
            })
          }

          return {
            type: 'group',
            children,
          }
        },
        encode: {
          x: [1, 2],
          y: 0,
        },
        data: items.map((item, index) => [
          index,
          item.startDate.getTime(),
          item.endDate.getTime(),
        ]),
      },
    ],
  }
}

/**
 * Generate mock Gantt items for development/testing.
 */
export function generateMockGanttItems(count: number = 8): GanttItem[] {
  const statuses = [
    'discovered', 'analyzing', 'applying', 'applied',
    'interviewing', 'offered', 'rejected', 'withdrawn', 'closed',
  ]
  const companies = [
    'Cloudflare', 'Acme Corp', 'BigTech Inc', 'Startup Co',
    'MegaCorp', 'CloudSec', 'DataFlow', 'CyberGuard',
  ]
  const titles = [
    'Sr Security Engineer', 'DevOps Lead', 'Cloud Architect',
    'Platform Engineer', 'Staff SRE', 'Security Architect',
    'Infrastructure Lead', 'DevSecOps Manager',
  ]

  const now = new Date()
  return Array.from({ length: count }, (_, i) => {
    const daysAgo = 90 - i * 10 + Math.floor(Math.random() * 10)
    const startDate = new Date(now.getTime() - daysAgo * 86400000)
    const status = statuses[i % statuses.length]
    const isActive = !TERMINAL_STATUSES.has(status)
    const duration = (30 + Math.random() * 30) * 86400000
    // Clamp terminal item endDate to now — mock data should not produce future dates.
    const endDate = isActive
      ? now
      : new Date(Math.min(startDate.getTime() + duration, Date.now()))

    return {
      id: `mock-${i}`,
      title: titles[i % titles.length],
      orgName: companies[i % companies.length],
      status,
      startDate,
      endDate,
      isActive,
      color: STATUS_COLORS[status] ?? '#d1d5db',
    }
  })
}
```

**Acceptance criteria:**
- `buildGanttItems` sorts by `created_at` descending.
- `buildGanttItems` truncates titles > 30 chars with `...`.
- `buildGanttItems` sets `isActive = true` for non-terminal, `false` for terminal.
- `buildGanttItems` uses `new Date()` for active end dates, `updated_at` for terminal.
- `buildGanttItems` maps status to correct `STATUS_COLORS` color.
- `buildGanttOption` produces valid custom series with `renderItem`.
- `buildGanttOption` includes `dataZoom` with `filterMode: 'weakFilter'` for > 10 items.
- `buildGanttOption` omits `dataZoom` for <= 10 items.
- `buildGanttOption` sets `yAxis.inverse = true`.
- `buildGanttOption` computes time axis range with 5% padding.
- `renderItem` returns two-shape group (rect + dashed line) for active items.
- `renderItem` returns single-shape group (rect only) for terminal items.
- Tooltip uses `params.value[0]` for item lookup.
- `generateMockGanttItems` produces the requested number of items.

**Failure criteria:**
- `lineDash` applied to rect instead of using two-shape group (all four edges dashed).
- `params.dataIndex` used instead of `params.value[0]` (wrong item in tooltip when scrolled).
- `filterMode: 'none'` used instead of `'weakFilter'` (rendering artifacts).
- Time padding divides by zero when all JDs have the same date.

---

### T67.2: Write ApplicationGantt Component

**File:** `packages/webui/src/lib/components/charts/ApplicationGantt.svelte`

[IMPORTANT] Use `onMount` for data loading, not `$effect`. The component loads data once on mount -- there are no reactive props that would trigger a reload. Using `$effect` with async state writes would risk infinite reactive loops.

[IMPORTANT] Chart height scales dynamically: `max(200px, min(items, 10) * 50px + 80px)`. This prevents the chart from being too short with few items or too tall with many (capped at 580px with Y-axis scrolling).

[IMPORTANT] Clicking a bar navigates to the JD detail page using `goto()`. The click handler uses `params.value[0]` (explicit category index) for robust item lookup.

```svelte
<script lang="ts">
  import { onMount } from 'svelte'
  import { forge } from '$lib/sdk'
  import { goto } from '$app/navigation'
  import EChart from './EChart.svelte'
  import { buildGanttItems, buildGanttOption, STATUS_COLORS } from './gantt-utils'
  import type { GanttItem } from './gantt-utils'

  let loading = $state(true)
  let items = $state<GanttItem[]>([])
  let error = $state<string | null>(null)

  // Use onMount instead of $effect to avoid infinite reactive loops:
  // loadData() writes to reactive state (items, loading, error)
  // which $effect would re-track, causing re-execution.
  onMount(() => { loadData() })

  async function loadData() {
    loading = true
    error = null
    try {
      const result = await forge.jobDescriptions.list({ limit: 500 })
      if (result.ok) {
        items = buildGanttItems(result.data)
      } else {
        error = 'Failed to load job descriptions'
      }
    } catch {
      error = 'An error occurred loading application data'
    } finally {
      loading = false
    }
  }

  let chartOption = $derived(
    items.length > 0 ? buildGanttOption(items) : null
  )

  let chartHeight = $derived(
    `${Math.max(200, Math.min(items.length, 10) * 50 + 80)}px`
  )

  function handleChartClick(params: any) {
    // Use params.value[0] (explicit category index) for robust item lookup
    const item = items[params.value?.[0]]
    if (item) {
      goto(`/opportunities/job-descriptions?selected=${item.id}`)
    }
  }
</script>

{#if loading}
  <div class="gantt-loading">Loading application timeline...</div>
{:else if error}
  <div class="gantt-error">{error}</div>
{:else if items.length === 0}
  <div class="gantt-empty">
    No job descriptions yet. Create one to start tracking your application timeline.
  </div>
{:else}
  <div class="gantt-container">
    <EChart
      option={chartOption!}
      height={chartHeight}
      notMerge={true}
      onChartEvent={{ click: handleChartClick }}
    />
    <div class="gantt-legend">
      {#each Object.entries(STATUS_COLORS) as [status, color]}
        <span class="legend-item">
          <span class="legend-swatch" style:background={color}></span>
          {status}
        </span>
      {/each}
    </div>
  </div>
{/if}

<style>
  .gantt-legend {
    display: flex;
    flex-wrap: wrap;
    gap: 0.75rem;
    justify-content: center;
    padding: 0.5rem 0;
    font-size: 11px;
    color: #6b7280;
  }

  .legend-item {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .legend-swatch {
    display: inline-block;
    width: 12px;
    height: 12px;
    border-radius: 2px;
  }
</style>
```

**Acceptance criteria:**
- Component shows loading state while data is being fetched.
- Component shows "No job descriptions yet" when list is empty.
- Component renders chart when JDs exist.
- Component shows error state on API failure.
- Chart height scales with item count (min 200px, max ~580px).
- HTML legend below chart shows all 9 pipeline status colors.
- Clicking a bar navigates to JD detail page.
- Chart uses `notMerge={true}` for clean option updates.
- Legend uses HTML (not ECharts legend component) because custom series does not generate automatic legend entries.

**Failure criteria:**
- `$effect` used instead of `onMount` (infinite reactive loop risk).
- Chart click uses `params.dataIndex` instead of `params.value[0]` (wrong JD).
- Legend missing statuses or using wrong colors.

---

### T67.3: Mount Component on Dashboard

**File:** `packages/webui/src/routes/+page.svelte` (modify existing)

[IMPORTANT] The `ApplicationGantt` section must be placed inside the `{:else}` block in `+page.svelte` (after the loading/error guards). Placing it outside will render the chart while data is still loading.

```svelte
import ApplicationGantt from '$lib/components/charts/ApplicationGantt.svelte'

<!-- Inside the {:else} block, after other dashboard sections -->
<section class="section">
  <h2 class="section-title">Application Timeline</h2>
  <div class="chart-card">
    <ApplicationGantt />
  </div>
</section>
```

**Acceptance criteria:**
- "Application Timeline" section appears on dashboard.
- Section is inside the `{:else}` block (not rendered during loading).
- Component handles its own empty state internally.

**Failure criteria:**
- Section rendered outside `{:else}` block (chart renders during page loading).
- Import path incorrect.

---

### T67.4: Write Gantt Utility Tests

**File:** `packages/webui/src/lib/components/charts/__tests__/gantt-utils.test.ts`

```typescript
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import {
  buildGanttItems,
  buildGanttOption,
  generateMockGanttItems,
  STATUS_COLORS,
  TERMINAL_STATUSES,
} from '../gantt-utils'

describe('buildGanttItems', () => {
  const now = new Date('2026-04-03T12:00:00Z')

  beforeEach(() => {
    vi.useFakeTimers()
    vi.setSystemTime(now)
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('sorts JDs by created_at descending (newest first)', () => {
    const jds = [
      { id: '1', title: 'Old', status: 'applied', created_at: '2026-01-01', updated_at: '2026-02-01', organization_name: null },
      { id: '2', title: 'New', status: 'applied', created_at: '2026-03-01', updated_at: '2026-03-15', organization_name: null },
    ]
    const items = buildGanttItems(jds as any)
    expect(items[0].title).toBe('New')
    expect(items[1].title).toBe('Old')
  })

  it('truncates titles longer than 30 characters', () => {
    const jds = [
      { id: '1', title: 'A'.repeat(35), status: 'applied', created_at: '2026-01-01', updated_at: '2026-02-01', organization_name: null },
    ]
    const items = buildGanttItems(jds as any)
    expect(items[0].title).toBe('A'.repeat(30) + '...')
  })

  it('sets isActive true for non-terminal statuses', () => {
    const activeStatuses = ['discovered', 'analyzing', 'applying', 'applied', 'interviewing', 'offered']
    for (const status of activeStatuses) {
      const jds = [{ id: '1', title: 'Test', status, created_at: '2026-01-01', updated_at: '2026-02-01', organization_name: null }]
      const items = buildGanttItems(jds as any)
      expect(items[0].isActive).toBe(true)
    }
  })

  it('sets isActive false for rejected, withdrawn, closed', () => {
    for (const status of ['rejected', 'withdrawn', 'closed']) {
      const jds = [{ id: '1', title: 'Test', status, created_at: '2026-01-01', updated_at: '2026-02-01', organization_name: null }]
      const items = buildGanttItems(jds as any)
      expect(items[0].isActive).toBe(false)
    }
  })

  it('uses current date as endDate for active JDs', () => {
    const jds = [
      { id: '1', title: 'Test', status: 'applied', created_at: '2026-01-01', updated_at: '2026-02-01', organization_name: null },
    ]
    const items = buildGanttItems(jds as any)
    expect(items[0].endDate.getTime()).toBe(now.getTime())
  })

  it('uses updated_at as endDate for terminal JDs', () => {
    const jds = [
      { id: '1', title: 'Test', status: 'rejected', created_at: '2026-01-01', updated_at: '2026-02-15T00:00:00Z', organization_name: null },
    ]
    const items = buildGanttItems(jds as any)
    expect(items[0].endDate.getTime()).toBe(new Date('2026-02-15T00:00:00Z').getTime())
  })

  it('maps status to correct color from STATUS_COLORS', () => {
    const jds = [
      { id: '1', title: 'Test', status: 'offered', created_at: '2026-01-01', updated_at: '2026-02-01', organization_name: null },
    ]
    const items = buildGanttItems(jds as any)
    expect(items[0].color).toBe('#22c55e')
  })

  it('handles JDs with no organization name', () => {
    const jds = [
      { id: '1', title: 'Test', status: 'applied', created_at: '2026-01-01', updated_at: '2026-02-01', organization_name: null },
    ]
    const items = buildGanttItems(jds as any)
    expect(items[0].orgName).toBeNull()
  })
})

describe('buildGanttOption', () => {
  const makeItems = (count: number): any[] =>
    Array.from({ length: count }, (_, i) => ({
      id: `${i}`,
      title: `Job ${i}`,
      orgName: `Org ${i}`,
      status: 'applied',
      startDate: new Date('2026-01-01'),
      endDate: new Date('2026-03-01'),
      isActive: true,
      color: '#818cf8',
    }))

  it('produces valid ECharts option with custom series', () => {
    const option = buildGanttOption(makeItems(3))
    expect(option.series[0].type).toBe('custom')
    expect(option.series[0].data).toHaveLength(3)
  })

  it('includes dataZoom when item count > 10', () => {
    const option = buildGanttOption(makeItems(12))
    expect(option.dataZoom).toHaveLength(1)
    expect(option.dataZoom[0].filterMode).toBe('weakFilter')
  })

  it('omits dataZoom when item count <= 10', () => {
    const option = buildGanttOption(makeItems(8))
    expect(option.dataZoom).toHaveLength(0)
  })

  it('omits dataZoom when item count is exactly 10 (boundary)', () => {
    const option = buildGanttOption(makeItems(10))
    expect(option.dataZoom).toHaveLength(0)
  })

  it('sets yAxis.inverse = true (newest at top)', () => {
    const option = buildGanttOption(makeItems(3))
    expect(option.yAxis.inverse).toBe(true)
  })

  it('computes time axis range with 5% padding', () => {
    const items = makeItems(1)
    const option = buildGanttOption(items)
    const startTime = items[0].startDate.getTime()
    const endTime = items[0].endDate.getTime()
    const range = endTime - startTime
    const expectedPadding = range * 0.05
    expect(option.xAxis.min).toBeLessThan(startTime)
    expect(option.xAxis.max).toBeGreaterThan(endTime)
  })
})

describe('generateMockGanttItems', () => {
  it('produces the requested number of items', () => {
    expect(generateMockGanttItems(5)).toHaveLength(5)
    expect(generateMockGanttItems(12)).toHaveLength(12)
  })

  it('items have valid dates and status colors', () => {
    const items = generateMockGanttItems(8)
    for (const item of items) {
      expect(item.startDate).toBeInstanceOf(Date)
      expect(item.endDate).toBeInstanceOf(Date)
      expect(item.startDate.getTime()).toBeLessThanOrEqual(item.endDate.getTime())
      expect(item.color).toBeTruthy()
      expect(STATUS_COLORS[item.status] ?? '#d1d5db').toBe(item.color)
    }
  })

  it('covers all 9 statuses (including withdrawn and closed) with 12 items', () => {
    const items = generateMockGanttItems(12)
    const statuses = new Set(items.map(i => i.status))
    const allStatuses = [
      'discovered', 'analyzing', 'applying', 'applied',
      'interviewing', 'offered', 'rejected', 'withdrawn', 'closed',
    ]
    for (const status of allStatuses) {
      expect(statuses.has(status)).toBe(true)
    }
  })
})
```

**Acceptance criteria:**
- All 17 test cases pass.
- Sort order, title truncation, active/terminal logic, color mapping all verified.
- Option structure (custom series, dataZoom, yAxis inverse) validated.
- dataZoom boundary (exactly 10 items -> no dataZoom) verified.
- Mock data generator produces valid items and covers all 9 statuses.

**Failure criteria:**
- Any test fails, indicating a bug in transformation or option building.

---

## Testing

### Unit Tests

**File:** `packages/webui/src/lib/components/charts/__tests__/gantt-utils.test.ts` (T67.4)

| Test | Assertion |
|------|-----------|
| Sort by `created_at` descending | Newest first in output |
| Title truncation at 30 chars | `'A'.repeat(30) + '...'` |
| Active status detection | `isActive = true` for 6 active statuses |
| Terminal status detection | `isActive = false` for rejected/withdrawn/closed |
| Active JD endDate = now | `endDate.getTime() === now.getTime()` |
| Terminal JD endDate = updated_at | `endDate` matches `updated_at` |
| Status-to-color mapping | `offered` -> `#22c55e` |
| Null orgName handled | `orgName` is `null` |
| Custom series in option | `series[0].type === 'custom'` |
| dataZoom for > 10 items | `dataZoom.length === 1`, `filterMode: 'weakFilter'` |
| No dataZoom for <= 10 items | `dataZoom.length === 0` |
| No dataZoom for exactly 10 items (boundary) | `dataZoom.length === 0` |
| yAxis inverse | `yAxis.inverse === true` |
| Time axis padding | `xAxis.min < startTime`, `xAxis.max > endTime` |
| Mock item count | Requested count matches output |
| Mock item validity | Dates, colors, statuses are valid |
| Mock status coverage | 12 items covers all 9 statuses (including `withdrawn`, `closed`) |

### Component Tests (Manual / Future)

| Test | What to verify |
|------|---------------|
| Loading state | "Loading application timeline..." appears |
| Empty state | "No job descriptions yet" message |
| Chart renders | EChart div with custom series for existing JDs |
| Error state | Error message on API failure |
| Dynamic height | Chart height scales with item count |
| Legend | All 9 status colors shown below chart |
| Bar click | Navigation to JD detail page |
| Active bar | Dashed right edge (two-shape group) |
| Terminal bar | Solid right edge (single rect) |
| Tooltip | Title, org, status, date range, duration in days |

### Integration Tests

| Test | What to verify |
|------|---------------|
| 3 JDs with different statuses | 3 bars with correct colors |
| Active JD bar extends to now | Bar right edge at current date |
| Terminal JD bar ends at updated_at | Bar right edge at updated_at |
| Tooltip content | All fields present and formatted |

---

## Documentation Requirements

- No new documentation files required.
- The spec file serves as the design document.
- This plan file serves as the implementation reference.
- Inline TSDoc comments on all exported interfaces and functions:
  - `GanttItem`: field semantics, active vs terminal
  - `STATUS_COLORS`: color source (Phase 61 pipeline)
  - `TERMINAL_STATUSES`: set membership
  - `buildGanttItems`: sorting, truncation, date logic
  - `buildGanttOption`: renderItem two-shape group, tooltip params.value[0], dataZoom filterMode
  - `generateMockGanttItems`: development utility, not for production

---

## Parallelization Notes

**Within this phase:**
- T67.1 (utils) is foundational -- no dependencies.
- T67.2 (component) depends on T67.1 (imports utils).
- T67.3 (dashboard mounting) depends on T67.2 (component must exist).
- T67.4 (tests) depends on T67.1 (tests the utils).

**Recommended execution order:**
1. T67.1 (utils) -- foundational
2. T67.2 (component) + T67.4 (tests) -- parallel, both depend on T67.1
3. T67.3 (dashboard mounting) -- depends on T67.2

**Cross-phase:**
- Phase 67 and 68 both modify `+page.svelte` (dashboard) -- different sections, but coordinated merge needed.
- Phase 67 does not modify `echarts-registry.ts` (CustomChart already registered by Phase 59).
