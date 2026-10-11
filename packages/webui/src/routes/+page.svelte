<script lang="ts">
  import { forge, friendlyError } from '$lib/sdk'
  import { LoadingSpinner, EmptyState, PageHeader } from '$lib/components'
  import { goto } from '$app/navigation'
  import SkillsSunburst from '$lib/components/charts/SkillsSunburst.svelte'
  import SkillsTreemap from '$lib/components/charts/SkillsTreemap.svelte'
  import BulletsTreemap from '$lib/components/charts/BulletsTreemap.svelte'
  import DomainsTreemap from '$lib/components/charts/DomainsTreemap.svelte'
  import ApplicationGantt from '$lib/components/charts/ApplicationGantt.svelte'
  import RoleChoropleth from '$lib/components/charts/RoleChoropleth.svelte'

  let loading = $state(true)
  let error = $state<string | null>(null)

  let pendingBullets = $state(0)
  let pendingPerspectives = $state(0)
  let totalSources = $state(0)
  let totalBullets = $state(0)
  let totalPerspectives = $state(0)
  let totalOrganizations = $state(0)
  let totalResumes = $state(0)
  let driftCount = $state(0)

  let nothingPending = $derived(pendingBullets === 0 && pendingPerspectives === 0)

  $effect(() => {
    loadData()
  })

  async function loadData() {
    loading = true
    error = null
    try {
      const [review, sources, bullets, perspectives, orgs, resumesList, drift] = await Promise.all([
        forge.review.pending(),
        forge.sources.list({ limit: 1 }),
        forge.bullets.list({ limit: 1 }),
        forge.perspectives.list({ limit: 1 }),
        forge.organizations.list({ limit: 1 }),
        forge.resumes.list({ limit: 1 }),
        forge.integrity.drift(),
      ])

      // Check if API is reachable
      const allFailed = !review.ok && !sources.ok && !bullets.ok && !perspectives.ok
      if (allFailed) {
        error = friendlyError(!review.ok ? review.error : { code: 'UNKNOWN_ERROR', message: 'Unknown error' })
        return
      }

      if (review.ok) {
        pendingBullets = review.data.bullets.count
        pendingPerspectives = review.data.perspectives.count
      }

      if (sources.ok) {
        totalSources = sources.pagination.total
      }

      if (bullets.ok) {
        totalBullets = bullets.pagination.total
      }

      if (perspectives.ok) {
        totalPerspectives = perspectives.pagination.total
      }

      if (orgs.ok) {
        totalOrganizations = orgs.pagination.total
      }

      if (resumesList.ok) {
        totalResumes = resumesList.pagination.total
      }

      if (drift.ok) {
        // drift.data may be a flat array or a grouped object — handle both
        if (Array.isArray(drift.data)) {
          driftCount = drift.data.length
        } else if (drift.data && typeof drift.data === 'object') {
          driftCount = (drift.data.bullets?.length ?? 0) + (drift.data.perspectives?.length ?? 0) + (drift.data.resume_entries?.length ?? 0)
        }
      }
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load dashboard data'
    } finally {
      loading = false
    }
  }
</script>

<div class="dashboard">
  <PageHeader title="Dashboard" />

  {#if loading}
    <div class="loading-container">
      <LoadingSpinner size="lg" message="Loading dashboard..." />
    </div>
  {:else if error}
    <div class="error-banner">
      <p>{error}</p>
      <button class="retry-btn" onclick={loadData}>Retry</button>
    </div>
  {:else}
    <!-- Pending Review Cards -->
    <section class="section">
      <h2 class="section-title">Pending Review</h2>

      {#if nothingPending}
        <EmptyState
          title="All caught up!"
          description="No bullets or perspectives are waiting for review."
        >
          <button class="btn btn-primary" onclick={() => goto('/experience/roles')}>View Sources</button>
        </EmptyState>
      {:else}
        <div class="pending-grid">
          <a href="/data/bullets?tab=bullets" class="pending-card">
            <div class="pending-count">{pendingBullets}</div>
            <div class="pending-label">Pending Bullets</div>
            <div class="pending-hint">Click to review</div>
          </a>
          <a href="/data/bullets?tab=perspectives" class="pending-card">
            <div class="pending-count">{pendingPerspectives}</div>
            <div class="pending-label">Pending Perspectives</div>
            <div class="pending-hint">Click to review</div>
          </a>
        </div>
      {/if}
    </section>

    <!-- Integrity Alerts -->
    <section class="section">
      <h2 class="section-title">Integrity Alerts</h2>

      {#if driftCount === 0}
        <div class="integrity-ok">
          <p>All snapshots are current. No drift detected.</p>
        </div>
      {:else}
        <a href="/chain" class="alert-card">
          <div class="alert-count">{driftCount}</div>
          <div class="alert-label">Drifted Entities</div>
          <div class="alert-hint">Snapshots no longer match current content. Click to view in Chain View.</div>
        </a>
      {/if}
    </section>

    <!-- Quick Stats -->
    <section class="section">
      <h2 class="section-title">Quick Stats</h2>
      <div class="stats-grid">
        <div class="stat-card">
          <div class="stat-number">{totalSources}</div>
          <div class="stat-label">Sources</div>
        </div>
        <div class="stat-card">
          <div class="stat-number">{totalBullets}</div>
          <div class="stat-label">Bullets</div>
        </div>
        <div class="stat-card">
          <div class="stat-number">{totalPerspectives}</div>
          <div class="stat-label">Perspectives</div>
        </div>
        <div class="stat-card">
          <div class="stat-number">{totalOrganizations}</div>
          <div class="stat-label">Organizations</div>
        </div>
        <div class="stat-card">
          <div class="stat-number">{totalResumes}</div>
          <div class="stat-label">Resumes</div>
        </div>
      </div>
    </section>

    <!-- Skill & Domain Distribution (Phase 63) -->
    <section class="section">
      <h2 class="section-title">Skill & Domain Distribution</h2>
      <SkillsSunburst />
    </section>

    <!-- Treemap Views (Phase 64) -->
    <section class="section">
      <h2 class="section-title">Treemap Views</h2>
      <div class="treemap-grid">
        <SkillsTreemap />
        <BulletsTreemap />
        <DomainsTreemap />
      </div>
    </section>

    <!-- Application Timeline (Phase 67) -->
    <section class="section">
      <h2 class="section-title">Application Timeline</h2>
      <div class="chart-card">
        <ApplicationGantt />
      </div>
    </section>

    <!-- Opportunity Map (Phase 68) -->
    <section class="section">
      <h2 class="section-title">Opportunity Map</h2>
      <div class="chart-card">
        <RoleChoropleth />
      </div>
    </section>
  {/if}
</div>

<style>
  .dashboard {
    max-width: 900px;
  }

  .loading-container {
    display: flex;
    justify-content: center;
    padding: 4rem 0;
  }

  .error-banner {
    background: var(--color-danger-subtle);
    border: 1px solid var(--color-danger);
    border-radius: var(--radius-lg);
    padding: 1.25rem 1.5rem;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    color: var(--color-danger-text);
    font-size: var(--text-base);
  }

  .retry-btn {
    padding: 0.4rem 1rem;
    background: var(--color-danger);
    color: var(--text-inverse);
    border: none;
    border-radius: var(--radius-md);
    font-size: var(--text-sm);
    font-weight: var(--font-medium);
    cursor: pointer;
    white-space: nowrap;
    transition: background 0.15s;
  }

  .retry-btn:hover {
    background: var(--color-danger-hover);
  }

  .section {
    margin-bottom: 2rem;
  }

  .section-title {
    font-size: var(--text-sm);
    font-weight: var(--font-semibold);
    color: var(--text-secondary);
    margin-bottom: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  /* Pending review cards -- 2-column grid */
  .pending-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 1rem;
  }

  .pending-card {
    display: block;
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-left: 4px solid var(--color-warning);
    border-radius: var(--radius-lg);
    padding: 1.5rem;
    text-decoration: none;
    color: inherit;
    transition: box-shadow 0.15s, border-color 0.15s;
    cursor: pointer;
  }

  .pending-card:hover {
    box-shadow: var(--shadow-sm);
    border-left-color: var(--color-warning-text);
  }

  .pending-count {
    font-size: 2.25rem;
    font-weight: var(--font-bold);
    color: var(--text-primary);
    line-height: 1;
    margin-bottom: 0.25rem;
  }

  .pending-label {
    font-size: var(--text-base);
    font-weight: var(--font-semibold);
    color: var(--text-secondary);
    margin-bottom: 0.5rem;
  }

  .pending-hint {
    font-size: var(--text-sm);
    color: var(--text-faint);
  }

  /* Integrity alerts */
  .integrity-ok {
    background: var(--color-success-subtle);
    border: 1px solid var(--color-success);
    border-radius: var(--radius-lg);
    padding: 1.25rem 1.5rem;
    color: var(--color-success-text);
    font-size: var(--text-base);
  }

  .alert-card {
    display: block;
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-left: 4px solid var(--color-danger);
    border-radius: var(--radius-lg);
    padding: 1.5rem;
    text-decoration: none;
    color: inherit;
    transition: box-shadow 0.15s;
    cursor: pointer;
  }

  .alert-card:hover {
    box-shadow: var(--shadow-sm);
  }

  .alert-count {
    font-size: 2.25rem;
    font-weight: var(--font-bold);
    color: var(--color-danger);
    line-height: 1;
    margin-bottom: 0.25rem;
  }

  .alert-label {
    font-size: var(--text-base);
    font-weight: var(--font-semibold);
    color: var(--text-secondary);
    margin-bottom: 0.5rem;
  }

  .alert-hint {
    font-size: var(--text-sm);
    color: var(--text-faint);
  }

  /* Quick stats */
  .stats-grid {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: 1rem;
  }

  .stat-card {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: 1.25rem;
    text-align: center;
  }

  .stat-number {
    font-size: 2rem;
    font-weight: var(--font-bold);
    color: var(--color-primary);
    line-height: 1;
    margin-bottom: 0.25rem;
  }

  .stat-label {
    font-size: var(--text-sm);
    color: var(--text-muted);
    font-weight: var(--font-medium);
  }

  /* Treemap grid */
  .treemap-grid {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  /* Chart cards (Phase 67+) */
  .chart-card {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: 1rem;
  }
</style>
