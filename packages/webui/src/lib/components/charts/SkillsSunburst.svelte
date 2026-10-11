<!--
  SkillsSunburst.svelte — Combined skills sunburst + domain-archetype breakout charts.

  The skills sunburst shows every skill grouped by category with usage-based sizing
  and supports drill-down to single-category pie views. The domain-archetype nested
  pie shows how perspectives distribute across domains and archetypes.

  Data fetching uses onMount (NOT $effect) to avoid infinite reactive loops.
-->
<script lang="ts">
  import { onMount } from 'svelte'
  import { forge } from '$lib/sdk'
  import { listAll } from '$lib/list-all'
  import EChart from './EChart.svelte'
  import { LoadingSpinner, EmptyState } from '$lib/components'
  import type { Skill, Bullet, Perspective } from '@forge/sdk'
  import {
    buildSkillsSunburstData,
    buildSunburstOption,
    buildDrillDownOption,
    getCategoryChildren,
    buildDomainArchetypeData,
    buildDomainArchetypeOption,
  } from './skills-chart-utils'

  // ── State ───────────────────────────────────────────────────────────
  let loading = $state(true)
  let skills = $state<Skill[]>([])
  let bullets = $state<Bullet[]>([])
  let perspectives = $state<Perspective[]>([])

  // Drill-down state for sunburst
  let drillCategory = $state<string | null>(null)

  // Empty data guard
  let hasData = $derived(skills.length > 0)

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
  // Use onMount instead of $effect to avoid infinite reactive loops
  onMount(() => { loadData() })

  async function loadData() {
    loading = true
    const [s, b, p] = await Promise.all([
      forge.skills.list({ limit: 500 }),
      listAll((params) => forge.bullets.list(params)),
      listAll((params) => forge.perspectives.list(params)),
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

<div class="skills-charts">
  {#if loading}
    <LoadingSpinner size="lg" message="Loading skill data..." />
  {:else if !hasData}
    <EmptyState
      title="No skill data"
      description="Add skills to sources and derive bullets to see distributions."
    />
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

<style>
  .skills-charts {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  .chart-card {
    background: var(--color-surface, #fff);
    border: 1px solid var(--color-border, #e5e7eb);
    border-radius: var(--radius-lg, 8px);
    padding: 1rem;
  }
</style>
