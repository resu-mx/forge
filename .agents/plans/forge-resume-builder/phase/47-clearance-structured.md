# Phase 47: Security Clearance Structured Data

**Status:** Planning
**Date:** 2026-04-03
**Spec:** [2026-04-03-clearance-structured-data.md](../refs/specs/2026-04-03-clearance-structured-data.md)
**Depends on:** Migration 002 (`source_clearances` table)
**Blocks:** None currently identified
**Parallelizable with:** Phase 44 (IR Data Quality), Phase 45 (Editor Restructuring), Phase 46 (LaTeX/XeTeX Docs) -- independent migration number (018), different files

## Goal

Replace free-text clearance fields in `source_clearances` with enum CHECK constraints on `level`, `polygraph`, and `status`; add `type` (personnel/facility) and `continuous_investigation` columns; replace `sponsoring_agency` text with a `sponsor_organization_id` FK to `organizations`; create a `clearance_access_programs` junction table for SCI/SAP/NATO program tracking; and implement a `clearanceLevelRank` utility function encoding the clearance hierarchy for JD matching. This eliminates data quality problems ("TS/SCI" vs "Top Secret/SCI" vs "top_secret") and enables hierarchy-aware JD matching.

## Non-Goals

- Clearance verification tracking (e.g., FSO confirmation records)
- Reinvestigation reminders or scheduling
- Clearance expiry alerts or notifications
- Reciprocity mapping between agency clearances (e.g., DoD TS <-> DOE Q automatic recognition)
- SCI compartment-level tracking (e.g., specific SCI codewords)
- Clearance-based JD filtering in the UI (future feature; hierarchy utility is the foundation)
- Removing `sponsoring_agency` column from old data (kept as deprecated for safety)

## Context

The `source_clearances` table currently stores all fields as free text:

```sql
-- Current schema (from migration 002)
CREATE TABLE source_clearances (
  source_id TEXT PRIMARY KEY REFERENCES sources(id) ON DELETE CASCADE,
  level TEXT NOT NULL,
  polygraph TEXT,
  status TEXT,
  sponsoring_agency TEXT,
  investigation_date TEXT,
  adjudication_date TEXT,
  reinvestigation_date TEXT,
  read_on TEXT
) STRICT;
```

This creates data quality problems:
- **Level:** "TS/SCI" vs "Top Secret/SCI" vs "top_secret" vs "ts" -- no canonical form
- **Polygraph:** "CI" vs "ci" vs "CI Polygraph" vs "counterintelligence" -- inconsistent casing and naming
- **Status:** "active" vs "Active" vs "current" -- no constraint
- **Sponsoring agency:** Free text with no FK to organizations

The fix: table rebuild with CHECK constraints, new columns, junction table for access programs, and enum constants with hierarchy utility.

## Scope

| Spec section | Covered here? |
|-------------|---------------|
| 1. Schema changes (migration 018) | Yes |
| 1.2 clearance_access_programs table | Yes |
| 1.3 Rebuilt source_clearances columns | Yes |
| 1.4 Enum values | Yes |
| 2. Clearance level hierarchy | Yes |
| 3. Type definitions | Yes |
| 4. Repository changes | Yes |
| 5. UI changes | Yes |
| 6. API route changes | Yes |
| 7-8. Files to create/modify | Yes |
| 9. Testing | Yes |

## Files to Create

| File | Description |
|------|-------------|
| `packages/core/src/db/migrations/018_clearance_structured_data.sql` | Table rebuild + junction table + data migration |
| `packages/core/src/constants/clearance.ts` | Hierarchy utility, enum arrays, label maps |

## Files to Modify

| File | Change |
|------|--------|
| `packages/core/src/types/index.ts` | Add 5 union types (`ClearanceLevel`, `ClearancePolygraph`, `ClearanceStatus`, `ClearanceType`, `ClearanceAccessProgram`); update `SourceClearance`, `CreateSource`, `UpdateSource` |
| `packages/sdk/src/types.ts` | Mirror type changes: `SourceClearance`, union types |
| `packages/sdk/src/index.ts` | Export new union types |
| `packages/core/src/db/repositories/source-repository.ts` | Update `getExtension()`, `create()`, `updateExtension()` for new schema + access programs hydration |
| `packages/core/src/index.ts` | Re-export from `constants/clearance.ts` |
| `packages/webui/src/routes/data/sources/SourcesView.svelte` | Replace clearance text inputs with dropdowns, checkboxes, org selector |
| `packages/webui/src/routes/experience/clearances/+page.svelte` | Update to display structured clearance data with labels |

## Fallback Strategies

- **PRAGMA foreign_keys in transactions:** The migration runner wraps each migration in `BEGIN`/`COMMIT`. SQLite silently ignores `PRAGMA foreign_keys = OFF` inside an active transaction. The PRAGMA calls are defensive only -- the actual atomicity comes from the runner's transaction. This is consistent with migrations 002, 007, and 012.
- **Unknown level values:** The CASE WHEN mapping in the migration uses `ELSE 'secret'` as a safe fallback for unrecognized level text. After migration, a manual review query (`SELECT * FROM source_clearances WHERE level = 'secret'`) can identify entries that may have been incorrectly mapped.
- **Orphaned sponsoring_agency values:** If an existing `sponsoring_agency` text does not match any organization name, the migration creates a new organization with `org_type = 'government'` and links it. The `INSERT OR IGNORE` prevents duplicates.
- **Access program seeding:** The migration cannot detect original "ts/sci" values at the point where it needs to insert into `clearance_access_programs` because the old table is already dropped. Access programs default to empty after migration. Users must manually add SCI/SAP/NATO via the UI. This is documented in the migration SQL comments.
- **UI org dropdown empty:** If no organizations have `government` or `military` tags, the sponsor org dropdown shows an empty list with a "+" button to create a new org inline.

---

## Tasks

### T47.1: Write Migration `018_clearance_structured_data.sql` [CRITICAL]

**File:** `packages/core/src/db/migrations/018_clearance_structured_data.sql`

Rebuilds `source_clearances` with CHECK constraints, creates `clearance_access_programs` junction table, migrates existing free-text data with enum mapping, and creates organizations from `sponsoring_agency` text values.

```sql
-- Clearance Structured Data
-- Migration: 018_clearance_structured_data
-- Date: 2026-04-03
-- Replaces free-text clearance fields with enum constraints,
-- adds type/continuous_investigation columns, replaces sponsoring_agency
-- with sponsor_organization_id FK.
-- Uses table rebuild pattern (SQLite cannot ALTER CHECK constraints).
-- Builds on migration 002 (source_clearances table).

PRAGMA foreign_keys = OFF;

-- Note: The migration runner wraps each migration in BEGIN/COMMIT.
-- PRAGMA foreign_keys = OFF is silently ignored inside an active transaction.
-- The PRAGMA calls are defensive only -- the actual FK protection comes from
-- the runner's transaction ensuring all statements execute atomically.
-- This is consistent with how migrations 002, 007, and 012 handle table rebuilds.

-- Step 1: Create organizations from existing sponsoring_agency text values.
-- Must happen BEFORE the old table is dropped.
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

-- Step 2: Tag newly created sponsor orgs as 'government'
INSERT OR IGNORE INTO org_tags (organization_id, tag)
  SELECT o.id, 'government'
  FROM organizations o
  WHERE o.org_type = 'government'
  AND o.id NOT IN (SELECT organization_id FROM org_tags WHERE tag = 'government');

-- Step 3: Create new source_clearances with constraints
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

-- Step 4: Migrate data with enum mapping
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
    WHEN 'public trust' THEN 'public'
    WHEN 'confidential' THEN 'confidential'
    WHEN 'secret' THEN 'secret'
    WHEN 's' THEN 'secret'
    WHEN 'top secret' THEN 'top_secret'
    WHEN 'top_secret' THEN 'top_secret'
    WHEN 'ts' THEN 'top_secret'
    WHEN 'ts/sci' THEN 'top_secret'
    WHEN 'top secret/sci' THEN 'top_secret'
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
  -- Link to org by name match
  (SELECT o.id FROM organizations o WHERE lower(o.name) = lower(source_clearances.sponsoring_agency) LIMIT 1),
  0,  -- continuous_investigation default
  investigation_date,
  adjudication_date,
  reinvestigation_date,
  read_on
FROM source_clearances;

-- Step 5: Drop old table and rename
DROP TABLE source_clearances;
ALTER TABLE source_clearances_new RENAME TO source_clearances;

PRAGMA foreign_keys = ON;

-- Step 6: Create clearance_access_programs junction table
CREATE TABLE clearance_access_programs (
  source_id TEXT NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
  program TEXT NOT NULL CHECK (program IN ('sci', 'sap', 'nato')),
  PRIMARY KEY (source_id, program)
) STRICT;

-- Step 7: SCI seeding note
-- The original table is dropped in Step 5, so we cannot detect original 'ts/sci' level
-- values at this point. Access programs default to empty after migration. Users must
-- manually add SCI/SAP/NATO via the UI.

INSERT INTO _migrations (name) VALUES ('018_clearance_structured_data');
```

**Key points:**
- Org creation (Step 1) happens BEFORE the old table is dropped (Step 5) so we can read `sponsoring_agency` text.
- The CASE WHEN mapping handles known variations of clearance level text. Unknown values fall back to `'secret'`.
- The org name matching uses `lower()` for case-insensitive comparison.
- Status remapping done inline via CASE WHEN in the INSERT (same pattern as migration 012).
- Explicit column list (no `SELECT *`) prevents breakage if columns differ.

**Acceptance criteria:**
- After migration, `SELECT * FROM source_clearances WHERE level NOT IN ('public','confidential','secret','top_secret','q','l')` returns 0 rows.
- After migration, `SELECT * FROM source_clearances WHERE status NOT IN ('active','inactive')` returns 0 rows.
- Original "TS/SCI" maps to `level='top_secret'`.
- Original "CI" maps to `polygraph='ci'`.
- Original "active"/"current" maps to `status='active'`.
- `sponsoring_agency` text creates org and links via `sponsor_organization_id` FK.
- Date fields preserved unchanged.
- `type` defaults to `'personnel'`.
- `continuous_investigation` defaults to `0`.
- `clearance_access_programs` table exists with correct CHECK constraint.
- `INSERT INTO source_clearances (..., level) VALUES (..., 'invalid')` fails with CHECK constraint.
- The `_migrations` table contains `018_clearance_structured_data`.

**Failure criteria:**
- Migration fails with SQL error.
- Rows lost during table rebuild (count mismatch before/after).
- `sponsoring_agency` org not created or not linked.

---

### T47.2: Create `constants/clearance.ts` with Hierarchy Utility [CRITICAL]

**File:** `packages/core/src/constants/clearance.ts`

Hierarchy utility, enum arrays, and human-readable label maps.

```typescript
/**
 * Security clearance constants, hierarchy, and label maps.
 *
 * Enum arrays mirror the CHECK constraints in migration 018.
 * The hierarchy utility enables JD matching: a Top Secret holder
 * qualifies for any Secret-level requirement.
 */

import type {
  ClearanceLevel,
  ClearancePolygraph,
  ClearanceStatus,
  ClearanceType,
  ClearanceAccessProgram,
} from '../types'

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

/** All valid clearance levels (ordered: lowest to highest). */
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

**Acceptance criteria:**
- `clearanceLevelRank('public')` returns `0`.
- `clearanceLevelRank('l')` returns `1`.
- `clearanceLevelRank('confidential')` returns `2`.
- `clearanceLevelRank('secret')` returns `3`.
- `clearanceLevelRank('top_secret')` returns `4`.
- `clearanceLevelRank('q')` returns `4` (reciprocal with top_secret).
- `clearanceMeetsRequirement('top_secret', 'secret')` returns `true`.
- `clearanceMeetsRequirement('secret', 'top_secret')` returns `false`.
- `clearanceMeetsRequirement('q', 'top_secret')` returns `true` (reciprocal).
- `clearanceMeetsRequirement('top_secret', 'q')` returns `true` (reciprocal).
- `clearanceMeetsRequirement('public', 'confidential')` returns `false`.
- `clearanceMeetsRequirement('secret', 'public')` returns `true`.
- All label maps have entries for every enum value.

**Failure criteria:**
- `q` and `top_secret` have different ranks.
- `public` satisfies a non-public requirement.
- Missing label for any enum value.

---

### T47.3: Add Type Definitions [CRITICAL]

**File:** `packages/core/src/types/index.ts`

Add five new union types and update `SourceClearance`, `CreateSource`, and `UpdateSource`.

**Add after line 49 (after `SourceType`):**
```typescript
/** Valid clearance level values. */
export type ClearanceLevel = 'public' | 'confidential' | 'secret' | 'top_secret' | 'q' | 'l'

/** Valid clearance polygraph values. */
export type ClearancePolygraph = 'none' | 'ci' | 'full_scope'

/** Valid clearance status values. */
export type ClearanceStatus = 'active' | 'inactive'

/** Valid clearance type values. */
export type ClearanceType = 'personnel' | 'facility'

/** Valid clearance access program values. */
export type ClearanceAccessProgram = 'sci' | 'sap' | 'nato'
```

**Replace `SourceClearance` interface (current lines 233-244):**
```typescript
/** Clearance-specific details for a source with source_type='clearance'. */
export interface SourceClearance {
  source_id: string
  level: string
  polygraph: string | null
  status: string | null
  sponsoring_agency: string | null
  investigation_date: string | null
  adjudication_date: string | null
  reinvestigation_date: string | null
  read_on: string | null
}
```

**With:**
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

**Replace clearance extension fields in `CreateSource` (current lines 544-548):**
```typescript
  // Clearance extension fields
  level?: string
  polygraph?: string
  clearance_status?: string
  sponsoring_agency?: string
```

**With:**
```typescript
  // Clearance extension fields
  level?: ClearanceLevel
  polygraph?: ClearancePolygraph
  clearance_status?: ClearanceStatus
  clearance_type?: ClearanceType
  sponsor_organization_id?: string
  continuous_investigation?: number
  access_programs?: ClearanceAccessProgram[]
  /** @deprecated Use sponsor_organization_id. */
  sponsoring_agency?: string
```

**Replace clearance extension fields in `UpdateSource` (current lines 583-587):**
```typescript
  // Clearance extension fields
  level?: string
  polygraph?: string | null
  clearance_status?: string | null
  sponsoring_agency?: string | null
```

**With:**
```typescript
  // Clearance extension fields
  level?: ClearanceLevel
  polygraph?: ClearancePolygraph | null
  clearance_status?: ClearanceStatus
  clearance_type?: ClearanceType
  sponsor_organization_id?: string | null
  continuous_investigation?: number
  access_programs?: ClearanceAccessProgram[]
  /** @deprecated Use sponsor_organization_id. */
  sponsoring_agency?: string | null
```

**Acceptance criteria:**
- TypeScript compiler accepts `{ level: 'top_secret' }` in `CreateSource`.
- TypeScript compiler rejects `{ level: 'invalid' }` in `CreateSource`.
- `SourceClearance.access_programs` is typed as `ClearanceAccessProgram[]`.
- `SourceClearance.status` is `ClearanceStatus` (non-nullable with default).
- `SourceClearance.type` is `ClearanceType`.
- `sponsoring_agency` is marked `@deprecated`.

**Failure criteria:**
- Old `string` types remain on clearance fields.
- Missing `access_programs` on `SourceClearance`.

---

### T47.4: Update SDK Types [IMPORTANT]

**File:** `packages/sdk/src/types.ts`

Mirror the type changes: add 5 union types, update `SourceClearance` (SDK version omits `source_id`).

Add the 5 union types at the appropriate location in the SDK types file:
```typescript
export type ClearanceLevel = 'public' | 'confidential' | 'secret' | 'top_secret' | 'q' | 'l'
export type ClearancePolygraph = 'none' | 'ci' | 'full_scope'
export type ClearanceStatus = 'active' | 'inactive'
export type ClearanceType = 'personnel' | 'facility'
export type ClearanceAccessProgram = 'sci' | 'sap' | 'nato'
```

Update the SDK `SourceClearance` interface (without `source_id`, per SDK convention):
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

**File:** `packages/sdk/src/index.ts`

Export the new union types.

**Acceptance criteria:**
- SDK types match core types structurally.
- All 5 union types are exported from the SDK.
- SDK `SourceClearance` omits `source_id` (SDK convention).

**Failure criteria:**
- Type mismatch between SDK and core types.
- Missing exports.

---

### T47.5: Update Source Repository [CRITICAL]

**File:** `packages/core/src/db/repositories/source-repository.ts`

Update `getExtension()` to hydrate `access_programs` from the junction table. Update `create()` to use new columns and insert access programs. Update `updateExtension()` to handle new fields and replace access programs.

**Replace the clearance branch in `getExtension()` (current line 53-54):**
```typescript
    case 'clearance':
      return db.query('SELECT * FROM source_clearances WHERE source_id = ?').get(sourceId) as SourceClearance | null
```

**With:**
```typescript
    case 'clearance': {
      const clearance = db.query('SELECT * FROM source_clearances WHERE source_id = ?')
        .get(sourceId) as (Omit<SourceClearance, 'access_programs'> & { access_programs?: ClearanceAccessProgram[] }) | null
      if (clearance) {
        const programs = db.query('SELECT program FROM clearance_access_programs WHERE source_id = ?')
          .all(sourceId) as Array<{ program: ClearanceAccessProgram }>
        clearance.access_programs = programs.map(p => p.program)
      }
      return clearance as SourceClearance | null
    }
```

**Add import:** Add `ClearanceAccessProgram` to the imports from `../../types`:
```typescript
import type {
  Source,
  CreateSource,
  UpdateSource,
  SourceStatus,
  SourceType,
  SourceRole,
  SourceProject,
  SourceEducation,
  SourceClearance,
  SourceWithExtension,
  ClearanceAccessProgram,
} from '../../types'
```

**Replace the clearance branch in `create()` (current lines 230-246):**
```typescript
    } else if (sourceType === 'clearance') {
      db.run(
        `INSERT INTO source_clearances (source_id, level, polygraph, status, sponsoring_agency, investigation_date, adjudication_date, reinvestigation_date, read_on)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)`,
        [
          id,
          input.level ?? '',
          input.polygraph ?? null,
          input.clearance_status ?? null,
          input.sponsoring_agency ?? null,
          null,
          null,
          null,
          null,
        ],
      )
    }
```

**With:**
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

**Replace the clearance branch in `updateExtension()` (current lines 124-137):**
```typescript
  } else if (sourceType === 'clearance') {
    const sets: string[] = []
    const params: unknown[] = []

    if ('level' in input) { sets.push('level = ?'); params.push(input.level) }
    if ('polygraph' in input) { sets.push('polygraph = ?'); params.push(input.polygraph ?? null) }
    if ('clearance_status' in input) { sets.push('status = ?'); params.push(input.clearance_status ?? null) }
    if ('sponsoring_agency' in input) { sets.push('sponsoring_agency = ?'); params.push(input.sponsoring_agency ?? null) }

    if (sets.length > 0) {
      params.push(sourceId)
      db.run(`UPDATE source_clearances SET ${sets.join(', ')} WHERE source_id = ?`, params)
    }
  }
```

**With:**
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

**Acceptance criteria:**
- `create()` with `level='top_secret', polygraph='ci', access_programs=['sci']` inserts correctly.
- `get()` returns `access_programs: ['sci']` from junction table hydration.
- `update()` with `access_programs=['sci', 'sap']` replaces old rows (delete-then-insert).
- `update()` with `access_programs=[]` deletes all junction rows.
- `create()` defaults: `level='secret'`, `status='active'`, `type='personnel'`, `continuous_investigation=0`.
- `get()` returns empty array `[]` for `access_programs` when no junction rows exist (not `undefined`).
- After migration 018 removes the `sponsoring_agency` column, any repository code that tries to SET `sponsoring_agency = ?` will fail. Remove `sponsoring_agency` from the UPDATE statement in `updateExtension()` even while keeping the type field as @deprecated.
- Ensure `updateExtension()` handles `continuous_investigation` field: `if ('continuous_investigation' in input) { sets.push('continuous_investigation = ?'); params.push(input.continuous_investigation ?? 0) }`

**Failure criteria:**
- Access programs not hydrated in `get()`.
- Update appends instead of replacing junction rows.
- Old `sponsoring_agency` column referenced in INSERT or UPDATE (column no longer exists after migration 018).

---

### T47.6: Update Core Index Exports [MINOR]

**File:** `packages/core/src/index.ts`

Add re-export for clearance constants.

Add to the exports:
```typescript
export {
  CLEARANCE_LEVELS,
  CLEARANCE_POLYGRAPHS,
  CLEARANCE_STATUSES,
  CLEARANCE_TYPES,
  CLEARANCE_ACCESS_PROGRAMS,
  CLEARANCE_LEVEL_LABELS,
  CLEARANCE_POLYGRAPH_LABELS,
  CLEARANCE_ACCESS_PROGRAM_LABELS,
  CLEARANCE_LEVEL_HIERARCHY,
  clearanceLevelRank,
  clearanceMeetsRequirement,
} from './constants/clearance'
```

**Acceptance criteria:**
- Clearance constants importable from `@forge/core`.

**Failure criteria:**
- Import resolution error.

---

### T47.7: Update Clearance Form in SourcesView [IMPORTANT]

**File:** `packages/webui/src/routes/data/sources/SourcesView.svelte`

Replace free-text clearance inputs with dropdown selects and checkbox group. This task provides the UI specification; the exact line numbers depend on the current state of the file.

**New form state variables** (replace existing clearance text vars):
```typescript
import {
  CLEARANCE_LEVELS,
  CLEARANCE_POLYGRAPHS,
  CLEARANCE_STATUSES,
  CLEARANCE_TYPES,
  CLEARANCE_ACCESS_PROGRAMS,
  CLEARANCE_LEVEL_LABELS,
  CLEARANCE_POLYGRAPH_LABELS,
  CLEARANCE_ACCESS_PROGRAM_LABELS,
} from '@forge/core'
import type {
  ClearanceLevel,
  ClearancePolygraph,
  ClearanceStatus,
  ClearanceType,
  ClearanceAccessProgram,
} from '@forge/sdk'

// Clearance extension fields
let formLevel = $state<ClearanceLevel>('secret')
let formPolygraph = $state<ClearancePolygraph | ''>('')
let formClearanceStatus = $state<ClearanceStatus>('active')
let formClearanceType = $state<ClearanceType>('personnel')
let formSponsorOrgId = $state('')
let formContinuousInvestigation = $state(false)
let formAccessPrograms = $state<ClearanceAccessProgram[]>([])
```

**Reset logic in `startNew()`:**
```typescript
formLevel = 'secret'
formPolygraph = ''
formClearanceStatus = 'active'
formClearanceType = 'personnel'
formSponsorOrgId = ''
formContinuousInvestigation = false
formAccessPrograms = []
```

**Population logic in `startEditing()` / `populateFormFromSource()`:**
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

**Form template (replaces clearance text inputs):**
```svelte
{#if formSourceType === 'clearance'}
  <div class="form-row">
    <div class="form-group">
      <label for="clearance-level">Level</label>
      <select id="clearance-level" bind:value={formLevel}>
        {#each CLEARANCE_LEVELS as level}
          <option value={level}>{CLEARANCE_LEVEL_LABELS[level]}</option>
        {/each}
      </select>
    </div>
    <div class="form-group">
      <label for="clearance-status">Status</label>
      <select id="clearance-status" bind:value={formClearanceStatus}>
        {#each CLEARANCE_STATUSES as status}
          <option value={status}>{status.charAt(0).toUpperCase() + status.slice(1)}</option>
        {/each}
      </select>
    </div>
  </div>

  <div class="form-row">
    <div class="form-group">
      <label for="clearance-polygraph">Polygraph</label>
      <select id="clearance-polygraph" bind:value={formPolygraph}>
        <option value="">-- None --</option>
        {#each CLEARANCE_POLYGRAPHS as poly}
          <option value={poly}>{CLEARANCE_POLYGRAPH_LABELS[poly]}</option>
        {/each}
      </select>
    </div>
    <div class="form-group">
      <label for="clearance-type">Type</label>
      <select id="clearance-type" bind:value={formClearanceType}>
        {#each CLEARANCE_TYPES as ctype}
          <option value={ctype}>{ctype.charAt(0).toUpperCase() + ctype.slice(1)}</option>
        {/each}
      </select>
    </div>
  </div>

  <div class="form-group">
    <label>Access Programs</label>
    <div class="checkbox-group">
      {#each CLEARANCE_ACCESS_PROGRAMS as prog}
        <label class="checkbox-label">
          <input
            type="checkbox"
            checked={formAccessPrograms.includes(prog)}
            onchange={(e) => {
              if (e.currentTarget.checked) {
                formAccessPrograms = [...formAccessPrograms, prog]
              } else {
                formAccessPrograms = formAccessPrograms.filter(p => p !== prog)
              }
            }}
          />
          {CLEARANCE_ACCESS_PROGRAM_LABELS[prog]}
        </label>
      {/each}
    </div>
  </div>

  <div class="form-group">
    <label>
      <input
        type="checkbox"
        bind:checked={formContinuousInvestigation}
      />
      Continuous Investigation (CE/CV)
    </label>
  </div>
{/if}
```

**Save payload construction:**
```typescript
} else if (formSourceType === 'clearance') {
  Object.assign(payload, {
    level: formLevel,
    polygraph: formPolygraph || undefined,
    clearance_status: formClearanceStatus,
    clearance_type: formClearanceType,
    sponsor_organization_id: formSponsorOrgId || undefined,
    continuous_investigation: formContinuousInvestigation ? 1 : 0,
    access_programs: formAccessPrograms,
  })
}
```

**Acceptance criteria:**
- Level dropdown shows all 6 options with human-readable labels.
- Polygraph dropdown shows 3 options plus an empty "None" option.
- Status dropdown shows Active/Inactive.
- Type dropdown shows Personnel/Facility.
- Access Programs shows 3 checkboxes: SCI, SAP, NATO.
- Continuous Investigation is a single checkbox.
- Editing an existing clearance source populates all fields correctly.
- Creating a new clearance source defaults to: level=Secret, status=Active, type=Personnel.
- Save payload includes all new fields.

**Failure criteria:**
- Text inputs still present for clearance fields.
- Enum values not matching database CHECK constraints.
- Access programs not sent as array in payload.

---

### T47.8: Update Clearances Display Page [MINOR]

**File:** `packages/webui/src/routes/experience/clearances/+page.svelte`

Update the clearance list display to use human-readable labels from the constants and show the new fields (type, access programs, continuous investigation).

This task updates the display portion only. The exact changes depend on the current page layout, but the key change is replacing raw enum values with labeled display:

```svelte
<!-- Instead of showing raw "top_secret" -->
<span>{CLEARANCE_LEVEL_LABELS[clearance.level] ?? clearance.level}</span>

<!-- Show access programs as badges -->
{#if clearance.access_programs?.length > 0}
  <div class="program-badges">
    {#each clearance.access_programs as prog}
      <span class="badge">{CLEARANCE_ACCESS_PROGRAM_LABELS[prog]}</span>
    {/each}
  </div>
{/if}

<!-- Show type and CI status -->
<span class="meta">{clearance.type === 'facility' ? 'Facility' : 'Personnel'}</span>
{#if clearance.continuous_investigation}
  <span class="badge ci">CE/CV</span>
{/if}
```

**Acceptance criteria:**
- Clearance list shows "Top Secret (TS)" instead of "top_secret".
- Access programs show as badges (SCI, SAP, NATO).
- Facility vs Personnel type is visible.
- Continuous Investigation shows a CE/CV badge when enabled.

**Failure criteria:**
- Raw enum values displayed to user.
- Missing access program display.

---

## Testing Support

### Test Fixtures

The existing `createTestDb()` helper runs all migrations including the new `018_clearance_structured_data.sql`. No changes to `createTestDb()` are needed.

For tests with clearance data, create sources via the repository:
```typescript
import * as sourceRepo from '../source-repository'

const source = sourceRepo.create(db, {
  title: 'TS/SCI Clearance',
  description: 'Active clearance',
  source_type: 'clearance',
  level: 'top_secret',
  polygraph: 'ci',
  clearance_status: 'active',
  clearance_type: 'personnel',
  access_programs: ['sci'],
})
```

### Unit Tests -- Repository

| Test | Category | Assertion |
|------|----------|-----------|
| Create clearance with all fields | [CRITICAL] | All fields persisted and round-tripped via `get()` |
| Create clearance with access programs | [CRITICAL] | Junction rows created; `get()` returns `access_programs: ['sci']` |
| Create clearance with defaults | [IMPORTANT] | `level='secret'`, `status='active'`, `type='personnel'`, `access_programs=[]` |
| Update clearance level | [CRITICAL] | `secret` -> `top_secret` round-trips correctly |
| Update access programs | [CRITICAL] | `['sci']` -> `['sci', 'sap']` replaces rows |
| Update access programs to empty | [IMPORTANT] | `access_programs=[]` deletes all junction rows |
| Create with sponsor org | [IMPORTANT] | `sponsor_organization_id` FK round-trips |
| CHECK constraint enforcement | [IMPORTANT] | `level='invalid'` throws SQLite constraint error |
| Get clearance with no programs | [IMPORTANT] | Returns `access_programs: []` (empty array, not undefined) |

### Unit Tests -- Hierarchy Utility

| Test | Category | Assertion |
|------|----------|-----------|
| `clearanceMeetsRequirement('top_secret', 'secret')` | [CRITICAL] | `true` |
| `clearanceMeetsRequirement('secret', 'top_secret')` | [CRITICAL] | `false` |
| `clearanceMeetsRequirement('q', 'top_secret')` | [CRITICAL] | `true` (reciprocal) |
| `clearanceMeetsRequirement('top_secret', 'q')` | [CRITICAL] | `true` (reciprocal) |
| `clearanceMeetsRequirement('public', 'confidential')` | [IMPORTANT] | `false` |
| `clearanceMeetsRequirement('secret', 'public')` | [IMPORTANT] | `true` |
| `clearanceLevelRank('public')` | [IMPORTANT] | `0` |
| `clearanceLevelRank('top_secret')` | [IMPORTANT] | `4` |
| `clearanceLevelRank('q')` | [IMPORTANT] | `4` |
| `clearanceLevelRank('l')` | [IMPORTANT] | `1` |

### Integration Tests -- API

| Test | Category | Assertion |
|------|----------|-----------|
| POST clearance source with full payload | [CRITICAL] | 201, structured response with all fields |
| PATCH clearance to change level and add programs | [IMPORTANT] | 200, updated response |
| GET clearance source | [IMPORTANT] | `access_programs` is array (not undefined), all new fields present |

### Contract Tests -- SDK

| Test | Category | Assertion |
|------|----------|-----------|
| SDK `SourceClearance` includes all new fields | [IMPORTANT] | TypeScript compilation succeeds |
| `access_programs` is array when empty | [IMPORTANT] | `[]` not `undefined` |
| `sponsor_organization_id` is nullable | [MINOR] | `null` accepted |

### Migration Integrity Tests

| Test | Category | Assertion |
|------|----------|-----------|
| Row count preserved | [CRITICAL] | Count before = count after |
| "TS/SCI" maps to `level='top_secret'` | [CRITICAL] | Correct enum mapping |
| "CI" maps to `polygraph='ci'` | [CRITICAL] | Correct enum mapping |
| "active" maps to `status='active'` | [IMPORTANT] | Correct enum mapping |
| `sponsoring_agency` creates org and links FK | [IMPORTANT] | `sponsor_organization_id IS NOT NULL` |
| Date fields preserved | [IMPORTANT] | Values unchanged |
| `type` defaults to `'personnel'` | [MINOR] | Default applied |
| `continuous_investigation` defaults to `0` | [MINOR] | Default applied |

---

## Documentation Requirements

- No new documentation files required.
- TSDoc on new union types explains each value's meaning.
- TSDoc on `clearanceMeetsRequirement` explains the hierarchy direction.
- Inline SQL comments in the migration explain the CASE WHEN mapping and the table rebuild pattern.
- `@deprecated` JSDoc on `sponsoring_agency` in `SourceClearance` and `CreateSource`/`UpdateSource`.
- Human-readable label maps serve as self-documenting references for UI consumers.

---

## Parallelization Notes

**Within this phase:**
- T47.1 (migration) must be done first -- it defines the database schema all other tasks depend on.
- T47.2 (constants) and T47.3 (core types) can be done in parallel after T47.1.
- T47.4 (SDK types) depends on T47.3 (mirrors core types).
- T47.5 (repository) depends on T47.1 (schema) and T47.3 (types).
- T47.6 (index exports) depends on T47.2 (constants exist).
- T47.7 (UI form) depends on T47.2 (constants), T47.4 (SDK types), and T47.5 (repository).
- T47.8 (display page) depends on T47.2 (constants) and T47.4 (SDK types).

**Recommended execution order:**
1. T47.1 (migration -- schema foundation)
2. T47.2 + T47.3 (constants + core types -- parallel)
3. T47.4 + T47.5 + T47.6 (SDK types + repository + exports -- parallel, all depend on step 2)
4. T47.7 + T47.8 (UI changes -- parallel, both depend on step 3)

**Cross-phase:**
- Phase 44 (IR Data Quality) modifies the resume compiler. The compiler's `buildClearanceItems` function reads from `source_clearances`. After this phase, the clearance fields have different types (enum instead of free text). The compiler should be updated in a follow-up to use `CLEARANCE_LEVEL_LABELS` for display formatting, but this is not required for correctness -- the compiler uses the raw value, which is now a cleaner string.
- Phase 45 (Editor Restructuring) is purely UI tab changes with no data layer overlap.
- Phase 46 (LaTeX/XeTeX Docs) is documentation-only.
- Migration 017 is reserved for Phase 43 (Generic Kanban). This phase uses migration 018. The next available migration number is 019.
