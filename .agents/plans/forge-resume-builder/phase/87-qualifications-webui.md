# Phase 87: Qualifications — WebUI

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans

**Spec:** `refs/specs/2026-04-05-qualifications-credentials-certifications.md` (Section 7)
**Depends on:** Phase 86 (SDK resources must exist)
**Blocks:** Nothing
**Parallelizable with:** Phase 88 (IR + MCP can run in parallel with UI work)
**Duration:** Medium (5 tasks: T87.1 through T87.5)

## Goal

Add the "Qualifications" sidebar group with Credentials and Certifications pages. Remove the clearances entry from the Experience group. Both pages use the existing shared component library (SplitPanel, ListPanelHeader, PageWrapper, etc.).

## Non-Goals

- IR compiler changes (Phase 88)
- MCP tool registration (Phase 88)
- JD matching integration (deferred)
- Credential expiry notifications

## Files to Create

| File | Description |
|------|-------------|
| `packages/webui/src/routes/qualifications/credentials/+page.svelte` | Credentials split-panel page |
| `packages/webui/src/routes/qualifications/certifications/+page.svelte` | Certifications split-panel page |

## Files to Modify

| File | Change |
|------|--------|
| `packages/webui/src/lib/nav.ts` | Add Qualifications group, remove Clearances from Experience |
| `packages/webui/src/routes/experience/clearances/+page.svelte` | Delete or redirect to /qualifications/credentials |

---

## Tasks

### T87.1: Update Navigation

**File:** `packages/webui/src/lib/nav.ts`
**Goal:** Add Qualifications sidebar group, remove Clearances from Experience.

**Steps:**
1. Remove `{ href: '/experience/clearances', label: 'Clearances' }` from Experience children
2. Add new Qualifications group after Experience, before Data:
   ```typescript
   {
     label: 'Qualifications',
     prefix: '/qualifications',
     children: [
       { href: '/qualifications/credentials', label: 'Credentials' },
       { href: '/qualifications/certifications', label: 'Certifications' },
     ],
   },
   ```
3. Update nav tests if any assert on group count or Experience children

**Acceptance Criteria:**
- [ ] Qualifications appears in sidebar between Experience and Data
- [ ] Clearances no longer appears under Experience
- [ ] Nav auto-expand works for `/qualifications/*` routes

### T87.2: Create Credentials Page

**Route:** `/qualifications/credentials/+page.svelte`
**Goal:** Split-panel list + type-specific form for all credential types.

**Left panel:**
- `ListPanelHeader` with "Credentials" title and "New" button (dropdown or selector for credential type)
- List of credentials grouped or sorted by `credential_type`
- Each item shows: label, type badge (clearance/license/admission), status badge (active/inactive/expired)
- `ListSearchInput` for filtering by label

**Right panel (empty state):**
- `EmptyPanel` with "Select a credential or create one"

**Right panel (selected):**
- Form fields common to all types: label, status (dropdown), organization (OrgCombobox), issued date, expiry date
- Type-specific form section based on `credential_type`:
  - **Clearance:** level dropdown, polygraph dropdown, clearance_type dropdown, access_programs multi-select
  - **Driver's License:** class input, state input, endorsements multi-input
  - **Bar Admission:** jurisdiction input, bar_number input
  - **Medical License:** license_type input, state input, license_number input
- Save and Delete buttons using `.btn` + `.btn-primary` / `.btn-danger` classes

**Implementation notes:**
- Use `PageWrapper` for layout
- Use `SplitPanel` for the two-column layout
- Use shared component imports from `$lib/components`
- `onMount` for data loading (NOT `$effect`)
- Save via `forge.credentials.update()`, create via `forge.credentials.create()`

**Acceptance Criteria:**
- [ ] All 4 credential types render correct form fields
- [ ] CRUD operations work (create, read, update, delete)
- [ ] Type-specific details saved correctly in JSON
- [ ] OrgCombobox works for issuing/sponsor org
- [ ] Uses all required shared components (no inline CSS violations)

### T87.3: Create Certifications Page

**Route:** `/qualifications/certifications/+page.svelte`
**Goal:** Split-panel list + detail form with skill tagging.

**Left panel:**
- `ListPanelHeader` with "Certifications" title and "New" button
- List of certifications sorted by name
- Each item shows: name, issuer, active/expired status (derived from expiry_date)
- `ListSearchInput` for filtering by name/issuer

**Right panel (empty state):**
- `EmptyPanel` with "Select a certification or create one"

**Right panel (selected):**
- Form fields: name, issuer, date_earned (date picker), expiry_date (date picker), credential_id, credential_url
- Education source link: dropdown of education-type sources (optional)
- Skills section: tag display of linked skills with add/remove buttons (similar to JD skill tagging pattern)
- Save and Delete buttons

**Skill tagging pattern:**
- Display linked skills as removable tags/chips
- "Add Skill" button opens a skill search/select dropdown
- Adding a skill calls `forge.certifications.addSkill()`
- Removing calls `forge.certifications.removeSkill()`

**Acceptance Criteria:**
- [ ] CRUD operations work
- [ ] Skill add/remove works with immediate visual feedback
- [ ] Education source dropdown shows only education-type sources
- [ ] Expiry-based status display (active vs expired)
- [ ] Uses all required shared components

### T87.4: Remove Clearances Experience Page

**Goal:** Clean up the old clearances route.

**Steps:**
1. Delete `/experience/clearances/+page.svelte` (or the relevant route file)
2. If SvelteKit doesn't support route-level redirects easily, add a simple page that redirects to `/qualifications/credentials`
3. Verify no other pages link to `/experience/clearances`

**Acceptance Criteria:**
- [ ] `/experience/clearances` route removed or redirects
- [ ] No broken links in the application

### T87.5: WebUI Tests

**Goal:** Behavioral tests for the new pages.

**Tests:**
1. Navigation: Qualifications group exists with Credentials and Certifications children
2. Navigation: Experience group no longer contains Clearances
3. Credentials page: renders split-panel layout, shows credential list, type-specific forms
4. Certifications page: renders split-panel layout, shows cert list, skill tags section
5. Shared component usage: verify PageWrapper, SplitPanel, ListPanelHeader used (not inline CSS)

**Acceptance Criteria:**
- [ ] All behavioral tests pass
- [ ] WebUI test suite has zero failures
