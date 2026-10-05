<!--
  HeaderEditor.svelte — Resume header display with tagline editor (Phase 92).

  Tagline resolution order (display):
    1. tagline_override (user-authored) — shown with a "Using override" badge
    2. generated_tagline (from linked JDs via TF-IDF)
    3. Empty placeholder
-->
<script lang="ts">
  import type { ResumeHeader, ResumeTaglineState } from '@forge/sdk'
  import { forge, friendlyError } from '$lib/sdk'
  import { addToast } from '$lib/stores/toast.svelte'
  import { taglineView } from '$lib/tagline-view'

  let {
    header,
    resumeId,
    onSave,
  }: {
    header: ResumeHeader
    resumeId: string
    onSave: () => Promise<void>
  } = $props()

  let taglineState = $state<ResumeTaglineState | null>(null)
  let editingTagline = $state(false)
  let saving = $state(false)
  let regenerating = $state(false)
  let taglineValue = $state('')
  /** Set when GET /resumes/:id/tagline fails; cleared by the next successful load. */
  let taglineError = $state<string | null>(null)
  /**
   * Whether the "Show Clearance" toggle should appear when clearance is
   * hidden. True when the user has active clearance credentials but the
   * resume's show_clearance_in_header is off (or was just toggled off).
   */
  let showClearanceToggle = $state(false)

  // Load the tagline taglineState on mount and whenever the resumeId changes
  $effect(() => {
    loadTaglineState()
    checkClearanceCredentials()
  })

  async function loadTaglineState() {
    taglineError = null
    const res = await forge.resumes.getTagline(resumeId)
    if (res.ok) {
      taglineState = res.data
      taglineValue = taglineView(res.data, null).editSeed // the seed does not depend on header
    } else {
      taglineState = null
      taglineError = friendlyError(res.error, 'Tagline status unavailable')
    }
  }

  /** Check if the user has any active clearance credentials — if so, the
   *  "Show Clearance" toggle should be visible when clearance is hidden. */
  async function checkClearanceCredentials() {
    const res = await forge.credentials.list({ type: 'clearance' })
    showClearanceToggle = res.ok && res.data.some(c => c.status === 'active')
  }

  /** Toggle show_clearance_in_header on the resume and refresh. */
  async function toggleClearanceInHeader() {
    // Determine current state from the header: if clearance is shown, hide it; else show it.
    const newValue = header.clearance ? 0 : 1
    const res = await forge.resumes.update(resumeId, {
      show_clearance_in_header: newValue,
    } as any)
    if (res.ok) {
      addToast({
        message: newValue ? 'Clearance shown in header' : 'Clearance hidden from header',
        type: 'success',
      })
      await onSave()
    } else {
      addToast({ message: friendlyError(res.error), type: 'error' })
    }
  }

  /** The tagline actually shown in the preview: override > generated > header.tagline (legacy fallback). */
  let view = $derived(taglineView(taglineState, header.tagline))

  async function handleSaveOverride() {
    saving = true
    try {
      const result = await forge.resumes.updateTaglineOverride(
        resumeId,
        taglineValue.trim() || null,
      )
      if (result.ok) {
        taglineState = result.data
        addToast({ message: 'Tagline override saved', type: 'success' })
        editingTagline = false
        await onSave()
      } else {
        addToast({ message: friendlyError(result.error), type: 'error' })
      }
    } finally {
      saving = false
    }
  }

  async function handleResetOverride() {
    saving = true
    try {
      const result = await forge.resumes.updateTaglineOverride(resumeId, null)
      if (result.ok) {
        taglineState = result.data
        taglineValue = result.data.generated_tagline ?? ''
        addToast({ message: 'Override cleared — using generated tagline', type: 'success' })
        await onSave()
      } else {
        addToast({ message: friendlyError(result.error), type: 'error' })
      }
    } finally {
      saving = false
    }
  }

  async function handleRegenerate() {
    regenerating = true
    try {
      const result = await forge.resumes.regenerateTagline(resumeId)
      if (result.ok) {
        addToast({
          message: result.data.generated_tagline
            ? 'Tagline regenerated from linked JDs'
            : 'No linked JDs — generated tagline cleared',
          type: 'success',
        })
        // Refresh the taglineState to pick up the new generated value
        await loadTaglineState()
        await onSave()
      } else {
        addToast({ message: friendlyError(result.error), type: 'error' })
      }
    } finally {
      regenerating = false
    }
  }

  function startEdit() {
    editingTagline = true
    taglineValue = view.editSeed
  }

  function cancelEdit() {
    editingTagline = false
    taglineValue = view.editSeed
  }
</script>

<div class="header-editor">
  <div class="header-display">
    <div class="header-display-top">
      <h2 class="header-name">{header.name}</h2>
    </div>

    {#if editingTagline}
      <form class="tagline-edit" onsubmit={(e) => { e.preventDefault(); handleSaveOverride() }}>
        <input type="text" bind:value={taglineValue}
               placeholder="Security Engineer -- cloud + devsecops + detection"
               class="tagline-input" />
        <div class="tagline-actions">
          <button class="btn btn-ghost btn-sm" type="button" onclick={cancelEdit}>
            Cancel
          </button>
          <button class="btn btn-primary btn-sm" type="submit" disabled={saving}>
            {saving ? 'Saving...' : 'Save Override'}
          </button>
        </div>
      </form>
    {:else}
      <div class="tagline-row">
        {#if view.display}
          <p class="header-tagline">{view.display}</p>
        {:else}
          <p class="header-tagline placeholder">No tagline — link a job description or set an override</p>
        {/if}
        {#if view.badge === 'override'}
          <span class="tagline-badge override" title="Using manual override">OVERRIDE</span>
        {:else if view.badge === 'auto'}
          <span class="tagline-badge generated" title="Auto-generated from linked JDs">AUTO</span>
        {/if}
      </div>
      {#if taglineError}
        <p class="tagline-error" role="alert">
          {taglineError}
          <button class="btn btn-sm btn-ghost" type="button" onclick={() => loadTaglineState()}>Retry</button>
        </p>
      {/if}
      <div class="tagline-controls">
        <button class="btn btn-sm btn-ghost" onclick={startEdit}>
          {view.canReset ? 'Edit Override' : 'Set Override'}
        </button>
        <button class="btn btn-sm btn-ghost" onclick={handleRegenerate} disabled={regenerating}>
          {regenerating ? 'Regenerating...' : 'Regenerate'}
        </button>
        {#if view.canReset}
          <button class="btn btn-sm btn-ghost" onclick={handleResetOverride} disabled={saving}>
            Reset to Generated
          </button>
        {/if}
      </div>
    {/if}

    <!-- Clearance one-liner (between tagline and contact info) -->
    {#if header.clearance}
      <div class="clearance-row">
        <p class="header-clearance">{header.clearance}</p>
        <button class="btn btn-sm btn-ghost" onclick={toggleClearanceInHeader} title="Hide clearance from this resume's header">
          Hide
        </button>
      </div>
    {:else if !header.clearance && showClearanceToggle}
      <div class="clearance-row">
        <p class="header-clearance placeholder">Clearance hidden</p>
        <button class="btn btn-sm btn-ghost" onclick={toggleClearanceInHeader} title="Show clearance in this resume's header">
          Show Clearance
        </button>
      </div>
    {/if}

    <div class="header-contact">
      {#if header.location}<span>{header.location}</span>{/if}
      {#if header.email}<span>{header.email}</span>{/if}
      {#if header.phone}<span>{header.phone}</span>{/if}
      {#if header.linkedin}<a href={header.linkedin} target="_blank" rel="noopener">LinkedIn</a>{/if}
      {#if header.github}<a href={header.github} target="_blank" rel="noopener">GitHub</a>{/if}
      {#if header.website}<a href={header.website} target="_blank" rel="noopener">Website</a>{/if}
    </div>

    <a href="/config/profile" class="edit-profile-link">
      Edit contact info in Profile
    </a>
  </div>
</div>

<style>
  .header-display {
    padding: 1.25rem;
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: 8px;
    margin-bottom: 1rem;
    text-align: center;
  }

  .header-display-top {
    display: flex;
    justify-content: center;
    align-items: flex-start;
  }

  .header-name {
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--text-primary);
  }

  .tagline-row {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    margin-top: 0.25rem;
  }

  .header-tagline {
    font-size: 0.95rem;
    color: var(--text-secondary);
  }

  .header-tagline.placeholder {
    color: var(--text-faint);
    font-style: italic;
  }

  .tagline-badge {
    display: inline-block;
    padding: 0.1em 0.45em;
    border-radius: 3px;
    font-size: 0.65rem;
    font-weight: 600;
    letter-spacing: 0.04em;
  }

  .tagline-badge.override {
    background: var(--color-warning-subtle);
    color: var(--color-warning-text);
  }

  .tagline-badge.generated {
    background: var(--color-info-subtle);
    color: var(--color-info-text);
  }

  .tagline-error {
    color: var(--color-danger);
    font-size: 0.8rem;
    margin-top: 0.25rem;
  }

  .tagline-controls {
    display: flex;
    gap: 0.35rem;
    justify-content: center;
    margin-top: 0.35rem;
  }

  .tagline-edit {
    margin-top: 0.5rem;
  }

  .tagline-input {
    width: 100%;
    max-width: 480px;
    padding: 0.5rem 0.75rem;
    border: 1px solid var(--color-border-strong);
    border-radius: 6px;
    font-size: 0.875rem;
    color: var(--text-primary);
    text-align: center;
  }

  .tagline-input:focus {
    outline: none;
    border-color: var(--color-border-focus);
    box-shadow: 0 0 0 2px var(--color-primary-subtle);
  }

  .tagline-actions {
    display: flex;
    justify-content: center;
    gap: 0.5rem;
    margin-top: 0.5rem;
  }

  .header-contact {
    display: flex;
    gap: 1rem;
    justify-content: center;
    flex-wrap: wrap;
    margin-top: 0.5rem;
    font-size: 0.85rem;
    color: var(--text-muted);
  }

  .header-contact a {
    color: var(--color-primary);
    text-decoration: none;
  }

  .header-contact a:hover {
    text-decoration: underline;
  }

  .clearance-row {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    margin-top: 0.35rem;
  }

  .header-clearance {
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--color-success-text);
    margin: 0;
  }

  .header-clearance.placeholder {
    color: var(--text-faint);
    font-style: italic;
    font-weight: normal;
  }

  .edit-profile-link {
    display: inline-block;
    margin-top: 0.75rem;
    font-size: 0.8rem;
    color: var(--color-primary);
    text-decoration: none;
  }

  .edit-profile-link:hover {
    text-decoration: underline;
  }
</style>
