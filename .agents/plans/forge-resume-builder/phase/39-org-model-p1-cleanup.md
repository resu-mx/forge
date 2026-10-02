# Phase 39: Organization Model P1 Cleanup

**Status:** Planning
**Date:** 2026-04-03
**Spec:** [2026-04-03-org-model-evolution.md](../refs/specs/2026-04-03-org-model-evolution.md) — Part B items 2, 3, 5, 7
**Depends on:** Phases 37-38 (education subtypes + kanban must be stable)
**Blocks:** None currently identified
**Parallelizable with:** Any phase that does not touch campus-repository, resume-compiler, SourcesView, or the `/data/organizations` page

## Goal

Close four P1 gaps in the organization model without introducing new migrations. Campus editing completes basic CRUD. The searchable OrgCombobox replaces the plain `<select>` in education forms and is reusable in the kanban picker. Removing stale `location`/`headquarters` from the save payload prevents accidental overwrites. Wiring campus data into the IR compiler ensures structured campus locations flow onto rendered resumes.

## Non-Goals

- **P2: Legacy column removal** — `institution`, `issuing_body` on `source_education` and `location`, `headquarters` on `organizations` stay in the schema. Removing them requires a table-rebuild migration (future phase).
- **P2: `org_type` vs tags redundancy** — No changes to the `org_type` column or its relationship to tags.
- **P3: Org card detail enrichment** — Alias pills and campus count on list cards are deferred.
- **P3: Tag-based filtering in OrgPickerModal** — Deferred.
- **SDK resource methods for campus/alias** — The UI continues using raw `fetch()` for campus and alias CRUD. SDK wrapper methods are deferred.

## Context

After Phases 37-38, the organization model has campuses, aliases, tags, and a kanban pipeline. Four small gaps remain at P1 priority:

1. **Campus editing** — `campus-repository.ts` has create/read/delete but no update. The only recourse for a typo is delete-and-recreate, which orphans `campus_id` FKs on `source_education`.
2. **Searchable education dropdown** — The education org `<select>` in SourcesView forces users to scroll. Aliases exist in the DB but are invisible in the dropdown.
3. **Stale payload fields** — `saveOrg()` in the org editor still sends `location` and `headquarters` even though the form fields are gone.
4. **Campus in IR compiler** — The resume compiler JOINs `organizations` for the institution name but does not JOIN `org_campuses` for city/state. Campus location data is informational only and does not flow to the rendered resume.

Already resolved prior to this phase: OrganizationStatus type (updated to kanban values), SDK exports for OrgCampus/OrgAlias/CampusModality (exported from index.ts), roleFilteredOrgs (includes project sources).

## Scope

| Spec item | Part B # | Covered here? |
|-----------|----------|---------------|
| Campus editing (PATCH + inline UI) | 2 | Yes |
| Searchable education dropdown (OrgCombobox) | 3 | Yes |
| Remove location/headquarters from save payload | 5 | Yes |
| Campus in IR compiler | 7 | Yes |
| Legacy column removal (institution/issuing_body/location/headquarters) | 1 | No (P2) |
| org_type vs tags redundancy | 6 | No (P2) |
| Org card detail enrichment | 4 | No (P3) |
| Tag-based kanban picker filter | 8 | No (P3) |
| Stale text field cleanup | 9 | No (P2) |
| SDK campus/alias resource methods | 10 | No (deferred) |

## Files to Create

| File | Description |
|------|-------------|
| `packages/webui/src/lib/components/OrgCombobox.svelte` | Searchable combobox/autocomplete for org selection with alias matching and tag pills |

## Files to Modify

| File | Change |
|------|--------|
| `packages/core/src/db/repositories/campus-repository.ts` | Add `UpdateCampusInput` interface and `update()` function |
| `packages/core/src/routes/campuses.ts` | Add `PATCH /campuses/:id` route |
| `packages/core/src/services/resume-compiler.ts` | JOIN `org_campuses` in `buildEducationItems`, select campus fields |
| `packages/core/src/types/index.ts` | Add `campus_name`, `campus_city`, `campus_state` to `EducationItem` |
| `packages/sdk/src/types.ts` | Mirror `campus_name`, `campus_city`, `campus_state` fields onto `EducationItem` |
| `packages/core/src/templates/sb2nov.ts` | Use campus city/state for education location field with fallback |
| `packages/webui/src/lib/components/SourcesView.svelte` | Replace `<select>` with `OrgCombobox` in education forms |
| `packages/webui/src/routes/data/sources/SourcesView.svelte` | Legacy copy of SourcesView — delete or apply same OrgCombobox changes |
| `packages/webui/src/routes/data/organizations/+page.svelte` | Remove `formLocation`/`formHeadquarters`; add inline campus editing |

## Fallback Strategies

1. **OrgCombobox complexity** — If the custom combobox proves too complex, a simpler approach is to add an `<input>` filter field above the existing `<select>`. This provides 80% of the UX benefit with minimal risk.
2. **Campus inline edit** — If inline editing in the campus card is too complex, fall back to a simple "Edit" button that opens the same add-campus form pre-populated with existing values.
3. **Campus in compiler** — If the LEFT JOIN causes performance issues (unlikely with SQLite), the campus data can be fetched in a separate query per education item.

---

## Tasks

### Task 39.1 — Campus Repository `update()` Function

**File:** `packages/core/src/db/repositories/campus-repository.ts`

Add an `UpdateCampusInput` interface and an `update()` function following the partial-update pattern used in `organization-repository.ts`.

```typescript
export interface UpdateCampusInput {
  name?: string
  modality?: string
  address?: string | null
  city?: string | null
  state?: string | null
  zipcode?: string | null
  country?: string | null
  is_headquarters?: number
}

/** Partially update a campus. Returns the updated campus or null if not found. */
export function update(db: Database, id: string, input: UpdateCampusInput): OrgCampus | null {
  const existing = get(db, id)
  if (!existing) return null

  const sets: string[] = []
  const params: unknown[] = []

  if (input.name !== undefined) { sets.push('name = ?'); params.push(input.name) }
  if (input.modality !== undefined) { sets.push('modality = ?'); params.push(input.modality) }
  if (input.address !== undefined) { sets.push('address = ?'); params.push(input.address) }
  if (input.city !== undefined) { sets.push('city = ?'); params.push(input.city) }
  if (input.state !== undefined) { sets.push('state = ?'); params.push(input.state) }
  if (input.zipcode !== undefined) { sets.push('zipcode = ?'); params.push(input.zipcode) }
  if (input.country !== undefined) { sets.push('country = ?'); params.push(input.country) }
  if (input.is_headquarters !== undefined) { sets.push('is_headquarters = ?'); params.push(input.is_headquarters) }

  if (sets.length === 0) return existing

  params.push(id)

  return db
    .query(`UPDATE org_campuses SET ${sets.join(', ')} WHERE id = ? RETURNING *`)
    .get(...params) as OrgCampus
}
```

**Acceptance criteria:**
- `update(db, id, { name: 'New Name' })` returns updated `OrgCampus` with changed name, other fields unchanged.
- `update(db, id, { city: null })` sets city to NULL.
- `update(db, 'nonexistent-id', { name: 'X' })` returns `null`.
- `update(db, id, {})` returns the existing campus unchanged (no SQL executed).

**Failure criteria:**
- If the campus does not exist, must return `null`, not throw.

---

### Task 39.2 — PATCH `/campuses/:id` Route

**File:** `packages/core/src/routes/campuses.ts`

Add a PATCH route between the existing POST and DELETE routes.

Insert the following after the `app.post('/organizations/:orgId/campuses', ...)` handler (after line 33, before the `app.delete` on line 35):

```typescript
  app.patch('/campuses/:id', async (c) => {
    const body = await c.req.json()
    const updated = CampusRepo.update(db, c.req.param('id'), {
      name: body.name,
      modality: body.modality,
      address: body.address,
      city: body.city,
      state: body.state,
      zipcode: body.zipcode,
      country: body.country,
      is_headquarters: body.is_headquarters !== undefined ? (body.is_headquarters ? 1 : 0) : undefined,
    })
    if (!updated) return c.json({ error: { code: 'NOT_FOUND', message: 'Campus not found' } }, 404)
    return c.json({ data: updated })
  })
```

**Acceptance criteria:**
- `PATCH /api/campuses/:id` with `{ "name": "Online Campus" }` returns `200` with the updated campus.
- `PATCH /api/campuses/nonexistent` returns `404` with standard error shape.
- Boolean `is_headquarters` from JSON is converted to integer `0`/`1` for the repository.
- Omitting a field from the body leaves that column unchanged.

If `c.req.json()` throws (malformed body), let Hono's error handler return 400.

**Failure criteria:**
- Must not require `organization_id` in the PATCH body (it is immutable).
- Malformed JSON body must not cause a 500.

---

### Task 39.3 — Inline Campus Editing in Org Editor

**File:** `packages/webui/src/routes/data/organizations/+page.svelte`

Replace the read-only campus cards with an inline edit mode. Clicking a campus card toggles it into edit mode with pre-populated fields. Saving calls `PATCH /api/campuses/:id`.

**Step 3a — Add edit state variables** (after existing campus management state, around line 52):

```typescript
  // Campus inline editing
  let editingCampusId = $state<string | null>(null)
  let editCampusName = $state('')
  let editCampusModality = $state('in_person')
  let editCampusAddress = $state('')
  let editCampusCity = $state('')
  let editCampusState = $state('')
  let editCampusZipcode = $state('')
  let editCampusCountry = $state('')
  let editCampusIsHQ = $state(false)
  let savingCampusEdit = $state(false)
```

**Step 3b — Add edit functions** (after the existing `deleteCampus` function, around line 276):

```typescript
  function startEditCampus(campus: OrgCampus) {
    editingCampusId = campus.id
    editCampusName = campus.name
    editCampusModality = campus.modality
    editCampusAddress = campus.address ?? ''
    editCampusCity = campus.city ?? ''
    editCampusState = campus.state ?? ''
    editCampusZipcode = campus.zipcode ?? ''
    editCampusCountry = campus.country ?? ''
    editCampusIsHQ = !!campus.is_headquarters
  }

  function cancelEditCampus() {
    editingCampusId = null
  }

  async function saveEditCampus() {
    if (!editingCampusId || !editCampusName.trim()) return
    savingCampusEdit = true
    const res = await fetch(`/api/campuses/${editingCampusId}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        name: editCampusName.trim(),
        modality: editCampusModality,
        address: editCampusAddress.trim() || null,
        city: editCampusCity.trim() || null,
        state: editCampusState.trim() || null,
        zipcode: editCampusZipcode.trim() || null,
        country: editCampusCountry.trim() || null,
        is_headquarters: editCampusIsHQ,
      }),
    })
    if (res.ok) {
      const body = await res.json()
      orgCampuses = orgCampuses.map(c => c.id === editingCampusId ? body.data : c)
      editingCampusId = null
      addToast({ message: 'Campus updated.', type: 'success' })
    } else {
      addToast({ message: 'Failed to update campus.', type: 'error' })
    }
    savingCampusEdit = false
  }
```

**Step 3c — Replace the campus list item template** (lines 566-591). Replace the existing `<li class="campus-item">` block with:

```svelte
                {#each orgCampuses as campus (campus.id)}
                  {#if editingCampusId === campus.id}
                    <li class="campus-item campus-editing">
                      <div class="campus-edit-form">
                        <div class="form-row">
                          <div class="form-group">
                            <label for="edit-campus-name">Name *</label>
                            <input id="edit-campus-name" type="text" bind:value={editCampusName} />
                          </div>
                          <div class="form-group">
                            <label for="edit-campus-modality">Modality</label>
                            <select id="edit-campus-modality" bind:value={editCampusModality}>
                              <option value="in_person">In Person</option>
                              <option value="remote">Remote / Online</option>
                              <option value="hybrid">Hybrid</option>
                            </select>
                          </div>
                        </div>
                        <div class="form-row">
                          <div class="form-group">
                            <label for="edit-campus-address">Address</label>
                            <input id="edit-campus-address" type="text" bind:value={editCampusAddress} />
                          </div>
                        </div>
                        <div class="form-row">
                          <div class="form-group">
                            <label for="edit-campus-city">City</label>
                            <input id="edit-campus-city" type="text" bind:value={editCampusCity} />
                          </div>
                          <div class="form-group">
                            <label for="edit-campus-state">State</label>
                            <input id="edit-campus-state" type="text" bind:value={editCampusState} />
                          </div>
                          <div class="form-group">
                            <label for="edit-campus-zip">Zip</label>
                            <input id="edit-campus-zip" type="text" bind:value={editCampusZipcode} />
                          </div>
                        </div>
                        <div class="form-row">
                          <div class="form-group">
                            <label for="edit-campus-country">Country</label>
                            <input id="edit-campus-country" type="text" bind:value={editCampusCountry} />
                          </div>
                          <div class="form-group checkbox-group">
                            <label>
                              <input type="checkbox" bind:checked={editCampusIsHQ} /> Headquarters
                            </label>
                          </div>
                        </div>
                        <div class="campus-edit-actions">
                          <button class="btn btn-save btn-sm" onclick={saveEditCampus} disabled={savingCampusEdit || !editCampusName.trim()}>
                            {savingCampusEdit ? 'Saving...' : 'Save'}
                          </button>
                          <button class="btn btn-cancel btn-sm" onclick={cancelEditCampus} disabled={savingCampusEdit}>
                            Cancel
                          </button>
                        </div>
                      </div>
                    </li>
                  {:else}
                    <li class="campus-item">
                      <div class="campus-info" role="button" tabindex="0"
                        onclick={() => startEditCampus(campus)}
                        onkeydown={(e) => { if (e.key === 'Enter') startEditCampus(campus) }}
                        title="Click to edit"
                      >
                        <span class="campus-name">{campus.name}</span>
                        {#if campus.is_headquarters}
                          <span class="campus-hq-badge">HQ</span>
                        {/if}
                        <span class="campus-modality">{MODALITY_LABELS[campus.modality] ?? campus.modality}</span>
                        {#if campus.city || campus.state || campus.zipcode}
                          <span class="campus-location">
                            {[campus.city, campus.state, campus.zipcode].filter(Boolean).join(', ')}
                          </span>
                        {/if}
                        {#if campus.address}
                          <span class="campus-address">{campus.address}</span>
                        {/if}
                      </div>
                      <button
                        class="btn-delete-sm"
                        onclick={() => deleteCampus(campus.id)}
                        disabled={deletingCampusId === campus.id}
                        title="Delete campus"
                      >
                        {deletingCampusId === campus.id ? '...' : '×'}
                      </button>
                    </li>
                  {/if}
                {/each}
```

**Step 3d — Add CSS for edit mode** (append to existing `<style>` block):

```css
  .campus-editing { padding: 0.75rem; }
  .campus-edit-form { width: 100%; }
  .campus-edit-form .form-row { display: flex; gap: 0.5rem; margin-bottom: 0.4rem; }
  .campus-edit-form .form-group { flex: 1; min-width: 0; }
  .campus-edit-form label { display: block; font-size: 0.7rem; color: #6b7280; margin-bottom: 0.15rem; }
  .campus-edit-form input, .campus-edit-form select { width: 100%; padding: 0.3rem 0.5rem; font-size: 0.8rem; border: 1px solid #d1d5db; border-radius: 4px; }
  .campus-edit-actions { display: flex; gap: 0.4rem; margin-top: 0.4rem; }
  .campus-info[role="button"] { cursor: pointer; }
  .campus-info[role="button"]:hover { opacity: 0.8; }
  .btn-cancel { background: #f3f4f6; color: #374151; border: 1px solid #d1d5db; }
  .btn-cancel:hover:not(:disabled) { background: #e5e7eb; }
```

**Acceptance criteria:**
- Clicking a campus card enters edit mode with fields pre-populated.
- Saving sends PATCH and updates the card in-place.
- Cancel exits edit mode without changes.
- Only one campus can be in edit mode at a time.
- Keyboard accessible via Enter key on the campus info div.

**Failure criteria:**
- Edit mode must not conflict with the add-campus form (both can be visible).

---

### Task 39.4 — Remove `location`/`headquarters` from Save Payload

**File:** `packages/webui/src/routes/data/organizations/+page.svelte`

**Step 4a — Remove state variables.** Delete lines 29-30:

```typescript
  // DELETE these two lines:
  let formLocation = $state('')
  let formHeadquarters = $state('')
```

**Step 4b — Remove from `populateForm()`.** Delete lines 99-100 inside `populateForm(org)`:

```typescript
  // DELETE these two lines:
    formLocation = org.location ?? ''
    formHeadquarters = org.headquarters ?? ''
```

**Step 4c — Remove from `startNew()`.** Delete lines 123-124 inside the `startNew` function (or wherever the form reset sets these):

```typescript
  // DELETE these two lines:
    formLocation = ''
    formHeadquarters = ''
```

**Step 4d — Remove from `saveOrg()` payload.** Delete lines 165-166 in the payload object:

```typescript
  // DELETE these two lines from the payload:
      location: formLocation || undefined,
      headquarters: formHeadquarters || undefined,
```

**Acceptance criteria:**
- `formLocation` and `formHeadquarters` variables no longer exist in the file.
- The `saveOrg()` payload object does not include `location` or `headquarters`.
- Scoped to the `/data/organizations` `+page.svelte` save payload only. The `location`/`headquarters` fields are intentionally retained in `Organization` and `UpdateOrganization` types until Phase 40 migration.
- The component compiles without errors.
- Saving an organization does not send stale location/headquarters data.

**Failure criteria:**
- If any other code in the file references `formLocation` or `formHeadquarters`, those references must also be removed.

---

### Task 39.5 — OrgCombobox Component

**File:** `packages/webui/src/lib/components/OrgCombobox.svelte` (new)

A searchable combobox that replaces plain `<select>` elements for org selection. Features:
- Text input filters by org name and aliases (client-side, case-insensitive)
- Each option row shows the org name plus tag pills
- "Create New" option at bottom triggers a callback
- Controlled via `value` (org ID) and `onchange` event

```svelte
<script lang="ts">
  import type { Organization } from '@forge/sdk'

  let {
    id = '',
    organizations = [],
    aliases = new Map<string, string[]>(),
    value = $bindable<string | null>(null),
    placeholder = '-- Select --',
    disabled = false,
    oncreate,
  }: {
    id?: string
    organizations: Organization[]
    aliases?: Map<string, string[]>
    value: string | null
    placeholder?: string
    disabled?: boolean
    oncreate?: () => void
  } = $props()

  let query = $state('')
  let open = $state(false)
  let highlightIndex = $state(-1)
  let inputEl: HTMLInputElement | undefined = $state()
  let listEl: HTMLUListElement | undefined = $state()
  let listId = `org-combo-${crypto.randomUUID().slice(0,8)}`

  // Display the selected org name in the input when not focused
  let selectedOrg = $derived(organizations.find(o => o.id === value) ?? null)
  let displayValue = $derived(open ? query : (selectedOrg?.name ?? ''))

  let filtered = $derived.by(() => {
    if (!query.trim()) return organizations
    const q = query.toLowerCase()
    return organizations.filter(o => {
      const aliasMatch = aliases?.get(o.id)?.some(a => a.toLowerCase().includes(q))
      return o.name.toLowerCase().includes(q) || aliasMatch
    })
  })

  // NOTE: The `Organization` type does not include an `aliases` field. To support
  // alias search, OrgCombobox accepts an optional `aliases` prop (`Map<string, string[]>`)
  // pre-loaded by the parent. The parent loads aliases via `GET /api/organizations/:id/aliases`
  // for each org on mount, or the combobox falls back to name-only search if no aliases map
  // is provided.

  function openDropdown() {
    if (disabled) return
    open = true
    query = ''
    highlightIndex = -1
  }

  function closeDropdown() {
    // Small delay to allow click to register on options
    setTimeout(() => {
      open = false
      query = ''
    }, 150)
  }

  function selectOrg(orgId: string) {
    value = orgId
    open = false
    query = ''
  }

  function clearSelection() {
    value = null
    query = ''
    if (inputEl) inputEl.focus()
  }

  function handleKeydown(e: KeyboardEvent) {
    if (!open) {
      if (e.key === 'ArrowDown' || e.key === 'Enter') {
        e.preventDefault()
        openDropdown()
      }
      return
    }

    if (e.key === 'ArrowDown') {
      e.preventDefault()
      const max = filtered.length + (oncreate ? 1 : 0) - 1
      highlightIndex = Math.min(highlightIndex + 1, max)
      scrollToHighlighted()
    } else if (e.key === 'ArrowUp') {
      e.preventDefault()
      highlightIndex = Math.max(highlightIndex - 1, 0)
      scrollToHighlighted()
    } else if (e.key === 'Enter') {
      e.preventDefault()
      if (highlightIndex >= 0 && highlightIndex < filtered.length) {
        selectOrg(filtered[highlightIndex].id)
      } else if (highlightIndex === filtered.length && oncreate) {
        oncreate()
        open = false
      }
    } else if (e.key === 'Escape') {
      e.preventDefault()
      open = false
      query = ''
    }
  }

  function scrollToHighlighted() {
    if (!listEl) return
    const item = listEl.children[highlightIndex] as HTMLElement | undefined
    if (item) item.scrollIntoView({ block: 'nearest' })
  }
</script>

<div class="combobox" class:disabled>
  <div class="combobox-input-wrap">
    <input
      {id}
      type="text"
      class="combobox-input"
      value={displayValue}
      {placeholder}
      {disabled}
      oninput={(e) => { query = (e.target as HTMLInputElement).value; if (!open) openDropdown() }}
      onfocus={openDropdown}
      onblur={closeDropdown}
      onkeydown={handleKeydown}
      bind:this={inputEl}
      role="combobox"
      aria-expanded={open}
      aria-autocomplete="list"
      aria-controls={listId}
      autocomplete="off"
    />
    {#if value && !disabled}
      <button class="combobox-clear" onclick={clearSelection} type="button" title="Clear selection">&times;</button>
    {/if}
  </div>

  {#if open}
    <ul class="combobox-list" id={listId} bind:this={listEl} role="listbox">
      {#if filtered.length === 0}
        <li class="combobox-empty">No matches found</li>
      {/if}
      {#each filtered as org, i (org.id)}
        <li
          class="combobox-option"
          class:highlighted={i === highlightIndex}
          class:selected={org.id === value}
          role="option"
          aria-selected={org.id === value}
          onmousedown={() => selectOrg(org.id)}
          onmouseenter={() => { highlightIndex = i }}
        >
          <span class="option-name">{org.name}</span>
          {#if org.tags && org.tags.length > 0}
            <span class="option-tags">
              {#each org.tags as tag}
                <span class="option-tag">{tag}</span>
              {/each}
            </span>
          {/if}
        </li>
      {/each}
      {#if oncreate}
        <li
          class="combobox-option combobox-create"
          class:highlighted={highlightIndex === filtered.length}
          role="option"
          onmousedown={() => { oncreate?.(); open = false }}
          onmouseenter={() => { highlightIndex = filtered.length }}
        >
          + Create New Organization
        </li>
      {/if}
    </ul>
  {/if}
</div>

<style>
  .combobox {
    position: relative;
    width: 100%;
  }

  .combobox.disabled {
    opacity: 0.6;
    pointer-events: none;
  }

  .combobox-input-wrap {
    display: flex;
    align-items: center;
    position: relative;
  }

  .combobox-input {
    width: 100%;
    padding: 0.4rem 1.6rem 0.4rem 0.6rem;
    border: 1px solid #d1d5db;
    border-radius: 5px;
    font-size: 0.82rem;
    color: #374151;
    background: #fff;
    font-family: inherit;
  }

  .combobox-input:focus {
    outline: none;
    border-color: #6c63ff;
    box-shadow: 0 0 0 2px rgba(108, 99, 255, 0.15);
  }

  .combobox-clear {
    position: absolute;
    right: 0.3rem;
    background: none;
    border: none;
    font-size: 1rem;
    color: #9ca3af;
    cursor: pointer;
    padding: 0.1rem 0.3rem;
    line-height: 1;
  }

  .combobox-clear:hover {
    color: #374151;
  }

  .combobox-list {
    position: absolute;
    z-index: 50;
    top: 100%;
    left: 0;
    right: 0;
    max-height: 200px;
    overflow-y: auto;
    background: #fff;
    border: 1px solid #d1d5db;
    border-top: none;
    border-radius: 0 0 5px 5px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
    list-style: none;
    padding: 0;
    margin: 0;
  }

  .combobox-option {
    padding: 0.4rem 0.6rem;
    cursor: pointer;
    font-size: 0.8rem;
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }

  .combobox-option:hover,
  .combobox-option.highlighted {
    background: #f0f0ff;
  }

  .combobox-option.selected {
    background: #e0e7ff;
    font-weight: 500;
  }

  .combobox-empty {
    padding: 0.6rem;
    font-size: 0.78rem;
    color: #9ca3af;
    font-style: italic;
    text-align: center;
  }

  .combobox-create {
    border-top: 1px solid #e5e7eb;
    color: #6c63ff;
    font-weight: 500;
  }

  .combobox-create:hover,
  .combobox-create.highlighted {
    background: #f0f0ff;
  }

  .option-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .option-tags {
    display: flex;
    gap: 0.15rem;
    flex-shrink: 0;
  }

  .option-tag {
    padding: 0.05em 0.25em;
    background: #e0e7ff;
    color: #3730a3;
    border-radius: 3px;
    font-size: 0.55rem;
    font-weight: 500;
  }
</style>
```

**Acceptance criteria:**
- Typing into the input filters organizations by name (case-insensitive substring match).
- Selecting an org sets the `value` bindable to the org's ID and displays the org name.
- Pressing Escape closes the dropdown. Arrow keys navigate options.
- The clear button (x) resets value to null.
- Tag pills render next to each org name in the dropdown.
- The "Create New" option appears at the bottom when `oncreate` is provided.
- The component is keyboard-accessible (ArrowDown/Up/Enter/Escape).

**Failure criteria:**
- The dropdown must close after selection (no stale open state).
- `onblur` must not fire before `onmousedown` on an option (the 150ms delay handles this).

---

### Task 39.6 — Replace Education `<select>` with OrgCombobox

**File:** `packages/webui/src/lib/components/SourcesView.svelte`

Replace all three education org `<select>` elements (degree form, certificate form, course form) with the `OrgCombobox` component.

**Step 6a — Add import** (at top of `<script>`, after existing imports):

```typescript
  import OrgCombobox from '$lib/components/OrgCombobox.svelte'
```

**Step 6b — Replace degree form org select** (around line 846). Replace:

```svelte
                <div class="org-select-row">
                  <select id="edu-org" bind:value={formEduOrgId}>
                    <option value={null}>-- Select --</option>
                    {#each eduFilteredOrgs as org}
                      <option value={org.id}>{org.name}</option>
                    {/each}
                  </select>
                  <button class="btn-new-sm" onclick={openOrgModal} type="button">+</button>
                </div>
```

With:

```svelte
                <div class="org-select-row">
                  <OrgCombobox
                    id="edu-org"
                    organizations={eduFilteredOrgs}
                    bind:value={formEduOrgId}
                    placeholder="Search institutions..."
                    oncreate={openOrgModal}
                  />
                </div>
```

**Step 6c — Replace certificate form org select** (around line 912). Replace:

```svelte
              <div class="org-select-row">
                <select id="edu-org" bind:value={formEduOrgId}>
                  <option value={null}>-- Select --</option>
                  {#each eduFilteredOrgs as org}
                    <option value={org.id}>{org.name}</option>
                  {/each}
                </select>
                <button class="btn-new-sm" onclick={openOrgModal} type="button">+</button>
              </div>
```

With:

```svelte
              <div class="org-select-row">
                <OrgCombobox
                  id="edu-org"
                  organizations={eduFilteredOrgs}
                  bind:value={formEduOrgId}
                  placeholder="Search issuers..."
                  oncreate={openOrgModal}
                />
              </div>
```

**Step 6d — Replace course form org select** (around line 945). Replace:

```svelte
                <div class="org-select-row">
                  <select id="edu-org" bind:value={formEduOrgId}>
                    <option value={null}>-- Select --</option>
                    {#each eduFilteredOrgs as org}
                      <option value={org.id}>{org.name}</option>
                    {/each}
                  </select>
                  <button class="btn-new-sm" onclick={openOrgModal} type="button">+</button>
                </div>
```

With:

```svelte
                <div class="org-select-row">
                  <OrgCombobox
                    id="edu-org"
                    organizations={eduFilteredOrgs}
                    bind:value={formEduOrgId}
                    placeholder="Search providers..."
                    oncreate={openOrgModal}
                  />
                </div>
```

**Step 6e — Remove the standalone `+` button** from each replaced block. The OrgCombobox handles the "Create New" action via its `oncreate` prop, so the separate `<button class="btn-new-sm" onclick={openOrgModal}>+</button>` is no longer needed. Verify the `btn-new-sm` button is gone from all three replaced blocks.

**Acceptance criteria:**
- All three education org dropdowns (degree, certificate, course) use OrgCombobox.
- Typing into any of the three filters the `eduFilteredOrgs` list.
- Selecting an org sets `formEduOrgId` and triggers the campus loading effect.
- The "Create New Organization" option in the dropdown opens the quick-create org modal.
- The `+` button is gone — the combobox handles it.

**Failure criteria:**
- Switching education type must still reset `formEduOrgId` to null (existing `onEducationTypeChange` handles this).

**Note:** There is a second copy of SourcesView at `packages/webui/src/routes/data/sources/SourcesView.svelte`. This file is a legacy copy — the canonical version is at `packages/webui/src/lib/components/SourcesView.svelte`. Either delete the legacy copy and update any imports, or apply the same OrgCombobox changes to both files.

---

### Task 39.7 — Campus in IR Compiler

**PREREQUISITE:** Task 39.8 (type changes) must be completed before this task — the compiler code references `campus_name`, `campus_city`, `campus_state` fields that are defined in 39.8.

**File:** `packages/core/src/services/resume-compiler.ts`

**Step 7a — Update the SQL query in `buildEducationItems`** (starting at line 313). Add a LEFT JOIN on `org_campuses` and select campus fields:

Replace the existing query (lines 313-338) with:

```typescript
  const rows = db
    .query(
      `SELECT
        re.id AS entry_id,
        re.content AS entry_content,
        p.content AS perspective_content,
        bs.source_id,
        se.education_type,
        COALESCE(o.name, se.institution) AS institution,
        se.field,
        se.end_date,
        se.degree_level,
        se.degree_type,
        se.gpa,
        se.location,
        se.credential_id,
        COALESCE(o.name, se.issuing_body) AS issuing_body,
        se.certificate_subtype,
        se.edu_description,
        se.organization_id,
        oc.name AS campus_name,
        oc.city AS campus_city,
        oc.state AS campus_state
      FROM resume_entries re
      JOIN perspectives p ON p.id = re.perspective_id
      JOIN bullet_sources bs ON bs.bullet_id = p.bullet_id AND bs.is_primary = 1
      JOIN sources s ON s.id = bs.source_id
      LEFT JOIN source_education se ON se.source_id = s.id
      LEFT JOIN organizations o ON o.id = se.organization_id
      LEFT JOIN org_campuses oc ON oc.id = se.campus_id
      WHERE re.section_id = ?
      ORDER BY re.position ASC`
    )
    .all(sectionId) as Array<{
      entry_id: string
      entry_content: string | null
      perspective_content: string
      source_id: string
      education_type: string | null
      institution: string | null
      field: string | null
      end_date: string | null
      degree_level: string | null
      degree_type: string | null
      gpa: string | null
      location: string | null
      credential_id: string | null
      issuing_body: string | null
      certificate_subtype: string | null
      edu_description: string | null
      organization_id: string | null
      campus_name: string | null
      campus_city: string | null
      campus_state: string | null
    }>
```

**Step 7b — Update the row mapping** (lines 361-378). Add campus fields to the returned `EducationItem`:

Replace the existing mapping with:

```typescript
  return rows.map(row => ({
    kind: 'education' as const,
    institution: row.institution ?? 'Unknown',
    degree: row.entry_content ?? row.perspective_content,
    date: row.end_date ? new Date(row.end_date).getFullYear().toString() : '',
    entry_id: row.entry_id,
    source_id: row.source_id,
    education_type: row.education_type ?? undefined,
    degree_level: row.degree_level,
    degree_type: row.degree_type,
    field: row.field ?? null,
    gpa: row.gpa,
    location: row.location,
    credential_id: row.credential_id,
    issuing_body: row.issuing_body,
    certificate_subtype: row.certificate_subtype,
    edu_description: row.edu_description,
    campus_name: row.campus_name ?? null,
    campus_city: row.campus_city ?? null,
    campus_state: row.campus_state ?? null,
  }))
```

**Acceptance criteria:**
- The query executes without error (the LEFT JOIN produces NULLs when no campus is linked).
- Education items with a `campus_id` include `campus_name`, `campus_city`, `campus_state` in the IR.
- Education items without a `campus_id` have all three campus fields as `null`.

**Failure criteria:**
- The LEFT JOIN must not filter out education entries that have no campus (use LEFT JOIN, not INNER JOIN).

---

### Task 39.8 — Add Campus Fields to `EducationItem` Type

**File:** `packages/core/src/types/index.ts`

Add three new optional fields to the `EducationItem` interface (after `edu_description`, before the closing `}`):

```typescript
  campus_name?: string | null
  campus_city?: string | null
  campus_state?: string | null
```

The updated interface (lines 906-927):

```typescript
export interface EducationItem {
  kind: 'education'
  institution: string
  degree: string
  date: string
  entry_id: string | null
  source_id: string | null
  // New optional fields from education sub-types
  education_type?: string
  degree_level?: string | null
  degree_type?: string | null
  field?: string | null
  gpa?: string | null
  location?: string | null
  credential_id?: string | null
  issuing_body?: string | null
  certificate_subtype?: string | null
  edu_description?: string | null
  // Campus fields from org_campuses JOIN
  campus_name?: string | null
  campus_city?: string | null
  campus_state?: string | null
}
```

**Step 8b — Mirror fields onto SDK `EducationItem` in `packages/sdk/src/types.ts`.** Add the same three fields to the SDK's `EducationItem` interface:

```typescript
  campus_name?: string | null
  campus_city?: string | null
  campus_state?: string | null
```

**Acceptance criteria:**
- TypeScript compiles with the new fields.
- Existing code that constructs `EducationItem` objects without campus fields still compiles (fields are optional).
- The SDK's `EducationItem` in `packages/sdk/src/types.ts` includes `campus_name`, `campus_city`, `campus_state`.

---

### Task 39.9 — Use Campus Location in LaTeX Template

**File:** `packages/core/src/templates/sb2nov.ts`

Update `renderEducationSection` to prefer campus city/state over the deprecated `location` field.

**Step 9a — Update degree rendering** (line 273, the default `else` branch). Replace:

```typescript
      const location = item.location ?? ''
```

With:

```typescript
      // Prefer campus city/state; fall back to deprecated source_education.location
      const location = (item.campus_city && item.campus_state)
        ? `${item.campus_city}, ${item.campus_state}`
        : item.campus_city ?? item.campus_state ?? item.location ?? ''
```

**Step 9b — Update course rendering** (line 266, the `eduType === 'course'` branch). Replace:

```typescript
      const locPart = item.location ? `, ${item.location}` : ''
```

With:

```typescript
      // Prefer campus location over deprecated source_education.location
      const campusLoc = (item.campus_city && item.campus_state)
        ? `${item.campus_city}, ${item.campus_state}`
        : item.campus_city ?? item.campus_state ?? null
      const locPart = campusLoc ? `, ${campusLoc}` : (item.location ? `, ${item.location}` : '')
```

**Acceptance criteria:**
- A degree entry with `campus_city='Salt Lake City'` and `campus_state='UT'` renders `{Salt Lake City, UT}` in the location field.
- A degree entry with only `campus_city='Arlington'` (no state) renders `{Arlington}`.
- A degree entry with no campus but `location='Online'` falls back to `{Online}`.
- A degree entry with neither campus nor location renders `{}` (empty).
- Course entries follow the same fallback logic.

**Failure criteria:**
- Certificate entries are not affected (they do not use a location field in the template).

---

## Testing Support

### Unit Tests

**Campus repository update (Task 39.1):**
- Test `update()` with partial fields — verify only specified fields change.
- Test `update()` with nonexistent ID — returns null.
- Test `update()` with empty input — returns existing campus unchanged.
- Test `update()` setting a field to null (e.g., `city: null`).

These tests belong in `packages/core/src/db/repositories/__tests__/campus-repository.test.ts` (create if not exists). Follow the pattern in `source-repository.test.ts`.

Minimal test file skeleton:

```typescript
import { describe, it, expect, beforeEach } from 'bun:test'
import { Database } from 'bun:sqlite'
import * as CampusRepo from '../campus-repository'
import * as OrgRepo from '../organization-repository'
import { runMigrations } from '../../migrate'

describe('campus-repository', () => {
  let db: Database

  beforeEach(() => {
    db = new Database(':memory:')
    runMigrations(db)
  })

  describe('update()', () => {
    it('partially updates a campus', () => {
      const org = OrgRepo.create(db, { name: 'Test Org' })
      const campus = CampusRepo.create(db, { organization_id: org.id, name: 'Main', modality: 'in_person' })
      const updated = CampusRepo.update(db, campus.id, { name: 'Renamed' })
      expect(updated?.name).toBe('Renamed')
      expect(updated?.modality).toBe('in_person')
    })

    it('returns null for nonexistent ID', () => {
      expect(CampusRepo.update(db, 'no-such-id', { name: 'X' })).toBeNull()
    })

    it('returns existing campus unchanged for empty input', () => {
      const org = OrgRepo.create(db, { name: 'Test Org' })
      const campus = CampusRepo.create(db, { organization_id: org.id, name: 'Main', modality: 'in_person' })
      const same = CampusRepo.update(db, campus.id, {})
      expect(same).toEqual(campus)
    })

    it('sets a field to null', () => {
      const org = OrgRepo.create(db, { name: 'Test Org' })
      const campus = CampusRepo.create(db, { organization_id: org.id, name: 'Main', modality: 'in_person', city: 'Portland' })
      const updated = CampusRepo.update(db, campus.id, { city: null })
      expect(updated?.city).toBeNull()
    })
  })
})
```

**Resume compiler campus JOIN (Task 39.7):**
- Add compiler integration tests to `packages/core/src/services/__tests__/resume-compiler.test.ts` (this file already exists).
- Add a test case that creates a source_education with a campus_id pointing to a campus with city/state.
- Verify the resulting `EducationItem` includes `campus_city` and `campus_state`.
- Verify an education item without a campus_id has null campus fields.

### Integration Tests

**PATCH /campuses/:id (Task 39.2):**
- `PATCH` with valid fields returns 200 and updated campus.
- `PATCH` with nonexistent ID returns 404.
- `PATCH` with partial body (only name) updates name, leaves other fields unchanged.
- `PATCH` with `is_headquarters: true` stores as integer 1.

### Component Smoke Tests

**OrgCombobox (Task 39.5):**
- Manual: open dropdown, type a partial name, verify filtering.
- Manual: select an org, verify the input shows the org name.
- Manual: press Escape to close, ArrowDown/Up to navigate, Enter to select.
- Manual: click "Create New Organization", verify callback fires.

**Inline campus edit (Task 39.3):**
- Manual: click a campus card, verify edit form appears with pre-populated fields.
- Manual: change name, click Save, verify card updates.
- Manual: click Cancel, verify no changes.

**OrgCombobox in SourcesView (Task 39.6):**
- Manual: navigate to education form (degree/cert/course), verify searchable dropdown works.
- Manual: type partial name, select org, verify campus dropdown loads.
- Manual: click "Create New", verify org modal opens.

---

## Documentation Requirements

No documentation files to create. After implementation, prepend '**Done in Phase 39.**' to the opening line of Part B items 2, 3, 5, and 7 in the spec. The `EducationItem` type changes are self-documenting via the added comment block.

---

## Parallelization Notes

All four work items are independent at the code level and can be developed in parallel by separate agents:

| Task group | Files touched | Can parallel with |
|-----------|--------------|-------------------|
| Tasks 39.1 + 39.2 (campus PATCH) | campus-repository.ts, campuses.ts | All others |
| Task 39.3 (inline campus edit UI) | +page.svelte (org editor) | Tasks 39.5-39.6 (different file) |
| Task 39.4 (remove stale payload) | +page.svelte (org editor) | Should be done WITH 39.3 (same file) |
| Tasks 39.5 + 39.6 (OrgCombobox) | OrgCombobox.svelte (new), SourcesView.svelte | Tasks 39.1-39.4 |
| Tasks 39.7 + 39.8 + 39.9 (compiler) | resume-compiler.ts, types/index.ts, sb2nov.ts | All others |

**Constraint:** Tasks 39.3 and 39.4 both edit `+page.svelte` and should be done in the same session to avoid merge conflicts. Tasks 39.7, 39.8, and 39.9 form a dependency chain (type change -> compiler -> template) and must be done in order.

**Recommended execution order:**
1. Tasks 39.1 + 39.2 (backend, no deps)
2. Tasks 39.5 + 39.6 (OrgCombobox, no deps on backend)
3. Tasks 39.3 + 39.4 (org editor page, no deps on 39.5)
4. Tasks 39.7 + 39.8 + 39.9 (compiler chain, no deps on UI)
