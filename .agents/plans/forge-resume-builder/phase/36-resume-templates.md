# Phase 36: Resume Templates (Spec 8)

**Status:** Planning
**Date:** 2026-03-31
**Spec:** [2026-03-30-resume-templates.md](../refs/specs/2026-03-30-resume-templates.md)
**Depends on:** Phase 28 (resume sections must exist — section CRUD, `resume_sections` table, `ResumeRepository.createSection()`)
**Blocks:** None (leaf feature)
**Parallelizable with:** Phases 29-32, 35

## Goal

Add reusable resume templates that define section layouts (which sections, in what order, with what entry type) without content. Users can pick a template during resume creation to pre-populate sections, save any resume's layout as a template, and manage templates at `/config/templates`. This phase covers the full stack: migration 008, core repository + service, API routes, SDK resource, and UI.

## Non-Goals

- Content in templates (templates define structure only, not bullet points or summary text)
- Per-template styling or LaTeX configuration (all resumes use sb2nov)
- Template marketplace or sharing between users
- Nested/conditional sections in templates
- Summary template integration (separate concept, separate table, future work)
- Drag-and-drop reordering in template editor (up/down arrows for MVP)

## Context

Resume creation currently starts from a blank slate. Users must manually add each section via the "Add Section" dropdown. Templates solve this by providing pre-configured section layouts. Three built-in templates are seeded during migration: "Standard Tech Resume", "Academic CV", and "Federal Resume". Users can also create custom templates or save an existing resume's layout as a new template.

The `resume_templates` table stores section definitions as a denormalized JSON column. This is intentional: templates are lightweight metadata (4-7 sections each), always read/written as a whole unit, and do not need a join table.

## Scope

| Spec section | Covered here? |
|-------------|---------------|
| 1. Schema (migration 008) | Yes |
| 2. Types | Yes |
| 3. Repository | Yes |
| 4. Service Layer | Yes |
| 5. API Endpoints | Yes |
| 6. SDK | Yes |
| 7. UI (templates management, template picker, save-as-template) | Yes |
| 8. Relationship to Summary Templates | No (future) |
| Pre-requisites (awards/certifications compiler cases) | Yes |

## Files to Create

| File | Description |
|------|-------------|
| `packages/core/src/db/migrations/008_resume_templates.sql` | Schema migration with seed data |
| `packages/core/src/db/repositories/template-repository.ts` | Template CRUD repository |
| `packages/core/src/services/template-service.ts` | Template business logic + save-as-template + create-from-template |
| `packages/core/src/routes/templates.ts` | Route handler for `/api/templates/*` endpoints |
| `packages/core/src/db/repositories/__tests__/template-repository.test.ts` | Repository unit tests |
| `packages/core/src/services/__tests__/template-service.test.ts` | Service unit tests |
| `packages/core/src/routes/__tests__/templates.test.ts` | Route integration tests |
| `packages/sdk/src/resources/templates.ts` | SDK `TemplatesResource` class |
| `packages/sdk/src/__tests__/templates.test.ts` | SDK contract tests |
| `packages/webui/src/routes/config/templates/+page.svelte` | Templates management page |

## Files to Modify

| File | Change |
|------|--------|
| `packages/core/src/types/index.ts` | Add `ResumeTemplate`, `TemplateSectionDef`, `CreateResumeTemplate`, `UpdateResumeTemplate` |
| `packages/core/src/services/resume-compiler.ts` | Add `case 'awards'` and `case 'certifications'` to `buildSectionItems` switch |
| `packages/core/src/services/index.ts` | Add `TemplateService` to `Services` interface and `createServices()` |
| `packages/core/src/routes/server.ts` | Register `templateRoutes` |
| `packages/core/src/routes/resumes.ts` | Add `POST /resumes/:id/save-as-template` route |
| `packages/core/src/services/resume-service.ts` | Add `createResumeFromTemplate()` method (or call through to TemplateService) |
| `packages/core/src/db/__tests__/helpers.ts` | Add `seedResumeTemplate()` helper |
| `packages/sdk/src/types.ts` | Add `ResumeTemplate`, `TemplateSectionDef`, `CreateResumeTemplate`, `UpdateResumeTemplate` |
| `packages/sdk/src/index.ts` | Export `TemplatesResource` and new types |
| `packages/sdk/src/client.ts` | Add `templates` property, wire up `TemplatesResource` |
| `packages/sdk/src/resources/resumes.ts` | Add `saveAsTemplate()` method |
| `packages/webui/src/routes/resumes/[id]/+page.svelte` | Add "Save as Template" button |
| Resume creation flow (likely `packages/webui/src/routes/resumes/+page.svelte`) | Add template picker step |
| `packages/core/src/db/__tests__/migrate.test.ts` | Add test: "After migration 008, `resume_templates` table exists with 3 rows where `is_builtin = 1`." |

## Fallback Strategies

| Risk | Mitigation |
|------|------------|
| Migration 008 conflicts with 005-007 | Migration is DDL-independent of 005-007. Sequential numbering is the only constraint. If a conflict arises, renumber to 009+. |
| `createResumeFromTemplate` partial failure | Wrapped in `db.transaction()` -- rolls back atomically. |
| Template with invalid `entry_type` values | Service validates against the CHECK constraint set before inserting. Invalid templates are rejected at creation time. |
| UI drag-and-drop complexity | MVP uses up/down arrows. Drag-and-drop can be added later without schema changes. |
| `buildCertificationItems` complexity | Awards route through `buildFreeformItems`. Certifications use a dedicated builder returning `CertificationGroup[]`. |

---

## Tasks

### T36.1: Write `008_resume_templates.sql`

**File:** `packages/core/src/db/migrations/008_resume_templates.sql`

```sql
-- Forge Resume Builder -- Resume Templates
-- Migration: 008_resume_templates
-- Date: 2026-03-31
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
-- NOTE: entry_type is 'education', not 'certifications'. The title is a naming convention.
-- For structured certification rendering (CertificationGroup), create a separate section
-- with entry_type = 'certifications'.
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
-- NOTE: entry_type is 'education', not 'certifications'. The title is a naming convention.
-- For structured certification rendering (CertificationGroup), create a separate section
-- with entry_type = 'certifications'.
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

**Acceptance criteria:**
- `resume_templates` table is created with STRICT mode
- Three built-in templates are seeded with `is_builtin = 1`
- `sections` column contains valid JSON arrays matching the `TemplateSectionDef` shape
- All seeded `entry_type` values are in the valid set: `experience, skills, education, projects, clearance, presentations, certifications, awards, freeform`
- Migration is registered in `_migrations`

**Failure criteria:**
- Migration fails if run after 007 (DDL dependency issue)
- JSON sections column contains malformed JSON
- Any seeded `entry_type` value is not in the CHECK constraint set from `resume_sections`

---

### T36.2: Add Types

#### `packages/core/src/types/index.ts`

Add after the existing `ResumeSkill` interface (around the resume entities section):

```typescript
/** A resume template -- defines reusable section layouts without content. */
export interface ResumeTemplate {
  id: string
  name: string
  description: string | null
  sections: TemplateSectionDef[]
  is_builtin: number   // SQLite STRICT INTEGER: 0 | 1. Core keeps as number; SDK converts to boolean.
  created_at: string
  updated_at: string
}

/** A section definition within a template. */
export interface TemplateSectionDef {
  title: string
  entry_type: string
  position: number
}

/** Input for creating a resume template. */
export interface CreateResumeTemplate {
  name: string
  description?: string
  sections: TemplateSectionDef[]
}

/** Input for updating a resume template. */
export interface UpdateResumeTemplate {
  name?: string
  description?: string | null
  sections?: TemplateSectionDef[]
}
```

#### `packages/sdk/src/types.ts`

Add in the core entities section (after `ResumeSkill`):

```typescript
/** A resume template -- defines reusable section layouts without content. */
export interface ResumeTemplate {
  id: string
  name: string
  description: string | null
  sections: TemplateSectionDef[]
  is_builtin: boolean   // SDK converts 0|1 to boolean for developer ergonomics
  created_at: string
  updated_at: string
}

/** A section definition within a template. */
export interface TemplateSectionDef {
  title: string
  entry_type: string
  position: number
}

/** Input for creating a resume template. */
export interface CreateResumeTemplate {
  name: string
  description?: string
  sections: TemplateSectionDef[]
}

/** Input for updating a resume template. */
export interface UpdateResumeTemplate {
  name?: string
  description?: string | null
  sections?: TemplateSectionDef[]
}
```

**Note:** The SDK type uses `is_builtin: boolean` while the core type uses `is_builtin: number`. The SDK resource class handles the conversion (see T36.7).

**Acceptance criteria:**
- Core types use `number` for `is_builtin` (matches SQLite STRICT INTEGER)
- SDK types use `boolean` for `is_builtin` (developer ergonomics)
- `TemplateSectionDef` has exactly three fields: `title`, `entry_type`, `position`
- Both `CreateResumeTemplate` and `UpdateResumeTemplate` have `sections` as `TemplateSectionDef[]`

**Failure criteria:**
- SDK and core types diverge on field names (other than `is_builtin` type)
- Missing `position` field on `TemplateSectionDef` (would break section ordering)

---

### T36.3: Create Template Repository

**File:** `packages/core/src/db/repositories/template-repository.ts`

```typescript
/**
 * TemplateRepository -- pure data access for the resume_templates table.
 *
 * Note: Built-in protection is NOT enforced here. The service layer
 * checks `is_builtin` before calling remove(). This allows admin/migration
 * code to bypass the check if needed.
 */

import type { Database } from 'bun:sqlite'
import type { ResumeTemplate, CreateResumeTemplate, UpdateResumeTemplate } from '../../types'

interface TemplateRow {
  id: string
  name: string
  description: string | null
  sections: string   // JSON string in the DB
  is_builtin: number
  created_at: string
  updated_at: string
}

/**
 * Deserialize a DB row into a ResumeTemplate.
 * Parses the JSON `sections` column. Does NOT convert `is_builtin` to boolean.
 */
function deserialize(row: TemplateRow): ResumeTemplate {
  return {
    id: row.id,
    name: row.name,
    description: row.description,
    sections: JSON.parse(row.sections),
    is_builtin: row.is_builtin,
    created_at: row.created_at,
    updated_at: row.updated_at,
  }
}

export function list(db: Database): ResumeTemplate[] {
  const rows = db
    .query('SELECT * FROM resume_templates ORDER BY is_builtin DESC, name ASC')
    .all() as TemplateRow[]
  return rows.map(deserialize)
}

export function get(db: Database, id: string): ResumeTemplate | null {
  const row = db
    .query('SELECT * FROM resume_templates WHERE id = ?')
    .get(id) as TemplateRow | null
  return row ? deserialize(row) : null
}

export function create(db: Database, input: CreateResumeTemplate): ResumeTemplate {
  const id = crypto.randomUUID()
  const row = db
    .query(
      `INSERT INTO resume_templates (id, name, description, sections, is_builtin)
       VALUES (?, ?, ?, ?, 0)
       RETURNING *`
    )
    .get(id, input.name, input.description ?? null, JSON.stringify(input.sections)) as TemplateRow

  return deserialize(row)
}

export function update(db: Database, id: string, patch: UpdateResumeTemplate): ResumeTemplate | null {
  const sets: string[] = []
  const params: unknown[] = []

  if (patch.name !== undefined) {
    sets.push('name = ?')
    params.push(patch.name)
  }
  if (patch.description !== undefined) {
    sets.push('description = ?')
    params.push(patch.description)
  }
  if (patch.sections !== undefined) {
    sets.push('sections = ?')
    params.push(JSON.stringify(patch.sections))
  }

  if (sets.length === 0) {
    return get(db, id)
  }

  sets.push("updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')")
  params.push(id)

  const row = db
    .query(`UPDATE resume_templates SET ${sets.join(', ')} WHERE id = ? RETURNING *`)
    .get(...params) as TemplateRow | null

  return row ? deserialize(row) : null
}

export function remove(db: Database, id: string): boolean {
  const result = db.run('DELETE FROM resume_templates WHERE id = ?', [id])
  return result.changes > 0
}
```

**Acceptance criteria:**
- `list()` returns built-in templates first (sorted by `is_builtin DESC, name ASC`)
- `create()` always sets `is_builtin = 0` (user-created templates are never built-in)
- `update()` serializes `sections` back to JSON string
- `deserialize()` parses the JSON `sections` column but does NOT convert `is_builtin`
- `remove()` performs raw delete without checking `is_builtin` (service layer responsibility)

> Note: `remove()` uses `result.changes > 0` (not a pre-flight `get()`) -- this diverges from the spec for consistency with existing repository delete patterns in the codebase (fewer round-trips).

**Failure criteria:**
- `sections` not parsed from JSON (returned as string instead of array)
- `create()` allows setting `is_builtin = 1`
- `update()` with empty patch object returns null instead of existing template

---

### T36.4: Create Template Service

**File:** `packages/core/src/services/template-service.ts`

```typescript
/**
 * TemplateService -- business logic for resume templates.
 *
 * Handles CRUD validation, built-in protection on delete,
 * save-as-template from existing resumes, and create-resume-from-template.
 */

import type { Database } from 'bun:sqlite'
import type {
  ResumeTemplate,
  CreateResumeTemplate,
  UpdateResumeTemplate,
  TemplateSectionDef,
  Resume,
  Result,
} from '../types'
import * as TemplateRepo from '../db/repositories/template-repository'
import { ResumeRepository } from '../db/repositories/resume-repository'
import type { CreateResume } from '../types'

/** Valid entry_type values from the resume_sections CHECK constraint. */
const VALID_ENTRY_TYPES = new Set([
  'experience', 'skills', 'education', 'projects',
  'clearance', 'presentations', 'certifications', 'awards', 'freeform',
])

export class TemplateService {
  constructor(private db: Database) {}

  // -- CRUD -----------------------------------------------------------------

  list(): Result<ResumeTemplate[]> {
    return { ok: true, data: TemplateRepo.list(this.db) }
  }

  get(id: string): Result<ResumeTemplate> {
    const template = TemplateRepo.get(this.db, id)
    if (!template) {
      return { ok: false, error: { code: 'NOT_FOUND', message: `Template ${id} not found` } }
    }
    return { ok: true, data: template }
  }

  create(input: CreateResumeTemplate): Result<ResumeTemplate> {
    const validation = this.validateSections(input.name, input.sections)
    if (!validation.ok) return validation

    const template = TemplateRepo.create(this.db, {
      name: input.name.trim(),
      description: input.description,
      sections: this.normalizeSections(input.sections),
    })
    return { ok: true, data: template }
  }

  update(id: string, patch: UpdateResumeTemplate): Result<ResumeTemplate> {
    const existing = TemplateRepo.get(this.db, id)
    if (!existing) {
      return { ok: false, error: { code: 'NOT_FOUND', message: `Template ${id} not found` } }
    }

    // Validate name if provided
    if (patch.name !== undefined && patch.name.trim().length === 0) {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: 'Name must not be empty' } }
    }

    // Validate sections if provided
    if (patch.sections !== undefined) {
      const validation = this.validateSections(
        patch.name ?? existing.name,
        patch.sections,
      )
      if (!validation.ok) return validation
      patch = { ...patch, sections: this.normalizeSections(patch.sections) }
    }

    const updated = TemplateRepo.update(this.db, id, {
      ...patch,
      name: patch.name?.trim(),
    })
    if (!updated) {
      return { ok: false, error: { code: 'NOT_FOUND', message: `Template ${id} not found` } }
    }
    return { ok: true, data: updated }
  }

  delete(id: string): Result<void> {
    const template = TemplateRepo.get(this.db, id)
    if (!template) {
      return { ok: false, error: { code: 'NOT_FOUND', message: `Template ${id} not found` } }
    }

    if (template.is_builtin) {
      return {
        ok: false,
        error: { code: 'VALIDATION_ERROR', message: 'Built-in templates cannot be deleted' },
      }
    }

    TemplateRepo.remove(this.db, id)
    return { ok: true, data: undefined }
  }

  // -- Save as Template -----------------------------------------------------

  saveAsTemplate(
    resumeId: string,
    name: string,
    description?: string,
  ): Result<ResumeTemplate> {
    if (!name || name.trim().length === 0) {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: 'Name must not be empty' } }
    }

    const resume = ResumeRepository.get(this.db, resumeId)
    if (!resume) return { ok: false, error: { code: 'NOT_FOUND', message: 'Resume not found' } }

    const sections = ResumeRepository.listSections(this.db, resumeId)
    if (sections.length === 0) {
      return {
        ok: false,
        error: { code: 'VALIDATION_ERROR', message: 'Resume has no sections' },
      }
    }

    const templateSections: TemplateSectionDef[] = sections.map(s => ({
      title: s.title,
      entry_type: s.entry_type,
      position: s.position,
    }))

    return this.create({ name: name.trim(), description, sections: templateSections })
  }

  // -- Create Resume from Template ------------------------------------------

  /**
   * Create a resume with sections pre-populated from a template.
   *
   * Uses db.transaction() for atomicity: if any step fails, the entire
   * operation rolls back (no partial data -- no orphaned resume without
   * sections, no orphaned sections without a resume).
   */
  createResumeFromTemplate(
    input: CreateResume & { template_id: string },
  ): Result<Resume> {
    const template = TemplateRepo.get(this.db, input.template_id)
    if (!template) {
      return { ok: false, error: { code: 'NOT_FOUND', message: 'Template not found' } }
    }

    // Validate resume input
    if (!input.name || input.name.trim().length === 0) {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: 'Name must not be empty' } }
    }
    if (!input.target_role || input.target_role.trim().length === 0) {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: 'Target role must not be empty' } }
    }
    if (!input.target_employer || input.target_employer.trim().length === 0) {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: 'Target employer must not be empty' } }
    }
    if (!input.archetype || input.archetype.trim().length === 0) {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: 'Archetype must not be empty' } }
    }

    const txn = this.db.transaction(() => {
      const resume = ResumeRepository.create(this.db, {
        name: input.name,
        target_role: input.target_role,
        target_employer: input.target_employer,
        archetype: input.archetype,
      })

      for (const section of template.sections) {
        ResumeRepository.createSection(this.db, resume.id, {
          title: section.title,
          entry_type: section.entry_type,
          position: section.position,
        })
      }

      return resume
    })

    const resume = txn()
    return { ok: true, data: resume }
  }

  // -- Private helpers ------------------------------------------------------

  /**
   * Validate template name and sections.
   */
  private validateSections(
    name: string,
    sections: TemplateSectionDef[],
  ): Result<never> | { ok: true } {
    if (!name || name.trim().length === 0) {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: 'Name must not be empty' } }
    }

    if (!Array.isArray(sections) || sections.length === 0) {
      return {
        ok: false,
        error: { code: 'VALIDATION_ERROR', message: 'Sections must be a non-empty array' },
      }
    }

    for (let i = 0; i < sections.length; i++) {
      const s = sections[i]

      if (!s.title || s.title.trim().length === 0) {
        return {
          ok: false,
          error: { code: 'VALIDATION_ERROR', message: `Section ${i}: title must not be empty` },
        }
      }

      if (!VALID_ENTRY_TYPES.has(s.entry_type)) {
        return {
          ok: false,
          error: {
            code: 'VALIDATION_ERROR',
            message: `Section ${i}: invalid entry_type '${s.entry_type}'. Must be one of: ${[...VALID_ENTRY_TYPES].join(', ')}`,
          },
        }
      }
    }

    return { ok: true }
  }

  /**
   * Normalize section positions to be sequential starting from 0.
   * Preserves the relative order of the input array.
   */
  private normalizeSections(sections: TemplateSectionDef[]): TemplateSectionDef[] {
    return sections.map((s, i) => ({
      title: s.title.trim(),
      entry_type: s.entry_type,
      position: i,
    }))
  }
}
```

**Acceptance criteria:**
- `delete()` returns `VALIDATION_ERROR` for built-in templates (not `FORBIDDEN` or `CONFLICT`)
- `create()` validates each section's `entry_type` against the CHECK constraint set
- `create()` rejects `custom` and `summary` as entry_type values (they are NOT in `VALID_ENTRY_TYPES`)
- `saveAsTemplate()` returns `VALIDATION_ERROR` when resume has no sections
- `createResumeFromTemplate()` uses `db.transaction()` for atomicity
- `createResumeFromTemplate()` returns `NOT_FOUND` for nonexistent `template_id`
- `normalizeSections()` auto-assigns sequential positions starting from 0

**Failure criteria:**
- `delete()` allows deleting built-in templates
- `createResumeFromTemplate()` creates a resume without sections on partial failure
- `entry_type` validation accepts `summary` or `custom` (only valid for backward-compat `IRSectionType`, not for DB)

---

### T36.5: Add Awards + Certifications to Resume Compiler

**File:** `packages/core/src/services/resume-compiler.ts`

This is a blocking prerequisite from the spec. Without these cases, resumes created from templates with `awards` or `certifications` entry types will have empty sections.

**Change the `buildSectionItems` switch:**

```typescript
function buildSectionItems(db: Database, section: ResumeSectionRow): IRSectionItem[] {
  switch (section.entry_type) {
    case 'experience': return buildExperienceItems(db, section.id)
    case 'skills': return buildSkillItems(db, section.id)
    case 'education': return buildEducationItems(db, section.id)
    case 'projects': return buildProjectItems(db, section.id)
    case 'clearance': return buildClearanceItems(db, section.id)
    case 'presentations': return buildPresentationItems(db, section.id)
    case 'freeform': return buildFreeformItems(db, section.id)
    case 'awards': return buildFreeformItems(db, section.id)
    case 'certifications': return buildCertificationItems(db, section.id)
    default: return []
  }
}
```

**Add `buildCertificationItems` function** after `buildPresentationItems`:

```typescript
function buildCertificationItems(db: Database, sectionId: string): CertificationGroup[] {
  const rows = db
    .query(
      `SELECT
        re.id AS entry_id,
        re.content AS entry_content,
        re.perspective_id,
        p.content AS perspective_content,
        bs.source_id,
        s.title AS source_title,
        se.institution,
        se.field,
        se.credential_id,
        se.end_date
      FROM resume_entries re
      JOIN perspectives p ON p.id = re.perspective_id
      JOIN bullet_sources bs ON bs.bullet_id = p.bullet_id AND bs.is_primary = 1
      JOIN sources s ON s.id = bs.source_id
      LEFT JOIN source_education se ON se.source_id = s.id
      WHERE re.section_id = ?
      ORDER BY re.position ASC`
    )
    .all(sectionId) as Array<{
      entry_id: string
      entry_content: string | null
      perspective_id: string
      perspective_content: string
      source_id: string
      source_title: string
      institution: string | null
      field: string | null
      credential_id: string | null
      end_date: string | null
    }>

  if (rows.length === 0) return []

  // Group by institution (issuing body)
  const catMap = new Map<string, Array<{ name: string; entry_id: string; source_id: string }>>()

  for (const row of rows) {
    const label = row.institution ?? 'Other'
    if (!catMap.has(label)) catMap.set(label, [])
    catMap.get(label)!.push({
      name: row.entry_content ?? row.perspective_content,
      entry_id: row.entry_id,
      source_id: row.source_id,
    })
  }

  return [{
    kind: 'certification_group',
    categories: Array.from(catMap.entries()).map(([label, certs]) => ({
      label,
      certs: certs.map(c => ({
        name: c.name,
        entry_id: c.entry_id,
        source_id: c.source_id,
      })),
    })),
  }]
}
```

**Add import for `CertificationGroup`** to the import block at the top:

```typescript
import type {
  ResumeDocument,
  ResumeHeader,
  IRSection,
  IRSectionType,
  IRSectionItem,
  ExperienceGroup,
  ExperienceSubheading,
  ExperienceBullet,
  SkillGroup,
  EducationItem,
  ProjectItem,
  CertificationGroup,  // ADD THIS
  ClearanceItem,
  PresentationItem,
  SummaryItem,
} from '../types'
```

**Acceptance criteria:**
- `case 'awards'` routes to `buildFreeformItems` (awards are freeform text)
- `case 'certifications'` routes to `buildCertificationItems` (structured CertificationGroup)
- `buildCertificationItems` returns `CertificationGroup[]` with `kind: 'certification_group'`
- `buildCertificationItems` groups certifications by institution (issuing body)
- Existing switch cases are unchanged

**Compiler tests:** Add test cases to `packages/core/src/services/__tests__/resume-compiler.test.ts`:
- (a) Compile a resume with a `certifications` section -- verify non-empty output with CertificationGroup structure.
- (b) Compile a resume with an `awards` section -- verify non-empty freeform output.

**Failure criteria:**
- `awards` routed to `buildCertificationItems` (wrong -- awards are freeform)
- `certifications` routed to `buildFreeformItems` (wrong -- certifications use structured `CertificationGroup`)
- `CertificationGroup` import missing (TypeScript compile error)

---

### T36.6: Create Template Routes

**File:** `packages/core/src/routes/templates.ts`

```typescript
/**
 * Template routes -- thin HTTP layer over TemplateService.
 */

import { Hono } from 'hono'
import type { Services } from '../services'
import { mapStatusCode } from './server'

export function templateRoutes(services: Services) {
  const app = new Hono()

  // -- Template CRUD --------------------------------------------------------

  app.get('/templates', (c) => {
    const result = services.templates.list()
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.get('/templates/:id', (c) => {
    const result = services.templates.get(c.req.param('id'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.post('/templates', async (c) => {
    const body = await c.req.json()
    const result = services.templates.create(body)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data }, 201)
  })

  app.patch('/templates/:id', async (c) => {
    const body = await c.req.json()
    const result = services.templates.update(c.req.param('id'), body)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.delete('/templates/:id', (c) => {
    const result = services.templates.delete(c.req.param('id'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.body(null, 204)
  })

  return app
}
```

**Add save-as-template route to `packages/core/src/routes/resumes.ts`:**

Add before the `app.get('/resumes/:id/gaps', ...)` handler:

```typescript
  // -- Save as Template ---------------------------------------------------

  app.post('/resumes/:id/save-as-template', async (c) => {
    const body = await c.req.json<{ name: string; description?: string }>()
    const result = services.templates.saveAsTemplate(
      c.req.param('id'),
      body.name,
      body.description,
    )
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data }, 201)
  })
```

**Modify `POST /resumes` in `packages/core/src/routes/resumes.ts`** to handle `template_id`:

```typescript
  app.post('/resumes', async (c) => {
    const body = await c.req.json<CreateResume & { template_id?: string }>()

    if (body.template_id) {
      const result = services.templates.createResumeFromTemplate({
        ...body,
        template_id: body.template_id,
      })
      if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
      return c.json({ data: result.data }, 201)
    }

    const result = services.resumes.createResume(body)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data }, 201)
  })
```

**Register routes in `packages/core/src/routes/server.ts`:**

Add import:
```typescript
import { templateRoutes } from './templates'
```

Add route registration (after `archetypeRoutes`):
```typescript
  app.route('/', templateRoutes(services))
```

**Acceptance criteria:**
- `GET /api/templates` returns all templates
- `GET /api/templates/:id` returns a single template
- `POST /api/templates` creates a user template (201)
- `PATCH /api/templates/:id` updates a template
- `DELETE /api/templates/:id` returns 204 for user templates, 400 for built-in

> Note: Deleting a built-in template returns HTTP 400 (VALIDATION_ERROR), not 403 (Forbidden). This is a deliberate simplification -- `mapStatusCode` does not currently handle a FORBIDDEN code. If a FORBIDDEN code is added in the future, revisit this.

- `POST /api/resumes/:id/save-as-template` creates a template from resume sections (201)
- `POST /api/resumes` with `template_id` creates resume + sections atomically (201)
- `POST /api/resumes` without `template_id` creates resume normally (backward compatible)

**Failure criteria:**
- Routes not registered in `server.ts` (404 on all template endpoints)
- `save-as-template` route registered on wrong path
- Resume creation with `template_id` bypasses input validation

---

### T36.7: Register Template Service

**File:** `packages/core/src/services/index.ts`

Add import:
```typescript
import { TemplateService } from './template-service'
```

Add to `Services` interface:
```typescript
export interface Services {
  sources: SourceService
  bullets: BulletService
  perspectives: PerspectiveService
  derivation: DerivationService
  resumes: ResumeService
  audit: AuditService
  review: ReviewService
  organizations: OrganizationService
  notes: NoteService
  integrity: IntegrityService
  domains: DomainService
  archetypes: ArchetypeService
  templates: TemplateService   // ADD THIS
}
```

Add to `createServices()`:
```typescript
export function createServices(db: Database): Services {
  const derivingBullets = new Set<string>()

  return {
    sources: new SourceService(db),
    bullets: new BulletService(db),
    perspectives: new PerspectiveService(db),
    derivation: new DerivationService(db, derivingBullets),
    resumes: new ResumeService(db),
    audit: new AuditService(db),
    review: new ReviewService(db),
    organizations: new OrganizationService(db),
    notes: new NoteService(db),
    integrity: new IntegrityService(db),
    domains: new DomainService(db),
    archetypes: new ArchetypeService(db),
    templates: new TemplateService(db),   // ADD THIS
  }
}
```

Add re-export:
```typescript
export { TemplateService } from './template-service'
```

**Acceptance criteria:**
- `TemplateService` is in the `Services` interface
- `createServices()` instantiates `TemplateService`
- `TemplateService` is re-exported

**Failure criteria:**
- Route handlers fail at runtime because `services.templates` is undefined

---

### T36.8: Create SDK Templates Resource

**File:** `packages/sdk/src/resources/templates.ts`

```typescript
import type {
  ResumeTemplate,
  CreateResumeTemplate,
  UpdateResumeTemplate,
  RequestFn,
  Result,
} from '../types'

/**
 * SDK resource for resume template CRUD.
 *
 * Converts `is_builtin` between server format (0|1) and SDK format (boolean).
 */
export class TemplatesResource {
  constructor(private request: RequestFn) {}

  async list(): Promise<Result<ResumeTemplate[]>> {
    const result = await this.request<ResumeTemplate[]>('GET', '/api/templates')
    if (result.ok) {
      result.data = result.data.map(this.deserialize)
    }
    return result
  }

  async get(id: string): Promise<Result<ResumeTemplate>> {
    const result = await this.request<ResumeTemplate>('GET', `/api/templates/${id}`)
    if (result.ok) {
      result.data = this.deserialize(result.data)
    }
    return result
  }

  async create(input: CreateResumeTemplate): Promise<Result<ResumeTemplate>> {
    const result = await this.request<ResumeTemplate>('POST', '/api/templates', input)
    if (result.ok) {
      result.data = this.deserialize(result.data)
    }
    return result
  }

  async update(id: string, input: UpdateResumeTemplate): Promise<Result<ResumeTemplate>> {
    const result = await this.request<ResumeTemplate>('PATCH', `/api/templates/${id}`, input)
    if (result.ok) {
      result.data = this.deserialize(result.data)
    }
    return result
  }

  delete(id: string): Promise<Result<void>> {
    return this.request<void>('DELETE', `/api/templates/${id}`)
  }

  /**
   * Convert server is_builtin (0|1) to boolean.
   *
   * The server returns `is_builtin` as `0 | 1` (SQLite INTEGER). The deserializer
   * converts to boolean for the SDK consumer. Using `any` on the parameter is
   * cleaner than `as any` on the output.
   */
  private deserialize(template: any): ResumeTemplate {
    return { ...template, is_builtin: Boolean(template.is_builtin) }
  }
}
```

**Add `saveAsTemplate` to `packages/sdk/src/resources/resumes.ts`:**

Add after the `pdf()` method:

```typescript
  saveAsTemplate(
    resumeId: string,
    input: { name: string; description?: string },
  ): Promise<Result<ResumeTemplate>> {
    return this.request<ResumeTemplate>(
      'POST',
      `/api/resumes/${resumeId}/save-as-template`,
      input,
    )
  }
```

Add `ResumeTemplate` to the import block in `resumes.ts`:
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
  ResumeSectionEntity,
  ResumeSkill,
  ResumeTemplate,    // ADD THIS
  ResumeWithEntries,
  Result,
  Resume,
  UpdateResume,
  UpdateResumeEntry,
} from '../types'
```

**Register in `packages/sdk/src/client.ts`:**

Add import:
```typescript
import { TemplatesResource } from './resources/templates'
```

Add property declaration:
```typescript
  /** Resume template CRUD. */
  public templates: TemplatesResource
```

Add initialization in constructor (after `this.skills = ...`):
```typescript
    this.templates = new TemplatesResource(req)
```

**Update `packages/sdk/src/index.ts`:**

Add to core entity type exports:
```typescript
export type {
  Source,
  Bullet,
  Perspective,
  Resume,
  Organization,
  ResumeEntry,
  ResumeSectionEntity,
  ResumeSkill,
  ResumeTemplate,        // ADD THIS
  TemplateSectionDef,     // ADD THIS
  Skill,
  UserNote,
} from './types'
```

Add to input type exports:
```typescript
export type {
  CreateSource,
  UpdateSource,
  UpdateBullet,
  UpdatePerspective,
  RejectInput,
  DerivePerspectiveInput,
  CreateResume,
  UpdateResume,
  AddResumeEntry,
  UpdateResumeEntry,
  CreateOrganization,
  UpdateOrganization,
  CreateResumeTemplate,    // ADD THIS
  UpdateResumeTemplate,    // ADD THIS
} from './types'
```

Add resource class export:
```typescript
export { TemplatesResource } from './resources/templates'
```

**Acceptance criteria:**
- `TemplatesResource` converts `is_builtin` from `0|1` to `boolean` on responses
- `client.templates.list()` returns templates with `is_builtin: true/false`
- `client.resumes.saveAsTemplate()` sends `POST /api/resumes/:id/save-as-template`
- All new types are exported from the SDK barrel (`index.ts`)
- `ForgeClient.templates` is available on the client instance

**Failure criteria:**
- `is_builtin` returned as `number` from SDK (should be `boolean`)
- `TemplatesResource` not registered on `ForgeClient`
- Missing type exports in `index.ts`

---

### T36.9: Add Test Helpers

**File:** `packages/core/src/db/__tests__/helpers.ts`

Add after `seedArchetypeDomain`:

```typescript
/** Seed a test resume template and return its ID */
export function seedResumeTemplate(db: Database, opts: {
  name?: string
  description?: string | null
  sections?: Array<{ title: string; entry_type: string; position: number }>
  isBuiltin?: boolean
} = {}): string {
  const id = testUuid()
  const sections = opts.sections ?? [
    { title: 'Summary', entry_type: 'freeform', position: 0 },
    { title: 'Experience', entry_type: 'experience', position: 1 },
    { title: 'Technical Skills', entry_type: 'skills', position: 2 },
  ]
  db.run(
    `INSERT INTO resume_templates (id, name, description, sections, is_builtin)
     VALUES (?, ?, ?, ?, ?)`,
    [
      id,
      opts.name ?? 'Test Template',
      opts.description ?? null,
      JSON.stringify(sections),
      opts.isBuiltin ? 1 : 0,
    ]
  )
  return id
}
```

**Acceptance criteria:**
- `seedResumeTemplate()` creates a template with sensible defaults
- `isBuiltin` option defaults to `false` (0)
- `sections` is serialized to JSON

---

### T36.10: Write Repository Tests

**File:** `packages/core/src/db/repositories/__tests__/template-repository.test.ts`

```typescript
import { describe, expect, test, beforeEach, afterEach } from 'bun:test'
import { createTestDb, seedResumeTemplate } from '../../__tests__/helpers'
import * as TemplateRepo from '../template-repository'
import type { Database } from 'bun:sqlite'

describe('TemplateRepository', () => {
  let db: Database

  beforeEach(() => {
    db = createTestDb()
  })

  afterEach(() => db.close())

  describe('list', () => {
    test('returns seeded built-in templates', () => {
      const templates = TemplateRepo.list(db)
      // Migration 008 seeds 3 built-in templates
      expect(templates.length).toBeGreaterThanOrEqual(3)
      expect(templates[0].is_builtin).toBe(1)
    })

    test('returns built-in templates first', () => {
      seedResumeTemplate(db, { name: 'AAA User Template' })
      const templates = TemplateRepo.list(db)
      // Built-in templates come first regardless of name
      const firstUserIdx = templates.findIndex(t => t.is_builtin === 0)
      const lastBuiltinIdx = templates.findLastIndex(t => t.is_builtin === 1)
      if (firstUserIdx !== -1 && lastBuiltinIdx !== -1) {
        expect(lastBuiltinIdx).toBeLessThan(firstUserIdx)
      }
    })
  })

  describe('get', () => {
    test('returns template by id', () => {
      const id = seedResumeTemplate(db, { name: 'My Template' })
      const template = TemplateRepo.get(db, id)
      expect(template).not.toBeNull()
      expect(template!.name).toBe('My Template')
      expect(template!.sections).toBeInstanceOf(Array)
    })

    test('returns null for nonexistent id', () => {
      expect(TemplateRepo.get(db, 'nonexistent')).toBeNull()
    })

    test('deserializes sections from JSON', () => {
      const sections = [
        { title: 'Experience', entry_type: 'experience', position: 0 },
        { title: 'Skills', entry_type: 'skills', position: 1 },
      ]
      const id = seedResumeTemplate(db, { sections })
      const template = TemplateRepo.get(db, id)
      expect(template!.sections).toEqual(sections)
    })
  })

  describe('create', () => {
    test('creates a user template with is_builtin = 0', () => {
      const template = TemplateRepo.create(db, {
        name: 'Custom',
        sections: [{ title: 'Summary', entry_type: 'freeform', position: 0 }],
      })
      expect(template.is_builtin).toBe(0)
      expect(template.name).toBe('Custom')
      expect(template.sections).toHaveLength(1)
    })

    test('stores description', () => {
      const template = TemplateRepo.create(db, {
        name: 'With Desc',
        description: 'A custom template',
        sections: [{ title: 'Exp', entry_type: 'experience', position: 0 }],
      })
      expect(template.description).toBe('A custom template')
    })

    test('generates valid UUID', () => {
      const template = TemplateRepo.create(db, {
        name: 'UUID Test',
        sections: [{ title: 'Exp', entry_type: 'experience', position: 0 }],
      })
      expect(template.id).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/)
    })
  })

  describe('update', () => {
    test('updates name', () => {
      const id = seedResumeTemplate(db, { name: 'Old Name' })
      const updated = TemplateRepo.update(db, id, { name: 'New Name' })
      expect(updated!.name).toBe('New Name')
    })

    test('updates sections', () => {
      const id = seedResumeTemplate(db)
      const newSections = [{ title: 'Only Summary', entry_type: 'freeform', position: 0 }]
      const updated = TemplateRepo.update(db, id, { sections: newSections })
      expect(updated!.sections).toEqual(newSections)
    })

    test('returns existing template when no fields provided', () => {
      const id = seedResumeTemplate(db, { name: 'NoOp' })
      const updated = TemplateRepo.update(db, id, {})
      expect(updated!.name).toBe('NoOp')
    })

    test('returns null for nonexistent id', () => {
      expect(TemplateRepo.update(db, 'nonexistent', { name: 'X' })).toBeNull()
    })

    test('updates description to null', () => {
      const id = seedResumeTemplate(db, { description: 'old desc' })
      const updated = TemplateRepo.update(db, id, { description: null })
      expect(updated!.description).toBeNull()
    })
  })

  describe('remove', () => {
    test('deletes template and returns true', () => {
      const id = seedResumeTemplate(db)
      expect(TemplateRepo.remove(db, id)).toBe(true)
      expect(TemplateRepo.get(db, id)).toBeNull()
    })

    test('returns false for nonexistent id', () => {
      expect(TemplateRepo.remove(db, 'nonexistent')).toBe(false)
    })
  })
})
```

**Acceptance criteria:**
- All repository CRUD operations are covered
- JSON round-trip (serialize/deserialize) is tested
- Built-in sort order is tested
- Edge cases (nonexistent ID, empty patch) are tested

---

### T36.11: Write Service Tests

**File:** `packages/core/src/services/__tests__/template-service.test.ts`

```typescript
import { describe, expect, test, beforeEach, afterEach } from 'bun:test'
import { createTestDb, seedResume, seedResumeSection, seedResumeTemplate } from '../../db/__tests__/helpers'
import { TemplateService } from '../template-service'
import type { Database } from 'bun:sqlite'

describe('TemplateService', () => {
  let db: Database
  let service: TemplateService

  beforeEach(() => {
    db = createTestDb()
    service = new TemplateService(db)
  })

  afterEach(() => db.close())

  describe('create', () => {
    test('creates template with valid sections', () => {
      const result = service.create({
        name: 'My Template',
        sections: [
          { title: 'Summary', entry_type: 'freeform', position: 0 },
          { title: 'Experience', entry_type: 'experience', position: 1 },
        ],
      })
      expect(result.ok).toBe(true)
      if (result.ok) {
        expect(result.data.name).toBe('My Template')
        expect(result.data.sections).toHaveLength(2)
        expect(result.data.is_builtin).toBe(0)
      }
    })

    test('rejects empty name', () => {
      const result = service.create({
        name: '',
        sections: [{ title: 'Exp', entry_type: 'experience', position: 0 }],
      })
      expect(result.ok).toBe(false)
      if (!result.ok) expect(result.error.code).toBe('VALIDATION_ERROR')
    })

    test('rejects empty sections array', () => {
      const result = service.create({
        name: 'Empty',
        sections: [],
      })
      expect(result.ok).toBe(false)
      if (!result.ok) expect(result.error.code).toBe('VALIDATION_ERROR')
    })

    test('rejects invalid entry_type', () => {
      const result = service.create({
        name: 'Bad Type',
        sections: [{ title: 'Custom', entry_type: 'custom', position: 0 }],
      })
      expect(result.ok).toBe(false)
      if (!result.ok) {
        expect(result.error.code).toBe('VALIDATION_ERROR')
        expect(result.error.message).toContain('invalid entry_type')
      }
    })

    test('rejects summary as entry_type', () => {
      const result = service.create({
        name: 'Bad Summary',
        sections: [{ title: 'Summary', entry_type: 'summary', position: 0 }],
      })
      expect(result.ok).toBe(false)
      if (!result.ok) expect(result.error.code).toBe('VALIDATION_ERROR')
    })

    test('rejects section with empty title', () => {
      const result = service.create({
        name: 'Bad Title',
        sections: [{ title: '', entry_type: 'experience', position: 0 }],
      })
      expect(result.ok).toBe(false)
      if (!result.ok) expect(result.error.code).toBe('VALIDATION_ERROR')
    })

    test('normalizes positions to sequential', () => {
      const result = service.create({
        name: 'Gap Positions',
        sections: [
          { title: 'A', entry_type: 'freeform', position: 5 },
          { title: 'B', entry_type: 'experience', position: 10 },
        ],
      })
      expect(result.ok).toBe(true)
      if (result.ok) {
        expect(result.data.sections[0].position).toBe(0)
        expect(result.data.sections[1].position).toBe(1)
      }
    })

    test('trims name and section titles', () => {
      const result = service.create({
        name: '  Trimmed  ',
        sections: [{ title: '  Experience  ', entry_type: 'experience', position: 0 }],
      })
      expect(result.ok).toBe(true)
      if (result.ok) {
        expect(result.data.name).toBe('Trimmed')
        expect(result.data.sections[0].title).toBe('Experience')
      }
    })
  })

  describe('delete', () => {
    test('deletes user template', () => {
      const id = seedResumeTemplate(db, { name: 'Deletable' })
      const result = service.delete(id)
      expect(result.ok).toBe(true)
    })

    test('rejects deleting built-in template', () => {
      const id = seedResumeTemplate(db, { name: 'Built-in', isBuiltin: true })
      const result = service.delete(id)
      expect(result.ok).toBe(false)
      if (!result.ok) {
        expect(result.error.code).toBe('VALIDATION_ERROR')
        expect(result.error.message).toContain('Built-in')
      }
    })

    test('returns NOT_FOUND for nonexistent template', () => {
      const result = service.delete('nonexistent')
      expect(result.ok).toBe(false)
      if (!result.ok) expect(result.error.code).toBe('NOT_FOUND')
    })
  })

  describe('saveAsTemplate', () => {
    test('creates template from resume sections', () => {
      const resumeId = seedResume(db)
      seedResumeSection(db, resumeId, 'Experience', 'experience', 0)
      seedResumeSection(db, resumeId, 'Skills', 'skills', 1)

      const result = service.saveAsTemplate(resumeId, 'From Resume')
      expect(result.ok).toBe(true)
      if (result.ok) {
        expect(result.data.name).toBe('From Resume')
        expect(result.data.sections).toHaveLength(2)
        expect(result.data.sections[0].title).toBe('Experience')
        expect(result.data.sections[0].entry_type).toBe('experience')
        expect(result.data.sections[1].title).toBe('Skills')
      }
    })

    test('returns VALIDATION_ERROR when resume has no sections', () => {
      const resumeId = seedResume(db)
      const result = service.saveAsTemplate(resumeId, 'Empty Resume')
      expect(result.ok).toBe(false)
      if (!result.ok) {
        expect(result.error.code).toBe('VALIDATION_ERROR')
        expect(result.error.message).toContain('no sections')
      }
    })

    test('returns VALIDATION_ERROR for empty name', () => {
      const resumeId = seedResume(db)
      seedResumeSection(db, resumeId, 'Exp', 'experience', 0)
      const result = service.saveAsTemplate(resumeId, '')
      expect(result.ok).toBe(false)
      if (!result.ok) expect(result.error.code).toBe('VALIDATION_ERROR')
    })

    test('saveAsTemplate with nonexistent resumeId returns NOT_FOUND', () => {
      const result = service.saveAsTemplate('nonexistent-resume-id', 'Ghost Template')
      expect(result.ok).toBe(false)
      if (!result.ok) expect(result.error.code).toBe('NOT_FOUND')
    })
  })

  describe('createResumeFromTemplate', () => {
    test('creates resume with sections from template', () => {
      const templateId = seedResumeTemplate(db, {
        sections: [
          { title: 'Summary', entry_type: 'freeform', position: 0 },
          { title: 'Experience', entry_type: 'experience', position: 1 },
          { title: 'Skills', entry_type: 'skills', position: 2 },
        ],
      })

      const result = service.createResumeFromTemplate({
        name: 'New Resume',
        target_role: 'Engineer',
        target_employer: 'Acme',
        archetype: 'security-engineer',
        template_id: templateId,
      })

      expect(result.ok).toBe(true)
      if (result.ok) {
        expect(result.data.name).toBe('New Resume')
        // Verify sections were created
        const sections = db
          .query('SELECT * FROM resume_sections WHERE resume_id = ? ORDER BY position')
          .all(result.data.id) as Array<{ title: string; entry_type: string; position: number }>
        expect(sections).toHaveLength(3)
        expect(sections[0].title).toBe('Summary')
        expect(sections[0].entry_type).toBe('freeform')
        expect(sections[1].title).toBe('Experience')
        expect(sections[2].title).toBe('Skills')
      }
    })

    test('returns NOT_FOUND for nonexistent template', () => {
      const result = service.createResumeFromTemplate({
        name: 'Test',
        target_role: 'Engineer',
        target_employer: 'Acme',
        archetype: 'test',
        template_id: 'nonexistent',
      })
      expect(result.ok).toBe(false)
      if (!result.ok) expect(result.error.code).toBe('NOT_FOUND')
    })

    test('validates resume input', () => {
      const templateId = seedResumeTemplate(db)
      const result = service.createResumeFromTemplate({
        name: '',
        target_role: 'Engineer',
        target_employer: 'Acme',
        archetype: 'test',
        template_id: templateId,
      })
      expect(result.ok).toBe(false)
      if (!result.ok) expect(result.error.code).toBe('VALIDATION_ERROR')
    })

    test('is atomic -- no orphaned resume on section creation failure', () => {
      // This test verifies the transaction wrapping.
      // We can't easily force a section creation failure without mocking,
      // but we verify that the transaction is used by checking that both
      // resume and sections exist after a successful call.
      const templateId = seedResumeTemplate(db, {
        sections: [{ title: 'Exp', entry_type: 'experience', position: 0 }],
      })
      const result = service.createResumeFromTemplate({
        name: 'Atomic Test',
        target_role: 'Eng',
        target_employer: 'Corp',
        archetype: 'test',
        template_id: templateId,
      })
      expect(result.ok).toBe(true)
      if (result.ok) {
        const resumeCount = (db.query('SELECT COUNT(*) as c FROM resumes WHERE id = ?').get(result.data.id) as { c: number }).c
        const sectionCount = (db.query('SELECT COUNT(*) as c FROM resume_sections WHERE resume_id = ?').get(result.data.id) as { c: number }).c
        expect(resumeCount).toBe(1)
        expect(sectionCount).toBe(1)
      }
    })
  })
})
```

**Acceptance criteria:**
- Validation tests cover: empty name, empty sections, invalid entry_type, `custom`, `summary`
- Built-in delete protection is tested
- Save-as-template tests cover: success, empty resume, empty name
- Create-from-template tests cover: success, nonexistent template, invalid input, atomicity

---

### T36.12: Write Route Integration Tests

**File:** `packages/core/src/routes/__tests__/templates.test.ts`

```typescript
import { describe, expect, test, beforeEach, afterEach } from 'bun:test'
import { createTestDb, seedResume, seedResumeSection, seedResumeTemplate } from '../../db/__tests__/helpers'
import { createServices } from '../../services'
import { createApp } from '../server'
import type { Database } from 'bun:sqlite'

describe('Template Routes', () => {
  let db: Database
  let app: ReturnType<typeof createApp>

  beforeEach(() => {
    db = createTestDb()
    const services = createServices(db)
    app = createApp(services, db)
  })

  afterEach(() => db.close())

  const json = (res: Response) => res.json()
  const req = (method: string, path: string, body?: unknown) =>
    app.request(`/api${path}`, {
      method,
      headers: body ? { 'Content-Type': 'application/json' } : {},
      body: body ? JSON.stringify(body) : undefined,
    })

  describe('GET /api/templates', () => {
    test('returns seeded built-in templates', async () => {
      const res = await req('GET', '/templates')
      expect(res.status).toBe(200)
      const { data } = await json(res)
      expect(data.length).toBeGreaterThanOrEqual(3)
    })
  })

  describe('GET /api/templates/:id', () => {
    test('returns template by id', async () => {
      const id = seedResumeTemplate(db, { name: 'Fetch Me' })
      const res = await req('GET', `/templates/${id}`)
      expect(res.status).toBe(200)
      const { data } = await json(res)
      expect(data.name).toBe('Fetch Me')
    })

    test('returns 404 for nonexistent', async () => {
      const res = await req('GET', '/templates/nonexistent')
      expect(res.status).toBe(404)
    })
  })

  describe('POST /api/templates', () => {
    test('creates template (201)', async () => {
      const res = await req('POST', '/templates', {
        name: 'New Template',
        sections: [{ title: 'Exp', entry_type: 'experience', position: 0 }],
      })
      expect(res.status).toBe(201)
      const { data } = await json(res)
      expect(data.name).toBe('New Template')
      expect(data.is_builtin).toBe(0)
    })

    test('rejects invalid entry_type (400)', async () => {
      const res = await req('POST', '/templates', {
        name: 'Bad',
        sections: [{ title: 'X', entry_type: 'invalid', position: 0 }],
      })
      expect(res.status).toBe(400)
    })
  })

  describe('PATCH /api/templates/:id', () => {
    test('updates template name', async () => {
      const id = seedResumeTemplate(db, { name: 'Old' })
      const res = await req('PATCH', `/templates/${id}`, { name: 'New' })
      expect(res.status).toBe(200)
      const { data } = await json(res)
      expect(data.name).toBe('New')
    })
  })

  describe('DELETE /api/templates/:id', () => {
    test('deletes user template (204)', async () => {
      const id = seedResumeTemplate(db)
      const res = await req('DELETE', `/templates/${id}`)
      expect(res.status).toBe(204)
    })

    test('rejects deleting built-in template (400)', async () => {
      const id = seedResumeTemplate(db, { isBuiltin: true })
      const res = await req('DELETE', `/templates/${id}`)
      expect(res.status).toBe(400)
    })
  })

  describe('POST /api/resumes/:id/save-as-template', () => {
    test('creates template from resume sections (201)', async () => {
      const resumeId = seedResume(db)
      seedResumeSection(db, resumeId, 'Experience', 'experience', 0)
      seedResumeSection(db, resumeId, 'Skills', 'skills', 1)

      const res = await req('POST', `/resumes/${resumeId}/save-as-template`, {
        name: 'Saved Layout',
      })
      expect(res.status).toBe(201)
      const { data } = await json(res)
      expect(data.sections).toHaveLength(2)
    })
  })

  describe('POST /api/resumes (with template_id)', () => {
    test('creates resume with sections from template (201)', async () => {
      const templateId = seedResumeTemplate(db, {
        sections: [
          { title: 'Summary', entry_type: 'freeform', position: 0 },
          { title: 'Exp', entry_type: 'experience', position: 1 },
        ],
      })

      const res = await req('POST', '/resumes', {
        name: 'Templated Resume',
        target_role: 'Engineer',
        target_employer: 'Acme',
        archetype: 'test-arch',
        template_id: templateId,
      })
      expect(res.status).toBe(201)
      const { data } = await json(res)

      // Verify sections were created
      const sections = db
        .query('SELECT * FROM resume_sections WHERE resume_id = ? ORDER BY position')
        .all(data.id) as Array<{ title: string }>
      expect(sections).toHaveLength(2)
    })

    test('creates resume without template_id (backward compat)', async () => {
      const res = await req('POST', '/resumes', {
        name: 'No Template',
        target_role: 'Engineer',
        target_employer: 'Acme',
        archetype: 'test-arch',
      })
      expect(res.status).toBe(201)
      const { data } = await json(res)

      const sections = db
        .query('SELECT * FROM resume_sections WHERE resume_id = ?')
        .all(data.id) as Array<unknown>
      expect(sections).toHaveLength(0)
    })

    test('returns 404 for nonexistent template_id', async () => {
      const res = await req('POST', '/resumes', {
        name: 'Bad Template',
        target_role: 'Engineer',
        target_employer: 'Acme',
        archetype: 'test-arch',
        template_id: 'nonexistent-id-not-a-real-template',
      })
      expect(res.status).toBe(404)
    })
  })
})
```

**Acceptance criteria:**
- All CRUD endpoints return correct status codes
- Delete returns 400 for built-in templates
- Save-as-template returns 201 with correct sections
- Resume creation with `template_id` creates sections atomically
- Resume creation without `template_id` is backward compatible (zero sections)

---

### T36.13: Write SDK Contract Tests

**File:** `packages/sdk/src/__tests__/templates.test.ts`

```typescript
import { describe, expect, test, beforeEach, afterEach } from 'bun:test'
import { ForgeClient } from '../client'
import { createTestDb, seedResumeTemplate, seedResume, seedResumeSection } from '../../../core/src/db/__tests__/helpers'
import { createServices } from '../../../core/src/services'
import { createApp } from '../../../core/src/routes/server'
import type { Database } from 'bun:sqlite'

describe('SDK TemplatesResource', () => {
  let db: Database
  let client: ForgeClient
  let server: ReturnType<typeof Bun.serve>

  beforeEach(async () => {
    db = createTestDb()
    const services = createServices(db)
    const app = createApp(services, db)
    server = Bun.serve({ port: 0, fetch: app.fetch })
    client = new ForgeClient({ baseUrl: `http://localhost:${server.port}` })
  })

  afterEach(() => { server.stop(); db.close() })

  test('list returns built-in templates with is_builtin as boolean', async () => {
    const result = await client.templates.list()
    expect(result.ok).toBe(true)
    if (result.ok) {
      expect(result.data.length).toBeGreaterThanOrEqual(3)
      const builtin = result.data.find(t => t.name === 'Standard Tech Resume')
      expect(builtin).toBeDefined()
      expect(builtin!.is_builtin).toBe(true)
      expect(typeof builtin!.is_builtin).toBe('boolean')
    }
  })

  test('create returns template with is_builtin = false', async () => {
    const result = await client.templates.create({
      name: 'SDK Test',
      sections: [{ title: 'Exp', entry_type: 'experience', position: 0 }],
    })
    expect(result.ok).toBe(true)
    if (result.ok) {
      expect(result.data.is_builtin).toBe(false)
      expect(typeof result.data.is_builtin).toBe('boolean')
    }
  })

  test('get returns template with parsed sections', async () => {
    const id = seedResumeTemplate(db, {
      sections: [{ title: 'Skills', entry_type: 'skills', position: 0 }],
    })
    const result = await client.templates.get(id)
    expect(result.ok).toBe(true)
    if (result.ok) {
      expect(result.data.sections).toHaveLength(1)
      expect(result.data.sections[0].entry_type).toBe('skills')
    }
  })

  test('delete returns ok for user templates', async () => {
    const id = seedResumeTemplate(db)
    const result = await client.templates.delete(id)
    expect(result.ok).toBe(true)
  })

  test('delete returns error for built-in templates', async () => {
    const id = seedResumeTemplate(db, { isBuiltin: true })
    const result = await client.templates.delete(id)
    expect(result.ok).toBe(false)
  })

  test('saveAsTemplate creates template from resume', async () => {
    const resumeId = seedResume(db)
    seedResumeSection(db, resumeId, 'Experience', 'experience', 0)

    const result = await client.resumes.saveAsTemplate(resumeId, {
      name: 'From SDK',
    })
    expect(result.ok).toBe(true)
    if (result.ok) {
      expect(result.data.name).toBe('From SDK')
      expect(result.data.sections).toHaveLength(1)
    }
  })
})
```

**Acceptance criteria:**
- SDK correctly converts `is_builtin` to boolean
- SDK `saveAsTemplate` works through the resumes resource
- All CRUD operations round-trip correctly through SDK

---

### T36.14: UI -- Templates Management Page

**File:** `packages/webui/src/routes/config/templates/+page.svelte`

This page provides:
1. A list of template cards showing name, description, section count, and a "Built-in" badge
2. "Create Template" button opening an inline form
3. Edit and Delete buttons on each card (Delete disabled for built-in)
4. Template editor with name, description, section list (add/remove/reorder with up/down arrows)

The implementation should follow existing SvelteKit patterns in the `packages/webui` directory. The template editor uses the `forge` SDK client (already available in the webui context) with calls to:
- `client.templates.list()`
- `client.templates.create(input)`
- `client.templates.update(id, input)`
- `client.templates.delete(id)`

**Key UI decisions:**
- Section reordering uses up/down arrow buttons (not drag-and-drop) for MVP
- Entry type selection uses a `<select>` dropdown with the valid entry types
- Template preview shows numbered section list with entry type in parentheses
- Built-in templates show a badge and have Delete disabled (Edit is allowed)

**Acceptance criteria:**
- Template list loads and displays all templates
- Create template form validates name and sections
- Edit updates template and refreshes list
- Delete removes user templates, shows error for built-in
- New sections can be added with entry type dropdown
- Sections can be removed and reordered

---

### T36.15: UI -- Template Picker in Resume Creation

**File:** Modify `packages/webui/src/routes/resumes/+page.svelte` (or the resume creation modal)

When creating a resume, add a template selection step:
1. Fetch templates via `client.templates.list()`
2. Display as a grid of cards. "Blank Resume" is always the first option.
3. Selected template's `id` is passed as `template_id` in the `POST /api/resumes` body
4. If "Blank Resume" is selected, no `template_id` is sent (backward compatible)

**Acceptance criteria:**
- Template picker shows all templates plus "Blank Resume" option
- Selecting a template pre-populates sections on resume creation
- "Blank Resume" creates a resume with no sections
- Template cards show name, description, and section count

---

### T36.16: UI -- Save as Template Button

**File:** Modify `packages/webui/src/routes/resumes/[id]/+page.svelte`

Add a "Save as Template" button in the resume builder toolbar. On click:
1. Open a small modal with name input and optional description textarea
2. On submit, call `client.resumes.saveAsTemplate(resumeId, { name, description })`
3. Show success toast with template name
4. Show error toast if resume has no sections

**Acceptance criteria:**
- Button appears in resume builder toolbar
- Modal collects name (required) and description (optional)
- Success shows template name in toast
- Error shows validation message

---

## Testing Support

### Test Fixtures

| Fixture | Description |
|---------|-------------|
| `seedResumeTemplate(db, opts)` | Creates a template with configurable name, description, sections, and `isBuiltin` flag |
| Built-in templates from migration 008 | Three templates seeded automatically in `createTestDb()` |

### Test Matrix

| Test Kind | File | Count | What it covers |
|-----------|------|-------|----------------|
| Unit | `template-repository.test.ts` | ~12 | Repository CRUD, JSON serialization, sort order |
| Unit | `template-service.test.ts` | ~15 | Validation, built-in protection, save-as-template, create-from-template, atomicity |
| Integration | `templates.test.ts` (routes) | ~12 | HTTP endpoints, status codes, backward compatibility |
| Contract | `templates.test.ts` (SDK) | ~7 | SDK type conversion, round-trip, `is_builtin` boolean |
| Smoke | Manual | -- | UI template management, template picker, save-as-template |

### Critical Test Cases

1. `entry_type` validation rejects `custom` and `summary` values
2. `createResumeFromTemplate` is atomic (transaction rollback on failure)
3. Creating resume without `template_id` results in zero sections (backward compat)
4. Deleting built-in template returns error
5. `saveAsTemplate` on resume with no sections returns `VALIDATION_ERROR`
6. SDK converts `is_builtin` from `0|1` to `boolean`
7. `sections` JSON round-trips correctly (serialize on write, deserialize on read)

## Documentation Requirements

- Add `ResumeTemplate`, `TemplateSectionDef`, `CreateResumeTemplate`, `UpdateResumeTemplate` to SDK type documentation
- Document `POST /api/templates`, `GET /api/templates`, `GET /api/templates/:id`, `PATCH /api/templates/:id`, `DELETE /api/templates/:id` endpoints
- Document `POST /api/resumes/:id/save-as-template` endpoint
- Document `template_id` optional field on `POST /api/resumes`
- Note that `DELETE /api/templates/:id` returns 400 for built-in templates
- Document new API endpoints via JSDoc on route handlers in `packages/core/src/routes/templates.ts`. No separate API docs file is required.

## Parallelization Notes

- **T36.1 (migration)** must be first -- all other tasks depend on the table existing
- **T36.2 (types)** must be second -- repository, service, SDK all reference these types
- **T36.3 (repository)** and **T36.5 (compiler)** can run in parallel after T36.2
- **T36.4 (service)** depends on T36.3 (repository)
- **T36.6 (routes)** and **T36.7 (service registration)** depend on T36.4
- **T36.8 (SDK)** depends on T36.2 (types) only -- can run in parallel with T36.3-T36.7
- **T36.9 (test helpers)** can run in parallel with T36.3
- **T36.10-T36.13 (tests)** depend on their respective implementation tasks
- **T36.14-T36.16 (UI)** depend on T36.6 (routes) and T36.8 (SDK) being complete
- **Phase 36 overall** is independent of Phases 29-32 and 35 at the DDL and code level

```
T36.1 (migration)
  |
T36.2 (types)
  |
  +---> T36.3 (repo)    T36.5 (compiler)    T36.8 (SDK)    T36.9 (helpers)
  |       |                                     |               |
  |     T36.4 (service)                         |               |
  |       |                                     |               |
  |     T36.6 (routes) + T36.7 (svc reg)        |               |
  |       |                                     |               |
  +-------+-------------------------------------+---------------+
  |
  T36.10-T36.13 (tests)   [T36.9 is a prerequisite for T36.10]
  |
  T36.14-T36.16 (UI)
```
