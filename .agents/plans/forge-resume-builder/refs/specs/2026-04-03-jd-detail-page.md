# JD Detail Page

**Date:** 2026-04-03
**Spec:** E1 (JD Detail Page)
**Phase:** TBD (next available)
**Builds on:** Phase 31 (Job Descriptions Entity — migration 007, repository, service, API routes, SDK resource)
**Dependencies:** None
**Blocks:** E2, E3, E4, E5, E6

## Overview

Phase 31 built the full JD backend: migration 007 (`job_descriptions` table), `JobDescriptionRepository`, `JobDescriptionService`, API routes (`/api/job-descriptions` CRUD), and the SDK `JobDescriptionsResource`. The placeholder at `/opportunities/job-descriptions` exists but only renders an `EmptyState` component.

This spec builds a full CRUD UI for job descriptions using a split-panel layout (list + editor). It also adds JD skill tagging — a `job_description_skills` junction table and API endpoints following the same pattern as `source_skills` and `bullet_skills`.

## Prerequisites

**PREREQUISITE MIGRATION 017:** Before any E-spec work, expand the `prompt_logs` table CHECK constraint to include `'job_description'` as a valid `entity_type`. This unblocks E6's prompt logging. Migration 017 also adds `status TEXT` and `error_message TEXT` columns to `prompt_logs` (needed by E6).

## Non-Goals

- JD parsing / NLP extraction of requirements, skills, experience levels
- Skill matching or gap analysis against user's skill inventory
- Resume-to-JD linking or recommendation
- JD fetching from job boards (Indeed, LinkedIn, Greenhouse)
- Kanban pipeline view for JDs (deferred to Spec E3)
- Contact linking on the JD page (deferred to Spec G)
- Structured salary parsing (salary_range remains free-text per the existing schema)
- Duplicate detection
- Cover letter generation

---

## 1. Schema Changes

### 1.1 Migration: `018_job_description_skills.sql`

A new junction table for linking skills to job descriptions, following the exact pattern of `source_skills` (migration 016) and `bullet_skills` (migration 001).

```sql
-- Job Description Skills Junction
-- Migration: 018_job_description_skills
-- Links skills to job descriptions for requirement tracking.
-- Skills describe what technologies/tools the JD requires.

CREATE TABLE job_description_skills (
  job_description_id TEXT NOT NULL REFERENCES job_descriptions(id) ON DELETE CASCADE,
  skill_id TEXT NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
  PRIMARY KEY (job_description_id, skill_id)
) STRICT;

CREATE INDEX idx_jd_skills_jd ON job_description_skills(job_description_id);
CREATE INDEX idx_jd_skills_skill ON job_description_skills(skill_id);

INSERT INTO _migrations (name) VALUES ('018_job_description_skills');
```

### 1.2 No Changes to `job_descriptions` Table

The existing schema (migration 007) already has all required columns: `title`, `organization_id`, `url`, `raw_text`, `status`, `salary_range`, `location`, `notes`, `created_at`, `updated_at`. The spec prompt mentions "role/position" and "description vs requirements" as separate fields, but the current schema stores the full JD text in `raw_text` and the job title in `title`. No new columns are added — the existing schema is sufficient for the UI. Structured field extraction (role, requirements, description as separate fields) is deferred to a future parsing spec.

---

## 2. API Changes

### 2.1 Existing Endpoints (No Changes)

All CRUD endpoints from Phase 31 are used as-is:

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/job-descriptions` | Create a JD |
| `GET` | `/api/job-descriptions` | List JDs (with `?status=`, `?organization_id=`, `?limit=`, `?offset=` filters) |
| `GET` | `/api/job-descriptions/:id` | Get a JD (with `organization_name` JOIN) |
| `PATCH` | `/api/job-descriptions/:id` | Update a JD |
| `DELETE` | `/api/job-descriptions/:id` | Delete a JD |

### 2.2 New Endpoints: JD Skills

Add to `packages/core/src/routes/job-descriptions.ts`, matching the source skills pattern from `sources.ts` lines 90-150:

| Method | Path | Description | Request Body | Response |
|--------|------|-------------|-------------|----------|
| `GET` | `/api/job-descriptions/:id/skills` | List skills linked to this JD | — | `{ data: Skill[] }` |
| `POST` | `/api/job-descriptions/:id/skills` | Link a skill (or create + link) | `{ skill_id: string }` or `{ name: string, category?: string }` | `{ data: Skill }` (201) |
| `DELETE` | `/api/job-descriptions/:jdId/skills/:skillId` | Unlink a skill | — | 204 |

**POST behavior (identical to source skills):**
- If `skill_id` is provided: `INSERT OR IGNORE INTO job_description_skills (job_description_id, skill_id) VALUES (?, ?)`
- If `name` is provided: capitalize first character (`raw.charAt(0).toUpperCase() + raw.slice(1)`), case-insensitive dedup via `SELECT * FROM skills WHERE name = ? COLLATE NOCASE`, create if not found, then link
- If neither: return 400 `VALIDATION_ERROR`

**DELETE behavior:**
- Delete the junction row. Return 204 regardless of whether the row existed (idempotent).

---

## 3. UI Layout

### 3.1 Split-Panel Design

The `/opportunities/job-descriptions` page uses a split-panel layout with a list panel on the left and an editor panel on the right, consistent with the patterns used in `/data/sources` (SourcesView), `/data/bullets` (BulletsView), and `/data/skills`.

```
┌──────────────────────┬──────────────────────────────────────┐
│ Job Descriptions     │ [Editor Panel]                       │
│ ┌──────────────────┐ │                                      │
│ │ [+ New JD]       │ │  Title: [________________________]   │
│ │ [Status ▾][🔍]   │ │  Organization: [dropdown________]   │
│ ├──────────────────┤ │  Status: [interested ▾]              │
│ │ ┌──────────────┐ │ │  Location: [____________________]   │
│ │ │ Sr Security  │ │ │  Salary Range: [________________]   │
│ │ │ Cloudflare   │ │ │  URL: [________________________]    │
│ │ │ 🟢 applied   │ │ │                                      │
│ │ │ Remote $180k │ │ │  Description:                        │
│ │ └──────────────┘ │ │  ┌──────────────────────────────┐   │
│ │ ┌──────────────┐ │ │  │                              │   │
│ │ │ DevOps Lead  │ │ │  │   (large textarea)           │   │
│ │ │ Acme Corp    │ │ │  │                              │   │
│ │ │ ⚪ interested│ │ │  └──────────────────────────────┘   │
│ │ │ NYC $150k    │ │ │                                      │
│ │ └──────────────┘ │ │  Required Skills:                    │
│ │                  │ │  [Python ×] [AWS ×] [+ add skill]   │
│ │ ...              │ │                                      │
│ └──────────────────┘ │  Notes:                              │
│                      │  ┌──────────────────────────────┐   │
│                      │  │ (textarea)                   │   │
│                      │  └──────────────────────────────┘   │
│                      │                                      │
│                      │     [Save]              [Delete]     │
└──────────────────────┴──────────────────────────────────────┘
```

### 3.2 List Panel

**Card content:**
- **Title** (bold, primary text)
- **Organization name** (muted text, or "No organization" if null)
- **Status badge** (colored pill using existing `StatusBadge.svelte` pattern)
- **Location** (if present, muted text)
- **Salary range** (if present, muted text)

**Controls (above card list):**
- "New JD" button (top, opens editor in create mode)
- Status filter dropdown (all 8 statuses + "All")
- Search input (client-side filter on title and organization_name)

**Sorting:** By `updated_at` descending (most recently modified first).

> **Note:** Update `JobDescriptionRepository.list()` to order by `updated_at DESC` (currently orders by `created_at DESC`). Add `job-description-repository.ts` to Files to Modify.

**Selection:** Clicking a card selects it and populates the editor panel. Active card has a highlighted border (same pattern as other split-panel views).

### 3.3 Editor Panel

The editor panel shows all JD fields in a form layout. It has two modes:

**Create mode** (triggered by "New JD" button):
- All fields empty
- Title and raw_text are required
- "Create" button at the bottom
- On success: new JD appears in list, is auto-selected

**Edit mode** (triggered by clicking a list card):
- Fields populated from the selected JD
- "Save" button at the bottom (disabled until changes detected)
- "Delete" button (bottom right, with ConfirmDialog)
- Status dropdown changes save immediately on selection (no need to click Save)

**Empty state** (no JD selected, no create mode):
- Show a centered message: "Select a job description or create a new one"

### 3.4 Field Definitions

| Field | Input Type | Required | Notes |
|-------|-----------|----------|-------|
| Title | Text input | Yes | Job title from the posting |
| Organization | Dropdown (select from all orgs) | No | `organization_id` FK. Shows `organization_name`. Includes a "None" option to clear. |
| Status | Dropdown (8 values) | Yes (default: `interested`) | Immediate save on change in edit mode |
| Location | Text input | No | Free text: "Remote", "San Francisco, CA", "Hybrid - DC" |
| Salary Range | Text input | No | Free text: "$150k-$200k", "DOE", "GS-13" |
| URL | Text input (type=url) | No | Link to original posting. Rendered as clickable link with external icon when populated. |
| Description | Textarea (large, min-height 200px) | Yes | Maps to `raw_text`. Preserves whitespace. |
| Required Skills | Skill tag picker | No | JD skill tagging (see Section 4) |
| Notes | Textarea (medium) | No | User's private notes |

### 3.5 Organization Dropdown

The organization dropdown loads all organizations via `forge.organizations.list({ limit: 500 })`. It displays `organization.name` and stores `organization.id` as the value. Includes:
- "None" option at the top (sets `organization_id = null`)
- All orgs listed alphabetically by name
- No autocomplete or create-on-the-fly in this spec (the org must already exist). Creating orgs on-the-fly from the JD page is a future enhancement.

### 3.6 Status Badge Colors

Uses the same color scheme from the JD Entity spec (Section 3.4):

| Status | Color | Visual |
|--------|-------|--------|
| `interested` | Gray | Muted |
| `analyzing` | Blue | Active |
| `applied` | Indigo | In-progress |
| `interviewing` | Purple | In-progress |
| `offered` | Green | Positive |
| `rejected` | Red | Negative |
| `withdrawn` | Orange | Warning |
| `closed` | Dark gray | Terminal |

Update `StatusBadge.svelte` to handle `JobDescriptionStatus` values if not already covered.

> **Note:** StatusBadge entries for the current 8 statuses are added here. E3 will add `discovered` and `applying` entries — these are intentionally deferred to E3.

---

## 4. Skill Tagging

### 4.1 UI Pattern

The required skills section uses the same tag picker pattern as source skills (SourcesView) and bullet skills (BulletDetailModal):

```
Required Skills:
[Python ×] [AWS ×] [Kubernetes ×]
[Search or add skill... ▾]
```

**Behavior:**
- Display: horizontal row of pill tags, each with a × remove button
- Add: text input with dropdown showing matching skills from `GET /api/skills`
  - Typing filters the dropdown (case-insensitive substring match on skill name)
  - Selecting from dropdown calls `POST /api/job-descriptions/:id/skills { skill_id: ... }`
  - Typing a new name and pressing Enter calls `POST /api/job-descriptions/:id/skills { name: ... }` (creates the skill if needed, with capitalizeFirst)
- Remove: clicking × calls `DELETE /api/job-descriptions/:jdId/skills/:skillId`
- All skill operations are immediate (no "Save" needed) — same as source/bullet skills

### 4.2 State Management

```typescript
let jdSkills = $state<Skill[]>([])
let allSkills = $state<Skill[]>([])
let skillSearch = $state('')
let showSkillDropdown = $state(false)
```

- On JD selection: `GET /api/job-descriptions/:id/skills` populates `jdSkills`
- On mount: `GET /api/skills` populates `allSkills` (for the dropdown)
- Dropdown items: `allSkills.filter(s => !jdSkills.some(js => js.id === s.id))` filtered by `skillSearch`

---

## 5. Component Architecture

### 5.1 Main Page Component

**File:** `packages/webui/src/routes/opportunities/job-descriptions/+page.svelte`

Replace the current placeholder with the split-panel layout. This component manages:
- JD list state (loaded from SDK)
- Selected JD state
- Create/edit mode toggle
- Filter state (status dropdown, search text)

### 5.2 Sub-Components

| Component | File | Purpose |
|-----------|------|---------|
| `JDCard.svelte` | `packages/webui/src/lib/components/jd/JDCard.svelte` | Card for the list panel |
| `JDEditor.svelte` | `packages/webui/src/lib/components/jd/JDEditor.svelte` | Editor form panel (create + edit modes) |
| `JDSkillPicker.svelte` | `packages/webui/src/lib/components/jd/JDSkillPicker.svelte` | Skill tag picker (reuses the same pattern as source/bullet skill pickers) |

### 5.3 State Flow

```typescript
// Main page state
let jds = $state<JobDescriptionWithOrg[]>([])
let selectedId = $state<string | null>(null)
let createMode = $state(false)
let statusFilter = $state<JobDescriptionStatus | 'all'>('all')
let searchText = $state('')
let loading = $state(true)
let organizations = $state<Organization[]>([])

// Derived
let filteredJds = $derived(
  jds
    .filter(jd => statusFilter === 'all' || jd.status === statusFilter)
    .filter(jd =>
      !searchText ||
      jd.title.toLowerCase().includes(searchText.toLowerCase()) ||
      jd.organization_name?.toLowerCase().includes(searchText.toLowerCase())
    )
)

let selectedJd = $derived(jds.find(jd => jd.id === selectedId) ?? null)
```

**Data loading (on mount):**
1. `forge.jobDescriptions.list({ limit: 200 })` — populate `jds`
2. `forge.organizations.list({ limit: 500 })` — populate `organizations` (for the org dropdown)

**Refresh:** After any mutation (create, update, delete), re-fetch the JD list.

---

## 6. Files to Create

| File | Purpose |
|------|---------|
| `packages/core/src/db/migrations/018_job_description_skills.sql` | Junction table for JD-to-skill linking |
| `packages/webui/src/lib/components/jd/JDCard.svelte` | JD card for list panel |
| `packages/webui/src/lib/components/jd/JDEditor.svelte` | JD editor form (create + edit) |
| `packages/webui/src/lib/components/jd/JDSkillPicker.svelte` | Skill tag picker for JD required skills |

## 7. Files to Modify

| File | Change |
|------|--------|
| `packages/webui/src/routes/opportunities/job-descriptions/+page.svelte` | Replace placeholder EmptyState with full split-panel CRUD layout |
| `packages/core/src/routes/job-descriptions.ts` | Add JD skills endpoints: `GET/POST /:id/skills`, `DELETE /:jdId/skills/:skillId` |
| `packages/sdk/src/resources/job-descriptions.ts` | Add `listSkills(jdId)`, `addSkill(jdId, input)`, `removeSkill(jdId, skillId)` methods |
| `packages/webui/src/lib/components/StatusBadge.svelte` | Add JD status color mappings (if not already present) |
| `packages/core/src/db/repositories/job-description-repository.ts` | Update `list()` to order by `updated_at DESC` instead of `created_at DESC` |

---

## 8. Testing

### 8.1 Migration Tests

- Migration 018 creates `job_description_skills` table with correct schema
- `INSERT INTO job_description_skills` with valid JD and skill IDs succeeds
- `INSERT INTO job_description_skills` with duplicate pair is rejected (PRIMARY KEY constraint)
- Deleting a JD cascades to `job_description_skills` rows
- Deleting a skill cascades to `job_description_skills` rows

### 8.2 API Tests (JD Skills)

- `GET /api/job-descriptions/:id/skills` returns empty array for JD with no skills
- `POST /api/job-descriptions/:id/skills` with `{ skill_id }` links an existing skill (201)
- `POST /api/job-descriptions/:id/skills` with `{ skill_id }` duplicate is idempotent (INSERT OR IGNORE)
- `POST /api/job-descriptions/:id/skills` with `{ name: 'python' }` creates "Python" skill (capitalizeFirst) and links it
- `POST /api/job-descriptions/:id/skills` with `{ name: 'python' }` when "Python" already exists links the existing one (case-insensitive dedup)
- `POST /api/job-descriptions/:id/skills` with neither `skill_id` nor `name` returns 400
- `DELETE /api/job-descriptions/:jdId/skills/:skillId` removes the junction row (204)
- `GET /api/job-descriptions/:id/skills` after linking returns correct skills

### 8.3 UI Component Tests

- Split-panel renders with list on left and editor on right
- "New JD" button switches to create mode with empty form
- Creating a JD with title and raw_text succeeds; new JD appears in list
- Creating a JD without title shows validation error
- Clicking a JD card selects it and populates editor
- Editing fields and clicking Save updates the JD
- Status dropdown change saves immediately
- Delete shows ConfirmDialog; confirming removes JD from list
- Status filter reduces visible cards
- Search filter matches on title and organization name
- Organization dropdown shows all orgs + "None" option
- Skill tag picker shows matching skills on type
- Adding a skill immediately appears as pill tag
- Removing a skill (× click) immediately removes the pill
- Creating a new skill via text input works (capitalizeFirst applied)
- URL field renders as clickable external link when populated
- Empty state shows when no JD is selected

### 8.4 SDK Tests

- `forge.jobDescriptions.listSkills(jdId)` returns `{ data: Skill[] }`
- `forge.jobDescriptions.addSkill(jdId, { skill_id })` returns `{ data: Skill }`
- `forge.jobDescriptions.addSkill(jdId, { name })` returns `{ data: Skill }`
- `forge.jobDescriptions.removeSkill(jdId, skillId)` returns void

---

## 9. Acceptance Criteria

1. `/opportunities/job-descriptions` renders a split-panel layout with JD list on the left and editor on the right
2. JD list cards show title, organization name, status badge, location, and salary range
3. "New JD" button opens the editor in create mode; creating a JD adds it to the list
4. Clicking a JD card populates the editor with that JD's data
5. All fields are editable: title, organization, status, location, salary range, URL, description (raw_text), notes
6. Status dropdown saves immediately on change (no "Save" click needed)
7. Organization dropdown shows all organizations with a "None" option to clear
8. URL field renders as a clickable external link when populated in view
9. "Save" button persists content changes via `PATCH /api/job-descriptions/:id`
10. "Delete" button shows ConfirmDialog, then deletes JD and deselects it
11. Status filter and search text filter the JD list client-side
12. Migration 018 creates `job_description_skills` junction table with CASCADE deletes
13. JD skills endpoints (`GET/POST/DELETE`) follow the source skills pattern exactly
14. Skill tag picker allows searching, selecting, creating, and removing skills with immediate persistence (no "Save" needed for skills)
15. New skills created inline appear in `/data/skills` with capitalizeFirst formatting
16. SDK `JobDescriptionsResource` has `listSkills`, `addSkill`, `removeSkill` methods
17. Editor shows "Select a job description or create a new one" when nothing is selected
18. JD list refreshes after any mutation (create, update, delete)
19. StatusBadge renders correct colors for all 8 JD status values
