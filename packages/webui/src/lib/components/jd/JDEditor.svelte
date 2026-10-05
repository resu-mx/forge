<!--
  JDEditor.svelte -- JD editor form for create + edit modes.
  Status changes save immediately. Other changes require Save.
-->
<script lang="ts">
  import { forge, friendlyError } from '$lib/sdk'
  import { addToast } from '$lib/stores/toast.svelte'
  import { ConfirmDialog, EntityNotes } from '$lib/components'
  import JDSkillPicker from './JDSkillPicker.svelte'
  import JDSkillExtraction from './JDSkillExtraction.svelte'
  import JDLinkedResumes from './JDLinkedResumes.svelte'
  import JDSkillRadar from '$lib/components/charts/JDSkillRadar.svelte'
  import CompensationBulletGraph from '$lib/components/charts/CompensationBulletGraph.svelte'
  import type { JobDescriptionWithOrg, Organization, Skill } from '@forge/sdk'

  let {
    jd = null,
    organizations = [],
    createMode = false,
    oncreated,
    onupdated,
    ondeleted,
  }: {
    jd: JobDescriptionWithOrg | null
    organizations: Organization[]
    createMode?: boolean
    oncreated: (jd: JobDescriptionWithOrg) => void
    onupdated: (jd: JobDescriptionWithOrg) => void
    ondeleted: (id: string) => void
  } = $props()

  // Form state
  let title = $state('')
  let organizationId = $state<string | null>(null)
  let status = $state('discovered')
  let location = $state('')
  let salaryRange = $state('')
  let salaryMin = $state<number | null>(null)
  let salaryMax = $state<number | null>(null)
  let url = $state('')
  let rawText = $state('')

  let jdSkills = $state<Skill[]>([])
  let skillsError = $state<string | null>(null)
  let saving = $state(false)
  let confirmDeleteOpen = $state(false)

  // Track dirty state for non-status fields
  let isDirty = $derived.by(() => {
    if (createMode || !jd) return false
    return (
      title !== jd.title ||
      organizationId !== (jd.organization_id ?? null) ||
      location !== (jd.location ?? '') ||
      salaryRange !== (jd.salary_range ?? '') ||
      salaryMin !== (jd.salary_min ?? null) ||
      salaryMax !== (jd.salary_max ?? null) ||
      url !== (jd.url ?? '') ||
      rawText !== jd.raw_text
    )
  })

  // Populate form when jd changes
  $effect(() => {
    if (jd && !createMode) {
      title = jd.title
      organizationId = jd.organization_id ?? null
      status = jd.status
      location = jd.location ?? ''
      salaryRange = jd.salary_range ?? ''
      salaryMin = jd.salary_min ?? null
      salaryMax = jd.salary_max ?? null
      url = jd.url ?? ''
      rawText = jd.raw_text
      loadSkills(jd.id)
    } else if (createMode) {
      title = ''
      organizationId = null
      status = 'discovered'
      location = ''
      salaryRange = ''
      salaryMin = null
      salaryMax = null
      url = ''
      rawText = ''
      jdSkills = []
    }
  })

  async function loadSkills(jdId: string) {
    skillsError = null
    const res = await forge.jobDescriptions.listSkills(jdId)
    if (res.ok) {
      jdSkills = res.data
    } else {
      jdSkills = []
      skillsError = friendlyError(res.error, 'Failed to load required skills')
    }
  }

  async function handleStatusChange(e: Event) {
    const newStatus = (e.target as HTMLSelectElement).value
    if (!jd || createMode) {
      status = newStatus
      return
    }
    // Immediate save for status change in edit mode
    const res = await forge.jobDescriptions.update(jd.id, { status: newStatus as any })
    if (res.ok) {
      status = newStatus
      onupdated(res.data)
      addToast({ type: 'success', message: `Status changed to ${newStatus}` })
    } else {
      addToast({ type: 'error', message: friendlyError(res.error) })
    }
  }

  async function handleSave() {
    if (!title.trim()) {
      addToast({ type: 'error', message: 'Title is required' })
      return
    }
    if (!rawText.trim()) {
      addToast({ type: 'error', message: 'Description is required' })
      return
    }

    saving = true
    const payload = {
      title: title.trim(),
      organization_id: organizationId,
      status: status as any,
      location: location.trim() || null,
      salary_range: salaryRange.trim() || null,
      salary_min: salaryMin ?? null,
      salary_max: salaryMax ?? null,
      url: url.trim() || null,
      raw_text: rawText.trim(),
    }

    if (createMode) {
      const res = await forge.jobDescriptions.create(payload as any)
      if (res.ok) {
        oncreated(res.data)
        addToast({ type: 'success', message: 'Job description created' })
      } else {
        addToast({ type: 'error', message: friendlyError(res.error) })
      }
    } else if (jd) {
      const res = await forge.jobDescriptions.update(jd.id, payload)
      if (res.ok) {
        onupdated(res.data)
        addToast({ type: 'success', message: 'Job description updated' })
      } else {
        addToast({ type: 'error', message: friendlyError(res.error) })
      }
    }
    saving = false
  }

  async function handleDelete() {
    if (!jd) return
    const res = await forge.jobDescriptions.delete(jd.id)
    if (res.ok) {
      ondeleted(jd.id)
      addToast({ type: 'success', message: 'Job description deleted' })
    } else {
      addToast({ type: 'error', message: friendlyError(res.error) })
    }
    confirmDeleteOpen = false
  }
</script>

<div class="editor">
  <div class="field">
    <label for="jd-title">Title <span class="required">*</span></label>
    <input id="jd-title" type="text" bind:value={title} placeholder="Job title" />
  </div>

  <div class="field">
    <label for="jd-org">Organization</label>
    <select id="jd-org" bind:value={organizationId}>
      <option value={null}>None</option>
      {#each [...organizations].sort((a, b) => a.name.localeCompare(b.name)) as org (org.id)}
        <option value={org.id}>{org.name}</option>
      {/each}
    </select>
  </div>

  <div class="field">
    <label for="jd-status">Status</label>
    <select id="jd-status" value={status} onchange={handleStatusChange}>
      <option value="discovered">Discovered</option>
      <option value="analyzing">Analyzing</option>
      <option value="applying">Applying</option>
      <option value="applied">Applied</option>
      <option value="interviewing">Interviewing</option>
      <option value="offered">Offered</option>
      <option value="rejected">Rejected</option>
      <option value="withdrawn">Withdrawn</option>
      <option value="closed">Closed</option>
    </select>
  </div>

  <div class="field-row">
    <div class="field half">
      <label for="jd-location">Location</label>
      <input id="jd-location" type="text" bind:value={location} placeholder="Remote, San Francisco, CA" />
    </div>
    <div class="field half">
      <label for="jd-salary">Salary Range (text)</label>
      <input id="jd-salary" type="text" bind:value={salaryRange} placeholder="$150k-$200k" />
    </div>
  </div>

  <div class="field-row">
    <div class="field half">
      <label for="jd-salary-min">Min ($)</label>
      <input id="jd-salary-min" type="number" bind:value={salaryMin} placeholder="150000" step="1000" />
    </div>
    <div class="field half">
      <label for="jd-salary-max">Max ($)</label>
      <input id="jd-salary-max" type="number" bind:value={salaryMax} placeholder="200000" step="1000" />
    </div>
  </div>

  <div class="field">
    <label for="jd-url">URL</label>
    <div class="url-field">
      <input id="jd-url" type="url" bind:value={url} placeholder="https://..." />
      {#if url.trim() && !createMode}
        <a href={url} target="_blank" rel="noopener noreferrer" class="url-link">
          Open &#8599;
        </a>
      {/if}
    </div>
  </div>

  <div class="field">
    <label for="jd-description">Description <span class="required">*</span></label>
    <textarea
      id="jd-description"
      bind:value={rawText}
      placeholder="Paste the full job description here..."
      rows="10"
      class="description-textarea"
    ></textarea>
  </div>

  {#if !createMode && jd}
    <div class="field">
      <label>Required Skills</label>
      {#if skillsError}
        <p class="skills-error" role="alert">
          {skillsError}
          <button type="button" class="retry-btn" onclick={() => jd && loadSkills(jd.id)}>Retry</button>
        </p>
      {/if}
      <JDSkillPicker jdId={jd.id} bind:jdSkills />
      <JDSkillExtraction
        jdId={jd.id}
        {jdSkills}
        {forge}
        onSkillsChanged={() => loadSkills(jd.id)}
      />
    </div>

    <JDSkillRadar jdId={jd.id} />

    <CompensationBulletGraph
      jdTitle={jd.title}
      salaryMin={salaryMin}
      salaryMax={salaryMax}
    />

    <div class="field">
      <JDLinkedResumes jdId={jd.id} />
    </div>
  {/if}

  <EntityNotes entityType="job_description" entityId={jd?.id} />

  <div class="actions">
    <button
      class="btn-primary"
      onclick={handleSave}
      disabled={saving || (!createMode && !isDirty)}
    >
      {saving ? 'Saving...' : createMode ? 'Create' : 'Save'}
    </button>
    {#if !createMode && jd}
      <button
        class="btn-danger"
        onclick={() => (confirmDeleteOpen = true)}
        type="button"
      >
        Delete
      </button>
    {/if}
  </div>
</div>

<ConfirmDialog
  open={confirmDeleteOpen}
  title="Delete Job Description"
  message="Are you sure you want to delete this job description? This cannot be undone."
  onconfirm={handleDelete}
  oncancel={() => (confirmDeleteOpen = false)}
/>

<style>
  .skills-error {
    color: var(--color-danger);
    font-size: 0.8rem;
    margin: 0;
  }

  .retry-btn {
    margin-left: 0.5rem;
    padding: 0.1rem 0.5rem;
    font-size: 0.75rem;
    background: var(--color-surface);
    border: 1px solid var(--color-border-strong);
    border-radius: 0.25rem;
    color: var(--text-secondary);
    cursor: pointer;
  }

  .retry-btn:hover {
    background: var(--color-surface-raised);
  }

  .editor {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    padding: 1rem;
    overflow-y: auto;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .field-row {
    display: flex;
    gap: 1rem;
  }

  .half {
    flex: 1;
  }

  label {
    font-size: 0.8rem;
    font-weight: 600;
    color: var(--text-secondary);
  }

  .required {
    color: var(--color-danger);
  }

  input[type="text"],
  input[type="url"],
  input[type="number"],
  select,
  textarea {
    padding: 0.5rem 0.6rem;
    border: 1px solid var(--color-border-strong);
    border-radius: 0.375rem;
    font-size: 0.9rem;
    outline: none;
    font-family: inherit;
  }

  input:focus,
  select:focus,
  textarea:focus {
    border-color: var(--color-info);
    box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.15);
  }

  .description-textarea {
    min-height: 200px;
    white-space: pre-wrap;
    font-family: inherit;
    resize: vertical;
  }

  .url-field {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .url-field input {
    flex: 1;
  }

  .url-link {
    font-size: 0.8rem;
    color: var(--color-info);
    text-decoration: none;
    white-space: nowrap;
  }

  .url-link:hover {
    text-decoration: underline;
  }

  .actions {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding-top: 0.5rem;
    border-top: 1px solid var(--color-border);
  }

</style>
