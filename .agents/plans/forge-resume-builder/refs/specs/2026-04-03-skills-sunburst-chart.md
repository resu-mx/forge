# Skills Sunburst / Pie Chart

**Date:** 2026-04-03
**Spec:** J2 (Skills Sunburst Chart)
**Phase:** TBD (next available)
**Depends on:** J1 (ECharts Infrastructure — `EChart.svelte` wrapper, `echarts-registry.ts`, `echarts-theme.ts`)
**Blocks:** None

## Overview

Add two interactive skill-distribution visualizations to the dashboard (`/`):

1. **Skills Sunburst** — hierarchical breakdown of skills by category, with drill-down.
2. **Domain-Archetype Breakout** — nested pie showing how perspectives distribute across domains and archetypes.

Both charts use the `EChart.svelte` wrapper from J1 and share a consistent color scheme tied to the design system's chart tokens.

## Non-Goals

- Creating or modifying skills/perspectives data (read-only visualizations)
- Adding new API endpoints in this spec (client-side aggregation from existing endpoints)
- Mobile-specific interactions (pinch, swipe)
- Chart export (PNG/SVG download)
- Accessibility (aria labels, screen reader) — deferred
- Filtering/search controls on the charts (deferred to a future toolbar spec)
- Caching or memoization of aggregated data (premature optimization)

---

## 1. Skills Sunburst Chart

### 1.1 Description

A sunburst chart showing every skill in the system, organized hierarchically:

- **Center:** total skill count (displayed as a centered label via ECharts `graphic` component)
- **Inner ring:** skill categories (`ai_ml`, `cloud`, `security`, `devops`, `languages`, `data`, `infrastructure`, `general`, etc.)
- **Outer ring:** individual skills within each category

Each wedge is sized by the skill's **usage count** — the number of distinct bullets and sources that reference it (via `bullet_skills` + `source_skills` junction tables). Skills with no references still appear but are sized at 1 (minimum).

Clicking an inner-ring category wedge drills down to show only that category's skills as a full pie. Clicking the center or an outer-ring wedge resets to the full sunburst.

### 1.2 Data Loading

Data is fetched client-side from existing endpoints and aggregated in the component:

```typescript
// Fetch all skills
const skillsResult = await forge.skills.list({ limit: 500 })

// Fetch all bullets (to count bullet_skills via technologies[])
const bulletsResult = await forge.bullets.list({ limit: 2000 })

// Fetch all perspectives (for the domain-archetype chart, loaded once)
const perspectivesResult = await forge.perspectives.list({ limit: 5000 })
```

**Aggregation logic** (in a helper function `buildSkillsSunburstData`):

```typescript
interface SkillUsage {
  id: string
  name: string
  category: string
  bulletCount: number   // how many bullets reference this skill (via technologies[])
  sourceCount: number   // not directly available from current API without new endpoint
  totalUsage: number    // bulletCount (sourceCount requires new API, deferred)
}

/**
 * Build ECharts sunburst data from skills + bullets.
 *
 * Since the SDK's Bullet type includes a `technologies: string[]` field
 * (which maps to bullet_technologies, not bullet_skills), we count
 * technology references per skill name. For source_skills, a new aggregate
 * endpoint would be needed — deferred. For now, usage = bullet technology count.
 */
function buildSkillsSunburstData(
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
```

### 1.3 ECharts Option

```typescript
function buildSunburstOption(data: SunburstDataItem[], totalSkills: number): EChartsOption {
  return {
    title: {
      text: 'Skills by Category',
      left: 'center',
      top: 10,
    },
    tooltip: {
      trigger: 'item',
      formatter: (params: any) => {
        if (params.treePathInfo?.length === 1) {
          // Root level — should not appear
          return ''
        }
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
            // Inner ring: categories
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
            // Outer ring: individual skills
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
    // Center label showing total skill count
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
```

### 1.4 Drill-Down Behavior

When a user clicks a category wedge (inner ring), the chart transitions to a single-level pie showing only that category's skills:

```typescript
function buildDrillDownOption(categoryName: string, children: SunburstChild[]): EChartsOption {
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
          // Reset to full sunburst — handled by component state
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
```

The component manages drill-down state:

```typescript
let drillCategory = $state<string | null>(null)

// When drillCategory changes, swap between sunburst and pie options
let chartOption = $derived(
  drillCategory
    ? buildDrillDownOption(drillCategory, getCategoryChildren(drillCategory))
    : buildSunburstOption(sunburstData, totalSkills)
)
```

ECharts click event handling uses J1's built-in `onChartEvent` prop (a `Record<string, (params: any) => void>`). No modification to `EChart.svelte` is needed.

The sunburst component listens for clicks:

```typescript
function handleSunburstClick(params: any) {
  if (drillCategory) {
    // In drill-down mode — clicking anything resets
    drillCategory = null
    return
  }

  // In sunburst mode — check if an inner ring (category) was clicked
  if (params.treePathInfo?.length === 2) {
    drillCategory = params.name
  }
}

// Pass to EChart via onChartEvent prop:
// onChartEvent={{ click: handleSunburstClick }}
```

---

## 2. Domain-Archetype Breakout Chart

### 2.1 Description

A nested pie (two concentric rings) showing how perspectives distribute across domains and archetypes:

- **Inner ring:** domains (`security`, `cloud`, `ai_ml`, `infrastructure`, `data`, `general`, etc.)
- **Outer ring:** archetypes within each domain (e.g., under `security`: `security_engineer`, `devsecops`, `appsec`)
- Each wedge is **sized by perspective count** — the number of approved perspectives tagged with that domain + archetype combination

This chart answers: "Where are my perspectives concentrated, and what roles do they support?"

### 2.2 Data Loading

Uses the same `perspectives` data fetched for the sunburst chart (single load, shared across both charts):

```typescript
/**
 * Build nested pie data from perspectives.
 * Groups by domain (inner ring) then target_archetype (outer ring).
 */
function buildDomainArchetypeData(
  perspectives: Perspective[],
): { inner: PieDataItem[]; outer: PieDataItem[] } {
  // Group: domain -> archetype -> count
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
```

### 2.3 ECharts Option

```typescript
function buildDomainArchetypeOption(
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

### 2.4 Color Coordination

Both rings share a color palette keyed by domain. The inner ring uses the base color; the outer ring uses lighter/darker variations. This is achieved by assigning explicit `itemStyle.color` values derived from the theme's chart palette:

```typescript
/**
 * ECharts does not resolve CSS variables. Domain colors must be resolved
 * at render time using getComputedStyle.
 */
function resolveTokenColor(token: string, fallback: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(token).trim() || fallback
}

/** Build resolved domain color map. Call at render time, not module scope. */
function buildDomainColors(): Record<string, string> {
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

The `resolveTokenColor` utility reads CSS custom property values at render time via `getComputedStyle`, since ECharts cannot interpret `var(--token)` strings directly.

---

## 3. Component: `SkillsSunburst.svelte`

### 3.1 File: `packages/webui/src/lib/components/charts/SkillsSunburst.svelte`

```svelte
<script lang="ts">
  import { onMount } from 'svelte'
  import { forge } from '$lib/sdk'
  import EChart from './EChart.svelte'
  import { LoadingSpinner } from '$lib/components'
  import type { Skill, Bullet, Perspective } from '@forge/sdk'

  // ── State ───────────────────────────────────────────────────────────
  let loading = $state(true)
  let skills = $state<Skill[]>([])
  let bullets = $state<Bullet[]>([])
  let perspectives = $state<Perspective[]>([])

  // Drill-down state for sunburst
  let drillCategory = $state<string | null>(null)

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
  // Use onMount instead of $effect to avoid infinite reactive loops:
  // loadData() writes to reactive state (skills, bullets, perspectives)
  // which $effect would re-track, causing re-execution.
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
```

### 3.2 Template

```svelte
<div class="skills-charts">
  {#if loading}
    <LoadingSpinner size="lg" message="Loading skill data..." />
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
```

### 3.3 Styling

```css
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
```

---

## 4. Dashboard Integration

### 4.1 Placement

The charts are added to the dashboard page (`packages/webui/src/routes/+page.svelte`) as a new section below the existing "Quick Stats" section.

**Important:** This section must be placed inside the `{:else}` block in `+page.svelte` (after the loading/error guards). Placing it outside will render the chart while data is still loading.

```svelte
<!-- After Quick Stats section -->
<section class="section">
  <h2 class="section-title">Skill & Domain Distribution</h2>
  <SkillsSunburst />
</section>
```

### 4.2 Import

```svelte
<script lang="ts">
  // ... existing imports ...
  import SkillsSunburst from '$lib/components/charts/SkillsSunburst.svelte'
</script>
```

### 4.3 Conditional Rendering

The chart section is only rendered when there is data to show. The component handles its own loading state, but the dashboard can optionally hide the section when `totalSkills === 0`:

```svelte
{#if totalSkills > 0}
  <section class="section">
    <h2 class="section-title">Skill & Domain Distribution</h2>
    <SkillsSunburst />
  </section>
{/if}
```

Since the component does its own data fetching, the simplest approach is to always render the component and let it show an `EmptyState` internally when data is empty.

---

## 5. EChart.svelte Event Extension

J1 includes `onChartEvent` from the start (see J1 spec, Section 4.4). No modification to `EChart.svelte` is required by this spec. The `onChartEvent` prop accepts a `Record<string, (params: any) => void>` and registers handlers via `chart.on()` during `onMount`.

---

## 6. Files to Create

| File | Purpose |
|------|---------|
| `packages/webui/src/lib/components/charts/SkillsSunburst.svelte` | Combined component: skills sunburst + domain-archetype breakout |
| `packages/webui/src/lib/components/charts/skills-chart-utils.ts` | Aggregation helpers: `buildSkillsSunburstData`, `buildDomainArchetypeData`, `buildSunburstOption`, `buildDrillDownOption`, `buildDomainArchetypeOption`, `getCategoryChildren` |

## 7. Files to Modify

| File | Change |
|------|--------|
| `packages/webui/src/routes/+page.svelte` | Import `SkillsSunburst` and add chart section below Quick Stats |

No changes to `EChart.svelte` — J1 includes `onChartEvent` from the start.

---

## 8. Testing

### 8.1 Aggregation Unit Tests

**File:** `packages/webui/src/lib/components/charts/__tests__/skills-chart-utils.test.ts`

```typescript
import { describe, it, expect } from 'vitest'
import {
  buildSkillsSunburstData,
  buildDomainArchetypeData,
  getCategoryChildren,
} from '../skills-chart-utils'

describe('buildSkillsSunburstData', () => {
  it('groups skills by category', () => {
    const skills = [
      { id: '1', name: 'Terraform', category: 'cloud', notes: null },
      { id: '2', name: 'AWS', category: 'cloud', notes: null },
      { id: '3', name: 'Python', category: 'languages', notes: null },
    ]
    const bullets = [
      { id: 'b1', technologies: ['terraform', 'aws'], /* ... other fields */ },
    ]
    const data = buildSkillsSunburstData(skills as any, bullets as any)

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
})

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
    expect(secDomain?.value).toBe(3)  // 3 perspectives total

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
})

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
```

### 8.2 Component Integration Tests

- `SkillsSunburst.svelte` renders loading spinner during fetch
- After fetch completes, two chart cards are visible
- With empty data, an `EmptyState` is shown instead of charts
- Drill-down state changes swap the first chart between sunburst and pie modes

### 8.3 EChart Event Extension Tests

- `EChart.svelte` with `onChartEvent` prop registers click handler on the chart instance
- Click events are forwarded to the callback with correct params
- Without `onChartEvent` prop, no event handlers are registered (backward compatible)

---

## 9. Acceptance Criteria

### Data aggregation
- [ ] `buildSkillsSunburstData` groups skills by category and computes usage counts from bullet technologies
- [ ] Skills with null category are grouped under `"uncategorized"`
- [ ] Unused skills appear in the chart with a minimum value of 1
- [ ] `buildDomainArchetypeData` groups perspectives by domain (inner ring) and archetype (outer ring)
- [ ] Null domain/archetype values are grouped under `"unassigned"`

### Skills Sunburst chart
- [ ] Renders a sunburst chart with inner ring (categories) and outer ring (skills)
- [ ] Center displays total skill count
- [ ] Wedges are sized by usage count (bullet technology references)
- [ ] Tooltip shows skill path and usage count
- [ ] Clicking a category wedge drills down to a pie chart of that category's skills
- [ ] Clicking anywhere in drill-down mode resets to full sunburst
- [ ] `notMerge={true}` ensures clean option swap on drill-down

### Domain-Archetype chart
- [ ] Renders a nested pie with inner ring (domains) and outer ring (archetypes)
- [ ] Wedges are sized by perspective count
- [ ] Tooltip shows name and count/percentage
- [ ] Scrollable legend on the right lists domain names

### EChart event extension
- [ ] `EChart.svelte` accepts optional `onChartEvent` prop
- [ ] Click events are forwarded with full ECharts params
- [ ] Existing consumers without `onChartEvent` are unaffected (backward compatible)
- [ ] `chart.dispose()` in cleanup removes all event listeners (no explicit `off()` needed)

### Dashboard integration
- [ ] Chart section appears below Quick Stats on the dashboard
- [ ] Section is labeled "Skill & Domain Distribution"
- [ ] Loading spinner displays while data is being fetched
- [ ] Component handles API errors gracefully (shows error state, does not crash)

### Testing
- [ ] All `buildSkillsSunburstData` unit tests pass (4 cases)
- [ ] All `buildDomainArchetypeData` unit tests pass (3 cases)
- [ ] All `getCategoryChildren` unit tests pass (2 cases)
- [ ] EChart event forwarding tests pass

---

## 10. Future Enhancements

- **Aggregate API endpoint:** A dedicated `GET /api/skills/usage-stats` endpoint returning skill names, categories, and usage counts would eliminate the need to fetch all bullets client-side. This is the recommended next step once the visualization proves valuable.
- **Source skill counts:** Once `source_skills` junction data is exposed via the SDK, the sunburst can incorporate source-level skill usage alongside bullet technology counts.
- **Perspective status filter:** Allow filtering the domain-archetype chart by perspective status (e.g., only `approved` perspectives) to show "production-ready" distribution.
- **Animation on drill:** ECharts supports `universalTransition` for smooth morphing between sunburst and pie. Can be enabled after validating performance.
