<!--
  RoleChoropleth.svelte — Choropleth map showing JD distribution by US state.

  Color intensity represents JD count per state via continuous indigo visualMap.
  Clicking a state shows a detail bar with count and "View JDs" link.
  Unresolved (Remote/Unknown) JDs are shown in a badge overlay.

  The mapLoaded flag is at module level to avoid re-registering GeoJSON
  across mount/unmount cycles (echarts.registerMap is a global operation).
-->
<script module lang="ts">
  // mapLoaded is at module level so it persists across mount/unmount cycles.
  // ECharts' registerMap is global -- once registered, it doesn't need re-registering.
  let mapLoaded = false
</script>

<script lang="ts">
  import { onMount } from 'svelte'
  import { goto } from '$app/navigation'
  import { forge } from '$lib/sdk'
  import { listAll } from '$lib/list-all'
  import { echarts } from './echarts-registry'
  import EChart from './EChart.svelte'
  import { aggregateByState, buildChoroplethOption } from './choropleth-utils'
  import type { ChoroplethData } from './choropleth-utils'

  let loading = $state(true)
  let data = $state<ChoroplethData | null>(null)
  let error = $state<string | null>(null)
  let selectedState = $state<string | null>(null)

  // Use onMount instead of $effect to avoid infinite reactive loops:
  // loadAll() writes to reactive state (data, loading, error)
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
        listAll((params) => forge.jobDescriptions.list(params)),
      ])

      if (geoResult.status === 'rejected') {
        error = 'Failed to load US map data. Check network connection.'
        return
      }

      if (jdsResult.status === 'rejected' || !jdsResult.value.ok) {
        error = 'Failed to load job descriptions'
        return
      }

      data = aggregateByState(jdsResult.value.data)
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

    {#if data.remoteCount > 0}
      <div class="unresolved-badge remote-badge">
        <span class="unresolved-count">{data.remoteCount}</span>
        Remote
      </div>
    {/if}
    {#if data.unknownCount > 0}
      <div class="unresolved-badge unknown-badge">
        <span class="unresolved-count">{data.unknownCount}</span>
        Unknown location
      </div>
    {/if}
  </div>
{/if}

<style>
  .choropleth-loading,
  .choropleth-empty {
    text-align: center;
    padding: 2rem 1rem;
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  .choropleth-error {
    text-align: center;
    padding: 2rem 1rem;
    color: var(--color-danger-text);
    font-size: var(--text-sm);
  }

  .choropleth-container {
    position: relative;
  }

  .state-detail {
    text-align: center;
    padding: 0.5rem;
    font-size: 13px;
    color: var(--text-secondary);
  }

  .link-button {
    background: none;
    border: none;
    color: var(--color-primary);
    text-decoration: underline;
    cursor: pointer;
    font-size: 13px;
  }

  .link-button:hover {
    color: var(--color-primary-hover);
  }

  .unresolved-badge {
    position: absolute;
    right: 20px;
    background: var(--color-ghost);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    padding: 4px 10px;
    font-size: 11px;
    color: var(--text-muted);
  }

  .remote-badge {
    bottom: 85px;
  }

  .unknown-badge {
    bottom: 60px;
  }

  .unresolved-count {
    font-weight: 700;
    color: var(--text-primary);
    margin-right: 4px;
  }
</style>
