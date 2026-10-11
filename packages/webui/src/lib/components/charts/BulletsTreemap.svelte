<!--
  BulletsTreemap.svelte — Bullets treemap: bullets sized by perspective count,
  grouped by primary source. Perspective counts are built client-side from the
  perspectives endpoint because the Bullet SDK type may not include perspective_count.

  Data fetching uses onMount (NOT $effect) to avoid infinite reactive loops.
-->
<script lang="ts">
  import { onMount } from 'svelte'
  import { forge } from '$lib/sdk'
  import { listAll } from '$lib/list-all'
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
      listAll((params) => forge.bullets.list(params)),
      listAll((params) => forge.perspectives.list(params)),
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
  <EmptyState title="No bullet data" description="Add bullets to see the treemap visualization." />
{:else}
  <div class="chart-card">
    <EChart {option} height="450px" notMerge={true} />
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
