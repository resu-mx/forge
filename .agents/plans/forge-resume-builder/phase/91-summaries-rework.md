# Phase 91: Summaries Rework

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans

**Depends on:** Phase 89 (Skills for keywords), Phase 90 (Industries & Role Types for structured fields)
**Blocks:** Nothing
**Parallelizable with:** Phase 92 (Tagline Engine) after 89 completes
**Duration:** Medium (6 tasks)

## Goal

Overhaul the Summaries page: add structured fields (keywords from skills, industry, role_type), make entries clickable with overlay modal instead of inline buttons, add sticky/collapsible section headers, add grouping/sorting/filtering. Remove tagline from summaries (moved to resume header in Phase 92).

## Non-Goals

- Tagline generation (Phase 92)
- Summary import into resumes (Phase 95)

---

## Tasks

### T91.1: Schema — Summary Structured Fields

**File:** `packages/core/src/db/migrations/033_summary_structured_fields.sql`

**Steps:**
1. Add `industry_id` FK to `summaries` (nullable, references industries)
2. Add `role_type_id` FK to `summaries` (nullable, references role_types)
3. Create `summary_skills` junction table (summary_id, skill_id — for keywords)
4. Remove `tagline` column from summaries (or mark deprecated if data needs preserving first)

**Acceptance Criteria:**
- [ ] FKs and junction table created
- [ ] Tagline column removed or deprecated
- [ ] Existing data preserved

### T91.2: Update Types, Repository, Service

**Steps:**
1. Add `industry_id`, `role_type_id` to Summary types
2. Add `SummaryWithRelations` interface (includes industry name, role_type name, skills array)
3. Add keyword skill junction methods to summary repository
4. Remove tagline from CreateSummary/UpdateSummary
5. Add filter params: `{ industry_id?, role_type_id?, skill_id? }`
6. Add sort params: `{ sort_by: 'title' | 'created_at' | 'updated_at' }`

**Acceptance Criteria:**
- [ ] Structured fields in types and repository
- [ ] Filtering and sorting work
- [ ] No tagline references remain

### T91.3: Update API & SDK

**Steps:**
1. Update summary routes with filter/sort query params
2. Update SDK SummaryResource
3. Add skill junction endpoints on summaries

**Acceptance Criteria:**
- [ ] API supports filtering by industry, role_type, keyword skill
- [ ] API supports sorting by title

### T91.4: Summaries Page — Clickable Entries + Overlay Modal

**Steps:**
1. Replace inline edit/clone/promote/delete buttons with clickable list entries
2. Clicking an entry opens an overlay modal (similar to ChainViewModal pattern)
3. Modal shows full summary content with edit capability
4. Action buttons (clone, promote, delete) move into the modal

**Acceptance Criteria:**
- [ ] List entries are clickable, no inline action buttons
- [ ] Overlay modal opens on click with full CRUD
- [ ] Modal follows existing modal patterns (z-index, backdrop, escape to close)

### T91.5: Summaries Page — Sticky Headers + Grouping/Sorting/Filtering

**Steps:**
1. Make page title, Templates section header, and Summaries section header sticky on scroll (CSS `position: sticky` with stacking)
2. Add group-by selector: None, Industry, Role Type, Keyword
3. Add sort-by selector: Title (A-Z, Z-A)
4. Add filter controls: industry dropdown, role_type dropdown, keyword/skill multi-select
5. Templates section and Summaries section both collapsible (click header to toggle)

**Acceptance Criteria:**
- [ ] All three headers remain visible when scrolled to bottom
- [ ] Grouping creates collapsible subsections
- [ ] Sorting reorders entries
- [ ] Filtering reduces visible entries
- [ ] Sections are collapsible

### T91.6: Tests

**Acceptance Criteria:**
- [ ] Migration test: structured fields added, tagline removed
- [ ] Repository test: filter by industry/role_type/skill, sort by title
- [ ] UI test: sticky headers, clickable entries, modal open/close
