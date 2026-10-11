<script lang="ts">
  import { forge } from '$lib/sdk'

  let { filters = $bindable(), onchange }: {
    filters: {
      organization?: string
      source_type?: string
      search?: string
    }
    onchange: () => void
  } = $props()

  let organizations = $state<{ id: string; name: string }[]>([])

  $effect(() => {
    loadFilterOptions()
  })

  async function loadFilterOptions() {
    const orgResult = await forge.organizations.list({ limit: 500 })
    if (orgResult.ok) organizations = orgResult.data.map(o => ({ id: o.id, name: o.name }))
  }

  function handleChange() {
    onchange()
  }
</script>

<div class="filter-bar">
  <select
    class="field-select"
    bind:value={filters.organization}
    onchange={handleChange}
  >
    <option value="">All Organizations</option>
    {#each organizations as org}
      <option value={org.id}>{org.name}</option>
    {/each}
  </select>

  <select
    class="field-select"
    bind:value={filters.source_type}
    onchange={handleChange}
  >
    <option value="">All Types</option>
    <option value="role">Role</option>
    <option value="project">Project</option>
    <option value="education">Education</option>
    <option value="clearance">Clearance</option>
    <option value="general">General</option>
  </select>

  <input
    type="text"
    class="field-input"
    placeholder="Search sources..."
    bind:value={filters.search}
    oninput={handleChange}
  />
</div>

<style>
  .filter-bar {
    display: flex;
    gap: var(--space-2, 0.5rem);
    flex-wrap: wrap;
    align-items: center;
  }

  .filter-bar .field-select,
  .filter-bar .field-input {
    font-size: var(--text-sm, 0.8rem);
    padding: var(--space-1, 0.25rem) var(--space-2, 0.5rem);
    border: 1px solid var(--color-border, #e5e7eb);
    border-radius: var(--radius-sm, 3px);
    background: var(--color-surface, #fff);
    color: var(--text-primary, #1a1a2e);
  }

  .filter-bar .field-input {
    min-width: 150px;
  }
</style>
