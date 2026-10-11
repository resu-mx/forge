<!--
  DomainsTreemap.svelte — Domains treemap: domains sized by perspective count.
  Single-level treemap with no drill-down (nodeClick: false), no breadcrumb.

  This component fetches Perspective[], NOT Bullet[]. The domain field exists
  on Perspective, not Bullet.

  Data fetching uses onMount (NOT $effect) to avoid infinite reactive loops.
-->
<script lang="ts">
  import { onMount } from 'svelte'
  import { forge } from '$lib/sdk'
  import { listAll } from '$lib/list-all'
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
    const p = await listAll((params) => forge.perspectives.list(params))
    if (p.ok) perspectives = p.data
    loading = false
  }
</script>

{#if loading}
  <LoadingSpinner message="Loading domain data..." />
{:else if !hasData}
  <EmptyState title="No domain data" description="Add perspectives to see domain coverage." />
{:else}
  <div class="chart-card">
    <EChart {option} height="350px" notMerge={true} />
  </div>
{/if}

<style>
  .chart-card {
    background: var(--color-surface, #fff);
    border: 1px solid var(--color-border, #e5e7eb);
    border-radius: var(--radius-lg, 8px);
    padding: 1rem;
  }
</style>
