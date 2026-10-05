<script lang="ts">
  import { onMount } from 'svelte'
  import type { ResumeTemplate, TemplateSectionDef } from '@forge/sdk'
  import { forge, friendlyError } from '$lib/sdk'
  import { addToast } from '$lib/stores/toast.svelte'
  import { LoadingSpinner, EmptyState, ConfirmDialog, PageHeader } from '$lib/components'

  const VALID_ENTRY_TYPES = [
    'experience', 'skills', 'education', 'projects',
    'clearance', 'presentations', 'certifications', 'awards', 'freeform',
  ]

  const ENTRY_TYPE_LABELS: Record<string, string> = {
    experience: 'Experience',
    skills: 'Skills',
    education: 'Education',
    projects: 'Projects',
    clearance: 'Clearance',
    presentations: 'Presentations',
    certifications: 'Certifications',
    awards: 'Awards',
    freeform: 'Freeform',
  }

  // ---- State ----
  let templates = $state<ResumeTemplate[]>([])
  let loading = $state(true)
  let saving = $state(false)

  // Create/Edit form
  let showForm = $state(false)
  let editingId = $state<string | null>(null)
  let formName = $state('')
  let formDescription = $state('')
  let formSections = $state<TemplateSectionDef[]>([])

  // Delete confirmation
  let deleteTarget = $state<ResumeTemplate | null>(null)

  // ---- Load ----
  async function loadTemplates() {
    loading = true
    const result = await forge.templates.list()
    if (result.ok) {
      templates = result.data
    } else {
      addToast({ message: friendlyError(result.error), type: 'error' })
    }
    loading = false
  }

  onMount(loadTemplates)

  // ---- Create/Edit ----
  function openCreateForm() {
    editingId = null
    formName = ''
    formDescription = ''
    formSections = [{ title: '', entry_type: 'experience', position: 0 }]
    showForm = true
  }

  function openEditForm(template: ResumeTemplate) {
    editingId = template.id
    formName = template.name
    formDescription = template.description ?? ''
    formSections = template.sections.map((s, i) => ({ ...s, position: i }))
    showForm = true
  }

  function cancelForm() {
    showForm = false
    editingId = null
  }

  function addSection() {
    formSections = [...formSections, {
      title: '',
      entry_type: 'experience',
      position: formSections.length,
    }]
  }

  function removeSection(index: number) {
    formSections = formSections.filter((_, i) => i !== index).map((s, i) => ({ ...s, position: i }))
  }

  function moveSection(index: number, direction: -1 | 1) {
    const newIndex = index + direction
    if (newIndex < 0 || newIndex >= formSections.length) return
    const updated = [...formSections]
    const temp = updated[index]
    updated[index] = updated[newIndex]
    updated[newIndex] = temp
    formSections = updated.map((s, i) => ({ ...s, position: i }))
  }

  async function handleSave() {
    if (!formName.trim()) {
      addToast({ message: 'Template name is required', type: 'error' })
      return
    }
    if (formSections.length === 0) {
      addToast({ message: 'At least one section is required', type: 'error' })
      return
    }
    for (const s of formSections) {
      if (!s.title.trim()) {
        addToast({ message: 'All section titles are required', type: 'error' })
        return
      }
    }

    saving = true
    try {
      if (editingId) {
        const result = await forge.templates.update(editingId, {
          name: formName.trim(),
          description: formDescription.trim() || undefined,
          sections: formSections,
        })
        if (result.ok) {
          addToast({ message: `Template "${result.data.name}" updated`, type: 'success' })
          showForm = false
          editingId = null
          await loadTemplates()
        } else {
          addToast({ message: friendlyError(result.error), type: 'error' })
        }
      } else {
        const result = await forge.templates.create({
          name: formName.trim(),
          description: formDescription.trim() || undefined,
          sections: formSections,
        })
        if (result.ok) {
          addToast({ message: `Template "${result.data.name}" created`, type: 'success' })
          showForm = false
          await loadTemplates()
        } else {
          addToast({ message: friendlyError(result.error), type: 'error' })
        }
      }
    } finally {
      saving = false
    }
  }

  // ---- Delete ----
  async function handleDelete() {
    if (!deleteTarget) return
    const result = await forge.templates.delete(deleteTarget.id)
    if (result.ok) {
      addToast({ message: `Template "${deleteTarget.name}" deleted`, type: 'success' })
      deleteTarget = null
      await loadTemplates()
    } else {
      addToast({ message: friendlyError(result.error), type: 'error' })
      deleteTarget = null
    }
  }
</script>

<div class="templates-page">
  <PageHeader title="Templates" subtitle="Reusable resume section layouts">
    {#snippet actions()}
      <button class="btn btn-primary" onclick={openCreateForm} disabled={showForm}>
        + Create Template
      </button>
    {/snippet}
  </PageHeader>

  {#if loading}
    <LoadingSpinner />
  {:else if showForm}
    <!-- Create/Edit Form -->
    <div class="form-card">
      <h2>{editingId ? 'Edit' : 'Create'} Template</h2>
      <div class="form-group">
        <label for="template-name">Name</label>
        <input id="template-name" type="text" bind:value={formName} placeholder="e.g. Standard Tech Resume" />
      </div>
      <div class="form-group">
        <label for="template-desc">Description (optional)</label>
        <textarea id="template-desc" bind:value={formDescription} rows="2" placeholder="Brief description of this layout"></textarea>
      </div>

      <h3>Sections</h3>
      {#each formSections as section, i}
        <div class="section-row">
          <span class="section-num">{i + 1}.</span>
          <input type="text" bind:value={section.title} placeholder="Section title" class="section-title-input" />
          <select bind:value={section.entry_type} class="section-type-select">
            {#each VALID_ENTRY_TYPES as et}
              <option value={et}>{ENTRY_TYPE_LABELS[et]}</option>
            {/each}
          </select>
          <div class="section-actions">
            <button class="btn-icon" onclick={() => moveSection(i, -1)} disabled={i === 0} title="Move up">&#9650;</button>
            <button class="btn-icon" onclick={() => moveSection(i, 1)} disabled={i === formSections.length - 1} title="Move down">&#9660;</button>
            <button class="btn-icon btn-danger" onclick={() => removeSection(i)} title="Remove section">&#10005;</button>
          </div>
        </div>
      {/each}
      <button class="btn btn-outline" onclick={addSection}>+ Add Section</button>

      <div class="form-actions">
        <button class="btn" onclick={cancelForm}>Cancel</button>
        <button class="btn btn-primary" onclick={handleSave} disabled={saving}>
          {saving ? 'Saving...' : (editingId ? 'Update' : 'Create')}
        </button>
      </div>
    </div>
  {:else if templates.length === 0}
    <EmptyState title="No templates yet" description="Create a template to define reusable resume section layouts." />
  {:else}
    <div class="template-list">
      {#each templates as template}
        <div class="template-card">
          <div class="template-header">
            <div class="template-title">
              <h3>{template.name}</h3>
              {#if template.is_builtin}
                <span class="badge badge-builtin">Built-in</span>
              {/if}
            </div>
            <div class="template-actions">
              <button class="btn btn-sm" onclick={() => openEditForm(template)}>Edit</button>
              <button
                class="btn btn-sm btn-danger"
                onclick={() => deleteTarget = template}
                disabled={template.is_builtin}
                title={template.is_builtin ? 'Built-in templates cannot be deleted' : 'Delete template'}
              >Delete</button>
            </div>
          </div>
          {#if template.description}
            <p class="template-desc">{template.description}</p>
          {/if}
          <div class="template-sections">
            {#each template.sections as section, i}
              <span class="section-tag">{i + 1}. {section.title} <small>({ENTRY_TYPE_LABELS[section.entry_type] ?? section.entry_type})</small></span>
            {/each}
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

{#if deleteTarget}
  <ConfirmDialog
    open={deleteTarget !== null}
    title="Delete Template"
    message={`Are you sure you want to delete "${deleteTarget.name}"?`}
    confirmLabel="Delete"
    onconfirm={handleDelete}
    oncancel={() => deleteTarget = null}
  />
{/if}

<style>
  .templates-page { max-width: 900px; }

  .btn-outline { background: transparent; border: 1px dashed var(--text-faint); color: var(--text-muted); margin-top: 0.5rem; }
  .btn-outline:hover { border-color: var(--color-info); color: var(--color-info); }

  .form-card { background: var(--color-surface); border: 1px solid var(--color-border); border-radius: var(--radius-lg); padding: 1.5rem; }
  .form-card h2 { font-size: var(--text-xl); font-weight: var(--font-semibold); margin-bottom: 1rem; }
  .form-card h3 { font-size: var(--text-base); font-weight: var(--font-semibold); margin: 1rem 0 0.5rem; }
  .form-group { margin-bottom: 0.75rem; }
  .form-group label { display: block; font-size: var(--text-sm); font-weight: var(--font-medium); margin-bottom: 0.25rem; color: var(--text-secondary); }
  .form-group input, .form-group textarea { width: 100%; padding: 0.4rem 0.6rem; border: 1px solid var(--color-border-strong); border-radius: var(--radius-md); font-size: var(--text-sm); }
  .form-actions { display: flex; gap: 0.5rem; justify-content: flex-end; margin-top: 1rem; padding-top: 1rem; border-top: 1px solid var(--color-border); }

  .section-row { display: flex; align-items: center; gap: 0.5rem; margin-bottom: 0.5rem; }
  .section-num { font-size: var(--text-sm); font-weight: var(--font-semibold); color: var(--text-muted); min-width: 1.5rem; }
  .section-title-input { flex: 1; padding: 0.35rem 0.5rem; border: 1px solid var(--color-border-strong); border-radius: var(--radius-md); font-size: var(--text-sm); }
  .section-type-select { padding: 0.35rem 0.5rem; border: 1px solid var(--color-border-strong); border-radius: var(--radius-md); font-size: var(--text-sm); background: var(--color-surface); }
  .section-actions { display: flex; gap: 0.15rem; }

  .template-list { display: flex; flex-direction: column; gap: 0.75rem; }
  .template-card { background: var(--color-surface); border: 1px solid var(--color-border); border-radius: var(--radius-lg); padding: 1rem; }
  .template-header { display: flex; justify-content: space-between; align-items: flex-start; }
  .template-title { display: flex; align-items: center; gap: 0.5rem; }
  .template-title h3 { font-size: var(--text-lg); font-weight: var(--font-semibold); margin: 0; }
  .template-actions { display: flex; gap: 0.25rem; }
  .template-desc { font-size: var(--text-sm); color: var(--text-muted); margin: 0.25rem 0 0.5rem; }
  .template-sections { display: flex; flex-wrap: wrap; gap: 0.35rem; margin-top: 0.5rem; }
  .section-tag { font-size: var(--text-sm); background: var(--color-ghost); padding: 0.2rem 0.5rem; border-radius: var(--radius-sm); color: var(--text-secondary); }
  .section-tag small { color: var(--text-faint); }

  .badge { font-size: var(--text-xs); padding: 0.1rem 0.4rem; border-radius: var(--radius-sm); font-weight: var(--font-semibold); }
  .badge-builtin { background: var(--color-info-subtle); color: var(--color-info-text); }
</style>
