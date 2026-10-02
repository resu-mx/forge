# Security Clearance as Structured Datatype

**Date:** 2026-04-03
**Migration:** 017 (`clearance_structured_data`)
**Builds on:** Migration 002 (`source_clearances` table)
**Related specs:**
- [Org Model Evolution](./2026-04-03-org-model-evolution.md) (org FK pattern from migration 010)
- [Education Sub-Type Fields](./2026-04-03-education-subtype-fields.md) (enum CHECK constraint pattern)

## Overview

The `source_clearances` table currently stores all fields as free text. This creates data quality problems: "TS/SCI" vs "Top Secret/SCI" vs "top_secret", "CI" vs "ci" vs "CI Polygraph", "active" vs "Active" vs "current". It also prevents hierarchy-aware JD matching — the system cannot determine that a Top Secret holder qualifies for Secret-level job descriptions.

This spec replaces free text with enum CHECK constraints on `level`, `polygraph`, and `status`; adds a `type` column (personnel vs facility); replaces `sponsoring_agency` text with an `sponsor_organization_id` FK to the organizations table; adds a `continuous_investigation` boolean; and introduces a `clearance_access_programs` junction table for SCI/SAP/NATO program tracking. A utility function encodes the clearance level hierarchy for JD matching.

## 1. Schema Changes

### 1.1 Table rebuild: `source_clearances`

SQLite cannot add CHECK constraints to existing columns via ALTER TABLE. A full table rebuild is required.

```sql
-- Clearance Structured Data
-- Migration: 017_clearance_structured_data
-- Replaces free-text clearance fields with enum constraints,
-- adds type/continuous_investigation columns, replaces sponsoring_agency
-- with sponsor_organization_id FK.

PRAGMA foreign_keys = OFF;

-- Step 1: Create new source_clearances with constraints
CREATE TABLE source_clearances_new (
  source_id TEXT PRIMARY KEY CHECK(typeof(source_id) = 'text' AND length(source_id) = 36)
    REFERENCES sources(id) ON DELETE CASCADE,
  level TEXT NOT NULL CHECK (level IN (
    'public', 'confidential', 'secret', 'top_secret', 'q', 'l'
  )),
  polygraph TEXT CHECK (polygraph IS NULL OR polygraph IN (
    'none', 'ci', 'full_scope'
  )),
  status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'inactive')),
  type TEXT NOT NULL DEFAULT 'personnel' CHECK (type IN ('personnel', 'facility')),
  sponsor_organization_id TEXT REFERENCES organizations(id) ON DELETE SET NULL,
  continuous_investigation INTEGER NOT NULL DEFAULT 0,
  investigation_date TEXT,
  adjudication_date TEXT,
  reinvestigation_date TEXT,
  read_on TEXT
) STRICT;

CREATE INDEX idx_source_clearances_sponsor ON source_clearances_new(sponsor_organization_id);

-- Step 2: Create organizations from existing sponsoring_agency text values
INSERT OR IGNORE INTO organizations (id, name, org_type, created_at, updated_at)
  SELECT
    lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)),2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)),2) || '-' || hex(randomblob(6))),
    sponsoring_agency,
    'government',
    strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
    strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
  FROM source_clearances
  WHERE sponsoring_agency IS NOT NULL AND sponsoring_agency != ''
  AND lower(sponsoring_agency) NOT IN (SELECT lower(name) FROM organizations);

-- Step 3: Migrate data with enum mapping
INSERT INTO source_clearances_new (
  source_id, level, polygraph, status, type,
  sponsor_organization_id, continuous_investigation,
  investigation_date, adjudication_date, reinvestigation_date, read_on
)
SELECT
  source_id,
  -- Map level text to enum
  CASE lower(trim(level))
    WHEN 'public' THEN 'public'
    WHEN 'confidential' THEN 'confidential'
    WHEN 'secret' THEN 'secret'
    WHEN 's' THEN 'secret'
    WHEN 'top secret' THEN 'top_secret'
    WHEN 'top_secret' THEN 'top_secret'
    WHEN 'ts' THEN 'top_secret'
    WHEN 'ts/sci' THEN 'top_secret'
    WHEN 'q' THEN 'q'
    WHEN 'l' THEN 'l'
    ELSE 'secret'  -- safe fallback; review after migration
  END,
  -- Map polygraph text to enum
  CASE lower(trim(COALESCE(polygraph, '')))
    WHEN '' THEN NULL
    WHEN 'none' THEN 'none'
    WHEN 'ci' THEN 'ci'
    WHEN 'ci polygraph' THEN 'ci'
    WHEN 'counterintelligence' THEN 'ci'
    WHEN 'full scope' THEN 'full_scope'
    WHEN 'full_scope' THEN 'full_scope'
    WHEN 'fs' THEN 'full_scope'
    WHEN 'lifestyle' THEN 'full_scope'
    ELSE NULL
  END,
  -- Map status text to enum
  CASE lower(trim(COALESCE(status, 'active')))
    WHEN 'active' THEN 'active'
    WHEN 'current' THEN 'active'
    WHEN 'inactive' THEN 'inactive'
    WHEN 'expired' THEN 'inactive'
    WHEN 'lapsed' THEN 'inactive'
    ELSE 'active'
  END,
  'personnel',  -- default; no existing data distinguishes type
  -- Link to org by name
  (SELECT o.id FROM organizations o WHERE o.name = source_clearances.sponsoring_agency LIMIT 1),
  0,  -- continuous_investigation default
  investigation_date,
  adjudication_date,
  reinvestigation_date,
  read_on
FROM source_clearances;

-- Step 4: Drop old table and rename
DROP TABLE source_clearances;
ALTER TABLE source_clearances_new RENAME TO source_clearances;

PRAGMA foreign_keys = ON;

-- Step 5: Create clearance_access_programs junction table
CREATE TABLE clearance_access_programs (
  source_id TEXT NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
  program TEXT NOT NULL CHECK (program IN ('sci', 'sap', 'nato')),
  PRIMARY KEY (source_id, program)
) STRICT;

-- Step 6: SCI seeding note
-- The original table is dropped in Step 4, so we cannot detect original 'ts/sci' level
-- values at this point. Access programs default to empty after migration. Users must
-- manually add SCI/SAP/NATO via the UI.

-- Step 7: Add government/military tags to newly created sponsor orgs
INSERT OR IGNORE INTO org_tags (organization_id, tag)
  SELECT o.id, 'government'
  FROM organizations o
  WHERE o.org_type = 'government'
  AND o.id NOT IN (SELECT organization_id FROM org_tags WHERE tag = 'government');

INSERT INTO _migrations (name) VALUES ('017_clearance_structured_data');
```

### 1.2 New table: `clearance_access_programs`

| Column | Type | Constraint | Purpose |
|--------|------|-----------|---------|
| `source_id` | TEXT | FK -> sources(id) ON DELETE CASCADE | The clearance source |
| `program` | TEXT | CHECK enum: `sci`, `sap`, `nato` | Access program type |

Composite PK on `(source_id, program)`.

### 1.3 Rebuilt columns: `source_clearances`

| Column | Type | Constraint | Previous |
|--------|------|-----------|----------|
| `source_id` | TEXT | PK, FK -> sources(id) | Unchanged |
| `level` | TEXT | NOT NULL, CHECK enum | Was free text |
| `polygraph` | TEXT | CHECK enum, nullable | Was free text |
| `status` | TEXT | NOT NULL DEFAULT 'active', CHECK enum | Was free text, nullable |
| `type` | TEXT | NOT NULL DEFAULT 'personnel', CHECK enum | **New column** |
| `sponsor_organization_id` | TEXT | FK -> organizations(id), nullable | Was `sponsoring_agency TEXT` |
| `continuous_investigation` | INTEGER | NOT NULL DEFAULT 0 | **New column** |
| `investigation_date` | TEXT | nullable | Unchanged |
| `adjudication_date` | TEXT | nullable | Unchanged |
| `reinvestigation_date` | TEXT | nullable | Unchanged |
| `read_on` | TEXT | nullable | Unchanged |

### 1.4 Enum values

**ClearanceLevel:** `public | confidential | secret | top_secret | q | l`

- `public` = no clearance / public trust only
- `confidential` = Confidential
- `secret` = Secret
- `top_secret` = Top Secret (TS)
- `q` = DOE Q clearance (equivalent to TS)
- `l` = DOE L clearance (equivalent to Secret-ish, but distinct lineage)

**ClearancePolygraph:** `none | ci | full_scope`

- `none` = no polygraph
- `ci` = Counterintelligence polygraph
- `full_scope` = Full Scope / Lifestyle polygraph

**ClearanceStatus:** `active | inactive`

**ClearanceType:** `personnel | facility`

**ClearanceAccessProgram:** `sci | sap | nato`

- `sci` = Sensitive Compartmented Information
- `sap` = Special Access Program
- `nato` = NATO Secret / NATO access

## 2. Clearance Level Hierarchy

The hierarchy defines which clearance levels satisfy a job description's clearance requirement. A holder at level X qualifies for any JD requiring level Y where Y <= X in the hierarchy.

```
l < confidential < secret < top_secret = q
public = no clearance (does not satisfy any requirement)
```

DOE `L` is roughly equivalent to Secret but from the DOE lineage. DOE `Q` is roughly equivalent to Top Secret. `Q` and `top_secret` are equal rank (reciprocal): `clearanceLevelRank('q') === clearanceLevelRank('top_secret')` — both return rank 4. `clearanceMeetsRequirement('top_secret', 'q')` returns true (reciprocity).

### 2.1 Utility function

Create a utility in `packages/core/src/constants/clearance.ts`:

```typescript
/**
 * Ordered clearance levels from lowest to highest access.
 * 'public' is excluded — it represents no clearance.
 * 'q' and 'top_secret' are equal rank (reciprocal).
 */
export const CLEARANCE_LEVEL_HIERARCHY: readonly ClearanceLevel[] = [
  'l',
  'confidential',
  'secret',
  'top_secret',
  'q',
] as const

/**
 * Returns the numeric rank of a clearance level (higher = more access).
 * 'public' returns 0. Unknown levels return -1.
 * 'q' and 'top_secret' both return rank 4 (reciprocal equivalence).
 */
export function clearanceLevelRank(level: ClearanceLevel): number {
  if (level === 'public') return 0
  if (level === 'q') return 4  // same rank as top_secret (reciprocal)
  const idx = CLEARANCE_LEVEL_HIERARCHY.indexOf(level)
  return idx === -1 ? -1 : idx + 1
}

/**
 * Returns true if `holderLevel` satisfies a JD requirement of `requiredLevel`.
 * A holder qualifies if their rank >= the required rank.
 * 'public' holders satisfy nothing. 'public' requirements are satisfied by anyone.
 */
export function clearanceMeetsRequirement(
  holderLevel: ClearanceLevel,
  requiredLevel: ClearanceLevel,
): boolean {
  if (requiredLevel === 'public') return true
  if (holderLevel === 'public') return false
  return clearanceLevelRank(holderLevel) >= clearanceLevelRank(requiredLevel)
}

/** All valid clearance levels (ordered by hierarchy: lowest to highest). */
export const CLEARANCE_LEVELS: readonly ClearanceLevel[] = [
  'public', 'l', 'confidential', 'secret', 'top_secret', 'q',
] as const

/** All valid polygraph types. */
export const CLEARANCE_POLYGRAPHS: readonly ClearancePolygraph[] = [
  'none', 'ci', 'full_scope',
] as const

/** All valid clearance statuses. */
export const CLEARANCE_STATUSES: readonly ClearanceStatus[] = [
  'active', 'inactive',
] as const

/** All valid clearance types. */
export const CLEARANCE_TYPES: readonly ClearanceType[] = [
  'personnel', 'facility',
] as const

/** All valid access programs. */
export const CLEARANCE_ACCESS_PROGRAMS: readonly ClearanceAccessProgram[] = [
  'sci', 'sap', 'nato',
] as const

/** Human-readable labels for clearance levels. */
export const CLEARANCE_LEVEL_LABELS: Record<ClearanceLevel, string> = {
  public: 'Public Trust',
  confidential: 'Confidential',
  secret: 'Secret',
  top_secret: 'Top Secret (TS)',
  q: 'DOE Q',
  l: 'DOE L',
}

/** Human-readable labels for polygraph types. */
export const CLEARANCE_POLYGRAPH_LABELS: Record<ClearancePolygraph, string> = {
  none: 'None',
  ci: 'CI Polygraph',
  full_scope: 'Full Scope (Lifestyle)',
}

/** Human-readable labels for access programs. */
export const CLEARANCE_ACCESS_PROGRAM_LABELS: Record<ClearanceAccessProgram, string> = {
  sci: 'SCI',
  sap: 'SAP',
  nato: 'NATO',
}
```

## 3. Type Definitions

### 3.1 Core types (`packages/core/src/types/index.ts`)

New union types:

```typescript
export type ClearanceLevel = 'public' | 'confidential' | 'secret' | 'top_secret' | 'q' | 'l'
export type ClearancePolygraph = 'none' | 'ci' | 'full_scope'
export type ClearanceStatus = 'active' | 'inactive'
export type ClearanceType = 'personnel' | 'facility'
export type ClearanceAccessProgram = 'sci' | 'sap' | 'nato'
```

Updated `SourceClearance` interface:

```typescript
/** Clearance-specific details for a source with source_type='clearance'. */
export interface SourceClearance {
  source_id: string
  level: ClearanceLevel
  polygraph: ClearancePolygraph | null
  status: ClearanceStatus
  type: ClearanceType
  sponsor_organization_id: string | null
  continuous_investigation: number
  access_programs: ClearanceAccessProgram[]
  investigation_date: string | null
  adjudication_date: string | null
  reinvestigation_date: string | null
  read_on: string | null
  /** @deprecated Use sponsor_organization_id. Kept for legacy reads during transition. */
  sponsoring_agency?: string | null
}
```

Key changes:
- `level` narrowed from `string` to `ClearanceLevel`
- `polygraph` narrowed from `string | null` to `ClearancePolygraph | null`
- `status` narrowed from `string | null` to `ClearanceStatus` (non-nullable, defaults to 'active')
- `type` added as `ClearanceType`
- `sponsoring_agency` replaced by `sponsor_organization_id`
- `continuous_investigation` added as integer boolean
- `access_programs` added as `ClearanceAccessProgram[]` (hydrated from junction table)

Updated `CreateSource` clearance fields:

```typescript
export interface CreateSource {
  // ... existing fields ...
  // Clearance extension fields
  level?: ClearanceLevel
  polygraph?: ClearancePolygraph
  clearance_status?: ClearanceStatus
  clearance_type?: ClearanceType
  sponsor_organization_id?: string
  continuous_investigation?: number
  access_programs?: ClearanceAccessProgram[]
}
```

Updated `UpdateSource` clearance fields:

```typescript
export interface UpdateSource {
  // ... existing fields ...
  // Clearance extension fields
  level?: ClearanceLevel
  polygraph?: ClearancePolygraph | null
  clearance_status?: ClearanceStatus
  clearance_type?: ClearanceType
  sponsor_organization_id?: string | null
  continuous_investigation?: number
  access_programs?: ClearanceAccessProgram[]
}
```

Note: `clearance_status` and `clearance_type` are prefixed to avoid collision with the base `Source.status` and `Source.source_type` fields on the flattened `CreateSource`/`UpdateSource` interfaces.

### 3.2 SDK types (`packages/sdk/src/types.ts`)

Mirror the updated `SourceClearance` (without `source_id`, per SDK convention):

```typescript
export interface SourceClearance {
  level: ClearanceLevel
  polygraph: ClearancePolygraph | null
  status: ClearanceStatus
  type: ClearanceType
  sponsor_organization_id: string | null
  continuous_investigation: number
  access_programs: ClearanceAccessProgram[]
  investigation_date: string | null
  adjudication_date: string | null
  reinvestigation_date: string | null
  read_on: string | null
}
```

Add the 5 new union types to `packages/sdk/src/types.ts` and export them from `packages/sdk/src/index.ts`.

## 4. Repository Changes

### 4.1 `source-repository.ts` — `getExtension()`

Update the clearance branch to hydrate `access_programs` from the junction table:

```typescript
case 'clearance': {
  const clearance = db.query('SELECT * FROM source_clearances WHERE source_id = ?')
    .get(sourceId) as SourceClearance | null
  if (clearance) {
    const programs = db.query('SELECT program FROM clearance_access_programs WHERE source_id = ?')
      .all(sourceId) as Array<{ program: ClearanceAccessProgram }>
    clearance.access_programs = programs.map(p => p.program)
  }
  return clearance
}
```

### 4.2 `source-repository.ts` — `create()`

Update the clearance INSERT to use new columns:

```typescript
} else if (sourceType === 'clearance') {
  db.run(
    `INSERT INTO source_clearances (
      source_id, level, polygraph, status, type,
      sponsor_organization_id, continuous_investigation,
      investigation_date, adjudication_date, reinvestigation_date, read_on
    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
    [
      id,
      input.level ?? 'secret',
      input.polygraph ?? null,
      input.clearance_status ?? 'active',
      input.clearance_type ?? 'personnel',
      input.sponsor_organization_id ?? null,
      input.continuous_investigation ?? 0,
      null, // investigation_date
      null, // adjudication_date
      null, // reinvestigation_date
      null, // read_on
    ],
  )

  // Insert access programs
  if (input.access_programs?.length) {
    const insertProgram = db.prepare(
      'INSERT INTO clearance_access_programs (source_id, program) VALUES (?, ?)',
    )
    for (const program of input.access_programs) {
      insertProgram.run(id, program)
    }
  }
}
```

### 4.3 `source-repository.ts` — `updateExtension()`

Update the clearance branch:

```typescript
} else if (sourceType === 'clearance') {
  const sets: string[] = []
  const params: unknown[] = []

  if ('level' in input) { sets.push('level = ?'); params.push(input.level) }
  if ('polygraph' in input) { sets.push('polygraph = ?'); params.push(input.polygraph ?? null) }
  if ('clearance_status' in input) { sets.push('status = ?'); params.push(input.clearance_status) }
  if ('clearance_type' in input) { sets.push('type = ?'); params.push(input.clearance_type) }
  if ('sponsor_organization_id' in input) { sets.push('sponsor_organization_id = ?'); params.push(input.sponsor_organization_id ?? null) }
  if ('continuous_investigation' in input) { sets.push('continuous_investigation = ?'); params.push(input.continuous_investigation ?? 0) }

  if (sets.length > 0) {
    params.push(sourceId)
    db.run(`UPDATE source_clearances SET ${sets.join(', ')} WHERE source_id = ?`, params)
  }

  // Replace access programs if provided
  if ('access_programs' in input) {
    db.run('DELETE FROM clearance_access_programs WHERE source_id = ?', [sourceId])
    if (input.access_programs?.length) {
      const insertProgram = db.prepare(
        'INSERT INTO clearance_access_programs (source_id, program) VALUES (?, ?)',
      )
      for (const program of input.access_programs) {
        insertProgram.run(sourceId, program)
      }
    }
  }
}
```

## 5. UI Changes

### 5.1 Clearance form in `SourcesView.svelte`

Replace free-text inputs with dropdowns and structured controls.

**New form state variables:**

```typescript
// Clearance extension fields (replace existing free-text vars)
let formLevel = $state<ClearanceLevel>('secret')
let formPolygraph = $state<ClearancePolygraph | ''>('')
let formClearanceStatus = $state<ClearanceStatus>('active')
let formClearanceType = $state<ClearanceType>('personnel')
let formSponsorOrgId = $state('')
let formContinuousInvestigation = $state(false)
let formAccessPrograms = $state<ClearanceAccessProgram[]>([])
```

Remove `formSponsoringAgency`.

**Form layout:**

```
[Source Type: Clearance v]

[Level: Secret v          ] [Status: Active v      ]
[Polygraph: CI Polygraph v] [Type: Personnel v     ]
[Sponsor Organization: [org dropdown] v            ]
[ ] Continuous Investigation (CE/CV)

Access Programs:
[x] SCI  [ ] SAP  [ ] NATO

[Investigation Date:   ] [Adjudication Date:   ]
[Reinvestigation Date: ] [Read-On Date:        ]
```

**Level dropdown options** (use `CLEARANCE_LEVEL_LABELS`):
- Public Trust
- Confidential
- Secret
- Top Secret (TS)
- DOE Q
- DOE L

**Polygraph dropdown options** (use `CLEARANCE_POLYGRAPH_LABELS`):
- _(empty / none selected)_
- None
- CI Polygraph
- Full Scope (Lifestyle)

**Status dropdown options:**
- Active
- Inactive

**Type dropdown options:**
- Personnel
- Facility

**Sponsor organization dropdown:**
- Filter organizations by tags: `government`, `military`
- Same pattern as the education org dropdown with "+" quick-create button
- When creating a new sponsor org inline, default `org_type` to `'government'` and add `government` tag

**Access programs checkbox group:**
- Three checkboxes: SCI, SAP, NATO
- Maps to `formAccessPrograms` array

### 5.2 Reset logic

In `startNew()`:
```typescript
formLevel = 'secret'
formPolygraph = ''
formClearanceStatus = 'active'
formClearanceType = 'personnel'
formSponsorOrgId = ''
formContinuousInvestigation = false
formAccessPrograms = []
```

In `startEditing()` / `populateFormFromSource()`:
```typescript
if (source.source_type === 'clearance' && source.clearance) {
  formLevel = source.clearance.level
  formPolygraph = source.clearance.polygraph ?? ''
  formClearanceStatus = source.clearance.status
  formClearanceType = source.clearance.type
  formSponsorOrgId = source.clearance.sponsor_organization_id ?? ''
  formContinuousInvestigation = !!source.clearance.continuous_investigation
  formAccessPrograms = source.clearance.access_programs ?? []
}
```

### 5.3 Save payload construction

```typescript
} else if (formSourceType === 'clearance') {
  basePayload.clearance = {
    level: formLevel,
    polygraph: formPolygraph || undefined,
    status: formClearanceStatus,
    type: formClearanceType,
    sponsor_organization_id: formSponsorOrgId || undefined,
    continuous_investigation: formContinuousInvestigation ? 1 : 0,
    access_programs: formAccessPrograms,
  }
}
```

## 6. API Route Changes

### 6.1 `POST /api/sources` and `PATCH /api/sources/:id`

The API route handler already spreads `input.clearance` into the repository's flat field structure. The new fields (`clearance_type`, `sponsor_organization_id`, `continuous_investigation`, `access_programs`) need to be mapped from the SDK's nested `clearance` object to the flat `CreateSource`/`UpdateSource` keys:

```typescript
if (input.clearance) {
  flat.level = input.clearance.level
  flat.polygraph = input.clearance.polygraph
  flat.clearance_status = input.clearance.status
  flat.clearance_type = input.clearance.type
  flat.sponsor_organization_id = input.clearance.sponsor_organization_id
  flat.continuous_investigation = input.clearance.continuous_investigation
  flat.access_programs = input.clearance.access_programs
}
```

## 7. Files to Create

| File | Purpose |
|------|---------|
| `packages/core/src/db/migrations/017_clearance_structured_data.sql` | Table rebuild + junction table + data migration |
| `packages/core/src/constants/clearance.ts` | Hierarchy utility, enum arrays, label maps |

## 8. Files to Modify

| File | Change |
|------|--------|
| `packages/core/src/types/index.ts` | Add 5 union types; update `SourceClearance`, `CreateSource`, `UpdateSource` |
| `packages/sdk/src/types.ts` | Mirror type changes: `SourceClearance`, union types |
| `packages/sdk/src/index.ts` | Export new union types |
| `packages/core/src/db/repositories/source-repository.ts` | Update `getExtension()`, `create()`, `updateExtension()` for new schema + access programs |
| `packages/core/src/index.ts` | Re-export from `constants/clearance.ts` |
| `packages/webui/src/routes/data/sources/SourcesView.svelte` | Replace clearance text inputs with dropdowns, checkboxes, org selector |
| `packages/webui/src/routes/experience/clearances/+page.svelte` | Update to display structured clearance data with labels |

## 9. Testing

### 9.1 Unit tests — Repository

- **Create clearance source with all fields:** `level='top_secret', polygraph='ci', clearance_status='active', clearance_type='personnel', access_programs=['sci']` — verify all fields persisted and round-tripped
- **Create clearance with access programs:** verify `clearance_access_programs` rows created; verify hydration in `get()`
- **Update clearance level:** change `secret` -> `top_secret`, verify round-trip
- **Update access programs:** replace `['sci']` with `['sci', 'sap']`, verify old rows deleted and new rows inserted
- **Update access programs to empty:** set `access_programs=[]`, verify all junction rows deleted
- **Create clearance with sponsor org:** create an org, create clearance with `sponsor_organization_id`, verify FK round-trip
- **CHECK constraint enforcement:** attempt to insert `level='invalid'` — expect SQLite constraint error

### 9.2 Unit tests — Hierarchy utility

- `clearanceMeetsRequirement('top_secret', 'secret')` returns `true`
- `clearanceMeetsRequirement('secret', 'top_secret')` returns `false`
- `clearanceMeetsRequirement('q', 'top_secret')` returns `true` (reciprocal)
- `clearanceMeetsRequirement('top_secret', 'q')` returns `true` (reciprocal)
- `clearanceMeetsRequirement('public', 'confidential')` returns `false`
- `clearanceMeetsRequirement('secret', 'public')` returns `true`
- `clearanceLevelRank('public')` returns `0`
- `clearanceLevelRank('top_secret')` returns `4`

### 9.3 Integration tests — API

- POST source with `source_type='clearance'`, full clearance payload — verify structured response
- PATCH source clearance to change level, add access programs — verify response
- GET source with clearance extension — verify all new fields present including `access_programs` array

### 9.4 Contract tests — SDK

- SDK `SourceClearance` type includes all new fields
- `access_programs` is returned as an array (not omitted when empty)
- `sponsor_organization_id` is nullable
- Verify `GET /api/sources/:id` for a clearance source returns `access_programs: []` (empty array, not undefined) when no programs are linked

### 9.5 Migration tests

- Run migration on DB with existing clearance data — verify:
  - Free text "TS/SCI" maps to `level='top_secret'`
  - Free text "CI" maps to `polygraph='ci'`
  - Free text "active" maps to `status='active'`
  - `sponsoring_agency` text creates org and links via FK
  - Date fields preserved unchanged
  - `type` defaults to `'personnel'`
  - `continuous_investigation` defaults to `0`

## 10. Non-Goals

- Clearance verification tracking (e.g., FSO confirmation records)
- Reinvestigation reminders or scheduling
- Clearance expiry alerts or notifications
- Reciprocity mapping between agency clearances (e.g., DoD TS <-> DOE Q automatic recognition)
- SCI compartment-level tracking (e.g., specific SCI codewords)
- Clearance-based JD filtering in the UI (future feature; hierarchy utility is the foundation)
- Removing `sponsoring_agency` column from old data (kept as deprecated for safety; future migration can drop it)

## 11. Acceptance Criteria

1. Migration 017 rebuilds `source_clearances` with CHECK constraints and creates `clearance_access_programs` without data loss
2. Existing free-text clearance values are correctly mapped to enum values during migration
3. Existing `sponsoring_agency` text values create organizations and link via FK
4. Creating a clearance source with `level='top_secret', polygraph='ci', access_programs=['sci', 'sap']` persists and round-trips correctly
5. Updating a clearance's access programs replaces the junction rows atomically
6. `clearanceMeetsRequirement('top_secret', 'secret')` returns `true` (hierarchy works)
7. `clearanceMeetsRequirement('secret', 'top_secret')` returns `false` (hierarchy is directional)
8. UI shows dropdowns for level/polygraph/status/type instead of text inputs
9. UI shows checkbox group for access programs (SCI, SAP, NATO)
10. UI sponsor org dropdown filters to government/military tagged organizations
11. All existing clearance sources continue to work after migration
12. The `type` and `continuous_investigation` columns default correctly for migrated data
