# Education Sub-Type Fields

**Date:** 2026-04-03
**Spec:** 9 (Education Sub-Types)
**Phase:** TBD (next available)
**Builds on:** Migration 002 (`source_education` table)
**Related:** [Future approaches brainstorm](../../../docs/brainstorm/education-type-model-future.md)

**PREREQUISITE:** Migration `005_job_descriptions.sql` must be renamed to `007_job_descriptions.sql` before this migration (009) is created. Verify the migration directory has sequential numbering 001-008 before proceeding.

## Overview

The `source_education` extension table currently uses a flat set of nullable columns shared across all four `education_type` values (`degree`, `certificate`, `course`, `self_taught`). This spec adds 6 new columns to support sub-type-specific fields — degree level/type, certificate sub-type, GPA, location, and edu_description — and updates the UI to conditionally show fields based on the selected education type.

**Approach:** Add columns to the existing flat table (Approach 1). No new tables. Future normalization options are documented separately.

## 1. Schema Changes

Migration adds 6 columns via ALTER TABLE. All nullable to maintain flat-table simplicity. Validation is UI-enforced, not DB-enforced.

```sql
-- Education Sub-Type Fields
-- Migration: 009_education_subtype_fields
-- Adds degree_level, degree_type, certificate_subtype, gpa, location, edu_description
-- to source_education for per-type field support.

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

### Column definitions

| Column | Type | Constraint | Purpose |
|--------|------|-----------|---------|
| `degree_level` | TEXT | CHECK enum, nullable | `associate`, `bachelors`, `masters`, `doctoral`, `graduate_certificate` |
| `degree_type` | TEXT | nullable | Free text: "BS", "MS", "PhD", "MBA", "MPA", "BEng", etc. |
| `certificate_subtype` | TEXT | CHECK enum, nullable | `professional` (CISSP, PE, PMP), `vendor` (AWS, Azure, CompTIA), `completion` (Udemy, bootcamp) |
| `gpa` | TEXT | nullable | Free text to support "3.8/4.0" format |
| `location` | TEXT | nullable | City/state for degrees and courses |
| `edu_description` | TEXT | nullable | Free-form description; primary field for self_taught. Named `edu_description` to avoid collision with the base `sources.description` column. The UI label remains "Description". |

### Field relevance by education_type

| Field | degree | certificate | course | self_taught |
|-------|--------|-------------|--------|-------------|
| institution | Yes | Yes (issuing body) | Yes (provider) | -- |
| field | Yes (major/focus) | -- | -- | -- |
| degree_level | **UI-required** | -- | -- | -- |
| degree_type | **UI-required** | -- | -- | -- |
| certificate_subtype | -- | **UI-required** | -- | -- |
| gpa | Optional | -- | -- | -- |
| location | Optional | -- | Optional | -- |
| edu_description | Optional | Optional | Optional | **Primary field** |
| start_date | Yes | -- | Optional | -- |
| end_date | Yes | Yes (expiration) | Optional | -- |
| is_in_progress | Yes | -- | -- | -- |
| credential_id | -- | Yes | -- | -- |
| expiration_date | -- | Yes | -- | -- |
| issuing_body | -- | Yes | -- | -- |
| url | -- | Yes | Optional | Optional |

"UI-required" means the UI enforces it (shows validation), but the DB column stays nullable.

## 2. Type Changes

### 2.1 Core types (`packages/core/src/types/index.ts`)

Update `SourceEducation`:

> Graduate certificates are classified as a degree level (not a certificate subtype) because they come from accredited institutions, may have GPA/coursework, and represent academic rigor similar to a mini-graduate degree — unlike professional/vendor/completion certificates which are issued by industry bodies.

Note: `url` already exists on `SourceEducation` and `CreateSource` — it is not a new field.

```typescript
export interface SourceEducation {
  source_id: string
  education_type: 'degree' | 'certificate' | 'course' | 'self_taught'
  // Shared
  institution: string | null
  edu_description: string | null
  location: string | null
  start_date: string | null
  end_date: string | null
  url: string | null
  // Degree-specific
  degree_level: 'associate' | 'bachelors' | 'masters' | 'doctoral' | 'graduate_certificate' | null
  degree_type: string | null
  field: string | null
  gpa: string | null
  is_in_progress: number
  // Certificate-specific
  certificate_subtype: 'professional' | 'vendor' | 'completion' | null
  credential_id: string | null
  expiration_date: string | null
  issuing_body: string | null
}
```

The new fields (`degree_level`, `degree_type`, `certificate_subtype`, `gpa`, `location`, `edu_description`) are added to the `SourceEducation` interface only. `CreateSource` and `UpdateSource` already reference `SourceEducation` via their education extension fields.

### 2.2 SDK types (`packages/sdk/src/types.ts`)

The SDK's `CreateSource` and `UpdateSource` types nest education fields under an `education` key (`education?: Partial<SourceEducation>`). The new fields are added to `SourceEducation`, which is already nested. The API route handler already spreads `input.education` into the repository's flat field structure — no additional translation is needed for the new columns.

The SDK `SourceEducation` intentionally omits `source_id` (the SDK represents the resource from a consumer perspective). Add only the new data fields to the SDK's `SourceEducation` — do not add `source_id`.

Mirror the new fields on the SDK's `SourceEducation` interface. Use identical union types.

### 2.3 New union types

```typescript
export type DegreeLevelType = 'associate' | 'bachelors' | 'masters' | 'doctoral' | 'graduate_certificate'
export type CertificateSubtype = 'professional' | 'vendor' | 'completion'
export type EducationType = 'degree' | 'certificate' | 'course' | 'self_taught'
```

These can be used in both core and SDK types for DRY definitions.

## 3. Repository Changes

### 3.1 `source-repository.ts` — `create()`

Add the 6 new fields to the education INSERT:

```typescript
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

### 3.2 `source-repository.ts` — `updateExtension()`

Add new fields to the education update block:

```typescript
if ('degree_level' in input) { sets.push('degree_level = ?'); params.push(input.degree_level ?? null) }
if ('degree_type' in input) { sets.push('degree_type = ?'); params.push(input.degree_type ?? null) }
if ('certificate_subtype' in input) { sets.push('certificate_subtype = ?'); params.push(input.certificate_subtype ?? null) }
if ('gpa' in input) { sets.push('gpa = ?'); params.push(input.gpa ?? null) }
if ('location' in input) { sets.push('location = ?'); params.push(input.location ?? null) }
if ('edu_description' in input) { sets.push('edu_description = ?'); params.push(input.edu_description ?? null) }
```

## 4. UI Changes

### 4.1 Organization dropdown filter

In `SourcesView.svelte`, the organization dropdown currently loads all organizations. Change to filter by orgs that have at least one `source_roles` entry:

```typescript
// Instead of: const orgResult = await forge.organizations.list()
// Use a filtered approach:
const orgResult = await forge.organizations.list()
const sourceResult = await forge.sources.list({ source_type: 'role' })
const workedOrgIds = new Set(sourceResult.ok ? sourceResult.data.map(s => s.organization_id).filter(Boolean) : [])
organizations = orgResult.ok ? orgResult.data.filter(o => workedOrgIds.has(o.id)) : []
```

This organization dropdown filter applies to **role** and **project** source type forms only — education sources do not have an organization dropdown. The filter is included in this spec because it was identified during the education type design discussion as a general improvement needed across source forms.

### 4.2 Conditional education form

Replace the current flat education form with conditional sections based on `formEducationType`:

**Degree form:**
```
[Education Type: Degree ▾]
[Degree Level: Masters ▾]  [Degree Type: MS        ]
[Institution:                ] [Location:            ]
[Field (Major):              ] [GPA:                 ]
[Start Date:    ] [End Date:    ] [ ] In Progress
[Description:                                        ]
```

**Certificate form:**
```
[Education Type: Certificate ▾]
[Certificate Type: Vendor ▾   ]
[Issuing Body:                ] [Credential ID:      ]
[URL:                                                ]
[Expiration Date:    ]
[Description:                                        ]
```

**Course form:**
```
[Education Type: Course ▾]
[Institution (Provider):      ] [Location:            ]
[Start Date:    ] [End Date:    ]
[URL:                                                ]
[Description:                                        ]
```

**Self-Taught form:**
```
[Education Type: Self-Taught ▾]
[Description (textarea, larger):                     ]
[                                                    ]
[URL:                                                ]
```

### 4.3 New form state variables

```typescript
let formDegreeLevel = $state<string>('bachelors')
let formDegreeType = $state('')
let formCertificateSubtype = $state<string>('vendor')
let formGpa = $state('')
let formLocation = $state('')
let formEduDescription = $state('')
```

Reset logic in `startNew()` and `startEditing()` must include these new fields.

In `startEditing()` / `populateFormFromSource()`, populate the new form state from the stored education extension:
```typescript
formDegreeLevel = source.education?.degree_level ?? 'bachelors'
formDegreeType = source.education?.degree_type ?? ''
formCertificateSubtype = source.education?.certificate_subtype ?? 'vendor'
formGpa = source.education?.gpa ?? ''
formLocation = source.education?.location ?? ''
formEduDescription = source.education?.edu_description ?? ''
```

GPA is free text with no format validation. The UI should use `type='text'` (not `type='number'`) to support formats like '3.8/4.0' or 'Summa Cum Laude'.

## 5. IR Compiler Enhancement

### 5.1 Query changes

`buildEducationItems` in `resume-compiler.ts` adds the new columns to its SELECT:

```sql
SELECT
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
ORDER BY re.position ASC
```

Update the `.all()` result cast (`as Array<{...}>`) to include all new columns: `education_type`, `degree_level`, `degree_type`, `gpa`, `location`, `credential_id`, `issuing_body`, `certificate_subtype`, `edu_description`.

### 5.2 IR type changes

Update `EducationItem` in `packages/core/src/types/index.ts` (not in the compiler file). Also mirror the changes in `packages/sdk/src/types.ts`.

Extend `EducationItem` with optional fields:

```typescript
export interface EducationItem {
  kind: 'education'
  institution: string
  degree: string              // perspective content or entry content
  date: string
  entry_id: string
  source_id: string
  // New optional fields
  education_type?: string
  degree_level?: string | null
  degree_type?: string | null
  gpa?: string | null
  location?: string | null
  credential_id?: string | null
  issuing_body?: string | null
  certificate_subtype?: string | null
  edu_description?: string | null
}
```

Map `row.edu_description` to `edu_description` in the returned EducationItem in `buildEducationItems`.

### 5.3 Rendering logic

The LaTeX template's education section rendering adapts per `education_type`:

**Degree:**
```latex
\resumeSubheading
  {Institution Name}{Location}
  {M.S. in Computer Science, GPA: 3.9/4.0}{Aug 2020 -- May 2022}
```
- Line 1: institution + location
- Line 2: degree_type + " in " + field + optional GPA + date range

**Certificate:**
```latex
\resumeSubheading
  {AWS Solutions Architect Professional}{Exp. Dec 2027}
  {Amazon Web Services -- Credential ID: ABC123}{}
```
- Line 1: title (from perspective/entry content) + expiration
- Line 2: issuing_body + credential_id

**Course:**
```latex
\resumeSubheading
  {SANS SEC504: Hacker Tools}{Mar 2024}
  {Black Hat Conference, Las Vegas}{}
```
- Line 1: title + date
- Line 2: institution + location

**Self-taught:** Self-taught entries in an `education` section render using only the perspective/entry content (same as the default case). The `edu_description` field is available for display but the LaTeX rendering uses the derived bullet/perspective content. If `education_type === 'self_taught'`, `renderEducationSection` in sb2nov.ts renders a simple `\resumeItem` with the content text (no subheading structure).

## 6. Files to Create

| File | Purpose |
|------|---------|
| `packages/core/src/db/migrations/009_education_subtype_fields.sql` | ALTER TABLE migration |

## 7. Files to Modify

| File | Change |
|------|--------|
| `packages/core/src/types/index.ts` | Update `SourceEducation`, `EducationItem`; add union types |
| `packages/sdk/src/types.ts` | Mirror type changes including `SourceEducation` and `EducationItem` |
| `packages/core/src/db/repositories/source-repository.ts` | Update `create()` and `updateExtension()` for new fields |
| `packages/core/src/services/resume-compiler.ts` | Extend `buildEducationItems` query + mapping |
| `packages/core/src/templates/sb2nov.ts` | Update education section rendering for per-type formatting |
| `packages/webui/src/routes/data/sources/SourcesView.svelte` | Conditional education form; org dropdown filter; new form state |
| `packages/sdk/src/index.ts` | Export new union types |

## 8. Testing

### Unit tests
- Repository: create education source with each type (degree/certificate/course/self_taught) + verify new fields persisted and retrieved
- Repository: update education source — change degree_level, verify round-trip
- Repository: create degree with gpa "3.8/4.0" — verify text preservation

### Integration tests
- API: POST source with education_type=degree, degree_level=masters, degree_type=MS — verify response
- API: PATCH source education to change certificate_subtype — verify response
- API: GET source with education extension — verify all new fields present

### Contract tests
- SDK types include new fields on SourceEducation
- Null fields are properly returned (not omitted)

### Compiler tests
- Compile resume with degree section — verify IR includes degree_level, degree_type, gpa, location
- Compile resume with certificate section — verify IR includes credential_id, issuing_body, certificate_subtype
- Compile resume with self_taught section — verify freeform rendering

### Template tests
Add test cases to `packages/core/src/templates/__tests__/sb2nov.test.ts`:
- (a) degree renders with 'M.S. in Field, GPA: X' format
- (b) certificate renders with issuing body and credential ID
- (c) course renders with institution and location
- (d) self-taught renders as simple resumeItem

## 9. Non-Goals

- Sub-type extension tables (Approach 2 — see future brainstorm doc)
- JSON extension column (Approach 3 — see future brainstorm doc)
- Cross-entity relationships (self_taught ↔ projects)
- Changes to resume section entry_types (certifications already handled in Phase 36)
- DB-level "required" enforcement for degree_level on degrees (UI-enforced only)

## 10. Acceptance Criteria

1. Migration 009 adds 6 columns to `source_education` without data loss
2. Creating a degree source with `degree_level=masters, degree_type=MS, gpa=3.9/4.0` persists and round-trips correctly
3. Creating a certificate source with `certificate_subtype=vendor` persists correctly
4. UI shows only relevant fields per education_type selection
5. Organization dropdown in source forms shows only orgs where user has role sources
6. IR compiler produces enriched EducationItem with new fields
7. LaTeX renders degree with "M.S. in Field, GPA: X" format
8. LaTeX renders certificate with issuing body and credential ID
9. Self-taught renders as freeform text
10. All existing education sources continue to work (new columns default to NULL)
11. The `edu_description` field on `source_education` is independent of the base `sources.description` field — updating one does not affect the other
