# JD Compensation Bullet Graph

**Date:** 2026-04-03
**Spec:** E5 (JD Compensation Bullet Graph)
**Phase:** TBD (next available)
**Depends on:** E1 (JD Detail Page — JD entity with CRUD UI), J1 (ECharts Infrastructure — `EChart.svelte` wrapper, `echarts-registry.ts`, `echarts-theme.ts`)
**Blocks:** None

## Overview

Add a bullet graph to the JD detail page that visualizes where a JD's compensation falls relative to the user's salary expectations. The chart uses horizontal bands for the user's salary ranges (minimum, target, stretch) and overlays the JD's salary range as a bar, making it immediately clear whether a JD's compensation meets, exceeds, or falls short of expectations.

The existing `job_descriptions` table has a `salary_range TEXT` column (free-text, e.g., "$150k-$200k", "DOE", "GS-13"). This spec adds structured salary fields (`salary_min`, `salary_max` as integers) to the `job_descriptions` table so the chart has reliable numeric data to work with. The free-text `salary_range` field is preserved for display purposes.

The user's salary expectations are stored as new fields on the `user_profile` table (from the Config Profile spec): `salary_minimum`, `salary_target`, `salary_stretch`.

## Non-Goals

- Market data comparison (how the JD compares to other applicants' expectations or industry benchmarks — requires external data we don't have)
- Total compensation modeling (stock, bonus, benefits — only base salary)
- Multi-JD comparison view on a single chart (deferred; the chart shows one JD at a time)
- Currency conversion (all values assumed USD)
- Automatic salary extraction from JD text (user enters structured salary manually)
- Salary negotiation advice or recommendations
- Historical salary tracking or trending
- Cost-of-living adjustments by location
- Mobile-specific chart interactions

---

## 1. Data Model Changes

### 1.1 Migration: `021_salary_structured_fields.sql`

Add structured salary columns to `job_descriptions` and salary expectation columns to `user_profile`.

```sql
-- Structured Salary Fields
-- Migration: 021_salary_structured_fields
-- Adds salary_min/salary_max INTEGER columns to job_descriptions.
-- Adds salary_minimum/salary_target/salary_stretch to user_profile.
-- salary_range TEXT is preserved for free-text display.

-- Step 1: Add structured salary columns to job_descriptions
ALTER TABLE job_descriptions ADD COLUMN salary_min INTEGER;
ALTER TABLE job_descriptions ADD COLUMN salary_max INTEGER;

-- Step 2: Add salary expectation columns to user_profile
ALTER TABLE user_profile ADD COLUMN salary_minimum INTEGER;
ALTER TABLE user_profile ADD COLUMN salary_target INTEGER;
ALTER TABLE user_profile ADD COLUMN salary_stretch INTEGER;

-- Step 3: Register migration
INSERT INTO _migrations (name) VALUES ('021_salary_structured_fields');
```

**Design notes:**
- `salary_min` and `salary_max` are nullable INTEGER columns (in dollars, not thousands). A JD might not have salary information.
- `salary_range` (TEXT) remains for display — it may contain non-numeric information ("DOE", "competitive") that the structured fields cannot capture.
- `salary_minimum`, `salary_target`, `salary_stretch` on `user_profile` represent the user's personal salary bands. All nullable — if unset, the chart shows JD salary without background bands.
- No CHECK constraint on salary ordering is added at the database level. The API validates `salary_min <= salary_max` and `salary_minimum <= salary_target <= salary_stretch` on write.

### 1.2 Type Changes

**`packages/core/src/types/index.ts`:**

```typescript
// Add to JobDescription interface:
salary_min?: number | null
salary_max?: number | null

// Add to UserProfile interface:
salary_minimum?: number | null
salary_target?: number | null
salary_stretch?: number | null
```

**`packages/sdk/src/types.ts`:**

Mirror the same additions.

### 1.3 API Changes

**JD endpoints (existing PATCH):**
- Accept `salary_min` and `salary_max` as optional integer fields in `PATCH /api/job-descriptions/:id`
- Validate: if both are provided, `salary_min <= salary_max`. If only one is provided, accept it.
- Accept in `POST /api/job-descriptions` as well.

**Profile endpoint (existing PATCH):**
- Accept `salary_minimum`, `salary_target`, `salary_stretch` as optional integer fields in `PATCH /api/profile`
- Validate ordering: `salary_minimum <= salary_target <= salary_stretch` (when all three are provided). Partial updates are allowed.

### 1.4 SDK Changes

Add to `JobDescriptionsResource`:
- Include `salary_min` and `salary_max` in create/update payloads

Add to `ProfileResource` (or wherever profile is managed):
- Include `salary_minimum`, `salary_target`, `salary_stretch` in update payload

---

## 2. Chart Design

### 2.1 Bullet Graph Structure

A bullet graph is a variation of a bar chart designed by Stephen Few for comparing a single value against a contextual range. The chart has:

- **Background bands (qualitative ranges):** Three horizontal bands representing the user's salary expectations:
  - Band 1 (lightest): `$0` to `salary_minimum` — below minimum (red-tinted, `rgba(239, 68, 68, 0.08)`)
  - Band 2 (medium): `salary_minimum` to `salary_target` — acceptable range (amber-tinted, `rgba(245, 158, 11, 0.12)`)
  - Band 3 (strongest): `salary_target` to `salary_stretch` — target range (green-tinted, `rgba(34, 197, 94, 0.15)`)
  - Band 4 (lightest): beyond `salary_stretch` — above stretch (green-tinted, `rgba(34, 197, 94, 0.08)`)
- **Feature bar:** Horizontal bar from `salary_min` to `salary_max` of the JD (dark, filled)
- **Target marker:** Vertical line at the midpoint of the JD's salary range

### 2.2 Visual Mockup

```
                        Your Expectations
     Below Min        Acceptable        Target         Stretch+
  |░░░░░░░░░░░░░|▒▒▒▒▒▒▒▒▒▒▒▒▒|████████████████|░░░░░░░░░░|
  |              |               |      ┌────────┐ |          |
  |              |               |      │ JD Pay │ |          |
  |              |               |      └────┼───┘ |          |
  |              |               |           ▲     |          |
  $0           $120k           $160k   $170k-$210k          $250k
                                       midpoint: $190k
```

### 2.3 Label Annotations

The chart includes reference lines with labels:
- `salary_minimum`: labeled "Min" with a vertical dashed line
- `salary_target`: labeled "Target" with a vertical dashed line
- `salary_stretch`: labeled "Stretch" with a vertical dashed line
- JD salary midpoint: labeled with the dollar value

### 2.4 Color Coding the JD Bar

The JD bar's color depends on where its midpoint falls relative to user expectations:
- Midpoint below `salary_minimum`: Red (`#ef4444`) — below minimum
- Midpoint between `salary_minimum` and `salary_target`: Amber (`#f59e0b`) — acceptable
- Midpoint between `salary_target` and `salary_stretch`: Green (`#22c55e`) — target
- Midpoint above `salary_stretch`: Blue (`#6c63ff`) — above stretch

### 2.5 Missing Data States

- **JD has no salary data:** Show a muted message "No salary data available for this JD" instead of the chart
- **User has no salary expectations set:** Show the JD salary bar without background bands, with a link to `/config/profile` to set expectations
- **JD has only `salary_min` or only `salary_max`:** Use the single value as both min and max (point marker instead of bar)

---

## 3. ECharts Configuration

### 3.1 Option Builder

**File:** `packages/webui/src/lib/components/charts/compensation-utils.ts`

```typescript
export interface SalaryExpectations {
  minimum: number | null
  target: number | null
  stretch: number | null
}

export interface JDSalary {
  min: number | null
  max: number | null
}

export interface CompensationData {
  jdSalary: JDSalary
  expectations: SalaryExpectations
  jdTitle: string
}

/**
 * Determine chart X-axis range from available data.
 * Includes padding (10%) on both sides.
 */
function computeAxisRange(data: CompensationData): [number, number] {
  const values: number[] = []

  if (data.jdSalary.min != null) values.push(data.jdSalary.min)
  if (data.jdSalary.max != null) values.push(data.jdSalary.max)
  if (data.expectations.minimum != null) values.push(data.expectations.minimum)
  if (data.expectations.target != null) values.push(data.expectations.target)
  if (data.expectations.stretch != null) values.push(data.expectations.stretch)

  if (values.length === 0) return [0, 200000]

  const min = Math.min(...values)
  const max = Math.max(...values)
  const padding = (max - min) * 0.1 || 20000

  return [Math.max(0, min - padding), max + padding]
}

/**
 * Format a dollar amount for display.
 */
function formatSalary(value: number): string {
  if (value >= 1000) {
    return `$${Math.round(value / 1000)}k`
  }
  return `$${value.toLocaleString()}`
}

/**
 * Determine JD bar color based on midpoint vs expectations.
 */
function getJDBarColor(midpoint: number, expectations: SalaryExpectations): string {
  if (expectations.minimum != null && midpoint < expectations.minimum) return '#ef4444'
  if (expectations.target != null && midpoint < expectations.target) return '#f59e0b'
  if (expectations.stretch != null && midpoint <= expectations.stretch) return '#22c55e'
  if (expectations.stretch != null && midpoint > expectations.stretch) return '#6c63ff'
  return '#374151'  // neutral when no expectations set
}

export function buildCompensationOption(data: CompensationData): EChartsOption {
  const [axisMin, axisMax] = computeAxisRange(data)
  const { expectations, jdSalary } = data

  const jdMin = jdSalary.min ?? jdSalary.max ?? 0
  const jdMax = jdSalary.max ?? jdSalary.min ?? 0
  const jdMidpoint = (jdMin + jdMax) / 2
  const barColor = getJDBarColor(jdMidpoint, expectations)

  // Build markArea bands for expectations
  const markAreaData: any[] = []
  const markLineData: any[] = []

  if (expectations.minimum != null) {
    markAreaData.push([
      { xAxis: axisMin, itemStyle: { color: 'rgba(239, 68, 68, 0.06)' } },
      { xAxis: expectations.minimum },
    ])
    markLineData.push({
      xAxis: expectations.minimum,
      label: { formatter: `Min\n${formatSalary(expectations.minimum)}`, position: 'start' },
      lineStyle: { type: 'dashed', color: '#ef4444', width: 1 },
    })
  }

  if (expectations.minimum != null && expectations.target != null) {
    markAreaData.push([
      { xAxis: expectations.minimum, itemStyle: { color: 'rgba(245, 158, 11, 0.08)' } },
      { xAxis: expectations.target },
    ])
  }

  if (expectations.target != null) {
    markLineData.push({
      xAxis: expectations.target,
      label: { formatter: `Target\n${formatSalary(expectations.target)}`, position: 'start' },
      lineStyle: { type: 'dashed', color: '#f59e0b', width: 1 },
    })
  }

  if (expectations.target != null && expectations.stretch != null) {
    markAreaData.push([
      { xAxis: expectations.target, itemStyle: { color: 'rgba(34, 197, 94, 0.10)' } },
      { xAxis: expectations.stretch },
    ])
  }

  if (expectations.stretch != null) {
    markLineData.push({
      xAxis: expectations.stretch,
      label: { formatter: `Stretch\n${formatSalary(expectations.stretch)}`, position: 'start' },
      lineStyle: { type: 'dashed', color: '#22c55e', width: 1 },
    })
  }

  return {
    tooltip: {
      trigger: 'item',
      formatter: () => {
        const parts = [`<strong>${data.jdTitle}</strong>`]
        if (jdMin !== jdMax) {
          parts.push(`Salary: ${formatSalary(jdMin)} - ${formatSalary(jdMax)}`)
          parts.push(`Midpoint: ${formatSalary(jdMidpoint)}`)
        } else {
          parts.push(`Salary: ${formatSalary(jdMin)}`)
        }
        return parts.join('<br/>')
      },
    },
    grid: {
      left: '5%',
      right: '5%',
      top: '20%',
      bottom: '25%',
      containLabel: true,
    },
    xAxis: {
      type: 'value',
      min: axisMin,
      max: axisMax,
      axisLabel: {
        formatter: (val: number) => formatSalary(val),
      },
    },
    yAxis: {
      type: 'category',
      data: [data.jdTitle],
      axisLabel: { show: false },
      axisTick: { show: false },
    },
    // NOTE: ECharts standard `bar` series does not support range bars via `encode`.
    // Use a stacked bar trick: an invisible bar from 0 to jdMin, then a visible bar
    // from jdMin to jdMax. Alternatively, use a custom series (`renderItem`) to draw
    // a horizontal rect from jdMin to jdMax.
    series: [
      {
        // Invisible spacer bar: 0 to jdMin
        type: 'bar',
        stack: 'salary',
        data: [jdMin],
        barWidth: 24,
        itemStyle: { color: 'transparent' },
        emphasis: { disabled: true },
        tooltip: { show: false },
      },
      {
        // Visible range bar: jdMin to jdMax
        type: 'bar',
        stack: 'salary',
        data: [jdMax - jdMin],
        barWidth: 24,
        itemStyle: { color: barColor, borderRadius: 3 },
        markArea: {
          silent: true,
          data: markAreaData,
        },
        markLine: {
          silent: true,
          symbol: 'none',
          data: markLineData,
        },
      },
    ],
    graphic: [
      {
        type: 'text',
        left: 'center',
        top: 5,
        style: {
          text: 'Compensation',
          fontSize: 14,
          fontWeight: 600,
          fill: '#374151',
        },
      },
    ],
  }
}
```

### 3.2 ECharts Registry Update

The bullet graph uses `BarChart` (already needed for the E4 fallback) plus `MarkAreaComponent` and `MarkLineComponent`. Add to `echarts-registry.ts`:

```typescript
import { MarkAreaComponent } from 'echarts/components'
import { MarkLineComponent } from 'echarts/components'

// Add to echarts.use([...]):
MarkAreaComponent,
MarkLineComponent,
```

If `BarChart` was already registered by E4, no additional chart type import is needed.

---

## 4. Component

### 4.1 File: `packages/webui/src/lib/components/charts/CompensationBulletGraph.svelte`

```svelte
<script lang="ts">
  import { forge } from '$lib/sdk'
  import EChart from './EChart.svelte'
  import { buildCompensationOption } from './compensation-utils'
  import type { CompensationData, SalaryExpectations, JDSalary } from './compensation-utils'

  let {
    jdTitle,
    salaryMin,
    salaryMax,
  }: {
    jdTitle: string
    salaryMin: number | null
    salaryMax: number | null
  } = $props()

  let expectations = $state<SalaryExpectations>({
    minimum: null,
    target: null,
    stretch: null,
  })
  let loadingProfile = $state(true)

  // Load user salary expectations from profile on mount (not $effect) to avoid
  // unnecessary re-fetches.
  import { onMount } from 'svelte'
  onMount(() => { loadExpectations() })

  async function loadExpectations() {
    loadingProfile = true
    try {
      const result = await forge.profile.get()
      if (result.ok) {
        expectations = {
          minimum: result.data.salary_minimum ?? null,
          target: result.data.salary_target ?? null,
          stretch: result.data.salary_stretch ?? null,
        }
      }
    } finally {
      loadingProfile = false
    }
  }

  // Treat salary_min = 0 and salary_max = 0 as 'not set' — same as null.
  let hasJDSalary = $derived(
    (salaryMin != null && salaryMin > 0) || (salaryMax != null && salaryMax > 0)
  )
  let hasExpectations = $derived(
    expectations.minimum != null ||
    expectations.target != null ||
    expectations.stretch != null
  )

  let chartOption = $derived(
    hasJDSalary
      ? buildCompensationOption({
          jdSalary: { min: salaryMin, max: salaryMax },
          expectations,
          jdTitle,
        })
      : null
  )
</script>

{#if loadingProfile}
  <div class="comp-loading">Loading compensation data...</div>
{:else if !hasJDSalary}
  <div class="comp-empty">
    No salary data entered for this JD. Add salary values to see compensation analysis.
  </div>
{:else}
  <div class="comp-container">
    <EChart
      option={chartOption!}
      height="180px"
      notMerge={true}
    />
    {#if !hasExpectations}
      <div class="comp-hint">
        Set your salary expectations in
        <a href="/config/profile">Profile Settings</a>
        to see how this JD compares.
      </div>
    {/if}
  </div>
{/if}
```

### 4.2 Props

| Prop | Type | Required | Description |
|------|------|----------|-------------|
| `jdTitle` | `string` | Yes | The JD title, used as the Y-axis label and in the tooltip |
| `salaryMin` | `number \| null` | Yes | Structured salary minimum from JD (may be null) |
| `salaryMax` | `number \| null` | Yes | Structured salary maximum from JD (may be null) |

### 4.3 Chart Height

The bullet graph is compact — `180px` height for a single JD. This fits well in the editor panel without dominating the layout.

---

## 5. Placement

### 5.1 JD Editor Integration

The `CompensationBulletGraph` component is mounted on the JD editor panel (`JDEditor.svelte`), in a "Compensation" section between the salary input fields and the skill tags section:

```
Location: [____________________]
Salary Range: [$150k-$200k______]
Salary Min ($): [150000___]  Salary Max ($): [200000___]

┌─ Compensation ──────────────────────────────────┐
│ [Bullet graph visualization]                     │
│ Set your salary expectations in Profile Settings │
└──────────────────────────────────────────────────┘

Required Skills:
[Python x] [AWS x]
```

### 5.2 Salary Input Fields

The JD editor needs two new numeric input fields for `salary_min` and `salary_max`. These are placed on the same row as (or just below) the free-text `salary_range` field:

```svelte
<div class="salary-fields">
  <label>Salary Range (text)
    <input type="text" bind:value={form.salary_range} placeholder="e.g., $150k-$200k" />
  </label>
  <div class="salary-structured">
    <label>Min ($)
      <input type="number" bind:value={form.salary_min} placeholder="150000" step="1000" />
    </label>
    <label>Max ($)
      <input type="number" bind:value={form.salary_max} placeholder="200000" step="1000" />
    </label>
  </div>
</div>
```

### 5.3 Profile Settings Integration

The profile editor at `/config/profile` needs three new numeric input fields:

```svelte
<h3>Salary Expectations</h3>
<div class="salary-expectations">
  <label>Minimum Acceptable ($)
    <input type="number" bind:value={form.salary_minimum} placeholder="120000" step="1000" />
  </label>
  <label>Target ($)
    <input type="number" bind:value={form.salary_target} placeholder="160000" step="1000" />
  </label>
  <label>Stretch ($)
    <input type="number" bind:value={form.salary_stretch} placeholder="200000" step="1000" />
  </label>
</div>
```

---

## 6. Files to Create

| File | Purpose |
|------|---------|
| `packages/core/src/db/migrations/021_salary_structured_fields.sql` | Add `salary_min`/`salary_max` to JD, `salary_minimum`/`salary_target`/`salary_stretch` to profile |
| `packages/webui/src/lib/components/charts/CompensationBulletGraph.svelte` | Bullet graph component |
| `packages/webui/src/lib/components/charts/compensation-utils.ts` | `buildCompensationOption`, `computeAxisRange`, `formatSalary`, `getJDBarColor` |

## 7. Files to Modify

| File | Change |
|------|--------|
| `packages/core/src/types/index.ts` | Add `salary_min`, `salary_max` to `JobDescription`; add `salary_minimum`, `salary_target`, `salary_stretch` to `UserProfile` |
| `packages/sdk/src/types.ts` | Mirror type changes |
| `packages/core/src/routes/job-descriptions.ts` | Accept `salary_min`/`salary_max` in POST/PATCH; validate ordering |
| `packages/core/src/routes/profile.ts` | Accept salary expectation fields in PATCH; validate ordering |
| `packages/core/src/db/repositories/job-description-repository.ts` | Include `salary_min`/`salary_max` in INSERT/UPDATE/SELECT queries |
| `packages/core/src/db/repositories/profile-repository.ts` (or equivalent) | Include salary expectation fields in UPDATE/SELECT queries |
| `packages/sdk/src/resources/job-descriptions.ts` | Include `salary_min`/`salary_max` in create/update payloads |
| `packages/sdk/src/resources/profile.ts` (or equivalent) | Include salary expectation fields in update payload |
| `packages/webui/src/lib/components/jd/JDEditor.svelte` | Add salary_min/salary_max input fields; mount `CompensationBulletGraph` |
| `packages/webui/src/routes/config/profile/+page.svelte` | Add salary expectation input fields |
| `packages/webui/src/lib/components/charts/echarts-registry.ts` | Register `MarkAreaComponent`, `MarkLineComponent` (and `BarChart` if not already registered by E4) |

---

## 8. Testing

### 8.1 Migration Tests

- Migration 021 adds `salary_min` and `salary_max` columns to `job_descriptions`
- Migration 021 adds `salary_minimum`, `salary_target`, `salary_stretch` columns to `user_profile`
- All new columns are nullable (existing rows get NULL values)
- Inserting a JD with `salary_min = 150000, salary_max = 200000` succeeds
- Inserting profile with `salary_minimum = 120000, salary_target = 160000, salary_stretch = 200000` succeeds

### 8.2 API Tests

- `PATCH /api/job-descriptions/:id { salary_min: 150000, salary_max: 200000 }` succeeds
- `PATCH /api/job-descriptions/:id { salary_min: 200000, salary_max: 150000 }` returns 400 (min > max)
- `PATCH /api/job-descriptions/:id { salary_min: 150000 }` succeeds (partial update)
- `GET /api/job-descriptions/:id` includes `salary_min` and `salary_max` in response
- `PATCH /api/profile { salary_minimum: 120000, salary_target: 160000, salary_stretch: 200000 }` succeeds
- `PATCH /api/profile { salary_minimum: 200000, salary_target: 100000 }` returns 400 (minimum > target)
- `GET /api/profile` includes salary expectation fields

### 8.3 Utility Unit Tests

**File:** `packages/webui/src/lib/components/charts/__tests__/compensation-utils.test.ts`

- `computeAxisRange` includes padding around min/max values
- `computeAxisRange` returns default range when no values are provided
- `formatSalary` formats 150000 as "$150k"
- `formatSalary` formats 75000 as "$75k"
- `getJDBarColor` returns red when midpoint is below minimum
- `getJDBarColor` returns amber when midpoint is between minimum and target
- `getJDBarColor` returns green when midpoint is between target and stretch
- `getJDBarColor` returns blue when midpoint is above stretch
- `getJDBarColor` returns neutral gray when no expectations are set
- `buildCompensationOption` produces valid ECharts option with bar series
- `buildCompensationOption` includes markArea bands when expectations are set
- `buildCompensationOption` omits markArea bands when expectations are null
- `buildCompensationOption` includes markLine reference lines for min/target/stretch
- `buildCompensationOption` handles single salary value (min only, max only)

### 8.4 Component Tests

- `CompensationBulletGraph` shows loading state while profile is being fetched
- `CompensationBulletGraph` shows "No salary data" message when both `salaryMin` and `salaryMax` are null
- `CompensationBulletGraph` renders chart when JD has salary data
- `CompensationBulletGraph` shows profile settings hint when user has no salary expectations
- `CompensationBulletGraph` hides hint when user has salary expectations set
- Chart bar is colored based on midpoint vs. expectations (red/amber/green/blue)
- Chart updates reactively when salary props change

### 8.5 UI Integration Tests

- JD editor shows salary_min and salary_max input fields
- Saving a JD with structured salary values persists them
- Profile editor shows salary expectation input fields
- Saving profile with salary expectations persists them
- Bullet graph renders on JD detail page when salary data exists
- Bullet graph bands reflect profile salary expectations

---

## 9. Acceptance Criteria

1. Migration 021 adds `salary_min`/`salary_max` (INTEGER, nullable) to `job_descriptions`
2. Migration 021 adds `salary_minimum`/`salary_target`/`salary_stretch` (INTEGER, nullable) to `user_profile`
3. JD API accepts and validates structured salary fields (min <= max)
4. Profile API accepts and validates salary expectation fields (minimum <= target <= stretch)
5. JD editor includes numeric input fields for `salary_min` and `salary_max`
6. Profile editor includes numeric input fields for salary expectations
7. `CompensationBulletGraph` renders a horizontal bullet graph on the JD detail page
8. Background bands show user's salary ranges (below-min = red, acceptable = amber, target = green)
9. JD salary bar is overlaid on the bands, colored by where midpoint falls relative to expectations
10. Reference lines with labels mark minimum, target, and stretch thresholds
11. Tooltip shows JD title, salary range, and midpoint
12. Chart shows "No salary data" when JD has no structured salary values
13. Chart shows "Set expectations in Profile Settings" link when user profile has no salary expectations
14. Chart renders JD salary bar without bands when expectations are unset (neutral presentation)
15. JD with only `salary_min` or only `salary_max` renders as a point marker
16. `MarkAreaComponent` and `MarkLineComponent` are registered in `echarts-registry.ts`
17. All utility unit tests pass (14 cases)
18. API validation rejects `salary_min > salary_max` and `salary_minimum > salary_target`
