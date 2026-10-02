# Phase 66: JD Compensation Bullet Graph (Spec E5)

**Status:** Planning
**Date:** 2026-04-03
**Spec:** [2026-04-03-jd-compensation-chart.md](../refs/specs/2026-04-03-jd-compensation-chart.md)
**Depends on:** Phase 49 (JD Detail — JD entity with CRUD UI), Phase 59 (ECharts Infrastructure — `EChart.svelte` wrapper, `echarts-registry.ts`, `echarts-theme.ts`)
**Blocks:** None
**Parallelizable with:** Phase 65, Phase 67, Phase 68 -- touches different files except shared `echarts-registry.ts` and `JDEditor.svelte`

## Goal

Add a bullet graph to the JD detail page that compares a JD's salary range against the user's personal salary expectations (minimum, target, stretch). The chart uses ECharts stacked bar trick (invisible spacer + visible range bar) for the range display, overlaid on qualitative background bands (markArea) and reference lines (markLine) representing the user's salary thresholds. This phase also adds the data model changes: structured salary columns on `job_descriptions` (`salary_min`, `salary_max`) and salary expectation columns on `user_profile` (`salary_minimum`, `salary_target`, `salary_stretch`), with corresponding API, SDK, and UI updates.

## Non-Goals

- Market data comparison or industry benchmarks (requires external data)
- Total compensation modeling (stock, bonus, benefits -- only base salary)
- Multi-JD comparison on a single chart (one JD at a time)
- Currency conversion (all values assumed USD)
- Automatic salary extraction from JD text
- Salary negotiation advice or recommendations
- Historical salary tracking, cost-of-living adjustments
- Mobile-specific chart interactions, chart export, accessibility

## Context

The `job_descriptions` table has a `salary_range TEXT` column for free-text salary info ("$150k-$200k", "DOE", "GS-13"). This is useful for display but unusable for charts. This phase adds `salary_min` and `salary_max` as nullable INTEGER columns (in dollars, not thousands) for structured numeric data. The free-text `salary_range` column is preserved.

The user's salary expectations are stored on `user_profile` as three tiers: `salary_minimum` (floor), `salary_target` (ideal), `salary_stretch` (aspirational). These drive the qualitative background bands on the chart.

ECharts standard `bar` series does not support range bars via `encode`. The spec uses a stacked bar trick: an invisible bar from 0 to `salary_min`, then a visible bar of width `salary_max - salary_min`. This is documented as a key pattern fix.

## Scope

| Spec section | Covered here? |
|-------------|---------------|
| 1. Data Model Changes (migration, types, API, SDK) | Yes |
| 2. Chart Design (bullet graph structure, color coding, missing data states) | Yes |
| 3. ECharts Configuration (option builder, registry) | Yes |
| 4. Component (`CompensationBulletGraph.svelte`, props) | Yes |
| 5. Placement (JD editor, profile settings) | Yes |
| 6-7. Files to create/modify | Yes |
| 8. Testing | Yes |
| 9. Acceptance criteria | Yes |

## Files to Create

| File | Description |
|------|-------------|
| `packages/core/src/db/migrations/021_salary_structured_fields.sql` | Add `salary_min`/`salary_max` to `job_descriptions`; add `salary_minimum`/`salary_target`/`salary_stretch` to `user_profile` |
| `packages/webui/src/lib/components/charts/compensation-utils.ts` | `buildCompensationOption`, `computeAxisRange`, `formatSalary`, `getJDBarColor` |
| `packages/webui/src/lib/components/charts/CompensationBulletGraph.svelte` | Bullet graph component with loading, empty, and chart states |
| `packages/webui/src/lib/components/charts/__tests__/compensation-utils.test.ts` | Unit tests for utility functions (15 cases) |

## Files to Modify

| File | Change |
|------|--------|
| `packages/core/src/types/index.ts` | Add `salary_min`, `salary_max` to `JobDescription`; add `salary_minimum`, `salary_target`, `salary_stretch` to `UserProfile` |
| `packages/sdk/src/types.ts` | Mirror type changes |
| `packages/core/src/routes/job-descriptions.ts` | Accept `salary_min`/`salary_max` in POST/PATCH; validate `salary_min <= salary_max` |
| `packages/core/src/routes/profile.ts` | Accept salary expectation fields in PATCH; validate `salary_minimum <= salary_target <= salary_stretch` |
| `packages/core/src/db/repositories/job-description-repository.ts` | Include `salary_min`/`salary_max` in INSERT/UPDATE/SELECT queries |
| `packages/core/src/db/repositories/profile-repository.ts` | Include salary expectation fields in UPDATE/SELECT queries |
| `packages/sdk/src/resources/job-descriptions.ts` | Include `salary_min`/`salary_max` in create/update payloads |
| `packages/sdk/src/resources/profile.ts` | Include salary expectation fields in update payload |
| `packages/webui/src/lib/components/jd/JDEditor.svelte` | Add `salary_min`/`salary_max` numeric input fields; mount `CompensationBulletGraph` |
| `packages/webui/src/routes/config/profile/+page.svelte` | Add salary expectation input fields |
| `packages/webui/src/lib/components/charts/echarts-registry.ts` | Register `MarkAreaComponent`, `MarkLineComponent` (and `BarChart` if not already registered by Phase 65) |

## Fallback Strategies

- **JD has no salary data (`salary_min` and `salary_max` both null/0):** Component shows a clear empty state ("No salary data entered for this JD. Add salary values to see compensation analysis."). No chart rendered.
- **User has no salary expectations set:** The JD salary bar renders without background bands. A hint with a link to `/config/profile` tells the user to set expectations. The bar is colored neutral gray.
- **JD has only `salary_min` or only `salary_max`:** The single value is used as both min and max (point marker: zero-width bar with a visible marker at that value). `computeAxisRange` still produces valid padding.
- **Salary values are 0:** Treated as "not set" -- same as null. The `hasJDSalary` derived check uses `> 0` guard to prevent rendering a chart anchored at $0.
- **`BarChart` not yet registered (Phase 65 not landed):** This phase registers `BarChart` as well. If Phase 65 already registered it, the duplicate `echarts.use` call is idempotent.
- **`MarkAreaComponent` or `MarkLineComponent` not registered:** The chart renders without background bands and reference lines. The spec ensures these are registered in T66.3.

---

## Tasks

### T66.1: Write Database Migration

**File:** `packages/core/src/db/migrations/021_salary_structured_fields.sql`

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

[IMPORTANT] All new columns are nullable INTEGER. No CHECK constraints on ordering -- the API layer validates `salary_min <= salary_max` and `salary_minimum <= salary_target <= salary_stretch` on write.

**Acceptance criteria:**
- Migration adds 5 new columns (2 on `job_descriptions`, 3 on `user_profile`).
- All columns are nullable (existing rows get NULL values).
- Inserting a JD with `salary_min = 150000, salary_max = 200000` succeeds.
- Inserting profile with `salary_minimum = 120000, salary_target = 160000, salary_stretch = 200000` succeeds.
- Migration is registered in `_migrations` table.

**Failure criteria:**
- Column names collide with existing columns.
- Non-nullable constraint blocks migration on existing rows.

---

### T66.2: Update Types, Repository, API, and SDK

**Files (types):**
- `packages/core/src/types/index.ts`: Add `salary_min?: number | null` and `salary_max?: number | null` to `JobDescription` interface; add `salary_minimum?: number | null`, `salary_target?: number | null`, `salary_stretch?: number | null` to `UserProfile` interface.
- `packages/sdk/src/types.ts`: Mirror the same additions.

**Files (repository):**
- `packages/core/src/db/repositories/job-description-repository.ts`: Include `salary_min` and `salary_max` in INSERT, UPDATE, and SELECT column lists.
- `packages/core/src/db/repositories/profile-repository.ts`: Include `salary_minimum`, `salary_target`, `salary_stretch` in UPDATE and SELECT column lists.

**Files (API routes):**
- `packages/core/src/routes/job-descriptions.ts`: Accept `salary_min` and `salary_max` in POST and PATCH. Validate: if both are provided, `salary_min <= salary_max`. Return 400 if violated. Partial updates allowed (only one provided).
- `packages/core/src/routes/profile.ts`: Accept `salary_minimum`, `salary_target`, `salary_stretch` in PATCH. Validate ordering: `salary_minimum <= salary_target <= salary_stretch` when all three are provided. Partial updates allowed.

**Files (SDK):**
- `packages/sdk/src/resources/job-descriptions.ts`: Include `salary_min`/`salary_max` in create/update payloads.
- `packages/sdk/src/resources/profile.ts`: Include salary expectation fields in update payload.

[IMPORTANT] Validation is API-level only, not database-level. This keeps the migration simple (ALTER TABLE only) and allows the API to provide clear error messages.

**Acceptance criteria:**
- `GET /api/job-descriptions/:id` response includes `salary_min` and `salary_max`.
- `PATCH /api/job-descriptions/:id { salary_min: 200000, salary_max: 150000 }` returns 400.
- `PATCH /api/job-descriptions/:id { salary_min: 150000 }` succeeds (partial).
- `GET /api/profile` response includes `salary_minimum`, `salary_target`, `salary_stretch`.
- `PATCH /api/profile { salary_minimum: 200000, salary_target: 100000 }` returns 400.
- SDK create/update methods accept the new fields.

**Failure criteria:**
- New fields not included in SELECT, causing null responses even when data exists.
- Validation logic reversed (allowing min > max).

---

### T66.3: Register ECharts Components

**File:** `packages/webui/src/lib/components/charts/echarts-registry.ts` (modify existing)

Register `MarkAreaComponent` and `MarkLineComponent` from `echarts/components`. Also register `BarChart` if not already registered by Phase 65.

```typescript
import { MarkAreaComponent, MarkLineComponent } from 'echarts/components'
// import { BarChart } from 'echarts/charts'  // if not already registered

// Add to echarts.use([...]):
MarkAreaComponent,
MarkLineComponent,
// BarChart,  // if not already registered
```

**Acceptance criteria:**
- `MarkAreaComponent` and `MarkLineComponent` are registered.
- `setOption()` with `markArea` and `markLine` does not throw unregistered component errors.
- Existing registrations unchanged.

**Failure criteria:**
- Background bands and reference lines fail to render due to missing component registration.

---

### T66.4: Write Compensation Utility Functions

**File:** `packages/webui/src/lib/components/charts/compensation-utils.ts`

Pure utility functions for axis range computation, salary formatting, bar color determination, and the main ECharts option builder.

[CRITICAL] ECharts range bar: use stacked bar trick (invisible spacer + visible range), not `encode`. The invisible spacer bar goes from 0 to `jdMin`, the visible bar goes from `jdMin` to `jdMax` (data value = `jdMax - jdMin`), stacked on top. The invisible bar has `itemStyle: { color: 'transparent' }`, `emphasis: { disabled: true }`, and `tooltip: { show: false }`.

[IMPORTANT] Zero-salary guard: treat `salary_min = 0` and `salary_max = 0` as "not set". The component checks `> 0` before rendering.

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
export function computeAxisRange(data: CompensationData): [number, number] {
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
export function formatSalary(value: number): string {
  if (value >= 1000) {
    return `$${Math.round(value / 1000)}k`
  }
  return `$${value.toLocaleString()}`
}

/**
 * Determine JD bar color based on midpoint vs expectations.
 */
export function getJDBarColor(midpoint: number, expectations: SalaryExpectations): string {
  if (expectations.minimum != null && midpoint < expectations.minimum) return '#ef4444'
  if (expectations.target != null && midpoint < expectations.target) return '#f59e0b'
  if (expectations.stretch != null && midpoint <= expectations.stretch) return '#22c55e'
  if (expectations.stretch != null && midpoint > expectations.stretch) return '#6c63ff'
  return '#374151'  // neutral when no expectations set
}

/**
 * Build ECharts option for the compensation bullet graph.
 * Uses stacked bar trick: invisible spacer (0 to jdMin) + visible range (jdMin to jdMax).
 * Background bands via markArea, reference lines via markLine.
 */
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
    // Band 1 starts at `axisMin` (chart left edge), not `$0`. The spec's '$0'
    // refers to the conceptual floor — we only chart the visible range.
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

    // Band 4: above stretch (very light green)
    markAreaData.push([
      { xAxis: expectations.stretch, itemStyle: { color: 'rgba(34, 197, 94, 0.06)' } },
      { xAxis: axisMax },
    ])
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

**Acceptance criteria:**
- `computeAxisRange` includes padding around min/max values.
- `computeAxisRange` returns `[0, 200000]` when no values are provided.
- `formatSalary(150000)` returns `"$150k"`.
- `formatSalary(75000)` returns `"$75k"`.
- `getJDBarColor` returns correct color for each range (red/amber/green/blue/gray).
- `buildCompensationOption` produces two stacked series (invisible spacer + visible range).
- `buildCompensationOption` includes markArea bands when expectations are set.
- `buildCompensationOption` omits markArea/markLine when expectations are null.
- `buildCompensationOption` handles single salary value (point marker: range width = 0).

**Failure criteria:**
- ECharts `encode` used instead of stacked bar trick (range bars do not render).
- `computeAxisRange` divides by zero when min === max.
- `getJDBarColor` logic has gaps (midpoint exactly at boundary not handled).

---

### T66.5: Write CompensationBulletGraph Component

**File:** `packages/webui/src/lib/components/charts/CompensationBulletGraph.svelte`

[IMPORTANT] Profile salary expectations are loaded via `onMount` (not `$effect`) to avoid unnecessary re-fetches. The profile data does not depend on the JD and only needs to be loaded once per component lifecycle.

[IMPORTANT] Zero-salary guard: `hasJDSalary` is `$derived` with `> 0` check. Values of 0 are treated as "not set" to prevent a chart anchored at $0.

```svelte
<script lang="ts">
  import { onMount } from 'svelte'
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

  // Treat salary_min = 0 and salary_max = 0 as 'not set' -- same as null.
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

**Acceptance criteria:**
- Component accepts `jdTitle`, `salaryMin`, `salaryMax` props.
- Loading state shows while profile is being fetched.
- Empty state shows when JD has no structured salary data (both null or 0).
- Chart renders when JD has salary data.
- Profile settings hint shows when user has no salary expectations.
- Hint hidden when expectations are set.
- Chart height is compact (180px).
- `notMerge={true}` ensures clean chart updates.

**Failure criteria:**
- `onMount` causes SSR issues (it should not -- `onMount` only runs in browser).
- Zero-salary values render a chart anchored at $0.
- Chart re-fetches profile on every reactive tick.

---

### T66.6: Update JD Editor and Profile Page UI

**File (JD Editor):** `packages/webui/src/lib/components/jd/JDEditor.svelte`

Add two numeric input fields for `salary_min` and `salary_max` below the free-text `salary_range` field. Mount `CompensationBulletGraph` below the salary inputs.

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

{#if selectedJd}
  <CompensationBulletGraph
    jdTitle={selectedJd.title}
    salaryMin={form.salary_min}
    salaryMax={form.salary_max}
  />
{/if}
```

**File (Profile):** `packages/webui/src/routes/config/profile/+page.svelte`

Add three numeric input fields for salary expectations.

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

**Acceptance criteria:**
- JD editor shows `salary_min` and `salary_max` input fields.
- Saving a JD with structured salary values persists them.
- Profile editor shows salary expectation input fields.
- Saving profile with salary expectations persists them.
- Bullet graph renders in JD editor when salary data exists.
- `CompensationBulletGraph` updates reactively when salary inputs change.

**Failure criteria:**
- Input type is `text` instead of `number` (no numeric validation).
- Form binding does not include new fields in PATCH payload.

---

### T66.7: Write Compensation Utility Tests

**File:** `packages/webui/src/lib/components/charts/__tests__/compensation-utils.test.ts`

```typescript
import { describe, it, expect } from 'vitest'
import {
  computeAxisRange,
  formatSalary,
  getJDBarColor,
  buildCompensationOption,
} from '../compensation-utils'

describe('computeAxisRange', () => {
  it('includes padding around min/max values', () => {
    const data = {
      jdSalary: { min: 150000, max: 200000 },
      expectations: { minimum: null, target: null, stretch: null },
      jdTitle: 'Test',
    }
    const [min, max] = computeAxisRange(data)
    expect(min).toBeLessThan(150000)
    expect(max).toBeGreaterThan(200000)
  })

  it('returns default range when no values provided', () => {
    const data = {
      jdSalary: { min: null, max: null },
      expectations: { minimum: null, target: null, stretch: null },
      jdTitle: 'Test',
    }
    const [min, max] = computeAxisRange(data)
    expect(min).toBe(0)
    expect(max).toBe(200000)
  })
})

describe('formatSalary', () => {
  it('formats 150000 as "$150k"', () => {
    expect(formatSalary(150000)).toBe('$150k')
  })

  it('formats 75000 as "$75k"', () => {
    expect(formatSalary(75000)).toBe('$75k')
  })
})

describe('getJDBarColor', () => {
  const expectations = { minimum: 120000, target: 160000, stretch: 200000 }

  it('returns red when midpoint is below minimum', () => {
    expect(getJDBarColor(100000, expectations)).toBe('#ef4444')
  })

  it('returns amber when midpoint is between minimum and target', () => {
    expect(getJDBarColor(140000, expectations)).toBe('#f59e0b')
  })

  it('returns green when midpoint is between target and stretch', () => {
    expect(getJDBarColor(180000, expectations)).toBe('#22c55e')
  })

  it('returns blue when midpoint is above stretch', () => {
    expect(getJDBarColor(250000, expectations)).toBe('#6c63ff')
  })

  it('returns neutral gray when no expectations are set', () => {
    expect(getJDBarColor(150000, { minimum: null, target: null, stretch: null })).toBe('#374151')
  })
})

describe('buildCompensationOption', () => {
  it('produces option with two stacked bar series', () => {
    const data = {
      jdSalary: { min: 150000, max: 200000 },
      expectations: { minimum: 120000, target: 160000, stretch: 200000 },
      jdTitle: 'Sr Engineer',
    }
    const option = buildCompensationOption(data)
    expect(option.series).toHaveLength(2)
    expect(option.series[0].itemStyle.color).toBe('transparent') // invisible spacer
    expect(option.series[0].data[0]).toBe(150000) // spacer height = jdMin
    expect(option.series[1].data[0]).toBe(50000) // visible bar = jdMax - jdMin
  })

  it('includes markArea bands when expectations are set', () => {
    const data = {
      jdSalary: { min: 150000, max: 200000 },
      expectations: { minimum: 120000, target: 160000, stretch: 200000 },
      jdTitle: 'Test',
    }
    const option = buildCompensationOption(data)
    expect(option.series[1].markArea.data.length).toBeGreaterThan(0)
  })

  it('omits markArea bands when expectations are null', () => {
    const data = {
      jdSalary: { min: 150000, max: 200000 },
      expectations: { minimum: null, target: null, stretch: null },
      jdTitle: 'Test',
    }
    const option = buildCompensationOption(data)
    expect(option.series[1].markArea.data).toHaveLength(0)
    expect(option.series[1].markLine.data).toHaveLength(0)
  })

  it('includes markLine reference lines for min/target/stretch', () => {
    const data = {
      jdSalary: { min: 150000, max: 200000 },
      expectations: { minimum: 120000, target: 160000, stretch: 200000 },
      jdTitle: 'Test',
    }
    const option = buildCompensationOption(data)
    expect(option.series[1].markLine.data).toHaveLength(3) // min, target, stretch
  })

  it('handles single salary value (min only)', () => {
    const data = {
      jdSalary: { min: 150000, max: null },
      expectations: { minimum: null, target: null, stretch: null },
      jdTitle: 'Test',
    }
    const option = buildCompensationOption(data)
    expect(option.series[0].data[0]).toBe(150000)
    expect(option.series[1].data[0]).toBe(0) // zero-width range
  })

  it('handles single salary value (max only)', () => {
    const data = {
      jdSalary: { min: null, max: 200000 },
      expectations: { minimum: null, target: null, stretch: null },
      jdTitle: 'Test',
    }
    const option = buildCompensationOption(data)
    expect(option.series[0].data[0]).toBe(200000)
    expect(option.series[1].data[0]).toBe(0)
  })

  it('includes Band 4 (above stretch) markArea when stretch is set', () => {
    const data = {
      jdSalary: { min: 150000, max: 200000 },
      expectations: { minimum: 120000, target: 160000, stretch: 200000 },
      jdTitle: 'Test',
    }
    const option = buildCompensationOption(data)
    // 4 bands: below min, min-target, target-stretch, above stretch
    expect(option.series[1].markArea.data).toHaveLength(4)
    const band4 = option.series[1].markArea.data[3]
    expect(band4[0].xAxis).toBe(200000) // stretch value
    expect(band4[0].itemStyle.color).toBe('rgba(34, 197, 94, 0.06)')
  })
})
```

**Acceptance criteria:**
- All 15 test cases pass.
- Axis range, formatting, color logic, and option structure validated.
- Edge cases (null expectations, single salary value, Band 4 above stretch) handled.

**Failure criteria:**
- Any test fails, indicating a bug in utility functions.

---

## Testing

### Migration Tests

> **Note:** Migration 021 integration tests are part of the general migration test suite (`migrate.test.ts`). No separate migration test file is needed -- add the expected migration count and name to the existing assertions.

| Test | Assertion |
|------|-----------|
| Migration adds `salary_min`/`salary_max` to `job_descriptions` | Column exists, nullable |
| Migration adds `salary_minimum`/`salary_target`/`salary_stretch` to `user_profile` | Columns exist, nullable |
| Existing rows get NULL values | No data loss on migration |

### API Tests

| Test | Assertion |
|------|-----------|
| PATCH JD with valid salary range | 200, values persisted |
| PATCH JD with min > max | 400 |
| PATCH JD with partial salary | 200, single value persisted |
| GET JD includes salary fields | Response has `salary_min`/`salary_max` |
| PATCH profile with valid expectations | 200, values persisted |
| PATCH profile with minimum > target | 400 |
| GET profile includes salary fields | Response has all three expectation fields |

### Utility Unit Tests

**File:** `packages/webui/src/lib/components/charts/__tests__/compensation-utils.test.ts` (T66.7)

| Test | Assertion |
|------|-----------|
| `computeAxisRange` pads min/max | Result extends beyond input range |
| `computeAxisRange` default range | `[0, 200000]` for no values |
| `formatSalary(150000)` | `"$150k"` |
| `formatSalary(75000)` | `"$75k"` |
| `getJDBarColor` red | Below minimum |
| `getJDBarColor` amber | Between minimum and target |
| `getJDBarColor` green | Between target and stretch |
| `getJDBarColor` blue | Above stretch |
| `getJDBarColor` neutral | No expectations set |
| `buildCompensationOption` two series | Spacer + visible bar |
| `buildCompensationOption` markArea present | When expectations set |
| `buildCompensationOption` markArea absent | When expectations null |
| `buildCompensationOption` markLine count | 3 reference lines |
| `buildCompensationOption` single salary | Zero-width range |
| `buildCompensationOption` Band 4 above stretch | 4th markArea from stretch to axisMax with `rgba(34, 197, 94, 0.06)` |

### Component Tests (Manual / Future)

| Test | What to verify |
|------|---------------|
| Loading state | "Loading compensation data..." during profile fetch |
| Empty state | "No salary data" when both min/max null |
| Chart renders | EChart div with bar series when salary data exists |
| Hint visible | Profile settings link when no expectations |
| Hint hidden | When expectations are set |
| Bar color correct | Red/amber/green/blue based on midpoint |
| Reactive updates | Chart changes when salary props change |

---

## Documentation Requirements

- No new documentation files required.
- The spec file serves as the design document.
- This plan file serves as the implementation reference.
- Inline TSDoc comments on all exported interfaces and functions:
  - `SalaryExpectations`, `JDSalary`, `CompensationData`: field semantics
  - `computeAxisRange`: padding logic, default range
  - `formatSalary`: formatting rules
  - `getJDBarColor`: threshold logic and color meanings
  - `buildCompensationOption`: stacked bar trick explanation, markArea/markLine structure

---

## Parallelization Notes

**Within this phase:**
- T66.1 (migration) is foundational -- no dependencies.
- T66.2 (types/repo/API/SDK) depends on T66.1 (migration must exist for integration tests).
- T66.3 (registry) is independent of T66.1/T66.2.
- T66.4 (utils) is independent of T66.1/T66.2/T66.3.
- T66.5 (component) depends on T66.4 (imports utils).
- T66.6 (UI mounting) depends on T66.2 (API fields) and T66.5 (component).
- T66.7 (tests) depends on T66.4 (tests the utils).

**Recommended execution order:**
1. T66.1 (migration) + T66.3 (registry) + T66.4 (utils) -- all three parallel
2. T66.2 (types/repo/API/SDK) + T66.5 (component) + T66.7 (tests) -- parallel, each depends on one item above
3. T66.6 (UI mounting) -- depends on T66.2 and T66.5

**Cross-phase:**
- Phase 65 and 66 both modify `echarts-registry.ts` and `JDEditor.svelte` -- coordinated merge needed.
- Phase 66 is the only phase that modifies database schema, API routes, and profile page.
