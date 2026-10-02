# JD Skill Alignment Spider Chart

**Date:** 2026-04-03
**Spec:** E4 (JD Skill Alignment Spider Chart)
**Phase:** TBD (next available)
**Depends on:** E1 (JD Detail Page — `job_description_skills` junction, skill tagging UI), J1 (ECharts Infrastructure — `EChart.svelte` wrapper, `echarts-registry.ts`, `echarts-theme.ts`)
**Blocks:** None

## Overview

Add a radar (spider) chart to the JD detail page that visualizes how well the user's existing skills match a job description's required skills. The chart overlays two polygons — JD requirements vs. the user's skill inventory — making skill gaps immediately visible. An overall match percentage is displayed prominently.

The chart reads JD skill requirements from the `job_description_skills` junction table (Spec E1) and the user's skill inventory from `source_skills` + `bullet_skills` (aggregated by skill category). The comparison is done by category: for each skill category (e.g., `cloud`, `security`, `languages`), the chart shows how many JD-required skills the user has vs. how many the JD requires.

## Non-Goals

- Modifying skills or JD data from the chart (read-only visualization)
- Weighting skills by proficiency level (all skills are treated equally — you have it or you don't)
- Per-skill granularity on the radar (axes are categories, not individual skills; individual skills are in the tooltip)
- Suggesting skills to learn (that is a future gap-analysis feature)
- Comparing across multiple JDs simultaneously (single JD at a time)
- Adding new API endpoints (client-side aggregation from existing endpoints)
- Mobile-specific interactions (pinch, rotate)
- Chart export (PNG/SVG download)
- Accessibility (aria labels, screen reader) — deferred

---

## 1. Chart Design

### 1.1 Radar Chart Structure

- **Axes:** One axis per skill category that appears in either the JD's required skills or the user's skill inventory. Axes are labeled with the category name (e.g., "Cloud", "Security", "Languages").
- **Polygon 1 (JD Requirements):** Outer polygon, semi-transparent fill (e.g., `rgba(108, 99, 255, 0.15)` with solid border `#6c63ff`). Each axis value = count of JD-required skills in that category.
- **Polygon 2 (Your Skills):** Inner polygon, stronger fill (e.g., `rgba(34, 197, 94, 0.25)` with solid border `#22c55e`). Each axis value = count of your skills that match JD-required skills in that category.
- **Gap areas:** Where the JD polygon extends beyond the Your Skills polygon, the purple-tinted area is visually apparent as the gap.
- **Center:** Match score (e.g., "78%") displayed as a `graphic` text element.

### 1.2 Visual Mockup

```
              Cloud (3/4)
               /    \
              /      \
   Security  /   78%  \  DevOps
   (2/2)    /          \ (1/3)
             \          /
              \        /
               \      /
            Languages (3/3)

   ■ JD Requirements   ■ Your Skills
```

### 1.3 Axis Scaling

Each axis uses the JD requirement count as the maximum value. This means:
- If the JD requires 4 cloud skills and you have 3, the cloud axis shows 4 as the outer bound and 3 as your value.
- If a category has zero JD requirements, it does not appear as an axis (only categories present in the JD's skills are shown).
- If you have skills in a category that the JD does not require, those do not appear (this chart is JD-centric, not user-centric).

### 1.4 Minimum Axes

If the JD has skills in fewer than 3 categories, the radar chart degrades to a simple bar comparison instead of a polygon. A radar with 2 axes is a line, not a polygon, so the component falls back to a horizontal stacked bar chart showing "X of Y skills matched" per category.

---

## 2. Data Loading

### 2.1 Data Sources

The component fetches data client-side from existing SDK endpoints:

```typescript
// 1. JD skills (from Spec E1)
const jdSkillsResult = await forge.jobDescriptions.listSkills(jdId)

// 2. All skills (to get category info for JD skills)
const allSkillsResult = await forge.skills.list({ limit: 500 })

// 3. All bullets (to identify user's skills via technologies[])
const bulletsResult = await forge.bullets.list({ limit: 2000 })

// 4. All sources (to identify user's skills via source_skills — if exposed by SDK)
// NOTE: source_skills are not directly listed via SDK. For MVP, user skills
// come from bullet technologies only. Source skills add coverage later.
```

### 2.2 Aggregation Logic

**File:** `packages/webui/src/lib/components/charts/jd-radar-utils.ts`

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
```

---

## 3. ECharts Configuration

### 3.1 Radar Chart Option

```typescript
export function buildRadarOption(alignment: JDSkillAlignment): EChartsOption {
  const categories = alignment.categories
  const maxValues = categories.map(c => c.jdCount)

  return {
    tooltip: {
      trigger: 'item',
      // NOTE: ECharts radar tooltip `params` structure: `params.value` is the data array
      // for the series. Use `params.value[i]` indexed by radar indicator order, not by
      // `categories[i]` from a separate array.
      formatter: (params: any) => {
        const seriesName = params.seriesName
        const values: number[] = params.value
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
```

### 3.2 Fallback Bar Chart (< 3 Categories)

When the JD has skills in fewer than 3 categories, use a horizontal bar chart instead:

```typescript
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

### 3.3 ECharts Registry Update

The radar chart requires `RadarChart` and `RadarComponent` to be registered. Add to `echarts-registry.ts`:

```typescript
import { RadarChart } from 'echarts/charts'
import { BarChart } from 'echarts/charts'
// Register both `RadarChart` from `echarts/charts` AND `RadarComponent` from
// `echarts/components` in `echarts-registry.ts`. RadarComponent provides the
// radar coordinate system (the `radar` option key). Without it, radar charts
// will fail to render.
import { RadarComponent } from 'echarts/components'

// Add to echarts.use([...]):
RadarChart,
RadarComponent,
BarChart,
```

---

## 4. Component

### 4.1 File: `packages/webui/src/lib/components/charts/JDSkillRadar.svelte`

```svelte
<script lang="ts">
  import { forge } from '$lib/sdk'
  import EChart from './EChart.svelte'
  import {
    computeJDSkillAlignment,
    buildRadarOption,
    buildFallbackBarOption,
  } from './jd-radar-utils'
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
      // NOTE: Consider memoizing the `forge.bullets.list({ limit: 2000 })` call or sharing
      // it with the parent component to avoid refetching on every JD selection.
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

### 4.2 Props

| Prop | Type | Required | Description |
|------|------|----------|-------------|
| `jdId` | `string` | Yes | The job description ID to show alignment for. Reactive — changing triggers reload. |

### 4.3 Supplementary Gap List

Below the chart, a textual list of unmatched skills grouped by category provides actionable detail. This is rendered as pill badges in a muted style, making it easy to scan which specific skills are missing.

---

## 5. Placement

### 5.1 JD Detail Page Integration

The `JDSkillRadar` component is mounted on the JD editor panel (`JDEditor.svelte` from Spec E1), below the skill tags section and above the notes textarea:

```
Required Skills:
[Python x] [AWS x] [Kubernetes x]
[Search or add skill... v]  [Extract Skills]

┌─ Skill Alignment ───────────────────────────────┐
│                                                   │
│  [Radar/bar chart]                                │
│                                                   │
│  Skill Gaps:                                      │
│  cloud: [GCP] [Azure]                            │
│  devops: [Ansible] [Jenkins]                     │
│                                                   │
└───────────────────────────────────────────────────┘

Notes:
[textarea]
```

### 5.2 Conditional Rendering

The radar section is only rendered when the JD has at least one tagged skill. The component handles this internally (shows "No required skills tagged" when `totalJDSkills === 0`).

### 5.3 Import

```svelte
<!-- In JDEditor.svelte, after the skill picker section -->
{#if selectedJd}
  <JDSkillRadar jdId={selectedJd.id} />
{/if}
```

---

## 6. Files to Create

| File | Purpose |
|------|---------|
| `packages/webui/src/lib/components/charts/JDSkillRadar.svelte` | Radar chart component with loading, error, empty, and chart states |
| `packages/webui/src/lib/components/charts/jd-radar-utils.ts` | `computeJDSkillAlignment`, `buildRadarOption`, `buildFallbackBarOption` |

## 7. Files to Modify

| File | Change |
|------|--------|
| `packages/webui/src/lib/components/charts/echarts-registry.ts` | Register `RadarChart` and `BarChart` |
| `packages/webui/src/lib/components/jd/JDEditor.svelte` | Mount `JDSkillRadar` below the skill tags section |

---

## 8. Testing

### 8.1 Aggregation Unit Tests

**File:** `packages/webui/src/lib/components/charts/__tests__/jd-radar-utils.test.ts`

- `computeJDSkillAlignment` with no JD skills returns `matchPercentage: 100` and empty categories
- `computeJDSkillAlignment` with JD skills that all match user skills returns `matchPercentage: 100`
- `computeJDSkillAlignment` with JD skills where none match returns `matchPercentage: 0`
- `computeJDSkillAlignment` correctly groups by category and counts matched/gap skills
- `computeJDSkillAlignment` uses case-insensitive matching ("python" matches "Python")
- `computeJDSkillAlignment` with null category groups under `"uncategorized"`
- `computeJDSkillAlignment` sorts categories by `jdCount` descending
- `buildRadarOption` produces valid ECharts radar option with correct indicator count
- `buildRadarOption` includes graphic text element with match percentage
- `buildFallbackBarOption` produces valid ECharts bar option with stacked bars
- `buildFallbackBarOption` includes match percentage text

### 8.2 Component Tests

- `JDSkillRadar` shows loading state while data is being fetched
- `JDSkillRadar` shows "No required skills tagged" when JD has zero skills
- `JDSkillRadar` renders radar chart when JD has 3+ skill categories
- `JDSkillRadar` renders bar chart fallback when JD has fewer than 3 skill categories
- `JDSkillRadar` displays match summary text ("X of Y required skills matched")
- `JDSkillRadar` displays gap list with unmatched skills grouped by category
- `JDSkillRadar` hides gap list when all JD skills are matched
- `JDSkillRadar` reloads when `jdId` prop changes
- `JDSkillRadar` shows error state on API failure
- Chart uses `notMerge={true}` to ensure clean option swap between radar and bar

### 8.3 Registry Tests

- `RadarChart` is registered in `echarts-registry.ts` and can be used in `setOption()`
- `BarChart` is registered in `echarts-registry.ts` and can be used in `setOption()`

---

## 9. Acceptance Criteria

1. `JDSkillRadar` component renders on the JD detail page below the skill tags section
2. Radar chart shows two overlapping polygons: JD requirements (purple) and your skills (green)
3. Each radar axis represents a skill category, labeled with category name and matched/total count
4. Match percentage is displayed prominently in the center of the radar chart
5. Match percentage color is green (>=75%), amber (50-74%), or red (<50%)
6. When JD has fewer than 3 skill categories, a horizontal stacked bar chart is used instead of radar
7. Gap list below the chart shows unmatched skills grouped by category as pill badges
8. Gap list is hidden when all JD skills are matched (100% match)
9. Component shows empty state when no skills are tagged on the JD
10. Component shows loading state while fetching data
11. Component reloads reactively when `jdId` changes
12. `RadarChart` and `BarChart` are registered in `echarts-registry.ts`
13. User skills are derived from bullet technologies (case-insensitive name matching)
14. All aggregation unit tests pass (11 cases)
15. API errors are handled gracefully with an error message (no crash)

---

## 10. Future Enhancements

- **Source skills integration:** Once `source_skills` are exposed via the SDK (or an aggregate endpoint exists), include them in the user's skill inventory for higher match rates.
- **Proficiency weighting:** If skills gain a proficiency/experience level field, the radar could weight matches by proficiency instead of treating all skills as binary.
- **Per-skill drill-down:** Clicking a radar axis could show a detail panel listing exactly which skills match and which are gaps, with links to relevant bullets/sources.
- **Multi-JD comparison:** Overlaying multiple JDs on the same radar to compare requirement profiles.
- **Aggregate skills endpoint:** A dedicated `GET /api/skills/user-inventory` endpoint returning the user's aggregated skills with counts would eliminate fetching all bullets client-side.
