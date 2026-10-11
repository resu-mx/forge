<!--
  JDSkillRadar.svelte -- Radar (spider) chart showing skill alignment
  between user skills (from bullet technologies) and JD-required skills.

  Falls back to a horizontal bar chart when fewer than 3 skill categories
  exist (radar with <3 axes is uninformative). Shows a gap list of
  unmatched skills grouped by category below the chart.
-->
<script lang="ts">
  import { forge } from '$lib/sdk'
  import { listAll } from '$lib/list-all'
  import EChart from './EChart.svelte'
  import {
    computeJDSkillAlignment,
    buildRadarOption,
    buildFallbackBarOption,
  } from './jd-radar-utils'
  import type { JDSkillAlignment } from './jd-radar-utils'

  let { jdId }: { jdId: string } = $props()

  let loading = $state(true)
  let alignment = $state<JDSkillAlignment | null>(null)
  let error = $state<string | null>(null)

  // Reactive: reload when jdId changes
  $effect(() => {
    if (!jdId) return
    loadAlignment(jdId)
  })

  async function loadAlignment(id: string) {
    loading = true
    error = null

    try {
      const [jdSkillsRes, allSkillsRes, bulletsRes] = await Promise.all([
        forge.jobDescriptions.listSkills(id),
        forge.skills.list({ limit: 500 }),
        listAll((params) => forge.bullets.list(params)),
      ])

      if (!jdSkillsRes.ok || !allSkillsRes.ok || !bulletsRes.ok) {
        error = 'Failed to load skill data'
        return
      }

      alignment = computeJDSkillAlignment(
        jdSkillsRes.data,
        allSkillsRes.data,
        bulletsRes.data,
      )
    } catch (e) {
      error = 'An error occurred loading skill alignment'
    } finally {
      loading = false
    }
  }

  let chartOption = $derived(
    alignment
      ? alignment.categories.length >= 3
        ? buildRadarOption(alignment)
        : buildFallbackBarOption(alignment)
      : null
  )
</script>

{#if loading}
  <div class="radar-loading">Loading skill alignment...</div>
{:else if error}
  <div class="radar-error">{error}</div>
{:else if alignment && alignment.totalJDSkills === 0}
  <div class="radar-empty">
    No required skills tagged on this JD. Add skills to see alignment.
  </div>
{:else if chartOption}
  <div class="radar-container">
    <div class="radar-header">
      <h3>Skill Alignment</h3>
      <span class="radar-summary">
        {alignment!.totalMatched} of {alignment!.totalJDSkills} required skills matched
      </span>
    </div>
    <EChart
      option={chartOption}
      height="400px"
      notMerge={true}
    />
    {#if alignment!.categories.some(c => c.gapSkills.length > 0)}
      <div class="gap-list">
        <h4>Skill Gaps</h4>
        {#each alignment!.categories.filter(c => c.gapSkills.length > 0) as cat}
          <div class="gap-category">
            <span class="gap-category-name">{cat.category}</span>
            <span class="gap-skills">
              {#each cat.gapSkills as skill}
                <span class="gap-pill">{skill}</span>
              {/each}
            </span>
          </div>
        {/each}
      </div>
    {/if}
  </div>
{/if}

<style>
  .radar-loading,
  .radar-error,
  .radar-empty {
    padding: 1.5rem;
    text-align: center;
    font-size: var(--text-sm, 0.875rem);
    color: var(--text-muted);
    border: 1px dashed var(--color-border);
    border-radius: 0.5rem;
    margin: 0.75rem 0;
  }

  .radar-error {
    color: var(--color-danger);
    border-color: var(--color-danger);
  }

  .radar-container {
    margin: 0.75rem 0;
  }

  .radar-header {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    margin-bottom: 0.5rem;
  }

  .radar-header h3 {
    font-size: var(--text-base, 1rem);
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
  }

  .radar-summary {
    font-size: var(--text-sm, 0.875rem);
    color: var(--text-secondary);
  }

  .gap-list {
    margin-top: 1rem;
    padding: 0.75rem;
    background: var(--color-surface-raised, var(--color-surface));
    border: 1px solid var(--color-border);
    border-radius: 0.5rem;
  }

  .gap-list h4 {
    font-size: var(--text-sm, 0.875rem);
    font-weight: 600;
    color: var(--text-primary);
    margin: 0 0 0.5rem 0;
  }

  .gap-category {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
  }

  .gap-category:last-child {
    margin-bottom: 0;
  }

  .gap-category-name {
    font-size: var(--text-xs, 0.75rem);
    font-weight: 600;
    color: var(--text-secondary);
    min-width: 100px;
    text-transform: capitalize;
  }

  .gap-skills {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
  }

  .gap-pill {
    display: inline-block;
    padding: 0.15rem 0.5rem;
    font-size: var(--text-xs, 0.75rem);
    background: var(--color-danger-subtle, rgba(239, 68, 68, 0.1));
    color: var(--color-danger);
    border-radius: 1rem;
  }
</style>
