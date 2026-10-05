<!--
  SummaryDetailModal.svelte — Overlay modal for viewing and editing a summary.

  Opens on row click from the Summaries page (Phase 91). Handles:
  - Full CRUD (title, role, description, notes, industry, role type, keyword skills)
  - Clone, promote/demote template, delete
  - Skill keyword picker (existing skills + create new)
  - Tagline field is intentionally NOT shown here — Phase 92 moves tagline
    to resume-level (generated + override). The column still exists but is
    deprecated for summaries.
-->
<script lang="ts">
  import { forge, friendlyError } from '$lib/sdk'
  import { addToast } from '$lib/stores/toast.svelte'
  import { Modal, LoadingSpinner, ConfirmDialog } from '$lib/components'
  import { attachKeywords, keywordSaveToast } from '$lib/summary-keywords'
  import type { Summary, SummaryWithRelations, Industry, RoleType, Skill } from '@forge/sdk'

  let {
    summaryId,
    onclose,
    onupdate,
  }: {
    summaryId: string | 'new'
    onclose: () => void
    onupdate: () => void
  } = $props()

  const isNew = $derived(summaryId === 'new')

  // ── State ─────────────────────────────────────────────────────────────
  let loading = $state(true)
  let saving = $state(false)
  let summary = $state<Summary | null>(null)
  let linkedSkills = $state<Skill[]>([])
  let allIndustries = $state<Industry[]>([])
  let allRoleTypes = $state<RoleType[]>([])
  let allSkills = $state<Skill[]>([])
  let showDeleteConfirm = $state(false)

  // Form fields (bound to inputs)
  let formTitle = $state('')
  let formRole = $state('')
  let formDescription = $state('')
  let formNotes = $state('')
  let formIndustryId = $state<string | null>(null)
  let formRoleTypeId = $state<string | null>(null)

  // Skill picker
  let skillSearch = $state('')
  let showSkillDropdown = $state(false)

  let filteredSkillCandidates = $derived.by(() => {
    if (!skillSearch.trim()) return []
    const q = skillSearch.toLowerCase()
    const linkedIds = new Set(linkedSkills.map((s) => s.id))
    return allSkills
      .filter((s) => !linkedIds.has(s.id) && s.name.toLowerCase().includes(q))
      .slice(0, 10)
  })

  let canCreateSkill = $derived.by(() => {
    if (!skillSearch.trim()) return false
    const q = skillSearch.trim().toLowerCase()
    return !allSkills.some((s) => s.name.toLowerCase() === q)
  })

  // ── Data loading ──────────────────────────────────────────────────────
  $effect(() => {
    loadAll()
  })

  async function loadAll() {
    loading = true

    const [industriesRes, roleTypesRes, skillsRes] = await Promise.all([
      forge.industries.list({ limit: 200 }),
      forge.roleTypes.list({ limit: 200 }),
      forge.skills.list({ limit: 500 }),
    ])
    if (industriesRes.ok) allIndustries = industriesRes.data
    if (roleTypesRes.ok) allRoleTypes = roleTypesRes.data
    if (skillsRes.ok) allSkills = skillsRes.data

    if (isNew) {
      // Start with a blank form
      summary = null
      linkedSkills = []
      formTitle = ''
      formRole = ''
      formDescription = ''
      formNotes = ''
      formIndustryId = null
      formRoleTypeId = null
      loading = false
      return
    }

    const hydratedRes = await forge.summaries.getWithRelations(summaryId)
    if (hydratedRes.ok) {
      const data = hydratedRes.data as SummaryWithRelations
      summary = data
      linkedSkills = data.skills ?? []
      formTitle = data.title
      formRole = data.role ?? ''
      formDescription = data.description ?? ''
      formNotes = data.notes ?? ''
      formIndustryId = data.industry_id
      formRoleTypeId = data.role_type_id
    } else {
      addToast({ message: friendlyError(hydratedRes.error, 'Failed to load summary'), type: 'error' })
      onclose()
    }

    loading = false
  }

  // ── Actions ───────────────────────────────────────────────────────────
  async function save() {
    if (!formTitle.trim()) {
      addToast({ message: 'Title is required', type: 'error' })
      return
    }
    saving = true

    if (isNew) {
      const createRes = await forge.summaries.create({
        title: formTitle.trim(),
        role: formRole || undefined,
        description: formDescription || undefined,
        notes: formNotes || undefined,
        industry_id: formIndustryId,
        role_type_id: formRoleTypeId,
      })
      if (!createRes.ok) {
        addToast({ message: friendlyError(createRes.error, 'Failed to create summary'), type: 'error' })
        saving = false
        return
      }
      summary = createRes.data
      // Keywords picked before Save. Name any that didn't stick (resu-mx/forge#35).
      const failed = await attachKeywords(createRes.data.id, linkedSkills, (id, skillId) =>
        forge.summaries.addSkill(id, skillId),
      )
      addToast({ ...keywordSaveToast(failed), duration: failed.length > 0 ? 8000 : undefined })
      onupdate()
      onclose()
      saving = false
      return
    }

    const updateRes = await forge.summaries.update(summaryId, {
      title: formTitle.trim(),
      role: formRole || null,
      description: formDescription || null,
      notes: formNotes || null,
      industry_id: formIndustryId,
      role_type_id: formRoleTypeId,
    })
    if (updateRes.ok) {
      summary = updateRes.data
      addToast({ message: 'Summary saved', type: 'success' })
      onupdate()
    } else {
      addToast({ message: friendlyError(updateRes.error, 'Failed to save'), type: 'error' })
    }
    saving = false
  }

  async function cloneSummary() {
    if (isNew || !summary) return
    const res = await forge.summaries.clone(summaryId)
    if (res.ok) {
      addToast({ message: `Cloned as "${res.data.title}"`, type: 'success' })
      onupdate()
      onclose()
    } else {
      addToast({ message: friendlyError(res.error, 'Failed to clone'), type: 'error' })
    }
  }

  async function toggleTemplate() {
    if (isNew || !summary) return
    const res = await forge.summaries.toggleTemplate(summaryId)
    if (res.ok) {
      summary = res.data
      addToast({
        message: res.data.is_template ? 'Promoted to template' : 'Demoted from template',
        type: 'success',
      })
      onupdate()
    } else {
      addToast({ message: friendlyError(res.error, 'Failed to toggle'), type: 'error' })
    }
  }

  async function deleteSummary() {
    if (isNew || !summary) return
    const res = await forge.summaries.delete(summaryId)
    if (res.ok) {
      addToast({ message: 'Summary deleted', type: 'success' })
      onupdate()
      onclose()
    } else {
      addToast({ message: friendlyError(res.error, 'Failed to delete'), type: 'error' })
    }
    showDeleteConfirm = false
  }

  // ── Skill keyword picker ─────────────────────────────────────────────

  async function addExistingSkill(skill: Skill) {
    // For existing summaries, persist immediately. For new summaries, queue
    // until the summary is created (pushed through in save()).
    if (isNew) {
      linkedSkills = [...linkedSkills, skill]
      skillSearch = ''
      showSkillDropdown = false
      return
    }
    const res = await forge.summaries.addSkill(summaryId, skill.id)
    if (res.ok) {
      linkedSkills = [...linkedSkills, skill]
      skillSearch = ''
      showSkillDropdown = false
    } else {
      addToast({ message: friendlyError(res.error, 'Failed to add skill'), type: 'error' })
    }
  }

  async function createAndAddSkill() {
    if (!skillSearch.trim()) return
    const created = await forge.skills.create({ name: skillSearch.trim() })
    if (!created.ok) {
      addToast({ message: friendlyError(created.error, 'Failed to create skill'), type: 'error' })
      return
    }
    // Mirror into local allSkills so future searches find it
    allSkills = [...allSkills, created.data]
    await addExistingSkill(created.data)
  }

  async function removeSkill(skillId: string) {
    if (isNew) {
      linkedSkills = linkedSkills.filter((s) => s.id !== skillId)
      return
    }
    const res = await forge.summaries.removeSkill(summaryId, skillId)
    if (res.ok) {
      linkedSkills = linkedSkills.filter((s) => s.id !== skillId)
    } else {
      addToast({ message: friendlyError(res.error, 'Failed to remove skill'), type: 'error' })
    }
  }
</script>

<Modal open={true} onClose={onclose} size="lg">
  {#snippet header()}
    <div class="modal-header">
      <h3 class="modal-title">
        {isNew ? 'New Summary' : (summary?.is_template ? '★ ' : '') + (summary?.title ?? 'Summary')}
      </h3>
      <button class="btn-icon modal-close" onclick={onclose} aria-label="Close">&times;</button>
    </div>
  {/snippet}

  {#snippet body()}
    {#if loading}
      <div class="loading-container">
        <LoadingSpinner size="lg" message="Loading summary..." />
      </div>
    {:else}
      <div class="form-grid">
        <div class="field">
          <label for="summary-title" class="field-label">Title <span class="required">*</span></label>
          <input id="summary-title" bind:value={formTitle} class="field-input" placeholder="e.g. Cloud Security Summary" />
        </div>

        <div class="field">
          <label for="summary-role" class="field-label">Role</label>
          <input id="summary-role" bind:value={formRole} class="field-input" placeholder="e.g. Senior Security Engineer" />
        </div>

        <div class="field-row">
          <div class="field">
            <label for="summary-industry" class="field-label">Industry</label>
            <select id="summary-industry" bind:value={formIndustryId} class="field-input">
              <option value={null}>-- None --</option>
              {#each allIndustries as industry (industry.id)}
                <option value={industry.id}>{industry.name}</option>
              {/each}
            </select>
          </div>

          <div class="field">
            <label for="summary-role-type" class="field-label">Role Type</label>
            <select id="summary-role-type" bind:value={formRoleTypeId} class="field-input">
              <option value={null}>-- None --</option>
              {#each allRoleTypes as rt (rt.id)}
                <option value={rt.id}>{rt.name}</option>
              {/each}
            </select>
          </div>
        </div>

        <div class="field">
          <label for="summary-description" class="field-label">Description</label>
          <textarea id="summary-description" bind:value={formDescription} rows="4" class="field-input"
                    placeholder="Full summary paragraph..."></textarea>
        </div>

        <div class="field">
          <span class="field-label">Keyword Skills</span>
          <div class="keyword-pills">
            {#each linkedSkills as skill (skill.id)}
              <span class="pill">
                {skill.name}
                <button class="pill-remove" onclick={() => removeSkill(skill.id)} aria-label="Remove {skill.name}">&times;</button>
              </span>
            {/each}
            {#if linkedSkills.length === 0}
              <span class="empty-hint">No keywords yet. Add skills below.</span>
            {/if}
          </div>
          <div class="skill-picker">
            <input
              type="text"
              class="field-input"
              placeholder="Search or create skill..."
              bind:value={skillSearch}
              onfocus={() => showSkillDropdown = true}
              onblur={() => setTimeout(() => showSkillDropdown = false, 200)}
              onkeydown={(e) => {
                if (e.key === 'Enter' && canCreateSkill) {
                  e.preventDefault()
                  createAndAddSkill()
                }
              }}
            />
            {#if showSkillDropdown && (filteredSkillCandidates.length > 0 || canCreateSkill)}
              <div class="skill-dropdown">
                {#each filteredSkillCandidates as skill (skill.id)}
                  <button class="dropdown-item" onmousedown={() => addExistingSkill(skill)}>
                    {skill.name}
                    {#if skill.category}<span class="dropdown-category">{skill.category}</span>{/if}
                  </button>
                {/each}
                {#if canCreateSkill}
                  <button class="dropdown-item create-item" onmousedown={createAndAddSkill}>
                    Create "{skillSearch.trim()}"
                  </button>
                {/if}
              </div>
            {/if}
          </div>
        </div>

        <div class="field">
          <label for="summary-notes" class="field-label">Notes (internal)</label>
          <textarea id="summary-notes" bind:value={formNotes} rows="2" class="field-input"></textarea>
        </div>

        {#if !isNew && summary}
          {#if summary.is_template}
            <div class="banner banner-info">
              This is a template. Changes here will NOT affect resumes previously created from it.
            </div>
          {:else if summary.linked_resume_count > 1}
            <div class="banner banner-warn">
              Linked to {summary.linked_resume_count} resumes. Changes propagate to all of them.
            </div>
          {/if}
        {/if}
      </div>
    {/if}
  {/snippet}

  {#snippet footer()}
    {#if !loading}
      <div class="footer-row">
        <div class="footer-left">
          {#if !isNew}
            <button class="btn btn-ghost" onclick={cloneSummary}>Clone</button>
            <button class="btn btn-ghost" onclick={toggleTemplate}>
              {summary?.is_template ? 'Demote from Template' : 'Promote to Template'}
            </button>
          {/if}
        </div>
        <div class="footer-right">
          {#if !isNew}
            <button class="btn btn-danger" onclick={() => showDeleteConfirm = true}>Delete</button>
          {/if}
          <button class="btn btn-ghost" onclick={onclose}>Cancel</button>
          <button class="btn btn-primary" onclick={save} disabled={saving}>
            {saving ? 'Saving...' : (isNew ? 'Create' : 'Save')}
          </button>
        </div>
      </div>
    {/if}
  {/snippet}
</Modal>

<ConfirmDialog
  open={showDeleteConfirm}
  title="Delete Summary"
  message="Are you sure? This will detach this summary from any linked resumes."
  confirmLabel="Delete"
  onconfirm={deleteSummary}
  oncancel={() => showDeleteConfirm = false}
/>

<style>
  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 1rem 1.25rem;
    border-bottom: 1px solid var(--color-border);
  }
  .modal-title {
    font-size: var(--text-lg);
    font-weight: var(--font-semibold);
    color: var(--text-primary);
    margin: 0;
  }
  .btn-icon {
    background: none;
    border: none;
    font-size: 1.5rem;
    color: var(--text-faint);
    cursor: pointer;
    padding: 0.2rem;
    line-height: 1;
  }
  .btn-icon:hover { color: var(--text-secondary); }

  .loading-container {
    display: flex;
    justify-content: center;
    padding: 3rem 0;
  }

  .form-grid {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .field-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
  }

  .field-label {
    font-size: var(--text-sm);
    font-weight: var(--font-medium);
    color: var(--text-secondary);
  }

  .required { color: var(--color-danger); }

  .field-input {
    padding: 0.5rem 0.65rem;
    border: 1px solid var(--color-border-strong);
    border-radius: var(--radius-md);
    font-size: var(--text-base);
    color: var(--text-primary);
    background: var(--color-surface);
    font-family: inherit;
  }

  .field-input:focus {
    outline: none;
    border-color: var(--color-border-focus);
    box-shadow: 0 0 0 2px var(--color-primary-subtle);
  }

  textarea.field-input {
    resize: vertical;
    min-height: 60px;
    line-height: 1.5;
  }

  .keyword-pills {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
    min-height: 1.75rem;
    padding: 0.25rem 0;
  }

  .pill {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.2em 0.55em;
    background: var(--color-info-subtle);
    color: var(--color-info-text);
    border-radius: var(--radius-sm);
    font-size: var(--text-sm);
    font-weight: var(--font-medium);
  }

  .pill-remove {
    background: none;
    border: none;
    color: inherit;
    font-size: 1rem;
    line-height: 1;
    cursor: pointer;
    opacity: 0.6;
    padding: 0;
  }
  .pill-remove:hover { opacity: 1; }

  .empty-hint {
    font-size: var(--text-sm);
    color: var(--text-faint);
    font-style: italic;
  }

  .skill-picker {
    position: relative;
  }

  .skill-dropdown {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    background: var(--color-surface);
    border: 1px solid var(--color-border-strong);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-md, 0 4px 12px rgba(0, 0, 0, 0.1));
    max-height: 200px;
    overflow-y: auto;
    z-index: 10;
    margin-top: 2px;
  }

  .dropdown-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 0.45rem 0.65rem;
    border: none;
    background: none;
    font-size: var(--text-sm);
    color: var(--text-secondary);
    cursor: pointer;
    text-align: left;
    font-family: inherit;
  }

  .dropdown-item:hover {
    background: var(--color-ghost);
  }

  .dropdown-category {
    font-size: var(--text-xs);
    color: var(--text-faint);
  }

  .create-item {
    color: var(--color-primary);
    font-weight: var(--font-medium);
    border-top: 1px solid var(--color-border);
  }

  .banner {
    border-radius: var(--radius-md);
    padding: 0.6rem 0.85rem;
    font-size: var(--text-sm);
    line-height: 1.4;
  }
  .banner-info {
    background: var(--color-info-subtle);
    border: 1px solid var(--color-info-border);
    color: var(--color-info-text);
  }
  .banner-warn {
    background: var(--color-warning-subtle);
    border: 1px solid var(--color-warning-border);
    color: var(--color-warning-text);
  }

  .footer-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    gap: 0.75rem;
  }
  .footer-left { display: flex; gap: 0.5rem; }
  .footer-right { display: flex; gap: 0.5rem; }
</style>
