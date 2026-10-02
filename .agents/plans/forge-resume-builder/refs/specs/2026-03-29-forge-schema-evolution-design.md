# Forge Schema Evolution: Data Migration & Entity Expansion

**Date:** 2026-03-29
**Status:** Design
**Builds on:** 2026-03-28-forge-resume-builder-design.md

## Purpose

Evolve the Forge schema to support importing real resume data from v1, add missing entity types (organizations, education, certifications), and restructure the UI around entity management rather than pipeline stages.

## Goals

1. Non-destructively import v1 data: 73 bullets, 60 skills, 13 roles, 22 education/certs, 18 organizations, 1 clearance, 2 languages, 55 resume_bullet mappings
2. Make sources polymorphic (roles, projects, education, general)
3. Make bullets many-to-many with sources (junction-only, no `source_id` column)
4. Extend the derivation chain with resume entries (copy-on-write)
5. Add workflow notes (per-entity) and user notes (cross-entity, two-way searchable)
6. Restructure UI views around entities

## Non-Goals

- Job tracking (applications, contacts, job boards) — future work
- Resume export to PDF/DOCX — still 501 stub
- Real-time collaboration — single-user app
- Activity log / event feed — deferred (mentioned in UI for future, no schema in this spec)
- v1 entities with 0 records and no immediate use: `awards` (0 records), `research`, `publications`, `events` — schema exists in v1 but no data to migrate

## v1 Data Inventory

| v1 Entity | Records | Migration Target |
|-----------|---------|-----------------|
| organizations | 18 (11 worked) | organizations |
| roles | 13 | sources + source_roles |
| projects | 0 | sources + source_projects (schema only) |
| education | 22 (20 certs, 2 degrees) | sources + source_education |
| bullets | 73 (3 with parent_id) | bullets + bullet_sources |
| skills | 60 | skills |
| bullet_roles | many | bullet_sources |
| bullet_projects | 0 | bullet_sources |
| bullet_education | some | bullet_sources |
| bullet_skills | many | bullet_skills |
| clearances | 1 (TS/SCI) | sources + source_clearances |
| languages | 2 | skills (category = 'language') |
| resumes | 3 | resumes |
| resume_bullets | 55 | resume_entries (via synthetic perspectives) |

---

## 1. Schema Changes

All new tables use STRICT mode. All IDs are UUID TEXT with `CHECK(typeof(id) = 'text' AND length(id) = 36)` consistent with `001_initial.sql`. All dates use `TEXT` (not `DATE`). All booleans use `INTEGER` (not `BOOLEAN`). All timestamps use `TEXT` with ISO 8601 format.

### 1.1 Organizations Replace Employers

**Must be created BEFORE source extension tables** (they reference `organizations`).

```sql
CREATE TABLE organizations (
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
  location TEXT,
  headquarters TEXT,
  website TEXT,
  linkedin_url TEXT,
  glassdoor_url TEXT,
  glassdoor_rating REAL,
  reputation_notes TEXT,
  notes TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;
```

**Migration from employers:** Existing `employers` rows are copied to `organizations` with `worked = 1`, `org_type = 'company'`. New UUIDs preserve the mapping. After data migration, `employers` table is dropped. All existing FK references (`projects.employer_id`, `sources.employer_id`) are handled by dropping those columns in subsequent steps.

### 1.2 Sources Become Polymorphic

The `sources` table becomes a base table. Type-specific fields live in extension tables.

```sql
-- Add source_type to existing sources table
-- (existing rows get 'general' by default)
ALTER TABLE sources ADD COLUMN source_type TEXT NOT NULL DEFAULT 'general'
  CHECK (source_type IN ('role', 'project', 'education', 'clearance', 'general'));

-- Add workflow notes to sources
ALTER TABLE sources ADD COLUMN notes TEXT;
```

The existing `employer_id` and `project_id` columns on `sources` are dropped via table rebuild (SQLite `ALTER TABLE DROP COLUMN` requires SQLite 3.35.0+; Bun's bundled SQLite supports this).

**Extension: Roles**
```sql
CREATE TABLE source_roles (
  source_id TEXT PRIMARY KEY CHECK(typeof(source_id) = 'text' AND length(source_id) = 36)
    REFERENCES sources(id) ON DELETE CASCADE,
  organization_id TEXT REFERENCES organizations(id) ON DELETE SET NULL,
  start_date TEXT,
  end_date TEXT,
  is_current INTEGER NOT NULL DEFAULT 0,
  work_arrangement TEXT,
  base_salary INTEGER,
  total_comp_notes TEXT
) STRICT;
```

**Extension: Projects**
```sql
CREATE TABLE source_projects (
  source_id TEXT PRIMARY KEY CHECK(typeof(source_id) = 'text' AND length(source_id) = 36)
    REFERENCES sources(id) ON DELETE CASCADE,
  organization_id TEXT REFERENCES organizations(id) ON DELETE SET NULL,
  is_personal INTEGER NOT NULL DEFAULT 0,
  url TEXT,
  start_date TEXT,
  end_date TEXT
) STRICT;
```

**Extension: Education (flat polymorphic)**
```sql
CREATE TABLE source_education (
  source_id TEXT PRIMARY KEY CHECK(typeof(source_id) = 'text' AND length(source_id) = 36)
    REFERENCES sources(id) ON DELETE CASCADE,
  education_type TEXT NOT NULL CHECK (education_type IN ('degree', 'certificate', 'course', 'self_taught')),
  institution TEXT,
  field TEXT,
  start_date TEXT,
  end_date TEXT,
  is_in_progress INTEGER NOT NULL DEFAULT 0,
  credential_id TEXT,
  expiration_date TEXT,
  issuing_body TEXT,
  url TEXT
) STRICT;
```

**Extension: Clearances**
```sql
CREATE TABLE source_clearances (
  source_id TEXT PRIMARY KEY CHECK(typeof(source_id) = 'text' AND length(source_id) = 36)
    REFERENCES sources(id) ON DELETE CASCADE,
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

### 1.3 Bullets Many-to-Many with Sources (Junction-Only)

Replace `bullets.source_id` with a junction table. No `source_id` column remains on `bullets`.

```sql
CREATE TABLE bullet_sources (
  bullet_id TEXT NOT NULL CHECK(typeof(bullet_id) = 'text' AND length(bullet_id) = 36)
    REFERENCES bullets(id) ON DELETE CASCADE,
  source_id TEXT NOT NULL CHECK(typeof(source_id) = 'text' AND length(source_id) = 36)
    REFERENCES sources(id) ON DELETE CASCADE,
  is_primary INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (bullet_id, source_id)
) STRICT;

CREATE INDEX idx_bullet_sources_source ON bullet_sources(source_id);
```

**`is_primary` enforcement:** Application-level, not DB constraint. The services ensure at most one `is_primary = 1` per bullet. When a bullet has only one source, that source is primary.

**`source_content_snapshot` semantics:** The `source_content_snapshot` column stays on `bullets`. It always captures the primary source's `description` at derivation time. For imported bullets (human-authored, no derivation), the snapshot is set to the primary source's description at import time.

**Columns dropped from `bullets`:** `source_id`.

**Columns added to `bullets`:** `notes TEXT` (workflow notes).

**Existing code that breaks and must be updated:**
- `BulletRepository.create` — no longer accepts `source_id`; after creating bullet, insert `bullet_sources` row
- `BulletRepository.list` — filter by `source_id` becomes JOIN through `bullet_sources`
- `BulletFilter.source_id` — becomes a JOIN filter
- `DerivationService.deriveBulletsFromSource` — create bullet, then insert `bullet_sources(bullet_id, source_id, is_primary=1)` inside the transaction
- `AuditService.traceChain` — fetch source via `bullet_sources WHERE is_primary = 1` instead of `bullet.source_id`
- `AuditService.checkIntegrity` — same JOIN change for source lookup
- `ReviewService.getPendingReview` — JOIN `bullet_sources bs ON b.id = bs.bullet_id AND bs.is_primary = 1` then `JOIN sources s ON bs.source_id = s.id`
- `ResumeService.findBulletsForGap` — same JOIN pattern
- `ResumeService.getSourceTitle` — same JOIN pattern
- Test helpers: `seedBullet` must create a `bullet_sources` row instead of inserting `source_id`
- SDK types: `Bullet.source_id` removed, add `sources: Array<{id, title, is_primary}>` or similar
- Route source filter: `?source_id=X` on bullets endpoint becomes a junction query

### 1.4 Resume Entries (Chain Extension)

Add a fourth layer: `perspective → resume_entry`. This replaces the current `resume_perspectives` junction table.

```sql
CREATE TABLE resume_entries (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  resume_id TEXT NOT NULL REFERENCES resumes(id) ON DELETE CASCADE,
  perspective_id TEXT NOT NULL REFERENCES perspectives(id) ON DELETE RESTRICT,
  content TEXT,                          -- NULL = reference mode, non-NULL = cloned/edited
  perspective_content_snapshot TEXT,     -- captured when cloned
  section TEXT NOT NULL,
  position INTEGER NOT NULL DEFAULT 0,
  notes TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;
```

**Copy-on-write behavior:**
- Default: `content` is NULL — UI displays the perspective's content (reference mode)
- When user edits: `content` is populated, `perspective_content_snapshot` captures the perspective content at that moment (clone mode)
- User can reset to reference mode: `PATCH /resumes/:id/entries/:entryId` with `{"content": null}` sets `content` back to NULL
- Absent `content` in PATCH body = no change; explicit `null` = reset to reference mode

**Migration from `resume_perspectives`:** Existing `resume_perspectives` rows become `resume_entries` with `content = NULL` (reference mode). The `resume_perspectives` table is then dropped.

**Existing code that breaks:**
- `ResumeRepository` — all methods referencing `resume_perspectives` must use `resume_entries`
- `ResumeService` — `addPerspective`, `removePerspective`, `reorderPerspectives` become entry operations
- Resume routes — `POST /resumes/:id/perspectives` → `POST /resumes/:id/entries`, etc.
- SDK `ResumesResource` — method names and paths update
- `ResumeWithPerspectives` type — becomes `ResumeWithEntries` with `sections: Record<string, Array<ResumeEntry>>`

### 1.5 Workflow Notes (Per-Entity)

Add `notes TEXT` column to existing tables via ALTER TABLE:
- `sources` (added in §1.2)
- `bullets` (added in §1.3)
- `perspectives`
- `skills` (added in §1.7)
- `resumes`
- `resume_entries` (included in §1.4 DDL)

### 1.6 User Notes (Cross-Entity)

```sql
CREATE TABLE user_notes (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  title TEXT,
  content TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

CREATE TABLE note_references (
  note_id TEXT NOT NULL CHECK(typeof(note_id) = 'text' AND length(note_id) = 36)
    REFERENCES user_notes(id) ON DELETE CASCADE,
  entity_type TEXT NOT NULL CHECK (entity_type IN (
    'source', 'bullet', 'perspective', 'resume_entry',
    'resume', 'skill', 'organization'
  )),
  entity_id TEXT NOT NULL,
  PRIMARY KEY (note_id, entity_type, entity_id)
) STRICT;

CREATE INDEX idx_note_refs_entity ON note_references(entity_type, entity_id);
```

**Orphan cleanup:** `note_references` uses polymorphic `entity_id` with no FK enforcement (intentional — single FK can't span multiple tables). When an entity is deleted, orphaned `note_references` rows persist. Cleanup is handled lazily: when fetching notes for a deleted entity, the query returns no results. A periodic cleanup job or on-read pruning can remove stale references. Not critical for single-user app.

### 1.7 Skills Enhancement

```sql
ALTER TABLE skills ADD COLUMN notes TEXT;
```

### 1.8 Perspectives — Notes Column

```sql
ALTER TABLE perspectives ADD COLUMN notes TEXT;
```

### 1.9 Resumes — Notes Column

```sql
ALTER TABLE resumes ADD COLUMN notes TEXT;
```

---

## 2. UI View Restructuring

### Navigation

```
Dashboard        — review queue + integrity alerts + stats
Sources          — tabbed: All | Roles | Projects | Education | Clearances | General
Bullets          — filter: bullets | perspectives | resume entries
Resumes          — builder + gap analysis
Organizations    — list with CRUD
Skills           — list with CRUD + category filter
Archetypes       — view archetype definitions + domain mappings
Chain View       — interactive graph (Sigma.js + graphology)
Logs             — prompt logs
Notes            — user notes with entity linking
```

### Sources View (Tabbed)

- **All** tab: every source regardless of type
- **Roles** tab: `source_type = 'role'`, shows organization, dates, is_current
- **Projects** tab: `source_type = 'project'`, shows org, URL, personal flag
- **Education** tab: `source_type = 'education'`, shows institution, education_type, credential
- **Clearances** tab: `source_type = 'clearance'`, shows level, status, polygraph
- **General** tab: `source_type = 'general'`, catch-all

Each tab has a list + editor panel. The editor form adapts based on source type.

### Bullets View (Unified Content Atoms)

Single list with a type filter dropdown: Bullets | Perspectives | Resume Entries.

Unified view enables cross-chain search ("where did I mention Kubernetes?").

List columns: Content (truncated), Type, Status, Domain/Archetype, Source(s), Created.

### Organizations View

List with CRUD. Shows: name, type, industry, location, worked status. Filter by org_type, worked.

### Skills View

List with CRUD. Shows: name, category, notes. Filter by category. Categories from v1: `ai_ml, cloud, database, devops, frameworks, general, languages, os, security, tools`.

### Chain View (Graph)

Interactive graph using Sigma.js (WebGL-rendered) with graphology for the data layer.

**Node types with colors:**
- Source (role/project/education/clearance/general) → purple
- Bullet → blue
- Perspective → green
- Resume Entry → amber

**Edges:** directed, showing derivation flow. Solid = snapshot matches. Dashed = drifted.

**Interactions:**
- Click node → details panel
- Hover edge → snapshot diff if drifted
- Filter by source type, status, archetype
- Search nodes by content
- Highlight path from any node back to its source(s)

**Secondary mode (toggle):** Sankey diagram showing flow volumes.

### Content Drift (Integrity Alerts)

**Dashboard (B):** "Integrity Alerts" card — count of drifted entities. Click to see full list. Batch actions: re-derive, dismiss.

**On-entity banners (C):** When viewing a bullet or perspective with stale snapshot:
```
Source content has changed since this bullet was derived.
[View diff]  [Re-derive]  [Dismiss]
```

### Notes View

List of user notes with search. Each note shows linked entities as tags. Click tag to navigate. Create notes from here or from any entity detail panel.

### Logs View

Prompt logs — AI derivation audit trail (existing `prompt_logs` table). Shows template, input, response, entity link.

---

## 3. Data Migration (v1 → Forge)

### Migration Script

CLI command: `forge import v1 <path-to-v1-db>`

Lives in `packages/cli/src/commands/import.ts`. Reads v1 SQLite directly (opened read-only). Writes to the Forge database via the SDK or direct DB access.

### Idempotency

A `v1_import_map` table in the Forge database tracks what has been imported:

```sql
CREATE TABLE v1_import_map (
  v1_entity_type TEXT NOT NULL,
  v1_id INTEGER NOT NULL,
  forge_id TEXT NOT NULL,
  imported_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  PRIMARY KEY (v1_entity_type, v1_id)
) STRICT;
```

On re-run, the script checks this table and skips already-imported records. This is more reliable than content-hash matching (content may be edited in Forge after import).

### Mapping

| v1 Entity | Forge Entity | Details |
|-----------|-------------|---------|
| `organizations` | `organizations` | Direct field mapping. v1 `worked` → Forge `worked`. v1 `org_type`, `industry`, `size`, `employment_type`, `location`, `website`, etc. all carry over. Integer IDs → UUIDs via `v1_import_map`. |
| `roles` | `sources` + `source_roles` | Role title → `sources.title`. Source description = concatenation of the role's linked bullet contents (provides a narrative summary). `source_type = 'role'`. Extension fields: `organization_id` (mapped from `roles.organization_id` → orgs import map), `start_date`, `end_date`, `is_current`, `work_arrangement`. |
| `projects` | `sources` + `source_projects` | 0 records in v1, but schema is ready. `source_type = 'project'`. If v1 projects had `type` values (`work`, `personal`, `open_source`): `personal` and `open_source` → `is_personal = 1`, `work` → `is_personal = 0`. Organization linkage: v1 links projects to roles via `project_roles`, not directly to orgs. For import, organization is resolved via the primary role's org. |
| `education` | `sources` + `source_education` | `source_type = 'education'`. v1 `type` maps to `education_type`: `certificate` → `certificate`, `degree` → `degree`. (v1 data only has these two types.) Institution, field, dates, credential_id, expiration_date, url carry over. `issuing_body` derived from `institution` for certs. |
| `clearances` | `sources` + `source_clearances` | 1 record (TS/SCI). `source_type = 'clearance'`. Title = "TS/SCI Security Clearance". Description from notes. Extension: level, polygraph, status, dates. |
| `languages` | `skills` | 2 records (English, Japanese). Imported as skills with `category = 'language'`. Proficiency info stored in `notes`. |
| `bullets` | `bullets` | Content, notes carry over. Status = `approved`. `updated_by = 'human'`. `source_content_snapshot` = primary source's description at import time. |
| `bullets.parent_id` | Flattened | 3 bullets have `parent_id`. Imported as independent bullets (hierarchy dropped). Parent-child relationship is not meaningful in Forge's derivation model. |
| `bullets.framing` | `bullets` metadata column | v1 framing values: `ai_ml`, `devops`, `leadership`, `security`, `software_engineering`, `systems_engineering`. These map directly to Forge domains. Added as a `domain` column on `bullets` (nullable TEXT). Used later when deriving perspectives — the domain is pre-populated as a suggestion. |
| `bullet_roles` | `bullet_sources` | Maps v1 role ID → source UUID via import map. `is_primary` carries over from v1. |
| `bullet_projects` | `bullet_sources` | 0 records. Schema ready. `is_primary = 0` (projects are secondary associations). |
| `bullet_education` | `bullet_sources` | Maps education → source. `is_primary = 0` (education is a secondary association). |
| `skills` | `skills` | Direct mapping. Name, category carry over. v1 categories: `ai_ml, cloud, database, devops, frameworks, general, languages, os, security, tools`. |
| `bullet_skills` | `bullet_skills` | Junction mapping with new UUIDs via import map. |
| `resumes` | `resumes` | Name carries over. `target_company` → `target_employer`. `target_role` carries over. `archetype` inferred from resume name or set to NULL for manual assignment. |
| `resume_bullets` | `resume_entries` | v1 links bullets directly to resumes. Forge requires perspectives in between. For each `resume_bullet`, create a **synthetic identity perspective**: `content` = bullet content, `bullet_content_snapshot` = bullet content, `framing` = `'accomplishment'` (default), `target_archetype` = NULL, `domain` = bullet's domain/framing value, `status` = `'approved'`. Then create a `resume_entry` referencing this perspective with `content = NULL` (reference mode). |
| `resume_bullets.section` | `resume_entries.section` | v1 sections: `ai_engineering, devops, government, infrastructure, leadership, security, software_engineering`. These are domain-like categories, not Forge's structural sections (`summary, work_history, projects, education, skills, awards`). **Mapping:** All v1 sections map to `work_history` (they're all work experience bullets). The v1 section value is preserved in the resume_entry's `notes` field as `"v1_section: {value}"` for reference. |

### Migration Order

1. Create `v1_import_map` table
2. `organizations` (no dependencies)
3. `skills` + `languages` as skills (no dependencies)
4. `sources` + `source_roles` (depends on organizations)
5. `sources` + `source_education` (depends on organizations — institution may map)
6. `sources` + `source_clearances` (no org dependency)
7. `sources` + `source_projects` (depends on organizations; 0 records but schema ready)
8. `bullets` (no source_id column, just the bullet itself)
9. `bullet_sources` junction (depends on bullets + sources)
10. `bullet_skills` (depends on bullets + skills)
11. `resumes` (no data dependencies)
12. Synthetic `perspectives` for resume_bullets (depends on bullets)
13. `resume_entries` (depends on resumes + perspectives)

### Non-Destructive Guarantees

- v1 database is opened **read-only** (`{ readonly: true }`)
- Forge database is backed up before import (`just dump`)
- Import is idempotent via `v1_import_map` — re-running skips existing records
- No v1 data is modified or deleted
- Import script prints summary: entities imported, skipped, failed

---

## 4. Schema Migration DDL Order

The schema migration (`002_schema_evolution.sql`) must execute in this order:

1. Create `organizations` table
2. Migrate `employers` → `organizations` (INSERT INTO organizations SELECT ... FROM employers)
3. Add `source_type` and `notes` columns to `sources`
4. Create `source_roles`, `source_projects`, `source_education`, `source_clearances`
5. Migrate existing `sources.employer_id` data into `source_roles.organization_id` where applicable
6. Drop `sources.employer_id` and `sources.project_id` columns (table rebuild)
7. Drop `employers` table
8. Drop `projects` table (subsumed by `source_projects`)
9. Create `bullet_sources` table
10. Migrate `bullets.source_id` → `bullet_sources(bullet_id, source_id, is_primary=1)`
11. Drop `bullets.source_id` column (table rebuild)
12. Add `notes` column to `bullets`
13. Add `domain` column to `bullets` (nullable TEXT, for v1 framing import)
14. Add `notes` column to `perspectives`, `resumes`
15. Create `resume_entries` table
16. Migrate `resume_perspectives` → `resume_entries` (content=NULL, reference mode)
17. Drop `resume_perspectives` table
18. Create `user_notes` + `note_references` tables
19. Create `v1_import_map` table
20. Add `notes` column to `skills`

**SQLite constraint:** `ALTER TABLE DROP COLUMN` requires SQLite 3.35.0+. Bun 1.x bundles SQLite 3.38.0+, so this is supported. If needed, the alternative is the table-rebuild pattern (CREATE new → INSERT SELECT → DROP old → ALTER RENAME).

---

## 5. API Changes

### New Endpoints

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/organizations` | Create organization |
| GET | `/api/organizations` | List organizations (filter: org_type, worked) |
| GET | `/api/organizations/:id` | Get organization |
| PATCH | `/api/organizations/:id` | Update organization |
| DELETE | `/api/organizations/:id` | Delete organization (SET NULL on source_roles) |
| POST | `/api/resumes/:id/entries` | Add resume entry (perspective_id, section, position) |
| GET | `/api/resumes/:id/entries` | List entries for a resume |
| PATCH | `/api/resumes/:id/entries/:entryId` | Update entry (content, section, position, notes). `content: null` = reset to reference mode |
| DELETE | `/api/resumes/:id/entries/:entryId` | Remove entry from resume |
| POST | `/api/notes` | Create user note |
| GET | `/api/notes` | List notes (search by content) |
| GET | `/api/notes/:id` | Get note with linked entities |
| PATCH | `/api/notes/:id` | Update note |
| DELETE | `/api/notes/:id` | Delete note (cascades to note_references) |
| POST | `/api/notes/:id/references` | Link entity to note (entity_type, entity_id) |
| DELETE | `/api/notes/:id/references/:entityType/:entityId` | Unlink entity from note |
| GET | `/api/integrity/drift` | List all entities with stale snapshots |

### Modified Endpoints

| Endpoint | Change |
|----------|--------|
| `POST /api/sources` | Accepts `source_type` + extension fields in body. Creates base + extension atomically. |
| `GET /api/sources` | Returns `source_type`. Supports `?source_type=role` filter. Extension data included in response. |
| `GET /api/sources/:id` | Response includes extension data (role fields, education fields, etc.) based on `source_type`. |
| `PATCH /api/sources/:id` | Can update extension fields alongside base fields. |
| `GET /api/bullets` | `?source_id=X` filter works via `bullet_sources` JOIN. Response no longer has `source_id` — has `sources: [{id, title, is_primary}]`. |
| `GET /api/bullets/:id` | Same — `sources` array replaces `source_id`. |
| `POST /api/sources/:id/derive-bullets` | Creates `bullet_sources` row alongside bullet. |
| `GET /api/review/pending` | Source title fetched via `bullet_sources` JOIN. |
| Resume perspective endpoints | Replaced by entry endpoints (see new endpoints above). |

### Removed Endpoints

| Endpoint | Replacement |
|----------|-------------|
| `POST /api/resumes/:id/perspectives` | `POST /api/resumes/:id/entries` |
| `DELETE /api/resumes/:id/perspectives/:pid` | `DELETE /api/resumes/:id/entries/:eid` |
| `PATCH /api/resumes/:id/reorder` | `PATCH /api/resumes/:id/entries/:eid` (update position per entry) |

---

## 6. Service Layer Changes

### DerivationService

`deriveBulletsFromSource` transaction updated:
```
1. Get source, acquire lock (unchanged)
2. Render prompt, invoke AI, validate (unchanged)
3. Transaction:
   a. Create bullet (NO source_id)
   b. INSERT INTO bullet_sources (bullet_id, source_id, is_primary=1)
   c. Create bullet_technologies (unchanged)
   d. Create prompt_log (unchanged)
   e. Release lock (unchanged)
```

### AuditService

`traceChain(perspectiveId)`:
```
1. Get perspective (unchanged)
2. Get bullet (unchanged)
3. Get primary source: SELECT s.* FROM sources s
   JOIN bullet_sources bs ON s.id = bs.source_id
   WHERE bs.bullet_id = ? AND bs.is_primary = 1
```

`checkIntegrity(perspectiveId)`:
- Same change — source lookup via `bullet_sources` JOIN
- `source_snapshot_matches` compares `bullet.source_content_snapshot` against primary source's description

### ReviewService

`getPendingReview` bullet query:
```sql
SELECT b.*, s.title AS source_title
FROM bullets b
JOIN bullet_sources bs ON b.id = bs.bullet_id AND bs.is_primary = 1
JOIN sources s ON bs.source_id = s.id
WHERE b.status = 'pending_review'
```

### ResumeService

- `addPerspective` → `addEntry` (creates resume_entry)
- `removePerspective` → `removeEntry`
- `reorderPerspectives` → individual entry position updates
- `getResume` → returns `ResumeWithEntries` (entries grouped by section, each with resolved content)
- `analyzeGaps` — source title lookup via `bullet_sources` JOIN

### New Services

- `OrganizationService` — CRUD for organizations
- `NoteService` — CRUD for user_notes + note_references
- `IntegrityService` — replaces ad-hoc audit checks; provides `getDriftedEntities()` for dashboard

---

## 7. SDK Changes

### New Types

```typescript
interface Organization { id, name, org_type, industry, size, worked, employment_type, location, ... }
interface ResumeEntry { id, resume_id, perspective_id, content, perspective_content_snapshot, section, position, notes, ... }
interface UserNote { id, title, content, references: NoteReference[], ... }
interface NoteReference { entity_type, entity_id }
interface SourceRole { organization_id, start_date, end_date, is_current, ... }
interface SourceEducation { education_type, institution, field, credential_id, ... }
interface SourceClearance { level, polygraph, status, ... }
interface SourceProject { organization_id, is_personal, url, ... }
```

### Modified Types

```typescript
// Source gains source_type + extension data
interface Source {
  // ... existing fields minus employer_id, project_id
  source_type: 'role' | 'project' | 'education' | 'clearance' | 'general'
  role?: SourceRole        // present when source_type = 'role'
  project?: SourceProject  // present when source_type = 'project'
  education?: SourceEducation
  clearance?: SourceClearance
}

// Bullet loses source_id, gains sources array
interface Bullet {
  // ... existing fields minus source_id
  sources: Array<{ id: string; title: string; is_primary: boolean }>
  domain: string | null  // from v1 framing import
}

// Resume uses entries instead of perspectives
interface ResumeWithEntries extends Resume {
  sections: Record<string, ResumeEntry[]>
}
```

### New SDK Resources

- `OrganizationsResource` — CRUD
- `NotesResource` — CRUD + references
- Updated `ResumesResource` — entries instead of perspectives

---

## 8. Acceptance Criteria

### Schema
- [ ] `organizations` table created with all v1 fields, UUID PKs, STRICT mode
- [ ] `employers` table dropped after data migration to `organizations`
- [ ] `projects` table dropped (subsumed by `source_projects`)
- [ ] `sources` has `source_type` discriminator column
- [ ] Extension tables exist: `source_roles`, `source_projects`, `source_education`, `source_clearances`
- [ ] All extension tables use UUID CHECK constraints, STRICT mode, TEXT for dates, INTEGER for booleans
- [ ] `bullet_sources` junction replaces `bullets.source_id` — column dropped
- [ ] `bullet_sources.is_primary` enforced at application layer (at most one per bullet)
- [ ] `bullets` has `domain` and `notes` columns
- [ ] `resume_entries` replaces `resume_perspectives` — old table dropped
- [ ] `resume_entries` implements copy-on-write: `content = NULL` = reference, non-NULL = clone
- [ ] `resume_entries` has `updated_at` column
- [ ] `notes TEXT` column on: sources, bullets, perspectives, resumes, skills, resume_entries
- [ ] `user_notes` + `note_references` tables with correct CHECK constraints and indexes
- [ ] `v1_import_map` table for idempotent migration tracking

### Data Migration
- [ ] All 18 v1 organizations imported
- [ ] All 13 v1 roles imported as sources with `source_roles` extension
- [ ] All 22 v1 education entries imported as sources with `source_education` extension
- [ ] 1 v1 clearance imported as source with `source_clearances` extension
- [ ] 2 v1 languages imported as skills with `category = 'language'`
- [ ] All 73 v1 bullets imported with `status = 'approved'`, `updated_by = 'human'`
- [ ] 3 hierarchical bullets (parent_id) imported as independent bullets
- [ ] `bullet.framing` values imported as `bullets.domain`
- [ ] All `bullet_roles` mapped to `bullet_sources` with `is_primary` preserved
- [ ] All `bullet_education` mapped to `bullet_sources` with `is_primary = 0`
- [ ] All 60 v1 skills imported with categories
- [ ] All `bullet_skills` associations preserved
- [ ] All 3 v1 resumes imported
- [ ] All 55 `resume_bullets` imported via synthetic perspectives → resume_entries
- [ ] v1 resume sections (`ai_engineering`, `devops`, etc.) mapped to `work_history` with original value in notes
- [ ] v1 database is not modified (opened read-only)
- [ ] Import is idempotent via `v1_import_map`
- [ ] Import prints summary (counts per entity type)

### Services
- [ ] `DerivationService` creates `bullet_sources` row in transaction (no `source_id` on bullet)
- [ ] `AuditService` traces chain via `bullet_sources WHERE is_primary = 1`
- [ ] `ReviewService` JOINs source title via `bullet_sources`
- [ ] `ResumeService` uses `resume_entries` instead of `resume_perspectives`
- [ ] `ResumeService.analyzeGaps` works with new join pattern
- [ ] New `OrganizationService` with CRUD
- [ ] New `NoteService` with CRUD + entity linking
- [ ] New `IntegrityService` with `getDriftedEntities()`

### API
- [ ] Source CRUD returns `source_type` + extension data
- [ ] Source `?source_type=role` filter works
- [ ] Bullet response has `sources` array, no `source_id`
- [ ] Bullet `?source_id=X` filter works via junction
- [ ] Organization CRUD endpoints (POST, GET list, GET by id, PATCH, DELETE)
- [ ] Resume entry endpoints replace perspective endpoints
- [ ] Entry `PATCH` with `content: null` resets to reference mode
- [ ] User note CRUD + reference linking endpoints
- [ ] `GET /integrity/drift` returns all stale-snapshot entities

### SDK
- [ ] `Source` type has `source_type` + optional extension objects
- [ ] `Bullet` type has `sources` array, no `source_id`
- [ ] `OrganizationsResource` with CRUD methods
- [ ] `NotesResource` with CRUD + reference methods
- [ ] `ResumesResource` uses entries instead of perspectives

### UI
- [ ] Sources view has tabs: All, Roles, Projects, Education, Clearances, General
- [ ] Source editor form adapts to source type
- [ ] Bullets view shows bullets, perspectives, resume entries with type filter
- [ ] Organizations view with list + CRUD
- [ ] Skills view with list + CRUD + category filter
- [ ] Chain View renders interactive graph (Sigma.js + graphology)
- [ ] Dashboard shows integrity alerts
- [ ] Entity detail views show drift banner
- [ ] Resume builder uses resume_entries with copy-on-write toggle
- [ ] Notes view with entity linking
- [ ] Logs view shows prompt logs

### Tests
- [ ] All existing tests updated for schema changes (seedBullet, seedSource, etc.)
- [ ] Source polymorphic round-trip test (create role source → GET → verify extension data)
- [ ] Bullet many-to-many test (create bullet with two sources, one primary)
- [ ] Derivation creates `bullet_sources` row in transaction
- [ ] Integrity check works via `bullet_sources` JOIN
- [ ] Resume entry copy-on-write toggle test
- [ ] Idempotent import test (run twice, verify no duplicates)
- [ ] Organization delete cascades SET NULL to source_roles
- [ ] Note two-way linking test (create → query by entity → query by note)

---

## 9. Dependencies & Parallelization

### Sequential Bottleneck

The schema migration (`002_schema_evolution.sql`) must be complete and tested before any parallel work begins. This includes updating `seedBullet`, `seedSource`, and other test helpers.

### Parallel Tracks (after schema migration)

**Track A — New Repositories + Services:**
- `OrganizationRepository` + `OrganizationService`
- `NoteRepository` + `NoteService`
- `ResumeEntryRepository` (replaces resume_perspectives logic)
- `IntegrityService`

**Track B — Existing Service Updates:**
- `DerivationService` rewrite (bullet_sources)
- `AuditService` rewrite (bullet_sources JOIN)
- `ReviewService` rewrite (bullet_sources JOIN)
- `ResumeService` rewrite (resume_entries)
- `SourceService` + `SourceRepository` (polymorphic extension handling)
- `BulletService` + `BulletRepository` (drop source_id, add domain)

**Track C — SDK + CLI:**
- New SDK types and resources
- CLI command updates for new entities
- `forge import v1` command

**Track D — Routes:**
- Organization routes
- Note routes
- Updated source routes (extension data)
- Updated bullet routes (sources array)
- Resume entry routes (replace perspective routes)
- Integrity drift endpoint

**Track E — UI (can start with mocks):**
- Sources tabbed view
- Organizations view
- Bullets unified view
- Chain graph (Sigma.js)
- Notes view
- Skills view
- Updated resume builder (entries + copy-on-write)
- Dashboard integrity alerts

Tracks A and B can run in parallel. Track C depends on A+B (SDK wraps services). Track D depends on A+B (routes call services). Track E can start with mock data before A-D complete.
