# Gantt Chart for JD Application Tracking

**Date:** 2026-04-03
**Spec:** J4 (Application Gantt Chart)
**Phase:** TBD (next available)
**Depends on:** J1 (ECharts Infrastructure — `EChart.svelte` wrapper, `echarts-registry.ts` with `CustomChart` registered), E3 (JD Kanban Pipeline — `JobDescriptionStatus` with 9 pipeline statuses)
**Blocks:** None

## Overview

Add a Gantt-style chart to the dashboard showing JD application timelines. Each row represents a job description. Bars are colored by pipeline stage, with length proportional to the time spent. This provides a visual timeline of the user's job search, making it easy to see how long applications stay in each stage and which opportunities are stale.

**MVP approach:** Since the current schema only stores the current status and `created_at`/`updated_at` timestamps on each JD (not a history of status changes), the MVP renders a single bar per JD spanning from `created_at` to `updated_at`, colored by the current status. The full timeline breakdown (bar segments per stage) requires a `jd_status_history` table, which is documented here but deferred to a future spec.

The chart uses ECharts `custom` series type (already registered by J1) with `renderItem` for full control over bar rendering.

## Non-Goals

- Full status history tracking in the MVP (deferred — data model documented in Section 6)
- Per-stage duration analytics or metrics
- Deadline tracking or SLA alerts
- Calendar integration
- Drag-and-drop status changes from the Gantt chart
- Zoom or pan on the time axis (ECharts handles scroll zoom natively if enabled, but no custom controls)
- Grouping by organization
- Mobile-specific interactions
- Chart export (PNG/SVG download)
- Accessibility (aria labels, screen reader) — deferred

---

## 1. MVP Chart Design

### 1.1 Layout

- **X axis (time):** Spans from the earliest `created_at` across all JDs to today (or the latest `updated_at`, whichever is later). Uses ECharts `time` axis type for automatic date formatting.
- **Y axis (categories):** One row per JD. Label shows the JD title (truncated to ~30 chars) and organization name (if present).
- **Bars:** Single horizontal bar per JD from `created_at` to `updated_at` (or to "now" if the JD is in an active status).
- **Color:** Each bar is colored by the JD's current pipeline status (using the same color scheme as the kanban from E3).

### 1.2 Visual Mockup

```
                    Jan          Feb          Mar          Apr
Sr Security Eng   │████████████████████████████████████████│ applied
Cloudflare        │                                        │
                  │                                        │
DevOps Lead       │██████████████████████│                  │ rejected
Acme Corp         │                     │                  │
                  │                     │                  │
Cloud Architect   │                ██████████████│          │ interviewing
BigTech Inc       │                             │          │
                  │                             │          │
Platform Eng      │                         ████│          │ discovered
Startup Co        │                             │          │
```

### 1.3 Status Colors

Reuse the accent colors from E3's pipeline column definitions:

```typescript
const STATUS_COLORS: Record<string, string> = {
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
```

### 1.4 Active vs. Terminal Statuses

- **Active statuses** (`discovered`, `analyzing`, `applying`, `applied`, `interviewing`, `offered`): The bar extends from `created_at` to **now** (current date), with a dashed right edge indicating the bar is still growing.
- **Terminal statuses** (`rejected`, `withdrawn`, `closed`): The bar extends from `created_at` to `updated_at` (the date the JD was closed), with a solid right edge.

### 1.5 Bar Click

Clicking a bar navigates to the JD detail page. Implementation: set `viewMode = 'list'` and `selectedId = jd.id` on the JD page, or navigate to `/opportunities/job-descriptions?selected=${jd.id}`.

### 1.6 Sorting

JDs are sorted by `created_at` descending (most recently created at the top). This puts the newest opportunities at the top of the chart.

### 1.7 Scrollable Y Axis

If there are more than 10 JDs, the chart uses ECharts `dataZoom` on the Y axis to create a scrollable region. The visible window shows 10 JDs at a time with a scroll slider on the right.

---

## 2. Data Loading

### 2.1 Data Source

```typescript
const jdsResult = await forge.jobDescriptions.list({ limit: 500 })
```

The JD list endpoint returns all fields including `status`, `created_at`, `updated_at`, `organization_name` (via JOIN).

### 2.2 Data Transformation

**File:** `packages/webui/src/lib/components/charts/gantt-utils.ts`

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

const TERMINAL_STATUSES = new Set(['rejected', 'withdrawn', 'closed'])

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
```

---

## 3. ECharts Configuration

### 3.1 Option Builder

```typescript
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

          // For the dashed-right-edge effect, use a two-shape group:
          // a solid rect for the bar body + a dashed line on the right edge.
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
```

### 3.2 Legend

A horizontal legend at the bottom maps status colors to status names. Since the custom series does not generate automatic legend entries, the legend is rendered as HTML below the chart (not as an ECharts legend component):

```svelte
<div class="gantt-legend">
  {#each Object.entries(STATUS_COLORS) as [status, color]}
    <span class="legend-item">
      <span class="legend-swatch" style:background={color}></span>
      {status}
    </span>
  {/each}
</div>
```

---

## 4. Component

### 4.1 File: `packages/webui/src/lib/components/charts/ApplicationGantt.svelte`

```svelte
<script lang="ts">
  import { onMount } from 'svelte'
  import { forge } from '$lib/sdk'
  import { goto } from '$app/navigation'
  import EChart from './EChart.svelte'
  import { buildGanttItems, buildGanttOption, STATUS_COLORS } from './gantt-utils'
  import type { JobDescriptionWithOrg } from '@forge/sdk'

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
```

### 4.2 Styling

```css
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
```

### 4.3 Dynamic Height

The chart height scales with the number of visible items: `max(200px, min(items, 10) * 50px + 80px)`. For 5 JDs, the chart is 330px. For 10+, it caps at 580px with Y-axis scrolling via `dataZoom`.

---

## 5. Dashboard Integration

### 5.1 Placement

The Gantt chart is added to the dashboard page (`/`) as a card in the dashboard section, alongside other visualization cards.

**Important:** This section must be placed inside the `{:else}` block in `+page.svelte` (after the loading/error guards). Placing it outside will render the chart while data is still loading.

```svelte
<section class="section">
  <h2 class="section-title">Application Timeline</h2>
  <div class="chart-card">
    <ApplicationGantt />
  </div>
</section>
```

### 5.2 Import

```svelte
import ApplicationGantt from '$lib/components/charts/ApplicationGantt.svelte'
```

### 5.3 Empty State

The component handles its own empty state internally. The dashboard always renders the section; the component shows "No job descriptions yet" when the list is empty.

---

## 6. Future: Status History Table (Deferred)

This section documents the data model for tracking status change history. This is NOT implemented in this spec — only the schema design is documented here for reference.

### 6.1 Purpose

To render per-stage bar segments on the Gantt chart (e.g., "3 days in Discovered, 5 days in Analyzing, 12 days in Applying"), the system needs to record when each status change occurred.

### 6.2 Proposed Schema

```sql
-- DEFERRED — DO NOT IMPLEMENT
CREATE TABLE jd_status_history (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  job_description_id TEXT NOT NULL REFERENCES job_descriptions(id) ON DELETE CASCADE,
  status TEXT NOT NULL CHECK (status IN (
    'discovered', 'analyzing', 'applying', 'applied',
    'interviewing', 'offered', 'rejected', 'withdrawn', 'closed'
  )),
  entered_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  exited_at TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

CREATE INDEX idx_jd_status_history_jd ON jd_status_history(job_description_id);
CREATE INDEX idx_jd_status_history_entered ON jd_status_history(job_description_id, entered_at);
```

### 6.3 Integration with Gantt

When status history is available, each row in the Gantt chart would render multiple bar segments:

```
Sr Security Eng   │▓▓▓▓│████│▒▒▒▒▒▒▒▒│░░░░░░│
                   disc  anlz  applying  applied
```

Each segment is colored by status and sized by duration (`exited_at - entered_at`). The `renderItem` function would iterate over status history entries instead of rendering a single bar.

### 6.4 History Recording

Status changes would be recorded by a trigger or by the PATCH handler. When `PATCH /api/job-descriptions/:id { status }` is called:
1. Close the current status history entry: `UPDATE jd_status_history SET exited_at = now WHERE job_description_id = ? AND exited_at IS NULL`
2. Insert a new entry: `INSERT INTO jd_status_history (id, job_description_id, status) VALUES (?, ?, ?)`

---

## 7. Mock Data for Development

During development, use mock data to test the chart before the JD pipeline has real data:

```typescript
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
    const isActive = !['rejected', 'withdrawn', 'closed'].includes(status)
    const endDate = isActive
      ? now
      : new Date(startDate.getTime() + (30 + Math.random() * 30) * 86400000)

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

---

## 8. Files to Create

| File | Purpose |
|------|---------|
| `packages/webui/src/lib/components/charts/ApplicationGantt.svelte` | Gantt chart component with loading, error, empty, and chart states |
| `packages/webui/src/lib/components/charts/gantt-utils.ts` | `buildGanttItems`, `buildGanttOption`, `STATUS_COLORS`, `generateMockGanttItems` |

## 9. Files to Modify

| File | Change |
|------|--------|
| `packages/webui/src/routes/+page.svelte` | Import `ApplicationGantt` and add "Application Timeline" section |

No changes to `echarts-registry.ts` — `CustomChart` is already registered by J1.

---

## 10. Testing

### 10.1 Utility Unit Tests

**File:** `packages/webui/src/lib/components/charts/__tests__/gantt-utils.test.ts`

- `buildGanttItems` sorts JDs by `created_at` descending (newest first)
- `buildGanttItems` truncates titles longer than 30 characters
- `buildGanttItems` sets `isActive = true` for non-terminal statuses
- `buildGanttItems` sets `isActive = false` for `rejected`, `withdrawn`, `closed`
- `buildGanttItems` uses current date as `endDate` for active JDs
- `buildGanttItems` uses `updated_at` as `endDate` for terminal JDs
- `buildGanttItems` maps status to correct color from `STATUS_COLORS`
- `buildGanttItems` handles JDs with no organization name
- `buildGanttOption` produces valid ECharts option with custom series
- `buildGanttOption` includes `dataZoom` when item count > 10
- `buildGanttOption` omits `dataZoom` when item count <= 10
- `buildGanttOption` sets `yAxis.inverse = true` (newest at top)
- `buildGanttOption` computes time axis range with 5% padding
- `generateMockGanttItems` produces the requested number of items
- `generateMockGanttItems` items have valid dates and status colors

### 10.2 Component Tests

- `ApplicationGantt` shows loading state while data is being fetched
- `ApplicationGantt` shows "No job descriptions yet" when list is empty
- `ApplicationGantt` renders chart when JDs exist
- `ApplicationGantt` shows error state on API failure
- Chart height scales with item count (min 200px)
- Legend shows all 9 pipeline status colors
- Clicking a bar triggers navigation to JD detail page
- Chart uses `notMerge={true}` for clean option updates

### 10.3 Integration Tests

- Create 3 JDs with different statuses and dates; verify Gantt chart renders 3 bars with correct colors
- Verify active JD bars extend to current date
- Verify terminal JD bars end at `updated_at`
- Verify tooltip shows JD title, org name, status, date range, and duration in days

---

## 11. Acceptance Criteria

### MVP
1. `ApplicationGantt` component renders on the dashboard in an "Application Timeline" section
2. X axis shows time, Y axis shows JD titles with organization names
3. Each JD is rendered as a single horizontal bar from `created_at` to `updated_at` (or now for active JDs)
4. Bars are colored by current pipeline status using E3's color scheme
5. Active JD bars have a dashed right edge indicating they are still in progress
6. Terminal JD bars (rejected, withdrawn, closed) have a solid right edge
7. JDs are sorted by `created_at` descending (newest at top)
8. Tooltip shows JD title, organization, status, date range, and duration in days
9. Clicking a bar navigates to the JD detail page
10. Y axis scrolls via `dataZoom` when there are more than 10 JDs
11. Chart height dynamically scales with item count (200px to 580px)
12. HTML legend below the chart shows all 9 status colors
13. Component handles empty state, loading state, and error state gracefully
14. Mock data generator is available for development/testing
15. All utility unit tests pass (15 cases)

### Deferred (Future Spec)
16. `jd_status_history` table tracks per-stage durations
17. Gantt bars are segmented by status with per-stage coloring
18. Duration-per-stage analytics
