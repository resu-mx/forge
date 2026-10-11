<!--
  ApplicationGantt.svelte — Gantt-style chart showing JD application timelines.

  Each row is a JD with a horizontal bar from created_at to updated_at (or "now"
  for active JDs), colored by pipeline status. Active JDs have a dashed right edge.
  Supports Y-axis scrolling via dataZoom when > 10 JDs exist.
  Clicking a bar navigates to the JD detail page.
-->
<script lang="ts">
  import { onMount } from 'svelte'
  import { forge } from '$lib/sdk'
  import { listAll } from '$lib/list-all'
  import { goto } from '$app/navigation'
  import EChart from './EChart.svelte'
  import { buildGanttItems, buildGanttOption, STATUS_COLORS } from './gantt-utils'
  import type { GanttItem } from './gantt-utils'

  let loading = $state(true)
  let items = $state<GanttItem[]>([])
  let error = $state<string | null>(null)

  // Use onMount instead of $effect to avoid infinite reactive loops:
  // loadData() writes to reactive state (items, loading, error)
  // which $effect would re-track, causing re-execution.
  onMount(() => { loadData() })

  async function loadData() {
    loading = true
    error = null
    try {
      const result = await listAll((params) => forge.jobDescriptions.list(params))
      if (result.ok) {
        items = buildGanttItems(result.data)
      } else {
        error = 'Failed to load job descriptions'
      }
    } catch {
      error = 'An error occurred loading application data'
    } finally {
      loading = false
    }
  }

  let chartOption = $derived(
    items.length > 0 ? buildGanttOption(items) : null
  )

  // Dynamic height: min 200px, scales with item count, capped at ~580px (10 items)
  let chartHeight = $derived(
    `${Math.max(200, Math.min(items.length, 10) * 50 + 80)}px`
  )

  function handleChartClick(params: any) {
    // Use params.value[0] (explicit category index) for robust item lookup
    const item = items[params.value?.[0]]
    if (item) {
      goto(`/opportunities/job-descriptions?selected=${item.id}`)
    }
  }
</script>

{#if loading}
  <div class="gantt-loading">Loading application timeline...</div>
{:else if error}
  <div class="gantt-error">{error}</div>
{:else if items.length === 0}
  <div class="gantt-empty">
    No job descriptions yet. Create one to start tracking your application timeline.
  </div>
{:else}
  <div class="gantt-container">
    <EChart
      option={chartOption!}
      height={chartHeight}
      notMerge={true}
      onChartEvent={{ click: handleChartClick }}
    />
    <div class="gantt-legend">
      {#each Object.entries(STATUS_COLORS) as [status, color]}
        <span class="legend-item">
          <span class="legend-swatch" style:background={color}></span>
          {status}
        </span>
      {/each}
    </div>
  </div>
{/if}

<style>
  .gantt-loading,
  .gantt-empty {
    text-align: center;
    padding: 2rem 1rem;
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  .gantt-error {
    text-align: center;
    padding: 2rem 1rem;
    color: var(--color-danger-text);
    font-size: var(--text-sm);
  }

  .gantt-legend {
    display: flex;
    flex-wrap: wrap;
    gap: 0.75rem;
    justify-content: center;
    padding: 0.5rem 0;
    font-size: 11px;
    color: var(--text-muted);
  }

  .legend-item {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .legend-swatch {
    display: inline-block;
    width: 12px;
    height: 12px;
    border-radius: 2px;
  }
</style>
