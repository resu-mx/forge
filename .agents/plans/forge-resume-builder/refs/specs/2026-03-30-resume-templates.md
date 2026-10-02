# Resume Templates

**Date:** 2026-03-30
**Status:** Design
**Builds on:** Resume Sections as Entities (Phases 27-28), Config -- Profile (Spec 6)

## Purpose

Resume templates define reusable section layouts -- which sections exist, in what order, and what entry type each section uses -- but not the content. When creating a new resume, the user picks a template and gets a pre-configured set of sections instead of starting from scratch and manually adding each section via the "Add Section" dropdown.

Templates also enable "Save as Template" from any existing resume, allowing users to capture a layout they like and reuse it.

## Goals

1. `resume_templates` table with section layout stored as JSON
2. Built-in templates seeded during migration ("Standard Tech Resume", "Academic CV", "Federal Resume")
3. "Save as Template" button on the resume builder
4. Template picker during resume creation
5. CRUD API for templates
6. Templates management UI at `/config/templates`

## Non-Goals

- Content in templates (templates define structure only, not bullet points or summary text)
- Per-template styling or LaTeX configuration (all resumes use the same sb2nov template for now)
- Template marketplace or sharing
- Nested/conditional sections in templates
- Summary template integration (described below as a future connection point)

---

## 1. Schema (Migration 008)

**Migration number: 008.** Prior migrations in sequence:
| # | Name | Source |
|---|------|--------|
| 001 | Initial | -- |
| 002 | Schema Evolution | -- |
| 003 | Renderer and Entities | -- |
| 004 | Resume Sections | -- |
| 005 | User Profile | Spec 6 |
| 006 | Summaries | Spec 2 |
| 007 | Job Descriptions | Spec 4 |
| 008 | Resume Templates | Spec 8 (this spec) |

> **Note:** This migration is independent of 005-007 at the DDL level but must be numbered sequentially.

**File:** `packages/core/src/db/migrations/008_resume_templates.sql`

```sql
-- Forge Resume Builder -- Resume Templates
-- Migration: 008_resume_templates
-- Date: 2026-03-30
--
-- Creates resume_templates table for reusable section layouts.
-- Seeds three built-in templates.
-- Builds on 007_job_descriptions.

-- Step 1: Create resume_templates table
CREATE TABLE resume_templates (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  name TEXT NOT NULL,
  description TEXT,
  sections TEXT NOT NULL,     -- JSON array of section definitions
  is_builtin INTEGER NOT NULL DEFAULT 0,  -- 1 for seeded templates, 0 for user-created
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

-- Step 2: Seed built-in templates

-- Standard Tech Resume
INSERT INTO resume_templates (id, name, description, sections, is_builtin) VALUES (
  lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)),2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)),2) || '-' || hex(randomblob(6))),
  'Standard Tech Resume',
  'Classic layout for software engineering and technical roles. Summary, experience, skills, education, projects.',
  '[
    {"title": "Summary", "entry_type": "freeform", "position": 0},
    {"title": "Experience", "entry_type": "experience", "position": 1},
    {"title": "Technical Skills", "entry_type": "skills", "position": 2},
    {"title": "Education & Certifications", "entry_type": "education", "position": 3},
    {"title": "Selected Projects", "entry_type": "projects", "position": 4}
  ]',
  1
);

-- Academic CV
INSERT INTO resume_templates (id, name, description, sections, is_builtin) VALUES (
  lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)),2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)),2) || '-' || hex(randomblob(6))),
  'Academic CV',
  'Extended format for academic and research positions. Education first, then research, publications, presentations.',
  '[
    {"title": "Education", "entry_type": "education", "position": 0},
    {"title": "Research Experience", "entry_type": "experience", "position": 1},
    {"title": "Publications", "entry_type": "projects", "position": 2},
    {"title": "Presentations", "entry_type": "presentations", "position": 3},
    {"title": "Technical Skills", "entry_type": "skills", "position": 4},
    {"title": "Awards", "entry_type": "awards", "position": 5}
  ]',
  1
);

-- Federal Resume
INSERT INTO resume_templates (id, name, description, sections, is_builtin) VALUES (
  lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)),2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)),2) || '-' || hex(randomblob(6))),
  'Federal Resume',
  'Detailed format for US government positions. Includes security clearance section and extended experience detail.',
  '[
    {"title": "Summary of Qualifications", "entry_type": "freeform", "position": 0},
    {"title": "Security Clearance", "entry_type": "clearance", "position": 1},
    {"title": "Professional Experience", "entry_type": "experience", "position": 2},
    {"title": "Technical Competencies", "entry_type": "skills", "position": 3},
    {"title": "Education & Certifications", "entry_type": "education", "position": 4},
    {"title": "Selected Projects", "entry_type": "projects", "position": 5}
  ]',
  1
);

-- Step 3: Register migration
INSERT INTO _migrations (name) VALUES ('008_resume_templates');
```

### 1.1 The `sections` JSON Column

The `sections` column stores a JSON array of section definitions. Each element:

```typescript
interface TemplateSectionDef {
  title: string        // Display name: "Experience", "Technical Skills", etc.
  entry_type: string   // One of the valid entry_type values from resume_sections CHECK constraint
  position: number     // Sort order (0-based)
}
```

This is intentionally denormalized. Templates are lightweight metadata -- there's no need for a `template_sections` join table. The JSON array is small (typically 4-7 elements) and is always read/written as a whole unit.

### 1.2 `is_builtin` Column

Distinguishes seeded templates from user-created ones. Built-in templates:
- Cannot be deleted (enforced at the service layer, not DB)
- Can be edited (user might want to customize the built-in layouts)
- Are restored if deleted from the DB and re-seeded (but this is an edge case)

## 2. Types

**File:** `packages/core/src/types/index.ts`

```typescript
export interface ResumeTemplate {
  id: string
  name: string
  description: string | null
  sections: TemplateSectionDef[]
  is_builtin: number   // SQLite STRICT INTEGER: 0 | 1. Core keeps as number; SDK converts to boolean.
  created_at: string
  updated_at: string
}

export interface TemplateSectionDef {
  title: string
  entry_type: string
  position: number
}

export interface CreateResumeTemplate {
  name: string
  description?: string
  sections: TemplateSectionDef[]
}

export interface UpdateResumeTemplate {
  name?: string
  description?: string | null
  sections?: TemplateSectionDef[]
}
```

**Note on `is_builtin` type convention:** The core type uses `number` (`0 | 1`) to match the SQLite STRICT INTEGER column, consistent with the `is_template` convention used in `Summary`. The repository `deserialize()` does NOT convert -- it keeps the raw `0 | 1` value. The SDK type uses `boolean` for developer ergonomics; the SDK resource class converts `0 | 1` to `true | false` during deserialization and back to `0 | 1` during serialization.

## 3. Repository: `template-repository.ts`

**File:** `packages/core/src/db/repositories/template-repository.ts`

```typescript
export function list(db: Database): ResumeTemplate[] {
  const rows = db.query('SELECT * FROM resume_templates ORDER BY is_builtin DESC, name ASC').all()
  return rows.map(deserialize)
}

export function get(db: Database, id: string): ResumeTemplate | null {
  const row = db.query('SELECT * FROM resume_templates WHERE id = ?').get(id)
  return row ? deserialize(row) : null
}

export function create(db: Database, input: CreateResumeTemplate): ResumeTemplate {
  const id = crypto.randomUUID()
  db.run(
    `INSERT INTO resume_templates (id, name, description, sections, is_builtin)
     VALUES (?, ?, ?, ?, 0)`,
    [id, input.name, input.description ?? null, JSON.stringify(input.sections)]
  )
  return get(db, id)!
}

export function update(db: Database, id: string, patch: UpdateResumeTemplate): ResumeTemplate | null {
  // Build dynamic UPDATE
  // ...
}

export function remove(db: Database, id: string): boolean {
  const template = get(db, id)
  if (!template) return false
  db.run('DELETE FROM resume_templates WHERE id = ?', [id])
  return true
}

// Note: Built-in protection is enforced at the service layer, not the repository.
// This allows admin/migration code to bypass the check if needed.
// TemplateService.delete() checks `is_builtin` and returns an error before calling remove().

// Deserialize JSON sections column. Note: is_builtin stays as 0 | 1 (no boolean conversion).
function deserialize(row: any): ResumeTemplate {
  return {
    ...row,
    sections: JSON.parse(row.sections),
  }
}
```

## 4. Service Layer

**File:** `packages/core/src/services/template-service.ts` (or methods on `ResumeService`)

### 4.1 CRUD

Standard CRUD operations wrapping the repository.

**Validation on create/update:**
- `name` must be non-empty
- `sections` must be a non-empty array
- Each section must have a valid `entry_type` (one of the values in the `resume_sections` CHECK constraint)
- Each section must have a `title` (non-empty string)
- Positions must be sequential starting from 0 (or auto-assigned if not provided)

> **`entry_type` validation:** Validation uses the DB CHECK constraint values, NOT `IRSectionType`. The valid set is: `experience, skills, education, projects, clearance, presentations, certifications, awards, freeform`. Values like `custom` and `summary` are in `IRSectionType` for backward compat but are NOT valid `entry_type` values for sections.

### 4.2 Save as Template

Creates a template from an existing resume's section layout:

```typescript
saveAsTemplate(resumeId: string, name: string, description?: string): Result<ResumeTemplate> {
  const sections = ResumeRepository.listSections(this.db, resumeId)
  if (!sections.length) {
    return { ok: false, error: { code: 'VALIDATION_ERROR', message: 'Resume has no sections' } }
  }

  const templateSections: TemplateSectionDef[] = sections.map(s => ({
    title: s.title,
    entry_type: s.entry_type,
    position: s.position,
  }))

  return this.createTemplate({ name, description, sections: templateSections })
}
```

### 4.3 Apply Template on Resume Creation

When creating a resume with a `template_id`, the service creates the resume and then creates `resume_sections` rows matching the template's layout:

```typescript
createResumeFromTemplate(
  input: CreateResume & { template_id: string }
): Result<Resume> {
  const template = TemplateRepository.get(this.db, input.template_id)
  if (!template) {
    return { ok: false, error: { code: 'NOT_FOUND', message: 'Template not found' } }
  }

  // Atomic: create resume + sections in a single transaction.
  // If any step fails, the entire operation rolls back (no partial data).
  const txn = this.db.transaction(() => {
    const resumeResult = this.createResume(input)
    if (!resumeResult.ok) return resumeResult

    for (const section of template.sections) {
      ResumeRepository.createSection(this.db, resumeResult.data.id, {
        title: section.title,
        entry_type: section.entry_type,
        position: section.position,
      })
    }

    return resumeResult
  })

  return txn()
}
```

> **Why `db.transaction()`?** The codebase uses `db.transaction()` (better-sqlite3 / bun:sqlite helper) for atomic multi-step writes. See `source-repository.ts`, `resume-repository.ts`, and `derivation-service.ts` for precedent. This wraps the callback in `BEGIN IMMEDIATE` / `COMMIT` with automatic `ROLLBACK` on throw.

> **Note:** `createResumeFromTemplate()` returns the bare `Resume`, not `ResumeWithEntries`. Callers that need sections should call `getResume()` after creation.

## 5. API Endpoints

### 5.1 Template CRUD

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/templates` | List all templates |
| `GET` | `/api/templates/:id` | Get a template |
| `POST` | `/api/templates` | Create a template |
| `PATCH` | `/api/templates/:id` | Update a template |
| `DELETE` | `/api/templates/:id` | Delete a template (fails for built-in) |

### 5.2 Save as Template

**`POST /api/resumes/:id/save-as-template`**

Request body:
```json
{
  "name": "My Custom Layout",
  "description": "The layout I use for security engineer roles"
}
```

Response: `{ ok: true, data: ResumeTemplate }`

### 5.3 Create Resume from Template

The existing `POST /api/resumes` endpoint gains an optional `template_id` field:

```json
{
  "name": "Security Engineer - Acme Corp",
  "target_role": "Senior Security Engineer",
  "target_employer": "Acme Corp",
  "archetype": "security-engineer",
  "template_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

If `template_id` is provided, sections are created from the template. If omitted, the resume starts with no sections (current behavior).

## 6. SDK

**File:** `packages/sdk/src/index.ts`

```typescript
templates = {
  list: () => this.request<ResumeTemplate[]>('GET', '/api/templates'),
  get: (id: string) => this.request<ResumeTemplate>('GET', `/api/templates/${id}`),
  create: (data: CreateResumeTemplate) => this.request<ResumeTemplate>('POST', '/api/templates', data),
  update: (id: string, data: UpdateResumeTemplate) => this.request<ResumeTemplate>('PATCH', `/api/templates/${id}`, data),
  delete: (id: string) => this.request<void>('DELETE', `/api/templates/${id}`),
}

// On the resumes namespace:
resumes = {
  // ... existing methods ...
  saveAsTemplate: (id: string, data: { name: string; description?: string }) =>
    this.request<ResumeTemplate>('POST', `/api/resumes/${id}/save-as-template`, data),
}
```

## 7. UI

### 7.1 Templates Management: `/config/templates`

**File:** `packages/webui/src/routes/config/templates/+page.svelte`

**List view:**
- Cards for each template showing name, description, section count
- Built-in templates have a "Built-in" badge
- "Create Template" button opens an inline form or modal
- Each card has Edit and Delete buttons (Delete disabled for built-in)

**Template editor (inline or separate route):**
- Name input
- Description textarea
- Section list with drag-to-reorder (or up/down arrows for MVP)
- Each section row shows: title (editable), entry_type (dropdown), delete button
- "Add Section" button with entry_type dropdown
- Preview panel showing the section layout as it would appear on a resume

```
+----------------------------------------------+
| Standard Tech Resume            [Built-in]   |
| Classic layout for software engineering...    |
|                                               |
| Sections:                                     |
|   0. Summary (freeform)                       |
|   1. Experience (experience)                  |
|   2. Technical Skills (skills)                |
|   3. Education & Certifications (education)   |
|   4. Selected Projects (projects)             |
|                                               |
| [Edit]  [Duplicate]                           |
+----------------------------------------------+
```

### 7.2 Template Picker in Resume Creation

When the user clicks "Create Resume", the flow becomes:

1. **Step 1: Pick a template** -- grid of template cards. "Blank Resume" is always the first option (no template). Each card shows the template name, description, and a miniature section list.
2. **Step 2: Fill in resume details** -- name, target_role, target_employer, archetype (same as current).
3. **Create** -- if a template was selected, sections are pre-populated.

This can be a two-step modal or a single form with a template dropdown at the top.

### 7.3 "Save as Template" on Resume Builder

**File:** `packages/webui/src/routes/resumes/[id]/+page.svelte`

A button in the resume builder toolbar:

```svelte
<button class="btn btn-sm btn-ghost" onclick={() => showSaveAsTemplate = true}>
  Save as Template
</button>
```

Opens a small modal asking for template name and optional description. On submit, calls `POST /api/resumes/:id/save-as-template`.

## 8. Relationship to Summary Templates (Future)

This spec covers **resume templates** (section layout). There is a separate concept of **summary templates** which define the title/role/tagline/description for the resume header. A full resume creation flow could be:

1. Pick a **resume template** (section layout: "Standard Tech Resume")
2. Pick a **summary template** (header content: "Cloud Security Specialist")
3. Resume is created with sections from the resume template + header from the summary template
4. User fills in entries per section

Summary templates are out of scope for this spec but the architecture does not conflict. Summary templates would be a separate table and a separate step in the creation flow.

## Pre-requisites

**BLOCKING PRE-REQUISITE:** Before seeding the Academic CV template, add the following cases to `buildSectionItems()` in `resume-compiler.ts`:

```typescript
case 'awards':          return buildFreeformItems(db, section.id)
case 'certifications':  return buildCertificationItems(db, section.id)
```

Without these, any resume created from templates using these entry types will have empty sections (the current `default: return []` swallows them silently).

**`certifications` entry_type and `CertificationGroup`:** The `certifications` entry type is NOT the same as freeform -- it uses the existing `CertificationGroup` type (defined in `packages/core/src/types/index.ts`, `kind: 'certification_group'`), which has structured `categories` with `label` and `certs` arrays. The `buildCertificationItems()` function must be implemented to return `CertificationGroup` items. This type already exists in the codebase and does not conflict with any other type.

The full set of entry types the compiler must handle after this change:

| `entry_type` | Builder function | Return type |
|---|---|---|
| `experience` | `buildExperienceItems` | `ExperienceItem[]` |
| `skills` | `buildSkillItems` | `SkillGroup[]` |
| `education` | `buildEducationItems` | `EducationItem[]` |
| `projects` | `buildProjectItems` | `ProjectItem[]` |
| `clearance` | `buildClearanceItems` | `ClearanceItem[]` |
| `presentations` | `buildPresentationItems` | `PresentationItem[]` |
| `certifications` | `buildCertificationItems` | `CertificationGroup[]` |
| `awards` | `buildFreeformItems` | `FreeformItem[]` |
| `freeform` | `buildFreeformItems` | `FreeformItem[]` |

## Files to Create

| File | Description |
|------|-------------|
| `packages/core/src/db/migrations/008_resume_templates.sql` | Schema migration with seed data |
| `packages/core/src/db/repositories/template-repository.ts` | Template CRUD repository |
| `packages/core/src/services/template-service.ts` | Template service (or methods on ResumeService) |
| `packages/core/src/routes/templates.ts` | Route handler for `/api/templates/*` endpoints. Register in `server.ts`. |
| `packages/webui/src/routes/config/templates/+page.svelte` | Templates management page |

## Files to Modify

| File | Change |
|------|--------|
| `packages/core/src/types/index.ts` | Add `ResumeTemplate`, `TemplateSectionDef`, `CreateResumeTemplate`, `UpdateResumeTemplate` |
| `packages/core/src/services/resume-compiler.ts` | Add `case 'awards': return buildFreeformItems(...)` and `case 'certifications': return buildCertificationItems(...)` to `buildSectionItems` switch |
| `packages/core/src/services/resume-service.ts` | Add `saveAsTemplate()`, `createResumeFromTemplate()` methods |
| `packages/core/src/services/index.ts` | Export template service |
| `packages/sdk/src/index.ts` | Add `templates` namespace and `resumes.saveAsTemplate()` |
| `packages/webui/src/routes/resumes/[id]/+page.svelte` | Add "Save as Template" button |
| Resume creation flow (likely `packages/webui/src/routes/resumes/+page.svelte`) | Add template picker step |
| `packages/core/src/server.ts` | Register templates router from `packages/core/src/routes/templates.ts` |

## Testing

- Verify migration creates `resume_templates` table and seeds three built-in templates
- Verify built-in templates have correct section layouts
- Verify `POST /api/templates` creates a user template with valid sections
- Verify `DELETE /api/templates/:id` fails for built-in templates
- Verify `POST /api/resumes/:id/save-as-template` captures the resume's current section layout
- Verify creating a resume with `template_id` pre-populates sections
- Verify creating a resume without `template_id` starts with no sections (backward compatible)
- Verify the template editor allows adding/removing/reordering sections
- Verify the template picker shows all templates during resume creation
- Verify sections JSON is properly serialized/deserialized (round-trip)
- Verify entry_type validation rejects invalid values
- Verify creating a resume with nonexistent `template_id` returns NOT_FOUND
- Verify `saveAsTemplate()` on a resume with no sections returns VALIDATION_ERROR
- Verify deleting a built-in template returns 400/403 error
- Verify `entry_type` validation rejects `custom` and `summary` values
- Verify creating a resume without `template_id` results in zero sections
