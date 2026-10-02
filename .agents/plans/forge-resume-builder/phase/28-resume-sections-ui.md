# Phase 28: API Routes + SDK + UI (Section Management)

**Status:** Planning
**Date:** 2026-03-30
**Spec:** [2026-03-30-resume-sections-as-entities.md](../refs/specs/2026-03-30-resume-sections-as-entities.md)
**Depends on:** Phase 27 (schema migration 004 + core layer)
**Blocks:** Nothing

## Goal

Add API endpoints for section and skills CRUD, update the SDK with section/skills resource methods, and implement the DragNDrop UI for managing resume sections: section add/rename/reorder/delete, skills picker, source picker, and freeform entry creation.

## Scope

| Spec section | Covered here? |
|-------------|---------------|
| 2.1. Section CRUD routes | Yes |
| 2.2. Resume Skills routes | Yes |
| 2.3. Updated entry endpoints | Yes |
| 2.4. Freeform entry creation | Yes |
| 2.5. Updated IR endpoint | Yes (already handled by Phase 27 compiler, just verify) |
| 4.1. Add Section button | Yes |
| 4.2. Section header controls | Yes |
| 4.3. Type-specific pickers | Yes |
| 4.4. onAddEntry callback fix | Yes |
| 4.5. Skills picker | Yes |
| 4.6. Source picker | Yes |

## Files to Modify

- `packages/core/src/routes/resumes.ts`
- `packages/sdk/src/resources/resumes.ts`
- `packages/sdk/src/types.ts` (add section/skill SDK input types if needed)
- `packages/webui/src/lib/components/resume/DragNDropView.svelte`
- `packages/webui/src/routes/resumes/+page.svelte`

## Files to Create

- `packages/webui/src/lib/components/resume/SkillsPicker.svelte`
- `packages/webui/src/lib/components/resume/SourcePicker.svelte`
- `packages/webui/src/lib/components/resume/AddSectionDropdown.svelte`

## Test Files to Modify/Create

- `packages/core/src/routes/__tests__/resumes.test.ts`
- `packages/core/src/routes/__tests__/contracts.test.ts`

---

## Tasks

### T28.1: API Routes for Sections

**File:** `packages/core/src/routes/resumes.ts`

Add section CRUD endpoints and update existing entry endpoints.

#### Section CRUD

```typescript
// POST /resumes/:id/sections — Create section
app.post('/resumes/:id/sections', async (c) => {
  const body = await c.req.json<CreateResumeSection>()
  const result = services.resumes.createSection(c.req.param('id'), body)
  if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
  return c.json({ data: result.data }, 201)
})

// GET /resumes/:id/sections — List sections
app.get('/resumes/:id/sections', (c) => {
  const result = services.resumes.listSections(c.req.param('id'))
  if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
  return c.json({ data: result.data })
})

// PATCH /resumes/:id/sections/:sectionId — Update section
app.patch('/resumes/:id/sections/:sectionId', async (c) => {
  const body = await c.req.json<UpdateResumeSection>()
  const result = services.resumes.updateSection(c.req.param('id'), c.req.param('sectionId'), body)
  if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
  return c.json({ data: result.data })
})

// DELETE /resumes/:id/sections/:sectionId — Delete section
app.delete('/resumes/:id/sections/:sectionId', (c) => {
  const result = services.resumes.deleteSection(c.req.param('id'), c.req.param('sectionId'))
  if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
  return c.body(null, 204)
})
```

#### Resume Skills CRUD

```typescript
// POST /resumes/:id/sections/:sectionId/skills — Add skill to section
app.post('/resumes/:id/sections/:sectionId/skills', async (c) => {
  const body = await c.req.json<{ skill_id: string }>()
  const result = services.resumes.addSkill(c.req.param('id'), c.req.param('sectionId'), body.skill_id)
  if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
  return c.json({ data: result.data }, 201)
})

// GET /resumes/:id/sections/:sectionId/skills — List skills for section
app.get('/resumes/:id/sections/:sectionId/skills', (c) => {
  const section = services.resumes.getSection(c.req.param('id'), c.req.param('sectionId'))
  if (!section) return c.json({ error: { code: 'NOT_FOUND', message: 'Section not found' } }, 404)
  const skills = services.resumes.listSkillsForSection(c.req.param('sectionId'))
  return c.json({ data: skills })
})

// DELETE /resumes/:id/sections/:sectionId/skills/:skillId — Remove skill
app.delete('/resumes/:id/sections/:sectionId/skills/:skillId', (c) => {
  const result = services.resumes.removeSkill(
    c.req.param('id'),
    c.req.param('sectionId'),
    c.req.param('skillId')
  )
  if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
  return c.body(null, 204)
})

// PATCH /resumes/:id/sections/:sectionId/skills/reorder — Reorder skills
app.patch('/resumes/:id/sections/:sectionId/skills/reorder', async (c) => {
  const body = await c.req.json<{ skill_ids: string[] }>()
  // Validate section ownership
  const section = services.resumes.getSection(c.req.param('id'), c.req.param('sectionId'))
  // For simplicity, delegate to ResumeRepository.reorderSkills
  // The service should expose a reorderSkills method
  const result = services.resumes.reorderSkills(
    c.req.param('id'),
    c.req.param('sectionId'),
    body.skill_ids
  )
  if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
  return c.json({ data: null })
})
```

#### Updated Entry Endpoints

Update the existing entry routes to accept `section_id` instead of `section`:

```typescript
// POST /resumes/:id/entries — body now has section_id
app.post('/resumes/:id/entries', async (c) => {
  const body = await c.req.json<AddResumeEntry>()
  // body = { section_id, perspective_id?, content?, position? }
  const result = services.resumes.addEntry(c.req.param('id'), body)
  if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
  return c.json({ data: result.data }, 201)
})

// PATCH /resumes/:id/entries/:entryId — body now has section_id instead of section
app.patch('/resumes/:id/entries/:entryId', async (c) => {
  const body = await c.req.json<{
    content?: string | null
    section_id?: string     // was: section?: string
    position?: number
    notes?: string | null
  }>()
  const result = services.resumes.updateEntry(c.req.param('id'), c.req.param('entryId'), body)
  if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
  return c.json({ data: result.data })
})

// PATCH /resumes/:id/entries/reorder — entries use section_id
app.patch('/resumes/:id/entries/reorder', async (c) => {
  const body = await c.req.json<{
    entries: Array<{ id: string; section_id: string; position: number }>
  }>()
  const result = services.resumes.reorderEntries(c.req.param('id'), body.entries)
  if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
  return c.json({ data: null })
})
```

#### Import updates

Add new type imports at top of file:

```typescript
import type { CreateResume, UpdateResume, AddResumeEntry, CreateResumeSection, UpdateResumeSection } from '../types'
```

---

### T28.2: SDK Updates

**File:** `packages/sdk/src/resources/resumes.ts`

Add section and skills methods to `ResumesResource`:

```typescript
// ── Section methods ───────────────────────────────────────────────────

createSection(
  resumeId: string,
  input: { title: string; entry_type: string; position?: number },
): Promise<Result<ResumeSectionEntity>> {
  return this.request<ResumeSectionEntity>(
    'POST',
    `/api/resumes/${resumeId}/sections`,
    input,
  )
}

listSections(resumeId: string): Promise<Result<ResumeSectionEntity[]>> {
  return this.request<ResumeSectionEntity[]>(
    'GET',
    `/api/resumes/${resumeId}/sections`,
  )
}

updateSection(
  resumeId: string,
  sectionId: string,
  input: { title?: string; position?: number },
): Promise<Result<ResumeSectionEntity>> {
  return this.request<ResumeSectionEntity>(
    'PATCH',
    `/api/resumes/${resumeId}/sections/${sectionId}`,
    input,
  )
}

deleteSection(resumeId: string, sectionId: string): Promise<Result<void>> {
  return this.request<void>(
    'DELETE',
    `/api/resumes/${resumeId}/sections/${sectionId}`,
  )
}

// ── Resume Skills methods ─────────────────────────────────────────────

addSkill(
  resumeId: string,
  sectionId: string,
  skillId: string,
): Promise<Result<ResumeSkill>> {
  return this.request<ResumeSkill>(
    'POST',
    `/api/resumes/${resumeId}/sections/${sectionId}/skills`,
    { skill_id: skillId },
  )
}

removeSkill(
  resumeId: string,
  sectionId: string,
  skillId: string,
): Promise<Result<void>> {
  return this.request<void>(
    'DELETE',
    `/api/resumes/${resumeId}/sections/${sectionId}/skills/${skillId}`,
  )
}

listSkills(
  resumeId: string,
  sectionId: string,
): Promise<Result<ResumeSkill[]>> {
  return this.request<ResumeSkill[]>(
    'GET',
    `/api/resumes/${resumeId}/sections/${sectionId}/skills`,
  )
}

reorderSkills(
  resumeId: string,
  sectionId: string,
  skillIds: string[],
): Promise<Result<void>> {
  return this.request<void>(
    'PATCH',
    `/api/resumes/${resumeId}/sections/${sectionId}/skills/reorder`,
    { skill_ids: skillIds },
  )
}
```

Update the `addEntry` method to accept the new `AddResumeEntry` type (with `section_id`):

```typescript
addEntry(
  resumeId: string,
  input: AddResumeEntry,  // { section_id, perspective_id?, content?, position? }
): Promise<Result<ResumeEntry>> {
  return this.request<ResumeEntry>(
    'POST',
    `/api/resumes/${resumeId}/entries`,
    input,
  )
}
```

**Import updates:**

```typescript
import type {
  AddResumeEntry,
  CreateResume,
  ForgeError,
  GapAnalysis,
  PaginatedResult,
  PaginationParams,
  RequestFn,
  RequestListFn,
  ResumeDocument,
  ResumeEntry,
  ResumeSectionEntity,   // new
  ResumeSkill,           // new
  ResumeWithEntries,
  Result,
  Resume,
  UpdateResume,
  UpdateResumeEntry,
} from '../types'
```

---

### T28.3: DragNDrop — Section Management

**File:** `packages/webui/src/lib/components/resume/DragNDropView.svelte`

#### Update prop types

```typescript
// Before:
let {
  ir,
  resumeId,
  onUpdate,
  onAddEntry,
}: {
  ir: ResumeDocument
  resumeId: string
  onUpdate: () => Promise<void>
  onAddEntry?: (section: string, sourceId?: string, sourceLabel?: string) => void
} = $props()

// After:
let {
  ir,
  resumeId,
  onUpdate,
  onAddEntry,
  onAddSection,
  onDeleteSection,
  onRenameSection,
  onMoveSection,
}: {
  ir: ResumeDocument
  resumeId: string
  onUpdate: () => Promise<void>
  onAddEntry?: (sectionId: string, entryType: string, sourceId?: string, sourceLabel?: string) => void
  onAddSection?: (entryType: string, title: string) => void
  onDeleteSection?: (sectionId: string) => void
  onRenameSection?: (sectionId: string, newTitle: string) => void
  onMoveSection?: (sectionId: string, direction: 'up' | 'down') => void
} = $props()
```

#### Update `onAddEntry` call sites

The per-role "+ Add from this role" button:

```svelte
<!-- Before: -->
<button onclick={() => onAddEntry('experience', sub.source_id ?? undefined, sub.title)}>

<!-- After: -->
<button onclick={() => onAddEntry(section.id, section.type, sub.source_id ?? undefined, sub.title)}>
```

The section-level "+ Add Entry" button:

```svelte
<!-- Before: -->
<button onclick={() => onAddEntry(section.type)}>

<!-- After: -->
<button onclick={() => onAddEntry(section.id, section.type)}>
```

#### Section header with controls

Replace the plain `<h3>` with an editable header:

```svelte
{#each [...ir.sections].sort((a, b) => a.display_order - b.display_order) as section, i (section.id)}
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
        {#if i < ir.sections.length - 1}
          <button class="btn btn-xs btn-ghost" onclick={() => onMoveSection?.(section.id, 'down')} title="Move down">
            &#9660;
          </button>
        {/if}
        <button
          class="btn btn-xs btn-ghost btn-danger"
          onclick={() => confirmDeleteSection(section.id, section.title)}
          title="Delete section"
        >
          &#10005;
        </button>
      </div>
    </div>

    <!-- ... existing section content rendering ... -->
  </div>
{/each}
```

#### Section rename state

```typescript
let editingSectionId = $state<string | null>(null)
let editSectionTitle = $state('')

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

let deleteSectionConfirm = $state<{ id: string; title: string } | null>(null)

function confirmDeleteSection(sectionId: string, title: string) {
  deleteSectionConfirm = { id: sectionId, title }
}

function executeDeleteSection() {
  if (deleteSectionConfirm && onDeleteSection) {
    onDeleteSection(deleteSectionConfirm.id)
  }
  deleteSectionConfirm = null
}
```

#### "Add Section" dropdown at bottom

```svelte
<!-- At the end of the sections loop -->
{#if onAddSection}
  <AddSectionDropdown
    existingTypes={ir.sections.map(s => s.type)}
    onSelect={(entryType, title) => onAddSection(entryType, title)}
  />
{/if}
```

#### Delete confirmation dialog

```svelte
{#if deleteSectionConfirm}
  <div class="modal-overlay" onclick={() => deleteSectionConfirm = null}>
    <div class="modal-content" onclick|stopPropagation>
      <h4>Delete Section</h4>
      <p>Delete "{deleteSectionConfirm.title}"? All entries in this section will be removed.</p>
      <div class="modal-actions">
        <button class="btn btn-sm btn-ghost" onclick={() => deleteSectionConfirm = null}>Cancel</button>
        <button class="btn btn-sm btn-danger" onclick={executeDeleteSection}>Delete</button>
      </div>
    </div>
  </div>
{/if}
```

#### Styles for new elements

```css
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
}

.section-title-input {
  flex: 1;
  font-size: 0.85rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.03em;
  border: 1px solid #6c63ff;
  border-radius: 4px;
  padding: 0.15rem 0.35rem;
  font-family: inherit;
}

.entry-type-badge {
  font-size: 0.6rem;
  padding: 0.1em 0.4em;
  background: #e0e7ff;
  color: #3730a3;
  border-radius: 3px;
  font-weight: 600;
  text-transform: uppercase;
  white-space: nowrap;
}

.section-controls {
  display: flex;
  gap: 0.15rem;
  flex-shrink: 0;
}

.btn-danger { color: #dc2626; }
.btn-danger:hover { background: #fef2f2; }
```

---

### T28.4: AddSectionDropdown Component

**File:** `packages/webui/src/lib/components/resume/AddSectionDropdown.svelte` (NEW)

```svelte
<script lang="ts">
  const ENTRY_TYPES = [
    { value: 'experience', label: 'Experience', defaultTitle: 'Experience' },
    { value: 'skills', label: 'Skills', defaultTitle: 'Technical Skills' },
    { value: 'education', label: 'Education', defaultTitle: 'Education' },
    { value: 'projects', label: 'Projects', defaultTitle: 'Selected Projects' },
    { value: 'clearance', label: 'Clearance', defaultTitle: 'Security Clearance' },
    { value: 'presentations', label: 'Presentations', defaultTitle: 'Presentations' },
    { value: 'certifications', label: 'Certifications', defaultTitle: 'Certifications' },
    { value: 'awards', label: 'Awards', defaultTitle: 'Awards' },
    { value: 'freeform', label: 'Custom (Freeform)', defaultTitle: 'Custom Section' },
  ]

  let {
    existingTypes,
    onSelect,
  }: {
    existingTypes: string[]
    onSelect: (entryType: string, title: string) => void
  } = $props()

  let showDropdown = $state(false)

  // Show types not yet present first, then all types (user can have multiples)
  let sortedTypes = $derived.by(() => {
    const existing = new Set(existingTypes)
    const notPresent = ENTRY_TYPES.filter(t => !existing.has(t.value))
    const present = ENTRY_TYPES.filter(t => existing.has(t.value))
    return [...notPresent, ...present]
  })

  function handleSelect(entryType: string, defaultTitle: string) {
    onSelect(entryType, defaultTitle)
    showDropdown = false
  }
</script>

<div class="add-section-container">
  <button
    class="btn btn-add-section"
    onclick={() => showDropdown = !showDropdown}
  >
    + Add Section
  </button>
  {#if showDropdown}
    <div class="add-section-dropdown">
      {#each sortedTypes as entryType}
        <button
          class="dropdown-item"
          class:existing={existingTypes.includes(entryType.value)}
          onclick={() => handleSelect(entryType.value, entryType.defaultTitle)}
        >
          {entryType.label}
          {#if existingTypes.includes(entryType.value)}
            <span class="existing-badge">exists</span>
          {/if}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .add-section-container {
    position: relative;
    margin-top: 1rem;
  }

  .btn-add-section {
    width: 100%;
    padding: 0.6rem;
    border: 2px dashed #d1d5db;
    background: transparent;
    color: #6b7280;
    border-radius: 8px;
    cursor: pointer;
    font-size: 0.85rem;
    font-weight: 500;
    font-family: inherit;
    transition: all 0.15s;
  }

  .btn-add-section:hover {
    border-color: #6c63ff;
    color: #6c63ff;
    background: #f5f3ff;
  }

  .add-section-dropdown {
    position: absolute;
    bottom: 100%;
    left: 0;
    right: 0;
    background: white;
    border: 1px solid #e5e7eb;
    border-radius: 8px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
    margin-bottom: 0.25rem;
    z-index: 100;
    max-height: 300px;
    overflow-y: auto;
  }

  .dropdown-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 0.5rem 0.75rem;
    border: none;
    background: transparent;
    color: #374151;
    font-size: 0.825rem;
    cursor: pointer;
    font-family: inherit;
    text-align: left;
  }

  .dropdown-item:hover {
    background: #f5f3ff;
    color: #6c63ff;
  }

  .dropdown-item.existing {
    color: #9ca3af;
  }

  .existing-badge {
    font-size: 0.6rem;
    color: #9ca3af;
    font-style: italic;
  }
</style>
```

---

### T28.5: Skills Picker

**File:** `packages/webui/src/lib/components/resume/SkillsPicker.svelte` (NEW)

A modal component showing all skills grouped by category with checkboxes.

```svelte
<script lang="ts">
  import { forge, friendlyError } from '$lib/sdk'
  import { addToast } from '$lib/stores/toast.svelte'
  import type { Skill } from '@forge/sdk'

  let {
    resumeId,
    sectionId,
    onClose,
    onUpdate,
  }: {
    resumeId: string
    sectionId: string
    onClose: () => void
    onUpdate: () => Promise<void>
  } = $props()

  let allSkills = $state<Skill[]>([])
  let selectedSkillIds = $state<Set<string>>(new Set())
  let loading = $state(true)

  // Load all skills and currently-selected skills
  $effect(() => {
    loadData()
  })

  async function loadData() {
    loading = true
    try {
      // Load all skills
      const skillsResult = await forge.skills.list({ limit: 500 })
      if (skillsResult.ok) {
        allSkills = skillsResult.data
      }

      // Load currently-selected skill IDs from the API (ID-based, not name-matching)
      await loadSelectedSkills()
    } catch (e) {
      addToast({ message: 'Failed to load skills', type: 'error' })
    } finally {
      loading = false
    }
  }

  async function loadSelectedSkills() {
    const result = await forge.resumes.listSkills(resumeId, sectionId)
    if (result.ok) {
      selectedSkillIds = new Set(result.data.map(rs => rs.skill_id))
    }
  }

  // Group skills by category
  let groupedSkills = $derived.by(() => {
    const groups = new Map<string, Skill[]>()
    for (const skill of allSkills) {
      const cat = skill.category ?? 'Other'
      if (!groups.has(cat)) groups.set(cat, [])
      groups.get(cat)!.push(skill)
    }
    return [...groups.entries()].sort(([a], [b]) => a.localeCompare(b))
  })

  async function toggleSkill(skillId: string) {
    if (selectedSkillIds.has(skillId)) {
      // Remove
      const result = await forge.resumes.removeSkill(resumeId, sectionId, skillId)
      if (result.ok) {
        await loadSelectedSkills()  // refresh from server rather than optimistic local state
        await onUpdate()
      } else {
        addToast({ message: friendlyError(result.error), type: 'error' })
      }
    } else {
      // Add
      const result = await forge.resumes.addSkill(resumeId, sectionId, skillId)
      if (result.ok) {
        await loadSelectedSkills()  // refresh from server rather than optimistic local state
        await onUpdate()
      } else {
        addToast({ message: friendlyError(result.error), type: 'error' })
      }
    }
  }
</script>

<div class="modal-overlay" onclick={onClose}>
  <div class="skills-picker-modal" onclick|stopPropagation>
    <div class="picker-header">
      <h3>Select Skills</h3>
      <button class="btn btn-sm btn-ghost" onclick={onClose}>Close</button>
    </div>

    {#if loading}
      <p class="loading-text">Loading skills...</p>
    {:else if groupedSkills.length === 0}
      <p class="empty-text">No skills found. Add skills first.</p>
    {:else}
      <div class="skills-grid">
        {#each groupedSkills as [category, skills]}
          <div class="skill-category-group">
            <h4 class="category-label">{category}</h4>
            <div class="skill-checkboxes">
              {#each skills as skill}
                <label class="skill-checkbox" class:checked={selectedSkillIds.has(skill.id)}>
                  <input
                    type="checkbox"
                    checked={selectedSkillIds.has(skill.id)}
                    onchange={() => toggleSkill(skill.id)}
                  />
                  <span>{skill.name}</span>
                </label>
              {/each}
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .skills-picker-modal {
    background: white;
    border-radius: 12px;
    padding: 1.25rem;
    max-width: 600px;
    width: 90%;
    max-height: 80vh;
    overflow-y: auto;
    box-shadow: 0 8px 30px rgba(0, 0, 0, 0.15);
  }

  .picker-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1rem;
  }

  .picker-header h3 {
    margin: 0;
    font-size: 1rem;
    color: #1a1a2e;
  }

  .skill-category-group {
    margin-bottom: 1rem;
  }

  .category-label {
    font-size: 0.8rem;
    font-weight: 600;
    color: #6b7280;
    text-transform: uppercase;
    letter-spacing: 0.03em;
    margin-bottom: 0.35rem;
    border-bottom: 1px solid #f3f4f6;
    padding-bottom: 0.15rem;
  }

  .skill-checkboxes {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
  }

  .skill-checkbox {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
    font-size: 0.8rem;
    color: #374151;
    cursor: pointer;
    transition: background 0.1s;
  }

  .skill-checkbox:hover {
    background: #f5f3ff;
  }

  .skill-checkbox.checked {
    background: #e0e7ff;
    color: #3730a3;
  }

  .skill-checkbox input[type="checkbox"] {
    accent-color: #6c63ff;
  }

  .loading-text, .empty-text {
    text-align: center;
    color: #9ca3af;
    font-size: 0.85rem;
    padding: 2rem;
  }
</style>
```

---

### T28.6: Source Picker

**File:** `packages/webui/src/lib/components/resume/SourcePicker.svelte` (NEW)

For education/projects/clearance sections. Shows sources of the matching type, allows selecting one to create an entry.

```svelte
<script lang="ts">
  import { forge, friendlyError } from '$lib/sdk'
  import { addToast } from '$lib/stores/toast.svelte'
  import type { Source } from '@forge/sdk'

  let {
    resumeId,
    sectionId,
    sourceType,    // 'education' | 'project' | 'clearance'
    onClose,
    onUpdate,
  }: {
    resumeId: string
    sectionId: string
    sourceType: string
    onClose: () => void
    onUpdate: () => Promise<void>
  } = $props()

  let sources = $state<Source[]>([])
  let loading = $state(true)
  let adding = $state<string | null>(null)

  $effect(() => {
    loadSources()
  })

  async function loadSources() {
    loading = true
    try {
      const result = await forge.sources.list({ source_type: sourceType, status: 'approved' })
      if (result.ok) {
        sources = result.data
      }
    } catch (e) {
      addToast({ message: 'Failed to load sources', type: 'error' })
    } finally {
      loading = false
    }
  }

  async function addSource(source: Source) {
    adding = source.id
    try {
      // Check if source has approved perspectives
      const perspResult = await forge.perspectives.list({
        source_id: source.id,
        status: 'approved',
        limit: 50,
      })

      if (perspResult.ok && perspResult.data.length > 0) {
        // Has perspectives — add the first one as an entry
        // In future, show a picker for which perspective to use
        const result = await forge.resumes.addEntry(resumeId, {
          section_id: sectionId,
          perspective_id: perspResult.data[0].id,
        })
        if (result.ok) {
          addToast({ message: `Added ${source.title}`, type: 'success' })
          await onUpdate()
        } else {
          addToast({ message: friendlyError(result.error), type: 'error' })
        }
      } else {
        // No perspectives — add as direct content entry (freeform-like)
        const result = await forge.resumes.addEntry(resumeId, {
          section_id: sectionId,
          content: source.description,
        })
        if (result.ok) {
          addToast({ message: `Added ${source.title} (direct)`, type: 'success' })
          await onUpdate()
        } else {
          addToast({ message: friendlyError(result.error), type: 'error' })
        }
      }
    } catch (e) {
      addToast({ message: 'Failed to add source', type: 'error' })
    } finally {
      adding = null
    }
  }
</script>

<div class="modal-overlay" onclick={onClose}>
  <div class="source-picker-modal" onclick|stopPropagation>
    <div class="picker-header">
      <h3>Select {sourceType === 'education' ? 'Education' : sourceType === 'project' ? 'Project' : 'Clearance'}</h3>
      <button class="btn btn-sm btn-ghost" onclick={onClose}>Close</button>
    </div>

    {#if loading}
      <p class="loading-text">Loading sources...</p>
    {:else if sources.length === 0}
      <p class="empty-text">No {sourceType} sources found.</p>
    {:else}
      <div class="source-list">
        {#each sources as source (source.id)}
          <button
            class="source-item"
            onclick={() => addSource(source)}
            disabled={adding === source.id}
          >
            <span class="source-title">{source.title}</span>
            <span class="source-desc">{source.description.slice(0, 100)}{source.description.length > 100 ? '...' : ''}</span>
            {#if adding === source.id}
              <span class="adding-indicator">Adding...</span>
            {/if}
          </button>
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .source-picker-modal {
    background: white;
    border-radius: 12px;
    padding: 1.25rem;
    max-width: 550px;
    width: 90%;
    max-height: 70vh;
    overflow-y: auto;
    box-shadow: 0 8px 30px rgba(0, 0, 0, 0.15);
  }

  .picker-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1rem;
  }

  .picker-header h3 {
    margin: 0;
    font-size: 1rem;
    color: #1a1a2e;
  }

  .source-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .source-item {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    padding: 0.65rem 0.75rem;
    border: 1px solid #e5e7eb;
    border-radius: 8px;
    background: white;
    cursor: pointer;
    font-family: inherit;
    text-align: left;
    transition: border-color 0.15s, background 0.15s;
  }

  .source-item:hover:not(:disabled) {
    border-color: #6c63ff;
    background: #f5f3ff;
  }

  .source-item:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .source-title {
    font-weight: 600;
    font-size: 0.85rem;
    color: #1a1a2e;
  }

  .source-desc {
    font-size: 0.75rem;
    color: #6b7280;
    margin-top: 0.15rem;
  }

  .adding-indicator {
    font-size: 0.7rem;
    color: #6c63ff;
    font-style: italic;
    margin-top: 0.15rem;
  }

  .loading-text, .empty-text {
    text-align: center;
    color: #9ca3af;
    font-size: 0.85rem;
    padding: 2rem;
  }
</style>
```

---

### T28.7: Freeform Entry Input

Handled inline in the resumes page picker modal. When `entryType === 'freeform'`, show a textarea instead of the perspective picker.

---

### T28.8: Update Resumes Page

**File:** `packages/webui/src/routes/resumes/+page.svelte`

#### Update `pickerModal` state

```typescript
// Before:
let pickerModal = $state({ open: false, section: '', sourceId: null as string | null, sourceLabel: null as string | null })

// After:
let pickerModal = $state({
  open: false,
  sectionId: '',
  entryType: '',
  sourceId: null as string | null,
  sourceLabel: null as string | null,
})

// Also add new picker states:
let skillsPickerSectionId = $state<string | null>(null)
let sourcePickerState = $state<{ sectionId: string; sourceType: string } | null>(null)
let freeformInput = $state('')
let freeformSaving = $state(false)
```

#### Update `openPicker`

```typescript
// Before:
async function openPicker(section: string, sourceId?: string, sourceLabel?: string) {
  pickerModal = { open: true, section, sourceId: sourceId ?? null, sourceLabel: sourceLabel ?? null }
  // ...load perspectives...
}

// After:
async function openPicker(sectionId: string, entryType: string, sourceId?: string, sourceLabel?: string) {
  // Type-specific dispatch
  switch (entryType) {
    case 'skills':
      skillsPickerSectionId = sectionId
      return

    case 'education':
      sourcePickerState = { sectionId, sourceType: 'education' }
      return

    case 'projects':
      sourcePickerState = { sectionId, sourceType: 'project' }
      return

    case 'clearance':
      sourcePickerState = { sectionId, sourceType: 'clearance' }
      return

    case 'freeform':
      pickerModal = { open: true, sectionId, entryType, sourceId: null, sourceLabel: null }
      freeformInput = ''
      return

    case 'experience':
    case 'presentations':
    default:
      // Perspective picker (existing behavior)
      pickerModal = { open: true, sectionId, entryType, sourceId: sourceId ?? null, sourceLabel: sourceLabel ?? null }
      pickerArchetypeFilter = ''
      pickerDomainFilter = ''
      pickerLoading = true
      try {
        const filter: Record<string, unknown> = { status: 'approved', limit: 500 }
        if (sourceId) {
          filter.source_id = sourceId
        }
        const result = await forge.perspectives.list(filter)
        if (result.ok) {
          availablePerspectives = result.data
        } else {
          addToast({ message: friendlyError(result.error), type: 'error' })
        }
      } catch (e) {
        addToast({ message: 'Failed to load perspectives', type: 'error' })
      } finally {
        pickerLoading = false
      }
      return
  }
}
```

#### Update `addEntry`

```typescript
// Before:
async function addEntry(perspectiveId: string) {
  if (!selectedResumeId || !pickerModal.section) return
  const section = pickerModal.section
  const currentSection = resumeDetail?.sections[section] ?? []
  const position = currentSection.length
  const result = await forge.resumes.addEntry(selectedResumeId, {
    perspective_id: perspectiveId,
    section,
    position,
  })
  // ...
}

// After:
async function addEntry(perspectiveId: string) {
  if (!selectedResumeId || !pickerModal.sectionId) return
  const sectionId = pickerModal.sectionId

  // Find current entry count for position
  const currentSection = resumeDetail?.sections.find(s => s.id === sectionId)
  const position = currentSection?.entries.length ?? 0

  try {
    const result = await forge.resumes.addEntry(selectedResumeId, {
      section_id: sectionId,
      perspective_id: perspectiveId,
      position,
    })
    if (result.ok) {
      addToast({ message: 'Entry added', type: 'success' })
      // Refresh
      const refreshFilter: Record<string, unknown> = { status: 'approved', limit: 500 }
      if (pickerModal.sourceId) {
        refreshFilter.source_id = pickerModal.sourceId
      }
      const listResult = await forge.perspectives.list(refreshFilter)
      if (listResult.ok) {
        availablePerspectives = listResult.data
      }
      await Promise.all([
        loadResumeDetail(selectedResumeId!),
        loadGapAnalysis(selectedResumeId!),
        loadIR(selectedResumeId!),
      ])
    } else {
      addToast({ message: friendlyError(result.error), type: 'error' })
    }
  } catch (e) {
    addToast({ message: 'Failed to add entry', type: 'error' })
  }
}
```

#### Add freeform entry handler

```typescript
async function addFreeformEntry() {
  if (!selectedResumeId || !pickerModal.sectionId || !freeformInput.trim()) return
  freeformSaving = true
  try {
    const sectionId = pickerModal.sectionId
    const currentSection = resumeDetail?.sections.find(s => s.id === sectionId)
    const position = currentSection?.entries.length ?? 0

    const result = await forge.resumes.addEntry(selectedResumeId, {
      section_id: sectionId,
      content: freeformInput.trim(),
      position,
    })
    if (result.ok) {
      addToast({ message: 'Freeform entry added', type: 'success' })
      freeformInput = ''
      closePicker()
      await Promise.all([
        loadResumeDetail(selectedResumeId!),
        loadIR(selectedResumeId!),
      ])
    } else {
      addToast({ message: friendlyError(result.error), type: 'error' })
    }
  } catch (e) {
    addToast({ message: 'Failed to add entry', type: 'error' })
  } finally {
    freeformSaving = false
  }
}
```

#### Update `closePicker`

```typescript
// Before:
function closePicker() {
  pickerModal = { open: false, section: '', sourceId: null, sourceLabel: null }
  availablePerspectives = []
}

// After:
function closePicker() {
  pickerModal = { open: false, sectionId: '', entryType: '', sourceId: null, sourceLabel: null }
  availablePerspectives = []
  freeformInput = ''
}
```

#### Section management callbacks

```typescript
async function handleAddSection(entryType: string, title: string) {
  if (!selectedResumeId) return
  const currentSections = ir?.sections ?? []
  const position = currentSections.length

  const result = await forge.resumes.createSection(selectedResumeId, {
    title,
    entry_type: entryType,
    position,
  })
  if (result.ok) {
    addToast({ message: `Section "${title}" added`, type: 'success' })
    await handleIRUpdate()
  } else {
    addToast({ message: friendlyError(result.error), type: 'error' })
  }
}

async function handleDeleteSection(sectionId: string) {
  if (!selectedResumeId) return
  const result = await forge.resumes.deleteSection(selectedResumeId, sectionId)
  if (result.ok) {
    addToast({ message: 'Section deleted', type: 'success' })
    await handleIRUpdate()
  } else {
    addToast({ message: friendlyError(result.error), type: 'error' })
  }
}

async function handleRenameSection(sectionId: string, newTitle: string) {
  if (!selectedResumeId) return
  const result = await forge.resumes.updateSection(selectedResumeId, sectionId, { title: newTitle })
  if (result.ok) {
    addToast({ message: 'Section renamed', type: 'success' })
    await handleIRUpdate()
  } else {
    addToast({ message: friendlyError(result.error), type: 'error' })
  }
}

async function handleMoveSection(sectionId: string, direction: 'up' | 'down') {
  if (!selectedResumeId || !ir) return
  const sections = [...ir.sections].sort((a, b) => a.display_order - b.display_order)
  const idx = sections.findIndex(s => s.id === sectionId)
  if (idx < 0) return
  const swapIdx = direction === 'up' ? idx - 1 : idx + 1
  if (swapIdx < 0 || swapIdx >= sections.length) return

  // Swap positions
  const currentPos = sections[idx].display_order
  const swapPos = sections[swapIdx].display_order

  await Promise.all([
    forge.resumes.updateSection(selectedResumeId, sections[idx].id, { position: swapPos }),
    forge.resumes.updateSection(selectedResumeId, sections[swapIdx].id, { position: currentPos }),
  ])

  await handleIRUpdate()
}
```

#### Update `DragNDropView` props in template

```svelte
<!-- Before: -->
<DragNDropView
  {ir}
  {resumeId}
  onUpdate={handleIRUpdate}
  onAddEntry={openPicker}
/>

<!-- After: -->
<DragNDropView
  ir={ir}
  resumeId={selectedResumeId}
  onUpdate={handleIRUpdate}
  onAddEntry={(sectionId, entryType, sourceId, sourceLabel) => openPicker(sectionId, entryType, sourceId, sourceLabel)}
  onAddSection={handleAddSection}
  onDeleteSection={handleDeleteSection}
  onRenameSection={handleRenameSection}
  onMoveSection={handleMoveSection}
/>
```

#### Add picker modals in template

```svelte
<!-- Skills picker modal -->
{#if skillsPickerSectionId && selectedResumeId}
  <SkillsPicker
    resumeId={selectedResumeId}
    sectionId={skillsPickerSectionId}
    onClose={() => skillsPickerSectionId = null}
    onUpdate={handleIRUpdate}
  />
{/if}

<!-- Source picker modal -->
{#if sourcePickerState && selectedResumeId}
  <SourcePicker
    resumeId={selectedResumeId}
    sectionId={sourcePickerState.sectionId}
    sourceType={sourcePickerState.sourceType}
    onClose={() => sourcePickerState = null}
    onUpdate={handleIRUpdate}
  />
{/if}

<!-- Freeform input in existing picker modal -->
{#if pickerModal.open && pickerModal.entryType === 'freeform'}
  <div class="modal-overlay" onclick={closePicker}>
    <div class="picker-modal" onclick|stopPropagation>
      <h3>Add Freeform Content</h3>
      <textarea
        class="freeform-textarea"
        bind:value={freeformInput}
        placeholder="Enter text content..."
        rows={5}
      ></textarea>
      <div class="picker-actions">
        <button class="btn btn-sm btn-ghost" onclick={closePicker}>Cancel</button>
        <button
          class="btn btn-sm btn-primary"
          onclick={addFreeformEntry}
          disabled={!freeformInput.trim() || freeformSaving}
        >
          {freeformSaving ? 'Adding...' : 'Add Entry'}
        </button>
      </div>
    </div>
  </div>
{/if}
```

#### Update imports

```typescript
import SkillsPicker from '$lib/components/resume/SkillsPicker.svelte'
import SourcePicker from '$lib/components/resume/SourcePicker.svelte'
```

#### Update `SECTIONS` constant and `filteredPickerPerspectives`

The `SECTIONS` constant and `SECTION_LABELS` at the top of the file are no longer needed for the new section-entity model. They were used for the old `Record<string, ...>` section format. Remove or keep for backward compat.

Update `filteredPickerPerspectives` to use the new sections array format:

```typescript
let filteredPickerPerspectives = $derived.by(() => {
  let result = availablePerspectives
  if (pickerArchetypeFilter) {
    result = result.filter(p => p.target_archetype === pickerArchetypeFilter)
  }
  if (pickerDomainFilter) {
    result = result.filter(p => p.domain?.toLowerCase().includes(pickerDomainFilter.toLowerCase()))
  }
  // Exclude perspectives already in this resume
  if (resumeDetail) {
    const existingPerspectiveIds = new Set(
      resumeDetail.sections
        .flatMap(s => s.entries)
        .filter(e => e.perspective_id)
        .map(e => e.perspective_id)
    )
    result = result.filter(p => !existingPerspectiveIds.has(p.id))
  }
  return result
})
```

#### Update `getEntryDisplayContent`

No change needed — already handles null gracefully.

#### Update `collapsedSections`

Currently keyed by section name string. After migration, key by section ID:

```typescript
// Works unchanged — already uses string keys
// The section name changes from 'experience' to a UUID, but the logic is the same
```

---

### T28.9: Route Tests

**File:** `packages/core/src/routes/__tests__/resumes.test.ts`

#### Section CRUD tests

```typescript
describe('Section CRUD routes', () => {
  test('POST /resumes/:id/sections creates a section', async () => {
    const resumeId = seedResume(ctx.db)
    const res = await app.request(`/api/resumes/${resumeId}/sections`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ title: 'Work History', entry_type: 'experience', position: 0 }),
    })
    expect(res.status).toBe(201)
    const { data } = await res.json()
    expect(data.title).toBe('Work History')
    expect(data.entry_type).toBe('experience')
    expect(data.id).toHaveLength(36)
  })

  test('GET /resumes/:id/sections lists sections ordered by position', async () => {
    const resumeId = seedResume(ctx.db)
    seedResumeSection(ctx.db, resumeId, { title: 'Skills', entryType: 'skills', position: 1 })
    seedResumeSection(ctx.db, resumeId, { title: 'Summary', entryType: 'freeform', position: 0 })

    const res = await app.request(`/api/resumes/${resumeId}/sections`)
    expect(res.status).toBe(200)
    const { data } = await res.json()
    expect(data).toHaveLength(2)
    expect(data[0].title).toBe('Summary')
    expect(data[1].title).toBe('Skills')
  })

  test('PATCH /resumes/:id/sections/:sectionId updates title', async () => {
    const resumeId = seedResume(ctx.db)
    const sectionId = seedResumeSection(ctx.db, resumeId, { title: 'Old Title', entryType: 'experience' })

    const res = await app.request(`/api/resumes/${resumeId}/sections/${sectionId}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ title: 'Professional Experience' }),
    })
    expect(res.status).toBe(200)
    const { data } = await res.json()
    expect(data.title).toBe('Professional Experience')
    expect(data.entry_type).toBe('experience')  // immutable
  })

  test('DELETE /resumes/:id/sections/:sectionId deletes with cascade', async () => {
    const resumeId = seedResume(ctx.db)
    const sectionId = seedResumeSection(ctx.db, resumeId, { entryType: 'experience' })
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }])
    const perspId = seedPerspective(ctx.db, bulletId)
    seedResumeEntry(ctx.db, resumeId, sectionId, { perspectiveId: perspId })

    const res = await app.request(`/api/resumes/${resumeId}/sections/${sectionId}`, {
      method: 'DELETE',
    })
    expect(res.status).toBe(204)

    // Entries should be gone
    const entries = ctx.db.query('SELECT * FROM resume_entries WHERE section_id = ?').all(sectionId)
    expect(entries).toHaveLength(0)
  })

  test('returns 404 for section on wrong resume', async () => {
    const resume1 = seedResume(ctx.db)
    const resume2 = seedResume(ctx.db)
    const sectionId = seedResumeSection(ctx.db, resume1, { entryType: 'experience' })

    const res = await app.request(`/api/resumes/${resume2}/sections/${sectionId}`, {
      method: 'DELETE',
    })
    expect(res.status).toBe(404)
  })
})
```

#### Skills CRUD tests

```typescript
describe('Resume Skills routes', () => {
  test('POST /resumes/:id/sections/:sectionId/skills adds a skill', async () => {
    const resumeId = seedResume(ctx.db)
    const sectionId = seedResumeSection(ctx.db, resumeId, { title: 'Skills', entryType: 'skills' })
    const skillId = seedSkill(ctx.db, { name: 'Python' })

    const res = await app.request(
      `/api/resumes/${resumeId}/sections/${sectionId}/skills`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ skill_id: skillId }),
      }
    )
    expect(res.status).toBe(201)
    const { data } = await res.json()
    expect(data.skill_id).toBe(skillId)
  })

  test('DELETE /resumes/:id/sections/:sectionId/skills/:skillId removes a skill', async () => {
    const resumeId = seedResume(ctx.db)
    const sectionId = seedResumeSection(ctx.db, resumeId, { entryType: 'skills' })
    const skillId = seedSkill(ctx.db, { name: 'Go' })
    seedResumeSkill(ctx.db, sectionId, skillId)

    const res = await app.request(
      `/api/resumes/${resumeId}/sections/${sectionId}/skills/${skillId}`,
      { method: 'DELETE' }
    )
    expect(res.status).toBe(204)

    // Verify removed
    const skills = ctx.db.query('SELECT * FROM resume_skills WHERE section_id = ?').all(sectionId)
    expect(skills).toHaveLength(0)
  })

  test('rejects adding skill to non-skills section', async () => {
    const resumeId = seedResume(ctx.db)
    const sectionId = seedResumeSection(ctx.db, resumeId, { entryType: 'experience' })
    const skillId = seedSkill(ctx.db, { name: 'Python' })

    const res = await app.request(
      `/api/resumes/${resumeId}/sections/${sectionId}/skills`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ skill_id: skillId }),
      }
    )
    expect(res.status).toBe(422)  // VALIDATION_ERROR
  })

  test('duplicate skill returns 409', async () => {
    const resumeId = seedResume(ctx.db)
    const sectionId = seedResumeSection(ctx.db, resumeId, { entryType: 'skills' })
    const skillId = seedSkill(ctx.db, { name: 'Rust' })
    seedResumeSkill(ctx.db, sectionId, skillId)

    const res = await app.request(
      `/api/resumes/${resumeId}/sections/${sectionId}/skills`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ skill_id: skillId }),
      }
    )
    expect(res.status).toBe(409)
  })
})
```

#### Updated entry tests

```typescript
describe('Updated entry endpoints', () => {
  test('POST /resumes/:id/entries with section_id', async () => {
    const resumeId = seedResume(ctx.db)
    const sectionId = seedResumeSection(ctx.db, resumeId, { entryType: 'experience' })
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }])
    const perspId = seedPerspective(ctx.db, bulletId)

    const res = await app.request(`/api/resumes/${resumeId}/entries`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ section_id: sectionId, perspective_id: perspId, position: 0 }),
    })
    expect(res.status).toBe(201)
    const { data } = await res.json()
    expect(data.section_id).toBe(sectionId)
  })

  test('POST /resumes/:id/entries freeform (no perspective_id)', async () => {
    const resumeId = seedResume(ctx.db)
    const sectionId = seedResumeSection(ctx.db, resumeId, { entryType: 'freeform' })

    const res = await app.request(`/api/resumes/${resumeId}/entries`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ section_id: sectionId, content: 'Hello freeform', position: 0 }),
    })
    expect(res.status).toBe(201)
    const { data } = await res.json()
    expect(data.perspective_id).toBeNull()
    expect(data.content).toBe('Hello freeform')
  })

  test('reorder entries with section_id', async () => {
    const resumeId = seedResume(ctx.db)
    const sectionId = seedResumeSection(ctx.db, resumeId, { entryType: 'experience' })
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }])
    const p1 = seedPerspective(ctx.db, bulletId, { content: 'First' })
    const p2 = seedPerspective(ctx.db, bulletId, { content: 'Second' })
    const e1 = seedResumeEntry(ctx.db, resumeId, sectionId, { perspectiveId: p1, position: 0 })
    const e2 = seedResumeEntry(ctx.db, resumeId, sectionId, { perspectiveId: p2, position: 1 })

    const res = await app.request(`/api/resumes/${resumeId}/entries/reorder`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        entries: [
          { id: e2, section_id: sectionId, position: 0 },
          { id: e1, section_id: sectionId, position: 1 },
        ]
      }),
    })
    expect(res.status).toBe(200)
  })
})
```

#### Smoke tests

```typescript
describe('End-to-end smoke: section lifecycle', () => {
  test('add section -> add skills -> verify IR', async () => {
    const resumeId = seedResume(ctx.db)
    const skillId = seedSkill(ctx.db, { name: 'TypeScript', category: 'Languages' })

    // Create skills section
    const createRes = await app.request(`/api/resumes/${resumeId}/sections`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ title: 'Technical Skills', entry_type: 'skills', position: 0 }),
    })
    const { data: section } = await createRes.json()

    // Add skill
    await app.request(`/api/resumes/${resumeId}/sections/${section.id}/skills`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ skill_id: skillId }),
    })

    // Get IR
    const irRes = await app.request(`/api/resumes/${resumeId}/ir`)
    const { data: ir } = await irRes.json()
    expect(ir.sections).toHaveLength(1)
    expect(ir.sections[0].type).toBe('skills')
    expect(ir.sections[0].items[0].kind).toBe('skill_group')
    expect(ir.sections[0].items[0].categories[0].skills).toContain('TypeScript')
  })

  test('add freeform section -> add entry -> verify IR', async () => {
    const resumeId = seedResume(ctx.db)

    // Create freeform section
    const createRes = await app.request(`/api/resumes/${resumeId}/sections`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ title: 'Summary', entry_type: 'freeform', position: 0 }),
    })
    const { data: section } = await createRes.json()

    // Add freeform entry
    await app.request(`/api/resumes/${resumeId}/entries`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ section_id: section.id, content: 'Experienced engineer.', position: 0 }),
    })

    // Get IR
    const irRes = await app.request(`/api/resumes/${resumeId}/ir`)
    const { data: ir } = await irRes.json()
    expect(ir.sections).toHaveLength(1)
    expect(ir.sections[0].type).toBe('freeform')
    expect(ir.sections[0].items[0].kind).toBe('summary')
    expect(ir.sections[0].items[0].content).toBe('Experienced engineer.')
  })
})
```

---

### T28.10: WebUI Build Verification

After all changes, verify:

```bash
cd packages/webui && bun run build
cd packages/webui && bun run check  # svelte-check
```

No TypeScript errors from the type changes in `@forge/sdk`.

---

### T28.11: Documentation

- [ ] Update `docs/src/api/routes.md` with section CRUD and skills endpoints (including `GET /resumes/:id/sections/:sectionId/skills`)
- [ ] Update PLAN.md status

---

## Acceptance Criteria

### API
- [ ] POST/GET/PATCH/DELETE `/resumes/:id/sections` work correctly
- [ ] GET/POST/DELETE `/resumes/:id/sections/:sectionId/skills` work correctly
- [ ] PATCH `/resumes/:id/sections/:sectionId/skills/reorder` works correctly
- [ ] Entry creation accepts `section_id` instead of `section` string
- [ ] Freeform entries created with `perspective_id = null` and `content`
- [ ] Returns 404 when section belongs to different resume
- [ ] Returns 422 when adding skill to non-skills section

### SDK
- [ ] `forge.resumes.createSection()` / `listSections()` / `updateSection()` / `deleteSection()`
- [ ] `forge.resumes.addSkill()` / `removeSkill()` / `listSkills()` / `reorderSkills()`
- [ ] `forge.resumes.addEntry()` accepts new `AddResumeEntry` type

### UI: DragNDrop
- [ ] "Add Section" button shows dropdown of entry types
- [ ] Section header: editable title (double-click), entry type badge, up/down arrows, delete
- [ ] Delete section shows confirmation dialog
- [ ] Section reorder via up/down arrows persists
- [ ] `onAddEntry` passes `section.id` (UUID) not `section.type` (string)

### UI: Pickers
- [ ] Skills picker: modal with checkboxes grouped by category
- [ ] Skills changes persist immediately via API
- [ ] Source picker: shows matching sources for education/projects/clearance
- [ ] Adding source creates entry (with perspective if available, direct content otherwise)
- [ ] Freeform picker: textarea input, creates entry with null perspective_id

### UI: Resumes Page
- [ ] `pickerModal` uses `sectionId` + `entryType` (not `section` string)
- [ ] `openPicker` dispatches to type-specific picker
- [ ] `addEntry` sends `section_id` to API
- [ ] `closePicker` resets all picker states
- [ ] `filteredPickerPerspectives` uses new sections array format

### Tests
- [ ] Route tests for section CRUD (create, list, update, delete with cascade)
- [ ] Route tests for skills CRUD (add, remove, duplicate rejection)
- [ ] Route tests for updated entry endpoints (section_id, freeform)
- [ ] Smoke: add section + skills + verify IR
- [ ] Smoke: add freeform section + entry + verify IR
- [ ] WebUI builds without errors

---

## Estimated Effort

| Task | Lines changed (est.) |
|------|---------------------|
| T28.1 API routes | ~100 |
| T28.2 SDK methods | ~80 |
| T28.3 DragNDrop updates | ~150 |
| T28.4 AddSectionDropdown | ~120 |
| T28.5 SkillsPicker | ~180 |
| T28.6 SourcePicker | ~150 |
| T28.7 Freeform input | ~30 |
| T28.8 Resumes page | ~200 |
| T28.9 Route tests | ~250 |
| T28.10 Build verification | ~0 |
| T28.11 Documentation | ~30 |
| **Total** | **~1,290** |
