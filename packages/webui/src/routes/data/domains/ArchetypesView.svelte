<script lang="ts">
  import { forge, friendlyError } from '$lib/sdk'
  import { ConfirmDialog, LoadingSpinner, EmptyState, PageHeader } from '$lib/components'
  import { addToast } from '$lib/stores/toast.svelte'
  import type { Domain, Archetype } from '@forge/sdk'

  interface ArchetypeWithCounts extends Archetype {
    resume_count: number
    perspective_count: number
    domain_count: number
  }

  // ---- State ----
  let archetypes = $state<ArchetypeWithCounts[]>([])
  let allDomains = $state<Domain[]>([])
  let loading = $state(true)

  // Create form
  let showCreateForm = $state(false)
  let createName = $state('')
  let createDescription = $state('')
  let creating = $state(false)

  // Edit state
  let editingId = $state<string | null>(null)
  let editName = $state('')
  let editDescription = $state('')
  let saving = $state(false)

  // Expanded archetype (domain management)
  let expandedId = $state<string | null>(null)
  let expandedDomains = $state<Domain[]>([])
  let domainLoading = $state(false)

  // Delete confirmation
  let deleteConfirm = $state(false)
  let deleteTarget = $state<ArchetypeWithCounts | null>(null)

  // ---- Load ----
  async function loadArchetypes() {
    loading = true
    const [archResult, domResult] = await Promise.all([
      forge.archetypes.list({ limit: 200 }),
      forge.domains.list({ limit: 200 }),
    ])
    if (archResult.ok) {
      archetypes = archResult.data as ArchetypeWithCounts[]
    } else {
      addToast({ message: friendlyError(archResult.error, 'Failed to load archetypes'), type: 'error' })
    }
    if (domResult.ok) {
      allDomains = domResult.data
    }
    loading = false
  }

  loadArchetypes()

  // ---- Create ----
  async function handleCreate() {
    if (!createName.trim()) return
    creating = true
    const result = await forge.archetypes.create({
      name: createName.trim(),
      description: createDescription.trim() || undefined,
    })
    if (result.ok) {
      addToast({ message: `Archetype '${result.data.name}' created`, type: 'success' })
      createName = ''
      createDescription = ''
      showCreateForm = false
      await loadArchetypes()
    } else {
      addToast({ message: friendlyError(result.error, 'Failed to create archetype'), type: 'error' })
    }
    creating = false
  }

  // ---- Edit ----
  function startEdit(arch: ArchetypeWithCounts) {
    editingId = arch.id
    editName = arch.name
    editDescription = arch.description ?? ''
  }

  function cancelEdit() {
    editingId = null
  }

  async function saveEdit() {
    if (!editingId || !editName.trim()) return
    saving = true
    const result = await forge.archetypes.update(editingId, {
      name: editName.trim(),
      description: editDescription.trim() || null,
    })
    if (result.ok) {
      addToast({ message: 'Archetype updated', type: 'success' })
      editingId = null
      await loadArchetypes()
    } else {
      addToast({ message: friendlyError(result.error, 'Failed to update archetype'), type: 'error' })
    }
    saving = false
  }

  // ---- Expand / Domain management ----
  async function toggleExpand(arch: ArchetypeWithCounts) {
    if (expandedId === arch.id) {
      expandedId = null
      return
    }
    expandedId = arch.id
    domainLoading = true
    const result = await forge.archetypes.listDomains(arch.id)
    if (result.ok) {
      expandedDomains = result.data
    } else {
      addToast({ message: friendlyError(result.error, 'Failed to load domains'), type: 'error' })
    }
    domainLoading = false
  }

  function isDomainAssociated(domainId: string): boolean {
    return expandedDomains.some((d) => d.id === domainId)
  }

  async function toggleDomain(domainId: string) {
    if (!expandedId) return
    const associated = isDomainAssociated(domainId)
    if (associated) {
      const result = await forge.archetypes.removeDomain(expandedId, domainId)
      if (result.ok) {
        expandedDomains = expandedDomains.filter((d) => d.id !== domainId)
        await loadArchetypes()
      } else {
        addToast({ message: friendlyError(result.error, 'Failed to remove domain'), type: 'error' })
      }
    } else {
      const result = await forge.archetypes.addDomain(expandedId, domainId)
      if (result.ok) {
        const domain = allDomains.find((d) => d.id === domainId)
        if (domain) expandedDomains = [...expandedDomains, domain]
        await loadArchetypes()
      } else {
        addToast({ message: friendlyError(result.error, 'Failed to add domain'), type: 'error' })
      }
    }
  }

  // ---- Delete ----
  function confirmDelete(arch: ArchetypeWithCounts) {
    deleteTarget = arch
    deleteConfirm = true
  }

  async function handleDelete() {
    if (!deleteTarget) return
    const result = await forge.archetypes.delete(deleteTarget.id)
    if (result.ok) {
      addToast({ message: `Archetype '${deleteTarget.name}' deleted`, type: 'success' })
      deleteConfirm = false
      deleteTarget = null
      if (expandedId === deleteTarget?.id) expandedId = null
      await loadArchetypes()
    } else {
      addToast({ message: friendlyError(result.error, 'Cannot delete archetype'), type: 'error' })
      deleteConfirm = false
      deleteTarget = null
    }
  }

  function isReferenced(arch: ArchetypeWithCounts): boolean {
    return arch.resume_count > 0 || arch.perspective_count > 0
  }
</script>

<div class="archetypes-page">
  <PageHeader title="Archetypes" subtitle="Resume targeting profiles and their domain mappings">
    {#snippet actions()}
      <button class="btn btn-primary" onclick={() => (showCreateForm = !showCreateForm)}>
        {showCreateForm ? 'Cancel' : '+ Add Archetype'}
      </button>
    {/snippet}
  </PageHeader>

  {#if showCreateForm}
    <div class="create-form">
      <div class="form-row">
        <label>
          <span class="form-label">Name</span>
          <input
            type="text"
            bind:value={createName}
            placeholder="e.g. cloud-native"
            class="form-input"
          />
          <span class="form-hint">Lowercase letters, digits, and hyphens only</span>
        </label>
        <label>
          <span class="form-label">Description</span>
          <input
            type="text"
            bind:value={createDescription}
            placeholder="Optional description"
            class="form-input"
          />
        </label>
        <button
          class="btn btn-primary"
          onclick={handleCreate}
          disabled={creating || !createName.trim()}
        >
          {creating ? 'Creating...' : 'Create'}
        </button>
      </div>
    </div>
  {/if}

  {#if loading}
    <LoadingSpinner />
  {:else if archetypes.length === 0}
    <EmptyState title="No archetypes found" description="Create one to get started." />
  {:else}
    <table class="archetypes-table">
      <thead>
        <tr>
          <th>Name</th>
          <th>Description</th>
          <th>Domains</th>
          <th>Resumes</th>
          <th>Perspectives</th>
          <th>Actions</th>
        </tr>
      </thead>
      <tbody>
        {#each archetypes as arch}
          {#if editingId === arch.id}
            <tr class="editing-row">
              <td>
                <input type="text" bind:value={editName} class="form-input compact" />
              </td>
              <td>
                <input type="text" bind:value={editDescription} class="form-input compact" />
              </td>
              <td class="count">{arch.domain_count}</td>
              <td class="count">{arch.resume_count}</td>
              <td class="count">{arch.perspective_count}</td>
              <td class="actions">
                <button class="btn btn-sm btn-primary" onclick={saveEdit} disabled={saving}>
                  {saving ? 'Saving...' : 'Save'}
                </button>
                <button class="btn btn-sm btn-ghost" onclick={cancelEdit}>Cancel</button>
              </td>
            </tr>
          {:else}
            <tr class:expanded={expandedId === arch.id}>
              <td class="arch-name" onclick={() => toggleExpand(arch)} role="button" tabindex="0">
                <span class="expand-arrow">{expandedId === arch.id ? '\u25BC' : '\u25B6'}</span>
                {arch.name}
              </td>
              <td class="arch-desc">{arch.description ?? '--'}</td>
              <td class="count">{arch.domain_count}</td>
              <td class="count">{arch.resume_count}</td>
              <td class="count">{arch.perspective_count}</td>
              <td class="actions">
                <button class="btn btn-sm btn-ghost" onclick={() => startEdit(arch)}>
                  Edit
                </button>
                <button
                  class="btn btn-sm btn-danger"
                  onclick={() => confirmDelete(arch)}
                  disabled={isReferenced(arch)}
                  title={isReferenced(arch) ? 'Cannot delete: referenced by resumes or perspectives' : 'Delete archetype'}
                >
                  Delete
                </button>
              </td>
            </tr>
          {/if}

          {#if expandedId === arch.id && editingId !== arch.id}
            <tr class="domain-panel-row">
              <td colspan="6">
                <div class="domain-panel">
                  <h4>Associated Domains</h4>
                  {#if domainLoading}
                    <p class="loading-text">Loading domains...</p>
                  {:else}
                    <div class="domain-checkboxes">
                      {#each allDomains as domain}
                        <label class="domain-checkbox">
                          <input
                            type="checkbox"
                            checked={isDomainAssociated(domain.id)}
                            onchange={() => toggleDomain(domain.id)}
                          />
                          <span class="domain-checkbox-label">
                            {domain.name.replace(/_/g, ' ')}
                          </span>
                        </label>
                      {/each}
                    </div>
                  {/if}
                </div>
              </td>
            </tr>
          {/if}
        {/each}
      </tbody>
    </table>
  {/if}
</div>

{#if deleteConfirm && deleteTarget}
  <ConfirmDialog
    open={deleteConfirm}
    title="Delete Archetype"
    message={`Are you sure you want to delete '${deleteTarget.name}'? This will also remove all domain associations.`}
    onconfirm={handleDelete}
    oncancel={() => { deleteConfirm = false; deleteTarget = null }}
  />
{/if}

<style>
  .archetypes-page {
    max-width: 1100px;
  }

  .create-form {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: 8px;
    padding: 1.25rem;
    margin-bottom: 1.5rem;
  }

  .form-row {
    display: flex;
    gap: 1rem;
    align-items: flex-end;
  }

  .form-row label {
    flex: 1;
  }

  .form-label {
    display: block;
    font-size: 0.8rem;
    font-weight: 600;
    color: var(--text-secondary);
    margin-bottom: 0.25rem;
  }

  .form-hint {
    display: block;
    font-size: 0.7rem;
    color: var(--text-faint);
    margin-top: 0.15rem;
  }

  .form-input {
    width: 100%;
    padding: 0.5rem 0.75rem;
    border: 1px solid var(--color-border-strong);
    border-radius: 6px;
    font-size: 0.85rem;
  }

  .form-input.compact {
    padding: 0.35rem 0.5rem;
    font-size: 0.8rem;
  }

  .archetypes-table {
    width: 100%;
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: 8px;
    border-collapse: collapse;
    overflow: hidden;
  }

  .archetypes-table th {
    background: var(--color-surface-raised);
    padding: 0.75rem 1rem;
    text-align: left;
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.03em;
    border-bottom: 1px solid var(--color-border);
  }

  .archetypes-table td {
    padding: 0.75rem 1rem;
    font-size: 0.85rem;
    border-bottom: 1px solid var(--color-ghost);
  }

  .archetypes-table tbody tr:last-child td {
    border-bottom: none;
  }

  .arch-name {
    font-weight: 600;
    color: var(--text-primary);
    font-family: monospace;
    cursor: pointer;
    user-select: none;
  }

  .arch-name:hover {
    color: var(--color-primary);
  }

  .expand-arrow {
    font-size: 0.65rem;
    margin-right: 0.35rem;
    color: var(--text-faint);
  }

  .arch-desc {
    color: var(--text-muted);
  }

  .count {
    text-align: center;
    color: var(--text-secondary);
  }

  .actions {
    display: flex;
    gap: 0.5rem;
  }

  .editing-row {
    background: var(--color-info-subtle);
  }

  tr.expanded td {
    border-bottom-color: transparent;
  }

  .domain-panel-row td {
    padding: 0 1rem 1rem;
    background: var(--color-surface-raised);
    border-bottom: 1px solid var(--color-border);
  }

  .domain-panel {
    padding: 0.75rem 1rem;
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: 6px;
  }

  .domain-panel h4 {
    font-size: 0.8rem;
    font-weight: 600;
    color: var(--text-secondary);
    margin-bottom: 0.5rem;
  }

  .loading-text {
    font-size: 0.8rem;
    color: var(--text-faint);
  }

  .domain-checkboxes {
    display: flex;
    flex-wrap: wrap;
    gap: 0.75rem;
  }

  .domain-checkbox {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    cursor: pointer;
  }

  .domain-checkbox input[type="checkbox"] {
    accent-color: var(--color-primary);
  }

  .domain-checkbox-label {
    font-size: 0.8rem;
    color: var(--text-secondary);
    text-transform: capitalize;
  }

</style>
