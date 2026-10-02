# Phase 40: Organization Model P2+P3 Polish

**Status:** Planning
**Date:** 2026-04-03
**Spec:** [2026-04-03-org-model-evolution.md](../refs/specs/2026-04-03-org-model-evolution.md) -- Part B items 1, 4, 6, 8, 9, 12
**Depends on:** Phase 39 (P1 cleanup -- searchable dropdown replaces old selects, stale payload fields removed, campus editing done)
**Blocks:** None currently identified
**Parallelizable with:** Any phase that does not touch `source-repository.ts`, `organization-repository.ts`, `KanbanCard.svelte`, `OrgPickerModal.svelte`, or the `/data/organizations` page

## Goal

Complete six remaining cleanup items from the organization model spec. Two table-rebuild migrations remove deprecated columns (`institution`/`issuing_body` from `source_education`, `location`/`headquarters` from `organizations`). A one-time data cleanup NULLs stale text values before the columns are dropped. The `CreateOrganizationInput.status` type gets narrowed from `string` to the kanban status union. The `org_type` vs tags redundancy is resolved by documenting `org_type` as the "primary type" (Option B -- no migration). The kanban card and org list panel gain alias/campus enrichment. The `OrgPickerModal` gets a tag filter dropdown for the kanban picker.

## Non-Goals

- **Removing `org_type` entirely (Option A)** -- Kept as primary type. No table rebuild for this column.
- **SDK resource methods for campus/alias** -- Deferred (Phase 39 non-goal carries forward).
- **Full alias search in org cards** -- Cards show alias count/first alias, not a full searchable alias display.
- **Campus inline edit improvements beyond Phase 39** -- Phase 39 covers the PATCH route and inline UI.

## Context

After Phase 39, the four P1 gaps are closed: campus editing works, the searchable OrgCombobox replaces plain `<select>` elements, stale payload fields are removed, and campus data flows into the IR compiler. Six items remain at P2/P3 priority:

1. **Legacy column removal** -- `institution`/`issuing_body` on `source_education` and `location`/`headquarters` on `organizations` are deprecated but still in schema. Removing them requires table-rebuild migrations.
2. **Stale text data cleanup** -- Some `source_education` rows may have stale `institution`/`issuing_body` text that does not match the linked org. Must be cleaned before column removal.
3. **`org_type` vs tags redundancy** -- Two sources of truth for the same concept. Needs a documented decision.
4. **`CreateOrganizationInput.status` typing** -- Raw `string` instead of `OrganizationStatus` union.
5. **Org card enrichment** -- List panels missing alias pills and campus counts.
6. **Tag filtering in kanban picker** -- `OrgPickerModal` has a tag select but no default filter for the employer pipeline context.

## Scope

| Spec item | Part B # | Covered here? |
|-----------|----------|---------------|
| Legacy column removal (institution/issuing_body/location/headquarters) | 1 | Yes |
| org_type vs tags redundancy (document decision) | 6 | Yes |
| `CreateOrganizationInput.status` typing | 12 | Yes |
| Stale text field cleanup | 9 | Yes |
| Org card detail enrichment | 4 | Yes |
| Tag-based kanban picker filter | 8 | Yes |
| Campus editing | 2 | No (Phase 39) |
| Searchable education dropdown | 3 | No (Phase 39) |
| Remove location/headquarters from save payload | 5 | No (Phase 39) |
| Campus in IR compiler | 7 | No (Phase 39) |
| SDK campus/alias resource methods | 10 | No (deferred) |
| OrganizationStatus type stale in core | 11 | No (already fixed -- type matches migration 012) |
| roleFilteredOrgs missing project source type | 13 | No (separate fix) |

## Files to Create

| File | Description |
|------|-------------|
| `packages/core/src/db/migrations/016_stale_education_text_cleanup.sql` | One-time data fix: NULL out `institution`/`issuing_body` where `organization_id` is set |
| `packages/core/src/db/migrations/017_drop_legacy_education_columns.sql` | Table rebuild: remove `institution`, `issuing_body` from `source_education` |
| `packages/core/src/db/migrations/018_drop_legacy_org_location_columns.sql` | Table rebuild: remove `location`, `headquarters` from `organizations` |

## Files to Modify

| File | Change |
|------|--------|
| `packages/core/src/db/repositories/organization-repository.ts` | Remove `location`/`headquarters` from `CreateOrganizationInput`, INSERT, and update(); change `status` from `string` to `OrganizationStatus` |
| `packages/core/src/db/repositories/source-repository.ts` | Remove `institution`/`issuing_body` from education INSERT and updateExtension() |
| `packages/core/src/types/index.ts` | Remove `location`/`headquarters` from `Organization`; remove `institution`/`issuing_body` from `SourceEducation`; remove from `CreateSource`/`UpdateSource` |
| `packages/sdk/src/types.ts` | Remove `location`/`headquarters` from `Organization`, `CreateOrganization`, `UpdateOrganization`; remove `institution`/`issuing_body` from `SourceEducation` |
| `packages/webui/src/lib/components/kanban/KanbanCard.svelte` | Add alias count and campus/HQ location line |
| `packages/webui/src/lib/components/kanban/OrgPickerModal.svelte` | Default tag filter to `company` |
| `packages/webui/src/lib/components/kanban/OrgDetailModal.svelte` | Remove `org.location` block; optionally replace with HQ campus city/state |
| `packages/webui/src/routes/data/organizations/+page.svelte` | Add alias count and campus count to org list cards |

## Fallback Strategies

1. **Migration failure on existing data** -- If the stale text cleanup migration (016) reveals `source_education` rows with `organization_id IS NULL` and non-empty `institution`/`issuing_body`, the column removal migration (017) will still proceed because it copies `organization_id` and all other non-deprecated columns. Orphaned text data is lost, which is acceptable because it was stale. However, if the operator wants to preserve it, they can skip 016 and manually run the verification query first.
2. **Table rebuild PRAGMA transaction conflict** -- The migration runner wraps each file in BEGIN/COMMIT. `PRAGMA foreign_keys = OFF` is silently ignored inside a transaction. This is the known pattern from migrations 002, 007, and 012. The PRAGMA calls are defensive only -- atomicity comes from the runner's transaction.
3. **Type removal breaks downstream code** -- If removing `location`/`headquarters` from the `Organization` interface causes compile errors in unexpected places, the fallback is to mark them as `@deprecated` and set to `null` rather than removing from the type. But the Phase 39 payload cleanup should have already caught the main usage sites.
4. **Org card enrichment performance** -- If loading alias counts and campus data for every org in the list is too slow, the enrichment can be limited to the selected org only (show details in the editor panel instead of the list card).

---

## Tasks

### Task 40.1 -- Stale Text Data Cleanup Migration

**File:** `packages/core/src/db/migrations/016_stale_education_text_cleanup.sql` (new)

This migration NULLs out `institution` and `issuing_body` on all `source_education` rows where `organization_id IS NOT NULL`, confirming the FK is authoritative. This must run BEFORE the column removal migration.

```sql
-- Stale Education Text Cleanup
-- Migration: 016_stale_education_text_cleanup
-- Date: 2026-04-03
-- Clears deprecated institution/issuing_body text columns on source_education
-- rows where organization_id is already set. The FK is authoritative; the text
-- columns contained stale duplicates that could diverge from the org name.
-- Must run BEFORE 017 (which drops these columns entirely).

-- Verification: log how many rows have organization_id set vs text still populated.
-- (SQLite has no RAISE for warnings, so we just do the update.)

-- Clear institution where the FK is set
UPDATE source_education
SET institution = NULL
WHERE organization_id IS NOT NULL
  AND institution IS NOT NULL;

-- Clear issuing_body where the FK is set
UPDATE source_education
SET issuing_body = NULL
WHERE organization_id IS NOT NULL
  AND issuing_body IS NOT NULL;

INSERT INTO _migrations (name) VALUES ('016_stale_education_text_cleanup');
```

**Acceptance criteria:**
- After migration, `SELECT COUNT(*) FROM source_education WHERE organization_id IS NOT NULL AND (institution IS NOT NULL OR issuing_body IS NOT NULL)` returns 0.
- Rows where `organization_id IS NULL` retain their `institution`/`issuing_body` values unchanged (they have no FK replacement yet).
- The migration is idempotent -- running it on an already-clean database is a no-op.

**Failure criteria:**
- Must not update rows where `organization_id IS NULL`. Those rows still depend on the text columns for identification.

---

### Task 40.2 -- Drop Legacy Education Columns Migration

**File:** `packages/core/src/db/migrations/017_drop_legacy_education_columns.sql` (new)

Table rebuild of `source_education` to remove the `institution` and `issuing_body` columns. Follows the CREATE new -> INSERT SELECT -> DROP old -> RENAME pattern from migration 012.

```sql
-- Drop Legacy Education Columns
-- Migration: 017_drop_legacy_education_columns
-- Date: 2026-04-03
-- Removes deprecated institution and issuing_body columns from source_education.
-- These were replaced by organization_id FK in migration 010.
-- Stale text values were cleaned in migration 016.
-- Uses table rebuild pattern (SQLite cannot ALTER TABLE DROP COLUMN on STRICT tables).

PRAGMA foreign_keys = OFF;

-- **Note:** The migration runner wraps each migration in BEGIN/COMMIT.
-- PRAGMA foreign_keys = OFF is silently ignored inside an active transaction.
-- The PRAGMA calls are defensive only -- the actual FK protection comes from
-- the runner's transaction ensuring all statements execute atomically.
-- This is consistent with how migrations 002, 007, and 012 handle table rebuilds.

CREATE TABLE source_education_new (
  source_id TEXT PRIMARY KEY CHECK(typeof(source_id) = 'text' AND length(source_id) = 36)
    REFERENCES sources(id) ON DELETE CASCADE,
  education_type TEXT NOT NULL CHECK (education_type IN ('degree', 'certificate', 'course', 'self_taught')),
  organization_id TEXT REFERENCES organizations(id) ON DELETE SET NULL,
  campus_id TEXT REFERENCES org_campuses(id) ON DELETE SET NULL,
  field TEXT,
  start_date TEXT,
  end_date TEXT,
  is_in_progress INTEGER NOT NULL DEFAULT 0,
  credential_id TEXT,
  expiration_date TEXT,
  url TEXT,
  degree_level TEXT CHECK (degree_level IS NULL OR degree_level IN (
    'associate', 'bachelors', 'masters', 'doctoral', 'graduate_certificate'
  )),
  degree_type TEXT,
  certificate_subtype TEXT CHECK (certificate_subtype IS NULL OR certificate_subtype IN (
    'professional', 'vendor', 'completion'
  )),
  gpa TEXT,
  location TEXT,
  edu_description TEXT
) STRICT;

INSERT INTO source_education_new (
  source_id, education_type, organization_id, campus_id, field,
  start_date, end_date, is_in_progress, credential_id, expiration_date,
  url, degree_level, degree_type, certificate_subtype, gpa,
  location, edu_description
)
SELECT
  source_id, education_type, organization_id, campus_id, field,
  start_date, end_date, is_in_progress, credential_id, expiration_date,
  url, degree_level, degree_type, certificate_subtype, gpa,
  location, edu_description
FROM source_education;

DROP TABLE source_education;
ALTER TABLE source_education_new RENAME TO source_education;

-- Recreate index on organization_id (from migration 010)
CREATE INDEX idx_source_education_org ON source_education(organization_id);

PRAGMA foreign_keys = ON;

INSERT INTO _migrations (name) VALUES ('017_drop_legacy_education_columns');
```

**Acceptance criteria:**
- `PRAGMA table_info(source_education)` does not include `institution` or `issuing_body`.
- All other columns are preserved with their data intact.
- The `idx_source_education_org` index exists after the rebuild.
- Deleting an `org_campuses` row sets `source_education.campus_id` to NULL (ON DELETE SET NULL preserved, not changed to CASCADE).
- FK constraints on `source_id`, `organization_id`, and `campus_id` are intact.
- Existing `source_education` rows have identical data in all non-dropped columns before and after migration.

**Failure criteria:**
- Must not use `SELECT *` in the INSERT -- explicit column lists only.
- Must not lose any rows during the rebuild.

---

### Task 40.3 -- Drop Legacy Organization Location Columns Migration

**File:** `packages/core/src/db/migrations/018_drop_legacy_org_location_columns.sql` (new)

Table rebuild of `organizations` to remove the `location` and `headquarters` columns. These were replaced by `org_campuses` in migration 013.

```sql
-- Drop Legacy Organization Location Columns
-- Migration: 018_drop_legacy_org_location_columns
-- Date: 2026-04-03
-- Removes deprecated location and headquarters columns from organizations.
-- These were replaced by the org_campuses table in migration 013.
-- Phase 39 removed these fields from the save payload.
-- Uses table rebuild pattern (SQLite cannot ALTER TABLE DROP COLUMN on STRICT tables).

PRAGMA foreign_keys = OFF;

-- **Note:** The migration runner wraps each migration in BEGIN/COMMIT.
-- PRAGMA foreign_keys = OFF is silently ignored inside an active transaction.
-- The PRAGMA calls are defensive only -- the actual FK protection comes from
-- the runner's transaction ensuring all statements execute atomically.
-- This is consistent with how migrations 002, 007, and 012 handle table rebuilds.

CREATE TABLE organizations_new (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  name TEXT NOT NULL,
  org_type TEXT DEFAULT 'company' CHECK (org_type IN (
    'company', 'nonprofit', 'government', 'military',
    'education', 'volunteer', 'freelance', 'other'
  )),
  industry TEXT,
  size TEXT,
  worked INTEGER NOT NULL DEFAULT 0,
  employment_type TEXT CHECK (employment_type IN (
    'civilian', 'contractor', 'military_active',
    'military_reserve', 'volunteer', 'intern', NULL
  )),
  website TEXT,
  linkedin_url TEXT,
  glassdoor_url TEXT,
  glassdoor_rating REAL,
  reputation_notes TEXT,
  notes TEXT,
  status TEXT CHECK (status IS NULL OR status IN (
    'backlog', 'researching', 'exciting', 'interested', 'acceptable', 'excluded'
  )),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

INSERT INTO organizations_new (
  id, name, org_type, industry, size, worked, employment_type,
  website, linkedin_url, glassdoor_url, glassdoor_rating,
  reputation_notes, notes, status, created_at, updated_at
)
SELECT
  id, name, org_type, industry, size, worked, employment_type,
  website, linkedin_url, glassdoor_url, glassdoor_rating,
  reputation_notes, notes, status, created_at, updated_at
FROM organizations;

DROP TABLE organizations;
ALTER TABLE organizations_new RENAME TO organizations;

-- Recreate index on name (from migration 012)
CREATE INDEX idx_organizations_name ON organizations(name);

PRAGMA foreign_keys = ON;

-- Note: org_tags, org_campuses, org_aliases all reference organizations(id) via
-- ON DELETE CASCADE. Since we INSERT all rows into the new table with the same
-- IDs before dropping the old table, and PRAGMA foreign_keys is OFF during the
-- rebuild, no FK violations occur. After PRAGMA foreign_keys = ON, all child
-- table rows still reference valid organization IDs.
--
-- source_roles, source_projects, source_education also reference organizations(id)
-- via ON DELETE SET NULL. Same logic applies -- IDs are preserved.

INSERT INTO _migrations (name) VALUES ('018_drop_legacy_org_location_columns');
```

**Acceptance criteria:**
- `PRAGMA table_info(organizations)` does not include `location` or `headquarters`.
- All other columns are preserved with their data intact.
- The `idx_organizations_name` index exists after the rebuild.
- `org_tags`, `org_campuses`, `org_aliases` rows still reference valid organization IDs.
- `source_roles.organization_id`, `source_projects.organization_id`, `source_education.organization_id` FK references are still valid.
- The `status` CHECK constraint matches migration 012's set (backlog/researching/exciting/interested/acceptable/excluded).
- Run `SELECT sql FROM sqlite_master WHERE type='table' AND name='organizations'` and verify the CHECK constraint includes all six status values.

**Failure criteria:**
- Must not use `SELECT *` in the INSERT -- explicit column lists only.
- Must not lose any rows during the rebuild.
- Must not break cascade deletes on child tables.

---

### Task 40.4 -- Remove Legacy Columns from TypeScript Types

**File:** `packages/core/src/types/index.ts`

**Step 4a -- Remove from `Organization` interface.** Remove lines with `location` and `headquarters`:

```typescript
// REMOVE these two lines from the Organization interface:
  location: string | null
  headquarters: string | null
```

The `Organization` interface (currently lines 83-104) becomes:

```typescript
export interface Organization {
  id: string
  name: string
  org_type: string
  tags: OrgTag[]
  industry: string | null
  size: string | null
  worked: number
  employment_type: string | null
  website: string | null
  linkedin_url: string | null
  glassdoor_url: string | null
  glassdoor_rating: number | null
  reputation_notes: string | null
  notes: string | null
  status: OrganizationStatus | null
  created_at: string
  updated_at: string
}
```

**Step 4b -- Remove from `SourceEducation` interface.** Remove the `institution` and `issuing_body` fields and their `@deprecated` JSDoc comments:

```typescript
// REMOVE these lines from the SourceEducation interface:
  /** @deprecated Use organization_id + join. Kept for legacy data reads. */
  institution: string | null
  /** @deprecated Use organization_id + join. Kept for legacy data reads. */
  issuing_body: string | null
```

The `SourceEducation` interface (currently lines 205-231) becomes:

```typescript
export interface SourceEducation {
  source_id: string
  education_type: EducationType
  // Shared
  organization_id: string | null
  campus_id: string | null
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
}
```

**Step 4c -- Remove from `CreateSource` interface.** Remove the `institution` and `issuing_body` fields:

```typescript
// REMOVE these lines from CreateSource:
  institution?: string
  issuing_body?: string
```

**Step 4d -- Remove from `UpdateSource` interface.** Remove the `institution` and `issuing_body` fields:

```typescript
// REMOVE these lines from UpdateSource:
  institution?: string | null
  issuing_body?: string | null
```

**Acceptance criteria:**
- `tsc --noEmit` passes with no errors referencing `institution`, `issuing_body`, `location` (on Organization), or `headquarters`.
- The `Organization` interface no longer has `location` or `headquarters` fields.
- The `SourceEducation` interface no longer has `institution` or `issuing_body` fields.
- `CreateSource` and `UpdateSource` no longer have `institution` or `issuing_body` fields.

**Failure criteria:**
- If any code in the codebase reads `org.location`, `org.headquarters`, `edu.institution`, or `edu.issuing_body`, those references must also be updated or removed in this task. Check with `grep -r 'org\.location\|org\.headquarters\|\.institution\|\.issuing_body' packages/`.

---

### Task 40.5 -- Remove Legacy Columns from SDK Types

**File:** `packages/sdk/src/types.ts`

**Step 5a -- Remove from `Organization` interface** (lines 214-215):

```typescript
// REMOVE these lines:
  location: string | null
  headquarters: string | null
```

**Step 5b -- Remove from `SourceEducation` interface** (lines 94 and 111):

```typescript
// REMOVE these lines:
  institution: string | null
  issuing_body: string | null
```

**Step 5c -- Remove from `CreateOrganization` interface** (lines 917-918):

```typescript
// REMOVE these lines:
  location?: string
  headquarters?: string
```

**Step 5d -- Remove from `UpdateOrganization` interface** (lines 935-936):

```typescript
// REMOVE these lines:
  location?: string | null
  headquarters?: string | null
```

**Step 5e -- Remove `institution` from the education fields in the SDK's `CreateSource`/`UpdateSource` interfaces** (if present at lines 643 and 656):

```typescript
// REMOVE these lines:
  institution: string
  issuing_body?: string | null
```

**Acceptance criteria:**
- SDK types compile without `location`/`headquarters` on Organization or `institution`/`issuing_body` on SourceEducation.
- No SDK consumer references the removed fields (check `grep -r 'location\|headquarters' packages/sdk/src/`).

**Failure criteria:**
- SDK resource methods that spread these fields into API calls must also be updated if they reference the removed fields.

---

### Task 40.6 -- Remove Legacy Columns from Repository Code

**File:** `packages/core/src/db/repositories/organization-repository.ts`

**Step 6a -- Remove `location` and `headquarters` from `CreateOrganizationInput`:**

```typescript
// REMOVE these lines from CreateOrganizationInput:
  location?: string
  headquarters?: string
```

**Step 6b -- Change `status` type from `string` to `OrganizationStatus | null`.** Since `update()` uses `Partial<CreateOrganizationInput>`, this type tightening applies to both create and update automatically.

```typescript
// CHANGE in CreateOrganizationInput:
// FROM:
  status?: string
// TO:
  status?: OrganizationStatus | null
```

And add the import at the top of the file:

```typescript
// CHANGE the import:
// FROM:
import type { Organization, OrgTag } from '../../types'
// TO:
import type { Organization, OrganizationStatus, OrgTag } from '../../types'
```

**Step 6c -- Remove `location` and `headquarters` from the `create()` function's INSERT statement.** The INSERT column list (line 95) and parameter list (lines 99-116) must exclude these two columns:

```typescript
export function create(db: Database, input: CreateOrganizationInput): Organization {
  const id = crypto.randomUUID()
  const row = db
    .query(
      `INSERT INTO organizations (id, name, org_type, industry, size, worked, employment_type, website, linkedin_url, glassdoor_url, glassdoor_rating, reputation_notes, notes, status)
       VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
       RETURNING *`,
    )
    .get(
      id,
      input.name,
      input.org_type ?? 'company',
      input.industry ?? null,
      input.size ?? null,
      input.worked ?? 0,
      input.employment_type ?? null,
      input.website ?? null,
      input.linkedin_url ?? null,
      input.glassdoor_url ?? null,
      input.glassdoor_rating ?? null,
      input.reputation_notes ?? null,
      input.notes ?? null,
      input.status ?? null,
    ) as Organization

  const tags = input.tags ?? [input.org_type ?? 'company']
  setTags(db, id, tags)

  return { ...row, tags: tags as OrgTag[] }
}
```

**Step 6d -- Remove `location` and `headquarters` from the `update()` function's conditional SET clauses:**

```typescript
// REMOVE these lines from the update() function:
  if (input.location !== undefined) { sets.push('location = ?'); params.push(input.location) }
  if (input.headquarters !== undefined) { sets.push('headquarters = ?'); params.push(input.headquarters) }
```

**Acceptance criteria:**
- `CreateOrganizationInput` no longer has `location`, `headquarters`, or untyped `status`.
- `status` is typed as `OrganizationStatus | null` (not `string`).
- The `create()` INSERT has 14 columns and 14 parameter placeholders (down from 16).
- The `update()` function does not reference `location` or `headquarters`.
- `tsc --noEmit` passes.

**Failure criteria:**
- The `OrganizationFilter` interface still has `status?: string` -- this can remain as `string` for backward compatibility with query parameters, or be tightened. Recommendation: leave as `string` since URL query params are strings.

---

### Task 40.7 -- Remove Legacy Columns from Source Repository

**File:** `packages/core/src/db/repositories/source-repository.ts`

**Step 7a -- Remove `institution` and `issuing_body` from the education INSERT in `create()`** (around line 204). The INSERT column list and VALUES must exclude these two columns:

Replace the existing education INSERT (lines 204-221) with:

```typescript
        db.query(
          `INSERT INTO source_education (
            source_id, education_type, organization_id, campus_id, field, start_date, end_date,
            is_in_progress, credential_id, expiration_date, url,
            degree_level, degree_type, certificate_subtype, gpa, location, edu_description
          ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`
        ).run(
          id,
          input.education_type ?? 'degree',
          input.education_organization_id ?? input.organization_id ?? null,
          input.campus_id ?? null,
          input.field ?? null,
          input.start_date ?? null,
          input.end_date ?? null,
          input.is_in_progress ?? 0,
          input.credential_id ?? null,
          input.expiration_date ?? null,
          input.url ?? null,
          input.degree_level ?? null,
          input.degree_type ?? null,
          input.certificate_subtype ?? null,
          input.gpa ?? null,
          input.location ?? null,
          input.edu_description ?? null,
        )
```

**Step 7b -- Remove `institution` and `issuing_body` from `updateExtension()`** (around lines 104 and 109). Remove the conditional SET clauses:

```typescript
// REMOVE these lines from the education case in updateExtension():
    if ('institution' in input) { sets.push('institution = ?'); params.push(input.institution ?? null) }
    if ('issuing_body' in input) { sets.push('issuing_body = ?'); params.push(input.issuing_body ?? null) }
```

**Acceptance criteria:**
- The education INSERT has 17 columns and 17 parameter placeholders (no `institution` or `issuing_body`).
- `updateExtension()` does not reference `institution` or `issuing_body`.
- Creating an education source works without providing `institution` or `issuing_body`.
- Updating an education source does not attempt to SET `institution` or `issuing_body`.

**Failure criteria:**
- If any route handler or service spreads `institution`/`issuing_body` from the request body into the input object, those references must be removed too. Check `grep -r 'institution\|issuing_body' packages/core/src/routes/`.

---

### Task 40.8 -- Document `org_type` vs Tags Decision

This is a documentation-only task. No code changes.

**Decision:** Option B -- Keep `org_type` as "primary type" alongside tags. Document the relationship.

**Rationale:**
- `org_type` serves as the "primary classification" (the org is fundamentally a company, or fundamentally an education institution).
- Tags are supplementary labels (an education institution can also be tagged `school`, `university`).
- `org_type` is used as the default tag seed on create -- if no tags are provided, `[org_type]` becomes the initial tag set.
- The `org_type` CHECK constraint has a different value set than tags (notably `education` maps to `university` tag), but this is intentional: `org_type` is a coarser classification.
- Removing `org_type` would require a table rebuild migration, breaking changes to all consumers, and a decision about how to derive "primary type" from tags. The benefit does not justify the churn for a single-user tool.

**Add a comment in `organization-repository.ts`** after the `CreateOrganizationInput` interface:

```typescript
// NOTE: org_type is the "primary classification" (company, education, etc.).
// Tags (via org_tags junction table) are supplementary labels (vendor, platform, etc.).
// org_type seeds the initial tag set on create: tags default to [org_type].
// The two are intentionally not 1:1 (org_type='education' maps to tag='university').
// See spec: 2026-04-03-org-model-evolution.md, Part B item 6.
```

**Acceptance criteria:**
- The comment exists in `organization-repository.ts` after the `CreateOrganizationInput` interface.
- The spec's Part B item 6 decision log records Option B as the chosen approach.

**Failure criteria:**
- No code changes beyond the comment. No migration. No type changes.

---

### Task 40.9 -- Org Card Enrichment in List Panel

**File:** `packages/webui/src/routes/data/organizations/+page.svelte`

Add alias count and campus HQ location to the org list cards. This requires loading alias and campus data alongside the org list.

**Step 9a -- Add state for enrichment data** (after existing state declarations, around line 18):

```typescript
  // Enrichment data: alias counts and HQ locations per org
  let aliasCountMap = $state<Map<string, number>>(new Map())
  let hqLocationMap = $state<Map<string, string>>(new Map())
```

**Step 9b -- Load enrichment data after loading organizations.** Add `OrgCampus` to the import from `@forge/sdk`. Update `loadOrganizations()` to also fetch alias counts and HQ campus data:

```typescript
  async function loadOrganizations() {
    loading = true
    const result = await forge.organizations.list({ limit: 500 })
    if (result.ok) {
      organizations = result.data
      // Load enrichment data in parallel
      loadEnrichmentData(result.data)
    } else {
      addToast({ message: friendlyError(result.error, 'Failed to load organizations'), type: 'error' })
    }
    loading = false
  }

  async function loadEnrichmentData(orgs: Organization[]) {
    const aliasMap = new Map<string, number>()
    const hqMap = new Map<string, string>()

    // Load aliases and campuses for all orgs in parallel (batched)
    const promises = orgs.map(async (org) => {
      try {
        const [aliasRes, campusRes] = await Promise.all([
          fetch(`/api/organizations/${org.id}/aliases`),
          fetch(`/api/organizations/${org.id}/campuses`),
        ])
        if (aliasRes.ok) {
          const aliasBody = await aliasRes.json()
          const aliases = aliasBody.data ?? []
          if (aliases.length > 0) {
            aliasMap.set(org.id, aliases.length)
          }
        }
        if (campusRes.ok) {
          const campusBody = await campusRes.json()
          const campuses = campusBody.data ?? []
          const hq = campuses.find((c: OrgCampus) => c.is_headquarters)
          if (hq && (hq.city || hq.state)) {
            hqMap.set(org.id, [hq.city, hq.state].filter(Boolean).join(', '))
          }
        }
      } catch {
        // Silently skip enrichment failures
      }
    })

    await Promise.all(promises)
    aliasCountMap = aliasMap
    hqLocationMap = hqMap
  }
```

**Step 9c -- Update the org card template** (lines 352-378). Replace the org card template in the `{#each filteredOrgs}` block (the `<button class='org-card'>` through its closing `</button>`). Preserve the outer `<li>` wrapper. Replace the existing card content block:

```svelte
        {#each filteredOrgs as org (org.id)}
          <li>
            <button
              class="org-card"
              class:selected={selectedId === org.id}
              onclick={() => selectOrg(org.id)}
            >
              <div class="card-top">
                <span class="card-title">{org.name}</span>
                {#if aliasCountMap.get(org.id)}
                  <span class="alias-count" title="Aliases">({aliasCountMap.get(org.id)})</span>
                {/if}
                {#if org.worked}
                  <span class="worked-badge">Worked</span>
                {/if}
              </div>
              <div class="card-tags">
                {#each (org.tags ?? []) as tag}
                  <span class="tag-pill">{tag}</span>
                {/each}
              </div>
              {#if org.industry || hqLocationMap.get(org.id)}
                <div class="card-meta">
                  {#if org.industry}<span class="meta-item">{org.industry}</span>{/if}
                  {#if hqLocationMap.get(org.id)}<span class="meta-item">{hqLocationMap.get(org.id)}</span>{/if}
                </div>
              {/if}
            </button>
          </li>
        {/each}
```

**Step 9d -- Add CSS for alias count** (append to existing `<style>` block):

```css
  .alias-count {
    font-size: 0.65rem;
    color: #9ca3af;
    font-weight: 400;
    flex-shrink: 0;
  }
```

**Acceptance criteria:**
- Org cards show an alias count in parentheses next to the name (e.g., "Western Governors University (2)") when aliases exist.
- Org cards show the HQ campus city/state in the meta line (replacing the old `org.location` reference).
- Orgs without aliases or HQ campuses show no extra data (no "(0)" or empty strings).
- The enrichment loads after the org list loads -- the list appears immediately, and enrichment data fills in asynchronously.

**Note:** If the org list exceeds 50, consider batching enrichment requests (e.g., process in chunks of 10) to avoid overwhelming the local server with 100+ simultaneous requests.

**Failure criteria:**
- Enrichment failures must not prevent the org list from rendering.
- The enrichment must not block the initial page load (async after list renders).

---

### Task 40.10 -- Kanban Card Enrichment

**File:** `packages/webui/src/lib/components/kanban/KanbanCard.svelte`

Add alias count and HQ location to kanban cards. Since the kanban board loads all orgs but not their sub-resources, we need to accept enrichment data as props.

**Step 10a -- Update props to accept enrichment data:**

```svelte
<script lang="ts">
  import type { Organization } from '@forge/sdk'

  let { org, onclick, aliasCount = 0, hqLocation = '' }: {
    org: Organization
    onclick: () => void
    aliasCount?: number
    hqLocation?: string
  } = $props()

  const INTEREST_STYLES: Record<string, { bg: string; border: string; badge: string; label: string }> = {
    exciting: { bg: '#f0fdf4', border: '#22c55e', badge: '#16a34a', label: 'EXCITING' },
    interested: { bg: '#eff6ff', border: '#3b82f6', badge: '#2563eb', label: 'INTERESTED' },
    acceptable: { bg: '#fafafa', border: '#9ca3af', badge: '#6b7280', label: 'ACCEPTABLE' },
  }

  let interest = $derived(INTEREST_STYLES[org.status ?? ''] ?? null)
  let isExcluded = $derived(org.status === 'excluded')
</script>
```

**Step 10b -- Update the card template.** Add alias count after the name and HQ location in the meta section:

```svelte
<div
  class="kanban-card"
  class:excluded={isExcluded}
  style:background={interest?.bg ?? '#ffffff'}
  style:border-left={interest ? `4px solid ${interest.border}` : '1px solid #e5e7eb'}
  onclick={onclick}
  role="button"
  tabindex="0"
  onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onclick() } }}
>
  <div class="card-header">
    <span class="card-name" class:strike={isExcluded}>{org.name}</span>
    {#if aliasCount > 0}
      <span class="alias-count">({aliasCount})</span>
    {/if}
    {#if org.worked}
      <span class="worked-badge">Worked</span>
    {/if}
  </div>

  {#if org.tags && org.tags.length > 0}
    <div class="tag-pills">
      {#each org.tags as tag}
        <span class="tag-pill">{tag}</span>
      {/each}
    </div>
  {/if}

  <div class="card-meta">
    {#if org.industry}
      <span class="meta-text">{org.industry}</span>
    {/if}
    {#if hqLocation}
      <span class="meta-text">{hqLocation}</span>
    {/if}
  </div>

  {#if interest}
    <span class="interest-badge" style:background={interest.badge}>
      {interest.label}
    </span>
  {/if}
</div>
```

**Step 10c -- Add CSS for alias count** (append to existing `<style>` block):

```css
  .alias-count {
    font-size: 0.6rem;
    color: #9ca3af;
    font-weight: 400;
    flex-shrink: 0;
  }
```

**Step 10d -- Update the KanbanBoard to load and pass enrichment data.** The parent `KanbanBoard.svelte` (or the kanban page) must load alias counts and HQ locations for all pipeline orgs, then pass them to `KanbanCard`. Add enrichment maps to the board state and pass them through:

In the kanban board's page/component that renders `KanbanCard`, update the card invocation from:

```svelte
<KanbanCard org={org} onclick={() => openDetail(org)} />
```

To:

```svelte
<KanbanCard
  org={org}
  onclick={() => openDetail(org)}
  aliasCount={aliasCountMap.get(org.id) ?? 0}
  hqLocation={hqLocationMap.get(org.id) ?? ''}
/>
```

And add the enrichment loading logic (same pattern as Task 40.9 step 9b) to the kanban board's data loading function.

**Acceptance criteria:**
- Kanban cards show alias count in parentheses next to the org name when aliases exist.
- Kanban cards show HQ campus city/state in the meta section (replacing the deprecated `org.location` reference).
- Cards without aliases show no alias count indicator.
- Cards without an HQ campus show no location text.
- The card renders correctly with default props (`aliasCount=0`, `hqLocation=''`).

**Failure criteria:**
- The `org.location` reference in the existing template (line 49-51) must be removed -- the column no longer exists after migration 018.
- The card must not break if aliasCount/hqLocation props are not provided (default values handle this).

---

### Task 40.11 -- Tag Filter Default in OrgPickerModal

**File:** `packages/webui/src/lib/components/kanban/OrgPickerModal.svelte`

The `OrgPickerModal` already has a `tagFilter` state variable and a tag `<select>` dropdown (lines 16 and 130-135). The only change is to default the tag filter to `'company'` when the modal opens, since the kanban pipeline is primarily for employer vetting.

**Step 11a -- Update the `$effect` that runs when the modal opens** (lines 31-38). Change the `tagFilter` reset:

```typescript
  $effect(() => {
    if (open) {
      loadAllOrgs()
      searchQuery = ''
      tagFilter = 'company'  // Default to company for employer pipeline context
      showCreateForm = false
      collisionOrg = null
    }
  })
```

Also remove `org.location` reference at line ~162 of `OrgPickerModal.svelte`. After Task 40.4 removes `location` from the Organization type, this will be a compile error.

**Acceptance criteria:**
- When the OrgPickerModal opens, the tag filter dropdown defaults to "company" instead of "All tags".
- The user can still select "All tags" or any other tag to override the default.
- The available orgs list is filtered to only show orgs tagged "company" on initial open.
- When `tagFilter` is non-empty and `availableOrgs.length === 0`, the empty state message says 'No organizations with this tag. Try selecting All tags.' instead of the generic message.

**Failure criteria:**
- The default must not prevent adding non-company orgs. The user can always change the filter.
- The `tagFilter = 'company'` must match a value in the `TAG_OPTIONS` array (it does -- `company` is the first entry).

---

### Task 40.12 -- Remove `org.location` from OrgDetailModal

**File:** `packages/webui/src/lib/components/kanban/OrgDetailModal.svelte`

Remove the `org.location` block (lines ~147-151) from `OrgDetailModal.svelte`. After Task 40.4 removes `location` from the Organization type, this reference will be a compile error. Optionally replace with HQ campus city/state from enrichment data (same pattern as KanbanCard props in Task 40.10).

**Acceptance criteria:**
- No reference to `org.location` remains in `OrgDetailModal.svelte`.
- The component compiles cleanly after Task 40.4 type removal.
- If a replacement is added, it uses HQ campus data passed as a prop, not the removed `location` field.

**Failure criteria:**
- Must not break the modal layout if no HQ campus data is available.

---

## Testing Support

### Unit Tests

**Source repository education INSERT (Task 40.7):**
- Test `create()` with `source_type='education'` -- verify the INSERT succeeds without `institution` or `issuing_body` in the input.
- Test `updateExtension()` for education -- verify that passing `{ institution: 'X' }` does NOT add a SET clause (the field no longer exists).
- These tests belong in `packages/core/src/db/repositories/__tests__/source-repository.test.ts`.

**Organization repository (Task 40.6):**
- Test `create()` -- verify the INSERT succeeds without `location` or `headquarters`.
- Test `create()` with `status: 'backlog'` -- verify TypeScript accepts `OrganizationStatus` values.
- Test `update()` -- verify no `location` or `headquarters` SET clauses are generated.
- These tests belong in `packages/core/src/db/repositories/__tests__/organization-repository.test.ts` (if exists) or in the existing supporting-repositories test file.

### Migration Tests

**Data cleanup (Task 40.1):**
- Before running 016: insert a `source_education` row with `organization_id` set and `institution = 'Old Name'`.
- After running 016: verify `institution` is NULL on that row.
- Verify rows where `organization_id IS NULL` still have their `institution` value.

**Column removal (Tasks 40.2, 40.3):**
- After running 017: `PRAGMA table_info(source_education)` does not list `institution` or `issuing_body`.
- After running 018: `PRAGMA table_info(organizations)` does not list `location` or `headquarters`.
- Verify all rows are preserved (count matches before and after).
- Verify FK constraints work: deleting an organization sets `source_education.organization_id` to NULL.

### Component Smoke Tests

**Org card enrichment (Task 40.9):**
- Manual: navigate to `/data/organizations`, verify alias counts appear next to org names.
- Manual: verify HQ location appears in the card meta line.
- Manual: orgs without aliases show no parenthetical count.

**Kanban card enrichment (Task 40.10):**
- Manual: navigate to `/opportunities/organizations`, verify alias counts and HQ locations appear on pipeline cards.
- Manual: verify the deprecated `org.location` reference is gone (no stale text appears).

**Tag filter default (Task 40.11):**
- Manual: click "Add Organization" on the kanban board, verify the tag filter defaults to "company".
- Manual: change the filter to "All tags", verify all unassigned orgs appear.

---

## Documentation Requirements

No documentation files to create. After implementation:

1. Update the spec's Part B items 1, 4, 6, 8, 9, 12 to mark them as complete, referencing Phase 40.
2. The `org_type` vs tags decision is documented via the comment added in Task 40.8.
3. If no Decision Log table exists in the spec Part C, create one. Add entry: `2026-04-03 | Keep org_type as primary type alongside tags`.
4. Add additional Decision Log entries:
   - `2026-04-03 | Drop institution/issuing_body from source_education | organization_id FK is authoritative; text columns were stale duplicates`
   - `2026-04-03 | Drop location/headquarters from organizations | Replaced by org_campuses table; save payload already cleaned`

---

## Parallelization Notes

| Task group | Files touched | Can parallel with |
|-----------|--------------|-------------------|
| Tasks 40.1 + 40.2 + 40.3 (migrations) | New SQL files only | All code tasks (migrations run at startup, code changes are in TS/Svelte) |
| Tasks 40.4 + 40.5 (type removal) | types/index.ts, sdk/types.ts | Must complete BEFORE 40.6 + 40.7 (code depends on types) |
| Task 40.6 (org repository cleanup) | organization-repository.ts | Task 40.7 (different file) |
| Task 40.7 (source repository cleanup) | source-repository.ts | Task 40.6 (different file) |
| Task 40.8 (documentation) | organization-repository.ts (comment only) | All others (trivial change) |
| Task 40.9 (org list enrichment) | +page.svelte (data/organizations) | Task 40.10 (different file), Task 40.11 (different file) |
| Task 40.10 (kanban card enrichment) | KanbanCard.svelte, KanbanBoard.svelte | Task 40.9 (different file), Task 40.11 (different file) |
| Task 40.11 (tag filter default) | OrgPickerModal.svelte | Tasks 40.9, 40.10 (different files) |

**Deployment Note:** Migration files 016-018 must not be applied (server must not restart with a fresh migration run) until Tasks 40.4, 40.6, 40.7, 40.9, 40.10 are all code-complete. Write all migration SQL first, complete all code changes, then do a single clean migration run.

**Constraint:** Tasks 40.4 + 40.5 (type removal) must complete before Tasks 40.6 + 40.7 (repository code removal). The TS compiler will error if repository code references fields that no longer exist in the type interfaces.

**Constraint:** Tasks 40.1 -> 40.2 -> 40.3 must run in order (data cleanup before column drop, education before org). Migration file numbering enforces this.

**Migration numbering:** Verify Phase 39 does not add migrations. If it does, renumber Phase 40 migrations accordingly. The correct numbers are `max(existing) + 1` through `+3`.

**Constraint:** Task 40.10 must remove the `org.location` reference from KanbanCard.svelte, which will break if migration 018 has run but the code still references the field. Coordinate with Task 40.4 (type removal).

**Recommended execution order:**
1. Tasks 40.1 + 40.2 + 40.3 (write migration files -- no runtime effect until DB restart)
2. Tasks 40.4 + 40.5 (remove types -- enables compiler to catch stale references)
3. Tasks 40.6 + 40.7 (fix repository code to match new types) -- can run in parallel
4. Task 40.8 (add doc comment -- trivial)
5. Tasks 40.9 + 40.10 + 40.11 (UI enrichment -- all independent, can run in parallel)
