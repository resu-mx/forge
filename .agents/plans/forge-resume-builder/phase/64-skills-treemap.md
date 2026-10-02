# Phase 64: Skills / Bullets / Domains Treemap (Spec J3)

**Status:** Planning
**Date:** 2026-04-03
**Spec:** [2026-04-03-skills-treemap.md](../refs/specs/2026-04-03-skills-treemap.md)
**Depends on:** Phase 59 (ECharts Infrastructure -- `EChart.svelte` wrapper, `echarts-registry.ts`, `echarts-theme.ts`)
**Blocks:** None
**Parallelizable with:** Phase 63 (Skills Sunburst), and all other phases -- creates new files only, modifies `+page.svelte` (dashboard section addition)

## Goal

Add three treemap visualizations to the dashboard: (1) a Skills Treemap showing skills sized by bullet technology reference count and grouped by category, (2) a Bullets Treemap showing bullets sized by perspective count and grouped by primary source, and (3) a Domains Treemap showing domains sized by perspective count. Treemaps use rectangular packing to make relative proportions immediately visible in large flat datasets. Each chart is a self-contained Svelte component with its own data loading, and all aggregation logic lives in a shared pure utility module (`treemap-utils.ts`) for testability.

## Non-Goals

- Editing data from the treemap (read-only visualization)
- Adding new API endpoints (client-side aggregation from existing endpoints)
- Mobile-specific interactions (pinch zoom, pan)
- Chart export (PNG/SVG download)
- Accessibility (aria labels, screen reader) -- deferred
- Filtering or search controls on treemaps -- deferred to a future toolbar spec
- Real-time updates (treemaps refresh on page load or manual reload)
- Data sharing with J2 components (each component independently fetches; shared data lifting is a future optimization)
- Optimizing dashboard data load cost (the dashboard loads 12,000+ objects across J2+J3 chart components -- this is acceptable for a single-user tool but should be documented as a known cost; a shared data context or caching layer is deferred to a future optimization spec)

## Context

The dashboard needs visual representations of data proportions beyond the hierarchical sunburst/pie charts from Phase 63 (J2). Treemaps excel at showing relative proportions: which skills dominate usage, which sources produce the most productive bullets, and which domains have the most perspective coverage. The ECharts treemap series type provides built-in drill-down (`nodeClick: 'zoomToNode'`), breadcrumb navigation, and rectangular label layout.

Key patterns established in J1 and validated during spec review:
- Use `onMount(() => { loadData() })` NOT `$effect` for data fetching (avoids infinite reactive loops)
- `$derived` cannot be destructured in Svelte 5
- ECharts cannot resolve CSS custom properties -- but treemaps use `colorMappingBy: 'index'` which maps to ECharts' built-in palette, so `resolveTokenColor()` is not needed here
- Dashboard sections must go inside `{:else}` block in `+page.svelte`
- `onChartEvent` prop is built into J1's `EChart.svelte` -- no modifications needed (though treemaps use ECharts' built-in `nodeClick: 'zoomToNode'` rather than custom click handlers)

The Domains Treemap uses `Perspective[]` for domain data because the `domain` field exists on `Perspective`, NOT on `Bullet`. This is a key data model distinction -- bullets do not have a domain field.

The `TreemapNode` interface is extended with `perspectiveCount?` and `bulletCount?` optional fields for type-safe tooltip rendering in the Domains Treemap.

## Scope

| Spec section | Covered here? |
|-------------|---------------|
| 1. Skills Treemap (data, option, drill-down) | Yes |
| 2. Bullets Treemap (data, option, drill-down) | Yes |
| 3. Domains Treemap (data, option, no drill-down) | Yes |
| 4. Component architecture (3 separate components) | Yes |
| 5. Dashboard integration | Yes |
| 6. Files to create | Yes |
| 7. Files to modify | Yes |
| 8. Testing | Yes |
| 9. Acceptance criteria | Yes |

## Files to Create

| File | Description |
|------|-------------|
| `packages/webui/src/lib/components/charts/treemap-utils.ts` | `TreemapNode` type, `buildSkillsTreemapData`, `buildSkillsTreemapOption`, `buildBulletsTreemapData`, `buildBulletsTreemapOption`, `buildDomainsTreemapData`, `buildDomainsTreemapOption` |
| `packages/webui/src/lib/components/charts/SkillsTreemap.svelte` | Skills treemap: skills sized by bullet technology count, grouped by category |
| `packages/webui/src/lib/components/charts/BulletsTreemap.svelte` | Bullets treemap: bullets sized by perspective count, grouped by primary source |
| `packages/webui/src/lib/components/charts/DomainsTreemap.svelte` | Domains treemap: domains sized by perspective count |
| `packages/webui/src/lib/components/charts/__tests__/treemap-utils.test.ts` | Unit tests for all aggregation and option-building functions |

## Files to Modify

| File | Change |
|------|--------|
| `packages/webui/src/routes/+page.svelte` | Import three treemap components and add "Treemap Views" section inside `{:else}` block |

## Fallback Strategies

- **Empty data from API:** If `forge.skills.list()`, `forge.bullets.list()`, or `forge.perspectives.list()` returns `ok: false`, the corresponding array stays empty. Each component renders `EmptyState` when its primary data array is empty. No crash, no blank chart.
- **No technologies on bullets:** If `bullet.technologies` is empty for all bullets, every skill gets minimum value 1. The skills treemap renders with all rectangles equally sized, which is correct and communicates "no usage data yet."
- **No primary source on bullets:** Bullets without a primary source are grouped under `"No Source"` in the bullets treemap. The `sources?.find(s => s.is_primary)` safely handles undefined/empty arrays.
- **No perspectives for any bullet:** All bullets get minimum value 1 in the bullets treemap. The visualization still renders -- it just shows source grouping without meaningful sizing.
- **ECharts treemap type not registered:** If `echarts-registry.ts` from J1 does not include the treemap series type, the chart will fail silently. Fallback: add `TreemapChart` to the registry's `use()` call. Task T64.4 verifies this.
- **Very long bullet content:** `buildBulletsTreemapData` truncates bullet content to 60 characters with ellipsis. The tooltip formatter additionally truncates to 100 characters. No overflow or rendering issues.
- **`notMerge` for clean state:** All three `EChart` instances use `notMerge={true}` to ensure options are replaced cleanly when data changes (e.g., on re-fetch).
- **Perspective count not on Bullet type:** The spec acknowledges that `perspective_count` may not be on the `Bullet` SDK type. The `BulletsTreemap` component fetches perspectives separately and builds a `Map<string, number>` counting perspectives per `bullet_id` client-side.

---

## Tasks

### T64.1: Write Treemap Utility Functions

**File:** `packages/webui/src/lib/components/charts/treemap-utils.ts`

Defines the `TreemapNode` interface and all pure aggregation + option-builder functions. No DOM access, no side effects.

[CRITICAL] The `TreemapNode` interface includes optional `perspectiveCount?` and `bulletCount?` fields for type-safe tooltip access in the Domains Treemap. These fields are NOT standard ECharts fields -- they are custom data attached to nodes and accessed via `params.data` in tooltip formatters.

[CRITICAL] `buildDomainsTreemapData` accepts `Perspective[]`, NOT `Bullet[]`. The `domain` field exists on `Perspective`, not on `Bullet`. Using bullets would produce incorrect data.

[MINOR] All `buildXxxTreemapData` functions sort output by value descending so the largest rectangles render first (top-left in ECharts treemap layout).

```typescript
import type { EChartsOption } from 'echarts'
import type { Skill, Bullet, Perspective } from '@forge/sdk'

// ── Types ────────────────────────────────────────────────────────────────

export interface TreemapNode {
  name: string
  value: number
  children?: TreemapNode[]
  itemStyle?: { color?: string }
  perspectiveCount?: number   // type-safe tooltip data for domains treemap
  bulletCount?: number        // type-safe tooltip data (reserved for future use)
}

// ── Skills Treemap ───────────────────────────────────────────────────────

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

  // Convert to treemap hierarchy, sorted by total value descending
  return Array.from(categories.entries())
    .map(([category, children]) => ({
      name: category,
      value: children.reduce((sum, c) => sum + c.value, 0),
      children: children.sort((a, b) => b.value - a.value),
    }))
    .sort((a, b) => b.value - a.value)
}

/** Build ECharts option for the skills treemap. */
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
        colorMappingBy: 'index',
      },
    ],
  }
}

// ── Bullets Treemap ──────────────────────────────────────────────────────

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

/** Build ECharts option for the bullets treemap. */
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

// ── Domains Treemap ──────────────────────────────────────────────────────

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
      perspectiveCount: counts.perspectiveCount,
    }))
    .sort((a, b) => b.value - a.value) as TreemapNode[]
}

/** Build ECharts option for the domains treemap. */
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

**Acceptance criteria:**
- All functions export correctly from the module.
- `TreemapNode` type includes optional `perspectiveCount?` and `bulletCount?` fields.
- `buildSkillsTreemapData` groups skills by category, sizes by bullet tech count, sorts descending.
- Skills with null category go under `"uncategorized"`.
- Unused skills get minimum value of 1.
- `buildBulletsTreemapData` groups bullets by primary source, sizes by perspective count.
- Bullets without a primary source go under `"No Source"`.
- Long bullet content is truncated to 60 chars with `...`.
- `buildDomainsTreemapData` accepts `Perspective[]` (NOT `Bullet[]`) and groups by `p.domain`.
- Null domain values go under `"unassigned"`.
- `buildDomainsTreemapOption` sets `nodeClick: false` (no drill-down).
- All option builders return objects with `series[0].type === 'treemap'`.

**Failure criteria:**
- `buildDomainsTreemapData` accepts `Bullet[]` instead of `Perspective[]`.
- `buildBulletsTreemapData` crashes on bullets with undefined `sources` array.
- Option builders produce invalid ECharts config (wrong series type, missing width/height).
- `TreemapNode` does not have the extended tooltip fields.

---

### T64.2: Write SkillsTreemap Component

**File:** `packages/webui/src/lib/components/charts/SkillsTreemap.svelte`

Self-contained component that fetches skills + bullets, aggregates into treemap data, and renders via `EChart.svelte`. Uses ECharts' built-in `nodeClick: 'zoomToNode'` for drill-down -- no custom click handler needed.

[CRITICAL] Data fetching uses `onMount(() => { loadData() })`, NOT `$effect`. Writing to reactive state inside an effect that reads that state causes an infinite loop.

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

  // Use onMount instead of $effect to avoid infinite reactive loops
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

**Acceptance criteria:**
- Renders loading spinner during data fetch.
- Shows `EmptyState` when no skills are loaded.
- Renders treemap chart card when data is available.
- Clicking a category zooms into it (built-in `nodeClick: 'zoomToNode'`).
- Breadcrumb appears for navigation back to top level.
- `notMerge={true}` ensures clean option replacement.

**Failure criteria:**
- Infinite loop from `$effect`-based data loading.
- Component crashes when `forge.skills.list()` returns `ok: false`.

---

### T64.3: Write BulletsTreemap Component

**File:** `packages/webui/src/lib/components/charts/BulletsTreemap.svelte`

Self-contained component that fetches bullets + perspectives, builds perspective count map, and renders bullets treemap. The perspective count map is built client-side because the `Bullet` SDK type may not include `perspective_count`.

[CRITICAL] Perspective counts must be built as a `Map<string, number>` by iterating `perspectives` and counting by `bullet_id`. The `perspectiveCounts` state variable is a `Map`, not an array.

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

  // Use onMount instead of $effect to avoid infinite reactive loops
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

**Acceptance criteria:**
- Renders loading spinner during data fetch.
- Shows `EmptyState` when no bullets are loaded.
- Renders treemap chart card when data is available.
- Bullets are grouped by primary source.
- Clicking a source group zooms into it.
- Perspective counts are computed client-side from the perspectives endpoint.

**Failure criteria:**
- Component uses `bullet.perspective_count` which may not exist on the SDK type.
- Infinite loop from `$effect`-based data loading.

---

### T64.4: Write DomainsTreemap Component

**File:** `packages/webui/src/lib/components/charts/DomainsTreemap.svelte`

Self-contained component that fetches perspectives and renders a single-level domains treemap. No drill-down (`nodeClick: false`), no breadcrumb.

[CRITICAL] This component fetches `Perspective[]`, NOT `Bullet[]`. The `domain` field exists on `Perspective`, not `Bullet`. The spec explicitly calls this out.

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

  // Use onMount instead of $effect to avoid infinite reactive loops
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

**Acceptance criteria:**
- Renders loading spinner during data fetch.
- Shows `EmptyState` when no perspectives are loaded.
- Renders single-level treemap with domain rectangles.
- No drill-down (clicking a rectangle does nothing).
- Breadcrumb is hidden.
- Domain labels are visible inside rectangles with count.
- Tooltip shows perspective count via `perspectiveCount` field.

**Failure criteria:**
- Component fetches `Bullet[]` instead of `Perspective[]`.
- Drill-down is accidentally enabled.

---

### T64.5: Verify ECharts Registry Includes Treemap

**File:** `packages/webui/src/lib/components/charts/echarts-registry.ts` (from Phase 59)

[CRITICAL] The treemap chart type must be registered in the ECharts registry from J1. If J1 only registered `bar`, `line`, `sunburst`, and `pie`, this phase must add `TreemapChart` to the `use()` call.

Verify the registry includes:
```typescript
import { TreemapChart } from 'echarts/charts'
// ... in use() call:
echarts.use([TreemapChart, /* ... existing registrations ... */])
```

If not present, add it. This extends the registry without changing existing registrations.

**Acceptance criteria:**
- `TreemapChart` is registered in `echarts-registry.ts`.
- Existing chart type registrations are unchanged.
- Import compiles without errors.

**Failure criteria:**
- Treemap charts fail to render because the chart type is not registered.
- Removing existing registrations breaks other charts.

---

### T64.6: Integrate into Dashboard

**File:** `packages/webui/src/routes/+page.svelte`

[CRITICAL] The treemap section MUST be placed inside the `{:else}` block in `+page.svelte`. If Phase 63 (J2) has already landed, this section goes below the "Skill & Domain Distribution" section. Otherwise, it goes below Quick Stats.

Add the imports:
```svelte
<script lang="ts">
  // ... existing imports ...
  import SkillsTreemap from '$lib/components/charts/SkillsTreemap.svelte'
  import BulletsTreemap from '$lib/components/charts/BulletsTreemap.svelte'
  import DomainsTreemap from '$lib/components/charts/DomainsTreemap.svelte'
</script>
```

Add the section inside the `{:else}` block:
```svelte
<!-- After Quick Stats (or after J2 charts if present), inside {:else} block -->
<section class="section">
  <h2 class="section-title">Treemap Views</h2>
  <div class="treemap-grid">
    <SkillsTreemap />
    <BulletsTreemap />
    <DomainsTreemap />
  </div>
</section>
```

Add the grid styling:
```css
.treemap-grid {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}
```

**Acceptance criteria:**
- "Treemap Views" section appears on dashboard below existing sections.
- Three treemap components are stacked vertically inside `.treemap-grid`.
- Section is inside the `{:else}` block.
- All three imports are added.

**Failure criteria:**
- Section renders outside `{:else}` block.
- Import paths are wrong.
- Missing `.treemap-grid` styling causes layout issues.

---

### T64.7: Write Unit Tests

**File:** `packages/webui/src/lib/components/charts/__tests__/treemap-utils.test.ts`

```typescript
import { describe, it, expect } from 'vitest'
import {
  buildSkillsTreemapData,
  buildBulletsTreemapData,
  buildDomainsTreemapData,
  buildSkillsTreemapOption,
  buildBulletsTreemapOption,
  buildDomainsTreemapOption,
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
    expect(data[0].name).toBe('big')
  })

  it('returns empty array for no skills', () => {
    const data = buildSkillsTreemapData([], [])
    expect(data).toHaveLength(0)
  })

  it('computes category value as sum of children', () => {
    const skills = [
      { id: '1', name: 'A', category: 'cat', notes: null },
      { id: '2', name: 'B', category: 'cat', notes: null },
    ]
    const bullets = [
      { id: 'b1', technologies: ['a'] },
      { id: 'b2', technologies: ['b', 'b'] },
    ] as any[]
    const data = buildSkillsTreemapData(skills as any, bullets)
    // A: 1 ref, B: 2 refs (from two entries in b2) -> incorrect: b has 1 ref in b2
    // Actually: 'a' appears once in b1, 'b' appears once in b2
    // A=1, B=1 -> category total = 2
    expect(data[0].value).toBe(2)
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
    expect(data).toHaveLength(2)

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

  it('sorts source groups by total value descending', () => {
    const bullets = [
      { id: 'b1', content: 'A', sources: [{ id: 's1', title: 'Small', is_primary: true }] },
      { id: 'b2', content: 'B', sources: [{ id: 's2', title: 'Big', is_primary: true }] },
    ] as any[]
    const counts = new Map([['b1', 1], ['b2', 10]])
    const data = buildBulletsTreemapData(bullets, counts)
    expect(data[0].name).toBe('Big')
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

// ── Option builders ──────────────────────────────────────────────────

describe('buildSkillsTreemapOption', () => {
  it('returns option with treemap series type', () => {
    const data = [{ name: 'cloud', value: 3, children: [{ name: 'AWS', value: 3 }] }]
    const option = buildSkillsTreemapOption(data)
    expect((option.series as any[])[0].type).toBe('treemap')
  })

  it('enables nodeClick zoomToNode', () => {
    const option = buildSkillsTreemapOption([])
    expect((option.series as any[])[0].nodeClick).toBe('zoomToNode')
  })

  it('shows breadcrumb', () => {
    const option = buildSkillsTreemapOption([])
    expect((option.series as any[])[0].breadcrumb.show).toBe(true)
  })

  it('has two levels (category + skill)', () => {
    const option = buildSkillsTreemapOption([])
    expect((option.series as any[])[0].levels).toHaveLength(2)
  })
})

describe('buildBulletsTreemapOption', () => {
  it('returns option with treemap series type', () => {
    const option = buildBulletsTreemapOption([])
    expect((option.series as any[])[0].type).toBe('treemap')
  })

  it('has title "Bullets by Source"', () => {
    const option = buildBulletsTreemapOption([])
    expect((option.title as any).text).toBe('Bullets by Source')
  })

  it('enables nodeClick zoomToNode', () => {
    const option = buildBulletsTreemapOption([])
    expect((option.series as any[])[0].nodeClick).toBe('zoomToNode')
  })
})

describe('buildDomainsTreemapOption', () => {
  it('returns option with treemap series type', () => {
    const option = buildDomainsTreemapOption([])
    expect((option.series as any[])[0].type).toBe('treemap')
  })

  it('disables nodeClick (no drill-down)', () => {
    const option = buildDomainsTreemapOption([])
    expect((option.series as any[])[0].nodeClick).toBe(false)
  })

  it('hides breadcrumb', () => {
    const option = buildDomainsTreemapOption([])
    expect((option.series as any[])[0].breadcrumb.show).toBe(false)
  })

  it('has title "Domain Coverage"', () => {
    const option = buildDomainsTreemapOption([])
    expect((option.title as any).text).toBe('Domain Coverage')
  })

  it('has single level', () => {
    const option = buildDomainsTreemapOption([])
    expect((option.series as any[])[0].levels).toHaveLength(1)
  })
})
```

**Acceptance criteria:**
- All `buildSkillsTreemapData` tests pass (7 cases).
- All `buildBulletsTreemapData` tests pass (6 cases).
- All `buildDomainsTreemapData` tests pass (5 cases).
- All `buildSkillsTreemapOption` tests pass (4 cases).
- All `buildBulletsTreemapOption` tests pass (3 cases).
- All `buildDomainsTreemapOption` tests pass (5 cases).
- Total: 30 test cases.

**Failure criteria:**
- Tests import functions that do not exist in the utility module.
- Tests assert against wrong field names or values.

---

## Testing

| Test file | Cases | Type |
|-----------|-------|------|
| `packages/webui/src/lib/components/charts/__tests__/treemap-utils.test.ts` | 30 | Unit |

**Run command:** `cd packages/webui && npx vitest run src/lib/components/charts/__tests__/treemap-utils.test.ts`

Component integration tests (rendering, loading state, drill-down interaction, empty state) are deferred to visual QA since they require a full Svelte component testing setup with mocked SDK client.

## Docs

No documentation files created. The spec and this plan serve as the reference. Component usage is self-documenting: import `SkillsTreemap`, `BulletsTreemap`, or `DomainsTreemap` and render them.

## Parallelization

This phase creates new files and adds a section to `+page.svelte`. It can run in parallel with:

- **Phase 63 (Skills Sunburst):** Both phases modify `+page.svelte` to add dashboard sections. The modifications are additive (new sections in the `{:else}` block) and will not conflict as long as they are added in sequence (J2 section first, J3 section below).
- **All other phases** that do not touch `+page.svelte` or the `charts/` directory.

Dependencies:
- **Phase 59 (ECharts Infrastructure):** Must be complete. Provides `EChart.svelte`, `echarts-registry.ts`, and `echarts-theme.ts`. T64.5 may extend the registry with `TreemapChart` if not already included.

Task ordering within this phase:
- T64.1 (utils) and T64.5 (registry verify) can run in parallel.
- T64.2 (SkillsTreemap), T64.3 (BulletsTreemap), and T64.4 (DomainsTreemap) all depend on T64.1 but can run in parallel with each other.
- T64.6 (dashboard) depends on T64.2 + T64.3 + T64.4.
- T64.7 (tests) depends on T64.1.
- T64.2, T64.3, T64.4, and T64.7 can run in parallel after T64.1 completes.
