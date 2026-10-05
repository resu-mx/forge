<!--
  JDLinkedResumes.svelte -- "Linked Resumes" section for JD editor.
  Rendered in edit mode only. Manages its own data fetching and state.
-->
<script lang="ts">
  import { onMount } from 'svelte'
  import { forge, friendlyError } from '$lib/sdk'
  import { addToast } from '$lib/stores/toast.svelte'
  import type { ResumeLink } from '@forge/sdk'
  import ResumePickerModal from './ResumePickerModal.svelte'

  let { jdId }: {
    jdId: string
  } = $props()

  let linkedResumes = $state<ResumeLink[]>([])
  let showPicker = $state(false)
  let loading = $state(true)
  let loadError = $state<string | null>(null)

  async function loadLinkedResumes() {
    const result = await forge.jobDescriptions.listResumes(jdId)
    if (result.ok) {
      linkedResumes = result.data
      loadError = null
    } else {
      linkedResumes = []
      loadError = friendlyError(result.error, 'Failed to load linked resumes')
    }
    loading = false
  }

  async function handleUnlink(resumeId: string) {
    const result = await forge.jobDescriptions.unlinkResume(jdId, resumeId)
    if (result.ok) {
      linkedResumes = linkedResumes.filter(r => r.resume_id !== resumeId)
      addToast({ type: 'success', message: 'Resume unlinked' })
    } else {
      addToast({ type: 'error', message: friendlyError(result.error) })
    }
  }

  async function handleLink(resumeId: string) {
    const result = await forge.jobDescriptions.linkResume(jdId, resumeId)
    if (result.ok) {
      linkedResumes = [result.data, ...linkedResumes]
      addToast({ type: 'success', message: 'Resume linked' })
    } else {
      addToast({ type: 'error', message: friendlyError(result.error) })
    }
    showPicker = false
  }

  onMount(() => { loadLinkedResumes() })

  // Reload when jdId prop changes (e.g., switching between JDs)
  $effect(() => {
    const _id = jdId
    if (_id) {
      loading = true
      loadLinkedResumes()
    }
  })
</script>

<section class="jd-linked-resumes">
  <h3>Linked Resumes</h3>

  {#if loading}
    <p class="muted">Loading...</p>
  {:else if loadError}
    <p class="load-error" role="alert">{loadError}</p>
  {:else if linkedResumes.length === 0}
    <p class="muted">No resumes linked to this job description.</p>
  {:else}
    <ul class="linked-list">
      {#each linkedResumes as resume (resume.resume_id)}
        <li class="linked-card">
          <div class="linked-card-info">
            <span class="linked-card-name">{resume.resume_name}</span>
            <span class="status-badge status-{resume.status}">{resume.status}</span>
            <span class="linked-card-target muted">
              {resume.target_role} @ {resume.target_employer}
            </span>
          </div>
          <button
            class="unlink-btn"
            onclick={() => handleUnlink(resume.resume_id)}
            aria-label="Unlink {resume.resume_name}"
          >
            &times;
          </button>
        </li>
      {/each}
    </ul>
  {/if}

  <button class="link-btn" onclick={() => showPicker = true}>
    + Link Resume
  </button>

  {#if showPicker}
    <ResumePickerModal
      excludeIds={linkedResumes.map(r => r.resume_id)}
      onselect={handleLink}
      onclose={() => showPicker = false}
    />
  {/if}
</section>

<style>
  .load-error {
    color: var(--color-danger);
    font-size: 0.8rem;
    margin: 0;
  }

  .jd-linked-resumes {
    margin-top: 0.5rem;
  }

  h3 {
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--text-secondary);
    margin: 0 0 0.5rem 0;
  }

  .muted {
    font-size: 0.8rem;
    color: var(--text-muted);
    margin: 0.25rem 0;
  }

  .linked-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.375rem;
  }

  .linked-card {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.5rem 0.75rem;
    border: 1px solid var(--color-border);
    border-radius: 0.375rem;
    background: var(--color-surface);
  }

  .linked-card-info {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
    min-width: 0;
  }

  .linked-card-name {
    font-weight: 600;
    font-size: 0.85rem;
    color: var(--text-primary);
  }

  .linked-card-target {
    font-size: 0.75rem;
  }

  .status-badge {
    font-size: 0.65rem;
    padding: 0.1rem 0.4rem;
    border-radius: 0.25rem;
    font-weight: 600;
    text-transform: uppercase;
  }

  .status-draft { background: var(--color-info-subtle); color: var(--color-info); }
  .status-in_review { background: var(--color-warning-subtle); color: var(--color-warning-text); }
  .status-approved { background: var(--color-success-subtle); color: var(--color-success-text); }
  .status-rejected { background: var(--color-danger-subtle); color: var(--color-danger-text); }
  .status-archived { background: var(--color-surface-sunken); color: var(--text-muted); }

  .unlink-btn {
    background: none;
    border: none;
    font-size: 1.1rem;
    cursor: pointer;
    color: var(--text-muted);
    padding: 0.15rem 0.35rem;
    border-radius: 0.25rem;
    line-height: 1;
  }

  .unlink-btn:hover {
    background: var(--color-danger-subtle);
    color: var(--color-danger);
  }

  .link-btn {
    margin-top: 0.5rem;
    padding: 0.35rem 0.75rem;
    font-size: 0.8rem;
    background: var(--color-surface);
    border: 1px dashed var(--color-border-strong);
    border-radius: 0.375rem;
    cursor: pointer;
    color: var(--text-secondary);
    font-weight: 500;
  }

  .link-btn:hover {
    border-color: var(--color-info);
    color: var(--color-info);
    background: var(--color-info-subtle);
  }
</style>
