<script lang="ts">
  import { dndzone } from 'svelte-dnd-action'
  import { forge, friendlyError } from '$lib/sdk'
  import { addToast } from '$lib/stores/toast.svelte'
  import { openChainView } from '$lib/stores/chain-view.svelte'
  import HeaderEditor from './HeaderEditor.svelte'
  import AddSectionDropdown from './AddSectionDropdown.svelte'
  import ResumeSummaryCard from './ResumeSummaryCard.svelte'
  import type {
    ResumeDocument,
    ExperienceGroup,
    ExperienceBullet,
    ExperienceSubheading,
    SkillGroup,
    EducationItem,
    ProjectItem,
    CertificationGroup,
    ClearanceItem,
    PresentationItem,
    SummaryItem,
  } from '@forge/sdk'

  let {
    ir,
    resumeId,
    onUpdate,
    onAddEntry,
    onAddSection,
    onDeleteSection,
    onRenameSection,
    onMoveSection,
    onRemoveEntry,
    onRemoveCertification,
    onUpdateSummary,
  }: {
    ir: ResumeDocument
    resumeId: string
    onUpdate: () => Promise<void>
    onAddEntry?: (sectionId: string, entryType: string, sourceId?: string, sourceLabel?: string) => void
    onAddSection?: (entryType: string, title: string) => void
    onDeleteSection?: (sectionId: string) => void
    onRenameSection?: (sectionId: string, newTitle: string) => void
    onMoveSection?: (sectionId: string, direction: 'up' | 'down') => void
    /**
     * Remove a single resume entry by id. Wired to per-entry delete
     * buttons on education, clearance, and project sections so users
     * can remove one item without tearing down the whole section.
     */
    onRemoveEntry?: (entryId: string) => Promise<void>
    /**
     * Remove a certification from the resume by resume_certifications.id.
     * Cert entries use a separate junction table, so they cannot share
     * the onRemoveEntry path which targets resume_entries.id.
     */
    onRemoveCertification?: (rcId: string) => Promise<void>
    onUpdateSummary?: (update: {
      summary_id?: string | null
      summary_override?: string | null
    }) => Promise<void>
  } = $props()

  // Inline editing
  let editingEntryId = $state<string | null>(null)
  let editContent = $state('')
  let editSaving = $state(false)

  // Section rename
  let editingSectionId = $state<string | null>(null)
  let editSectionTitle = $state('')

  // Section delete confirmation
  let deleteSectionConfirm = $state<{ id: string; title: string } | null>(null)

  // Provenance tooltip
  let tooltipEntry = $state<ExperienceBullet | null>(null)
  let tooltipPosition = $state({ x: 0, y: 0 })

  // ── Education display helpers ──────────────────────────────────────
  // Format degree_type (e.g. "BS") when present; otherwise fall back to a
  // human-readable form of degree_level (e.g. "graduate_certificate" → "Graduate Cert").
  /**
   * Format the full degree display line: "DegreeType Title".
   *
   * Examples:
   *   - "BS Cybersecurity & Information Assurance" (degree_type + degree title)
   *   - "Graduate Certificate, Cloud Security" (degree_level label + title)
   *   - "Engineering" (no degree type/level → title only)
   *
   * The order is: type FIRST, title SECOND — matching resume convention
   * where the credential abbreviation leads (BS, MS, AAS, PhD).
   */
  function formatDegreeLine(edu: EducationItem): string {
    const type = edu.degree_type ?? formatDegreeLevel(edu.degree_level)
    const title = edu.degree || ''
    if (type && title) return `${type} ${title}`
    return type || title || ''
  }

  /** Map degree_level enum → human-readable label. */
  function formatDegreeLevel(level: string | null | undefined): string | null {
    if (!level) return null
    const levelLabels: Record<string, string> = {
      bachelors: 'Bachelors',
      masters: 'Masters',
      doctoral: 'Doctoral',
      associate: 'Associates',
      graduate_certificate: 'Graduate Certificate',
    }
    return levelLabels[level] ?? level.replace(/_/g, ' ').replace(/\b\w/g, c => c.toUpperCase())
  }

  function startEdit(bullet: ExperienceBullet) {
    editingEntryId = bullet.entry_id
    editContent = bullet.content
  }

  async function saveEdit(entryId: string) {
    editSaving = true
    try {
      const result = await forge.resumes.updateEntry(resumeId, entryId, {
        content: editContent,
      })
      if (result.ok) {
        addToast({ message: 'Entry updated (cloned)', type: 'success' })
        editingEntryId = null
        await onUpdate()
      } else {
        addToast({ message: friendlyError(result.error), type: 'error' })
      }
    } finally {
      editSaving = false
    }
  }

  async function resetClone(entryId: string) {
    const result = await forge.resumes.updateEntry(resumeId, entryId, {
      content: null,
    })
    if (result.ok) {
      addToast({ message: 'Entry reset to reference', type: 'success' })
      await onUpdate()
    } else {
      addToast({ message: friendlyError(result.error), type: 'error' })
    }
  }

  // Section rename helpers
  function startRenameSection(sectionId: string, currentTitle: string) {
    editingSectionId = sectionId
    editSectionTitle = currentTitle
  }

  async function saveRename(sectionId: string) {
    if (editSectionTitle.trim() && onRenameSection) {
      onRenameSection(sectionId, editSectionTitle.trim())
    }
    editingSectionId = null
  }

  function confirmDeleteSection(sectionId: string, title: string) {
    deleteSectionConfirm = { id: sectionId, title }
  }

  function executeDeleteSection() {
    if (deleteSectionConfirm && onDeleteSection) {
      onDeleteSection(deleteSectionConfirm.id)
    }
    deleteSectionConfirm = null
  }

  // Local mutable copy of bullet arrays for svelte-dnd-action.
  // DnD needs a stable, mutable array — we can't use the reactive IR directly.
  let dndBullets = $state<Record<string, (ExperienceBullet & { id: string })[]>>({})

  // Sync from IR whenever it changes
  $effect(() => {
    const newBullets: Record<string, (ExperienceBullet & { id: string })[]> = {}
    for (const section of ir.sections) {
      if (section.type === 'experience') {
        for (const item of section.items) {
          if (item.kind === 'experience_group') {
            for (const sub of (item as ExperienceGroup).subheadings) {
              newBullets[sub.id] = bulletsWithId(sub.bullets)
            }
          }
        }
      } else if (section.type === 'projects') {
        for (const item of section.items) {
          if (item.kind === 'project') {
            const proj = item as ProjectItem
            const key = proj.source_id ?? proj.entry_id ?? `proj-${proj.name}`
            newBullets[key] = bulletsWithId(proj.bullets)
          }
        }
      } else if (section.type === 'presentations') {
        for (const item of section.items) {
          if (item.kind === 'presentation') {
            const pres = item as PresentationItem
            const key = pres.source_id ?? pres.entry_id ?? `pres-${pres.title}`
            newBullets[key] = bulletsWithId(pres.bullets)
          }
        }
      }
    }
    dndBullets = newBullets
  })

  function handleDndConsider(_sectionId: string, subId: string, e: CustomEvent) {
    dndBullets[subId] = e.detail.items
  }

  async function handleDndFinalize(_sectionId: string, subId: string, e: CustomEvent) {
    const items = e.detail.items as (ExperienceBullet & { id: string })[]
    dndBullets[subId] = items
    // Persist new positions in parallel
    const updates = items
      .filter(item => item.entry_id)
      .map((item, i) => forge.resumes.updateEntry(resumeId, item.entry_id!, { position: i }))
    await Promise.all(updates)
    // Defer onUpdate so svelte-dnd-action finishes DOM cleanup first
    setTimeout(() => onUpdate(), 50)
  }

  let tooltipHovered = $state(false)
  let dismissTimer: ReturnType<typeof setTimeout> | null = null

  function showTooltip(bullet: ExperienceBullet, e: MouseEvent) {
    if (!bullet.source_chain) return
    if (dismissTimer) { clearTimeout(dismissTimer); dismissTimer = null }
    tooltipEntry = bullet
    tooltipPosition = { x: e.clientX + 10, y: e.clientY + 10 }
  }

  // Schedule tooltip dismissal with a grace period so the user can
  // move their cursor onto the tooltip to click links.
  function scheduleHideTooltip() {
    dismissTimer = setTimeout(() => {
      if (!tooltipHovered) {
        tooltipEntry = null
      }
    }, 150)  // 150ms grace period to reach the tooltip
  }

  // Add unique `id` to bullets for svelte-dnd-action (requires `id` property)
  function bulletsWithId(bullets: ExperienceBullet[]): (ExperienceBullet & { id: string })[] {
    return bullets.map((b, i) => ({ ...b, id: b.entry_id ?? `bullet-${i}` }))
  }

  // Sorted sections — filter out the synthetic summary section (rendered
  // by ResumeSummaryCard above, not as a regular section).
  let sortedSections = $derived(
    [...ir.sections]
      .filter(s => s.type !== 'summary')
      .sort((a, b) => a.display_order - b.display_order)
  )
</script>

<div class="dnd-view">
  <!-- Header -->
  <HeaderEditor header={ir.header} {resumeId} onSave={onUpdate} />

  <!-- Summary Card -->
  {#if onUpdateSummary}
    <ResumeSummaryCard summary={ir.summary} {onUpdateSummary} />
  {/if}

  <!-- Sections -->
  {#each sortedSections as section, i (section.id)}
    <div class="dnd-section">
      <div class="section-header">
        {#if editingSectionId === section.id}
          <input
            class="section-title-input"
            bind:value={editSectionTitle}
            onkeydown={(e) => { if (e.key === 'Enter') saveRename(section.id) }}
            onblur={() => saveRename(section.id)}
          />
        {:else}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <h3
            class="dnd-section-title"
            ondblclick={() => startRenameSection(section.id, section.title)}
          >
            {section.title}
          </h3>
        {/if}
        <span class="entry-type-badge">{section.type}</span>
        <div class="section-controls">
          {#if i > 0}
            <button class="btn btn-xs btn-ghost" onclick={() => onMoveSection?.(section.id, 'up')} title="Move up">
              &#9650;
            </button>
          {/if}
          {#if i < sortedSections.length - 1}
            <button class="btn btn-xs btn-ghost" onclick={() => onMoveSection?.(section.id, 'down')} title="Move down">
              &#9660;
            </button>
          {/if}
          <button
            class="btn btn-xs btn-ghost btn-section-delete"
            onclick={() => confirmDeleteSection(section.id, section.title)}
            title="Delete section"
          >
            &#10005;
          </button>
        </div>
      </div>

      {#if section.type === 'experience'}
        {#each section.items as item}
          {#if item.kind === 'experience_group'}
            {@const group = item as ExperienceGroup}
            <div class="experience-group">
              <h4 class="org-name">{group.organization}</h4>
              {#each group.subheadings as sub (sub.id)}
                <div class="subheading">
                  <div class="subheading-header">
                    <span class="role-title">{sub.title}</span>
                    <span class="date-range">{sub.date_range}</span>
                  </div>
                  <div
                    class="bullet-list"
                    use:dndzone={{ items: dndBullets[sub.id] ?? [], flipDurationMs: 200 }}
                    onconsider={(e) => handleDndConsider(section.id, sub.id, e)}
                    onfinalize={(e) => handleDndFinalize(section.id, sub.id, e)}
                  >
                    {#each dndBullets[sub.id] ?? [] as bullet (bullet.id)}
                      <div
                        class="bullet-item"
                        class:cloned={bullet.is_cloned}
                        role="listitem"
                        onmouseenter={(e) => showTooltip(bullet, e)}
                        onmouseleave={scheduleHideTooltip}
                      >
                        {#if editingEntryId === bullet.entry_id}
                          <textarea
                            class="bullet-edit-textarea"
                            bind:value={editContent}
                            rows={3}
                          ></textarea>
                          <div class="bullet-edit-actions">
                            <button class="btn btn-sm btn-primary"
                                    onclick={() => saveEdit(bullet.entry_id!)}
                                    disabled={editSaving}>
                              {editSaving ? 'Saving...' : 'Save'}
                            </button>
                            <button class="btn btn-sm btn-ghost"
                                    onclick={() => editingEntryId = null}>
                              Cancel
                            </button>
                          </div>
                        {:else}
                          <span class="drag-handle">&#x2630;</span>
                          <span class="bullet-content">{bullet.content}</span>
                          <div class="bullet-actions">
                            {#if bullet.is_cloned}
                              <span class="clone-badge">Edited</span>
                              <button class="btn btn-xs btn-ghost"
                                      onclick={() => resetClone(bullet.entry_id!)}
                                      title="Reset to reference">
                                Reset
                              </button>
                            {/if}
                            <button class="btn btn-xs btn-ghost"
                                    onclick={() => startEdit(bullet)}>
                              Edit
                            </button>
                            {#if onRemoveEntry && bullet.entry_id}
                              <button class="btn btn-xs btn-ghost entry-remove-btn"
                                      onclick={() => onRemoveEntry?.(bullet.entry_id!)}
                                      title="Remove this bullet"
                                      aria-label="Remove bullet">
                                &times;
                              </button>
                            {/if}
                          </div>
                        {/if}
                      </div>
                    {/each}
                  </div>
                  <!-- Per-role add button -->
                  {#if onAddEntry && sub.source_id}
                    <button
                      class="btn btn-xs btn-add-role"
                      onclick={() => onAddEntry(section.id, section.type, sub.source_id ?? undefined, sub.title)}
                      title="Add a bullet from {sub.title}"
                    >
                      + Add from this role
                    </button>
                  {/if}
                </div>
              {/each}
            </div>
          {/if}
        {/each}

      {:else if section.type === 'skills'}
        {#each section.items as item}
          {#if item.kind === 'skill_group'}
            {@const skillGroup = item as SkillGroup}
            <div class="skill-categories">
              {#each skillGroup.categories as cat}
                <div class="skill-category">
                  <strong>{cat.label}:</strong> {cat.skills.join(', ')}
                </div>
              {/each}
            </div>
          {/if}
        {/each}

      {:else if section.type === 'education'}
        {#each section.items as item}
          {#if item.kind === 'education'}
            {@const edu = item as EducationItem}
            <div class="education-item">
              <div class="edu-header">
                <span class="edu-institution">{edu.institution}</span>
                <span class="edu-meta">
                  <span class="edu-date">{edu.date}</span>
                  {#if onRemoveEntry && edu.entry_id}
                    <button
                      type="button"
                      class="btn btn-xs btn-ghost entry-remove-btn"
                      onclick={() => onRemoveEntry?.(edu.entry_id!)}
                      aria-label="Remove {edu.degree}"
                      title="Remove this entry"
                    >&times;</button>
                  {/if}
                </span>
              </div>
              <span class="edu-degree">{formatDegreeLine(edu)}</span>
            </div>
          {/if}
        {/each}

      {:else if section.type === 'certifications'}
        {#each section.items as item}
          {#if item.kind === 'certification_group'}
            {@const certGroup = item as CertificationGroup}
            {#each certGroup.categories as cat}
              <div class="cert-category">
                <strong>{cat.label}:</strong>
                <span class="cert-list">
                  {#each cat.certs as cert, i}
                    <span class="cert-item">
                      {cert.name}{#if cert.in_progress}{' (in progress)'}{/if}{#if onRemoveCertification && cert.entry_id}<button
                          type="button"
                          class="btn btn-xs btn-ghost entry-remove-btn"
                          onclick={() => onRemoveCertification?.(cert.entry_id!)}
                          aria-label="Remove {cert.name}"
                          title="Remove this entry"
                        >&times;</button>{/if}{#if i < cat.certs.length - 1}, {/if}
                    </span>
                  {/each}
                </span>
              </div>
            {/each}
          {/if}
        {/each}

      {:else if section.type === 'projects'}
        {#each section.items as item}
          {#if item.kind === 'project'}
            {@const proj = item as ProjectItem}
            {@const projKey = proj.source_id ?? proj.entry_id ?? `proj-${proj.name}`}
            <div class="project-item">
              <div class="project-header">
                <span class="project-name">{proj.name}</span>
                <span class="project-meta">
                  {#if proj.date}<span class="project-date">{proj.date}</span>{/if}
                  {#if onRemoveEntry && proj.entry_id}
                    <button
                      type="button"
                      class="btn btn-xs btn-ghost entry-remove-btn"
                      onclick={() => onRemoveEntry?.(proj.entry_id!)}
                      aria-label="Remove {proj.name}"
                      title="Remove this project"
                    >&times;</button>
                  {/if}
                </span>
              </div>
              {#if (dndBullets[projKey]?.length ?? 0) === 0 && proj.description}
                <p class="project-description">{proj.description}</p>
              {/if}
              <div
                class="bullet-list"
                use:dndzone={{ items: dndBullets[projKey] ?? [], flipDurationMs: 200 }}
                onconsider={(e) => handleDndConsider(section.id, projKey, e)}
                onfinalize={(e) => handleDndFinalize(section.id, projKey, e)}
              >
                {#each dndBullets[projKey] ?? [] as bullet (bullet.id)}
                  <div
                    class="bullet-item"
                    class:cloned={bullet.is_cloned}
                    role="listitem"
                    onmouseenter={(e) => showTooltip(bullet, e)}
                    onmouseleave={scheduleHideTooltip}
                  >
                    {#if editingEntryId === bullet.entry_id}
                      <textarea
                        class="bullet-edit-textarea"
                        bind:value={editContent}
                        rows={3}
                      ></textarea>
                      <div class="bullet-edit-actions">
                        <button class="btn btn-sm btn-primary"
                                onclick={() => saveEdit(bullet.entry_id!)}
                                disabled={editSaving}>
                          {editSaving ? 'Saving...' : 'Save'}
                        </button>
                        <button class="btn btn-sm btn-ghost"
                                onclick={() => editingEntryId = null}>
                          Cancel
                        </button>
                      </div>
                    {:else}
                      <span class="drag-handle">&#x2630;</span>
                      <span class="bullet-content">{bullet.content}</span>
                      <div class="bullet-actions">
                        {#if bullet.is_cloned}
                          <span class="clone-badge">Edited</span>
                          <button class="btn btn-xs btn-ghost"
                                  onclick={() => resetClone(bullet.entry_id!)}
                                  title="Reset to reference">
                            Reset
                          </button>
                        {/if}
                        <button class="btn btn-xs btn-ghost"
                                onclick={() => startEdit(bullet)}>
                          Edit
                        </button>
                        {#if onRemoveEntry && bullet.entry_id}
                          <button class="btn btn-xs btn-ghost entry-remove-btn"
                                  onclick={() => onRemoveEntry?.(bullet.entry_id!)}
                                  title="Remove this bullet"
                                  aria-label="Remove bullet">
                            &times;
                          </button>
                        {/if}
                      </div>
                    {/if}
                  </div>
                {/each}
              </div>
              {#if onAddEntry && proj.source_id}
                <button
                  class="btn btn-xs btn-add-role"
                  onclick={() => onAddEntry(section.id, section.type, proj.source_id ?? undefined, proj.name)}
                  title="Add a bullet from {proj.name}"
                >
                  + Add from this project
                </button>
              {/if}
            </div>
          {/if}
        {/each}

      {:else if section.type === 'summary'}
        {#each section.items as item}
          {#if item.kind === 'summary'}
            {@const summary = item as SummaryItem}
            <p class="summary-text">{summary.content}</p>
          {/if}
        {/each}

      {:else if section.type === 'presentations'}
        {#each section.items as item}
          {#if item.kind === 'presentation'}
            {@const pres = item as PresentationItem}
            {@const presKey = pres.source_id ?? pres.entry_id ?? `pres-${pres.title}`}
            <div class="project-item">
              <div class="project-header">
                <span class="project-name">{pres.title}</span>
                <span class="project-meta">
                  {#if pres.date}<span class="project-date">{pres.date}</span>{/if}
                  {#if onRemoveEntry && pres.entry_id}
                    <button
                      type="button"
                      class="btn btn-xs btn-ghost entry-remove-btn"
                      onclick={() => onRemoveEntry?.(pres.entry_id!)}
                      aria-label="Remove {pres.title}"
                      title="Remove this presentation"
                    >&times;</button>
                  {/if}
                </span>
              </div>
              {#if pres.venue}
                <span class="edu-degree">{pres.venue}{#if pres.presentation_type} · {pres.presentation_type.replace(/_/g, ' ')}{/if}</span>
              {/if}
              {#if (dndBullets[presKey]?.length ?? 0) === 0 && pres.description}
                <p class="project-description">{pres.description}</p>
              {/if}
              <div
                class="bullet-list"
                use:dndzone={{ items: dndBullets[presKey] ?? [], flipDurationMs: 200 }}
                onconsider={(e) => handleDndConsider(section.id, presKey, e)}
                onfinalize={(e) => handleDndFinalize(section.id, presKey, e)}
              >
                {#each dndBullets[presKey] ?? [] as bullet (bullet.id)}
                  <div
                    class="bullet-item"
                    class:cloned={bullet.is_cloned}
                    role="listitem"
                    onmouseenter={(e) => showTooltip(bullet, e)}
                    onmouseleave={scheduleHideTooltip}
                  >
                    {#if editingEntryId === bullet.entry_id}
                      <textarea
                        class="bullet-edit-textarea"
                        bind:value={editContent}
                        rows={3}
                      ></textarea>
                      <div class="bullet-edit-actions">
                        <button class="btn btn-sm btn-primary"
                                onclick={() => saveEdit(bullet.entry_id!)}
                                disabled={editSaving}>
                          {editSaving ? 'Saving...' : 'Save'}
                        </button>
                        <button class="btn btn-sm btn-ghost"
                                onclick={() => editingEntryId = null}>
                          Cancel
                        </button>
                      </div>
                    {:else}
                      <span class="drag-handle">&#x2630;</span>
                      <span class="bullet-content">{bullet.content}</span>
                      <div class="bullet-actions">
                        {#if bullet.is_cloned}
                          <span class="clone-badge">Edited</span>
                          <button class="btn btn-xs btn-ghost"
                                  onclick={() => resetClone(bullet.entry_id!)}
                                  title="Reset to reference">
                            Reset
                          </button>
                        {/if}
                        <button class="btn btn-xs btn-ghost"
                                onclick={() => startEdit(bullet)}>
                          Edit
                        </button>
                        {#if onRemoveEntry && bullet.entry_id}
                          <button class="btn btn-xs btn-ghost entry-remove-btn"
                                  onclick={() => onRemoveEntry?.(bullet.entry_id!)}
                                  title="Remove this bullet"
                                  aria-label="Remove bullet">
                            &times;
                          </button>
                        {/if}
                      </div>
                    {/if}
                  </div>
                {/each}
              </div>
              {#if onAddEntry && pres.source_id}
                <button
                  class="btn btn-xs btn-add-role"
                  onclick={() => onAddEntry(section.id, section.type, pres.source_id ?? undefined, pres.title)}
                  title="Add a bullet from {pres.title}"
                >
                  + Add from this presentation
                </button>
              {/if}
            </div>
          {/if}
        {/each}

      {:else}
        <!-- Generic fallback for clearance, awards, custom -->
        {#each section.items as item}
          <div class="generic-item">
            {#if 'content' in item}
              <div class="generic-item-row">
                <p>{(item as ClearanceItem).content}</p>
                {#if onRemoveEntry && (item as ClearanceItem).entry_id}
                  <button
                    type="button"
                    class="btn btn-xs btn-ghost entry-remove-btn"
                    onclick={() => onRemoveEntry?.((item as ClearanceItem).entry_id!)}
                    aria-label="Remove entry"
                    title="Remove this entry"
                  >&times;</button>
                {/if}
              </div>
            {/if}
          </div>
        {/each}
      {/if}

      {#if onAddEntry}
        <button class="btn btn-sm btn-add-entry" onclick={() => onAddEntry(section.id, section.type)}>
          + Add Entry
        </button>
      {/if}
    </div>
  {/each}

  <!-- Add Section button -->
  {#if onAddSection}
    <AddSectionDropdown
      existingTypes={ir.sections.map(s => s.type)}
      onSelect={(entryType, title) => onAddSection(entryType, title)}
    />
  {/if}
</div>

<!-- Provenance Tooltip -->
{#if tooltipEntry?.source_chain}
  <div
    class="provenance-tooltip"
    style="left: {tooltipPosition.x}px; top: {tooltipPosition.y}px;"
    onmouseenter={() => { tooltipHovered = true; if (dismissTimer) clearTimeout(dismissTimer) }}
    onmouseleave={() => { tooltipHovered = false; tooltipEntry = null }}
  >
    <div class="tooltip-row">
      <strong>Source:</strong>
      <span class="tooltip-label">{tooltipEntry.source_chain.source_title}</span>
      <button
        class="tooltip-link"
        onclick={() => { const t = tooltipEntry; if (t?.source_chain) openChainView(`source-${t.source_chain.source_id}`) }}
      >&rarr;</button>
    </div>
    <div class="tooltip-row">
      <strong>Bullet:</strong>
      <span class="tooltip-label">{tooltipEntry.source_chain.bullet_preview}</span>
      <button
        class="tooltip-link"
        onclick={() => { const t = tooltipEntry; if (t?.source_chain) openChainView(`bullet-${t.source_chain.bullet_id}`) }}
      >&rarr;</button>
    </div>
    <div class="tooltip-row">
      <strong>Perspective:</strong>
      <span class="tooltip-label">{tooltipEntry.source_chain.perspective_preview}</span>
      <button
        class="tooltip-link"
        onclick={() => { const t = tooltipEntry; if (t?.source_chain) openChainView(`perspective-${t.source_chain.perspective_id}`) }}
      >&rarr;</button>
    </div>
    {#if tooltipEntry.is_cloned}
      <div class="tooltip-row tooltip-cloned">Content has been manually edited</div>
    {/if}
  </div>
{/if}

<!-- Delete Section Confirmation -->
{#if deleteSectionConfirm}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="delete-overlay" onclick={() => deleteSectionConfirm = null} role="presentation">
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="delete-modal" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true" aria-label="Confirm Delete">
      <h4>Delete Section</h4>
      <p>Delete "{deleteSectionConfirm.title}"? All entries in this section will be removed.</p>
      <div class="delete-modal-actions">
        <button class="btn btn-sm btn-ghost" onclick={() => deleteSectionConfirm = null}>Cancel</button>
        <button class="btn btn-sm btn-danger-solid" onclick={executeDeleteSection}>Delete</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .dnd-view {
    padding: 1.5rem;
    max-width: 800px;
    margin: 0 auto;
  }

  .dnd-section {
    margin-bottom: 1.5rem;
  }

  /* Section header with controls */
  .section-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.75rem;
  }

  .section-header .dnd-section-title {
    flex: 1;
    margin-bottom: 0;
    cursor: text;
    font-size: 0.85rem;
    font-weight: 700;
    color: var(--text-primary);
    border-bottom: 2px solid var(--text-primary);
    padding-bottom: 0.25rem;
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }

  .section-title-input {
    flex: 1;
    font-size: 0.85rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.03em;
    border: 1px solid var(--color-primary);
    border-radius: var(--radius-sm);
    padding: 0.15rem 0.35rem;
    font-family: inherit;
    color: var(--text-primary);
  }

  .section-title-input:focus {
    outline: none;
    box-shadow: 0 0 0 2px var(--color-primary-subtle);
  }

  .entry-type-badge {
    font-size: 0.6rem;
    padding: 0.1em 0.4em;
    background: var(--color-tag-bg);
    color: var(--color-tag-text);
    border-radius: var(--radius-sm);
    font-weight: 600;
    text-transform: uppercase;
    white-space: nowrap;
  }

  .section-controls {
    display: flex;
    gap: 0.15rem;
    flex-shrink: 0;
  }

  .btn-section-delete { color: var(--color-danger-text); }
  .btn-section-delete:hover { background: var(--color-danger-subtle); }

  .experience-group {
    margin-bottom: 1rem;
  }

  .org-name {
    font-size: 0.95rem;
    font-weight: 700;
    color: var(--text-primary);
    margin-bottom: 0.25rem;
  }

  .subheading {
    margin-bottom: 0.75rem;
    margin-left: 0.25rem;
  }

  .subheading-header {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    margin-bottom: 0.25rem;
  }

  .role-title {
    font-weight: 600;
    font-style: italic;
    font-size: 0.875rem;
    color: var(--text-secondary);
  }

  .date-range {
    font-size: 0.8rem;
    color: var(--text-muted);
    flex-shrink: 0;
  }

  .bullet-list {
    min-height: 20px;
    padding-left: 0.5rem;
  }

  .bullet-item {
    display: flex;
    align-items: flex-start;
    gap: 0.5rem;
    padding: 0.35rem 0.5rem;
    margin-bottom: 0.15rem;
    border-radius: 4px;
    font-size: 0.825rem;
    color: var(--text-secondary);
    line-height: 1.5;
    cursor: grab;
    transition: background 0.1s;
  }

  .bullet-item:hover {
    background: var(--color-surface-raised);
  }

  .bullet-item.cloned {
    border-left: 3px solid var(--color-warning);
    padding-left: 0.35rem;
  }

  .drag-handle {
    color: var(--color-border-strong);
    cursor: grab;
    user-select: none;
    flex-shrink: 0;
    font-size: 0.7rem;
    line-height: 1.5;
  }

  .bullet-content {
    flex: 1;
  }

  .bullet-actions {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    flex-shrink: 0;
    opacity: 0;
    transition: opacity 0.15s;
  }

  .bullet-item:hover .bullet-actions {
    opacity: 1;
  }

  .clone-badge {
    display: inline-block;
    padding: 0.1em 0.3em;
    background: var(--color-warning-bg);
    color: var(--color-warning-text);
    border-radius: var(--radius-sm);
    font-size: 0.6rem;
    font-weight: 600;
    text-transform: uppercase;
  }

  .bullet-edit-textarea {
    width: 100%;
    padding: 0.5rem 0.65rem;
    border: 1px solid var(--color-primary);
    border-radius: var(--radius-md);
    font-size: 0.825rem;
    color: var(--text-primary);
    font-family: inherit;
    line-height: 1.5;
    resize: vertical;
    min-height: 60px;
  }

  .bullet-edit-textarea:focus {
    outline: none;
    box-shadow: 0 0 0 2px var(--color-primary-subtle);
  }

  .bullet-edit-actions {
    display: flex;
    gap: 0.5rem;
    margin-top: 0.35rem;
  }

  /* Section-specific items */
  .skill-categories {
    padding: 0.25rem 0;
  }

  .skill-category {
    font-size: 0.825rem;
    color: var(--text-secondary);
    line-height: 1.6;
    margin-bottom: 0.15rem;
  }

  .education-item {
    margin-bottom: 0.5rem;
  }

  .edu-header {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 0.5rem;
  }

  .edu-institution {
    font-weight: 700;
    font-size: 0.875rem;
    color: var(--text-primary);
  }

  .edu-meta {
    display: inline-flex;
    align-items: baseline;
    gap: 0.35rem;
  }

  .edu-date {
    font-size: 0.8rem;
    color: var(--text-muted);
  }

  .edu-degree {
    font-size: 0.825rem;
    color: var(--text-secondary);
    font-style: italic;
  }

  /* Per-entry delete button used on education, certification, clearance,
     and project sections. Visual: small ghost button that only becomes
     prominent on hover, so it doesn't crowd the normal reading view. */
  .entry-remove-btn {
    min-width: 1.3rem;
    padding: 0 0.35rem;
    color: var(--text-faint);
    font-size: 0.95rem;
    line-height: 1;
  }

  .entry-remove-btn:hover {
    color: var(--color-danger);
  }

  .cert-list {
    display: inline;
  }

  .cert-item {
    display: inline;
  }

  .project-meta {
    display: inline-flex;
    align-items: baseline;
    gap: 0.35rem;
  }

  .generic-item-row {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 0.5rem;
  }

  .generic-item-row p {
    margin: 0;
    flex: 1;
  }

  .cert-category {
    font-size: 0.825rem;
    color: var(--text-secondary);
    line-height: 1.6;
    margin-bottom: 0.15rem;
  }

  .project-item {
    margin-bottom: 0.75rem;
  }

  .project-header {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    margin-bottom: 0.25rem;
  }

  .project-name {
    font-weight: 700;
    font-size: 0.875rem;
    color: var(--text-primary);
  }

  .project-date {
    font-size: 0.8rem;
    color: var(--text-muted);
  }

  .project-description {
    margin: 0.2rem 0 0.4rem 0;
    font-size: 0.825rem;
    color: var(--text-secondary);
    line-height: 1.5;
    white-space: pre-wrap;
  }

  .summary-text {
    font-size: 0.85rem;
    color: var(--text-secondary);
    line-height: 1.6;
    font-family: Georgia, 'Times New Roman', serif;
  }

  .generic-item {
    font-size: 0.825rem;
    color: var(--text-secondary);
    line-height: 1.6;
    margin-bottom: 0.25rem;
  }

  /* Provenance tooltip */
  .provenance-tooltip {
    position: fixed;
    z-index: var(--z-modal);
    background: var(--color-sidebar-bg);
    color: var(--color-sidebar-text-hover);
    padding: 0.5rem 0.75rem;
    border-radius: 6px;
    font-size: 0.7rem;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
    pointer-events: auto;
    max-width: 350px;
  }

  .tooltip-row {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    margin-bottom: 0.15rem;
  }

  .tooltip-label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tooltip-link {
    color: #6c63ff;
    background: none;
    border: none;
    padding: 0;
    margin-left: 0.35rem;
    flex-shrink: 0;
    cursor: pointer;
    font-size: inherit;
    font-family: inherit;
    line-height: inherit;
  }

  .tooltip-link:hover {
    text-decoration: underline;
  }

  .tooltip-row strong {
    color: #a5b4fc;
  }

  .tooltip-cloned {
    color: #fbbf24;
    font-style: italic;
    margin-top: 0.25rem;
  }

  /* Delete confirmation overlay */
  .delete-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .delete-modal {
    background: white;
    border-radius: 10px;
    padding: 1.25rem;
    max-width: 400px;
    width: 90%;
    box-shadow: 0 8px 30px rgba(0, 0, 0, 0.15);
  }

  .delete-modal h4 {
    margin: 0 0 0.5rem 0;
    font-size: 1rem;
    color: var(--text-primary);
  }

  .delete-modal p {
    font-size: 0.85rem;
    color: var(--text-muted);
    margin-bottom: 1rem;
  }

  .delete-modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
  }

  .btn-add-role {
    margin-top: 0.25rem;
    padding: 0.2rem 0.5rem;
    font-size: 0.7rem;
    color: #9ca3af;
    background: transparent;
    border: 1px dashed #e5e7eb;
    border-radius: 4px;
    cursor: pointer;
    font-family: inherit;
  }
  .btn-add-role:hover {
    color: #6c63ff;
    border-color: #6c63ff;
  }
  .btn-add-entry {
    margin-top: 0.5rem;
    border: 1px dashed #d1d5db;
    background: transparent;
    color: var(--text-muted);
    width: 100%;
    padding: 0.4rem;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.8rem;
    font-family: inherit;
  }
  .btn-add-entry:hover { background: #f9fafb; color: #374151; border-color: #9ca3af; }
</style>
