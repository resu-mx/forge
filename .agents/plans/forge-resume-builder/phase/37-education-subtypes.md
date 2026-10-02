# Phase 37: Education Sub-Type Fields (Migration 009 + Core + API + SDK + UI)

**Status:** Planning
**Date:** 2026-04-03
**Spec:** [2026-04-03-education-subtype-fields.md](../refs/specs/2026-04-03-education-subtype-fields.md)
**Depends on:** Phase 28 (resume sections -- stable baseline)
**Blocks:** None currently identified
**Parallelizable with:** Phases 29-36 (independent at the DDL level; migration numbered after 008)

## Goal

Add 6 new columns to the `source_education` table (`degree_level`, `degree_type`, `certificate_subtype`, `gpa`, `location`, `edu_description`) to support sub-type-specific fields for each `education_type` value (degree, certificate, course, self_taught). Update the core types, repository, IR compiler, LaTeX template, SDK, and UI to surface these fields end-to-end. The UI conditionally shows only relevant fields based on the selected education type. Degree entries render with "M.S. in Field, GPA: X" format, certificates render with issuing body and credential ID, and self-taught entries render as freeform `\resumeItem` text. The organization dropdown in role/project source forms is filtered to orgs with existing role sources.

## Non-Goals

- Sub-type extension tables (Approach 2 from the brainstorm doc)
- JSON extension column (Approach 3 from the brainstorm doc)
- Cross-entity relationships (self_taught <-> projects)
- Changes to resume section entry_types (certifications already handled in Phase 36)
- DB-level "required" enforcement for degree_level on degrees (UI-enforced only)
- Profile photo / avatar on education entries
- Multi-institution support per education source

## Context

The `source_education` extension table currently uses a flat set of nullable columns shared across all four `education_type` values. This works for certificates (the original primary use case) but does not distinguish degree sub-levels (associate vs. masters vs. doctoral), certificate sub-types (professional vs. vendor vs. completion), or provide fields for GPA, location, or a dedicated education description. Graduate certificates are classified as a degree level (not a certificate subtype) because they come from accredited institutions, may have GPA/coursework, and represent academic rigor similar to a mini-graduate degree. The `edu_description` column is named to avoid collision with the base `sources.description` column.

**PREREQUISITE:** Migration `005_job_descriptions.sql` must be renamed to `007_job_descriptions.sql` before this migration (009) is created. Verify the migration directory has sequential numbering 001-008 before proceeding.

## Scope

| Spec section | Covered here? |
|-------------|---------------|
| 1. Schema (migration 009) | Yes |
| 2. Type changes (core + SDK) | Yes |
| 3. Repository changes (create + updateExtension) | Yes |
| 4. UI changes (conditional form + org filter) | Yes |
| 5. IR compiler enhancement | Yes |
| 5.3 LaTeX rendering logic | Yes |

## Files to Create

| File | Description |
|------|-------------|
| `packages/core/src/db/migrations/009_education_subtype_fields.sql` | ALTER TABLE migration adding 6 columns |

## Files to Modify

| File | Change |
|------|--------|
| `packages/core/src/types/index.ts` | Update `SourceEducation`, `EducationItem`; add `DegreeLevelType`, `CertificateSubtype`, `EducationType` union types |
| `packages/sdk/src/types.ts` | Mirror type changes: `SourceEducation`, `EducationItem`, union types |
| `packages/sdk/src/index.ts` | Export new union types |
| `packages/core/src/db/repositories/source-repository.ts` | Update `create()` INSERT and `updateExtension()` for 6 new education fields |
| `packages/core/src/services/resume-compiler.ts` | Extend `buildEducationItems` query, result cast, and mapping |
| `packages/core/src/templates/sb2nov.ts` | Update `renderEducationSection` for per-type formatting |
| `packages/webui/src/routes/data/sources/SourcesView.svelte` | Conditional education form sections; org dropdown filter; 6 new form state variables |
| `packages/core/src/db/repositories/__tests__/source-repository.test.ts` | Add tests for new education fields |
| `packages/core/src/templates/__tests__/sb2nov.test.ts` | Add per-type education rendering tests |
| `packages/core/src/routes/sources.ts` | Add translation layer to spread nested SDK objects (education, role, project, clearance) into flat structure before passing to service |
| `packages/core/src/routes/__tests__/sources.test.ts` | Add integration tests for education sub-type round-trips |

## Fallback Strategies

- **Migration fails on existing data:** All 6 new columns are nullable with no NOT NULL constraint, so ALTER TABLE succeeds even if `source_education` has existing rows. Existing rows get NULL for all new columns.
- **UI form shows stale fields after type switch:** The `$effect` block that watches `formEducationType` resets irrelevant fields to empty/null when the education type changes, preventing stale data from leaking across types.
- **Compiler encounters education row with NULL education_type:** The compiler defaults to the existing behavior (institution + degree + date subheading) when `education_type` is null or unrecognized, so pre-migration data still renders correctly.
- **SDK consumer passes unknown education_type:** The repository already defaults to `'certificate'` via `input.education_type ?? 'certificate'`. New sub-type fields are simply ignored for types that don't use them (all nullable).

---

## Tasks

### T37.0: Verify migration numbering prerequisite

Before any implementation:
1. Run `ls packages/core/src/db/migrations/` and verify files 001-008 exist sequentially
2. If `005_job_descriptions.sql` exists, rename it to `007_job_descriptions.sql`
3. Verify no `009_*` file exists
4. Acceptance: `ls` shows 001, 002, 003, 004, 005_user_profile, 006_summaries, 007_job_descriptions, 008_resume_templates

---

### T37.0.1: Add SDK-to-core translation layer in sources route

**File:** `packages/core/src/routes/sources.ts`

The SDK sends nested extension objects (e.g. `{ education: { degree_level: 'masters', ... } }`) but the core service/repository reads fields at the top level (e.g. `input.degree_level`). This is a pre-existing architectural mismatch between SDK (nested) and core (flat) types. This fix resolves it for all source extension types, not just education.

In the POST handler, before passing to the service:

```typescript
// In the POST handler, before passing to service:
const body = await c.req.json()
const input = {
  ...body,
  // Spread nested extension objects into flat structure
  ...(body.education ?? {}),
  ...(body.role ?? {}),
  ...(body.project ?? {}),
  ...(body.clearance ?? {}),
}
```

Apply the same pattern in the PATCH handler for updates.

**Acceptance criteria:**
- SDK `forge.sources.create({ source_type: 'education', education: { degree_level: 'masters' } })` persists `degree_level = 'masters'` in the DB.
- SDK `forge.sources.update(id, { education: { gpa: '3.9/4.0' } })` persists `gpa = '3.9/4.0'` in the DB.
- Existing flat-style API calls (e.g. `{ source_type: 'education', degree_level: 'masters' }`) continue to work (spread of `undefined` is a no-op).

**Failure criteria:**
- Nested SDK payloads silently drop extension fields.
- Flat-style payloads break due to double-spreading.

---

### T37.1: Write Migration `009_education_subtype_fields.sql`

**File:** `packages/core/src/db/migrations/009_education_subtype_fields.sql`

```sql
-- Forge Resume Builder -- Education Sub-Type Fields
-- Migration: 009_education_subtype_fields
-- Date: 2026-04-03
--
-- Adds degree_level, degree_type, certificate_subtype, gpa, location,
-- edu_description to source_education for per-type field support.
-- Builds on 002_schema_evolution (source_education table).

ALTER TABLE source_education ADD COLUMN degree_level TEXT
  CHECK (degree_level IS NULL OR degree_level IN (
    'associate', 'bachelors', 'masters', 'doctoral', 'graduate_certificate'
  ));

ALTER TABLE source_education ADD COLUMN degree_type TEXT;

ALTER TABLE source_education ADD COLUMN certificate_subtype TEXT
  CHECK (certificate_subtype IS NULL OR certificate_subtype IN (
    'professional', 'vendor', 'completion'
  ));

ALTER TABLE source_education ADD COLUMN gpa TEXT;

ALTER TABLE source_education ADD COLUMN location TEXT;

ALTER TABLE source_education ADD COLUMN edu_description TEXT;

INSERT INTO _migrations (name) VALUES ('009_education_subtype_fields');
```

**Key points:**
- All 6 columns are nullable. No existing rows are broken.
- `degree_level` and `certificate_subtype` have CHECK constraints for enum validation at the DB level.
- `degree_type`, `gpa`, `location`, `edu_description` are free text with no constraints.
- `edu_description` is deliberately named to avoid collision with `sources.description`.

> **Note:** The `field` column (major/focus) is populated for all education types via `input.field ?? null`. For non-degree types, the UI does not show a field input, so it will always be NULL. This is correct -- the column exists for degree sources only but is safe to INSERT as NULL for other types.

**Acceptance criteria:**
- After migration, `PRAGMA table_info(source_education)` includes all 6 new columns.
- Existing rows in `source_education` have NULL for all new columns.
- Inserting a row with `degree_level = 'masters'` succeeds.
- Inserting a row with `degree_level = 'phd'` fails the CHECK constraint.
- Inserting a row with `certificate_subtype = 'vendor'` succeeds.
- Inserting a row with `certificate_subtype = 'bootcamp'` fails the CHECK constraint.
- The `_migrations` table contains `009_education_subtype_fields`.

**Failure criteria:**
- Migration fails with a SQL error on an existing database with education rows.
- A CHECK constraint is too restrictive (e.g., prevents NULL values).

---

### T37.2: Add Union Types and Update `SourceEducation` in Core Types

**File:** `packages/core/src/types/index.ts`

Add union types after the existing type definitions (before `SourceEducation`):

```typescript
// ── Education Sub-Type Unions ────────────────────────────────────────

export type DegreeLevelType = 'associate' | 'bachelors' | 'masters' | 'doctoral' | 'graduate_certificate'
export type CertificateSubtype = 'professional' | 'vendor' | 'completion'
export type EducationType = 'degree' | 'certificate' | 'course' | 'self_taught'
```

Update `SourceEducation` to add the 6 new fields:

```typescript
/** Education-specific details for a source with source_type='education'. */
export interface SourceEducation {
  source_id: string
  education_type: EducationType
  // Shared
  institution: string | null
  edu_description: string | null
  location: string | null
  start_date: string | null
  end_date: string | null
  url: string | null
  // Degree-specific
  degree_level: DegreeLevelType | null
  degree_type: string | null
  field: string | null
  gpa: string | null
  is_in_progress: number
  // Certificate-specific
  certificate_subtype: CertificateSubtype | null
  credential_id: string | null
  expiration_date: string | null
  issuing_body: string | null
}
```

Update `EducationItem` to add optional fields for compiler output:

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
}
```

**Acceptance criteria:**
- `SourceEducation` includes all 6 new fields (`degree_level`, `degree_type`, `certificate_subtype`, `gpa`, `location`, `edu_description`).
- `EducationItem` includes all new optional fields, including `field?: string | null`.
- Union types `DegreeLevelType`, `CertificateSubtype`, `EducationType` are exported.
- TypeScript compilation succeeds for all files importing from `../types` or `../../types`.
- `education_type` on `SourceEducation` references `EducationType` instead of an inline union.

Additionally, add the 6 new fields to core's `CreateSource` and `UpdateSource` interfaces as optional fields:

```typescript
// In CreateSource education fields section:
degree_level?: DegreeLevelType
degree_type?: string
certificate_subtype?: CertificateSubtype
gpa?: string
location?: string
edu_description?: string
```

Same pattern for `UpdateSource` (all optional, same types).

> **Note:** Core uses flat types where all extension fields are optional at the top level. The SDK uses nested types. The route translation layer (T37.0.1) bridges the gap.

**Failure criteria:**
- TypeScript compilation fails due to type mismatch.
- Existing code using `SourceEducation` breaks because a required field was added (all new fields are nullable).

---

### T37.3: Mirror Type Changes in SDK Types

**File:** `packages/sdk/src/types.ts`

Add union types (same as core, but SDK does not use `source_id`):

```typescript
// ── Education Sub-Type Unions ────────────────────────────────────────

export type DegreeLevelType = 'associate' | 'bachelors' | 'masters' | 'doctoral' | 'graduate_certificate'
export type CertificateSubtype = 'professional' | 'vendor' | 'completion'
export type EducationType = 'degree' | 'certificate' | 'course' | 'self_taught'
```

Update SDK `SourceEducation` (note: SDK intentionally omits `source_id`):

```typescript
export interface SourceEducation {
  education_type: EducationType
  // Shared
  institution: string | null
  edu_description: string | null
  location: string | null
  start_date: string | null
  end_date: string | null
  url: string | null
  // Degree-specific
  degree_level: DegreeLevelType | null
  degree_type: string | null
  field: string | null
  gpa: string | null
  is_in_progress: boolean
  // Certificate-specific
  certificate_subtype: CertificateSubtype | null
  credential_id: string | null
  expiration_date: string | null
  issuing_body: string | null
}
```

Update SDK `EducationItem`:

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
}
```

**File:** `packages/sdk/src/index.ts`

In `packages/sdk/src/index.ts`, add to the type exports block:

```typescript
export type { DegreeLevelType, CertificateSubtype, EducationType } from './types'
```

**Acceptance criteria:**
- SDK `SourceEducation` mirrors core `SourceEducation` minus `source_id`.
- SDK `EducationItem` mirrors core `EducationItem` exactly.
- Union types are exported from `@forge/sdk`.
- `is_in_progress` remains `boolean` in SDK (not `number` like core -- SDK uses boolean for consumer ergonomics).

**Failure criteria:**
- SDK `SourceEducation` includes `source_id` (it should not).
- TypeScript compilation fails in consuming projects.

---

### T37.4: Update Repository `create()` for New Education Fields

**File:** `packages/core/src/db/repositories/source-repository.ts`

In the `create()` function, replace the existing education INSERT block:

```typescript
    } else if (sourceType === 'education') {
      db.run(
        `INSERT INTO source_education (
          source_id, education_type, institution, field, start_date, end_date,
          is_in_progress, credential_id, expiration_date, issuing_body, url,
          degree_level, degree_type, certificate_subtype, gpa, location, edu_description
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
        [
          id,
          input.education_type ?? 'certificate',
          input.institution ?? null,
          input.field ?? null,
          input.start_date ?? null,
          input.end_date ?? null,
          input.is_in_progress ?? 0,
          input.credential_id ?? null,
          input.expiration_date ?? null,
          input.issuing_body ?? null,
          input.url ?? null,
          input.degree_level ?? null,
          input.degree_type ?? null,
          input.certificate_subtype ?? null,
          input.gpa ?? null,
          input.location ?? null,
          input.edu_description ?? null,
        ],
      )
```

**Expected inputs/outputs:**
- `create(db, { title: 'CS Degree', description: '...', source_type: 'education', education_type: 'degree', degree_level: 'masters', degree_type: 'MS', institution: 'MIT', gpa: '3.9/4.0' })` creates a source with all degree fields populated.
- `create(db, { title: 'AWS SAA', description: '...', source_type: 'education', education_type: 'certificate', certificate_subtype: 'vendor' })` creates a source with certificate_subtype populated.
- Omitting new fields defaults them to NULL.

**Acceptance criteria:**
- INSERT includes all 17 columns (11 existing + 6 new).
- VALUES has 17 placeholders.
- New fields default to null via `?? null`.
- Round-trip: create with `degree_level: 'masters'`, then `get()` returns `extension.degree_level === 'masters'`.

**Failure criteria:**
- Column count mismatch between INSERT column list and VALUES placeholders.
- Existing education source creation breaks.

---

### T37.5: Update Repository `updateExtension()` for New Education Fields

**File:** `packages/core/src/db/repositories/source-repository.ts`

In the `updateExtension()` function, add 6 new field handlers to the `} else if (sourceType === 'education') {` block, after the existing `if ('end_date' in input)` line:

```typescript
    if ('degree_level' in input) { sets.push('degree_level = ?'); params.push(input.degree_level ?? null) }
    if ('degree_type' in input) { sets.push('degree_type = ?'); params.push(input.degree_type ?? null) }
    if ('certificate_subtype' in input) { sets.push('certificate_subtype = ?'); params.push(input.certificate_subtype ?? null) }
    if ('gpa' in input) { sets.push('gpa = ?'); params.push(input.gpa ?? null) }
    if ('location' in input) { sets.push('location = ?'); params.push(input.location ?? null) }
    if ('edu_description' in input) { sets.push('edu_description = ?'); params.push(input.edu_description ?? null) }
```

These lines go immediately after the existing education update lines (after `if ('end_date' in input)` and before `if (sets.length > 0)`).

> **Note:** These fields compile because `UpdateSource` was updated in T37.2 to include the 6 new education fields.

**Expected inputs/outputs:**
- `update(db, id, { degree_level: 'doctoral' })` updates only `degree_level` on the education extension.
- `update(db, id, { gpa: '3.8/4.0', location: 'Cambridge, MA' })` updates both fields in a single UPDATE.
- `update(db, id, { degree_level: null })` clears `degree_level` to NULL.

**Acceptance criteria:**
- Each new field is independently updatable.
- Null values are accepted (clearing a field).
- Fields not present in the input are not touched.
- The `'in' operator` check pattern matches the existing style.

**Failure criteria:**
- Updating a new field fails with a SQL error.
- Updating a new field accidentally clears an existing field.

---

### T37.6: Extend IR Compiler `buildEducationItems`

**File:** `packages/core/src/services/resume-compiler.ts`

Replace the `buildEducationItems` function:

```typescript
function buildEducationItems(db: Database, sectionId: string): EducationItem[] {
  const rows = db
    .query(
      `SELECT
        re.id AS entry_id,
        re.content AS entry_content,
        p.content AS perspective_content,
        bs.source_id,
        se.education_type,
        se.institution,
        se.field,
        se.end_date,
        se.degree_level,
        se.degree_type,
        se.gpa,
        se.location,
        se.credential_id,
        se.issuing_body,
        se.certificate_subtype,
        se.edu_description
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
    }>

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
  }))
}
```

**Key changes from existing code:**
- SELECT adds 9 new columns from `se.*`.
- The `.all()` result cast adds all new column types.
- The return mapping includes all new fields. `education_type` maps `null` to `undefined` (optional field on `EducationItem`).
- `edu_description` maps `row.edu_description` (not `row.description` which would collide).
- `field` is mapped via `row.field ?? null` to ensure it is always present on the `EducationItem` (not `undefined`).

> **Note:** For certificates, `end_date` represents the expiration date. The `expiration_date` column on `source_education` is a legacy duplicate -- the compiler uses `end_date` for the IR's `date` field. Both columns exist in the DB but only `end_date` flows to the IR.

**Acceptance criteria:**
- Compiled IR for a degree education entry includes `degree_level`, `degree_type`, `gpa`, `location`.
- Compiled IR for a certificate education entry includes `credential_id`, `issuing_body`, `certificate_subtype`.
- Compiled IR for a self_taught entry includes `edu_description`.
- Pre-migration education data (null new fields) still compiles without errors.

**Failure criteria:**
- Query fails because a column name is wrong.
- `education_type` is `null` instead of `undefined` on the EducationItem (breaks optional check).
- `edu_description` is accidentally mapped from `row.description` (wrong column).

---

### T37.7: Update LaTeX Template Education Rendering

**File:** `packages/core/src/templates/sb2nov.ts`

Replace the `renderEducationSection` function:

```typescript
function renderEducationSection(section: IRSection): string {
  const lines: string[] = []
  lines.push(`\\section{${section.title}}`)
  lines.push('  \\resumeSubHeadingListStart')

  for (const item of section.items) {
    if (item.kind !== 'education') continue

    const eduType = item.education_type ?? 'degree'

    if (eduType === 'self_taught') {
      // Self-taught: freeform text as a simple resumeItem
      const content = item.edu_description ?? item.degree
      lines.push(`    \\resumeItem{${content}}`)
    } else if (eduType === 'certificate') {
      // Certificate: title + expiration on line 1, issuing_body + credential on line 2
      const title = item.degree
      const expDate = item.date ? `Exp. ${item.date}` : ''
      const issuer = item.issuing_body ?? item.institution
      const credPart = item.credential_id ? ` -- Credential ID: ${item.credential_id}` : ''
      const line2 = issuer ? `${issuer}${credPart}` : credPart.replace(/^ -- /, '')
      lines.push(`    \\resumeSubheading`)
      lines.push(`      {${title}}{${expDate}}`)
      lines.push(`      {${line2}}{}`)
    } else if (eduType === 'course') {
      // Course: title + date on line 1, institution + location on line 2
      const title = item.degree
      const date = item.date ?? ''
      const instPart = item.institution ?? ''
      const locPart = item.location ? `, ${item.location}` : ''
      const line2 = `${instPart}${locPart}`
      lines.push(`    \\resumeSubheading`)
      lines.push(`      {${title}}{${date}}`)
      lines.push(`      {${line2}}{}`)
    } else {
      // Degree (default): institution + location on line 1, degree info + date on line 2
      const location = item.location ?? ''
      const degreeType = item.degree_type ?? ''
      const field = item.field ?? null
      // Build degree string: "M.S. in Computer Science" or fall back to perspective content
      let degreeLine: string
      if (degreeType && field) {
        degreeLine = `${degreeType} in ${field}`
      } else if (degreeType) {
        degreeLine = degreeType
      } else {
        degreeLine = item.degree
      }
      if (item.gpa) {
        degreeLine += `, GPA: ${item.gpa}`
      }
      const dateRange = item.date ?? ''
      lines.push(`    \\resumeSubheading`)
      lines.push(`      {${item.institution}}{${location}}`)
      lines.push(`      {${degreeLine}}{${dateRange}}`)
    }
  }

  lines.push('  \\resumeSubHeadingListEnd')
  return lines.join('\n')
}
```

**Rendering examples:**

Degree:
```latex
\resumeSubheading
  {MIT}{Cambridge, MA}
  {M.S. in Computer Science, GPA: 3.9/4.0}{2022}
```

Certificate:
```latex
\resumeSubheading
  {AWS Solutions Architect Professional}{Exp. 2027}
  {Amazon Web Services -- Credential ID: ABC123}{}
```

Course:
```latex
\resumeSubheading
  {SANS SEC504: Hacker Tools}{2024}
  {Black Hat Conference, Las Vegas}{}
```

Self-taught:
```latex
\resumeItem{Self-taught Rust systems programming through open-source contributions}
```

**Acceptance criteria:**
- Degree renders with `{degree_type} in {field}, GPA: {gpa}` format when all fields present.
- Degree falls back to perspective content when `degree_type` is null.
- Certificate renders with issuing body and credential ID on line 2.
- Course renders with institution and location on line 2.
- Self-taught renders as `\resumeItem` with `edu_description` content (or falls back to `degree`).
- Existing education items without `education_type` render using the degree (default) path.

**Failure criteria:**
- Any education type renders with wrong LaTeX structure.
- Self-taught renders with `\resumeSubheading` instead of `\resumeItem`.
- GPA appears for non-degree education types.

---

### T37.8: Update UI -- Conditional Education Form and Org Filter

**File:** `packages/webui/src/routes/data/sources/SourcesView.svelte`

#### 8a. Add new form state variables

After the existing education extension field declarations (after `let formUrl = $state('')`):

```typescript
  // Education sub-type fields
  let formDegreeLevel = $state<string>('bachelors')
  let formDegreeType = $state('')
  let formCertificateSubtype = $state<string>('vendor')
  let formGpa = $state('')
  let formLocation = $state('')
  let formEduDescription = $state('')
```

#### 8a.1. Add `$effect` to reset irrelevant fields on education type switch

After the form state variables, add:

```typescript
  $effect(() => {
    // Reset degree-specific fields when not degree
    if (formEducationType !== 'degree') {
      formDegreeLevel = 'bachelors'
      formDegreeType = ''
      formGpa = ''
    }
    // Reset certificate-specific fields when not certificate
    if (formEducationType !== 'certificate') {
      formCertificateSubtype = 'vendor'
    }
    // Reset location when not degree/course
    if (formEducationType !== 'degree' && formEducationType !== 'course') {
      formLocation = ''
    }
  })
```

#### 8b. Update `populateFormFromSource()`

In the `} else if (source.source_type === 'education' && source.education) {` block, add after the existing field assignments:

```typescript
      formDegreeLevel = source.education.degree_level ?? ''  // empty = placeholder shown
      formDegreeType = source.education.degree_type ?? ''
      formCertificateSubtype = source.education.certificate_subtype ?? 'vendor'
      formGpa = source.education.gpa ?? ''
      formLocation = source.education.location ?? ''
      formEduDescription = source.education.edu_description ?? ''
```

Also add resets for these fields in the general reset block at the top of `populateFormFromSource()`:

```typescript
    formDegreeLevel = ''  // empty = placeholder shown for pre-migration data
    formDegreeType = ''
    formCertificateSubtype = 'vendor'
    formGpa = ''
    formLocation = ''
    formEduDescription = ''
```

#### 8c. Update `startNew()`

Add resets after the existing education field resets:

```typescript
    formDegreeLevel = ''  // empty = placeholder shown until user selects
    formDegreeType = ''
    formCertificateSubtype = 'vendor'
    formGpa = ''
    formLocation = ''
    formEduDescription = ''
```

#### 8d. Update `saveSource()` education payload

Replace the existing education payload block:

```typescript
    } else if (formSourceType === 'education') {
      basePayload.education = {
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
        degree_level: formDegreeLevel || undefined,
        degree_type: formDegreeType || undefined,
        certificate_subtype: formCertificateSubtype || undefined,
        gpa: formGpa || undefined,
        location: formLocation || undefined,
        edu_description: formEduDescription || undefined,
      }
```

#### 8e. Update `loadOrganizations()` with role-source filter

Replace the `loadOrganizations()` function:

```typescript
  async function loadOrganizations() {
    const orgResult = await forge.organizations.list({ limit: 500 })
    if (orgResult.ok) {
      // Filter to orgs that have at least one role source
      const sourceResult = await forge.sources.list({ source_type: 'role', limit: 500 })
      if (sourceResult.ok) {
        const workedOrgIds = new Set(
          sourceResult.data
            .map(s => s.role?.organization_id)
            .filter((id): id is string => id != null)
        )
        organizations = orgResult.data.filter(o => workedOrgIds.has(o.id))
      } else {
        organizations = orgResult.data
      }
    }
  }
```

#### 8f. Replace the education form section in the template

Replace the entire `{#if formSourceType === 'education'}` block:

```svelte
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

          {#if formEducationType === 'degree'}
            <!-- Degree fields -->
            <div class="form-row">
              <div class="form-group">
                <label for="edu-degree-level">Degree Level <span class="required">*</span></label>
                <select id="edu-degree-level" bind:value={formDegreeLevel}>
                  <option value="" disabled>-- Select Degree Level --</option>
                  <option value="associate">Associate</option>
                  <option value="bachelors">Bachelor's</option>
                  <option value="masters">Master's</option>
                  <option value="doctoral">Doctoral</option>
                  <option value="graduate_certificate">Graduate Certificate</option>
                </select>
              </div>
              <div class="form-group">
                <label for="edu-degree-type">Degree Type <span class="required">*</span></label>
                <input id="edu-degree-type" type="text" bind:value={formDegreeType}
                       placeholder="e.g. BS, MS, PhD, MBA" />
              </div>
            </div>
            <div class="form-row">
              <div class="form-group">
                <label for="edu-institution">Institution</label>
                <input id="edu-institution" type="text" bind:value={formInstitution} />
              </div>
              <div class="form-group">
                <label for="edu-location">Location</label>
                <input id="edu-location" type="text" bind:value={formLocation}
                       placeholder="e.g. Cambridge, MA" />
              </div>
            </div>
            <div class="form-row">
              <div class="form-group">
                <label for="edu-field">Field (Major)</label>
                <input id="edu-field" type="text" bind:value={formField} />
              </div>
              <div class="form-group">
                <label for="edu-gpa">GPA</label>
                <input id="edu-gpa" type="text" bind:value={formGpa}
                       placeholder="e.g. 3.8/4.0" />
              </div>
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
              <label for="edu-description">Description</label>
              <textarea id="edu-description" bind:value={formEduDescription} rows="3"
                        placeholder="Additional notes about this degree..."></textarea>
            </div>

          {:else if formEducationType === 'certificate'}
            <!-- Certificate fields -->
            <div class="form-group">
              <label for="edu-cert-subtype">Certificate Type <span class="required">*</span></label>
              <select id="edu-cert-subtype" bind:value={formCertificateSubtype}>
                <option value="professional">Professional (CISSP, PE, PMP)</option>
                <option value="vendor">Vendor (AWS, Azure, CompTIA)</option>
                <option value="completion">Completion (Udemy, bootcamp)</option>
              </select>
            </div>
            <div class="form-group">
              <label for="edu-issuer">Issuing Body</label>
              <input id="edu-issuer" type="text" bind:value={formIssuingBody} />
            </div>
            <div class="form-group">
              <label for="edu-credential">Credential ID</label>
              <input id="edu-credential" type="text" bind:value={formCredentialId} />
            </div>
            <div class="form-group">
              <label for="edu-url">URL</label>
              <input id="edu-url" type="url" bind:value={formUrl} />
            </div>
            <div class="form-group">
              <label for="edu-expiration">Expiration Date</label>
              <input id="edu-expiration" type="date" bind:value={formExpirationDate} />
            </div>
            <div class="form-group">
              <label for="edu-description">Description</label>
              <textarea id="edu-description" bind:value={formEduDescription} rows="3"
                        placeholder="Additional notes about this certificate..."></textarea>
            </div>

          {:else if formEducationType === 'course'}
            <!-- Course fields -->
            <div class="form-row">
              <div class="form-group">
                <label for="edu-institution">Institution (Provider)</label>
                <input id="edu-institution" type="text" bind:value={formInstitution} />
              </div>
              <div class="form-group">
                <label for="edu-location">Location</label>
                <input id="edu-location" type="text" bind:value={formLocation}
                       placeholder="e.g. Las Vegas, NV" />
              </div>
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
              <label for="edu-url">URL</label>
              <input id="edu-url" type="url" bind:value={formUrl} />
            </div>
            <div class="form-group">
              <label for="edu-description">Description</label>
              <textarea id="edu-description" bind:value={formEduDescription} rows="3"
                        placeholder="What you learned, key takeaways..."></textarea>
            </div>

          {:else if formEducationType === 'self_taught'}
            <!-- Self-taught fields -->
            <div class="form-group">
              <label for="edu-description">Description <span class="required">*</span></label>
              <textarea id="edu-description" bind:value={formEduDescription} rows="6"
                        placeholder="Describe what you learned, resources used, projects built..."></textarea>
            </div>
            <div class="form-group">
              <label for="edu-url">URL</label>
              <input id="edu-url" type="url" bind:value={formUrl} />
            </div>
          {/if}
        {/if}
```

> **Note:** For existing education sources created before migration 009, `degree_level` will be NULL. The form defaults to `''` (empty string) which shows the placeholder option `'-- Select Degree Level --'` instead of defaulting to `'bachelors'`, avoiding misleading pre-population. When the user selects a value, it persists normally.

**Acceptance criteria:**
- Selecting "Degree" shows: degree_level dropdown, degree_type input, institution, location, field, GPA, start/end dates, in-progress checkbox, description textarea.
- Selecting "Certificate" shows: certificate_subtype dropdown, issuing body, credential ID, URL, expiration date, description textarea.
- Selecting "Course" shows: institution, location, start/end dates, URL, description textarea.
- Selecting "Self-Taught" shows: description textarea (larger, 6 rows), URL.
- GPA uses `type='text'` (not `type='number'`).
- Organization dropdown in role/project forms only shows orgs with role sources.
- Switching education type does not carry stale field values (reset logic).
- Switching education type from Degree to Certificate clears degree_level, degree_type, and gpa fields.
- Save payload includes all new fields.

**Failure criteria:**
- Degree-specific fields appear for certificate education type.
- GPA field rejects "3.8/4.0" format.
- Organization dropdown shows all orgs instead of filtered list.
- Self-taught form shows institution or date fields.

---

## Testing Support

### Unit Tests -- Repository

**File:** `packages/core/src/db/repositories/__tests__/source-repository.test.ts`

Add to the existing `create` describe block:

```typescript
    test('creates a degree education source with sub-type fields', () => {
      const source = SourceRepo.create(db, {
        title: 'MS Computer Science',
        description: 'Graduate degree in CS.',
        source_type: 'education',
        education_type: 'degree',
        degree_level: 'masters',
        degree_type: 'MS',
        institution: 'MIT',
        field: 'Computer Science',
        gpa: '3.9/4.0',
        location: 'Cambridge, MA',
        edu_description: 'Focus on distributed systems.',
        start_date: '2020-08-01',
        end_date: '2022-05-15',
      })

      expect(source.source_type).toBe('education')
      const ext = source.extension as SourceEducation
      expect(ext.education_type).toBe('degree')
      expect(ext.degree_level).toBe('masters')
      expect(ext.degree_type).toBe('MS')
      expect(ext.gpa).toBe('3.9/4.0')
      expect(ext.location).toBe('Cambridge, MA')
      expect(ext.edu_description).toBe('Focus on distributed systems.')
    })

    test('creates a certificate source with certificate_subtype', () => {
      const source = SourceRepo.create(db, {
        title: 'AWS SAA',
        description: 'Cloud cert.',
        source_type: 'education',
        education_type: 'certificate',
        certificate_subtype: 'vendor',
        issuing_body: 'Amazon Web Services',
        credential_id: 'ABC-123',
      })

      const ext = source.extension as SourceEducation
      expect(ext.certificate_subtype).toBe('vendor')
      expect(ext.issuing_body).toBe('Amazon Web Services')
      expect(ext.credential_id).toBe('ABC-123')
    })

    test('creates a course source with location', () => {
      const source = SourceRepo.create(db, {
        title: 'SANS SEC504',
        description: 'Hacker Tools.',
        source_type: 'education',
        education_type: 'course',
        institution: 'SANS Institute',
        location: 'Las Vegas, NV',
        edu_description: 'Hands-on incident response training.',
      })

      const ext = source.extension as SourceEducation
      expect(ext.education_type).toBe('course')
      expect(ext.location).toBe('Las Vegas, NV')
      expect(ext.edu_description).toBe('Hands-on incident response training.')
    })

    test('creates a self_taught source with edu_description', () => {
      const source = SourceRepo.create(db, {
        title: 'Rust Programming',
        description: 'Self-taught Rust.',
        source_type: 'education',
        education_type: 'self_taught',
        edu_description: 'Learned Rust through the Book and open-source contributions.',
        url: 'https://github.com/my-rust-projects',
      })

      const ext = source.extension as SourceEducation
      expect(ext.education_type).toBe('self_taught')
      expect(ext.edu_description).toBe('Learned Rust through the Book and open-source contributions.')
      expect(ext.url).toBe('https://github.com/my-rust-projects')
      // degree-specific fields should be null
      expect(ext.degree_level).toBeNull()
      expect(ext.degree_type).toBeNull()
      expect(ext.gpa).toBeNull()
    })

    test('new education fields default to null when omitted', () => {
      const source = SourceRepo.create(db, {
        title: 'Legacy Cert',
        description: 'Old cert without sub-type.',
        source_type: 'education',
        education_type: 'certificate',
      })

      const ext = source.extension as SourceEducation
      expect(ext.degree_level).toBeNull()
      expect(ext.degree_type).toBeNull()
      expect(ext.certificate_subtype).toBeNull()
      expect(ext.gpa).toBeNull()
      expect(ext.location).toBeNull()
      expect(ext.edu_description).toBeNull()
    })
```

Add to the existing `update` describe block:

```typescript
    test('updates education degree_level', () => {
      const source = SourceRepo.create(db, {
        title: 'Degree',
        description: 'A degree.',
        source_type: 'education',
        education_type: 'degree',
        degree_level: 'bachelors',
      })

      const updated = SourceRepo.update(db, source.id, { degree_level: 'masters' })
      const ext = updated!.extension as SourceEducation
      expect(ext.degree_level).toBe('masters')
    })

    test('updates gpa with free text format', () => {
      const source = SourceRepo.create(db, {
        title: 'Degree',
        description: 'A degree.',
        source_type: 'education',
        education_type: 'degree',
      })

      const updated = SourceRepo.update(db, source.id, { gpa: '3.8/4.0' })
      const ext = updated!.extension as SourceEducation
      expect(ext.gpa).toBe('3.8/4.0')
    })

    test('clears edu_description to null', () => {
      const source = SourceRepo.create(db, {
        title: 'Self-study',
        description: 'Self-taught.',
        source_type: 'education',
        education_type: 'self_taught',
        edu_description: 'Original description.',
      })

      const updated = SourceRepo.update(db, source.id, { edu_description: null })
      const ext = updated!.extension as SourceEducation
      expect(ext.edu_description).toBeNull()
    })
```

### Integration Tests -- API Round-Trip

**File:** `packages/core/src/routes/__tests__/sources.test.ts`

Add integration tests for education sub-type round-trips:

```typescript
    test('POST source with education_type=degree round-trips all fields', async () => {
      const res = await forge.sources.create({
        title: 'PhD Physics',
        description: 'Doctoral program.',
        source_type: 'education',
        education: {
          education_type: 'degree',
          degree_level: 'doctoral',
          degree_type: 'PhD',
          institution: 'Caltech',
          field: 'Physics',
          gpa: '4.0/4.0',
          location: 'Pasadena, CA',
          edu_description: 'Research in quantum computing.',
        },
      })

      expect(res.ok).toBe(true)
      if (res.ok) {
        expect(res.data.education?.degree_level).toBe('doctoral')
        expect(res.data.education?.degree_type).toBe('PhD')
        expect(res.data.education?.gpa).toBe('4.0/4.0')
        expect(res.data.education?.location).toBe('Pasadena, CA')
        expect(res.data.education?.edu_description).toBe('Research in quantum computing.')
      }
    })

    test('PATCH source changes certificate_subtype', async () => {
      const created = await forge.sources.create({
        title: 'Cert',
        description: 'A cert.',
        source_type: 'education',
        education: {
          education_type: 'certificate',
          certificate_subtype: 'vendor',
        },
      })

      expect(created.ok).toBe(true)
      if (!created.ok) return

      const updated = await forge.sources.update(created.data.id, {
        education: { certificate_subtype: 'professional' },
      })

      expect(updated.ok).toBe(true)
      if (updated.ok) {
        expect(updated.data.education?.certificate_subtype).toBe('professional')
      }
    })

    test('GET source includes all new education fields', async () => {
      const created = await forge.sources.create({
        title: 'Course',
        description: 'A course.',
        source_type: 'education',
        education: {
          education_type: 'course',
          institution: 'SANS',
          location: 'Virtual',
          edu_description: 'Security training.',
        },
      })

      expect(created.ok).toBe(true)
      if (!created.ok) return

      const fetched = await forge.sources.get(created.data.id)
      expect(fetched.ok).toBe(true)
      if (fetched.ok) {
        expect(fetched.data.education?.location).toBe('Virtual')
        expect(fetched.data.education?.edu_description).toBe('Security training.')
        // Null fields are returned, not omitted
        expect(fetched.data.education?.degree_level).toBeNull()
        expect(fetched.data.education?.gpa).toBeNull()
      }
    })
```

### Contract Tests -- SDK Types

```typescript
    test('SDK SourceEducation includes new fields', () => {
      // Type-level check: if this compiles, the contract holds
      const edu: SourceEducation = {
        education_type: 'degree',
        institution: null,
        field: null,
        start_date: null,
        end_date: null,
        is_in_progress: false,
        credential_id: null,
        expiration_date: null,
        issuing_body: null,
        url: null,
        degree_level: 'masters',
        degree_type: 'MS',
        certificate_subtype: null,
        gpa: '3.9/4.0',
        location: 'Test City',
        edu_description: 'Test description',
      }

      expect(edu.degree_level).toBe('masters')
      expect(edu.edu_description).toBe('Test description')
    })

    test('null fields are present not omitted', () => {
      const edu: SourceEducation = {
        education_type: 'certificate',
        institution: null,
        field: null,
        start_date: null,
        end_date: null,
        is_in_progress: false,
        credential_id: null,
        expiration_date: null,
        issuing_body: null,
        url: null,
        degree_level: null,
        degree_type: null,
        certificate_subtype: 'vendor',
        gpa: null,
        location: null,
        edu_description: null,
      }

      expect(edu.degree_level).toBeNull()
      expect(edu.gpa).toBeNull()
    })
```

### Compiler Tests

```typescript
    test('compiles degree with enriched education fields', () => {
      // Setup: create resume with degree education entry
      // ... (seed degree source with degree_level, degree_type, gpa, location)
      const ir = compileResumeIR(db, resumeId)
      const eduSection = ir!.sections.find(s => s.type === 'education')
      const item = eduSection!.items[0] as EducationItem

      expect(item.education_type).toBe('degree')
      expect(item.degree_level).toBe('masters')
      expect(item.degree_type).toBe('MS')
      expect(item.gpa).toBe('3.9/4.0')
      expect(item.location).toBe('Cambridge, MA')
    })

    test('compiles certificate with sub-type fields', () => {
      // Setup: create resume with certificate education entry
      const ir = compileResumeIR(db, resumeId)
      const eduSection = ir!.sections.find(s => s.type === 'education')
      const item = eduSection!.items[0] as EducationItem

      expect(item.education_type).toBe('certificate')
      expect(item.credential_id).toBe('ABC-123')
      expect(item.issuing_body).toBe('AWS')
      expect(item.certificate_subtype).toBe('vendor')
    })

    test('compiles self_taught with edu_description', () => {
      // Setup: create resume with self_taught education entry
      const ir = compileResumeIR(db, resumeId)
      const eduSection = ir!.sections.find(s => s.type === 'education')
      const item = eduSection!.items[0] as EducationItem

      expect(item.education_type).toBe('self_taught')
      expect(item.edu_description).toBe('Learned Rust via open-source.')
    })
```

### Template Tests

**File:** `packages/core/src/templates/__tests__/sb2nov.test.ts`

Add a new describe block for education sub-types:

```typescript
  describe('renderSection - education sub-types', () => {
    test('degree renders with degree_type, field, GPA, location', () => {
      const section: IRSection = {
        id: 's1', type: 'education', title: 'Education', display_order: 0,
        items: [{
          kind: 'education',
          institution: 'MIT',
          degree: 'Master of Science in CS',
          date: '2022',
          entry_id: 'e1',
          source_id: 's1',
          education_type: 'degree',
          degree_level: 'masters',
          degree_type: 'M.S.',
          field: 'Computer Science',
          gpa: '3.9/4.0',
          location: 'Cambridge, MA',
        }],
      }

      const result = sb2nov.renderSection(section)
      expect(result).toContain('\\resumeSubheading')
      expect(result).toContain('{MIT}{Cambridge, MA}')
      expect(result).toContain('M.S. in Computer Science')
      expect(result).toContain('GPA: 3.9/4.0')
      expect(result).toContain('{2022}')
    })

    test('degree without degree_type falls back to perspective content', () => {
      const section: IRSection = {
        id: 's1', type: 'education', title: 'Education', display_order: 0,
        items: [{
          kind: 'education',
          institution: 'WGU',
          degree: 'B.S. Cybersecurity',
          date: '2023',
          entry_id: 'e1',
          source_id: 's1',
          education_type: 'degree',
          degree_level: 'bachelors',
          degree_type: null,
          gpa: null,
          location: null,
        }],
      }

      const result = sb2nov.renderSection(section)
      expect(result).toContain('{B.S. Cybersecurity}{2023}')
    })

    test('certificate renders with issuing body and credential ID', () => {
      const section: IRSection = {
        id: 's1', type: 'education', title: 'Education', display_order: 0,
        items: [{
          kind: 'education',
          institution: 'AWS',
          degree: 'AWS Solutions Architect Professional',
          date: '2027',
          entry_id: 'e1',
          source_id: 's1',
          education_type: 'certificate',
          certificate_subtype: 'vendor',
          issuing_body: 'Amazon Web Services',
          credential_id: 'ABC123',
        }],
      }

      const result = sb2nov.renderSection(section)
      expect(result).toContain('{AWS Solutions Architect Professional}{Exp. 2027}')
      expect(result).toContain('Amazon Web Services -- Credential ID: ABC123')
    })

    test('course renders with institution and location', () => {
      const section: IRSection = {
        id: 's1', type: 'education', title: 'Education', display_order: 0,
        items: [{
          kind: 'education',
          institution: 'SANS Institute',
          degree: 'SANS SEC504: Hacker Tools',
          date: '2024',
          entry_id: 'e1',
          source_id: 's1',
          education_type: 'course',
          location: 'Las Vegas',
        }],
      }

      const result = sb2nov.renderSection(section)
      expect(result).toContain('{SANS SEC504: Hacker Tools}{2024}')
      expect(result).toContain('{SANS Institute, Las Vegas}{}')
    })

    test('self_taught renders as resumeItem', () => {
      const section: IRSection = {
        id: 's1', type: 'education', title: 'Education', display_order: 0,
        items: [{
          kind: 'education',
          institution: 'Unknown',
          degree: 'Self-taught Rust',
          date: '',
          entry_id: 'e1',
          source_id: 's1',
          education_type: 'self_taught',
          edu_description: 'Learned Rust through open-source contributions and The Rust Book.',
        }],
      }

      const result = sb2nov.renderSection(section)
      expect(result).toContain('\\resumeItem{Learned Rust through open-source contributions and The Rust Book.}')
      expect(result).not.toContain('\\resumeSubheading')
    })

    test('self_taught without edu_description falls back to degree', () => {
      const section: IRSection = {
        id: 's1', type: 'education', title: 'Education', display_order: 0,
        items: [{
          kind: 'education',
          institution: 'Unknown',
          degree: 'Self-taught Python',
          date: '',
          entry_id: 'e1',
          source_id: 's1',
          education_type: 'self_taught',
        }],
      }

      const result = sb2nov.renderSection(section)
      expect(result).toContain('\\resumeItem{Self-taught Python}')
    })

    test('education without education_type defaults to degree rendering', () => {
      const section: IRSection = {
        id: 's1', type: 'education', title: 'Education', display_order: 0,
        items: [{
          kind: 'education',
          institution: 'State University',
          degree: 'B.A. English',
          date: '2018',
          entry_id: 'e1',
          source_id: 's1',
          // No education_type -- pre-migration data
        }],
      }

      const result = sb2nov.renderSection(section)
      expect(result).toContain('\\resumeSubheading')
      expect(result).toContain('{State University}{}')
      expect(result).toContain('{B.A. English}{2018}')
    })
  })
```

### Smoke Tests -- UI

Manual verification checklist (not automated):

1. Navigate to Sources -> New Source -> Education.
2. Select "Degree" -- verify degree_level dropdown, degree_type input, institution, location, field, GPA, start/end dates, in-progress, description visible.
3. Switch to "Certificate" -- verify degree fields disappear, certificate_subtype dropdown, issuing body, credential ID, URL, expiration visible.
4. Switch to "Course" -- verify institution, location, start/end dates, URL, description visible.
5. Switch to "Self-Taught" -- verify only description (6 rows) and URL visible.
6. Create a degree source with GPA "3.8/4.0" -- verify save succeeds and value round-trips.
7. Navigate to role source form -- verify org dropdown only shows orgs with existing role sources.
8. Navigate to project source form -- verify org dropdown only shows orgs with existing role sources.

---

## Documentation Requirements

- No new documentation files are created by this phase.
- The spec file (`2026-04-03-education-subtype-fields.md`) serves as the authoritative reference.
- The field relevance table in the spec (Section 1) documents which fields apply to which education_type.
- Future normalization options are documented in `docs/brainstorm/education-type-model-future.md` (already exists, not modified here).

---

## Parallelization Notes

- **T37.1** (migration) has no code dependencies and can be written first.
- **T37.2** (core types) and **T37.3** (SDK types) can be done in parallel.
- **T37.4** (repo create) and **T37.5** (repo updateExtension) depend on T37.2 for type definitions. They modify different functions in the same file (`source-repository.ts`). Execute sequentially in a single editing session -- do not dispatch to separate agents.
- **T37.6** (compiler) depends on T37.2 (EducationItem type changes).
- **T37.7** (template) depends on T37.6 (needs enriched EducationItem).
- **T37.8** (UI) depends on T37.3 (SDK types) and T37.4/T37.5 (API must handle new fields).
- **Tests** should be written alongside their corresponding tasks.

**Suggested execution order:**
1. T37.0 (verify migration numbering prerequisite)
2. T37.0.1 (route translation layer)
3. T37.1 (migration)
4. T37.2 + T37.3 (types, parallel)
5. T37.4 then T37.5 (repository, sequential -- same file)
6. T37.6 (compiler)
7. T37.7 (template)
8. T37.8 (UI)
9. All tests
