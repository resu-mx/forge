<!--
  Summaries page (Phase 91 rework).

  - Clickable list entries (no inline buttons) that open SummaryDetailModal
  - Sticky page header, Templates section header, Summaries section header
  - Templates and Summaries sections collapsible
  - Group-by: None | Industry | Role Type | Keyword
  - Sort-by: Title (A-Z, Z-A) — templates always float to the top
  - Filter: industry, role type, keyword skill, text search
-->
<script lang="ts">
  import { forge, friendlyError } from '$lib/sdk'
  import { addToast } from '$lib/stores/toast.svelte'
  import { LoadingSpinner, EmptyState, PageHeader, ListSearchInput } from '$lib/components'
  import SummaryDetailModal from '$lib/components/SummaryDetailModal.svelte'
  import { keywordLoadFailureMessage } from '$lib/summary-keywords'
  import type { ForgeError, Summary, Industry, RoleType, Skill } from '@forge/sdk'

  type GroupBy = 'none' | 'industry' | 'role_type' | 'keyword'
  type SortDirection = 'asc' | 'desc'

  let summaries = $state<Summary[]>([])
  let summaryKeywords = $state<Record<string, Skill[]>>({}) // summaryId → skills[]
  let allIndustries = $state<Industry[]>([])
  let allRoleTypes = $state<RoleType[]>([])
  let allSkills = $state<Skill[]>([])
  let loading = $state(true)

  // Modal state
  let modalSummaryId = $state<string | null>(null)

  // Filter / sort / group UI state
  let searchQuery = $state('')
  let filterIndustryId = $state<string>('all')
  let filterRoleTypeId = $state<string>('all')
  let filterSkillId = $state<string>('all')
  let groupBy = $state<GroupBy>('none')
  let sortDir = $state<SortDirection>('asc')

  // Collapsible sections
  let templatesCollapsed = $state(false)
  let summariesCollapsed = $state(false)
  let collapsedGroups = $state<Record<string, boolean>>({})

  $effect(() => {
    loadAll()
  })

  async function loadAll() {
    loading = true
    const [summariesRes, industriesRes, roleTypesRes, skillsRes] = await Promise.all([
      forge.summaries.list({ limit: 500, sort_by: 'title', direction: 'asc' }),
      forge.industries.list({ limit: 200 }),
      forge.roleTypes.list({ limit: 200 }),
      forge.skills.list({ limit: 500 }),
    ])

    if (summariesRes.ok) {
      summaries = summariesRes.data
      // Parallel-fetch keyword skills for each summary so grouping/filtering works client-side
      const keywordMap: Record<string, Skill[]> = {}
      let failed = 0
      let firstError: ForgeError | null = null
      await Promise.all(
        summaries.map(async (s) => {
          const r = await forge.summaries.listSkills(s.id)
          if (r.ok) {
            keywordMap[s.id] = r.data
          } else {
            failed++
            firstError ??= r.error
          }
        }),
      )
      summaryKeywords = keywordMap
      const message = keywordLoadFailureMessage(
        failed,
        summaries.length,
        firstError ? friendlyError(firstError) : undefined,
      )
      if (message) addToast({ message, type: 'error' })
    } else {
      addToast({ message: friendlyError(summariesRes.error, 'Failed to load summaries'), type: 'error' })
    }

    if (industriesRes.ok) allIndustries = industriesRes.data
    if (roleTypesRes.ok) allRoleTypes = roleTypesRes.data
    if (skillsRes.ok) allSkills = skillsRes.data

    loading = false
  }

  // ── Derived views ────────────────────────────────────────────────────

  let filteredSummaries = $derived.by(() => {
    let result = summaries

    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase()
      result = result.filter((s) =>
        s.title.toLowerCase().includes(q) ||
        (s.role?.toLowerCase().includes(q) ?? false) ||
        (s.description?.toLowerCase().includes(q) ?? false),
      )
    }

    if (filterIndustryId !== 'all') {
      result = result.filter((s) => s.industry_id === filterIndustryId)
    }

    if (filterRoleTypeId !== 'all') {
      result = result.filter((s) => s.role_type_id === filterRoleTypeId)
    }

    if (filterSkillId !== 'all') {
      result = result.filter((s) => summaryKeywords[s.id]?.some((k) => k.id === filterSkillId))
    }

    // Sort by title (templates already float to top from the API; re-sort client-side)
    const sorted = [...result].sort((a, b) => {
      // Templates first
      if (a.is_template !== b.is_template) return a.is_template ? -1 : 1
      const cmp = a.title.localeCompare(b.title)
      return sortDir === 'asc' ? cmp : -cmp
    })
    return sorted
  })

  let templateList = $derived(filteredSummaries.filter((s) => s.is_template))
  let instanceList = $derived(filteredSummaries.filter((s) => !s.is_template))

  /**
   * Build grouped instances for display. Returns Array<[groupLabel, summaries[]]>.
   * Groups are sorted by label; within a group, summaries keep their sort order.
   */
  let groupedInstances = $derived.by(() => {
    if (groupBy === 'none') return null
    const groups = new Map<string, Summary[]>()

    for (const summary of instanceList) {
      if (groupBy === 'industry') {
        const label = summary.industry_id
          ? (allIndustries.find((i) => i.id === summary.industry_id)?.name ?? 'Unknown')
          : '(No industry)'
        if (!groups.has(label)) groups.set(label, [])
        groups.get(label)!.push(summary)
      } else if (groupBy === 'role_type') {
        const label = summary.role_type_id
          ? (allRoleTypes.find((r) => r.id === summary.role_type_id)?.name ?? 'Unknown')
          : '(No role type)'
        if (!groups.has(label)) groups.set(label, [])
        groups.get(label)!.push(summary)
      } else if (groupBy === 'keyword') {
        const keywords = summaryKeywords[summary.id] ?? []
        if (keywords.length === 0) {
          const label = '(No keywords)'
          if (!groups.has(label)) groups.set(label, [])
          groups.get(label)!.push(summary)
        } else {
          // A summary can appear under multiple keyword groups
          for (const kw of keywords) {
            if (!groups.has(kw.name)) groups.set(kw.name, [])
            groups.get(kw.name)!.push(summary)
          }
        }
      }
    }

    return Array.from(groups.entries()).sort(([a], [b]) => a.localeCompare(b))
  })

  function toggleGroup(label: string) {
    collapsedGroups = { ...collapsedGroups, [label]: !collapsedGroups[label] }
  }

  function openSummary(id: string) {
    modalSummaryId = id
  }

  function openNew() {
    modalSummaryId = 'new'
  }

  function closeModal() {
    modalSummaryId = null
  }

  function onModalUpdate() {
    loadAll()
  }
</script>

<div class="summaries-page">
  <div class="sticky-page-header">
    <PageHeader title="Summaries" subtitle="Reusable professional summaries and templates">
      {#snippet actions()}
        <button class="btn btn-primary" onclick={openNew}>+ New Summary</button>
      {/snippet}
    </PageHeader>

    <div class="controls">
      <div class="controls-row">
        <ListSearchInput
          bind:value={searchQuery}
          placeholder="Search summaries..."
        />
      </div>
      <div class="controls-row controls-filters">
        <label class="control">
          <span class="control-label">Industry</span>
          <select bind:value={filterIndustryId} class="control-select">
            <option value="all">All</option>
            {#each allIndustries as industry (industry.id)}
              <option value={industry.id}>{industry.name}</option>
            {/each}
          </select>
        </label>

        <label class="control">
          <span class="control-label">Role Type</span>
          <select bind:value={filterRoleTypeId} class="control-select">
            <option value="all">All</option>
            {#each allRoleTypes as rt (rt.id)}
              <option value={rt.id}>{rt.name}</option>
            {/each}
          </select>
        </label>

        <label class="control">
          <span class="control-label">Keyword</span>
          <select bind:value={filterSkillId} class="control-select">
            <option value="all">All</option>
            {#each allSkills as skill (skill.id)}
              <option value={skill.id}>{skill.name}</option>
            {/each}
          </select>
        </label>

        <label class="control">
          <span class="control-label">Group By</span>
          <select bind:value={groupBy} class="control-select">
            <option value="none">None</option>
            <option value="industry">Industry</option>
            <option value="role_type">Role Type</option>
            <option value="keyword">Keyword</option>
          </select>
        </label>

        <label class="control">
          <span class="control-label">Sort</span>
          <select bind:value={sortDir} class="control-select">
            <option value="asc">Title A–Z</option>
            <option value="desc">Title Z–A</option>
          </select>
        </label>
      </div>
    </div>
  </div>

  {#if loading}
    <LoadingSpinner />
  {:else if summaries.length === 0}
    <EmptyState
      title="No summaries yet"
      description="Create one to get started. Summaries can be linked to resumes or promoted to templates for reuse."
    >
      <button class="btn btn-primary" onclick={openNew}>Create Summary</button>
    </EmptyState>
  {:else if filteredSummaries.length === 0}
    <EmptyState
      title="No summaries match your filters"
      description="Try adjusting your search or filter controls."
    />
  {:else}
    <!-- Templates section -->
    {#if templateList.length > 0}
      <section class="section">
        <button
          class="section-header sticky-section-header"
          onclick={() => templatesCollapsed = !templatesCollapsed}
          aria-expanded={!templatesCollapsed}
        >
          <span class="section-chevron" class:collapsed={templatesCollapsed}>&#9656;</span>
          <span class="star">&#9733;</span>
          <span class="section-title-text">Templates</span>
          <span class="section-count">{templateList.length}</span>
        </button>
        {#if !templatesCollapsed}
          <ul class="summary-list">
            {#each templateList as summary (summary.id)}
              {@render summaryRow(summary)}
            {/each}
          </ul>
        {/if}
      </section>
    {/if}

    <!-- Summaries section -->
    <section class="section">
      <button
        class="section-header sticky-section-header"
        onclick={() => summariesCollapsed = !summariesCollapsed}
        aria-expanded={!summariesCollapsed}
      >
        <span class="section-chevron" class:collapsed={summariesCollapsed}>&#9656;</span>
        <span class="section-title-text">Summaries</span>
        <span class="section-count">{instanceList.length}</span>
      </button>

      {#if !summariesCollapsed}
        {#if instanceList.length === 0}
          <p class="empty-note">No regular summaries.</p>
        {:else if groupedInstances}
          {#each groupedInstances as [label, items] (label)}
            <div class="group-section">
              <button
                class="group-header"
                onclick={() => toggleGroup(label)}
                aria-expanded={!collapsedGroups[label]}
              >
                <span class="group-chevron" class:collapsed={collapsedGroups[label]}>&#9656;</span>
                <span class="group-label">{label}</span>
                <span class="group-count">{items.length}</span>
              </button>
              {#if !collapsedGroups[label]}
                <ul class="summary-list">
                  {#each items as summary (summary.id + '-' + label)}
                    {@render summaryRow(summary)}
                  {/each}
                </ul>
              {/if}
            </div>
          {/each}
        {:else}
          <ul class="summary-list">
            {#each instanceList as summary (summary.id)}
              {@render summaryRow(summary)}
            {/each}
          </ul>
        {/if}
      {/if}
    </section>
  {/if}
</div>

{#snippet summaryRow(summary: Summary)}
  <li>
    <button
      type="button"
      class="summary-row"
      class:template={summary.is_template}
      onclick={() => openSummary(summary.id)}
    >
      <div class="row-main">
        <div class="row-title-line">
          {#if summary.is_template}
            <span class="star" title="Template">&#9733;</span>
          {/if}
          <span class="row-title">{summary.title}</span>
          {#if summary.linked_resume_count > 0}
            <span class="row-count">
              {summary.linked_resume_count} resume{summary.linked_resume_count !== 1 ? 's' : ''}
            </span>
          {/if}
        </div>
        {#if summary.role}
          <span class="row-role">{summary.role}</span>
        {/if}
        {#if summaryKeywords[summary.id] && summaryKeywords[summary.id].length > 0}
          <div class="row-keywords">
            {#each summaryKeywords[summary.id].slice(0, 5) as kw}
              <span class="kw-pill">{kw.name}</span>
            {/each}
            {#if summaryKeywords[summary.id].length > 5}
              <span class="kw-pill kw-more">+{summaryKeywords[summary.id].length - 5}</span>
            {/if}
          </div>
        {/if}
      </div>
    </button>
  </li>
{/snippet}

{#if modalSummaryId !== null}
  <SummaryDetailModal
    summaryId={modalSummaryId}
    onclose={closeModal}
    onupdate={onModalUpdate}
  />
{/if}

<style>
  .summaries-page {
    max-width: 900px;
    position: relative;
  }

  /* ── Sticky page header with controls ────────────────────────────── */
  .sticky-page-header {
    position: sticky;
    top: 0;
    z-index: 5;
    background: var(--color-bg);
    padding-bottom: 0.75rem;
    margin-bottom: 0.75rem;
    border-bottom: 1px solid var(--color-border);
  }

  .controls {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: 0 1rem;
  }

  .controls-row {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
    align-items: flex-end;
  }

  .controls-filters {
    row-gap: 0.5rem;
  }

  .control {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    min-width: 130px;
  }

  .control-label {
    font-size: var(--text-xs);
    font-weight: var(--font-medium);
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }

  .control-select {
    padding: 0.35rem 0.55rem;
    border: 1px solid var(--color-border-strong);
    border-radius: var(--radius-md);
    font-size: var(--text-sm);
    background: var(--color-surface);
    color: var(--text-primary);
    font-family: inherit;
  }

  .control-select:focus {
    outline: none;
    border-color: var(--color-border-focus);
    box-shadow: 0 0 0 2px var(--color-primary-subtle);
  }

  /* ── Sections ────────────────────────────────────────────────────── */
  .section {
    margin-bottom: 1.5rem;
  }

  .section-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    width: 100%;
    padding: 0.6rem 1rem;
    background: var(--color-surface-sunken);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    cursor: pointer;
    font-size: var(--text-base);
    font-weight: var(--font-semibold);
    color: var(--text-primary);
    text-align: left;
    font-family: inherit;
  }

  .section-header:hover { background: var(--color-surface-raised); }

  .sticky-section-header {
    position: sticky;
    /* Offset under the sticky page header. Approximate header height. */
    top: 8rem;
    z-index: 3;
  }

  .section-chevron {
    display: inline-block;
    font-size: var(--text-xs);
    color: var(--text-muted);
    transition: transform 0.15s ease;
  }

  .section-chevron.collapsed { transform: rotate(0deg); }
  .section-chevron:not(.collapsed) { transform: rotate(90deg); }

  .section-title-text {
    flex: 1;
  }

  .section-count {
    font-size: var(--text-xs);
    color: var(--text-faint);
    font-weight: var(--font-normal);
  }

  .star { color: var(--color-template-star); }

  .empty-note {
    color: var(--text-muted);
    font-size: var(--text-sm);
    padding: 0.5rem 1rem;
  }

  /* ── Grouping ───────────────────────────────────────────────────── */
  .group-section {
    margin-top: 0.75rem;
  }

  .group-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    width: 100%;
    padding: 0.5rem 0.85rem;
    background: var(--color-surface-raised);
    border: none;
    border-left: 3px solid var(--color-primary);
    border-radius: var(--radius-sm);
    cursor: pointer;
    font-size: var(--text-sm);
    font-weight: var(--font-semibold);
    color: var(--text-primary);
    text-align: left;
    font-family: inherit;
  }

  .group-header:hover { background: var(--color-surface-sunken); }

  .group-chevron {
    display: inline-block;
    font-size: var(--text-xs);
    color: var(--text-muted);
    transition: transform 0.15s ease;
  }
  .group-chevron.collapsed { transform: rotate(0deg); }
  .group-chevron:not(.collapsed) { transform: rotate(90deg); }

  .group-label {
    flex: 1;
    text-transform: capitalize;
  }

  .group-count {
    font-size: var(--text-xs);
    color: var(--text-faint);
    font-weight: var(--font-normal);
  }

  /* ── Summary rows ────────────────────────────────────────────────── */
  .summary-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    margin-top: 0.5rem;
  }

  .summary-row {
    display: block;
    width: 100%;
    text-align: left;
    padding: 0.75rem 1rem;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
    cursor: pointer;
    transition: background 0.12s, border-color 0.12s;
    font-family: inherit;
  }

  .summary-row:hover {
    background: var(--color-surface-raised);
    border-color: var(--color-border-focus);
  }

  .summary-row.template {
    border-color: var(--color-template-border);
    background: var(--color-template-bg);
  }

  .row-main { display: flex; flex-direction: column; gap: 0.25rem; }

  .row-title-line {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .row-title {
    font-size: var(--text-base);
    font-weight: var(--font-semibold);
    color: var(--text-primary);
  }

  .row-count {
    font-size: var(--text-xs);
    color: var(--text-muted);
  }

  .row-role {
    font-size: var(--text-sm);
    color: var(--text-secondary);
  }

  .row-keywords {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
    margin-top: 0.2rem;
  }

  .kw-pill {
    display: inline-block;
    padding: 0.1em 0.45em;
    background: var(--color-info-subtle);
    color: var(--color-info-text);
    border-radius: var(--radius-sm);
    font-size: var(--text-xs);
    font-weight: var(--font-medium);
  }

  .kw-more {
    background: var(--color-border);
    color: var(--text-muted);
  }
</style>
