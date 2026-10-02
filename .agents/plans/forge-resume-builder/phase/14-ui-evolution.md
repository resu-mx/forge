# Phase 14: UI Evolution

**Goal:** Evolve the WebUI views to reflect the schema evolution: polymorphic sources, many-to-many bullets, resume entries with copy-on-write, and new entity views (organizations, skills, notes, logs, archetypes).

**Non-Goals:** Pixel-perfect design. Graph visualization (Phase 15). Drag-and-drop reordering of resume entries (deferred — positional update via buttons is sufficient for MVP).

**Depends on:** Phase 12 (API/SDK must be ready with new endpoints and types)
**Blocks:** Phase 15 (Chain Graph + Integration Polish)

**Dependency note:** T14.1 (navigation update) can start after Phase 10. All other tasks (T14.2-T14.10) require Phase 12 (SDK types must exist for TypeScript compilation).

**Reference:** `docs/superpowers/specs/2026-03-29-forge-schema-evolution-design.md` section 2

**Conventions (all views):**
- Svelte 5 syntax: `$state`, `$effect`, `$derived`, `$props`
- Import shared components from `$lib/components` (`StatusBadge`, `LoadingSpinner`, `EmptyState`, `ConfirmDialog`, `ToastContainer`)
- Use `forge` SDK client from `$lib/sdk`
- Use `addToast` from `$lib/stores/toast.svelte` for notifications
- Use `friendlyError` from `$lib/sdk` for API error messages
- Types imported from `@forge/sdk`

---

## Task 14.1: Update Navigation

**Goal:** Update the sidebar navigation to reflect the new entity-centric view structure.

**File:** `packages/webui/src/routes/+layout.svelte`

**Changes:**

Replace the current `navItems` array:

```typescript
// Before
const navItems = [
  { href: '/', label: 'Review Queue' },
  { href: '/sources', label: 'Sources' },
  { href: '/derivation', label: 'Derivation' },
  { href: '/chain', label: 'Chain View' },
  { href: '/resumes', label: 'Resumes' },
]

// After
const navItems = [
  { href: '/', label: 'Dashboard' },
  { href: '/sources', label: 'Sources' },
  { href: '/bullets', label: 'Bullets' },
  { href: '/resumes', label: 'Resumes' },
  { href: '/organizations', label: 'Organizations' },
  { href: '/skills', label: 'Skills' },
  { href: '/archetypes', label: 'Archetypes' },
  { href: '/chain', label: 'Chain View' },
  { href: '/logs', label: 'Logs' },
  { href: '/notes', label: 'Notes' },
]
```

**Implementation notes:**
- Remove "Derivation" entry (merged into Bullets view workflow)
- Rename "Review Queue" to "Dashboard" (it now includes integrity alerts)
- Use `page.url.pathname.startsWith(item.href)` for active state on nested routes (e.g. `/sources/new` highlights "Sources"), but keep exact match for `/` to avoid highlighting Dashboard for all routes

**Active state logic update:**

```svelte
<a
  href={item.href}
  class:active={item.href === '/'
    ? page.url.pathname === '/'
    : page.url.pathname.startsWith(item.href)}
>
```

**Acceptance Criteria:**
- [ ] Navigation shows all 10 items in the specified order
- [ ] "Derivation" no longer appears in navigation
- [ ] Active state highlights correctly for each route including nested paths
- [ ] Layout still renders `ToastContainer` and `{@render children()}`

**Testing:**
- Smoke: All nav links render and navigate to correct routes
- Visual: Active state highlights on each page

---

## Task 14.2: Sources View (Tabbed)

**Goal:** Rewrite the Sources view with tabs for filtering by `source_type`, and an adaptive editor form that shows type-specific fields.

**File:** `packages/webui/src/routes/sources/+page.svelte`

**State:**

```typescript
import type { Source } from '@forge/sdk'

const SOURCE_TABS = [
  { value: 'all', label: 'All' },
  { value: 'role', label: 'Roles' },
  { value: 'project', label: 'Projects' },
  { value: 'education', label: 'Education' },
  { value: 'clearance', label: 'Clearances' },
  { value: 'general', label: 'General' },
]

let sources = $state<Source[]>([])
let activeTab = $state('all')
let selectedId = $state<string | null>(null)
let loading = $state(true)
let editing = $state(false)
let saving = $state(false)
let deriving = $state(false)
let confirmDeleteOpen = $state(false)

// Base form fields (all source types)
let formTitle = $state('')
let formDescription = $state('')
let formSourceType = $state<string>('general')
let formNotes = $state('')

// Role extension fields
let formOrgId = $state<string | null>(null)
let formStartDate = $state('')
let formEndDate = $state('')
let formIsCurrent = $state(false)
let formWorkArrangement = $state('')
let formBaseSalary = $state<number | null>(null)
let formTotalCompNotes = $state('')

// Education extension fields
let formEducationType = $state('certificate')
let formInstitution = $state('')
let formField = $state('')
let formIsInProgress = $state(false)
let formCredentialId = $state('')
let formExpirationDate = $state('')
let formIssuingBody = $state('')
let formUrl = $state('')

// Project extension fields
let formIsPersonal = $state(false)
let formProjectUrl = $state('')

// Clearance extension fields
let formLevel = $state('')
let formPolygraph = $state('')
let formClearanceStatus = $state('')
let formSponsoringAgency = $state('')

let filteredSources = $derived(
  activeTab === 'all'
    ? sources
    : sources.filter(s => s.source_type === activeTab)
)

let selectedSource = $derived(sources.find(s => s.id === selectedId) ?? null)
```

**Data loading:**

```typescript
async function loadSources() {
  loading = true
  const params: Record<string, string> = {}
  // Load all sources; client-side tab filtering
  const result = await forge.sources.list({ limit: 500 })
  if (result.ok) {
    sources = result.data
  } else {
    addToast({ message: friendlyError(result.error, 'Failed to load sources'), type: 'error' })
  }
  loading = false
}
```

**Editor form — adaptive fields:**

The editor renders different field groups based on `formSourceType`:

```svelte
<!-- Base fields (always shown) -->
<div class="form-group">
  <label for="source-type">Type</label>
  <select id="source-type" bind:value={formSourceType} disabled={!editing}>
    <option value="general">General</option>
    <option value="role">Role</option>
    <option value="project">Project</option>
    <option value="education">Education</option>
    <option value="clearance">Clearance</option>
  </select>
</div>

<div class="form-group">
  <label for="source-title">Title <span class="required">*</span></label>
  <input id="source-title" type="text" bind:value={formTitle} placeholder="..." />
</div>

<div class="form-group">
  <label for="source-description">Description <span class="required">*</span></label>
  <textarea id="source-description" bind:value={formDescription} rows="6"></textarea>
</div>

<!-- Role-specific fields -->
{#if formSourceType === 'role'}
  <div class="form-group">
    <label for="role-org">Organization</label>
    <select id="role-org" bind:value={formOrgId}>
      <option value={null}>None</option>
      {#each organizations as org}
        <option value={org.id}>{org.name}</option>
      {/each}
    </select>
  </div>
  <div class="form-row">
    <div class="form-group">
      <label for="role-start">Start Date</label>
      <input id="role-start" type="date" bind:value={formStartDate} />
    </div>
    <div class="form-group">
      <label for="role-end">End Date</label>
      <input id="role-end" type="date" bind:value={formEndDate} />
    </div>
  </div>
  <div class="form-group">
    <label>
      <input type="checkbox" bind:checked={formIsCurrent} /> Currently employed
    </label>
  </div>
  <div class="form-group">
    <label for="role-arrangement">Work Arrangement</label>
    <input id="role-arrangement" type="text" bind:value={formWorkArrangement}
           placeholder="e.g. remote, hybrid, on-site" />
  </div>
{/if}

<!-- Education-specific fields -->
{#if formSourceType === 'education'}
  <div class="form-group">
    <label for="edu-type">Education Type</label>
    <select id="edu-type" bind:value={formEducationType}>
      <option value="degree">Degree</option>
      <option value="certificate">Certificate</option>
      <option value="course">Course</option>
      <option value="self_taught">Self-Taught</option>
    </select>
  </div>
  <div class="form-group">
    <label for="edu-institution">Institution</label>
    <input id="edu-institution" type="text" bind:value={formInstitution} />
  </div>
  <div class="form-group">
    <label for="edu-field">Field</label>
    <input id="edu-field" type="text" bind:value={formField} />
  </div>
  <div class="form-row">
    <div class="form-group">
      <label for="edu-start">Start Date</label>
      <input id="edu-start" type="date" bind:value={formStartDate} />
    </div>
    <div class="form-group">
      <label for="edu-end">End Date</label>
      <input id="edu-end" type="date" bind:value={formEndDate} />
    </div>
  </div>
  <div class="form-group">
    <label>
      <input type="checkbox" bind:checked={formIsInProgress} /> In progress
    </label>
  </div>
  <div class="form-group">
    <label for="edu-credential">Credential ID</label>
    <input id="edu-credential" type="text" bind:value={formCredentialId} />
  </div>
  <div class="form-group">
    <label for="edu-issuer">Issuing Body</label>
    <input id="edu-issuer" type="text" bind:value={formIssuingBody} />
  </div>
  <div class="form-group">
    <label for="edu-url">URL</label>
    <input id="edu-url" type="url" bind:value={formUrl} />
  </div>
{/if}

<!-- Project-specific fields -->
{#if formSourceType === 'project'}
  <div class="form-group">
    <label for="proj-org">Organization</label>
    <select id="proj-org" bind:value={formOrgId}>
      <option value={null}>None</option>
      {#each organizations as org}
        <option value={org.id}>{org.name}</option>
      {/each}
    </select>
  </div>
  <div class="form-group">
    <label>
      <input type="checkbox" bind:checked={formIsPersonal} /> Personal project
    </label>
  </div>
  <div class="form-group">
    <label for="proj-url">URL</label>
    <input id="proj-url" type="url" bind:value={formProjectUrl} />
  </div>
  <div class="form-row">
    <div class="form-group">
      <label for="proj-start">Start Date</label>
      <input id="proj-start" type="date" bind:value={formStartDate} />
    </div>
    <div class="form-group">
      <label for="proj-end">End Date</label>
      <input id="proj-end" type="date" bind:value={formEndDate} />
    </div>
  </div>
{/if}

<!-- Clearance-specific fields -->
{#if formSourceType === 'clearance'}
  <div class="form-group">
    <label for="clr-level">Level <span class="required">*</span></label>
    <input id="clr-level" type="text" bind:value={formLevel} placeholder="e.g. TS/SCI" />
  </div>
  <div class="form-group">
    <label for="clr-polygraph">Polygraph</label>
    <input id="clr-polygraph" type="text" bind:value={formPolygraph} placeholder="e.g. CI, Full Scope" />
  </div>
  <div class="form-group">
    <label for="clr-status">Status</label>
    <input id="clr-status" type="text" bind:value={formClearanceStatus} placeholder="e.g. active, inactive" />
  </div>
  <div class="form-group">
    <label for="clr-agency">Sponsoring Agency</label>
    <input id="clr-agency" type="text" bind:value={formSponsoringAgency} />
  </div>
{/if}

<!-- Notes (all types) -->
<div class="form-group">
  <label for="source-notes">Notes</label>
  <textarea id="source-notes" bind:value={formNotes} rows="3"
            placeholder="Internal notes about this source..."></textarea>
</div>
```

**Save logic:**

```typescript
async function saveSource() {
  if (!formTitle.trim() || !formDescription.trim()) {
    addToast({ message: 'Title and description are required.', type: 'error' })
    return
  }

  saving = true
  const basePayload = {
    title: formTitle.trim(),
    description: formDescription.trim(),
    source_type: formSourceType,
    notes: formNotes || undefined,
  }

  // Build extension payload based on source_type
  let extensionPayload: Record<string, unknown> = {}
  if (formSourceType === 'role') {
    extensionPayload = {
      role: {
        organization_id: formOrgId,
        start_date: formStartDate || undefined,
        end_date: formEndDate || undefined,
        is_current: formIsCurrent,
        work_arrangement: formWorkArrangement || undefined,
        base_salary: formBaseSalary,
        total_comp_notes: formTotalCompNotes || undefined,
      }
    }
  } else if (formSourceType === 'education') {
    extensionPayload = {
      education: {
        education_type: formEducationType,
        institution: formInstitution || undefined,
        field: formField || undefined,
        start_date: formStartDate || undefined,
        end_date: formEndDate || undefined,
        is_in_progress: formIsInProgress,
        credential_id: formCredentialId || undefined,
        expiration_date: formExpirationDate || undefined,
        issuing_body: formIssuingBody || undefined,
        url: formUrl || undefined,
      }
    }
  } else if (formSourceType === 'project') {
    extensionPayload = {
      project: {
        organization_id: formOrgId,
        is_personal: formIsPersonal,
        url: formProjectUrl || undefined,
        start_date: formStartDate || undefined,
        end_date: formEndDate || undefined,
      }
    }
  } else if (formSourceType === 'clearance') {
    extensionPayload = {
      clearance: {
        level: formLevel,
        polygraph: formPolygraph || undefined,
        status: formClearanceStatus || undefined,
        sponsoring_agency: formSponsoringAgency || undefined,
      }
    }
  }

  const payload = { ...basePayload, ...extensionPayload }

  if (editing) {
    const result = await forge.sources.create(payload)
    if (result.ok) {
      sources = [...sources, result.data]
      selectedId = result.data.id
      editing = false
      addToast({ message: 'Source created.', type: 'success' })
    } else {
      addToast({ message: friendlyError(result.error, 'Failed to create source'), type: 'error' })
    }
  } else if (selectedId) {
    const result = await forge.sources.update(selectedId, payload)
    if (result.ok) {
      sources = sources.map(s => s.id === selectedId ? result.data : s)
      addToast({ message: 'Source updated.', type: 'success' })
    } else {
      addToast({ message: friendlyError(result.error, 'Failed to update source'), type: 'error' })
    }
  }
  saving = false
}
```

**Populate form from selected source:**

```typescript
function populateFormFromSource(source: Source) {
  formTitle = source.title
  formDescription = source.description
  formSourceType = source.source_type
  formNotes = source.notes ?? ''
  // Reset all extension fields
  formOrgId = null
  formStartDate = ''
  formEndDate = ''
  formIsCurrent = false
  formWorkArrangement = ''
  formBaseSalary = null
  formTotalCompNotes = ''
  formEducationType = 'certificate'
  formInstitution = ''
  formField = ''
  formIsInProgress = false
  formCredentialId = ''
  formExpirationDate = ''
  formIssuingBody = ''
  formUrl = ''
  formIsPersonal = false
  formProjectUrl = ''
  formLevel = ''
  formPolygraph = ''
  formClearanceStatus = ''
  formSponsoringAgency = ''

  // Populate extension fields based on type
  if (source.source_type === 'role' && source.role) {
    formOrgId = source.role.organization_id ?? null
    formStartDate = source.role.start_date ?? ''
    formEndDate = source.role.end_date ?? ''
    formIsCurrent = !!source.role.is_current
    formWorkArrangement = source.role.work_arrangement ?? ''
    formBaseSalary = source.role.base_salary ?? null
    formTotalCompNotes = source.role.total_comp_notes ?? ''
  } else if (source.source_type === 'education' && source.education) {
    formEducationType = source.education.education_type
    formInstitution = source.education.institution ?? ''
    formField = source.education.field ?? ''
    formStartDate = source.education.start_date ?? ''
    formEndDate = source.education.end_date ?? ''
    formIsInProgress = !!source.education.is_in_progress
    formCredentialId = source.education.credential_id ?? ''
    formExpirationDate = source.education.expiration_date ?? ''
    formIssuingBody = source.education.issuing_body ?? ''
    formUrl = source.education.url ?? ''
  } else if (source.source_type === 'project' && source.project) {
    formOrgId = source.project.organization_id ?? null
    formIsPersonal = !!source.project.is_personal
    formProjectUrl = source.project.url ?? ''
    formStartDate = source.project.start_date ?? ''
    formEndDate = source.project.end_date ?? ''
  } else if (source.source_type === 'clearance' && source.clearance) {
    formLevel = source.clearance.level
    formPolygraph = source.clearance.polygraph ?? ''
    formClearanceStatus = source.clearance.status ?? ''
    formSponsoringAgency = source.clearance.sponsoring_agency ?? ''
  }
}
```

**List card — show type-specific metadata:**

```svelte
<button
  class="source-card"
  class:selected={selectedId === source.id}
  onclick={() => selectSource(source.id)}
>
  <div class="card-top">
    <span class="card-title">{source.title}</span>
    <StatusBadge status={source.status} />
  </div>
  <div class="card-meta">
    <span class="type-badge type-{source.source_type}">{source.source_type}</span>
    {#if source.source_type === 'role' && source.role?.organization_id}
      <span class="card-org">{getOrgName(source.role.organization_id)}</span>
    {/if}
    {#if source.source_type === 'education' && source.education?.institution}
      <span class="card-org">{source.education.institution}</span>
    {/if}
  </div>
  <span class="card-date">{formatDate(source.updated_at)}</span>
</button>
```

**Tab bar (replaces status filter):**

```svelte
<div class="filter-tabs">
  {#each SOURCE_TABS as tab}
    <button
      class="filter-tab"
      class:active={activeTab === tab.value}
      onclick={() => activeTab = tab.value}
    >
      {tab.label}
      <span class="tab-count">
        {tab.value === 'all'
          ? sources.length
          : sources.filter(s => s.source_type === tab.value).length}
      </span>
    </button>
  {/each}
</div>
```

**Organizations preload:** On mount, also load organizations for the dropdown in role/project forms:

```typescript
let organizations = $state<Organization[]>([])

$effect(() => {
  loadSources()
  loadOrganizations()
})

async function loadOrganizations() {
  const result = await forge.organizations.list({ limit: 500 })
  if (result.ok) {
    organizations = result.data
  }
}

function getOrgName(id: string): string {
  return organizations.find(o => o.id === id)?.name ?? 'Unknown'
}
```

**Acceptance Criteria:**
- [ ] Tab bar shows All, Roles, Projects, Education, Clearances, General
- [ ] Tab counts update as sources are added/removed
- [ ] Selecting a tab filters the list by `source_type`
- [ ] Editor form adapts to show type-specific fields (role, education, project, clearance)
- [ ] Source type selector disabled when editing existing source (type is immutable after creation)
- [ ] Role form shows organization dropdown, date range, is_current, work arrangement
- [ ] Education form shows education_type, institution, field, dates, credential, issuer, URL
- [ ] Project form shows organization, is_personal, URL, dates
- [ ] Clearance form shows level, polygraph, status, sponsoring agency
- [ ] Create and update work with all source types and extensions
- [ ] Derive Bullets button still works
- [ ] Notes field visible and editable for all source types
- [ ] Delete works with confirmation dialog

**Testing:**
- Component: Tab filtering renders correct sources per tab
- Component: Form adapts fields when source type changes
- Component: Role form shows organization dropdown populated from API
- E2E: Create role source with organization, verify extension data round-trips

---

## Task 14.3: Bullets View (Unified Content Atoms)

**Goal:** Replace the three-column Derivation view with a unified list of content atoms (bullets, perspectives, resume entries) with type filtering and inline actions.

**File:** Create `packages/webui/src/routes/bullets/+page.svelte` (new route)

**Also:** Remove or redirect `packages/webui/src/routes/derivation/+page.svelte` (keep for backward compat with redirect, or delete)

**State:**

```typescript
import type { Bullet, Perspective } from '@forge/sdk'

type ContentType = 'bullet' | 'perspective' | 'resume_entry'

let contentType = $state<ContentType>('bullet')
let items = $state<any[]>([])
let loading = $state(true)
let searchQuery = $state('')
let statusFilter = $state('all')

// Derive perspective modal (carried from derivation view)
let deriveModal = $state({
  open: false,
  bulletId: '',
  archetype: 'agentic-ai',
  domain: 'ai_ml',
  framing: 'accomplishment' as 'accomplishment' | 'responsibility' | 'context',
  submitting: false,
})

// Reject modal
let rejectModal = $state<{
  open: boolean
  type: ContentType
  id: string
  reason: string
}>({ open: false, type: 'bullet', id: '', reason: '' })

let filteredItems = $derived.by(() => {
  let result = items
  if (statusFilter !== 'all') {
    result = result.filter((i: any) => i.status === statusFilter)
  }
  if (searchQuery.trim()) {
    const q = searchQuery.toLowerCase()
    result = result.filter((i: any) => i.content.toLowerCase().includes(q))
  }
  return result
})
```

**Data loading — switches by content type:**

```typescript
$effect(() => {
  loadItems()
})

async function loadItems() {
  loading = true
  items = []

  if (contentType === 'bullet') {
    const result = await forge.bullets.list({ limit: 500 })
    if (result.ok) items = result.data
    else addToast({ message: friendlyError(result.error, 'Failed to load bullets'), type: 'error' })
  } else if (contentType === 'perspective') {
    const result = await forge.perspectives.list({ limit: 500 })
    if (result.ok) items = result.data
    else addToast({ message: friendlyError(result.error, 'Failed to load perspectives'), type: 'error' })
  } else if (contentType === 'resume_entry') {
    // Resume entries are fetched per-resume; for unified view, may need a dedicated endpoint
    // or iterate resumes. For MVP, show a message to select a resume.
    items = []
  }

  loading = false
}
```

**Template — unified list with type-aware rendering:**

```svelte
<div class="bullets-page">
  <h1 class="page-title">Content Atoms</h1>
  <p class="subtitle">Unified view of bullets, perspectives, and resume entries</p>

  <!-- Type filter -->
  <div class="controls">
    <div class="type-tabs">
      {#each [
        { value: 'bullet', label: 'Bullets' },
        { value: 'perspective', label: 'Perspectives' },
        { value: 'resume_entry', label: 'Resume Entries' },
      ] as tab}
        <button
          class="type-tab"
          class:active={contentType === tab.value}
          onclick={() => { contentType = tab.value; }}
        >
          {tab.label}
        </button>
      {/each}
    </div>

    <input
      type="text"
      class="search-input"
      placeholder="Search content..."
      bind:value={searchQuery}
    />

    <select class="status-select" bind:value={statusFilter}>
      <option value="all">All statuses</option>
      <option value="pending_review">Pending</option>
      <option value="approved">Approved</option>
      <option value="rejected">Rejected</option>
    </select>
  </div>

  <!-- Item list -->
  {#if loading}
    <LoadingSpinner message="Loading {contentType}s..." />
  {:else if filteredItems.length === 0}
    <EmptyState
      title="No {contentType}s found"
      description={searchQuery ? 'Try adjusting your search.' : 'No items match the current filters.'}
    />
  {:else}
    <div class="item-list">
      {#each filteredItems as item (item.id)}
        <div class="item-card">
          <div class="item-header">
            <p class="item-content">{truncate(item.content, 200)}</p>
            <StatusBadge status={item.status} />
          </div>

          <!-- Bullet-specific metadata -->
          {#if contentType === 'bullet'}
            {#if item.sources?.length > 0}
              <div class="meta-row">
                <span class="meta-label">Sources:</span>
                {#each item.sources as src}
                  <span class="source-tag" class:primary={src.is_primary}>
                    {src.title}
                  </span>
                {/each}
              </div>
            {/if}
            {#if item.technologies?.length > 0}
              <div class="tech-tags">
                {#each item.technologies as tech}
                  <span class="tech-tag">{tech}</span>
                {/each}
              </div>
            {/if}
          {/if}

          <!-- Perspective-specific metadata -->
          {#if contentType === 'perspective'}
            <div class="perspective-meta">
              {#if item.target_archetype}
                <span class="meta-tag archetype">{item.target_archetype}</span>
              {/if}
              {#if item.domain}
                <span class="meta-tag domain">{item.domain}</span>
              {/if}
              <span class="meta-tag framing">{item.framing}</span>
            </div>
          {/if}

          {#if item.rejection_reason}
            <p class="rejection-reason">Reason: {item.rejection_reason}</p>
          {/if}

          <!-- Inline actions -->
          <div class="item-actions">
            {#if item.status === 'pending_review'}
              <button class="btn btn-approve" onclick={() => approveItem(item.id)}>Approve</button>
              <button class="btn btn-reject" onclick={() => openReject(item.id)}>Reject</button>
            {/if}
            {#if item.status === 'rejected'}
              <button class="btn btn-reopen" onclick={() => reopenItem(item.id)}>Reopen</button>
            {/if}
            {#if contentType === 'bullet' && item.status === 'approved'}
              <button class="btn btn-derive" onclick={() => openDeriveModal(item.id)}>
                Derive Perspective
              </button>
            {/if}
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>
```

**Action handlers:** Port approve/reject/reopen/derive logic from the derivation view, adjusted for the unified model. Each action calls the appropriate SDK method based on `contentType`.

```typescript
async function approveItem(id: string) {
  if (contentType === 'bullet') {
    const res = await forge.bullets.approve(id)
    if (res.ok) {
      items = items.map(i => i.id === id ? res.data : i)
      addToast({ message: 'Bullet approved', type: 'success' })
    } else {
      addToast({ message: `Approve failed: ${res.error.message}`, type: 'error' })
    }
  } else if (contentType === 'perspective') {
    const res = await forge.perspectives.approve(id)
    if (res.ok) {
      items = items.map(i => i.id === id ? res.data : i)
      addToast({ message: 'Perspective approved', type: 'success' })
    } else {
      addToast({ message: `Approve failed: ${res.error.message}`, type: 'error' })
    }
  }
}
```

**Acceptance Criteria:**
- [ ] Type tabs switch between bullets, perspectives, resume entries
- [ ] Content search filters items by text content
- [ ] Status filter works across all content types
- [ ] Bullet cards show sources array (with primary indicator), technologies
- [ ] Perspective cards show archetype, domain, framing tags
- [ ] Approve/reject/reopen actions work inline with optimistic updates
- [ ] Derive Perspective button opens modal on approved bullets
- [ ] Reject modal requires non-empty reason
- [ ] Items reload when content type changes
- [ ] Old `/derivation` route redirects or is removed from navigation

**Testing:**
- Component: Type tabs switch data source
- Component: Search filtering works across content
- Component: Approve/reject optimistic updates
- E2E: Approve bullet, switch to perspectives, verify derived perspective appears

---

## Task 14.4: Organizations View

**Goal:** Create a new view for CRUD management of organizations.

**File:** Create `packages/webui/src/routes/organizations/+page.svelte`

**Layout:** Two-panel (list + editor), matching the Sources view pattern.

**State:**

```typescript
import type { Organization } from '@forge/sdk'

const ORG_TYPES = ['company', 'nonprofit', 'government', 'military', 'education', 'volunteer', 'freelance', 'other']
const EMPLOYMENT_TYPES = ['civilian', 'contractor', 'military_active', 'military_reserve', 'volunteer', 'intern']

let organizations = $state<Organization[]>([])
let selectedId = $state<string | null>(null)
let typeFilter = $state('all')
let workedFilter = $state<'all' | 'worked' | 'not_worked'>('all')
let loading = $state(true)
let editing = $state(false)
let saving = $state(false)
let confirmDeleteOpen = $state(false)

// Form fields
let formName = $state('')
let formOrgType = $state('company')
let formIndustry = $state('')
let formSize = $state('')
let formWorked = $state(false)
let formEmploymentType = $state('')
let formLocation = $state('')
let formHeadquarters = $state('')
let formWebsite = $state('')
let formLinkedinUrl = $state('')
let formGlassdoorUrl = $state('')
let formGlassdoorRating = $state<number | null>(null)
let formReputationNotes = $state('')
let formNotes = $state('')

let filteredOrgs = $derived.by(() => {
  let result = organizations
  if (typeFilter !== 'all') {
    result = result.filter(o => o.org_type === typeFilter)
  }
  if (workedFilter === 'worked') {
    result = result.filter(o => o.worked)
  } else if (workedFilter === 'not_worked') {
    result = result.filter(o => !o.worked)
  }
  return result
})

let selectedOrg = $derived(organizations.find(o => o.id === selectedId) ?? null)
```

**List card:**

```svelte
<button
  class="org-card"
  class:selected={selectedId === org.id}
  onclick={() => selectOrg(org.id)}
>
  <div class="card-top">
    <span class="card-title">{org.name}</span>
    <span class="type-badge type-{org.org_type}">{org.org_type}</span>
  </div>
  <div class="card-meta">
    {#if org.industry}
      <span class="meta-item">{org.industry}</span>
    {/if}
    {#if org.location}
      <span class="meta-item">{org.location}</span>
    {/if}
    {#if org.worked}
      <span class="worked-badge">Worked</span>
    {/if}
  </div>
</button>
```

**Editor form:**

```svelte
<div class="form-group">
  <label for="org-name">Name <span class="required">*</span></label>
  <input id="org-name" type="text" bind:value={formName} placeholder="e.g. Anthropic" />
</div>

<div class="form-row">
  <div class="form-group">
    <label for="org-type">Type</label>
    <select id="org-type" bind:value={formOrgType}>
      {#each ORG_TYPES as t}
        <option value={t}>{t}</option>
      {/each}
    </select>
  </div>
  <div class="form-group">
    <label for="org-industry">Industry</label>
    <input id="org-industry" type="text" bind:value={formIndustry} placeholder="e.g. AI Safety" />
  </div>
</div>

<div class="form-group">
  <label>
    <input type="checkbox" bind:checked={formWorked} /> I have worked here
  </label>
</div>

{#if formWorked}
  <div class="form-group">
    <label for="org-employment">Employment Type</label>
    <select id="org-employment" bind:value={formEmploymentType}>
      <option value="">Not specified</option>
      {#each EMPLOYMENT_TYPES as t}
        <option value={t}>{t.replace('_', ' ')}</option>
      {/each}
    </select>
  </div>
{/if}

<div class="form-row">
  <div class="form-group">
    <label for="org-location">Location</label>
    <input id="org-location" type="text" bind:value={formLocation} />
  </div>
  <div class="form-group">
    <label for="org-hq">Headquarters</label>
    <input id="org-hq" type="text" bind:value={formHeadquarters} />
  </div>
</div>

<div class="form-group">
  <label for="org-website">Website</label>
  <input id="org-website" type="url" bind:value={formWebsite} />
</div>

<div class="form-row">
  <div class="form-group">
    <label for="org-linkedin">LinkedIn URL</label>
    <input id="org-linkedin" type="url" bind:value={formLinkedinUrl} />
  </div>
  <div class="form-group">
    <label for="org-glassdoor">Glassdoor URL</label>
    <input id="org-glassdoor" type="url" bind:value={formGlassdoorUrl} />
  </div>
</div>

<div class="form-group">
  <label for="org-rating">Glassdoor Rating</label>
  <input id="org-rating" type="number" step="0.1" min="0" max="5" bind:value={formGlassdoorRating} />
</div>

<div class="form-group">
  <label for="org-reputation">Reputation Notes</label>
  <textarea id="org-reputation" bind:value={formReputationNotes} rows="3"></textarea>
</div>

<div class="form-group">
  <label for="org-notes">Notes</label>
  <textarea id="org-notes" bind:value={formNotes} rows="3"></textarea>
</div>
```

**CRUD handlers:** Follow the same pattern as Sources view (`saveOrg`, `deleteOrg`, `startNew`, `selectOrg`).

**Acceptance Criteria:**
- [ ] List shows all organizations with name, type badge, industry, location, worked status
- [ ] Filter by org_type dropdown
- [ ] Filter by worked/not worked toggle
- [ ] Create organization with all fields
- [ ] Update organization
- [ ] Delete organization with confirmation
- [ ] Employment type field only visible when "worked" is checked
- [ ] Form validates name is required

**Testing:**
- Component: Organization list renders with filters
- Component: Form conditionally shows employment type
- E2E: Create organization, verify in list, update, delete

---

## Task 14.5: Skills View

**Goal:** Create a new view for CRUD management of skills.

**File:** Create `packages/webui/src/routes/skills/+page.svelte`

**Layout:** Two-panel (list + editor), simplified version of Organizations view.

**State:**

```typescript
import type { Skill } from '@forge/sdk'

const CATEGORIES = [
  'ai_ml', 'cloud', 'database', 'devops', 'frameworks',
  'general', 'language', 'languages', 'os', 'security', 'tools',
]

let skills = $state<Skill[]>([])
let selectedId = $state<string | null>(null)
let categoryFilter = $state('all')
let searchQuery = $state('')
let loading = $state(true)
let editing = $state(false)
let saving = $state(false)
let confirmDeleteOpen = $state(false)

let formName = $state('')
let formCategory = $state('general')
let formNotes = $state('')

let filteredSkills = $derived.by(() => {
  let result = skills
  if (categoryFilter !== 'all') {
    result = result.filter(s => s.category === categoryFilter)
  }
  if (searchQuery.trim()) {
    const q = searchQuery.toLowerCase()
    result = result.filter(s => s.name.toLowerCase().includes(q))
  }
  return result
})
```

**List card:**

```svelte
<button
  class="skill-card"
  class:selected={selectedId === skill.id}
  onclick={() => selectSkill(skill.id)}
>
  <span class="card-title">{skill.name}</span>
  <span class="category-badge">{skill.category}</span>
</button>
```

**Editor form:**

```svelte
<div class="form-group">
  <label for="skill-name">Name <span class="required">*</span></label>
  <input id="skill-name" type="text" bind:value={formName} placeholder="e.g. Kubernetes" />
</div>

<div class="form-group">
  <label for="skill-category">Category</label>
  <select id="skill-category" bind:value={formCategory}>
    {#each CATEGORIES as cat}
      <option value={cat}>{cat.replace('_', ' ')}</option>
    {/each}
  </select>
</div>

<div class="form-group">
  <label for="skill-notes">Notes</label>
  <textarea id="skill-notes" bind:value={formNotes} rows="3"
            placeholder="Proficiency level, years of experience, context..."></textarea>
</div>
```

**Acceptance Criteria:**
- [ ] List shows all skills with name and category badge
- [ ] Filter by category dropdown
- [ ] Search by name
- [ ] Create, update, delete skills
- [ ] Form validates name is required
- [ ] Categories match v1 data categories

**Testing:**
- Component: Skill list renders with category filter
- Component: Search filters by name
- E2E: Create skill, filter by category, delete

---

## Task 14.6: Notes View

**Goal:** Create a view for managing user notes with cross-entity linking.

**File:** Create `packages/webui/src/routes/notes/+page.svelte`

**Layout:** Two-panel (list + editor).

**State:**

```typescript
import type { UserNote, NoteReference } from '@forge/sdk'

const ENTITY_TYPES = ['source', 'bullet', 'perspective', 'resume_entry', 'resume', 'skill', 'organization']

let notes = $state<UserNote[]>([])
let selectedId = $state<string | null>(null)
let searchQuery = $state('')
let loading = $state(true)
let editing = $state(false)
let saving = $state(false)
let confirmDeleteOpen = $state(false)

let formTitle = $state('')
let formContent = $state('')

// Reference linking
let showRefPicker = $state(false)
let refEntityType = $state('source')
let refEntityId = $state('')

let filteredNotes = $derived.by(() => {
  if (!searchQuery.trim()) return notes
  const q = searchQuery.toLowerCase()
  return notes.filter(n =>
    n.content.toLowerCase().includes(q) ||
    (n.title && n.title.toLowerCase().includes(q))
  )
})

let selectedNote = $derived(notes.find(n => n.id === selectedId) ?? null)
```

**Template — note list with entity tags:**

```svelte
<!-- Note list card -->
<button
  class="note-card"
  class:selected={selectedId === note.id}
  onclick={() => selectNote(note.id)}
>
  <div class="card-top">
    <span class="card-title">{note.title || 'Untitled'}</span>
    <span class="card-date">{formatDate(note.updated_at)}</span>
  </div>
  <p class="card-preview">{truncate(note.content, 100)}</p>
  {#if note.references?.length > 0}
    <div class="ref-tags">
      {#each note.references as ref}
        <span class="ref-tag ref-{ref.entity_type}">
          {ref.entity_type}
        </span>
      {/each}
    </div>
  {/if}
</button>
```

**Editor — with reference management:**

```svelte
<div class="form-group">
  <label for="note-title">Title</label>
  <input id="note-title" type="text" bind:value={formTitle} placeholder="Optional title" />
</div>

<div class="form-group">
  <label for="note-content">Content <span class="required">*</span></label>
  <textarea id="note-content" bind:value={formContent} rows="10"
            placeholder="Write your note..."></textarea>
</div>

<!-- Linked entities -->
{#if selectedNote}
  <div class="references-section">
    <h4>Linked Entities</h4>
    {#if selectedNote.references?.length === 0}
      <p class="no-refs">No linked entities.</p>
    {:else}
      <div class="ref-list">
        {#each selectedNote.references as ref}
          <div class="ref-item">
            <span class="ref-tag ref-{ref.entity_type}">{ref.entity_type}</span>
            <span class="ref-id">{ref.entity_id.slice(0, 8)}...</span>
            <button class="btn btn-sm btn-danger" onclick={() => unlinkEntity(ref)}>
              Unlink
            </button>
          </div>
        {/each}
      </div>
    {/if}
    <button class="btn btn-sm btn-add" onclick={() => showRefPicker = true}>
      + Link Entity
    </button>
  </div>
{/if}
```

**Link entity handler:**

```typescript
async function linkEntity() {
  if (!selectedId || !refEntityId.trim()) return
  const result = await forge.notes.addReference(selectedId, {
    entity_type: refEntityType,
    entity_id: refEntityId.trim(),
  })
  if (result.ok) {
    addToast({ message: 'Entity linked', type: 'success' })
    await loadNoteDetail(selectedId)
    showRefPicker = false
    refEntityId = ''
  } else {
    addToast({ message: friendlyError(result.error, 'Failed to link entity'), type: 'error' })
  }
}

async function unlinkEntity(ref: NoteReference) {
  if (!selectedId) return
  const result = await forge.notes.removeReference(selectedId, ref.entity_type, ref.entity_id)
  if (result.ok) {
    addToast({ message: 'Entity unlinked', type: 'success' })
    await loadNoteDetail(selectedId)
  } else {
    addToast({ message: friendlyError(result.error, 'Failed to unlink entity'), type: 'error' })
  }
}
```

**Acceptance Criteria:**
- [ ] List shows all notes with title, preview, and entity type tags
- [ ] Search filters by title and content
- [ ] Create note with title and content
- [ ] Update note
- [ ] Delete note with confirmation
- [ ] View linked entities on selected note
- [ ] Link new entity to note (entity_type + entity_id)
- [ ] Unlink entity from note
- [ ] Entity type tags use distinct colors per type

**Testing:**
- Component: Note list renders with search filter
- Component: Reference tags render for each linked entity
- E2E: Create note, link to source entity, verify bidirectional search

---

## Task 14.7: Updated Resume Builder

**Goal:** Rewrite the resume builder to use `resume_entries` with copy-on-write semantics instead of direct perspective references.

**File:** `packages/webui/src/routes/resumes/+page.svelte`

**Key changes from current implementation:**

1. **Types:** `ResumeWithPerspectives` becomes `ResumeWithEntries`. Sections contain `ResumeEntry[]` instead of `Perspective[]`.
2. **Display content:** Each entry shows `entry.content ?? perspective.content` (copy-on-write: NULL content = reference mode, shows perspective content; non-NULL = clone mode, shows entry content).
3. **Entry editing:** Click to edit an entry. Editing populates `content` field (switches from reference to clone mode).
4. **Reset to reference:** A "Reset to original" button sets `content` back to `null` via `PATCH /resumes/:id/entries/:eid` with `{ content: null }`.
5. **Add entry:** The picker now creates a `resume_entry` via `POST /resumes/:id/entries` instead of `POST /resumes/:id/perspectives`.

**State changes:**

```typescript
import type { Resume, ResumeWithEntries, ResumeEntry, Perspective, GapAnalysis } from '@forge/sdk'

let resumeDetail = $state<ResumeWithEntries | null>(null)
// ...

// Entry editing
let editingEntryId = $state<string | null>(null)
let entryEditContent = $state('')
let entryEditSaving = $state(false)
```

**Entry card with copy-on-write:**

```svelte
{#each entries.sort((a, b) => a.position - b.position) as entry (entry.id)}
  <div class="entry-card" class:cloned={entry.content !== null}>
    {#if editingEntryId === entry.id}
      <!-- Inline edit mode -->
      <textarea
        class="entry-edit-textarea"
        bind:value={entryEditContent}
        rows="4"
      ></textarea>
      <div class="entry-edit-actions">
        <button class="btn btn-sm btn-primary" onclick={() => saveEntryEdit(entry.id)}
                disabled={entryEditSaving}>
          {entryEditSaving ? 'Saving...' : 'Save'}
        </button>
        <button class="btn btn-sm btn-ghost" onclick={() => editingEntryId = null}>
          Cancel
        </button>
      </div>
    {:else}
      <div class="entry-content" ondblclick={() => startEntryEdit(entry)}>
        {entry.content ?? entry.perspective_content}
      </div>

      <div class="entry-meta">
        {#if entry.content !== null}
          <span class="cow-badge cloned">Edited</span>
          <button class="btn btn-xs btn-ghost" onclick={() => resetEntry(entry.id)}
                  title="Reset to original perspective content">
            Reset to original
          </button>
        {:else}
          <span class="cow-badge reference">Reference</span>
        {/if}
      </div>

      <div class="entry-actions">
        <button class="btn btn-sm btn-ghost" onclick={() => startEntryEdit(entry)}>
          Edit
        </button>
        <button class="btn btn-sm btn-danger" onclick={() => removeEntry(entry.id)}>
          Remove
        </button>
      </div>
    {/if}
  </div>
{/each}
```

**Copy-on-write handlers:**

```typescript
function startEntryEdit(entry: ResumeEntry) {
  editingEntryId = entry.id
  // Start with current display content
  entryEditContent = entry.content ?? entry.perspective_content ?? ''
}

async function saveEntryEdit(entryId: string) {
  if (!selectedResumeId) return
  entryEditSaving = true

  const result = await forge.resumes.updateEntry(selectedResumeId, entryId, {
    content: entryEditContent,
  })
  if (result.ok) {
    addToast({ message: 'Entry updated (cloned)', type: 'success' })
    editingEntryId = null
    await loadResumeDetail(selectedResumeId)
  } else {
    addToast({ message: friendlyError(result.error, 'Failed to update entry'), type: 'error' })
  }

  entryEditSaving = false
}

async function resetEntry(entryId: string) {
  if (!selectedResumeId) return

  const result = await forge.resumes.updateEntry(selectedResumeId, entryId, {
    content: null,  // explicit null = reset to reference mode
  })
  if (result.ok) {
    addToast({ message: 'Entry reset to reference mode', type: 'success' })
    await loadResumeDetail(selectedResumeId)
  } else {
    addToast({ message: friendlyError(result.error, 'Failed to reset entry'), type: 'error' })
  }
}

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
  if (result.ok) {
    addToast({ message: 'Entry added', type: 'success' })
    await loadResumeDetail(selectedResumeId!)
    await loadGapAnalysis(selectedResumeId!)
  } else {
    addToast({ message: friendlyError(result.error, 'Failed to add entry'), type: 'error' })
  }
}

async function removeEntry(entryId: string) {
  if (!selectedResumeId) return
  const result = await forge.resumes.removeEntry(selectedResumeId, entryId)
  if (result.ok) {
    addToast({ message: 'Entry removed', type: 'success' })
    await loadResumeDetail(selectedResumeId!)
    await loadGapAnalysis(selectedResumeId!)
  } else {
    addToast({ message: friendlyError(result.error, 'Failed to remove entry'), type: 'error' })
  }
}
```

**Acceptance Criteria:**
- [ ] Resume builder shows entries grouped by section
- [ ] Reference mode entries display perspective content with "Reference" badge
- [ ] Cloned entries display entry content with "Edited" badge
- [ ] Double-click or Edit button opens inline editor for entry content
- [ ] Saving entry edit switches from reference to clone mode
- [ ] "Reset to original" sets content to null (reference mode)
- [ ] Add entry via perspective picker creates resume_entry
- [ ] Remove entry works
- [ ] Gap analysis panel still works with new data model
- [ ] Entry position ordering is maintained

**Testing:**
- Component: Entry card renders reference vs cloned states correctly
- Component: Inline edit mode toggles properly
- E2E: Add perspective as entry, edit (clone), reset to reference, verify content switches

---

## Task 14.8: Dashboard Updates

**Goal:** Add "Integrity Alerts" card to the dashboard showing drifted entity count.

**File:** `packages/webui/src/routes/+page.svelte`

**Changes:**

Add new state for integrity data:

```typescript
let driftCount = $state(0)
let driftLoading = $state(false)
```

Add integrity fetch to `loadData`:

```typescript
async function loadData() {
  loading = true
  error = null
  try {
    const [review, sources, bullets, perspectives, drift] = await Promise.all([
      forge.review.pending(),
      forge.sources.list({ limit: 1 }),
      forge.bullets.list({ limit: 1 }),
      forge.perspectives.list({ limit: 1 }),
      forge.integrity.drift(),
    ])

    // ... existing logic for review, sources, bullets, perspectives ...

    if (drift.ok) {
      driftCount = drift.data.length
    }
  } catch (e) {
    error = e instanceof Error ? e.message : 'Failed to load dashboard data'
  } finally {
    loading = false
  }
}
```

Add Integrity Alerts card after Pending Review section:

```svelte
<!-- Integrity Alerts -->
<section class="section">
  <h2 class="section-title">Integrity Alerts</h2>

  {#if driftCount === 0}
    <div class="integrity-ok">
      <p>All snapshots are current. No drift detected.</p>
    </div>
  {:else}
    <a href="/chain" class="alert-card">
      <div class="alert-count">{driftCount}</div>
      <div class="alert-label">Drifted Entities</div>
      <div class="alert-hint">Snapshots no longer match current content. Click to view in Chain View.</div>
    </a>
  {/if}
</section>
```

**Styles for alert card:**

```css
.integrity-ok {
  background: #f0fdf4;
  border: 1px solid #bbf7d0;
  border-radius: 8px;
  padding: 1.25rem 1.5rem;
  color: #166534;
  font-size: 0.9rem;
}

.alert-card {
  display: block;
  background: #fff;
  border: 1px solid #e5e7eb;
  border-left: 4px solid #ef4444;
  border-radius: 8px;
  padding: 1.5rem;
  text-decoration: none;
  color: inherit;
  transition: box-shadow 0.15s;
  cursor: pointer;
}

.alert-card:hover {
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
}

.alert-count {
  font-size: 2.25rem;
  font-weight: 700;
  color: #ef4444;
  line-height: 1;
  margin-bottom: 0.25rem;
}

.alert-label {
  font-size: 0.95rem;
  font-weight: 600;
  color: #374151;
  margin-bottom: 0.5rem;
}

.alert-hint {
  font-size: 0.75rem;
  color: #9ca3af;
}
```

Also update Quick Stats to include organizations, skills, and resumes counts:

```typescript
let totalOrganizations = $state(0)
let totalSkills = $state(0)
let totalResumes = $state(0)

// In loadData, add:
const [orgs, skillsList, resumesList] = await Promise.all([
  forge.organizations.list({ limit: 1 }),
  forge.skills.list({ limit: 1 }),
  forge.resumes.list({ limit: 1 }),
])

if (orgs.ok) totalOrganizations = orgs.pagination.total
if (skillsList.ok) totalSkills = skillsList.pagination.total
if (resumesList.ok) totalResumes = resumesList.pagination.total
```

**Acceptance Criteria:**
- [ ] Dashboard shows "Integrity Alerts" section
- [ ] When no drift: green "all clear" message
- [ ] When drift exists: red alert card with count, links to Chain View
- [ ] Quick Stats section shows all entity counts (sources, bullets, perspectives, organizations, skills, resumes)
- [ ] Dashboard title changed from "Review Queue" to "Dashboard"
- [ ] Pending review cards link to `/bullets` instead of `/derivation`

**Testing:**
- Component: Integrity alert renders with 0 drift
- Component: Integrity alert renders with N drifted entities
- Smoke: Dashboard loads all stats without errors

---

## Task 14.9: Logs View

**Goal:** Create a view to browse prompt logs (AI derivation audit trail).

**File:** Create `packages/webui/src/routes/logs/+page.svelte`

**Layout:** Single-panel table view.

**State:**

```typescript
let logs = $state<any[]>([])
let loading = $state(true)
let selectedLog = $state<any | null>(null)
```

**Data loading:**

```typescript
async function loadLogs() {
  loading = true
  // Use review.pending() or a dedicated logs endpoint if available
  // The prompt_logs table is exposed via the API
  const result = await forge.promptLogs.list({ limit: 100 })
  if (result.ok) {
    logs = result.data
  } else {
    addToast({ message: friendlyError(result.error, 'Failed to load logs'), type: 'error' })
  }
  loading = false
}
```

**Template:**

```svelte
<div class="logs-page">
  <h1 class="page-title">Prompt Logs</h1>
  <p class="subtitle">AI derivation audit trail</p>

  {#if loading}
    <LoadingSpinner message="Loading logs..." />
  {:else if logs.length === 0}
    <EmptyState title="No logs" description="Prompt logs will appear here after AI derivation runs." />
  {:else}
    <div class="logs-table-wrapper">
      <table class="logs-table">
        <thead>
          <tr>
            <th>Timestamp</th>
            <th>Template</th>
            <th>Entity Type</th>
            <th>Entity ID</th>
            <th>Actions</th>
          </tr>
        </thead>
        <tbody>
          {#each logs as log (log.id)}
            <tr class:selected={selectedLog?.id === log.id}>
              <td class="timestamp">{formatDateTime(log.created_at)}</td>
              <td>{log.template_name ?? 'unknown'}</td>
              <td>
                <span class="entity-type-badge">{log.entity_type ?? '-'}</span>
              </td>
              <td class="entity-link">
                {#if log.entity_id}
                  <span class="mono">{log.entity_id.slice(0, 8)}...</span>
                {:else}
                  -
                {/if}
              </td>
              <td>
                <button class="btn btn-sm btn-ghost" onclick={() => selectedLog = log}>
                  View
                </button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>

    <!-- Detail panel -->
    {#if selectedLog}
      <div class="log-detail">
        <div class="detail-header">
          <h3>Log Detail</h3>
          <button class="btn btn-sm btn-ghost" onclick={() => selectedLog = null}>Close</button>
        </div>
        <div class="detail-section">
          <h4>Input</h4>
          <pre class="detail-pre">{selectedLog.input_text ?? '(none)'}</pre>
        </div>
        <div class="detail-section">
          <h4>Response</h4>
          <pre class="detail-pre">{selectedLog.response_text ?? '(none)'}</pre>
        </div>
      </div>
    {/if}
  {/if}
</div>
```

**Helper:**

```typescript
function formatDateTime(dateStr: string): string {
  return new Date(dateStr).toLocaleString('en-US', {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  })
}
```

**Acceptance Criteria:**
- [ ] Table shows prompt logs with timestamp, template, entity type, entity ID
- [ ] Click "View" to see full input and response in detail panel
- [ ] Loading spinner while fetching
- [ ] Empty state when no logs exist
- [ ] Entity ID shown as truncated monospace with link potential

**Testing:**
- Component: Log table renders with mock data
- Component: Detail panel shows input/response
- Smoke: Page loads without errors

---

## Task 14.10: Archetypes View

**Goal:** Create a read-only view showing archetype definitions and their domain mappings.

**File:** Create `packages/webui/src/routes/archetypes/+page.svelte`

**Layout:** Card grid (no editor panel needed — read-only for MVP).

**Implementation:** Archetypes are defined as constants in the codebase (not database entities). This view renders them from a hardcoded list or from a constants endpoint.

```typescript
const ARCHETYPES = [
  {
    id: 'agentic-ai',
    label: 'Agentic AI',
    description: 'AI/ML engineering with focus on agentic systems, LLMs, and autonomous agents.',
    domains: ['ai_ml', 'software_engineering', 'systems_engineering'],
  },
  {
    id: 'infrastructure',
    label: 'Infrastructure',
    description: 'Cloud infrastructure, platform engineering, and systems reliability.',
    domains: ['systems_engineering', 'devops', 'software_engineering'],
  },
  {
    id: 'security-engineer',
    label: 'Security Engineer',
    description: 'Application and infrastructure security, threat modeling, and secure development.',
    domains: ['security', 'systems_engineering', 'software_engineering'],
  },
  {
    id: 'solutions-architect',
    label: 'Solutions Architect',
    description: 'Technical solution design, system integration, and client-facing architecture.',
    domains: ['systems_engineering', 'software_engineering', 'leadership'],
  },
  {
    id: 'public-sector',
    label: 'Public Sector',
    description: 'Government and military focused roles requiring clearance and compliance knowledge.',
    domains: ['security', 'systems_engineering', 'leadership'],
  },
  {
    id: 'hft',
    label: 'HFT',
    description: 'High-frequency trading infrastructure, low-latency systems, and financial technology.',
    domains: ['systems_engineering', 'software_engineering', 'devops'],
  },
]
```

**Template:**

```svelte
<div class="archetypes-page">
  <h1 class="page-title">Archetypes</h1>
  <p class="subtitle">Resume targeting profiles and their domain mappings</p>

  <div class="archetype-grid">
    {#each ARCHETYPES as archetype}
      <div class="archetype-card">
        <h3 class="archetype-name">{archetype.label}</h3>
        <p class="archetype-description">{archetype.description}</p>
        <div class="domain-list">
          <span class="domain-label">Domains:</span>
          {#each archetype.domains as domain}
            <span class="domain-tag">{domain.replace('_', ' ')}</span>
          {/each}
        </div>
      </div>
    {/each}
  </div>

  <div class="read-only-note">
    <p>Archetype definitions are currently read-only. Editing archetypes is a future feature.</p>
  </div>
</div>
```

**Styles:**

```css
.archetype-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: 1rem;
  margin-bottom: 2rem;
}

.archetype-card {
  background: #fff;
  border: 1px solid #e5e7eb;
  border-radius: 8px;
  padding: 1.25rem;
}

.archetype-name {
  font-size: 1.1rem;
  font-weight: 600;
  color: #1a1a2e;
  margin-bottom: 0.5rem;
}

.archetype-description {
  font-size: 0.85rem;
  color: #4b5563;
  line-height: 1.5;
  margin-bottom: 0.75rem;
}

.domain-list {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.35rem;
}

.domain-label {
  font-size: 0.75rem;
  font-weight: 600;
  color: #6b7280;
  text-transform: uppercase;
  letter-spacing: 0.03em;
}

.domain-tag {
  display: inline-block;
  padding: 0.15em 0.5em;
  background: #d1fae5;
  color: #065f46;
  border-radius: 4px;
  font-size: 0.7rem;
  font-weight: 500;
}

.read-only-note {
  background: #fffbeb;
  border: 1px solid #fde68a;
  border-radius: 8px;
  padding: 1rem 1.25rem;
  font-size: 0.85rem;
  color: #92400e;
}
```

**Acceptance Criteria:**
- [ ] Card grid shows all 6 archetypes
- [ ] Each card shows name, description, and domain tags
- [ ] Domain tags use green color scheme consistent with perspective domain tags elsewhere
- [ ] Read-only notice is visible
- [ ] No edit/delete functionality (MVP scope)

**Testing:**
- Component: All 6 archetype cards render
- Component: Domain tags render for each archetype
- Visual: Grid layout responsive on different screen widths

---

## Parallelization

T14.1 can start after Phase 10 (only touches navigation, no SDK type imports). All other tasks (T14.2-T14.10) require Phase 12 to be complete because they import SDK types (`Source`, `Bullet`, `Organization`, `ResumeEntry`, `UserNote`, `DriftReport`, etc.) that must exist for TypeScript compilation.

Once Phase 12 is complete, T14.2-T14.10 can all proceed in parallel since they are independent views:

```
Phase 10 ──> Task 14.1 (navigation) ──┐
                                       │
Phase 12 ──────────────────────────────┼──> Task 14.2 (sources tabbed)
                                       ├──> Task 14.3 (bullets unified)
                                       ├──> Task 14.4 (organizations)
                                       ├──> Task 14.5 (skills)
                                       ├──> Task 14.6 (notes)
                                       ├──> Task 14.7 (resume builder)
                                       ├──> Task 14.8 (dashboard)
                                       ├──> Task 14.9 (logs)
                                       └──> Task 14.10 (archetypes)
```

**Priority order (if sequential):**
1. T14.1 (navigation) — unblocks everything
2. T14.2 (sources) — most complex, core workflow
3. T14.3 (bullets) — replaces derivation, core workflow
4. T14.7 (resume builder) — core workflow, copy-on-write is new
5. T14.4 (organizations) — referenced by sources
6. T14.8 (dashboard) — integrity alerts are a key new feature
7. T14.5 (skills) — simple CRUD
8. T14.6 (notes) — entity linking is novel
9. T14.9 (logs) — read-only, low complexity
10. T14.10 (archetypes) — static data, lowest complexity

## Documentation

- `docs/src/webui/views.md` — update with new view descriptions
- `docs/src/webui/components.md` — add any new shared components (DriftBanner in Phase 15)
