# Phase 68: Choropleth Map for Role Saturation (Spec J5)

**Status:** Planning
**Date:** 2026-04-03
**Spec:** [2026-04-03-choropleth-map.md](../refs/specs/2026-04-03-choropleth-map.md)
**Depends on:** Phase 59 (ECharts Infrastructure — `EChart.svelte` wrapper, `echarts-registry.ts` with `MapChart` and `VisualMapComponent` registered), Phase 49 (JD Detail — JD entity with `location` field)
**Blocks:** None
**Parallelizable with:** Phase 65, Phase 66, Phase 67 -- creates new files, only modifies dashboard `+page.svelte` and JD list page

## Goal

Add a choropleth (geographic heat map) to the dashboard showing where job opportunities are concentrated across US states. Color intensity represents the number of JDs per state via a continuous indigo `visualMap`. The chart includes a state-resolution utility that extracts US state names from free-text JD location strings using context-aware regex patterns (avoiding false positives like "AI" matching Arizona or "OK" matching Oklahoma), city-to-state mappings, and full state name matching. Organization breakdown per state is precomputed during aggregation (not on every hover) and stored in `stateOrgsMap`. The `mapLoaded` flag is at module level to avoid re-registering the GeoJSON across mount/unmount cycles.

## Non-Goals

- International maps (world, Europe) -- deferred until international JDs are relevant
- City-level or zip-code-level granularity (state-level only)
- Real-time geocoding via external API (pattern matching only)
- NLP-based location extraction from JD text
- Distance-from-user calculations
- Remote vs. on-site vs. hybrid filtering on the map
- Custom map projections
- Mobile-specific interactions, chart export, accessibility

## Context

JD locations are stored as a free-text `location` field (e.g., "Remote", "San Francisco, CA", "Hybrid - DC"). The state-resolution utility parses these strings to extract US state names using three strategies: (1) context-aware regex for 2-letter abbreviations after commas or at specific positions, (2) full state name matching case-insensitively, and (3) common city-to-state mappings. JDs with non-resolvable locations ("Remote", "DOE") are aggregated under a "Remote / Unknown" badge outside the map.

ECharts requires a GeoJSON file registered via `echarts.registerMap('USA', geoJson)`. The GeoJSON is loaded as a static asset from `/geo/us-states.json` and lazy-loaded on component mount. The `mapLoaded` flag is at module level (not component `$state`) because `echarts.registerMap` is a global operation that persists across component lifecycles.

The `visualMap` uses swapped dimensions for horizontal orientation: `itemWidth: 120` (the length) and `itemHeight: 15` (the thickness). These names are counterintuitive because ECharts named them for vertical orientation.

## Scope

| Spec section | Covered here? |
|-------------|---------------|
| 1. GeoJSON Data (source, placement, loading, structure) | Yes |
| 2. State Resolution (`resolveState` with regex, name, city patterns) | Yes |
| 3. Data Aggregation (`aggregateByState`, `stateOrgsMap`, `getStateOrgs`) | Yes |
| 4. ECharts Configuration (option builder, color scale, interactions) | Yes |
| 5. Component (`RoleChoropleth.svelte`, module-level `mapLoaded`, state click) | Yes |
| 6. Dashboard Integration (placement, import) | Yes |
| 7-8. Files to create/modify | Yes |
| 9. Testing | Yes |
| 10. Acceptance criteria | Yes |
| 11. Future enhancements | No (informational only) |

## Files to Create

| File | Description |
|------|-------------|
| `packages/webui/src/lib/components/charts/state-resolver.ts` | `resolveState` function with state abbreviation-to-name map, name-to-abbreviation reverse map, city-to-state map, and context-aware regex |
| `packages/webui/src/lib/components/charts/choropleth-utils.ts` | `aggregateByState`, `buildChoroplethOption`, `getStateOrgs` |
| `packages/webui/src/lib/components/charts/RoleChoropleth.svelte` | Choropleth map component with module-level `mapLoaded`, loading/error/empty states, state click detail, unresolved badge |
| `packages/webui/static/geo/us-states.json` | US states GeoJSON file (public domain, simplified boundaries ~300-500KB) |
| `packages/webui/src/lib/components/charts/__tests__/state-resolver.test.ts` | Unit tests for state resolution (19 cases) |
| `packages/webui/src/lib/components/charts/__tests__/choropleth-utils.test.ts` | Unit tests for aggregation and option building (13 cases) |

## Files to Modify

| File | Change |
|------|--------|
| `packages/webui/src/routes/+page.svelte` | Import `RoleChoropleth` and add "Opportunity Map" section inside the `{:else}` block |
| `packages/webui/src/routes/opportunities/job-descriptions/+page.svelte` | Read `?location=` query parameter and pre-populate location filter |

No changes to `echarts-registry.ts` -- `MapChart` and `VisualMapComponent` are already registered by Phase 59.

## Fallback Strategies

- **GeoJSON fails to load (network error, 404):** The `loadGeoJSON` function throws, which is caught by `loadAll()`. The component shows an error state ("An error occurred loading map data"). No blank map or crash.
- **GeoJSON already registered (component re-mount):** The module-level `mapLoaded` flag prevents re-fetching and re-registering. `echarts.registerMap` with the same name would overwrite silently, but the fetch is wasted bandwidth. The flag avoids this.
- **JD location is "AI Engineer" (false positive risk for Arizona):** The context-aware regex `(?:,\s*|^)([A-Z]{2})(?:\s+\d{5})?(?:\s*$|[^A-Za-z])` requires the 2-letter abbreviation to appear after a comma or at the start of the string, followed by a zip code or end of string. "AI" inside "AI Engineer" does not match because it is followed by a letter ("E"). This prevents false positives for AI, ML, OK, etc.
- **JD location is "Remote OK":** Same regex protection -- "OK" is preceded by "Remote " (not a comma) and followed by end of string. But wait -- "OK" at end of string COULD match `(?:,\s*|^)` if interpreted as start-of-string. The regex requires `^` match for the full string "OK", but "Remote OK" has "OK" at position 7, not position 0. The `(?:,\s*|^)` group requires comma or start. "Remote OK" has neither before "OK". So "OK" is correctly rejected. Additional guard: the "skip obvious non-geographic values" check catches `lower === 'remote'` early for plain "Remote", but "Remote OK" passes that check. The regex handles it correctly.
- **All JDs are "Remote":** `stateCounts` is empty, `unresolvedCount === totalJDs`. The component shows the map with no colored states and the unresolved badge. This is technically correct but visually uninteresting. The component does NOT show the empty state for this case (it shows the map + badge) because there ARE JDs, just none with resolvable locations.
- **Zero JDs total:** Component shows the empty state ("No job descriptions yet.").
- **State click navigation:** Navigates to `/opportunities/job-descriptions?location=<state>`. The JD list page reads the `?location=` query parameter and pre-populates the location filter. If the JD list page does not support this parameter yet, the navigation still works but no pre-filtering occurs.
- **Tooltip performance:** Organization breakdown per state is precomputed during `aggregateByState()` and stored in `ChoroplethData.stateOrgsMap` (a `Map<string, Array<{name, count}>>`). The tooltip reads from this O(1) lookup instead of iterating all JDs on every hover (O(n)).

---

## Tasks

### T68.1: Write State Resolution Utility

**File:** `packages/webui/src/lib/components/charts/state-resolver.ts`

[CRITICAL] Context-aware regex for abbreviation matching: `(?:,\s*|^)([A-Z]{2})(?:\s+\d{5})?(?:\s*$|[^A-Za-z])`. This requires the 2-letter code to appear after a comma (with optional whitespace) or at the very start of the string, and be followed by a zip code, end of string, or a non-letter character. This prevents "AI" in "AI Engineer" from matching Arizona, and "OK" in "Remote OK" from matching Oklahoma. Without this context-aware pattern, common two-letter sequences in job titles and descriptions cause false positives.

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
 * Common city-to-state mappings for locations that lack explicit state info.
 */
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

/**
 * Attempt to resolve a US state from a free-text location string.
 * Returns the full state name (for GeoJSON matching) or null if unresolvable.
 *
 * Matching strategy (in order):
 * 1. Extract 2-letter state abbreviation from "City, ST" or "City, ST ZIP" patterns
 *    using context-aware regex to avoid false positives (AI, ML, OK, etc.)
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
  for (const [city, state] of Object.entries(CITY_TO_STATE)) {
    if (lower.includes(city)) {
      return state
    }
  }

  return null
}
```

**Acceptance criteria:**
- `resolveState("San Francisco, CA")` returns `"California"`.
- `resolveState("NYC")` returns `"New York"`.
- `resolveState("Austin, TX 78701")` returns `"Texas"`.
- `resolveState("Hybrid - Seattle, WA")` returns `"Washington"`.
- `resolveState("Remote (Denver, CO preferred)")` returns `"Colorado"`.
- `resolveState("Washington, DC")` returns `"District of Columbia"`.
- `resolveState("DC")` returns `"District of Columbia"`.
- `resolveState("Remote")` returns `null`.
- `resolveState("DOE")` returns `null`.
- `resolveState("")` returns `null`.
- `resolveState(null)` returns `null`.
- `resolveState(undefined)` returns `null`.
- `resolveState("California")` returns `"California"` (full state name match).
- `resolveState("new york")` returns `"New York"` (case-insensitive).
- `resolveState("AI Engineer")` returns `null` (NOT Arizona).
- `resolveState("Remote OK")` returns `null` (NOT Oklahoma).

**Failure criteria:**
- "AI Engineer" resolves to Arizona (regex not context-aware).
- "Remote OK" resolves to Oklahoma (regex not context-aware).
- "DC" fails to resolve (city mapping must cover standalone "dc").
- Case sensitivity causes "new york" to fail matching.

---

### T68.2: Write Choropleth Aggregation and Option Builder

**File:** `packages/webui/src/lib/components/charts/choropleth-utils.ts`

[IMPORTANT] Precompute `stateOrgsMap` during `aggregateByState` -- do NOT compute on every tooltip hover. The tooltip reads from `data.stateOrgsMap.get(stateName)` (O(1) lookup) instead of iterating all JDs (O(n) per hover). This map stores the top-5 organizations per state, sorted by count descending.

[IMPORTANT] `visualMap` dimensions are swapped for horizontal orientation: `itemWidth: 120` is the length (horizontal extent), `itemHeight: 15` is the thickness. ECharts named these for vertical orientation, so horizontal orientation requires swapping the conceptual meaning.

```typescript
import { resolveState } from './state-resolver'

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
  stateOrgsMap: Map<string, Array<{ name: string; count: number }>>
}

/**
 * Aggregate JDs by US state.
 * JDs with unresolvable locations go into the "unresolved" bucket.
 * Precomputes org breakdown per state for O(1) tooltip lookups.
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

/**
 * For a given state, get the top organizations (by JD count).
 * Note: For tooltip rendering, prefer ChoroplethData.stateOrgsMap
 * (precomputed during aggregation) to avoid O(n) on every hover.
 * This function is retained for the selected-state detail panel.
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

/**
 * Build ECharts option for the choropleth map.
 * Uses map series with continuous visualMap (indigo color scale).
 * Tooltip reads from precomputed stateOrgsMap for O(1) lookup.
 */
export function buildChoroplethOption(
  data: ChoroplethData,
): EChartsOption {
  const maxValue = Math.max(...data.stateCounts.map(s => s.value), 1)

  return {
    title: {
      text: 'JD Distribution by State',
      subtext: `${data.totalJDs} total \u2022 ${data.unresolvedCount} remote/unknown`,
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
      // NOTE: itemWidth and itemHeight names are swapped for horizontal orientation.
      // itemWidth: 120 is the length (horizontal extent), itemHeight: 15 is the thickness.
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
      },
    ],
  }
}
```

**Acceptance criteria:**
- `aggregateByState` groups JDs by resolved state.
- `aggregateByState` counts JDs per state correctly.
- `aggregateByState` puts "Remote" JDs in `unresolvedCount`.
- `aggregateByState` returns empty `stateCounts` for all-remote JDs.
- `aggregateByState` sorts `stateCounts` by value descending.
- `aggregateByState` includes JD titles in stateCount data.
- `aggregateByState` precomputes `stateOrgsMap` with top-5 orgs per state.
- `getStateOrgs` returns top organizations for a state sorted by count.
- `getStateOrgs` limits to 5 organizations.
- `getStateOrgs` uses "Unknown" for JDs with no organization name.
- `buildChoroplethOption` produces valid map series option.
- `buildChoroplethOption` includes `visualMap` with correct min/max.
- `buildChoroplethOption` subtitle shows total and unresolved counts.
- `buildChoroplethOption` tooltip reads from `stateOrgsMap` (O(1)).
- `visualMap` uses `itemWidth: 120, itemHeight: 15` for horizontal orientation.

**Failure criteria:**
- `stateOrgsMap` not precomputed (tooltip iterates all JDs on every hover).
- `visualMap` dimensions not swapped (visual bar renders as a tall narrow rectangle).
- `maxValue` is 0 when no states have JDs (divide by zero or visual glitch).

---

### T68.3: Write RoleChoropleth Component

**File:** `packages/webui/src/lib/components/charts/RoleChoropleth.svelte`

[CRITICAL] `mapLoaded` is at module level (`<script context="module">`), not component `$state`. `echarts.registerMap` is a global operation -- once registered, the map persists across component mount/unmount cycles. Using component-level `$state` would cause re-fetching and re-registering on every mount, wasting bandwidth and causing a brief flicker.

[IMPORTANT] Use `onMount` for data loading, not `$effect`. The component loads data once on mount -- no reactive props trigger reload. `$effect` with async state writes would risk infinite reactive loops.

```svelte
<script context="module" lang="ts">
  // mapLoaded is at module level so it persists across mount/unmount cycles.
  // ECharts' registerMap is global -- once registered, it doesn't need re-registering.
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

    // Separate error handling for GeoJSON vs JD API failures.
    // Use Promise.allSettled with specific error messages instead of
    // Promise.all which would collapse both failures into a single catch.
    try {
      const [geoResult, jdsResult] = await Promise.allSettled([
        loadGeoJSON(),
        forge.jobDescriptions.list({ limit: 500 }),
      ])

      if (geoResult.status === 'rejected') {
        error = 'Failed to load US map data. Check network connection.'
        return
      }

      if (jdsResult.status === 'rejected' || !jdsResult.value.ok) {
        error = 'Failed to load job descriptions'
        return
      }

      jds = jdsResult.value.data
      data = aggregateByState(jds)
    } catch {
      error = 'An unexpected error occurred loading map data'
    } finally {
      loading = false
    }
  }

  async function loadGeoJSON() {
    // Only load once -- check if map is already registered
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

<style>
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
</style>
```

**Acceptance criteria:**
- `mapLoaded` is at module level, persists across mount/unmount.
- GeoJSON is fetched only once (subsequent mounts reuse registered map).
- Component shows loading state while data is being fetched.
- Component shows empty state when no JDs exist.
- Component renders map when JDs exist.
- Component shows error state when GeoJSON or API fails.
- Clicking a state shows detail bar with count and "View JDs" link.
- Clicking "View JDs" navigates to JD list with `?location=` query parameter.
- Clicking the same state again deselects it.
- Unresolved badge shows count of Remote/Unknown JDs.
- Unresolved badge hidden when all JDs have resolvable locations.
- Chart uses `roam: true` for pan/zoom.
- Chart uses `notMerge={true}`.

**Failure criteria:**
- `mapLoaded` is component-level `$state` (re-fetches GeoJSON on every mount).
- `$effect` used instead of `onMount` (infinite reactive loop risk).
- GeoJSON fetch failure crashes the component instead of showing error state.

---

### T68.4: Obtain US States GeoJSON

**File:** `packages/webui/static/geo/us-states.json`

Obtain a simplified US states GeoJSON file from a public domain source (US Census Bureau cartographic boundaries or community repositories like d3-geo). The file must be a `FeatureCollection` where each `Feature` has `properties.name` set to the full state name (e.g., "California", "New York") for ECharts matching.

Recommended source: `https://raw.githubusercontent.com/PublicaMundi/MappingAPI/master/data/geojson/us-states.json`. Verify `properties.name` uses full state names (e.g., "California"). Add validation in T68.4 acceptance criteria: "GeoJSON has 51 features with `properties.name` matching `STATE_ABBR_TO_NAME` values."

Requirements:
- File size: ~300-500KB (simplified boundaries, not full-resolution)
- Format: GeoJSON `FeatureCollection`
- Each feature: `properties.name` = full state name
- Coverage: All 50 states + District of Columbia
- License: Public domain or compatible open-source license

**Acceptance criteria:**
- File exists at `packages/webui/static/geo/us-states.json`.
- File is valid GeoJSON (`FeatureCollection` with `features` array).
- Each feature has `properties.name` with full state name.
- All 50 states + DC are represented (51 features).
- File size is under 1MB (simplified boundaries).
- `echarts.registerMap('USA', geoJson)` succeeds without errors.
- GeoJSON has 51 features with `properties.name` matching `STATE_ABBR_TO_NAME` values.

**Failure criteria:**
- State names use abbreviations instead of full names (ECharts data matching fails).
- Missing states (map has blank regions).
- File too large (> 2MB) causing slow initial load.

---

### T68.5: Mount Component on Dashboard and Wire JD List Filter

**File (dashboard):** `packages/webui/src/routes/+page.svelte` (modify existing)

[IMPORTANT] The `RoleChoropleth` section must be placed inside the `{:else}` block in `+page.svelte` (after the loading/error guards).

```svelte
import RoleChoropleth from '$lib/components/charts/RoleChoropleth.svelte'

<!-- Inside the {:else} block, after other dashboard sections -->
<section class="section">
  <h2 class="section-title">Opportunity Map</h2>
  <div class="chart-card">
    <RoleChoropleth />
  </div>
</section>
```

**File (JD list):** `packages/webui/src/routes/opportunities/job-descriptions/+page.svelte` (modify existing)

Read `?location=` query parameter from the URL and pre-populate the location filter field:

```typescript
import { page } from '$app/stores'

// Initialize locationFilter reactively from URL params via $effect.
// Using `$state(locationParam ?? '')` would capture the initial value only —
// it would not update when the URL changes (e.g., browser back/forward).
let locationFilter = $state('')
$effect(() => {
  const param = $page.url.searchParams.get('location')
  if (param) locationFilter = param
})
```

**Acceptance criteria:**
- "Opportunity Map" section appears on dashboard inside `{:else}` block.
- `RoleChoropleth` component renders the choropleth map.
- Navigating to `/opportunities/job-descriptions?location=California` pre-populates the location filter.
- JD list shows only California JDs when filter is active.

**Failure criteria:**
- Section rendered outside `{:else}` block.
- `?location=` parameter ignored on JD list page.
- Import path incorrect.

---

### T68.6: Write State Resolver Unit Tests

**File:** `packages/webui/src/lib/components/charts/__tests__/state-resolver.test.ts`

```typescript
import { describe, it, expect } from 'vitest'
import { resolveState } from '../state-resolver'

describe('resolveState', () => {
  // Pattern 1: "City, ST" abbreviation after comma
  it('resolves "San Francisco, CA" to California', () => {
    expect(resolveState('San Francisco, CA')).toBe('California')
  })

  it('resolves "Austin, TX 78701" to Texas (with zip code)', () => {
    expect(resolveState('Austin, TX 78701')).toBe('Texas')
  })

  it('resolves "Hybrid - Seattle, WA" to Washington', () => {
    expect(resolveState('Hybrid - Seattle, WA')).toBe('Washington')
  })

  it('resolves "Remote (Denver, CO preferred)" to Colorado', () => {
    expect(resolveState('Remote (Denver, CO preferred)')).toBe('Colorado')
  })

  it('resolves "Washington, DC" to District of Columbia', () => {
    expect(resolveState('Washington, DC')).toBe('District of Columbia')
  })

  it('resolves "Reston, VA" to Virginia', () => {
    expect(resolveState('Reston, VA')).toBe('Virginia')
  })

  // Pattern 2: Full state name
  it('resolves "California" to California (full name)', () => {
    expect(resolveState('California')).toBe('California')
  })

  it('resolves "new york" to New York (case-insensitive)', () => {
    expect(resolveState('new york')).toBe('New York')
  })

  // Pattern 3: City mapping
  it('resolves "NYC" to New York', () => {
    expect(resolveState('NYC')).toBe('New York')
  })

  it('resolves "DC" to District of Columbia', () => {
    expect(resolveState('DC')).toBe('District of Columbia')
  })

  // Null/empty/non-geographic
  it('returns null for "Remote"', () => {
    expect(resolveState('Remote')).toBeNull()
  })

  it('returns null for "DOE"', () => {
    expect(resolveState('DOE')).toBeNull()
  })

  it('returns null for empty string', () => {
    expect(resolveState('')).toBeNull()
  })

  it('returns null for null', () => {
    expect(resolveState(null)).toBeNull()
  })

  it('returns null for undefined', () => {
    expect(resolveState(undefined)).toBeNull()
  })

  it('returns null for "Anywhere"', () => {
    expect(resolveState('Anywhere')).toBeNull()
  })

  // False positive protection
  it('returns null for "AI Engineer" (should NOT match Arizona)', () => {
    expect(resolveState('AI Engineer')).toBeNull()
  })

  it('returns null for "Remote OK" (should NOT match Oklahoma)', () => {
    expect(resolveState('Remote OK')).toBeNull()
  })

  // Standalone abbreviation edge case
  // resolveState('OK') returns 'Oklahoma' — standalone two-letter abbreviations
  // are resolved when they match a state. This is intentional: if a location is
  // literally 'OK', we interpret it as Oklahoma. If this causes false positives,
  // add an exclusion list in a future iteration.
  it('resolves standalone "OK" to Oklahoma (intentional)', () => {
    expect(resolveState('OK')).toBe('Oklahoma')
  })
})
```

**Acceptance criteria:**
- All 19 test cases pass.
- Abbreviation, full name, and city patterns all verified.
- False positive protection for "AI Engineer" and "Remote OK" verified.
- Standalone "OK" resolves to Oklahoma (intentional behavior documented).
- Null/empty/undefined edge cases verified.

**Failure criteria:**
- "AI Engineer" resolves to Arizona (regex not context-aware).
- "Remote OK" resolves to Oklahoma (regex not context-aware).
- Any other test fails.

---

### T68.7: Write Choropleth Aggregation Unit Tests

**File:** `packages/webui/src/lib/components/charts/__tests__/choropleth-utils.test.ts`

```typescript
import { describe, it, expect } from 'vitest'
import {
  aggregateByState,
  getStateOrgs,
  buildChoroplethOption,
} from '../choropleth-utils'

describe('aggregateByState', () => {
  it('groups JDs by resolved state', () => {
    const jds = [
      { id: '1', title: 'Job A', location: 'San Francisco, CA', organization_name: 'Acme' },
      { id: '2', title: 'Job B', location: 'NYC', organization_name: 'BigCo' },
      { id: '3', title: 'Job C', location: 'Los Angeles, CA', organization_name: 'Acme' },
    ]
    const data = aggregateByState(jds as any)
    const ca = data.stateCounts.find(s => s.name === 'California')!
    expect(ca.value).toBe(2)
    const ny = data.stateCounts.find(s => s.name === 'New York')!
    expect(ny.value).toBe(1)
  })

  it('counts JDs per state correctly', () => {
    const jds = [
      { id: '1', title: 'A', location: 'Austin, TX', organization_name: null },
      { id: '2', title: 'B', location: 'Dallas, TX', organization_name: null },
      { id: '3', title: 'C', location: 'Houston, TX', organization_name: null },
    ]
    const data = aggregateByState(jds as any)
    expect(data.stateCounts[0].value).toBe(3)
    expect(data.stateCounts[0].name).toBe('Texas')
  })

  it('puts "Remote" JDs in unresolvedCount', () => {
    const jds = [
      { id: '1', title: 'A', location: 'Remote', organization_name: null },
      { id: '2', title: 'B', location: 'San Francisco, CA', organization_name: null },
    ]
    const data = aggregateByState(jds as any)
    expect(data.unresolvedCount).toBe(1)
    expect(data.unresolvedJDs).toEqual(['A'])
  })

  it('returns empty stateCounts for all-remote JDs', () => {
    const jds = [
      { id: '1', title: 'A', location: 'Remote', organization_name: null },
      { id: '2', title: 'B', location: 'Anywhere', organization_name: null },
    ]
    const data = aggregateByState(jds as any)
    expect(data.stateCounts).toHaveLength(0)
    expect(data.unresolvedCount).toBe(2)
  })

  it('sorts stateCounts by value descending', () => {
    const jds = [
      { id: '1', title: 'A', location: 'NYC', organization_name: null },
      { id: '2', title: 'B', location: 'San Francisco, CA', organization_name: null },
      { id: '3', title: 'C', location: 'Los Angeles, CA', organization_name: null },
    ]
    const data = aggregateByState(jds as any)
    expect(data.stateCounts[0].name).toBe('California')
    expect(data.stateCounts[0].value).toBe(2)
  })

  it('includes JD titles in stateCount data', () => {
    const jds = [
      { id: '1', title: 'Engineer', location: 'Austin, TX', organization_name: null },
    ]
    const data = aggregateByState(jds as any)
    expect(data.stateCounts[0].jds).toEqual(['Engineer'])
  })

  it('precomputes stateOrgsMap with org names and counts', () => {
    const jds = [
      { id: '1', title: 'A', location: 'San Francisco, CA', organization_name: 'Acme' },
      { id: '2', title: 'B', location: 'Los Angeles, CA', organization_name: 'Acme' },
      { id: '3', title: 'C', location: 'San Jose, CA', organization_name: 'BigCo' },
    ]
    const data = aggregateByState(jds as any)
    const caOrgs = data.stateOrgsMap.get('California')!
    expect(caOrgs).toBeDefined()
    expect(caOrgs[0]).toEqual({ name: 'Acme', count: 2 })
    expect(caOrgs[1]).toEqual({ name: 'BigCo', count: 1 })
  })
})

describe('getStateOrgs', () => {
  it('returns top organizations sorted by count', () => {
    const jds = [
      { id: '1', title: 'A', location: 'San Francisco, CA', organization_name: 'Acme' },
      { id: '2', title: 'B', location: 'Los Angeles, CA', organization_name: 'Acme' },
      { id: '3', title: 'C', location: 'San Jose, CA', organization_name: 'BigCo' },
    ]
    const orgs = getStateOrgs(jds as any, 'California')
    expect(orgs[0]).toEqual({ name: 'Acme', count: 2 })
    expect(orgs[1]).toEqual({ name: 'BigCo', count: 1 })
  })

  it('limits to 5 organizations', () => {
    const jds = Array.from({ length: 10 }, (_, i) => ({
      id: `${i}`, title: `Job ${i}`, location: 'Austin, TX',
      organization_name: `Org${i}`,
    }))
    const orgs = getStateOrgs(jds as any, 'Texas')
    expect(orgs.length).toBeLessThanOrEqual(5)
  })

  it('uses "Unknown" for JDs with no organization name', () => {
    const jds = [
      { id: '1', title: 'A', location: 'NYC', organization_name: null },
    ]
    const orgs = getStateOrgs(jds as any, 'New York')
    expect(orgs[0].name).toBe('Unknown')
  })
})

describe('buildChoroplethOption', () => {
  it('produces valid map series option', () => {
    const data = {
      stateCounts: [{ name: 'California', value: 3, jds: [] }],
      unresolvedCount: 1,
      unresolvedJDs: [],
      totalJDs: 4,
      stateOrgsMap: new Map(),
    }
    const option = buildChoroplethOption(data)
    expect(option.series[0].type).toBe('map')
    expect(option.series[0].map).toBe('USA')
  })

  it('includes visualMap with correct min/max', () => {
    const data = {
      stateCounts: [
        { name: 'California', value: 5, jds: [] },
        { name: 'Texas', value: 3, jds: [] },
      ],
      unresolvedCount: 0,
      unresolvedJDs: [],
      totalJDs: 8,
      stateOrgsMap: new Map(),
    }
    const option = buildChoroplethOption(data)
    expect(option.visualMap.min).toBe(0)
    expect(option.visualMap.max).toBe(5)
  })

  it('subtitle shows total and unresolved counts', () => {
    const data = {
      stateCounts: [],
      unresolvedCount: 3,
      unresolvedJDs: [],
      totalJDs: 10,
      stateOrgsMap: new Map(),
    }
    const option = buildChoroplethOption(data)
    expect(option.title.subtext).toContain('10 total')
    expect(option.title.subtext).toContain('3 remote/unknown')
  })
})
```

**Acceptance criteria:**
- All 13 test cases pass.
- State grouping, counting, sorting, unresolved handling verified.
- Org aggregation, limit, and stateOrgsMap precomputation verified.
- Option structure validated.

**Failure criteria:**
- Any test fails, indicating a bug in aggregation or option building.

---

## Testing

### State Resolver Unit Tests

**File:** `packages/webui/src/lib/components/charts/__tests__/state-resolver.test.ts` (T68.6)

| Test | Assertion |
|------|-----------|
| "San Francisco, CA" | `"California"` |
| "Austin, TX 78701" | `"Texas"` |
| "Hybrid - Seattle, WA" | `"Washington"` |
| "Remote (Denver, CO preferred)" | `"Colorado"` |
| "Washington, DC" | `"District of Columbia"` |
| "Reston, VA" | `"Virginia"` |
| "California" | `"California"` (full name) |
| "new york" | `"New York"` (case-insensitive) |
| "NYC" | `"New York"` (city mapping) |
| "DC" | `"District of Columbia"` (city mapping) |
| "Remote" | `null` |
| "DOE" | `null` |
| "" | `null` |
| `null` | `null` |
| `undefined` | `null` |
| "Anywhere" | `null` |
| "AI Engineer" | `null` (NOT Arizona) |
| "Remote OK" | `null` (NOT Oklahoma) |
| "OK" (standalone) | `"Oklahoma"` (intentional -- document as known behavior) |

### Aggregation Unit Tests

**File:** `packages/webui/src/lib/components/charts/__tests__/choropleth-utils.test.ts` (T68.7)

| Test | Assertion |
|------|-----------|
| Groups JDs by state | CA: 2, NY: 1 |
| Counts per state | TX: 3 |
| Remote -> unresolvedCount | `unresolvedCount: 1` |
| All remote -> empty stateCounts | `stateCounts: []` |
| Sorts by value descending | CA (2) before NY (1) |
| Includes JD titles | `jds: ['Engineer']` |
| stateOrgsMap precomputed | `stateOrgsMap.get('California')` has Acme (2), BigCo (1) |
| getStateOrgs sorted by count | Acme (2) before BigCo (1) |
| getStateOrgs limits to 5 | `orgs.length <= 5` |
| getStateOrgs "Unknown" fallback | Null org -> "Unknown" |
| Option has map series | `type: 'map'`, `map: 'USA'` |
| visualMap min/max | `min: 0`, `max: 5` |
| Subtitle counts | "10 total", "3 remote/unknown" |

### Component Tests (Manual / Future)

| Test | What to verify |
|------|---------------|
| Loading state | "Loading map data..." appears |
| Empty state | "No job descriptions yet" message |
| Map renders | EChart div with map series |
| Error state (GeoJSON) | Error message when fetch fails |
| Error state (API) | Error message when JD API fails |
| State click | Detail bar with count and "View JDs" link |
| State click navigation | `/opportunities/job-descriptions?location=<state>` |
| State deselect | Click same state -> detail bar hidden |
| Unresolved badge visible | Count shown for remote/unknown JDs |
| Unresolved badge hidden | Not shown when all JDs have locations |
| Pan/zoom | `roam: true` enables native ECharts pan/zoom |
| `notMerge` | Clean chart updates |

### Integration Tests

| Test | What to verify |
|------|---------------|
| 3 JDs (CA, NYC, Remote) | CA: 1, NY: 1, unresolved: 1 |
| Click CA -> detail bar | "California: 1 JD", "View JDs" link |
| Navigate -> JD list | `?location=California` pre-populates filter |

---

## Documentation Requirements

- No new documentation files required.
- The spec file serves as the design document.
- This plan file serves as the implementation reference.
- Inline TSDoc comments on all exported interfaces and functions:
  - `STATE_ABBR_TO_NAME`, `STATE_NAME_TO_ABBR`, `CITY_TO_STATE`: mapping coverage
  - `resolveState`: matching strategy, false positive protection
  - `StateCount`, `ChoroplethData`: field semantics
  - `aggregateByState`: precomputed `stateOrgsMap` for O(1) tooltip
  - `getStateOrgs`: retained for detail panel, tooltip should use `stateOrgsMap`
  - `buildChoroplethOption`: visualMap dimension swap, tooltip formatter
- Inline comments in `RoleChoropleth.svelte`:
  - Module-level `mapLoaded` rationale
  - `onMount` vs `$effect` rationale
  - `componentType` guard in click handler

---

## Parallelization Notes

**Within this phase:**
- T68.1 (state resolver) is foundational -- no dependencies.
- T68.2 (choropleth utils) depends on T68.1 (imports `resolveState`).
- T68.3 (component) depends on T68.2 (imports utils).
- T68.4 (GeoJSON) is independent -- can be done in parallel with everything.
- T68.5 (dashboard/JD list mounting) depends on T68.3 (component must exist).
- T68.6 (resolver tests) depends on T68.1 (tests the resolver).
- T68.7 (utils tests) depends on T68.2 (tests the utils).

**Recommended execution order:**
1. T68.1 (state resolver) + T68.4 (GeoJSON) -- parallel, no dependencies
2. T68.2 (choropleth utils) + T68.6 (resolver tests) -- parallel, both depend on T68.1
3. T68.3 (component) + T68.7 (utils tests) -- parallel, both depend on T68.2
4. T68.5 (mounting) -- depends on T68.3

**Cross-phase:**
- Phase 67 and 68 both modify `+page.svelte` (dashboard) -- different sections, coordinated merge needed.
- Phase 68 does not modify `echarts-registry.ts` (MapChart and VisualMapComponent already registered by Phase 59).
- Phases 65 and 66 both modify `echarts-registry.ts`. `echarts.use()` is idempotent -- registering `BarChart` twice is safe. If Phase 66 lands before Phase 65, uncomment the `BarChart` import; if 65 lands first, leave it commented.
