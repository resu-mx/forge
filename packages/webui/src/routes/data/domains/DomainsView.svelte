<script lang="ts">
  import { forge, friendlyError } from '$lib/sdk'
  import { ConfirmDialog, LoadingSpinner, EmptyState, PageHeader } from '$lib/components'
  import { addToast } from '$lib/stores/toast.svelte'
  import type { DomainWithUsage } from '@forge/sdk'

  // ---- State ----
  let domains = $state<DomainWithUsage[]>([])
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

  // Delete confirmation
  let deleteConfirm = $state(false)
  let deleteTarget = $state<DomainWithUsage | null>(null)

  // ---- Load ----
  async function loadDomains() {
    loading = true
    const result = await forge.domains.list({ limit: 200 })
    if (result.ok) {
      domains = result.data
    } else {
      addToast(friendlyError(result.error, 'Failed to load domains'), 'error')
    }
    loading = false
  }

  loadDomains()

  // ---- Create ----
  async function handleCreate() {
    if (!createName.trim()) return
    creating = true
    const result = await forge.domains.create({
      name: createName.trim(),
      description: createDescription.trim() || undefined,
    })
    if (result.ok) {
      addToast(`Domain '${result.data.name}' created`, 'success')
      createName = ''
      createDescription = ''
      showCreateForm = false
      await loadDomains()
    } else {
      addToast(friendlyError(result.error, 'Failed to create domain'), 'error')
    }
    creating = false
  }

  // ---- Edit ----
  function startEdit(domain: DomainWithUsage) {
    editingId = domain.id
    editName = domain.name
    editDescription = domain.description ?? ''
  }

  function cancelEdit() {
    editingId = null
  }

  async function saveEdit() {
    if (!editingId || !editName.trim()) return
    saving = true
    const result = await forge.domains.update(editingId, {
      name: editName.trim(),
      description: editDescription.trim() || null,
    })
    if (result.ok) {
      addToast('Domain updated', 'success')
      editingId = null
      await loadDomains()
    } else {
      addToast(friendlyError(result.error, 'Failed to update domain'), 'error')
    }
    saving = false
  }

  // ---- Delete ----
  function confirmDelete(domain: DomainWithUsage) {
    deleteTarget = domain
    deleteConfirm = true
  }

  async function handleDelete() {
    if (!deleteTarget) return
    const result = await forge.domains.delete(deleteTarget.id)
    if (result.ok) {
      addToast(`Domain '${deleteTarget.name}' deleted`, 'success')
    } else {
      addToast(friendlyError(result.error, 'Cannot delete domain'), 'error')
    }
    deleteConfirm = false
    deleteTarget = null
    // Refresh in both cases: a 409 means our counts were stale.
    await loadDomains()
  }

  /** Counts are present only when the API sent them; never assume "unused". */
  function hasUsage(domain: DomainWithUsage): boolean {
    return typeof domain.perspective_count === 'number' && typeof domain.archetype_count === 'number'
  }

  function isReferenced(domain: DomainWithUsage): boolean {
    return !hasUsage(domain) || domain.perspective_count > 0 || domain.archetype_count > 0
  }
</script>

<div class="domains-page">
  <PageHeader title="Domains" subtitle="Experience domains used by perspectives and archetypes">
    {#snippet actions()}
      <button class="btn btn-primary" onclick={() => (showCreateForm = !showCreateForm)}>
        {showCreateForm ? 'Cancel' : '+ Add Domain'}
      </button>
    {/snippet}
  </PageHeader>

  {#if showCreateForm}
    <div class="create-form">
      <div class="form-row">
        <label>
          <span class="field-label">Name</span>
          <input
            type="text"
            bind:value={createName}
            placeholder="e.g. cloud_engineering"
            class="field-input"
          />
          <span class="field-hint">Lowercase letters, digits, and underscores only</span>
        </label>
        <label>
          <span class="field-label">Description</span>
          <input
            type="text"
            bind:value={createDescription}
            placeholder="Optional description"
            class="field-input"
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
  {:else if domains.length === 0}
    <EmptyState title="No domains found" description="Create one to get started." />
  {:else}
    <table class="domains-table">
      <thead>
        <tr>
          <th>Name</th>
          <th>Description</th>
          <th>Perspectives</th>
          <th>Archetypes</th>
          <th>Actions</th>
        </tr>
      </thead>
      <tbody>
        {#each domains as domain}
          {#if editingId === domain.id}
            <tr class="editing-row">
              <td>
                <input type="text" bind:value={editName} class="field-input" />
              </td>
              <td>
                <input type="text" bind:value={editDescription} class="field-input" />
              </td>
              <td>{domain.perspective_count ?? '—'}</td>
              <td>{domain.archetype_count ?? '—'}</td>
              <td class="actions">
                <button class="btn btn-sm btn-primary" onclick={saveEdit} disabled={saving}>
                  {saving ? 'Saving...' : 'Save'}
                </button>
                <button class="btn btn-sm btn-ghost" onclick={cancelEdit}>Cancel</button>
              </td>
            </tr>
          {:else}
            <tr>
              <td class="domain-name">{domain.name}</td>
              <td class="domain-desc">{domain.description ?? '--'}</td>
              <td class="count">{domain.perspective_count ?? '—'}</td>
              <td class="count">{domain.archetype_count ?? '—'}</td>
              <td class="actions">
                <button class="btn btn-sm btn-ghost" onclick={() => startEdit(domain)}>
                  Edit
                </button>
                <button
                  class="btn btn-sm btn-danger"
                  onclick={() => confirmDelete(domain)}
                  disabled={isReferenced(domain)}
                  title={!hasUsage(domain)
                    ? 'Usage unknown; delete disabled'
                    : isReferenced(domain)
                      ? 'Cannot delete: referenced by perspectives or archetypes'
                      : 'Delete domain'}
                >
                  Delete
                </button>
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
    title="Delete Domain"
    message={`Are you sure you want to delete '${deleteTarget.name}'? This cannot be undone.`}
    onconfirm={handleDelete}
    oncancel={() => { deleteConfirm = false; deleteTarget = null }}
  />
{/if}

<style>
  .domains-page {
    max-width: 1000px;
  }

  .create-form {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
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

  .domains-table {
    width: 100%;
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    border-collapse: collapse;
    overflow: hidden;
  }

  .domains-table th {
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

  .domains-table td {
    padding: 0.75rem 1rem;
    font-size: 0.85rem;
    border-bottom: 1px solid var(--color-ghost);
  }

  .domains-table tbody tr:last-child td {
    border-bottom: none;
  }

  .domain-name {
    font-weight: 600;
    color: var(--text-primary);
    font-family: monospace;
  }

  .domain-desc {
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
</style>
