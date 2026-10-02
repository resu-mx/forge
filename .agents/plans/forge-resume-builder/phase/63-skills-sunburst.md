# Phase 63: Skills Sunburst / Pie Chart (Spec J2)

**Status:** Planning
**Date:** 2026-04-03
**Spec:** [2026-04-03-skills-sunburst-chart.md](../refs/specs/2026-04-03-skills-sunburst-chart.md)
**Depends on:** Phase 59 (ECharts Infrastructure -- `EChart.svelte` wrapper, `echarts-registry.ts`, `echarts-theme.ts`)
**Blocks:** None
**Parallelizable with:** Phase 64 (Skills Treemap), and all other phases -- creates new files only, modifies `+page.svelte` (dashboard section addition)

## Goal

Add two interactive chart visualizations to the dashboard: (1) a Skills Sunburst showing every skill in the system organized by category with usage-based sizing and drill-down to single-category pie views, and (2) a Domain-Archetype Breakout nested pie showing how perspectives distribute across domains and archetypes. Both charts use the `EChart.svelte` wrapper from Phase 59 (J1) and the `onChartEvent` prop for click interaction. All aggregation logic is extracted into a pure utility module for testability.

## Non-Goals

- Creating or modifying skills/perspectives data (read-only visualizations)
- Adding new API endpoints (client-side aggregation from existing endpoints)
- Mobile-specific interactions (pinch, swipe)
- Chart export (PNG/SVG download)
- Accessibility (aria labels, screen reader) -- deferred
- Filtering/search controls on the charts -- deferred to a future toolbar spec
- Caching or memoization of aggregated data (premature optimization)
- Source-skill counts (`source_skills` junction not exposed via SDK yet)

## Context

The dashboard currently shows Quick Stats but no chart visualizations of skill or perspective distribution. The ECharts infrastructure from Phase 59 (J1) provides a reusable `EChart.svelte` wrapper component with built-in `onChartEvent` prop support (`Record<string, (params: any) => void>`) and lifecycle management (init, resize, dispose). This phase builds on that foundation to add the first interactive chart visualizations.

Key patterns established in J1 and validated during spec review:
- Use `onMount(() => { loadData() })` NOT `$effect` for data fetching (avoids infinite reactive loops when writing to reactive state inside the effect)
- `$derived` cannot be destructured in Svelte 5
- ECharts cannot resolve CSS custom properties -- use `resolveTokenColor()` utility with `getComputedStyle` at render time
- Dashboard sections must go inside `{:else}` block in `+page.svelte` (after loading/error guards)
- `onChartEvent` prop is built into J1's `EChart.svelte` -- no modifications needed

The SDK's `Bullet` type includes `technologies: string[]` (mapping to `bullet_technologies` junction). Usage counts are computed by counting how many bullets reference each skill name via this array.

## Scope

| Spec section | Covered here? |
|-------------|---------------|
| 1. Skills Sunburst Chart (data, option, drill-down) | Yes |
| 2. Domain-Archetype Breakout Chart (data, option, color) | Yes |
| 3. Component `SkillsSunburst.svelte` | Yes |
| 4. Dashboard integration | Yes |
| 5. EChart.svelte event extension | No -- already exists from J1 |
| 6. Files to create | Yes |
| 7. Files to modify | Yes |
| 8. Testing | Yes |
| 9. Acceptance criteria | Yes |

## Files to Create

| File | Description |
|------|-------------|
| `packages/webui/src/lib/components/charts/skills-chart-utils.ts` | Pure aggregation helpers: `buildSkillsSunburstData`, `buildSunburstOption`, `buildDrillDownOption`, `getCategoryChildren`, `buildDomainArchetypeData`, `buildDomainArchetypeOption`, `resolveTokenColor`, `buildDomainColors`, type exports |
| `packages/webui/src/lib/components/charts/SkillsSunburst.svelte` | Combined component: skills sunburst + domain-archetype breakout |
| `packages/webui/src/lib/components/charts/__tests__/skills-chart-utils.test.ts` | Unit tests for all aggregation and option-building functions |

## Files to Modify

| File | Change |
|------|--------|
| `packages/webui/src/routes/+page.svelte` | Import `SkillsSunburst` and add "Skill & Domain Distribution" section inside `{:else}` block below Quick Stats |

## Fallback Strategies

- **Empty data from API:** If `forge.skills.list()` or `forge.bullets.list()` returns `ok: false`, the corresponding array stays empty (`[]`). The component renders an `EmptyState` when `skills.length === 0` instead of crashing. Perspectives similarly default to `[]`.
- **No technologies on bullets:** If the `Bullet` type's `technologies` array is empty or undefined for all bullets, every skill gets the minimum value of 1. The sunburst still renders -- all wedges are equally sized. This is valid and communicates "no usage data yet."
- **ECharts sunburst type not registered:** If the `echarts-registry.ts` from J1 does not include the sunburst series type, the chart will fail silently. Fallback: add `import 'echarts/charts'` (sunburst + pie) to the registry. The task notes this as a verification step.
- **CSS custom property resolution fails:** `resolveTokenColor()` falls back to hardcoded hex values if `getComputedStyle` returns an empty string. Domain colors always resolve to visible values.
- **Drill-down click on wrong element:** The click handler checks `params.treePathInfo?.length === 2` to ensure only category-level clicks trigger drill-down. Clicking outer-ring skills or empty space does nothing in sunburst mode. In drill-down mode, any click resets to full sunburst.
- **`notMerge` omission:** Without `notMerge={true}`, switching between sunburst and pie options would merge series configurations and produce garbled output. The plan explicitly sets `notMerge={true}` on the EChart component.

---

## Tasks

### T63.1: Write Aggregation and Option Utilities

**File:** `packages/webui/src/lib/components/charts/skills-chart-utils.ts`

Defines all pure functions for data aggregation and ECharts option building. No DOM access, no side effects (except `resolveTokenColor` which reads `getComputedStyle` and must be called at render time).

[CRITICAL] `resolveTokenColor()` calls `getComputedStyle(document.documentElement)` which is only available in the browser. Never call at module scope -- only inside `onMount` or event handlers. `buildDomainColors()` wraps this and returns a `Record<string, string>`.

[CRITICAL] ECharts `graphic` elements use pixel positioning and `left: 'center'` / `top: '50%'` alignment. The center label in the sunburst must use `textAlign: 'center'` or the text will be offset.

```typescript
import type { EChartsOption } from 'echarts'
import type { Skill, Bullet, Perspective } from '@forge/sdk'

// ── Types ────────────────────────────────────────────────────────────────

export interface SunburstChild {
  name: string
  value: number
}

export interface SunburstDataItem {
  name: string
  children: SunburstChild[]
}

export interface PieDataItem {
  name: string
  value: number
}

// ── Color Resolution ─────────────────────────────────────────────────────

/**
 * Resolve a CSS custom property value at render time.
 * ECharts cannot interpret `var(--token)` strings directly.
 * Must be called in browser context (onMount, event handler), never at module scope.
 *
 * [STYLE] Use `resolveTokenColor('--color-text')` for ECharts config values.
 * Direct CSS var strings (`var(--...)`) cannot be used in ECharts options --
 * ECharts expects resolved color strings (#hex, rgb(), etc.), not CSS functions.
 */
export function resolveTokenColor(token: string, fallback: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(token).trim() || fallback
}

/** Build resolved domain color map. Call at render time, not module scope. */
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

// ── Skills Sunburst ──────────────────────────────────────────────────────

/**
 * Build ECharts sunburst data from skills + bullets.
 *
 * Since the SDK's Bullet type includes a `technologies: string[]` field
 * (which maps to bullet_technologies), we count technology references
 * per skill name. Usage = bullet technology count.
 */
export function buildSkillsSunburstData(
  skills: Skill[],
  bullets: Bullet[],
): SunburstDataItem[] {
  // Build a map: skill name (lowercased) -> count of bullets referencing it
  const techCounts = new Map<string, number>()
  for (const bullet of bullets) {
    for (const tech of bullet.technologies) {
      const key = tech.toLowerCase()
      techCounts.set(key, (techCounts.get(key) ?? 0) + 1)
    }
  }

  // Group skills by category
  const categories = new Map<string, Array<{ name: string; value: number }>>()
  for (const skill of skills) {
    const cat = skill.category ?? 'uncategorized'
    if (!categories.has(cat)) categories.set(cat, [])
    const usage = techCounts.get(skill.name.toLowerCase()) ?? 0
    categories.get(cat)!.push({
      name: skill.name,
      value: Math.max(usage, 1),  // minimum 1 so unused skills still appear
    })
  }

  // Convert to ECharts sunburst format
  return Array.from(categories.entries()).map(([category, children]) => ({
    name: category,
    children: children.sort((a, b) => b.value - a.value),
  }))
}

/** Get children for a specific category from sunburst data. */
export function getCategoryChildren(
  categoryName: string,
  data: SunburstDataItem[],
): SunburstChild[] {
  return data.find(d => d.name === categoryName)?.children ?? []
}

/** Build full sunburst ECharts option (all categories visible). */
export function buildSunburstOption(
  data: SunburstDataItem[],
  totalSkills: number,
): EChartsOption {
  return {
    title: {
      text: 'Skills by Category',
      left: 'center',
      top: 10,
    },
    tooltip: {
      trigger: 'item',
      formatter: (params: any) => {
        if (params.treePathInfo?.length === 1) return ''
        const path = params.treePathInfo
          ?.map((p: any) => p.name)
          .filter(Boolean)
          .join(' > ')
        return `${path}<br/>Usage: <strong>${params.value}</strong>`
      },
    },
    series: [
      {
        type: 'sunburst',
        data: data,
        radius: ['20%', '90%'],
        center: ['50%', '55%'],
        sort: 'desc',
        emphasis: {
          focus: 'ancestor',
        },
        levels: [
          {},  // root
          {
            r0: '20%',
            r: '50%',
            itemStyle: { borderWidth: 2, borderColor: '#fff' },
            label: {
              rotate: 'tangential',
              fontSize: 11,
              fontWeight: 600,
            },
          },
          {
            r0: '50%',
            r: '90%',
            itemStyle: { borderWidth: 1, borderColor: '#fff' },
            label: {
              position: 'outside',
              fontSize: 9,
              padding: 2,
              silent: false,
            },
          },
        ],
      },
    ],
    graphic: [
      {
        type: 'text',
        left: 'center',
        top: '50%',
        style: {
          text: `${totalSkills}\nskills`,
          textAlign: 'center',
          fontSize: 18,
          fontWeight: 700,
          fill: '#374151',
        },
      },
    ],
  }
}

/** Build drill-down pie option for a single category. */
export function buildDrillDownOption(
  categoryName: string,
  children: SunburstChild[],
): EChartsOption {
  return {
    title: {
      text: categoryName,
      subtext: 'Click center to go back',
      left: 'center',
      top: 10,
    },
    tooltip: {
      trigger: 'item',
      formatter: '{b}: {c} ({d}%)',
    },
    series: [
      {
        type: 'pie',
        radius: ['25%', '75%'],
        center: ['50%', '55%'],
        data: children.map(c => ({ name: c.name, value: c.value })),
        label: { show: true, fontSize: 10 },
        emphasis: {
          itemStyle: { shadowBlur: 10, shadowColor: 'rgba(0,0,0,0.15)' },
        },
      },
    ],
    graphic: [
      {
        type: 'circle',
        left: 'center',
        top: 'middle',
        shape: { r: 40 },
        style: { fill: 'transparent' },
        onclick: () => {
          // Reset handled by component state, not by graphic element
        },
      },
      {
        type: 'text',
        left: 'center',
        top: '50%',
        style: {
          text: 'Back',
          textAlign: 'center',
          fontSize: 12,
          fill: '#6c63ff',
          cursor: 'pointer',
        },
      },
    ],
  }
}

// ── Domain-Archetype Breakout ────────────────────────────────────────────

/**
 * Build nested pie data from perspectives.
 * Groups by domain (inner ring) then target_archetype (outer ring).
 */
export function buildDomainArchetypeData(
  perspectives: Perspective[],
): { inner: PieDataItem[]; outer: PieDataItem[] } {
  const groups = new Map<string, Map<string, number>>()

  for (const p of perspectives) {
    const domain = p.domain ?? 'unassigned'
    const archetype = p.target_archetype ?? 'unassigned'

    if (!groups.has(domain)) groups.set(domain, new Map())
    const archetypes = groups.get(domain)!
    archetypes.set(archetype, (archetypes.get(archetype) ?? 0) + 1)
  }

  const inner: PieDataItem[] = []
  const outer: PieDataItem[] = []

  for (const [domain, archetypes] of groups) {
    let domainTotal = 0
    for (const [archetype, count] of archetypes) {
      outer.push({ name: `${archetype}`, value: count })
      domainTotal += count
    }
    inner.push({ name: domain, value: domainTotal })
  }

  return { inner, outer }
}

/** Build ECharts option for the domain-archetype nested pie. */
export function buildDomainArchetypeOption(
  inner: PieDataItem[],
  outer: PieDataItem[],
): EChartsOption {
  return {
    title: {
      text: 'Perspectives by Domain & Archetype',
      left: 'center',
      top: 10,
    },
    tooltip: {
      trigger: 'item',
      formatter: '{a} <br/>{b}: {c} ({d}%)',
    },
    legend: {
      type: 'scroll',
      orient: 'vertical',
      right: 10,
      top: 50,
      bottom: 20,
      data: inner.map(d => d.name),
    },
    series: [
      {
        name: 'Domain',
        type: 'pie',
        radius: ['0%', '40%'],
        center: ['40%', '55%'],
        label: {
          position: 'inner',
          fontSize: 11,
          fontWeight: 600,
        },
        data: inner,
        emphasis: {
          itemStyle: { shadowBlur: 10, shadowColor: 'rgba(0,0,0,0.15)' },
        },
      },
      {
        name: 'Archetype',
        type: 'pie',
        radius: ['45%', '75%'],
        center: ['40%', '55%'],
        label: {
          fontSize: 9,
          formatter: '{b}: {c}',
        },
        data: outer,
        emphasis: {
          itemStyle: { shadowBlur: 10, shadowColor: 'rgba(0,0,0,0.15)' },
        },
      },
    ],
  }
}
```

**Acceptance criteria:**
- All functions export correctly from the module.
- `buildSkillsSunburstData` groups skills by category and sizes by bullet technology count.
- Skills with null category go under `"uncategorized"`.
- Unused skills get minimum value of 1.
- `getCategoryChildren` returns `[]` for nonexistent categories.
- `buildDomainArchetypeData` groups perspectives by `domain` (inner) and `target_archetype` (outer).
- Null domain/archetype values go under `"unassigned"`.
- All option builders return valid ECharts option objects with correct `type` values.
- `resolveTokenColor` returns fallback when CSS variable is empty.

**Failure criteria:**
- `buildSkillsSunburstData` crashes on bullets with undefined `technologies` array.
- `buildDomainArchetypeData` double-counts perspectives.
- Option builders produce objects that ECharts rejects (wrong series type, missing required fields).

---

### T63.2: Write SkillsSunburst Component

**File:** `packages/webui/src/lib/components/charts/SkillsSunburst.svelte`

Combined component housing both the skills sunburst and domain-archetype breakout charts. Manages its own data loading, drill-down state, and chart option derivation.

[CRITICAL] Data fetching uses `onMount(() => { loadData() })`, NOT `$effect`. The `loadData` function writes to reactive state (`skills`, `bullets`, `perspectives`). Using `$effect` would cause an infinite loop: $effect reads state -> loadData writes state -> $effect re-runs.

[CRITICAL] `notMerge={true}` is required on the sunburst's `EChart` instance. Without it, transitioning from sunburst to drill-down pie merges series configs, producing garbled output.

[MINOR] Click event forwarding uses J1's built-in `onChartEvent` prop. No modification to `EChart.svelte` is needed.

```svelte
<script lang="ts">
  import { onMount } from 'svelte'
  import { forge } from '$lib/sdk'
  import EChart from './EChart.svelte'
  import { LoadingSpinner, EmptyState } from '$lib/components'
  import type { Skill, Bullet, Perspective } from '@forge/sdk'
  import {
    buildSkillsSunburstData,
    buildSunburstOption,
    buildDrillDownOption,
    getCategoryChildren,
    buildDomainArchetypeData,
    buildDomainArchetypeOption,
  } from './skills-chart-utils'

  // ── State ───────────────────────────────────────────────────────────
  let loading = $state(true)
  let skills = $state<Skill[]>([])
  let bullets = $state<Bullet[]>([])
  let perspectives = $state<Perspective[]>([])

  // Drill-down state for sunburst
  let drillCategory = $state<string | null>(null)

  // Empty data guard
  let hasData = $derived(skills.length > 0)

  // ── Derived chart options ───────────────────────────────────────────
  let sunburstData = $derived(buildSkillsSunburstData(skills, bullets))
  let totalSkills = $derived(skills.length)

  let sunburstOption = $derived(
    drillCategory
      ? buildDrillDownOption(drillCategory, getCategoryChildren(drillCategory, sunburstData))
      : buildSunburstOption(sunburstData, totalSkills)
  )

  let domainData = $derived(buildDomainArchetypeData(perspectives))
  let domainArchetypeOption = $derived(
    buildDomainArchetypeOption(domainData.inner, domainData.outer)
  )

  // ── Data fetch ──────────────────────────────────────────────────────
  // Use onMount instead of $effect to avoid infinite reactive loops
  onMount(() => { loadData() })

  async function loadData() {
    loading = true
    const [s, b, p] = await Promise.all([
      forge.skills.list({ limit: 500 }),
      forge.bullets.list({ limit: 2000 }),
      forge.perspectives.list({ limit: 5000 }),
    ])
    if (s.ok) skills = s.data
    if (b.ok) bullets = b.data
    if (p.ok) perspectives = p.data
    loading = false
  }

  // ── Event handling ──────────────────────────────────────────────────
  function handleSunburstClick(params: any) {
    if (drillCategory) {
      drillCategory = null
      return
    }
    // Category click: treePathInfo length 2 = [root, category]
    if (params.treePathInfo?.length === 2) {
      drillCategory = params.name
    }
  }
</script>

<div class="skills-charts">
  {#if loading}
    <LoadingSpinner size="lg" message="Loading skill data..." />
  {:else if !hasData}
    <EmptyState
      title="No skill data"
      description="Add skills to sources and derive bullets to see distributions."
    />
  {:else}
    <div class="chart-card">
      <EChart
        option={sunburstOption}
        height="450px"
        loading={false}
        notMerge={true}
        onChartEvent={{ click: handleSunburstClick }}
      />
    </div>

    <div class="chart-card">
      <EChart
        option={domainArchetypeOption}
        height="400px"
      />
    </div>
  {/if}
</div>

<style>
  .skills-charts {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  .chart-card {
    background: #fff;
    border: 1px solid #e5e7eb;
    border-radius: 8px;
    padding: 1rem;
  }
</style>
```

**Acceptance criteria:**
- Component renders loading spinner during data fetch.
- After fetch, two chart cards are visible (sunburst + domain-archetype pie).
- Clicking a category wedge in the sunburst switches to drill-down pie view.
- Clicking anywhere in drill-down mode resets to full sunburst.
- `notMerge={true}` ensures clean option swap on drill-down.
- Domain-archetype pie renders inner (domains) and outer (archetypes) rings.
- Component handles API errors gracefully (arrays stay empty, no crash).

**Failure criteria:**
- Infinite reactive loop from `$effect`-based data loading.
- Drill-down fails to swap chart type due to missing `notMerge`.
- Component crashes when all API calls return `ok: false`.

---

### T63.3: Integrate into Dashboard

**File:** `packages/webui/src/routes/+page.svelte`

[CRITICAL] The chart section MUST be placed inside the `{:else}` block in `+page.svelte`. Placing it outside will render the chart while the dashboard's own data is still loading, causing visual jank and potential errors.

Add the import:
```svelte
<script lang="ts">
  // ... existing imports ...
  import SkillsSunburst from '$lib/components/charts/SkillsSunburst.svelte'
</script>
```

Add the section inside the `{:else}` block, after Quick Stats:
```svelte
<!-- After Quick Stats section, inside {:else} block -->
<section class="section">
  <h2 class="section-title">Skill & Domain Distribution</h2>
  <SkillsSunburst />
</section>
```

**Acceptance criteria:**
- "Skill & Domain Distribution" section appears below Quick Stats on the dashboard.
- Section is inside the `{:else}` block (not rendered during loading/error).
- Import is added at the top of the `<script>` block.

**Failure criteria:**
- Section renders outside `{:else}` block and appears during loading.
- Import path is wrong (must use `$lib/components/charts/SkillsSunburst.svelte`).

---

### T63.4: Verify ECharts Registry Includes Sunburst + Pie

**File:** `packages/webui/src/lib/components/charts/echarts-registry.ts` (from Phase 59)

[CRITICAL] The sunburst chart type and pie chart type must be registered in the ECharts registry from J1. If J1 only registered `bar` and `line`, this phase must add `SunburstChart` and `PieChart` to the `use()` call.

Verify the registry includes:
```typescript
import { SunburstChart, PieChart } from 'echarts/charts'
// ... in use() call:
echarts.use([SunburstChart, PieChart, /* ... existing registrations ... */])
```

If not present, add them. This is the only modification to a J1 file -- it extends the registry, does not change existing registrations.

**Acceptance criteria:**
- `SunburstChart` and `PieChart` are registered in `echarts-registry.ts`.
- Existing chart type registrations are unchanged.
- Import compiles without errors.

**Failure criteria:**
- Sunburst/pie charts fail to render because chart types are not registered.
- Removing existing registrations breaks other charts.

---

### T63.5: Write Unit Tests

**File:** `packages/webui/src/lib/components/charts/__tests__/skills-chart-utils.test.ts`

```typescript
import { describe, it, expect } from 'vitest'
import {
  buildSkillsSunburstData,
  buildDomainArchetypeData,
  getCategoryChildren,
  buildSunburstOption,
  buildDrillDownOption,
  buildDomainArchetypeOption,
} from '../skills-chart-utils'

// ── buildSkillsSunburstData ──────────────────────────────────────────

describe('buildSkillsSunburstData', () => {
  it('groups skills by category', () => {
    const skills = [
      { id: '1', name: 'Terraform', category: 'cloud', notes: null },
      { id: '2', name: 'AWS', category: 'cloud', notes: null },
      { id: '3', name: 'Python', category: 'languages', notes: null },
    ]
    const bullets = [
      { id: 'b1', technologies: ['terraform', 'aws'] },
    ] as any[]
    const data = buildSkillsSunburstData(skills as any, bullets)

    expect(data).toHaveLength(2)  // cloud, languages
    const cloudCat = data.find(d => d.name === 'cloud')
    expect(cloudCat?.children).toHaveLength(2)
  })

  it('assigns minimum value of 1 to unused skills', () => {
    const skills = [{ id: '1', name: 'Haskell', category: 'languages', notes: null }]
    const bullets: any[] = []
    const data = buildSkillsSunburstData(skills as any, bullets)

    expect(data[0].children[0].value).toBe(1)
  })

  it('counts bullet technology references correctly', () => {
    const skills = [{ id: '1', name: 'Python', category: 'languages', notes: null }]
    const bullets = [
      { id: 'b1', technologies: ['python'] },
      { id: 'b2', technologies: ['python', 'rust'] },
      { id: 'b3', technologies: ['go'] },
    ] as any[]
    const data = buildSkillsSunburstData(skills as any, bullets)

    const langCat = data.find(d => d.name === 'languages')
    expect(langCat?.children[0].value).toBe(2)  // python appears in b1, b2
  })

  it('handles skills with null category as "uncategorized"', () => {
    const skills = [{ id: '1', name: 'Misc', category: null, notes: null }]
    const data = buildSkillsSunburstData(skills as any, [])
    expect(data[0].name).toBe('uncategorized')
  })

  it('returns empty array for no skills', () => {
    const data = buildSkillsSunburstData([], [])
    expect(data).toHaveLength(0)
  })

  it('case-insensitive technology matching', () => {
    const skills = [{ id: '1', name: 'Python', category: 'languages', notes: null }]
    const bullets = [
      { id: 'b1', technologies: ['PYTHON'] },
      { id: 'b2', technologies: ['python'] },
    ] as any[]
    const data = buildSkillsSunburstData(skills as any, bullets)
    expect(data[0].children[0].value).toBe(2)
  })
})

// ── buildDomainArchetypeData ─────────────────────────────────────────

describe('buildDomainArchetypeData', () => {
  it('groups perspectives by domain and archetype', () => {
    const perspectives = [
      { domain: 'security', target_archetype: 'security_engineer' },
      { domain: 'security', target_archetype: 'security_engineer' },
      { domain: 'security', target_archetype: 'devsecops' },
      { domain: 'cloud', target_archetype: 'cloud_engineer' },
    ] as any[]
    const { inner, outer } = buildDomainArchetypeData(perspectives)

    expect(inner).toHaveLength(2)  // security, cloud
    const secDomain = inner.find(d => d.name === 'security')
    expect(secDomain?.value).toBe(3)

    expect(outer).toHaveLength(3)  // security_engineer, devsecops, cloud_engineer
  })

  it('handles null domain/archetype as "unassigned"', () => {
    const perspectives = [
      { domain: null, target_archetype: null },
    ] as any[]
    const { inner, outer } = buildDomainArchetypeData(perspectives)
    expect(inner[0].name).toBe('unassigned')
    expect(outer[0].name).toBe('unassigned')
  })

  it('returns empty arrays for no perspectives', () => {
    const { inner, outer } = buildDomainArchetypeData([])
    expect(inner).toHaveLength(0)
    expect(outer).toHaveLength(0)
  })

  it('sums domain totals correctly across multiple archetypes', () => {
    const perspectives = [
      { domain: 'cloud', target_archetype: 'sre' },
      { domain: 'cloud', target_archetype: 'devops' },
      { domain: 'cloud', target_archetype: 'sre' },
    ] as any[]
    const { inner } = buildDomainArchetypeData(perspectives)
    expect(inner[0].value).toBe(3)
  })
})

// ── getCategoryChildren ──────────────────────────────────────────────

describe('getCategoryChildren', () => {
  it('returns children for an existing category', () => {
    const data = [
      { name: 'cloud', children: [{ name: 'AWS', value: 5 }] },
    ]
    const children = getCategoryChildren('cloud', data)
    expect(children).toHaveLength(1)
    expect(children[0].name).toBe('AWS')
  })

  it('returns empty array for nonexistent category', () => {
    const children = getCategoryChildren('nonexistent', [])
    expect(children).toHaveLength(0)
  })
})

// ── Option builders ──────────────────────────────────────────────────

describe('buildSunburstOption', () => {
  it('returns option with sunburst series type', () => {
    const data = [{ name: 'cloud', children: [{ name: 'AWS', value: 3 }] }]
    const option = buildSunburstOption(data, 1)
    expect((option.series as any[])[0].type).toBe('sunburst')
  })

  it('includes graphic element with total skill count', () => {
    const option = buildSunburstOption([], 42)
    const graphic = option.graphic as any[]
    expect(graphic[0].style.text).toContain('42')
  })

  it('has title "Skills by Category"', () => {
    const option = buildSunburstOption([], 0)
    expect((option.title as any).text).toBe('Skills by Category')
  })
})

describe('buildDrillDownOption', () => {
  it('returns option with pie series type', () => {
    const children = [{ name: 'AWS', value: 5 }]
    const option = buildDrillDownOption('cloud', children)
    expect((option.series as any[])[0].type).toBe('pie')
  })

  it('uses category name as title', () => {
    const option = buildDrillDownOption('security', [])
    expect((option.title as any).text).toBe('security')
  })

  it('includes "Back" graphic text', () => {
    const option = buildDrillDownOption('cloud', [])
    const graphic = option.graphic as any[]
    const backText = graphic.find(g => g.style?.text === 'Back')
    expect(backText).toBeDefined()
  })
})

describe('buildDomainArchetypeOption', () => {
  it('returns option with two pie series (Domain + Archetype)', () => {
    const inner = [{ name: 'security', value: 3 }]
    const outer = [{ name: 'devsecops', value: 2 }]
    const option = buildDomainArchetypeOption(inner, outer)
    const series = option.series as any[]
    expect(series).toHaveLength(2)
    expect(series[0].name).toBe('Domain')
    expect(series[1].name).toBe('Archetype')
  })

  it('inner series has smaller radius than outer', () => {
    const option = buildDomainArchetypeOption([], [])
    const series = option.series as any[]
    // Inner: ['0%', '40%'], Outer: ['45%', '75%']
    expect(series[0].radius[1]).toBe('40%')
    expect(series[1].radius[0]).toBe('45%')
  })

  it('includes scrollable vertical legend', () => {
    const option = buildDomainArchetypeOption([{ name: 'sec', value: 1 }], [])
    expect((option.legend as any).type).toBe('scroll')
    expect((option.legend as any).orient).toBe('vertical')
  })
})
```

**Acceptance criteria:**
- All `buildSkillsSunburstData` tests pass (6 cases).
- All `buildDomainArchetypeData` tests pass (4 cases).
- All `getCategoryChildren` tests pass (2 cases).
- All `buildSunburstOption` tests pass (3 cases).
- All `buildDrillDownOption` tests pass (3 cases).
- All `buildDomainArchetypeOption` tests pass (3 cases).
- Total: 21 test cases.

**Failure criteria:**
- Tests import functions that do not exist in the utility module.
- Tests use DOM APIs that are not available in the Vitest environment (tests for `resolveTokenColor` are omitted because they require `document`).

---

## Testing

| Test file | Cases | Type |
|-----------|-------|------|
| `packages/webui/src/lib/components/charts/__tests__/skills-chart-utils.test.ts` | 21 | Unit |

**Run command:** `cd packages/webui && npx vitest run src/lib/components/charts/__tests__/skills-chart-utils.test.ts`

Component integration tests (rendering, loading state, drill-down interaction) are deferred to visual QA since they require a full Svelte component testing setup with mocked SDK client.

## Docs

No documentation files created. The spec and this plan serve as the reference. Component usage is self-documenting: import `SkillsSunburst` and render it.

## Parallelization

This phase creates new files and adds a section to `+page.svelte`. It can run in parallel with:

- **Phase 64 (Skills Treemap):** Both phases modify `+page.svelte` to add dashboard sections. The modifications are additive (new sections in the `{:else}` block) and will not conflict as long as they are added in sequence (J2 section first, J3 section below).
- **All other phases** that do not touch `+page.svelte` or the `charts/` directory.

Dependencies:
- **Phase 59 (ECharts Infrastructure):** Must be complete. Provides `EChart.svelte`, `echarts-registry.ts`, and `echarts-theme.ts`. T63.4 may extend the registry with `SunburstChart` + `PieChart` if not already included.

Task ordering within this phase:
- T63.1 (utils) and T63.4 (registry verify) can run in parallel.
- T63.2 (component) depends on T63.1.
- T63.3 (dashboard) depends on T63.2.
- T63.5 (tests) depends on T63.1.
- T63.2, T63.3, and T63.5 can run in parallel after T63.1 completes.
