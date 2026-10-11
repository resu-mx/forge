<!--
  SkillsTreemap.svelte — Skills treemap: skills sized by bullet technology count,
  grouped by category. Uses ECharts' built-in nodeClick: 'zoomToNode' for drill-down.

  Data fetching uses onMount (NOT $effect) to avoid infinite reactive loops.
-->
<script lang="ts">
  import { onMount } from 'svelte'
  import { forge } from '$lib/sdk'
  import { listAll } from '$lib/list-all'
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
      listAll((params) => forge.bullets.list(params)),
    ])
    if (s.ok) skills = s.data
    if (b.ok) bullets = b.data
    loading = false
  }
</script>

{#if loading}
  <LoadingSpinner message="Loading skills data..." />
{:else if !hasData}
  <EmptyState title="No skills data" description="Add skills to see the treemap visualization." />
{:else}
  <div class="chart-card">
    <EChart {option} height="400px" notMerge={true} />
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
