<script lang="ts">
  import { forge, friendlyError } from '$lib/sdk'
  import { sharedSummaryWarningName } from '$lib/summary-share'
  import { addToast } from '$lib/stores/toast.svelte'
  import { LoadingSpinner } from '$lib/components'
  import type { Summary } from '@forge/sdk'

  interface Props {
    open: boolean
    /** Called when the user picks a summary. `id` is the summary to link, or null to skip. */
    onpick: (summaryId: string | null) => void
    oncancel: () => void
  }

  let { open, onpick, oncancel }: Props = $props()

  let summaries = $state<Summary[]>([])
  let loading = $state(true)
  let showCreateForm = $state(false)
  let creating = $state(false)
  let warningForId = $state<string | null>(null)
  let warningResumeName = $state<string | null>(null)

  // Create form fields
  let newTitle = $state('')
  let newRole = $state('')
  let newTagline = $state('')
  let newDescription = $state('')

  let templates = $derived(summaries.filter(s => s.is_template))
  let instances = $derived(summaries.filter(s => !s.is_template))

  $effect(() => {
    if (open) {
      loadSummaries()
      // Reset form state on re-open
      showCreateForm = false
      warningForId = null
      warningResumeName = null
      newTitle = ''
      newRole = ''
      newTagline = ''
      newDescription = ''
    }
  })

  async function loadSummaries() {
    loading = true
    const result = await forge.summaries.list({ limit: 500 })
    if (result.ok) {
      summaries = result.data
    } else {
      addToast({ message: friendlyError(result.error, 'Failed to load summaries'), type: 'error' })
    }
    loading = false
  }

  /** [Use] a template: clone it, then link the clone */
  async function useTemplate(id: string) {
    const result = await forge.summaries.clone(id)
    if (result.ok) {
      onpick(result.data.id)
    } else {
      addToast({ message: friendlyError(result.error, 'Failed to clone template'), type: 'error' })
    }
  }

  /** [Link] an existing summary. A shared summary always gets the warning first. */
  async function linkSummary(id: string) {
    const summary = summaries.find(s => s.id === id)
    if (summary && summary.linked_resume_count > 0) {
      // The lookup only picks the name; whatever it returns, the warning is shown.
      const linked = await forge.summaries.linkedResumes(id, { limit: 1 })
      warningForId = id
      warningResumeName = sharedSummaryWarningName(linked)
      return
    }
    onpick(id)
  }

  /** User confirmed [Link Anyway] after seeing the shared-summary warning */
  function confirmLink() {
    if (warningForId) {
      onpick(warningForId)
      warningForId = null
    }
  }

  /** User chose [Clone] from the shared-summary warning dialog */
  async function cloneInstead() {
    if (warningForId) {
      const result = await forge.summaries.clone(warningForId)
      if (result.ok) {
        onpick(result.data.id)
      } else {
        addToast({ message: friendlyError(result.error, 'Failed to clone'), type: 'error' })
      }
      warningForId = null
    }
  }

  /** [Create New] inline form submission */
  async function createAndLink() {
    if (!newTitle.trim()) {
      addToast({ message: 'Title is required', type: 'error' })
      return
    }
    creating = true
    const result = await forge.summaries.create({
      title: newTitle.trim(),
      role: newRole.trim() || undefined,
      tagline: newTagline.trim() || undefined,
      description: newDescription.trim() || undefined,
    })
    if (result.ok) {
      onpick(result.data.id)
    } else {
      addToast({ message: friendlyError(result.error, 'Failed to create summary'), type: 'error' })
    }
    creating = false
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="overlay" onclick={oncancel}>
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="picker-dialog" onclick={(e) => e.stopPropagation()}>
      <div class="picker-header">
        <h2 class="picker-title">Pick a Summary</h2>
        <button onclick={() => { showCreateForm = true; warningForId = null }}
                class="btn btn-primary btn-sm">
          Create New
        </button>
      </div>

      <div class="picker-body">
        {#if loading}
          <LoadingSpinner />
        {:else if showCreateForm}
          <!-- Inline creation form -->
          <div class="create-form">
            <div class="form-field">
              <label class="field-label">Title *</label>
              <input bind:value={newTitle} class="field-input"
                     placeholder="e.g. Security Engineer - Cloud Focus" />
            </div>
            <div class="form-field">
              <label class="field-label">Role</label>
              <input bind:value={newRole} class="field-input"
                     placeholder="e.g. Senior Security Engineer" />
            </div>
            <div class="form-field">
              <label class="field-label">Tagline</label>
              <input bind:value={newTagline} class="field-input"
                     placeholder="e.g. Cloud + DevSecOps + Detection Engineering" />
            </div>
            <div class="form-field">
              <label class="field-label">Description</label>
              <textarea bind:value={newDescription} rows="3"
                        class="field-input"
                        placeholder="Full summary paragraph..." />
            </div>
            <div class="form-actions">
              <button onclick={createAndLink} disabled={creating} class="btn btn-primary btn-sm">
                {creating ? 'Creating...' : 'Create & Link'}
              </button>
              <button onclick={() => showCreateForm = false} class="btn btn-ghost btn-sm">Back</button>
            </div>
          </div>
        {:else if warningForId}
          <!-- Shared summary warning -->
          <div class="warning-box">
            <p class="warning-text">
              This summary is also used by <strong>{warningResumeName}</strong>.
              Changes will affect both resumes. Clone instead?
            </p>
            <div class="form-actions">
              <button onclick={cloneInstead} class="btn btn-primary btn-sm">Clone</button>
              <button onclick={confirmLink} class="btn btn-ghost btn-sm">Link Anyway</button>
              <button onclick={() => warningForId = null} class="btn btn-ghost btn-sm">Cancel</button>
            </div>
          </div>
        {:else}
          <!-- Templates -->
          {#if templates.length > 0}
            <div class="picker-section">
              <h3 class="picker-section-title">Templates</h3>
              <div class="picker-list">
                {#each templates as summary (summary.id)}
                  <div class="picker-item template">
                    <div class="picker-item-info">
                      <span class="star">&#9733;</span>
                      <span class="picker-item-title">{summary.title}</span>
                    </div>
                    <button onclick={() => useTemplate(summary.id)}
                            class="btn btn-primary btn-xs">Use</button>
                  </div>
                {/each}
              </div>
            </div>
          {/if}

          <!-- Existing Summaries -->
          {#if instances.length > 0}
            <div class="picker-section">
              <h3 class="picker-section-title">Existing Summaries</h3>
              <div class="picker-list">
                {#each instances as summary (summary.id)}
                  <div class="picker-item">
                    <span class="picker-item-title">{summary.title}</span>
                    <button onclick={() => linkSummary(summary.id)}
                            class="btn btn-ghost btn-xs">Link</button>
                  </div>
                {/each}
              </div>
            </div>
          {/if}

          {#if templates.length === 0 && instances.length === 0}
            <p class="empty-note">No summaries yet. Create one or skip for now.</p>
          {/if}
        {/if}
      </div>

      <!-- Footer -->
      <div class="picker-footer">
        <button onclick={oncancel} class="btn btn-ghost btn-sm">Cancel</button>
        <button onclick={() => onpick(null)} class="btn btn-ghost btn-sm">Skip</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: var(--color-overlay);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: var(--z-modal);
  }

  .picker-dialog {
    background: var(--color-surface);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-lg);
    max-width: 480px;
    width: 90%;
    max-height: 80vh;
    display: flex;
    flex-direction: column;
  }

  .picker-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-4) var(--space-5);
    border-bottom: 1px solid var(--color-border);
  }
  .picker-title { font-size: var(--text-xl); font-weight: var(--font-semibold); color: var(--text-primary); }

  .picker-body {
    overflow-y: auto;
    flex: 1;
    padding: var(--space-4) var(--space-5);
  }

  .picker-section { margin-bottom: var(--space-4); }
  .picker-section-title {
    font-size: var(--text-sm);
    font-weight: var(--font-semibold);
    color: var(--text-muted);
    margin-bottom: var(--space-2);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .picker-list { display: flex; flex-direction: column; gap: 0.25rem; }

  .picker-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    padding: var(--space-2) var(--space-3);
  }
  .picker-item.template {
    border-color: var(--color-template-border);
    background: var(--color-template-bg);
  }
  .picker-item-info {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
  }
  .picker-item-title {
    font-size: 0.85rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-primary);
  }
  .star { color: var(--color-template-star); flex-shrink: 0; }

  .picker-footer {
    display: flex;
    justify-content: space-between;
    padding: var(--space-3) var(--space-5);
    border-top: 1px solid var(--color-border);
    background: var(--color-surface-raised);
    border-radius: 0 0 var(--radius-lg) var(--radius-lg);
  }

  .create-form { display: flex; flex-direction: column; gap: var(--space-3); }
  .form-field { display: flex; flex-direction: column; gap: var(--space-1); }
  .field-label { font-size: var(--text-sm); font-weight: var(--font-medium); color: var(--text-secondary); }
  .field-input {
    padding: 0.375rem 0.625rem;
    border: 1px solid var(--color-border-strong);
    border-radius: var(--radius-sm);
    font-size: 0.85rem;
    color: var(--text-primary);
    background: var(--color-surface);
  }
  .field-input:focus {
    outline: none;
    border-color: var(--color-border-focus);
    box-shadow: 0 0 0 2px var(--color-primary-subtle);
  }
  .form-actions { display: flex; gap: var(--space-2); }

  .warning-box {
    background: var(--color-warning-subtle);
    border: 1px solid var(--color-warning-border);
    border-radius: var(--radius-md);
    padding: var(--space-4);
  }
  .warning-text {
    font-size: 0.85rem;
    color: var(--color-warning-text);
    margin-bottom: var(--space-3);
    line-height: var(--leading-normal);
  }
  .empty-note {
    font-size: 0.85rem;
    color: var(--text-muted);
    text-align: center;
    padding: var(--space-6) 0;
  }

</style>
