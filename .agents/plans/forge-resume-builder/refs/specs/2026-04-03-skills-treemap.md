# Skills / Bullets / Domains Treemap

**Date:** 2026-04-03
**Spec:** J3 (Skills Treemap)
**Phase:** TBD (next available)
**Depends on:** J1 (ECharts Infrastructure — `EChart.svelte` wrapper, `echarts-registry.ts`, `echarts-theme.ts`)
**Blocks:** None

## Overview

Add three treemap visualizations to the dashboard (`/`), positioned below the sunburst/pie charts from J2 (or standalone if J2 has not landed):

1. **Skills Treemap** — skills sized by usage count, grouped and colored by category
2. **Bullets Treemap** — bullets sized by perspective count, grouped by primary source
3. **Domains Treemap** — domains sized by perspective/bullet count, colored by domain

Treemaps excel at showing relative proportions in large flat datasets. Unlike the sunburst (which emphasizes hierarchy), treemaps use rectangular packing to make it immediately visible which items dominate.

All three charts use the `EChart.svelte` wrapper from J1, which includes `onChartEvent` from the start. No modification to `EChart.svelte` is required.

## Non-Goals

- Editing data from the treemap (read-only visualization)
- Adding new API endpoints (client-side aggregation from existing endpoints)
- Mobile-specific interactions (pinch zoom, pan)
- Chart export (PNG/SVG download)
- Accessibility (aria labels, screen reader) — deferred
- Filtering or search controls on treemaps — deferred to a future toolbar spec
- Real-time updates (treemaps refresh on page load or manual reload)

---

## 1. Skills Treemap

### 1.1 Description

A rectangular treemap where each rectangle represents a skill. Rectangles are:

- **Sized** by usage count — the number of distinct bullets whose `technologies[]` array includes this skill (via `bullet_technologies` junction)
- **Grouped** by `skill.category` — categories form the top-level rectangles; individual skills are nested within
- **Colored** by category — each category gets a distinct color from the chart palette

This answers: "Which skills appear most frequently in my bullet points, and how do categories compare?"

### 1.2 Data Loading

Uses existing SDK endpoints. Can share data with J2's `SkillsSunburst` component if both are on the same page, but this spec defines its own standalone loader for independence:

```typescript
async function loadSkillsTreemapData(): Promise<{
  skills: Skill[]
  bullets: Bullet[]
}> {
  const [s, b] = await Promise.all([
    forge.skills.list({ limit: 500 }),
    forge.bullets.list({ limit: 2000 }),
  ])
  return {
    skills: s.ok ? s.data : [],
    bullets: b.ok ? b.data : [],
  }
}
```

### 1.3 Aggregation

**File:** `packages/webui/src/lib/components/charts/treemap-utils.ts`

```typescript
export interface TreemapNode {
  name: string
  value: number
  children?: TreemapNode[]
  itemStyle?: { color?: string }
  perspectiveCount?: number
  bulletCount?: number
}

/**
 * Build treemap data: skills grouped by category, sized by bullet technology count.
 */
export function buildSkillsTreemapData(
  skills: Skill[],
  bullets: Bullet[],
): TreemapNode[] {
  // Count technology references across all bullets
  const techCounts = new Map<string, number>()
  for (const bullet of bullets) {
    for (const tech of bullet.technologies) {
      const key = tech.toLowerCase()
      techCounts.set(key, (techCounts.get(key) ?? 0) + 1)
    }
  }

  // Group skills by category
  const categories = new Map<string, TreemapNode[]>()
  for (const skill of skills) {
    const cat = skill.category ?? 'uncategorized'
    if (!categories.has(cat)) categories.set(cat, [])

    const usage = techCounts.get(skill.name.toLowerCase()) ?? 0
    categories.get(cat)!.push({
      name: skill.name,
      value: Math.max(usage, 1),  // minimum 1 so unused skills appear
    })
  }

  // Convert to treemap hierarchy
  return Array.from(categories.entries())
    .map(([category, children]) => ({
      name: category,
      value: children.reduce((sum, c) => sum + c.value, 0),
      children: children.sort((a, b) => b.value - a.value),
    }))
    .sort((a, b) => b.value - a.value)
}
```

### 1.4 ECharts Option

```typescript
export function buildSkillsTreemapOption(data: TreemapNode[]): EChartsOption {
  return {
    title: {
      text: 'Skills Usage Treemap',
      subtext: 'Sized by bullet reference count, grouped by category',
      left: 'center',
      top: 10,
    },
    tooltip: {
      formatter: (params: any) => {
        const path = params.treePathInfo
          ?.map((p: any) => p.name)
          .filter(Boolean)
          .join(' > ')
        return `${path}<br/>References: <strong>${params.value}</strong>`
      },
    },
    series: [
      {
        type: 'treemap',
        data: data,
        width: '95%',
        height: '80%',
        top: 60,
        roam: false,
        nodeClick: 'zoomToNode',
        breadcrumb: {
          show: true,
          top: 35,
          left: 'center',
          itemStyle: {
            color: '#f9fafb',
            borderColor: '#e5e7eb',
            textStyle: { color: '#374151', fontSize: 11 },
          },
        },
        levels: [
          {
            // Category level
            itemStyle: {
              borderWidth: 2,
              borderColor: '#fff',
              gapWidth: 2,
            },
            upperLabel: {
              show: true,
              height: 20,
              fontSize: 11,
              fontWeight: 600,
              color: '#fff',
              textShadowBlur: 2,
              textShadowColor: 'rgba(0,0,0,0.3)',
            },
            colorSaturation: [0.4, 0.7],
          },
          {
            // Skill level
            itemStyle: {
              borderWidth: 1,
              borderColor: '#fff',
              gapWidth: 1,
            },
            label: {
              show: true,
              fontSize: 10,
              formatter: '{b}',
            },
            colorSaturation: [0.3, 0.6],
            colorMappingBy: 'value',
          },
        ],
        // Use chart palette for category colors
        colorMappingBy: 'index',
      },
    ],
  }
}
```

The `nodeClick: 'zoomToNode'` enables ECharts' built-in treemap drill-down — clicking a category zooms in to show only that category's skills. The breadcrumb trail at the top allows navigating back.

---

## 2. Bullets Treemap

### 2.1 Description

A rectangular treemap where each rectangle represents a bullet. Rectangles are:

- **Sized** by perspective count — the number of perspectives derived from each bullet
- **Grouped** by primary source — bullets sharing the same primary source are nested under that source
- **Colored** by source

This answers: "Which bullets have generated the most perspectives, and which sources are the most productive?"

### 2.2 Data Loading

```typescript
async function loadBulletsTreemapData(): Promise<{
  bullets: BulletWithRelations[]
  sources: Source[]
}> {
  const [b, s] = await Promise.all([
    forge.bullets.list({ limit: 2000 }),
    forge.sources.list({ limit: 500 }),
  ])
  return {
    bullets: b.ok ? b.data : [],
    sources: s.ok ? s.data : [],
  }
}
```

The SDK's `Bullet` type includes `sources: Array<{ id: string; title: string; is_primary: boolean }>`. For perspective count, the SDK's `BulletWithRelations` type includes `perspective_count: number`.

If the `bullets.list()` endpoint does not return `perspective_count`, the component must also fetch perspectives and count client-side:

```typescript
// Fallback: count perspectives per bullet client-side
const perspectivesResult = await forge.perspectives.list({ limit: 5000 })
const perspsByBullet = new Map<string, number>()
if (perspectivesResult.ok) {
  for (const p of perspectivesResult.data) {
    perspsByBullet.set(p.bullet_id, (perspsByBullet.get(p.bullet_id) ?? 0) + 1)
  }
}
```

### 2.3 Aggregation

```typescript
/**
 * Build treemap data: bullets grouped by primary source, sized by perspective count.
 */
export function buildBulletsTreemapData(
  bullets: Bullet[],
  perspectiveCounts: Map<string, number>,
): TreemapNode[] {
  // Group bullets by primary source
  const sourceGroups = new Map<string, { title: string; bullets: TreemapNode[] }>()

  for (const bullet of bullets) {
    const primarySource = bullet.sources?.find(s => s.is_primary)
    const sourceTitle = primarySource?.title ?? 'No Source'
    const sourceId = primarySource?.id ?? '_no_source'

    if (!sourceGroups.has(sourceId)) {
      sourceGroups.set(sourceId, { title: sourceTitle, bullets: [] })
    }

    const perspCount = perspectiveCounts.get(bullet.id) ?? 0
    sourceGroups.get(sourceId)!.bullets.push({
      name: bullet.content.length > 60
        ? bullet.content.slice(0, 57) + '...'
        : bullet.content,
      value: Math.max(perspCount, 1),  // minimum 1 for bullets with no perspectives
    })
  }

  return Array.from(sourceGroups.values())
    .map(({ title, bullets }) => ({
      name: title,
      value: bullets.reduce((sum, b) => sum + b.value, 0),
      children: bullets.sort((a, b) => b.value - a.value),
    }))
    .sort((a, b) => b.value - a.value)
}
```

### 2.4 ECharts Option

```typescript
export function buildBulletsTreemapOption(data: TreemapNode[]): EChartsOption {
  return {
    title: {
      text: 'Bullets by Source',
      subtext: 'Sized by perspective count, grouped by primary source',
      left: 'center',
      top: 10,
    },
    tooltip: {
      formatter: (params: any) => {
        const name = params.data?.name ?? params.name
        const truncated = name.length > 100 ? name.slice(0, 97) + '...' : name
        return `${truncated}<br/>Perspectives: <strong>${params.value}</strong>`
      },
    },
    series: [
      {
        type: 'treemap',
        data: data,
        width: '95%',
        height: '80%',
        top: 60,
        roam: false,
        nodeClick: 'zoomToNode',
        breadcrumb: {
          show: true,
          top: 35,
          left: 'center',
        },
        levels: [
          {
            // Source level
            itemStyle: {
              borderWidth: 2,
              borderColor: '#fff',
              gapWidth: 2,
            },
            upperLabel: {
              show: true,
              height: 22,
              fontSize: 11,
              fontWeight: 600,
              color: '#fff',
              textShadowBlur: 2,
              textShadowColor: 'rgba(0,0,0,0.3)',
            },
            colorSaturation: [0.4, 0.7],
          },
          {
            // Bullet level
            itemStyle: {
              borderWidth: 1,
              borderColor: '#fff',
              gapWidth: 1,
            },
            label: {
              show: true,
              fontSize: 9,
              formatter: (params: any) => {
                const name = params.data?.name ?? ''
                return name.length > 30 ? name.slice(0, 27) + '...' : name
              },
            },
            colorSaturation: [0.3, 0.6],
            colorMappingBy: 'value',
          },
        ],
        colorMappingBy: 'index',
      },
    ],
  }
}
```

---

## 3. Domains Treemap

### 3.1 Description

A single-level treemap where each rectangle represents a domain. Rectangles are:

- **Sized** by perspective count associated with the domain (the `domain` field exists on `Perspective`, not `Bullet`)
- **Colored** by domain (consistent with the J2 domain-archetype pie chart colors)

This is a simpler, at-a-glance chart showing domain balance. Unlike J2's nested pie which shows archetype breakdown within domains, this treemap emphasizes the raw proportions.

### 3.2 Aggregation

```typescript
/**
 * Build treemap data: domains sized by perspective count.
 *
 * The `domain` field exists on `Perspective`, not `Bullet`.
 * This function accepts only perspectives and groups by `p.domain`.
 */
export function buildDomainsTreemapData(
  perspectives: Perspective[],
): TreemapNode[] {
  const domainCounts = new Map<string, { perspectiveCount: number }>()

  for (const p of perspectives) {
    const domain = p.domain ?? 'unassigned'
    if (!domainCounts.has(domain)) {
      domainCounts.set(domain, { perspectiveCount: 0 })
    }
    domainCounts.get(domain)!.perspectiveCount++
  }

  return Array.from(domainCounts.entries())
    .map(([domain, counts]) => ({
      name: domain,
      value: counts.perspectiveCount,
      // Attach count for tooltip
      perspectiveCount: counts.perspectiveCount,
    }))
    .sort((a, b) => b.value - a.value) as TreemapNode[]
}
```

### 3.3 ECharts Option

```typescript
export function buildDomainsTreemapOption(data: TreemapNode[]): EChartsOption {
  return {
    title: {
      text: 'Domain Coverage',
      subtext: 'Sized by perspective count',
      left: 'center',
      top: 10,
    },
    tooltip: {
      formatter: (params: any) => {
        const d = params.data
        const pCount = d?.perspectiveCount ?? 0
        return `<strong>${params.name}</strong><br/>
                Perspectives: ${pCount}`
      },
    },
    series: [
      {
        type: 'treemap',
        data: data,
        width: '95%',
        height: '80%',
        top: 60,
        roam: false,
        nodeClick: false,  // single level — no drill-down
        breadcrumb: { show: false },
        label: {
          show: true,
          fontSize: 14,
          fontWeight: 600,
          formatter: '{b}\n{c}',
          color: '#fff',
          textShadowBlur: 2,
          textShadowColor: 'rgba(0,0,0,0.3)',
        },
        itemStyle: {
          borderWidth: 2,
          borderColor: '#fff',
          gapWidth: 2,
        },
        levels: [
          {
            colorSaturation: [0.4, 0.7],
          },
        ],
        colorMappingBy: 'index',
      },
    ],
  }
}
```

---

## 4. Component Architecture

### 4.1 Single Configurable Component vs. Three Separate Components

**Decision: Three small components** — `SkillsTreemap.svelte`, `BulletsTreemap.svelte`, `DomainsTreemap.svelte`.

Rationale: Each treemap has different data sources, different fetch patterns, and different aggregation logic. A single configurable `Treemap.svelte` would need a complex discriminated-union config prop that makes it harder to understand and test. Three focused components are simpler.

However, all three share the same `TreemapNode` type and utility functions from `treemap-utils.ts`.

### 4.2 Component: `SkillsTreemap.svelte`

**File:** `packages/webui/src/lib/components/charts/SkillsTreemap.svelte`

```svelte
<script lang="ts">
  import { onMount } from 'svelte'
  import { forge } from '$lib/sdk'
  import EChart from './EChart.svelte'
  import { LoadingSpinner, EmptyState } from '$lib/components'
  import { buildSkillsTreemapData, buildSkillsTreemapOption } from './treemap-utils'
  import type { Skill, Bullet } from '@forge/sdk'

  let loading = $state(true)
  let skills = $state<Skill[]>([])
  let bullets = $state<Bullet[]>([])

  let treemapData = $derived(buildSkillsTreemapData(skills, bullets))
  let option = $derived(buildSkillsTreemapOption(treemapData))
  let hasData = $derived(skills.length > 0)

  // Use onMount instead of $effect to avoid infinite reactive loops:
  // loadData() writes to reactive state that $effect would re-track.
  onMount(() => { loadData() })

  async function loadData() {
    loading = true
    const [s, b] = await Promise.all([
      forge.skills.list({ limit: 500 }),
      forge.bullets.list({ limit: 2000 }),
    ])
    if (s.ok) skills = s.data
    if (b.ok) bullets = b.data
    loading = false
  }
</script>

{#if loading}
  <LoadingSpinner message="Loading skills data..." />
{:else if !hasData}
  <EmptyState message="No skills data available for treemap." />
{:else}
  <div class="chart-card">
    <EChart {option} height="400px" notMerge={true} />
  </div>
{/if}

<style>
  .chart-card {
    background: #fff;
    border: 1px solid #e5e7eb;
    border-radius: 8px;
    padding: 1rem;
  }
</style>
```

### 4.3 Component: `BulletsTreemap.svelte`

**File:** `packages/webui/src/lib/components/charts/BulletsTreemap.svelte`

```svelte
<script lang="ts">
  import { onMount } from 'svelte'
  import { forge } from '$lib/sdk'
  import EChart from './EChart.svelte'
  import { LoadingSpinner, EmptyState } from '$lib/components'
  import { buildBulletsTreemapData, buildBulletsTreemapOption } from './treemap-utils'
  import type { Bullet, Perspective } from '@forge/sdk'

  let loading = $state(true)
  let bullets = $state<Bullet[]>([])
  let perspectiveCounts = $state(new Map<string, number>())

  let treemapData = $derived(buildBulletsTreemapData(bullets, perspectiveCounts))
  let option = $derived(buildBulletsTreemapOption(treemapData))
  let hasData = $derived(bullets.length > 0)

  // Use onMount instead of $effect to avoid infinite reactive loops:
  // loadData() writes to reactive state that $effect would re-track.
  onMount(() => { loadData() })

  async function loadData() {
    loading = true
    const [b, p] = await Promise.all([
      forge.bullets.list({ limit: 2000 }),
      forge.perspectives.list({ limit: 5000 }),
    ])

    if (b.ok) bullets = b.data

    // Build perspective count map
    if (p.ok) {
      const counts = new Map<string, number>()
      for (const persp of p.data) {
        counts.set(persp.bullet_id, (counts.get(persp.bullet_id) ?? 0) + 1)
      }
      perspectiveCounts = counts
    }

    loading = false
  }
</script>

{#if loading}
  <LoadingSpinner message="Loading bullet data..." />
{:else if !hasData}
  <EmptyState message="No bullet data available for treemap." />
{:else}
  <div class="chart-card">
    <EChart {option} height="450px" notMerge={true} />
  </div>
{/if}

<style>
  .chart-card {
    background: #fff;
    border: 1px solid #e5e7eb;
    border-radius: 8px;
    padding: 1rem;
  }
</style>
```

### 4.4 Component: `DomainsTreemap.svelte`

**File:** `packages/webui/src/lib/components/charts/DomainsTreemap.svelte`

```svelte
<script lang="ts">
  import { onMount } from 'svelte'
  import { forge } from '$lib/sdk'
  import EChart from './EChart.svelte'
  import { LoadingSpinner, EmptyState } from '$lib/components'
  import { buildDomainsTreemapData, buildDomainsTreemapOption } from './treemap-utils'
  import type { Perspective } from '@forge/sdk'

  let loading = $state(true)
  let perspectives = $state<Perspective[]>([])

  let treemapData = $derived(buildDomainsTreemapData(perspectives))
  let option = $derived(buildDomainsTreemapOption(treemapData))
  let hasData = $derived(perspectives.length > 0)

  // Use onMount instead of $effect to avoid infinite reactive loops:
  // loadData() writes to reactive state that $effect would re-track.
  onMount(() => { loadData() })

  async function loadData() {
    loading = true
    const p = await forge.perspectives.list({ limit: 5000 })
    if (p.ok) perspectives = p.data
    loading = false
  }
</script>

{#if loading}
  <LoadingSpinner message="Loading domain data..." />
{:else if !hasData}
  <EmptyState message="No domain data available for treemap." />
{:else}
  <div class="chart-card">
    <EChart {option} height="350px" notMerge={true} />
  </div>
{/if}

<style>
  .chart-card {
    background: #fff;
    border: 1px solid #e5e7eb;
    border-radius: 8px;
    padding: 1rem;
  }
</style>
```

---

## 5. Dashboard Integration

### 5.1 Placement

The three treemap components are added to the dashboard page (`packages/webui/src/routes/+page.svelte`) as a new section. If J2 has already landed, this section appears below the "Skill & Domain Distribution" section. Otherwise, it appears below "Quick Stats".

**Important:** This section must be placed inside the `{:else}` block in `+page.svelte` (after the loading/error guards). Placing it outside will render the chart while data is still loading.

```svelte
<!-- After Quick Stats (or after J2 charts if present) -->
<section class="section">
  <h2 class="section-title">Treemap Views</h2>
  <div class="treemap-grid">
    <SkillsTreemap />
    <BulletsTreemap />
    <DomainsTreemap />
  </div>
</section>
```

### 5.2 Layout

The treemaps are stacked vertically in a single column. Each component manages its own card styling. The grid wrapper provides consistent spacing:

```css
.treemap-grid {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}
```

### 5.3 Import

```svelte
<script lang="ts">
  // ... existing imports ...
  import SkillsTreemap from '$lib/components/charts/SkillsTreemap.svelte'
  import BulletsTreemap from '$lib/components/charts/BulletsTreemap.svelte'
  import DomainsTreemap from '$lib/components/charts/DomainsTreemap.svelte'
</script>
```

### 5.4 Data Sharing Optimization (Optional)

If both J2 and J3 are on the dashboard, they will independently fetch skills, bullets, and perspectives. To avoid duplicate API calls, a future optimization can lift data fetching to the dashboard page and pass data as props. This is NOT part of this spec — each component is self-contained for simplicity and independence.

---

## 6. Files to Create

| File | Purpose |
|------|---------|
| `packages/webui/src/lib/components/charts/SkillsTreemap.svelte` | Skills treemap: skills sized by bullet technology count, grouped by category |
| `packages/webui/src/lib/components/charts/BulletsTreemap.svelte` | Bullets treemap: bullets sized by perspective count, grouped by primary source |
| `packages/webui/src/lib/components/charts/DomainsTreemap.svelte` | Domains treemap: domains sized by perspective + bullet count |
| `packages/webui/src/lib/components/charts/treemap-utils.ts` | Shared aggregation helpers: `buildSkillsTreemapData`, `buildBulletsTreemapData`, `buildDomainsTreemapData`, `buildSkillsTreemapOption`, `buildBulletsTreemapOption`, `buildDomainsTreemapOption`, `TreemapNode` type |

## 7. Files to Modify

| File | Change |
|------|--------|
| `packages/webui/src/routes/+page.svelte` | Import three treemap components and add "Treemap Views" section |

---

## 8. Testing

### 8.1 Aggregation Unit Tests

**File:** `packages/webui/src/lib/components/charts/__tests__/treemap-utils.test.ts`

```typescript
import { describe, it, expect } from 'vitest'
import {
  buildSkillsTreemapData,
  buildBulletsTreemapData,
  buildDomainsTreemapData,
} from '../treemap-utils'

// ── Skills Treemap ────────────────────────────────────────────────────

describe('buildSkillsTreemapData', () => {
  it('groups skills by category', () => {
    const skills = [
      { id: '1', name: 'Terraform', category: 'cloud', notes: null },
      { id: '2', name: 'AWS', category: 'cloud', notes: null },
      { id: '3', name: 'Python', category: 'languages', notes: null },
    ]
    const bullets = [
      { id: 'b1', technologies: ['terraform', 'aws'] },
    ] as any[]
    const data = buildSkillsTreemapData(skills as any, bullets)

    expect(data).toHaveLength(2)  // cloud, languages
    const cloud = data.find(d => d.name === 'cloud')
    expect(cloud?.children).toHaveLength(2)
  })

  it('sizes skills by bullet technology reference count', () => {
    const skills = [{ id: '1', name: 'Python', category: 'languages', notes: null }]
    const bullets = [
      { id: 'b1', technologies: ['python'] },
      { id: 'b2', technologies: ['python'] },
      { id: 'b3', technologies: ['go'] },
    ] as any[]
    const data = buildSkillsTreemapData(skills as any, bullets)

    const lang = data.find(d => d.name === 'languages')
    expect(lang?.children?.[0].value).toBe(2)
  })

  it('assigns minimum value of 1 to skills with no references', () => {
    const skills = [{ id: '1', name: 'Haskell', category: 'languages', notes: null }]
    const data = buildSkillsTreemapData(skills as any, [])
    expect(data[0].children?.[0].value).toBe(1)
  })

  it('puts null-category skills under "uncategorized"', () => {
    const skills = [{ id: '1', name: 'Misc', category: null, notes: null }]
    const data = buildSkillsTreemapData(skills as any, [])
    expect(data[0].name).toBe('uncategorized')
  })

  it('sorts categories and skills by value descending', () => {
    const skills = [
      { id: '1', name: 'A', category: 'small', notes: null },
      { id: '2', name: 'B', category: 'big', notes: null },
    ]
    const bullets = [
      { id: 'b1', technologies: ['b'] },
      { id: 'b2', technologies: ['b'] },
      { id: 'b3', technologies: ['b'] },
    ] as any[]
    const data = buildSkillsTreemapData(skills as any, bullets)
    expect(data[0].name).toBe('big')  // 3 refs > 1 ref
  })

  it('returns empty array for no skills', () => {
    const data = buildSkillsTreemapData([], [])
    expect(data).toHaveLength(0)
  })
})

// ── Bullets Treemap ───────────────────────────────────────────────────

describe('buildBulletsTreemapData', () => {
  it('groups bullets by primary source', () => {
    const bullets = [
      { id: 'b1', content: 'Built CI/CD pipeline', sources: [{ id: 's1', title: 'DevOps Role', is_primary: true }] },
      { id: 'b2', content: 'Deployed Kubernetes', sources: [{ id: 's1', title: 'DevOps Role', is_primary: true }] },
      { id: 'b3', content: 'Wrote Python scripts', sources: [{ id: 's2', title: 'SWE Role', is_primary: true }] },
    ] as any[]
    const counts = new Map([['b1', 3], ['b2', 1], ['b3', 5]])

    const data = buildBulletsTreemapData(bullets, counts)
    expect(data).toHaveLength(2)  // 2 sources

    const devops = data.find(d => d.name === 'DevOps Role')
    expect(devops?.children).toHaveLength(2)
  })

  it('sizes bullets by perspective count', () => {
    const bullets = [
      { id: 'b1', content: 'Big bullet', sources: [{ id: 's1', title: 'Src', is_primary: true }] },
    ] as any[]
    const counts = new Map([['b1', 7]])

    const data = buildBulletsTreemapData(bullets, counts)
    expect(data[0].children?.[0].value).toBe(7)
  })

  it('assigns minimum value of 1 to bullets with no perspectives', () => {
    const bullets = [
      { id: 'b1', content: 'Orphan bullet', sources: [{ id: 's1', title: 'Src', is_primary: true }] },
    ] as any[]
    const data = buildBulletsTreemapData(bullets, new Map())
    expect(data[0].children?.[0].value).toBe(1)
  })

  it('truncates long bullet content to 60 chars', () => {
    const longContent = 'A'.repeat(100)
    const bullets = [
      { id: 'b1', content: longContent, sources: [{ id: 's1', title: 'Src', is_primary: true }] },
    ] as any[]
    const data = buildBulletsTreemapData(bullets, new Map())
    expect(data[0].children?.[0].name.length).toBeLessThanOrEqual(60)
    expect(data[0].children?.[0].name).toContain('...')
  })

  it('handles bullets with no primary source', () => {
    const bullets = [
      { id: 'b1', content: 'No source', sources: [] },
    ] as any[]
    const data = buildBulletsTreemapData(bullets, new Map())
    expect(data[0].name).toBe('No Source')
  })
})

// ── Domains Treemap ───────────────────────────────────────────────────

describe('buildDomainsTreemapData', () => {
  it('counts perspectives per domain', () => {
    const perspectives = [
      { domain: 'security' },
      { domain: 'security' },
      { domain: 'cloud' },
    ] as any[]
    const data = buildDomainsTreemapData(perspectives)

    const security = data.find(d => d.name === 'security')
    expect(security?.value).toBe(2)

    const cloud = data.find(d => d.name === 'cloud')
    expect(cloud?.value).toBe(1)
  })

  it('handles null domain as "unassigned"', () => {
    const perspectives = [{ domain: null }] as any[]
    const data = buildDomainsTreemapData(perspectives)
    expect(data[0].name).toBe('unassigned')
  })

  it('sorts domains by total count descending', () => {
    const perspectives = [
      { domain: 'small' },
      { domain: 'big' },
      { domain: 'big' },
      { domain: 'big' },
    ] as any[]
    const data = buildDomainsTreemapData(perspectives)
    expect(data[0].name).toBe('big')
  })

  it('returns empty array for no data', () => {
    const data = buildDomainsTreemapData([])
    expect(data).toHaveLength(0)
  })

  it('attaches perspectiveCount for tooltip', () => {
    const perspectives = [{ domain: 'sec' }, { domain: 'sec' }] as any[]
    const data = buildDomainsTreemapData(perspectives)
    expect((data[0] as any).perspectiveCount).toBe(2)
  })
})
```

### 8.2 Component Integration Tests

- Each treemap component renders a `LoadingSpinner` during fetch
- After fetch completes, a chart card is visible
- With empty data, an `EmptyState` is shown instead of the chart
- Skills treemap: clicking a category zooms into it (ECharts built-in `nodeClick: 'zoomToNode'`); breadcrumb appears for navigation back
- Bullets treemap: clicking a source group zooms into it
- Domains treemap: clicking does nothing (single-level, `nodeClick: false`)

### 8.3 Option Builder Tests

- `buildSkillsTreemapOption` returns valid ECharts option with `type: 'treemap'`, title, tooltip, series, and levels
- `buildBulletsTreemapOption` returns valid ECharts option with `type: 'treemap'`, appropriate label truncation formatter
- `buildDomainsTreemapOption` returns valid ECharts option with `nodeClick: false` and custom tooltip showing perspective + bullet counts

---

## 9. Acceptance Criteria

### Skills Treemap
- [ ] Renders a treemap with top-level categories and nested skill rectangles
- [ ] Rectangles are sized by bullet technology reference count
- [ ] Skills with no references appear with minimum size 1
- [ ] Null-category skills are grouped under "uncategorized"
- [ ] Clicking a category zooms into it (ECharts built-in drill-down)
- [ ] Breadcrumb trail appears and allows navigating back to top level
- [ ] Tooltip shows skill path and reference count
- [ ] Categories and skills are sorted by value descending

### Bullets Treemap
- [ ] Renders a treemap with top-level source groups and nested bullet rectangles
- [ ] Rectangles are sized by perspective count
- [ ] Bullets with no perspectives appear with minimum size 1
- [ ] Bullets are grouped by primary source; bullets without a primary source appear under "No Source"
- [ ] Long bullet content is truncated to 60 characters with ellipsis
- [ ] Clicking a source group zooms into it
- [ ] Tooltip shows bullet content preview and perspective count

### Domains Treemap
- [ ] Renders a single-level treemap with one rectangle per domain
- [ ] Rectangles are sized by perspective count (the `domain` field exists on `Perspective`, not `Bullet`)
- [ ] Null domain values are grouped under "unassigned"
- [ ] Tooltip shows perspective count
- [ ] No drill-down (`nodeClick: false`)
- [ ] Breadcrumb is hidden
- [ ] Domain labels are visible inside rectangles with count

### Dashboard integration
- [ ] "Treemap Views" section appears on dashboard below existing sections
- [ ] Three treemap charts are stacked vertically
- [ ] Each chart independently loads its data and shows loading state
- [ ] Each chart handles empty data with `EmptyState` component
- [ ] Error states from API calls are handled gracefully (component does not crash)

### Shared utilities
- [ ] `TreemapNode` type is exported from `treemap-utils.ts`
- [ ] All aggregation functions are pure (no side effects, no DOM access)
- [ ] All option builder functions return valid ECharts option objects

### Testing
- [ ] `buildSkillsTreemapData` tests pass (6 cases)
- [ ] `buildBulletsTreemapData` tests pass (5 cases)
- [ ] `buildDomainsTreemapData` tests pass (5 cases)

---

## 10. Future Enhancements

- **Data sharing:** If J2 and J3 are both on the dashboard, lift data fetching to the dashboard page and pass skills/bullets/perspectives as props to avoid duplicate API calls.
- **Aggregate API endpoints:** Dedicated endpoints like `GET /api/skills/usage-stats` and `GET /api/bullets/perspective-counts` would eliminate the need to fetch and count client-side.
- **Click-through navigation:** Clicking a skill rectangle could navigate to the skills page filtered by that skill. Clicking a bullet rectangle could open the bullet detail modal (Spec D1). Clicking a domain could navigate to the domains page.
- **Responsive sizing:** Adjust treemap `height` based on viewport width for better mobile experience.
- **Treemap labels:** Smart label sizing that hides labels when rectangles are too small (ECharts' `visibleMin` option).
- **Animation:** Enable `animationDurationUpdate` for smooth transitions when zooming into/out of categories.
