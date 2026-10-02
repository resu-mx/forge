# Phase 65: JD Skill Alignment Spider Chart (Spec E4)

**Status:** Planning
**Date:** 2026-04-03
**Spec:** [2026-04-03-jd-skill-alignment-chart.md](../refs/specs/2026-04-03-jd-skill-alignment-chart.md)
**Depends on:** Phase 49 (JD Detail — `job_description_skills` junction, skill tagging UI), Phase 59 (ECharts Infrastructure — `EChart.svelte` wrapper, `echarts-registry.ts`, `echarts-theme.ts`)
**Blocks:** None
**Parallelizable with:** Phase 66, Phase 67, Phase 68 -- creates new files, only modifies `echarts-registry.ts` and `JDEditor.svelte`

## Goal

Add a radar (spider) chart to the JD detail page that visualizes skill alignment between the user's skill inventory and a job description's required skills. The chart overlays two polygons -- JD requirements (purple) vs. user skills (green) -- grouped by skill category, with a center match percentage and a fallback horizontal bar chart when fewer than 3 categories exist. A gap list below the chart enumerates unmatched skills as pill badges.

## Non-Goals

- Modifying skills or JD data from the chart (read-only visualization)
- Weighting skills by proficiency level (binary: have it or not)
- Per-skill granularity on radar axes (axes are categories; individual skills are in tooltips)
- Suggesting skills to learn (future gap-analysis feature)
- Comparing across multiple JDs simultaneously
- Adding new API endpoints (client-side aggregation from existing endpoints)
- Mobile-specific interactions, chart export, accessibility

## Context

The JD detail page (Phase 49) includes a skill tagging UI where users tag skills required by a JD. The `job_description_skills` junction table stores these associations. User skills are derived from bullet technologies (`bullet_technologies` junction). This chart compares those two sets by category, giving the user an instant visual read on how well they match a given JD.

ECharts infrastructure (Phase 59) provides the `EChart.svelte` wrapper and `echarts-registry.ts` for tree-shakeable chart type registration. This phase registers `RadarChart`, `RadarComponent`, and `BarChart` (for fallback).

## Scope

| Spec section | Covered here? |
|-------------|---------------|
| 1. Chart Design (radar structure, axes, scaling, minimum axes fallback) | Yes |
| 2. Data Loading (SDK calls, aggregation logic, `jd-radar-utils.ts`) | Yes |
| 3. ECharts Configuration (radar option, fallback bar option, registry) | Yes |
| 4. Component (`JDSkillRadar.svelte`, props, gap list) | Yes |
| 5. Placement (JD editor integration, conditional rendering) | Yes |
| 6-7. Files to create/modify | Yes |
| 8. Testing | Yes |
| 9. Acceptance criteria | Yes |
| 10. Future enhancements | No (informational only) |

> **Spec deviation:** Section 1 axis set is JD-centric (user categories not in JD are omitted). This is an intentional narrowing from the spec's "either JD or user" wording.

## Files to Create

| File | Description |
|------|-------------|
| `packages/webui/src/lib/components/charts/jd-radar-utils.ts` | `computeJDSkillAlignment`, `buildRadarOption`, `buildFallbackBarOption` — aggregation and ECharts option builders |
| `packages/webui/src/lib/components/charts/JDSkillRadar.svelte` | Radar chart component with loading, error, empty, and chart states; includes gap list |
| `packages/webui/src/lib/components/charts/__tests__/jd-radar-utils.test.ts` | Unit tests for aggregation and option builders (11 cases) |

## Files to Modify

| File | Change |
|------|--------|
| `packages/webui/src/lib/components/charts/echarts-registry.ts` | Register `RadarChart` from `echarts/charts`, `RadarComponent` from `echarts/components`, and `BarChart` from `echarts/charts` |
| `packages/webui/src/lib/components/jd/JDEditor.svelte` | Import `JDSkillRadar` and mount it below the skill tags section, guarded by `{#if selectedJd}` |

## Fallback Strategies

- **Fewer than 3 skill categories:** Radar chart degrades gracefully to a horizontal stacked bar chart. A radar with 2 axes renders as a line (not a polygon), which is uninformative. The component switches to `buildFallbackBarOption` automatically via `$derived`.
- **Zero JD skills tagged:** The component renders a clear empty state message ("No required skills tagged on this JD. Add skills to see alignment.") instead of a broken chart.
- **API failure on any of the three concurrent requests:** `Promise.all` rejects, the catch block sets an error state, and the component renders an error message. No crash.
- **Bullet list with zero technologies:** `userSkillNames` set is empty, so all JD skills appear as gaps. Match percentage is 0%. The chart renders correctly with the red color-coded percentage.
- **Radar polygon mismatch between two series:** Both series share the same `indicator` array (same category order, same max values), so polygons always align. The JD polygon shows full reach; the user polygon shows matched reach.
- **`BarChart` or `RadarChart` not registered:** The ECharts console error for unregistered chart types is caught at the `EChart.svelte` wrapper level. This phase ensures registration in T65.3.

---

## Tasks

### T65.1: Write Aggregation and Option Builder Utilities

**File:** `packages/webui/src/lib/components/charts/jd-radar-utils.ts`

Defines the data structures and pure functions for computing JD-to-user skill alignment and generating ECharts options.

[IMPORTANT] User skills come from bullet technologies only (MVP). Source skills are not exposed via SDK yet. The `technologies` field on each bullet is an array of technology names (strings). Matching is case-insensitive.

[IMPORTANT] Categories with zero JD-required skills do not appear as axes. The chart is JD-centric -- only categories present in the JD's skill set are shown.

```typescript
export interface CategoryComparison {
  category: string
  jdCount: number        // how many JD-required skills in this category
  matchedCount: number   // how many of those the user has
  jdSkills: string[]     // names of JD skills in this category
  matchedSkills: string[] // names of matched skills
  gapSkills: string[]    // names of unmatched skills
}

export interface JDSkillAlignment {
  categories: CategoryComparison[]
  totalJDSkills: number
  totalMatched: number
  matchPercentage: number  // 0-100
}

/**
 * Compare JD-required skills against the user's skill inventory.
 *
 * User skills are derived from bullet technologies (bullet_technologies junction).
 * JD skills come from job_description_skills junction.
 * Skills are matched by name (case-insensitive).
 */
export function computeJDSkillAlignment(
  jdSkills: Skill[],
  allSkills: Skill[],
  bullets: Bullet[],
): JDSkillAlignment {
  // Build a set of user's skill names (from bullet technologies)
  const userSkillNames = new Set<string>()
  for (const bullet of bullets) {
    for (const tech of bullet.technologies) {
      userSkillNames.add(tech.toLowerCase())
    }
  }

  // Build a map: skill id -> skill (for category lookup)
  const skillById = new Map(allSkills.map(s => [s.id, s]))

  // Group JD skills by category, compare against user skills
  const categoryMap = new Map<string, CategoryComparison>()

  for (const jdSkill of jdSkills) {
    const fullSkill = skillById.get(jdSkill.id) ?? jdSkill
    const category = fullSkill.category ?? 'uncategorized'

    if (!categoryMap.has(category)) {
      categoryMap.set(category, {
        category,
        jdCount: 0,
        matchedCount: 0,
        jdSkills: [],
        matchedSkills: [],
        gapSkills: [],
      })
    }

    const cat = categoryMap.get(category)!
    cat.jdCount++
    cat.jdSkills.push(fullSkill.name)

    if (userSkillNames.has(fullSkill.name.toLowerCase())) {
      cat.matchedCount++
      cat.matchedSkills.push(fullSkill.name)
    } else {
      cat.gapSkills.push(fullSkill.name)
    }
  }

  const categories = Array.from(categoryMap.values())
    .sort((a, b) => b.jdCount - a.jdCount)

  const totalJDSkills = categories.reduce((sum, c) => sum + c.jdCount, 0)
  const totalMatched = categories.reduce((sum, c) => sum + c.matchedCount, 0)
  const matchPercentage = totalJDSkills === 0
    ? 100
    : Math.round((totalMatched / totalJDSkills) * 100)

  return { categories, totalJDSkills, totalMatched, matchPercentage }
}

/**
 * Build ECharts radar option for JD skill alignment.
 * Requires 3+ categories for a meaningful polygon.
 */
export function buildRadarOption(alignment: JDSkillAlignment): EChartsOption {
  const categories = alignment.categories
  const maxValues = categories.map(c => c.jdCount)

  return {
    tooltip: {
      trigger: 'item',
      // [IMPORTANT] The tooltip formatter uses the `categories` closure directly
      // (available in scope). `params.value` is not needed here — the closure
      // provides the same data with clearer semantics.
      formatter: (params: any) => {
        const seriesName = params.seriesName
        return categories
          .map((cat, i) => {
            const matched = cat.matchedSkills.join(', ') || 'none'
            const gaps = cat.gapSkills.join(', ') || 'none'
            if (seriesName === 'JD Requirements') {
              return `<strong>${cat.category}</strong>: ${cat.jdCount} skill${cat.jdCount !== 1 ? 's' : ''}<br/>${cat.jdSkills.join(', ')}`
            }
            return `<strong>${cat.category}</strong>: ${cat.matchedCount}/${cat.jdCount}<br/>Matched: ${matched}<br/>Gaps: ${gaps}`
          })
          .join('<br/><br/>')
      },
    },
    legend: {
      data: ['JD Requirements', 'Your Skills'],
      bottom: 10,
    },
    radar: {
      indicator: categories.map(c => ({
        name: `${c.category}\n(${c.matchedCount}/${c.jdCount})`,
        max: c.jdCount,
      })),
      center: ['50%', '50%'],
      radius: '65%',
      axisName: {
        fontSize: 11,
        color: '#6b7280',
      },
      splitNumber: 4,
      splitArea: {
        areaStyle: {
          color: ['rgba(0,0,0,0.02)', 'rgba(0,0,0,0.04)'],
        },
      },
    },
    series: [
      {
        type: 'radar',
        data: [
          {
            name: 'JD Requirements',
            value: categories.map(c => c.jdCount),
            symbol: 'circle',
            symbolSize: 6,
            lineStyle: { width: 2, color: '#6c63ff' },
            areaStyle: { color: 'rgba(108, 99, 255, 0.12)' },
            itemStyle: { color: '#6c63ff' },
          },
          {
            name: 'Your Skills',
            value: categories.map(c => c.matchedCount),
            symbol: 'circle',
            symbolSize: 6,
            lineStyle: { width: 2, color: '#22c55e' },
            areaStyle: { color: 'rgba(34, 197, 94, 0.25)' },
            itemStyle: { color: '#22c55e' },
          },
        ],
      },
    ],
    graphic: [
      {
        type: 'text',
        left: 'center',
        top: '46%',
        style: {
          text: `${alignment.matchPercentage}%`,
          textAlign: 'center',
          fontSize: 28,
          fontWeight: 700,
          fill: alignment.matchPercentage >= 75 ? '#22c55e'
            : alignment.matchPercentage >= 50 ? '#f59e0b'
            : '#ef4444',
        },
      },
      {
        type: 'text',
        left: 'center',
        top: '53%',
        style: {
          text: 'match',
          textAlign: 'center',
          fontSize: 11,
          fill: '#6b7280',
        },
      },
    ],
  }
}

/**
 * Fallback bar chart option for < 3 categories.
 * Horizontal stacked bar showing matched vs gap per category.
 */
export function buildFallbackBarOption(alignment: JDSkillAlignment): EChartsOption {
  return {
    tooltip: {
      trigger: 'axis',
      axisPointer: { type: 'shadow' },
    },
    legend: {
      data: ['Matched', 'Gap'],
      bottom: 10,
    },
    grid: {
      left: '15%',
      right: '10%',
      top: '10%',
      bottom: '15%',
    },
    xAxis: {
      type: 'value',
      name: 'Skills',
    },
    yAxis: {
      type: 'category',
      data: alignment.categories.map(c => c.category),
    },
    series: [
      {
        name: 'Matched',
        type: 'bar',
        stack: 'total',
        data: alignment.categories.map(c => c.matchedCount),
        itemStyle: { color: '#22c55e' },
      },
      {
        name: 'Gap',
        type: 'bar',
        stack: 'total',
        data: alignment.categories.map(c => c.jdCount - c.matchedCount),
        itemStyle: { color: '#e5e7eb' },
      },
    ],
    graphic: [
      {
        type: 'text',
        right: '10%',
        top: 5,
        style: {
          text: `${alignment.matchPercentage}% match`,
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
- All three functions export correctly and produce valid objects.
- `computeJDSkillAlignment` groups skills by category, counts matched/gap, computes percentage.
- `computeJDSkillAlignment` with no JD skills returns `matchPercentage: 100` and empty categories.
- `computeJDSkillAlignment` uses case-insensitive matching (`"python"` matches `"Python"`).
- `computeJDSkillAlignment` groups null-category skills under `"uncategorized"`.
- `computeJDSkillAlignment` sorts categories by `jdCount` descending.
- `buildRadarOption` produces option with correct indicator count matching categories.
- `buildRadarOption` includes two radar series (JD Requirements, Your Skills).
- `buildRadarOption` includes graphic text element with match percentage.
- `buildRadarOption` uses green/amber/red color for percentage based on threshold.
- `buildFallbackBarOption` produces valid stacked bar option.
- `buildFallbackBarOption` includes match percentage text in graphic.

**Failure criteria:**
- Case-sensitive matching causes false negatives ("python" not matching "Python").
- Null category throws instead of defaulting to "uncategorized".
- Radar option has mismatched indicator count vs. series data length.

---

### T65.2: Write JDSkillRadar Component

**File:** `packages/webui/src/lib/components/charts/JDSkillRadar.svelte`

The component fetches JD skills, all skills, and all bullets concurrently, computes alignment, and renders either a radar chart (3+ categories) or bar chart fallback (< 3 categories). Below the chart, unmatched skills are shown as pill badges grouped by category.

[IMPORTANT] The component uses `$effect` (not `onMount`) for data loading because it must reload reactively when `jdId` changes. The `$effect` tracks `jdId` and calls `loadAlignment(jdId)`, which writes to `$state` variables. This is the correct pattern for prop-driven data fetching -- `onMount` would only fire once.

[IMPORTANT] The chart uses `notMerge={true}` to ensure clean option swap between radar and bar chart types when the number of categories changes.

```svelte
<script lang="ts">
  import { forge } from '$lib/sdk'
  import EChart from './EChart.svelte'
  import {
    computeJDSkillAlignment,
    buildRadarOption,
    buildFallbackBarOption,
  } from './jd-radar-utils'
  import type { JDSkillAlignment } from './jd-radar-utils'
  import type { Skill, Bullet } from '@forge/sdk'

  let { jdId }: { jdId: string } = $props()

  let loading = $state(true)
  let alignment = $state<JDSkillAlignment | null>(null)
  let error = $state<string | null>(null)

  // Reactive: reload when jdId changes
  $effect(() => {
    if (!jdId) return  // guard: skip if jdId is not set
    loadAlignment(jdId)
  })

  async function loadAlignment(id: string) {
    loading = true
    error = null

    try {
      const [jdSkillsRes, allSkillsRes, bulletsRes] = await Promise.all([
        forge.jobDescriptions.listSkills(id),
        forge.skills.list({ limit: 500 }),
        forge.bullets.list({ limit: 2000 }),
      ])

      if (!jdSkillsRes.ok || !allSkillsRes.ok || !bulletsRes.ok) {
        error = 'Failed to load skill data'
        return
      }

      alignment = computeJDSkillAlignment(
        jdSkillsRes.data,
        allSkillsRes.data,
        bulletsRes.data,
      )
    } catch (e) {
      error = 'An error occurred loading skill alignment'
    } finally {
      loading = false
    }
  }

  let chartOption = $derived(
    alignment
      ? alignment.categories.length >= 3
        ? buildRadarOption(alignment)
        : buildFallbackBarOption(alignment)
      : null
  )
</script>

{#if loading}
  <div class="radar-loading">Loading skill alignment...</div>
{:else if error}
  <div class="radar-error">{error}</div>
{:else if alignment && alignment.totalJDSkills === 0}
  <div class="radar-empty">
    No required skills tagged on this JD. Add skills to see alignment.
  </div>
{:else if chartOption}
  <div class="radar-container">
    <div class="radar-header">
      <h3>Skill Alignment</h3>
      <span class="radar-summary">
        {alignment!.totalMatched} of {alignment!.totalJDSkills} required skills matched
      </span>
    </div>
    <EChart
      option={chartOption}
      height="400px"
      notMerge={true}
    />
    {#if alignment!.categories.some(c => c.gapSkills.length > 0)}
      <div class="gap-list">
        <h4>Skill Gaps</h4>
        {#each alignment!.categories.filter(c => c.gapSkills.length > 0) as cat}
          <div class="gap-category">
            <span class="gap-category-name">{cat.category}</span>
            <span class="gap-skills">
              {#each cat.gapSkills as skill}
                <span class="gap-pill">{skill}</span>
              {/each}
            </span>
          </div>
        {/each}
      </div>
    {/if}
  </div>
{/if}
```

**Acceptance criteria:**
- Component accepts `jdId` prop and renders reactively when it changes.
- Loading state shows while data is being fetched.
- Error state shows on API failure with user-facing message.
- Empty state shows when JD has zero tagged skills.
- Radar chart renders when JD has 3+ skill categories.
- Bar chart fallback renders when JD has fewer than 3 skill categories.
- Match summary text ("X of Y required skills matched") is displayed.
- Gap list shows unmatched skills grouped by category as pill badges.
- Gap list is hidden when all JD skills are matched (100%).
- Chart uses `notMerge={true}` for clean radar-to-bar transitions.

**Failure criteria:**
- `$effect` causes infinite loop (it should not -- `loadAlignment` writes to `alignment`/`loading`/`error`, none of which are tracked in the effect's dependency besides `jdId`).
- Component crashes when `jdId` is initially undefined/null.
- Chart shows stale data from previous JD when switching between JDs.

---

### T65.3: Register ECharts Chart Types

**File:** `packages/webui/src/lib/components/charts/echarts-registry.ts` (modify existing)

[IMPORTANT] Register both `RadarChart` from `echarts/charts` AND `RadarComponent` from `echarts/components`. `RadarComponent` provides the radar coordinate system (the `radar` option key). Without it, radar charts fail to render with a silent error. `BarChart` is also registered for the fallback bar chart.

Add to the existing `echarts.use([...])` call:

```typescript
import { RadarChart } from 'echarts/charts'
import { BarChart } from 'echarts/charts'
import { RadarComponent } from 'echarts/components'

// Add to echarts.use([...]):
RadarChart,
RadarComponent,
BarChart,
```

**Acceptance criteria:**
- `RadarChart`, `RadarComponent`, and `BarChart` are registered.
- Existing chart types remain registered (no removals).
- `setOption()` with a radar type does not throw "Component radar is not registered."
- `setOption()` with a bar type does not throw "Component bar is not registered."

**Failure criteria:**
- `RadarComponent` omitted, causing `radar` coordinate system to be unavailable.
- Existing registrations disrupted by the additions.

---

### T65.4: Mount Component on JD Detail Page

**File:** `packages/webui/src/lib/components/jd/JDEditor.svelte` (modify existing)

Mount `JDSkillRadar` below the skill tags section and above the notes textarea. The component is conditionally rendered only when a JD is selected.

```svelte
<!-- In JDEditor.svelte, after the skill picker section -->
{#if selectedJd}
  <JDSkillRadar jdId={selectedJd.id} />
{/if}
```

Import:

```svelte
import JDSkillRadar from '$lib/components/charts/JDSkillRadar.svelte'
```

**Acceptance criteria:**
- `JDSkillRadar` appears on the JD detail page below skill tags.
- Component is not rendered when no JD is selected.
- Component re-renders when a different JD is selected.

**Failure criteria:**
- Import path incorrect, causing build error.
- Component rendered outside the selected-JD guard, causing null reference.

---

### T65.5: Write Aggregation Unit Tests

**File:** `packages/webui/src/lib/components/charts/__tests__/jd-radar-utils.test.ts`

```typescript
import { describe, it, expect } from 'vitest'
import {
  computeJDSkillAlignment,
  buildRadarOption,
  buildFallbackBarOption,
} from '../jd-radar-utils'

describe('computeJDSkillAlignment', () => {
  it('returns matchPercentage 100 and empty categories for no JD skills', () => {
    const result = computeJDSkillAlignment([], [], [])
    expect(result.matchPercentage).toBe(100)
    expect(result.categories).toHaveLength(0)
    expect(result.totalJDSkills).toBe(0)
    expect(result.totalMatched).toBe(0)
  })

  it('returns matchPercentage 100 when all JD skills match user skills', () => {
    const jdSkills = [
      { id: 's1', name: 'Python', category: 'languages' },
      { id: 's2', name: 'AWS', category: 'cloud' },
    ]
    const allSkills = [...jdSkills]
    const bullets = [
      { technologies: ['Python', 'AWS', 'Docker'] },
    ]
    const result = computeJDSkillAlignment(jdSkills as any, allSkills as any, bullets as any)
    expect(result.matchPercentage).toBe(100)
    expect(result.totalMatched).toBe(2)
  })

  it('returns matchPercentage 0 when no JD skills match', () => {
    const jdSkills = [
      { id: 's1', name: 'Rust', category: 'languages' },
      { id: 's2', name: 'GCP', category: 'cloud' },
    ]
    const allSkills = [...jdSkills]
    const bullets = [
      { technologies: ['Python', 'AWS'] },
    ]
    const result = computeJDSkillAlignment(jdSkills as any, allSkills as any, bullets as any)
    expect(result.matchPercentage).toBe(0)
    expect(result.totalMatched).toBe(0)
  })

  it('groups by category and counts matched/gap skills correctly', () => {
    const jdSkills = [
      { id: 's1', name: 'Python', category: 'languages' },
      { id: 's2', name: 'Go', category: 'languages' },
      { id: 's3', name: 'AWS', category: 'cloud' },
      { id: 's4', name: 'GCP', category: 'cloud' },
    ]
    const allSkills = [...jdSkills]
    const bullets = [
      { technologies: ['Python', 'AWS'] },
    ]
    const result = computeJDSkillAlignment(jdSkills as any, allSkills as any, bullets as any)
    expect(result.matchPercentage).toBe(50)
    expect(result.totalMatched).toBe(2)
    expect(result.totalJDSkills).toBe(4)

    const langs = result.categories.find(c => c.category === 'languages')!
    expect(langs.jdCount).toBe(2)
    expect(langs.matchedCount).toBe(1)
    expect(langs.matchedSkills).toEqual(['Python'])
    expect(langs.gapSkills).toEqual(['Go'])

    const cloud = result.categories.find(c => c.category === 'cloud')!
    expect(cloud.jdCount).toBe(2)
    expect(cloud.matchedCount).toBe(1)
    expect(cloud.matchedSkills).toEqual(['AWS'])
    expect(cloud.gapSkills).toEqual(['GCP'])
  })

  it('uses case-insensitive matching', () => {
    const jdSkills = [{ id: 's1', name: 'Python', category: 'languages' }]
    const allSkills = [...jdSkills]
    const bullets = [{ technologies: ['python'] }]
    const result = computeJDSkillAlignment(jdSkills as any, allSkills as any, bullets as any)
    expect(result.matchPercentage).toBe(100)
    expect(result.totalMatched).toBe(1)
  })

  it('groups null category under "uncategorized"', () => {
    const jdSkills = [{ id: 's1', name: 'WidgetTool', category: null }]
    const allSkills = [...jdSkills]
    const bullets: any[] = []
    const result = computeJDSkillAlignment(jdSkills as any, allSkills as any, bullets)
    expect(result.categories[0].category).toBe('uncategorized')
  })

  it('sorts categories by jdCount descending', () => {
    const jdSkills = [
      { id: 's1', name: 'Python', category: 'languages' },
      { id: 's2', name: 'AWS', category: 'cloud' },
      { id: 's3', name: 'GCP', category: 'cloud' },
      { id: 's4', name: 'Azure', category: 'cloud' },
    ]
    const allSkills = [...jdSkills]
    const result = computeJDSkillAlignment(jdSkills as any, allSkills as any, [])
    expect(result.categories[0].category).toBe('cloud')
    expect(result.categories[0].jdCount).toBe(3)
    expect(result.categories[1].category).toBe('languages')
    expect(result.categories[1].jdCount).toBe(1)
  })
})

describe('buildRadarOption', () => {
  it('produces option with correct indicator count', () => {
    const alignment = {
      categories: [
        { category: 'cloud', jdCount: 3, matchedCount: 2, jdSkills: [], matchedSkills: [], gapSkills: [] },
        { category: 'languages', jdCount: 2, matchedCount: 1, jdSkills: [], matchedSkills: [], gapSkills: [] },
        { category: 'devops', jdCount: 1, matchedCount: 0, jdSkills: [], matchedSkills: [], gapSkills: [] },
      ],
      totalJDSkills: 6,
      totalMatched: 3,
      matchPercentage: 50,
    }
    const option = buildRadarOption(alignment)
    expect(option.radar.indicator).toHaveLength(3)
    expect(option.series[0].data).toHaveLength(2) // two polygon series
  })

  it('includes graphic text with match percentage', () => {
    const alignment = {
      categories: [
        { category: 'a', jdCount: 1, matchedCount: 1, jdSkills: [], matchedSkills: [], gapSkills: [] },
        { category: 'b', jdCount: 1, matchedCount: 0, jdSkills: [], matchedSkills: [], gapSkills: [] },
        { category: 'c', jdCount: 1, matchedCount: 0, jdSkills: [], matchedSkills: [], gapSkills: [] },
      ],
      totalJDSkills: 3,
      totalMatched: 1,
      matchPercentage: 33,
    }
    const option = buildRadarOption(alignment)
    const percentText = option.graphic[0]
    expect(percentText.style.text).toBe('33%')
    expect(percentText.style.fill).toBe('#ef4444') // red for < 50%
  })
})

describe('buildFallbackBarOption', () => {
  it('produces valid stacked bar option with match percentage', () => {
    const alignment = {
      categories: [
        { category: 'cloud', jdCount: 3, matchedCount: 2, jdSkills: [], matchedSkills: [], gapSkills: [] },
      ],
      totalJDSkills: 3,
      totalMatched: 2,
      matchPercentage: 67,
    }
    const option = buildFallbackBarOption(alignment)
    expect(option.series).toHaveLength(2) // Matched + Gap stacked
    expect(option.series[0].name).toBe('Matched')
    expect(option.series[1].name).toBe('Gap')
    expect(option.graphic[0].style.text).toBe('67% match')
  })
})
```

**Acceptance criteria:**
- All 11 test cases pass.
- Case-insensitive matching verified.
- Null category handling verified.
- Sort ordering verified.
- Both option builders produce structurally valid ECharts options.

**Failure criteria:**
- Any test fails, indicating a bug in aggregation logic or option building.

---

## Testing

### Unit Tests

**File:** `packages/webui/src/lib/components/charts/__tests__/jd-radar-utils.test.ts` (T65.5)

| Test | Assertion |
|------|-----------|
| No JD skills returns 100% match, empty categories | `matchPercentage: 100`, `categories: []` |
| All JD skills match returns 100% | `matchPercentage: 100`, `totalMatched: 2` |
| No JD skills match returns 0% | `matchPercentage: 0`, `totalMatched: 0` |
| Groups by category with correct matched/gap counts | Category-level assertions on `jdCount`, `matchedCount`, `matchedSkills`, `gapSkills` |
| Case-insensitive matching | `"python"` matches `"Python"` |
| Null category grouped under `"uncategorized"` | First category is `"uncategorized"` |
| Sorts categories by jdCount descending | Cloud (3) before languages (1) |
| Radar option has correct indicator count | `indicator.length === 3` for 3 categories |
| Radar option includes percentage graphic | `text: '33%'`, `fill: '#ef4444'` |
| Fallback bar option has stacked series | Two series: Matched + Gap |
| Fallback bar option includes percentage text | `text: '67% match'` |

### Component Tests (Manual / Future)

| Test | What to verify |
|------|---------------|
| Loading state | "Loading skill alignment..." appears while fetching |
| Empty state | "No required skills tagged" when JD has zero skills |
| Radar chart renders | EChart div with radar series for 3+ categories |
| Bar chart fallback | EChart div with bar series for < 3 categories |
| Match summary text | "X of Y required skills matched" |
| Gap list visible | Unmatched skills as pills grouped by category |
| Gap list hidden at 100% | No gap list when all matched |
| Reactive reload | Changing `jdId` triggers fresh data fetch |
| Error state | Error message on API failure |
| `notMerge` applied | Clean chart swap when switching between JDs |

### Registry Tests

| Test | What to verify |
|------|---------------|
| `RadarChart` registered | ECharts `setOption` with radar type does not throw |
| `RadarComponent` registered | `radar` coordinate system available |
| `BarChart` registered | ECharts `setOption` with bar type does not throw |

---

## Documentation Requirements

- No new documentation files required.
- The spec file serves as the design document.
- This plan file serves as the implementation reference.
- Inline TSDoc comments on all exported interfaces and functions:
  - `CategoryComparison`: field semantics
  - `JDSkillAlignment`: aggregated result structure
  - `computeJDSkillAlignment`: data source explanation, matching strategy
  - `buildRadarOption`: indicator/series structure, tooltip format
  - `buildFallbackBarOption`: stacked bar structure, when used

---

## Parallelization Notes

**Within this phase:**
- T65.1 (utils) is foundational -- no dependencies.
- T65.2 (component) depends on T65.1 (imports utils).
- T65.3 (registry) is independent of T65.1 and T65.2.
- T65.4 (mounting) depends on T65.2 (component must exist).
- T65.5 (tests) depends on T65.1 (tests the utils).

**Recommended execution order:**
1. T65.1 (utils) + T65.3 (registry) -- parallel, no dependencies between them
2. T65.2 (component) + T65.5 (tests) -- parallel, both depend on T65.1
3. T65.4 (mounting) -- depends on T65.2

**Cross-phase:**
- Phases 65, 66, 67, 68 can all develop in parallel. The only shared file is `echarts-registry.ts`, which requires coordinated merges for chart type registration.
- Phase 65 and 66 both modify `JDEditor.svelte` (different sections), requiring coordinated merge.
