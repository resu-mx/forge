# Phase 18: Organization Updates + Personal Projects

**Goal:** Add status lifecycle to organizations and handle personal projects cleanly in the Sources view. This is the simplest renderer-adjacent phase -- mostly UI updates with minor repo/service/SDK plumbing.

**Non-Goals:** Organization status enforcement in the database (the UI guides the flow, no CHECK constraint on transitions). Adding org status to the Chain View graph (Phase 15 scope if ever).

**Depends on:** Phase 16 (migration adds `status` column to `organizations`)
**Blocks:** Nothing
**Parallelizable with:** Phase 17, Phase 19

**Shared file warning:** Phase 18 (T18.3) and Phase 17 (T17.6) both modify `packages/sdk/src/types.ts`. If running in parallel, coordinate or merge carefully.

**Tech Stack:** TypeScript, SQLite (via `bun:sqlite`), Svelte 5, `@forge/sdk`, `bun:test`

**Reference:** `refs/specs/2026-03-29-resume-renderer-and-entity-updates.md` section 3

**Conventions:**
- Svelte 5 syntax: `$state`, `$effect`, `$derived`
- Import shared components from `$lib/components` (`StatusBadge`, `LoadingSpinner`, `EmptyState`, `ConfirmDialog`)
- Use `forge` SDK client from `$lib/sdk`
- Use `addToast` from `$lib/stores/toast.svelte` for notifications
- Types imported from `@forge/sdk`

---

## Context

Phase 16 adds a `status` column to the `organizations` table:

```sql
ALTER TABLE organizations ADD COLUMN status TEXT CHECK (status IN (
  'interested', 'review', 'targeting', 'excluded', NULL
));
```

After Phase 16 the column exists but nothing reads or writes it. This phase wires it through from repo to UI.

For personal projects, the `source_projects.is_personal` column and `source_projects.organization_id` already exist from Phase 10 (migration 002). The Sources view already has a "Personal project" checkbox. This phase improves the UX: when `is_personal` is checked, the org dropdown should hide, and the Projects tab should show a "Personal" label.

---

## Tasks

### Task 18.1: Update OrganizationRepository + Service

**Files:**
- Modify: `packages/core/src/db/repositories/organization-repository.ts`
- Modify: `packages/core/src/services/organization-service.ts`
- Modify: `packages/core/src/db/repositories/__tests__/organization-repository.test.ts` (or create if absent)

**Goal:** Add `status` to filter options and CRUD operations.

- [ ] **Add `status` to `OrganizationFilter`.** The filter already has `org_type`, `worked`, and `search`. Add `status` as an optional string field.

```typescript
export interface OrganizationFilter {
  org_type?: string
  worked?: number
  status?: string  // 'interested' | 'review' | 'targeting' | 'excluded'
  search?: string
}
```

- [ ] **Update `list()` to support status filter.** Add a condition block in the filter-building loop:

```typescript
if (filter?.status !== undefined) {
  conditions.push('status = ?')
  params.push(filter.status)
}
```

- [ ] **Add `status` to `CreateOrganizationInput`.** Add `status?: string` to the input interface. This is optional -- defaults to NULL in the database.

```typescript
export interface CreateOrganizationInput {
  // ... existing fields ...
  status?: string
}
```

- [ ] **Update `create()` to include status.** Add `status` to the INSERT column list and bind `input.status ?? null`.

- [ ] **Update `update()` to handle status.** Add a condition in the SET-building block:

```typescript
if (input.status !== undefined) { sets.push('status = ?'); params.push(input.status) }
```

- [ ] **Add `status` to `OrganizationService` validation.** In `create()` and `update()`, validate that status is one of the allowed values when provided:

```typescript
const VALID_STATUSES = ['interested', 'review', 'targeting', 'excluded']

// In create() and update():
if (input.status !== undefined && input.status !== null && !VALID_STATUSES.includes(input.status)) {
  return { ok: false, error: { code: 'VALIDATION_ERROR', message: `Invalid status: ${input.status}. Must be one of: ${VALID_STATUSES.join(', ')}` } }
}
```

- [ ] **Tests:**
  - Create org with status = 'interested', verify it persists
  - List with `status: 'interested'` filter returns only matching orgs
  - List with `status: 'targeting'` filter returns empty when none match
  - Update org status from NULL to 'interested' to 'review'
  - Update org status to NULL (reset)
  - Validation rejects invalid status value

**Acceptance Criteria:**
- [ ] `status` appears in filter, create, and update interfaces
- [ ] `list()` filters by status when provided
- [ ] Service validates status values
- [ ] All tests pass

---

### Task 18.2: Update Organization Routes

**File:** `packages/core/src/routes/organizations.ts`

**Goal:** Wire the `status` query parameter through to the service filter.

- [ ] **Add `status` to GET /organizations filter parsing.** In the existing `app.get('/organizations', ...)` handler, add:

```typescript
if (c.req.query('status')) filter.status = c.req.query('status')!
```

This goes after the existing `org_type`, `worked`, and `search` filter lines.

No other route changes needed -- PATCH already passes the entire body through to `update()`, and POST passes through to `create()`.

**Acceptance Criteria:**
- [ ] `GET /api/organizations?status=interested` returns only orgs with that status
- [ ] `GET /api/organizations?status=targeting&org_type=company` combines filters correctly

---

### Task 18.3: Update SDK Types

**Files:**
- Modify: `packages/sdk/src/types.ts`

**Goal:** Add `status` field to Organization type and OrganizationFilter.

- [ ] ~~**Add `status` to `Organization` interface.**~~ **SKIP:** Phase 16 T16.3 already adds `status` to both core and SDK `Organization` types. Do NOT duplicate this change.

- [ ] **Add `status` to `OrganizationFilter`.** The SDK filter currently has `org_type` and `worked`:

```typescript
export interface OrganizationFilter {
  org_type?: string
  worked?: string
  status?: string
}
```

- [ ] **Add `status` to `CreateOrganization`.** Add `status?: string` to the input.

- [ ] **Add `status` to `UpdateOrganization`.** Add `status?: string | null` to the input.

- [ ] ~~**Update core types.**~~ **SKIP:** Phase 16 T16.3 already adds `status` to `Organization` in `packages/core/src/types/index.ts`. Do NOT duplicate.

**Acceptance Criteria:**
- [ ] SDK filter, create, and update types include `status` (the base `Organization.status` field is already added by Phase 16 T16.3)
- [ ] TypeScript compiles without errors

---

### Task 18.4: Update Organizations View (UI)

**File:** `packages/webui/src/routes/organizations/+page.svelte`

**Goal:** Add status filter, status badges, and status editor to the Organizations view.

- [ ] **Add ORG_STATUSES constant** at top of `<script>`:

```typescript
const ORG_STATUSES = [
  { value: 'all', label: 'All' },
  { value: 'interested', label: 'Interested' },
  { value: 'review', label: 'Review' },
  { value: 'targeting', label: 'Targeting' },
  { value: 'excluded', label: 'Excluded' },
  { value: 'worked', label: 'Worked' },
]
```

- [ ] **Add statusFilter state.** Below the existing `workedFilter`:

```typescript
let statusFilter = $state('all')
```

- [ ] **Update filteredOrgs derived.** Add status filtering logic. The "worked" option in the status tabs is a shortcut for `worked=1`, all others filter by the `status` column:

```typescript
let filteredOrgs = $derived.by(() => {
  let result = organizations
  if (typeFilter !== 'all') {
    result = result.filter(o => o.org_type === typeFilter)
  }
  if (statusFilter === 'worked') {
    result = result.filter(o => o.worked)
  } else if (statusFilter !== 'all') {
    result = result.filter(o => o.status === statusFilter)
  }
  // Remove the old workedFilter logic (replaced by statusFilter)
  return result
})
```

- [ ] **Replace the workedFilter dropdown with status filter tabs.** Replace the second `<select>` in the filter-bar with horizontal tabs (same pattern as Sources view):

```svelte
<div class="filter-tabs">
  {#each ORG_STATUSES as tab}
    <button
      class="filter-tab"
      class:active={statusFilter === tab.value}
      onclick={() => statusFilter = tab.value}
    >
      {tab.label}
    </button>
  {/each}
</div>
```

- [ ] **Add status badge to org cards.** In the `.card-top` div, after the type badge, add a status badge when status is non-null:

```svelte
{#if org.status}
  <span class="status-badge status-{org.status}">{org.status}</span>
{/if}
```

- [ ] **Add status badge CSS.** Use the colors from the spec:

```css
.status-badge {
  display: inline-block;
  padding: 0.1em 0.4em;
  border-radius: 3px;
  font-size: 0.65rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.03em;
}

.status-interested { background: #dbeafe; color: #1e40af; }
.status-review { background: #fef3c7; color: #92400e; }
.status-targeting { background: #d1fae5; color: #065f46; }
.status-excluded { background: #f3f4f6; color: #6b7280; text-decoration: line-through; }
```

- [ ] **Add formStatus state and populate it.** Add to the form state declarations:

```typescript
let formStatus = $state<string | null>(null)
```

In `populateForm()`:
```typescript
formStatus = org.status ?? null
```

In `startNew()`:
```typescript
formStatus = null
```

- [ ] **Add status dropdown to editor.** After the "Worked" checkbox, add:

```svelte
<div class="form-group">
  <label for="org-status">Status</label>
  <select id="org-status" bind:value={formStatus}>
    <option value={null}>No status</option>
    <option value="interested">Interested</option>
    <option value="review">Review</option>
    <option value="targeting">Targeting</option>
    <option value="excluded">Excluded</option>
  </select>
</div>
```

- [ ] **Include status in save payload.** In `saveOrg()`, add to the payload:

```typescript
const payload: Record<string, unknown> = {
  // ... existing fields ...
  status: formStatus,
}
```

**Acceptance Criteria:**
- [ ] Status filter tabs appear in the list panel header area
- [ ] Clicking a status tab filters the organization list
- [ ] "Worked" tab filters by `worked=true` (not the status column)
- [ ] Status badges appear on org cards with correct colors
- [ ] Excluded orgs show strikethrough text on the badge
- [ ] Status dropdown in editor allows setting/clearing status
- [ ] Saving persists the status value

---

### Task 18.5: Personal Project Handling in Sources View

**File:** `packages/webui/src/routes/sources/+page.svelte`

**Goal:** Show "Personal" label in the Projects tab and hide org dropdown when personal project is checked.

- [ ] **Add "Personal" label to project source cards.** In the source card meta area, after the type badge, add a personal label when applicable:

```svelte
{#if source.source_type === 'project' && source.project?.is_personal}
  <span class="personal-badge">Personal</span>
{/if}
```

- [ ] **Add personal-badge CSS:**

```css
.personal-badge {
  display: inline-block;
  padding: 0.1em 0.4em;
  background: #ede9fe;
  color: #6d28d9;
  border-radius: 3px;
  font-size: 0.65rem;
  font-weight: 600;
  text-transform: uppercase;
}
```

- [ ] **Hide org dropdown when personal project is checked.** In the project-specific fields section, wrap the org dropdown in a conditional:

```svelte
{#if formSourceType === 'project'}
  <div class="form-group">
    <label>
      <input type="checkbox" bind:checked={formIsPersonal} /> Personal project
    </label>
  </div>
  {#if !formIsPersonal}
    <div class="form-group">
      <label for="proj-org">Organization</label>
      <select id="proj-org" bind:value={formOrgId}>
        <option value={null}>None</option>
        {#each organizations as org}
          <option value={org.id}>{org.name}</option>
        {/each}
      </select>
    </div>
  {/if}
  <!-- ... url, dates ... -->
{/if}
```

- [ ] **Clear orgId when personal is toggled on.** Add an `$effect` or handle in the checkbox change:

```typescript
$effect(() => {
  if (formIsPersonal) {
    formOrgId = null
  }
})
```

**Acceptance Criteria:**
- [ ] Project sources with `is_personal=true` show "Personal" badge in the list
- [ ] Checking "Personal project" hides the Organization dropdown
- [ ] Unchecking "Personal project" shows the Organization dropdown again
- [ ] When personal is toggled on, `organization_id` is cleared to null
- [ ] Personal projects save correctly without an organization_id

---

### Task 18.6: Tests

**Files:**
- Create or modify: `packages/core/src/db/repositories/__tests__/organization-repository.test.ts`
- Create: `packages/webui/src/routes/organizations/__tests__/+page.test.ts` (optional -- depends on existing test patterns)

**Goal:** Verify status filtering and badge rendering.

- [ ] **Repository tests:**
  - `list() with status filter returns matching orgs only`
  - `list() with status + org_type combined filter`
  - `create() with status persists the value`
  - `update() can change status`
  - `update() can clear status to null`

- [ ] **Service tests:**
  - `create() rejects invalid status`
  - `create() accepts valid status`
  - `update() rejects invalid status`

- [ ] **Route integration tests (if test pattern exists):**
  - `GET /api/organizations?status=interested` returns filtered results
  - `POST /api/organizations` with `status: 'interested'` creates with status
  - `PATCH /api/organizations/:id` with `status: 'targeting'` updates status

**Acceptance Criteria:**
- [ ] All repository tests pass
- [ ] All service validation tests pass
- [ ] No regressions in existing organization tests

---

### Task 18.7: Documentation

**Files:**
- Modify: `docs/src/data/models/entity-types.md`

**Goal:** Document the `status` field on the Organization entity.

- [ ] **Add status to Organization entity docs.** In the Organization section, add:

```markdown
### Status Lifecycle

Organizations have an optional `status` field tracking pipeline progress:

| Status | Color | Description |
|--------|-------|-------------|
| `null` | — | No tracking (default, used for historical employers) |
| `interested` | Blue | On the radar, not yet researched |
| `review` | Amber | Actively researching |
| `targeting` | Green | Actively pursuing opportunities |
| `excluded` | Gray | Decided against (reason in notes) |

The `worked` boolean is orthogonal to `status`. An organization can be both `worked=true` (you worked there before) and `status='targeting'` (you want to go back).

Status transitions are guided by the UI but not enforced in the database:
```
NULL -> interested -> review -> targeting
                            -> excluded
targeting -> excluded (changed mind)
excluded -> interested (reconsidering)
```
```

- [ ] **Add personal project handling note.** In the Source entity docs or Projects section:

```markdown
### Personal Projects

When `source_projects.is_personal = 1`, the project has no associated organization.
The Sources view shows a "Personal" label. The organization dropdown is hidden
when the "Personal project" checkbox is checked.
```

**Acceptance Criteria:**
- [ ] Status lifecycle documented with table and transition diagram
- [ ] Personal project behavior documented
