# Choropleth Map for Role Saturation

**Date:** 2026-04-03
**Spec:** J5 (Role Saturation Choropleth)
**Phase:** TBD (next available)
**Depends on:** J1 (ECharts Infrastructure — `EChart.svelte` wrapper, `echarts-registry.ts` with `MapChart` registered), E1 (JD Detail Page — JD entity with `location` field)
**Blocks:** None

## Overview

Add a choropleth (geographic heat map) to the dashboard showing where job opportunities are concentrated across US states. Color intensity represents the number of JDs in each state. The chart provides a geographic overview of the job search, highlighting which regions have the most opportunities and which are sparse.

JD locations are derived from the `location` free-text field on the `job_descriptions` table. Since this is unstructured text (e.g., "Remote", "San Francisco, CA", "Hybrid - DC"), the spec includes a state-resolution utility that extracts a US state abbreviation from the location string using pattern matching. JDs with non-resolvable locations (e.g., "Remote", "DOE") are aggregated under a "Remote / Unknown" category shown as a count badge outside the map.

ECharts supports `registerMap()` with GeoJSON data. This spec uses a US states GeoJSON file (public domain) loaded as a static asset.

## Non-Goals

- International maps (world, Europe, etc.) — deferred until international JDs are relevant
- City-level or zip-code-level granularity (state-level only)
- Real-time location geocoding via external API (pattern matching only)
- NLP-based location extraction from JD text
- Distance-from-user calculations
- Remote vs. on-site vs. hybrid filtering on the map (all JDs counted equally)
- Map pan/zoom beyond ECharts' built-in `roam` feature
- Custom map projections
- Map label density optimization (ECharts handles this)
- Mobile-specific interactions (pinch, rotate)
- Chart export (PNG/SVG download)
- Accessibility (aria labels, screen reader) — deferred

---

## 1. GeoJSON Data

### 1.1 Source

Use a US states GeoJSON file from the public domain. The most commonly used source is the US Census Bureau's cartographic boundary files or a simplified version from community GeoJSON repositories (e.g., `us-states.json` from d3-geo or similar).

**Recommended source:** A simplified US states GeoJSON with state names and FIPS codes. File size: ~300-500KB (simplified boundaries). Full-resolution boundaries are unnecessary for a state-level choropleth.

### 1.2 File Placement

**File:** `packages/webui/static/geo/us-states.json`

Place the GeoJSON in the `static` directory so it is served as a static asset and can be lazy-loaded by the component. This avoids bundling the GeoJSON into the JavaScript bundle.

### 1.3 Loading Strategy

Lazy-load the GeoJSON on component mount, then register it with ECharts:

```typescript
import { echarts } from './echarts-registry'

async function loadUSMap(): Promise<void> {
  const response = await fetch('/geo/us-states.json')
  const geoJson = await response.json()
  echarts.registerMap('USA', geoJson)
}
```

The map is registered once globally. Subsequent renders reuse the registered map data. If the map is already registered, skip the fetch.

### 1.4 GeoJSON Structure

The GeoJSON must be a `FeatureCollection` where each `Feature` has:
- `properties.name`: full state name (e.g., "California", "New York")
- `geometry`: polygon/multipolygon coordinates

ECharts matches data to map regions by the `name` property. The data series must use full state names (not abbreviations) to match.

---

## 2. State Resolution

### 2.1 Location-to-State Mapping

**File:** `packages/webui/src/lib/components/charts/state-resolver.ts`

Since JD locations are free text, a utility function extracts the US state from the location string:

```typescript
/**
 * US state abbreviation to full name mapping.
 */
const STATE_ABBR_TO_NAME: Record<string, string> = {
  AL: 'Alabama', AK: 'Alaska', AZ: 'Arizona', AR: 'Arkansas',
  CA: 'California', CO: 'Colorado', CT: 'Connecticut', DE: 'Delaware',
  FL: 'Florida', GA: 'Georgia', HI: 'Hawaii', ID: 'Idaho',
  IL: 'Illinois', IN: 'Indiana', IA: 'Iowa', KS: 'Kansas',
  KY: 'Kentucky', LA: 'Louisiana', ME: 'Maine', MD: 'Maryland',
  MA: 'Massachusetts', MI: 'Michigan', MN: 'Minnesota', MS: 'Mississippi',
  MO: 'Missouri', MT: 'Montana', NE: 'Nebraska', NV: 'Nevada',
  NH: 'New Hampshire', NJ: 'New Jersey', NM: 'New Mexico', NY: 'New York',
  NC: 'North Carolina', ND: 'North Dakota', OH: 'Ohio', OK: 'Oklahoma',
  OR: 'Oregon', PA: 'Pennsylvania', RI: 'Rhode Island', SC: 'South Carolina',
  SD: 'South Dakota', TN: 'Tennessee', TX: 'Texas', UT: 'Utah',
  VT: 'Vermont', VA: 'Virginia', WA: 'Washington', WV: 'West Virginia',
  WI: 'Wisconsin', WY: 'Wyoming', DC: 'District of Columbia',
}

/**
 * Full state name to abbreviation mapping (reverse).
 */
const STATE_NAME_TO_ABBR: Record<string, string> = Object.fromEntries(
  Object.entries(STATE_ABBR_TO_NAME).map(([abbr, name]) => [name.toLowerCase(), abbr])
)

/**
 * Attempt to resolve a US state from a free-text location string.
 * Returns the full state name (for GeoJSON matching) or null if unresolvable.
 *
 * Matching strategy (in order):
 * 1. Extract 2-letter state abbreviation from "City, ST" or "City, ST ZIP" patterns
 * 2. Match full state name anywhere in the string (case-insensitive)
 * 3. Match common city-to-state mappings (e.g., "NYC" -> "New York")
 * 4. Return null for "Remote", "Hybrid", or unresolvable locations
 */
export function resolveState(location: string | null | undefined): string | null {
  if (!location) return null

  const trimmed = location.trim()
  if (!trimmed) return null

  // Skip obvious non-geographic values
  const lower = trimmed.toLowerCase()
  if (lower === 'remote' || lower === 'doe' || lower === 'anywhere') return null

  // Pattern 1: "City, ST" or "City, ST 12345"
  // Match state abbreviation only after a comma, at start of string,
  // or followed by a zip code / end of string. This avoids false positives
  // like "AI Engineer" matching Arizona or "Remote OK" matching Oklahoma.
  const abbrMatch = trimmed.match(/(?:,\s*|^)([A-Z]{2})(?:\s+\d{5})?(?:\s*$|[^A-Za-z])/)
  if (abbrMatch) {
    const abbr = abbrMatch[1]
    if (STATE_ABBR_TO_NAME[abbr]) {
      return STATE_ABBR_TO_NAME[abbr]
    }
  }

  // Pattern 2: Full state name in the string
  for (const [name, _abbr] of Object.entries(STATE_NAME_TO_ABBR)) {
    if (lower.includes(name)) {
      return STATE_ABBR_TO_NAME[_abbr]
    }
  }

  // Pattern 3: Common city mappings
  const CITY_TO_STATE: Record<string, string> = {
    'nyc': 'New York',
    'new york city': 'New York',
    'manhattan': 'New York',
    'brooklyn': 'New York',
    'san francisco': 'California',
    'sf': 'California',
    'los angeles': 'California',
    'la': 'California',
    'silicon valley': 'California',
    'san jose': 'California',
    'san diego': 'California',
    'seattle': 'Washington',
    'chicago': 'Illinois',
    'boston': 'Massachusetts',
    'austin': 'Texas',
    'dallas': 'Texas',
    'houston': 'Texas',
    'denver': 'Colorado',
    'portland': 'Oregon',
    'atlanta': 'Georgia',
    'miami': 'Florida',
    'dc': 'District of Columbia',
    'washington dc': 'District of Columbia',
    'washington, dc': 'District of Columbia',
    'arlington': 'Virginia',
    'reston': 'Virginia',
    'mclean': 'Virginia',
    'bethesda': 'Maryland',
    'baltimore': 'Maryland',
    'phoenix': 'Arizona',
    'minneapolis': 'Minnesota',
    'detroit': 'Michigan',
    'pittsburgh': 'Pennsylvania',
    'philadelphia': 'Pennsylvania',
  }

  for (const [city, state] of Object.entries(CITY_TO_STATE)) {
    if (lower.includes(city)) {
      return state
    }
  }

  return null
}
```

### 2.2 Hybrid / Remote with Location

Some JDs have locations like "Hybrid - San Francisco, CA" or "Remote (Austin, TX preferred)". The `resolveState` function handles these because it searches for state abbreviations and city names anywhere in the string, not just in a specific position.

---

## 3. Data Aggregation

### 3.1 Aggregation Logic

**File:** `packages/webui/src/lib/components/charts/choropleth-utils.ts`

```typescript
export interface StateCount {
  name: string   // full state name (for GeoJSON matching)
  value: number  // count of JDs in this state
  jds: string[]  // JD titles (for tooltip)
}

export interface ChoroplethData {
  stateCounts: StateCount[]
  unresolvedCount: number
  unresolvedJDs: string[]   // titles of JDs with unresolvable locations
  totalJDs: number
  stateOrgsMap: Map<string, Array<{ name: string; count: number }>>  // precomputed org breakdown per state
}

/**
 * Aggregate JDs by US state.
 * JDs with unresolvable locations go into the "unresolved" bucket.
 */
export function aggregateByState(jds: JobDescriptionWithOrg[]): ChoroplethData {
  const stateMap = new Map<string, { count: number; jds: string[] }>()
  const stateOrgCountMap = new Map<string, Map<string, number>>()
  const unresolvedJDs: string[] = []

  for (const jd of jds) {
    const state = resolveState(jd.location)
    if (state) {
      const existing = stateMap.get(state) ?? { count: 0, jds: [] }
      existing.count++
      existing.jds.push(jd.title)
      stateMap.set(state, existing)

      // Precompute org counts per state for tooltip (avoids O(n) on each hover)
      if (!stateOrgCountMap.has(state)) stateOrgCountMap.set(state, new Map())
      const orgMap = stateOrgCountMap.get(state)!
      const orgName = jd.organization_name ?? 'Unknown'
      orgMap.set(orgName, (orgMap.get(orgName) ?? 0) + 1)
    } else {
      unresolvedJDs.push(jd.title)
    }
  }

  const stateCounts: StateCount[] = Array.from(stateMap.entries())
    .map(([name, data]) => ({
      name,
      value: data.count,
      jds: data.jds,
    }))
    .sort((a, b) => b.value - a.value)

  // Build precomputed stateOrgsMap: state -> sorted top-5 org list
  const stateOrgsMap = new Map<string, Array<{ name: string; count: number }>>()
  for (const [state, orgMap] of stateOrgCountMap) {
    const orgs = Array.from(orgMap.entries())
      .map(([name, count]) => ({ name, count }))
      .sort((a, b) => b.count - a.count)
      .slice(0, 5)
    stateOrgsMap.set(state, orgs)
  }

  return {
    stateCounts,
    unresolvedCount: unresolvedJDs.length,
    unresolvedJDs,
    totalJDs: jds.length,
    stateOrgsMap,
  }
}
```

### 3.2 Organization Aggregation for Tooltip

The tooltip for each state includes the top organizations in that state. Organization counts are precomputed during `aggregateByState` and stored in `ChoroplethData.stateOrgsMap: Map<string, Array<{ name: string; count: number }>>`. The tooltip reads from this precomputed map (O(1) lookup) instead of iterating all JDs on each hover (O(n)).

The standalone `getStateOrgs` function is no longer needed for tooltip rendering but is retained as a utility for other use cases (e.g., the selected-state detail panel):

```typescript
export interface StateOrgBreakdown {
  state: string
  orgs: Array<{ name: string; count: number }>
}

/**
 * For a given state, get the top organizations (by JD count).
 * Note: For tooltip rendering, prefer ChoroplethData.stateOrgsMap
 * (precomputed during aggregation) to avoid O(n) on every hover.
 */
export function getStateOrgs(
  jds: JobDescriptionWithOrg[],
  stateName: string,
): Array<{ name: string; count: number }> {
  const orgMap = new Map<string, number>()

  for (const jd of jds) {
    const state = resolveState(jd.location)
    if (state !== stateName) continue

    const orgName = jd.organization_name ?? 'Unknown'
    orgMap.set(orgName, (orgMap.get(orgName) ?? 0) + 1)
  }

  return Array.from(orgMap.entries())
    .map(([name, count]) => ({ name, count }))
    .sort((a, b) => b.count - a.count)
    .slice(0, 5)  // top 5 orgs
}
```

---

## 4. ECharts Configuration

### 4.1 Option Builder

```typescript
export function buildChoroplethOption(
  data: ChoroplethData,
): EChartsOption {
  const maxValue = Math.max(...data.stateCounts.map(s => s.value), 1)

  return {
    title: {
      text: 'JD Distribution by State',
      subtext: `${data.totalJDs} total • ${data.unresolvedCount} remote/unknown`,
      left: 'center',
      top: 10,
    },
    tooltip: {
      trigger: 'item',
      formatter: (params: any) => {
        const stateName = params.name
        const count = params.value ?? 0

        if (count === 0 || isNaN(count)) {
          return `<strong>${stateName}</strong><br/>No JDs`
        }

        // Read from precomputed stateOrgsMap instead of iterating all JDs (O(1) vs O(n))
        const orgs = data.stateOrgsMap.get(stateName) ?? []
        const orgLines = orgs
          .map(o => `  ${o.name}: ${o.count}`)
          .join('<br/>')

        return [
          `<strong>${stateName}</strong>`,
          `${count} JD${count !== 1 ? 's' : ''}`,
          orgs.length > 0 ? `<br/>Top orgs:<br/>${orgLines}` : '',
        ].join('<br/>')
      },
    },
    visualMap: {
      type: 'continuous',
      min: 0,
      max: maxValue,
      left: 'left',
      top: 'bottom',
      text: ['High', 'Low'],
      inRange: {
        color: ['#e0e7ff', '#818cf8', '#4f46e5', '#312e81'],
      },
      calculable: false,
      orient: 'horizontal',
      itemWidth: 120,
      itemHeight: 15,
    },
    series: [
      {
        type: 'map',
        map: 'USA',
        roam: true,           // enable pan/zoom
        aspectScale: 0.75,    // US map aspect ratio
        layoutCenter: ['50%', '50%'],
        layoutSize: '95%',
        emphasis: {
          label: { show: true, fontSize: 10 },
          itemStyle: {
            areaColor: '#fbbf24',
            borderColor: '#374151',
            borderWidth: 1,
          },
        },
        select: {
          label: { show: true },
          itemStyle: {
            areaColor: '#f59e0b',
          },
        },
        data: data.stateCounts.map(s => ({
          name: s.name,
          value: s.value,
        })),
        nameMap: {
          // ECharts uses GeoJSON property names by default.
          // No custom mapping needed if GeoJSON uses full state names.
        },
      },
    ],
  }
}
```

### 4.2 Color Scale

The `visualMap` uses a continuous color scale from light indigo (`#e0e7ff`) to dark indigo (`#312e81`). States with no JDs appear in the map's default light gray. The indigo palette is consistent with the design system's chart color scheme.

### 4.3 Map Interaction

- **Hover:** State highlights with amber fill, shows tooltip with state name, JD count, and top organizations
- **Click:** Selecting a state filters (see Section 5.3)
- **Pan/Zoom:** `roam: true` enables native ECharts pan and scroll-wheel zoom
- **Reset:** Double-click the map to reset zoom

---

## 5. Component

### 5.1 File: `packages/webui/src/lib/components/charts/RoleChoropleth.svelte`

```svelte
<script context="module" lang="ts">
  // mapLoaded is at module level so it persists across mount/unmount cycles.
  // ECharts' registerMap is global — once registered, it doesn't need re-registering.
  let mapLoaded = false
</script>

<script lang="ts">
  import { onMount } from 'svelte'
  import { goto } from '$app/navigation'
  import { forge } from '$lib/sdk'
  import { echarts } from './echarts-registry'
  import EChart from './EChart.svelte'
  import { aggregateByState, buildChoroplethOption } from './choropleth-utils'
  import type { ChoroplethData } from './choropleth-utils'
  import type { JobDescriptionWithOrg } from '@forge/sdk'

  let loading = $state(true)
  let jds = $state<JobDescriptionWithOrg[]>([])
  let data = $state<ChoroplethData | null>(null)
  let error = $state<string | null>(null)
  let selectedState = $state<string | null>(null)

  // Use onMount instead of $effect to avoid infinite reactive loops:
  // loadAll() writes to reactive state (jds, data, loading, error)
  // which $effect would re-track, causing re-execution.
  onMount(() => { loadAll() })

  async function loadAll() {
    loading = true
    error = null

    try {
      const [_, jdsResult] = await Promise.all([
        loadGeoJSON(),
        forge.jobDescriptions.list({ limit: 500 }),
      ])

      if (jdsResult.ok) {
        jds = jdsResult.data
        data = aggregateByState(jds)
      } else {
        error = 'Failed to load job descriptions'
      }
    } catch {
      error = 'An error occurred loading map data'
    } finally {
      loading = false
    }
  }

  async function loadGeoJSON() {
    // Only load once — check if map is already registered
    if (mapLoaded) return

    try {
      const response = await fetch('/geo/us-states.json')
      if (!response.ok) throw new Error(`GeoJSON fetch failed: ${response.status}`)
      const geoJson = await response.json()
      echarts.registerMap('USA', geoJson)
      mapLoaded = true
    } catch (e) {
      throw new Error('Failed to load US map data')
    }
  }

  let chartOption = $derived(
    data && mapLoaded ? buildChoroplethOption(data) : null
  )

  function handleMapClick(params: any) {
    if (params.componentType !== 'series') return

    const stateName = params.name
    if (selectedState === stateName) {
      // Deselect
      selectedState = null
    } else {
      selectedState = stateName
    }
  }

  // Navigate to JD list filtered by state
  function viewStateJDs() {
    if (selectedState) {
      goto(`/opportunities/job-descriptions?location=${encodeURIComponent(selectedState)}`)
    }
  }
</script>

{#if loading}
  <div class="choropleth-loading">Loading map data...</div>
{:else if error}
  <div class="choropleth-error">{error}</div>
{:else if !data || data.totalJDs === 0}
  <div class="choropleth-empty">
    No job descriptions yet. Create one with a location to see geographic distribution.
  </div>
{:else if chartOption}
  <div class="choropleth-container">
    <EChart
      option={chartOption}
      height="450px"
      notMerge={true}
      onChartEvent={{ click: handleMapClick }}
    />

    {#if selectedState}
      {@const stateData = data.stateCounts.find(s => s.name === selectedState)}
      <div class="state-detail">
        <strong>{selectedState}</strong>: {stateData?.value ?? 0} JD{(stateData?.value ?? 0) !== 1 ? 's' : ''}
        <button class="link-button" onclick={viewStateJDs}>
          View JDs
        </button>
      </div>
    {/if}

    {#if data.unresolvedCount > 0}
      <div class="unresolved-badge">
        <span class="unresolved-count">{data.unresolvedCount}</span>
        Remote / Unknown location
      </div>
    {/if}
  </div>
{/if}
```

### 5.2 Styling

```css
.choropleth-container {
  position: relative;
}

.state-detail {
  text-align: center;
  padding: 0.5rem;
  font-size: 13px;
  color: #374151;
}

.link-button {
  background: none;
  border: none;
  color: #6c63ff;
  text-decoration: underline;
  cursor: pointer;
  font-size: 13px;
}

.unresolved-badge {
  position: absolute;
  bottom: 60px;
  right: 20px;
  background: #f3f4f6;
  border: 1px solid #e5e7eb;
  border-radius: 6px;
  padding: 4px 10px;
  font-size: 11px;
  color: #6b7280;
}

.unresolved-count {
  font-weight: 700;
  color: #374151;
  margin-right: 4px;
}
```

### 5.3 State Click to Filter

Clicking a state on the map selects it and shows a detail bar below the chart with the state name, JD count, and a "View JDs" link. Clicking "View JDs" navigates to `/opportunities/job-descriptions?location=<state>`, which the JD list page can use to pre-filter by location (via URL query parameter matching).

The JD list page's existing client-side location filter (from E3's `JDFilterBar`) supports substring search, so navigating with `?location=California` will pre-populate the location filter and show only California JDs.

---

## 6. Dashboard Integration

### 6.1 Placement

The choropleth is added to the dashboard page (`/`) as a card in the dashboard section.

**Important:** This section must be placed inside the `{:else}` block in `+page.svelte` (after the loading/error guards). Placing it outside will render the chart while data is still loading.

```svelte
<section class="section">
  <h2 class="section-title">Opportunity Map</h2>
  <div class="chart-card">
    <RoleChoropleth />
  </div>
</section>
```

### 6.2 Import

```svelte
import RoleChoropleth from '$lib/components/charts/RoleChoropleth.svelte'
```

### 6.3 Conditional Rendering

The component handles its own empty state internally. The dashboard always renders the section.

---

## 7. Files to Create

| File | Purpose |
|------|---------|
| `packages/webui/src/lib/components/charts/RoleChoropleth.svelte` | Choropleth map component with loading, error, empty, and chart states |
| `packages/webui/src/lib/components/charts/choropleth-utils.ts` | `aggregateByState`, `buildChoroplethOption`, `getStateOrgs` |
| `packages/webui/src/lib/components/charts/state-resolver.ts` | `resolveState` function with state abbreviation, name, and city mappings |
| `packages/webui/static/geo/us-states.json` | US states GeoJSON file (public domain) |

## 8. Files to Modify

| File | Change |
|------|--------|
| `packages/webui/src/routes/+page.svelte` | Import `RoleChoropleth` and add "Opportunity Map" section |
| `packages/webui/src/routes/opportunities/job-descriptions/+page.svelte` | Read `?location=` query parameter and pre-populate location filter |

No changes to `echarts-registry.ts` — `MapChart` and `VisualMapComponent` are already registered by J1.

---

## 9. Testing

### 9.1 State Resolver Unit Tests

**File:** `packages/webui/src/lib/components/charts/__tests__/state-resolver.test.ts`

- `resolveState("San Francisco, CA")` returns `"California"`
- `resolveState("NYC")` returns `"New York"`
- `resolveState("Austin, TX 78701")` returns `"Texas"`
- `resolveState("Hybrid - Seattle, WA")` returns `"Washington"`
- `resolveState("Remote (Denver, CO preferred)")` returns `"Colorado"`
- `resolveState("Washington, DC")` returns `"District of Columbia"`
- `resolveState("DC")` returns `"District of Columbia"`
- `resolveState("Reston, VA")` returns `"Virginia"`
- `resolveState("Remote")` returns `null`
- `resolveState("DOE")` returns `null`
- `resolveState("")` returns `null`
- `resolveState(null)` returns `null`
- `resolveState(undefined)` returns `null`
- `resolveState("California")` returns `"California"` (full state name match)
- `resolveState("new york")` returns `"New York"` (case-insensitive)
- `resolveState("Anywhere")` returns `null`
- `resolveState("AI Engineer")` returns `null` (should NOT match Arizona — `AI` is not preceded by comma or at start in valid context)
- `resolveState("Remote OK")` returns `null` (should NOT match Oklahoma — `OK` is not preceded by comma or at start in valid context)

### 9.2 Aggregation Unit Tests

**File:** `packages/webui/src/lib/components/charts/__tests__/choropleth-utils.test.ts`

- `aggregateByState` groups JDs by resolved state
- `aggregateByState` counts JDs per state correctly
- `aggregateByState` puts "Remote" JDs in unresolvedCount
- `aggregateByState` returns empty stateCounts for all-remote JDs
- `aggregateByState` sorts stateCounts by value descending (highest first)
- `aggregateByState` includes JD titles in stateCount data
- `getStateOrgs` returns top organizations for a state sorted by count
- `getStateOrgs` limits to 5 organizations
- `getStateOrgs` uses "Unknown" for JDs with no organization name
- `buildChoroplethOption` produces valid ECharts option with map series
- `buildChoroplethOption` includes visualMap with correct min/max
- `buildChoroplethOption` subtitle shows total and unresolved counts

### 9.3 Component Tests

- `RoleChoropleth` shows loading state while data is being fetched
- `RoleChoropleth` shows "No job descriptions yet" when list is empty
- `RoleChoropleth` renders map chart when JDs exist
- `RoleChoropleth` shows error state when GeoJSON fails to load
- `RoleChoropleth` shows error state when JD API fails
- Clicking a state shows state detail bar with count and "View JDs" link
- Clicking "View JDs" navigates to JD list with location filter
- Clicking the same state again deselects it
- Unresolved badge shows count of Remote/Unknown JDs
- Unresolved badge is hidden when all JDs have resolvable locations
- Chart uses `roam: true` for pan/zoom
- Chart uses `notMerge={true}` for clean option updates

### 9.4 Integration Tests

- Create 3 JDs with locations "San Francisco, CA", "NYC", "Remote". Verify map shows California with 1, New York with 1, and unresolved badge with 1.
- Click California on the map. Verify state detail shows "California: 1 JD". Click "View JDs". Verify navigation to `/opportunities/job-descriptions?location=California`.
- JD list page reads `?location=California` and pre-populates the location filter.

---

## 10. Acceptance Criteria

1. `RoleChoropleth` component renders on the dashboard in an "Opportunity Map" section
2. A US states GeoJSON file is loaded as a static asset from `/geo/us-states.json`
3. GeoJSON is lazy-loaded on component mount and registered with `echarts.registerMap('USA', ...)`
4. GeoJSON is loaded only once (subsequent renders reuse the registered map)
5. `resolveState` extracts US state from free-text location strings using abbreviation, full name, and city patterns
6. JDs with unresolvable locations ("Remote", null, etc.) are counted in `unresolvedCount` and shown as a badge
7. Map colors states by JD count using a continuous indigo color scale via `visualMap`
8. Tooltip shows state name, JD count, and top 5 organizations in that state
9. Clicking a state shows a detail bar with state name, count, and "View JDs" link
10. "View JDs" navigates to `/opportunities/job-descriptions?location=<state>`
11. JD list page reads the `?location=` query parameter and pre-populates the location filter
12. Map supports pan and zoom via `roam: true`
13. Hover highlights state with amber fill
14. Component handles empty state, loading state, and error state gracefully
15. All state resolver unit tests pass (16 cases)
16. All aggregation unit tests pass (12 cases)

---

## 11. Future Enhancements

- **International maps:** Add world or regional map support when international JDs become relevant. Use ECharts' built-in world GeoJSON or a separate file per region.
- **City-level detail:** Drill into a state to see city-level distribution using a higher-resolution GeoJSON and geocoded JD locations.
- **Structured location field:** Replace or supplement the free-text `location` field with structured `state`/`city`/`country` columns to eliminate the need for pattern-matching resolution.
- **Remote visualization:** Show "Remote" JDs as a separate badge or as a pie chart overlay, breaking down remote vs. hybrid vs. on-site.
- **Salary overlay:** Color states by average salary instead of JD count, showing geographic salary variation.
- **Organization campus integration:** Derive JD locations from the organization's campus locations (org -> campus -> state) instead of the JD's free-text location field.
