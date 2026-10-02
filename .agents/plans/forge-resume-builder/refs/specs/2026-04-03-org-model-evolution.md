# Organization Model Evolution

**Date:** 2026-04-03
**Status:** Retroactive documentation + forward cleanup roadmap
**Migrations covered:** 009-015
**Related specs:**
- [Education Sub-Type Fields](./2026-04-03-education-subtype-fields.md)
- [Org Kanban Board](./2026-04-03-org-kanban-board.md)
- [Education Type Model Future](../../../docs/brainstorm/education-type-model-future.md)
- [Nav Restructuring](./2026-03-30-nav-restructuring.md)

## Migration Index

| Migration | Name | Purpose |
|-----------|------|---------|
| 009 | `education_subtype_fields` | degree_level, degree_type, certificate_subtype, gpa, location, edu_description on source_education |
| 010 | `education_org_fk` | organization_id FK on source_education; data migration from text institution/issuing_body |
| 011 | `org_tags` | Multi-label org_tags junction table; seeds from org_type |
| 012 | `org_kanban_statuses` | Table rebuild to expand status CHECK for kanban pipeline |
| 013 | `org_campuses` | org_campuses table with modality, address, city, state, country; campus_id FK on source_education |
| 014 | `campus_zipcode_hq` | Adds zipcode and is_headquarters columns to org_campuses |
| 015 | `org_aliases` | org_aliases table for case-insensitive shorthand search |

---

## Part A: What Was Built (Retroactive)

### 1. Education Sub-Type Fields (Migration 009)

**What changed:**

- **Schema:** 6 new columns on `source_education`: `degree_level` (CHECK enum: associate/bachelors/masters/doctoral/graduate_certificate), `degree_type` (free text), `certificate_subtype` (CHECK enum: professional/vendor/completion), `gpa` (free text), `location` (free text), `edu_description` (free text). All nullable. Added via ALTER TABLE.
- **Types:** `DegreeLevelType`, `CertificateSubtype`, `EducationType` union types added to both `packages/core/src/types/index.ts` and `packages/sdk/src/types.ts`. `SourceEducation` interface extended with the 6 new fields.
- **Repository:** `source-repository.ts` — `create()` includes the 6 new columns in the INSERT. `updateExtension()` handles conditional SET clauses for each new field.
- **UI:** `SourcesView.svelte` shows conditional education forms based on `formEducationType`. Degree form shows degree_level/degree_type/gpa/field/location. Certificate form shows certificate_subtype/credential_id. Course form shows institution/location. Self-taught shows a description textarea.

**Why:** The flat `source_education` table had no way to distinguish between degree levels (BS vs MS vs PhD) or certificate categories (vendor vs professional). Resume rendering needs this distinction to format education items correctly (e.g., "M.S. in Computer Science, GPA: 3.9/4.0" vs "AWS Solutions Architect Professional -- Credential ID: ABC123"). The flat-table approach (Approach 1) was chosen over sub-type extension tables because column count is manageable (<20), the tool is single-user, and validation is UI-enforced.

**Current state:**
- Migration: `packages/core/src/db/migrations/009_education_subtype_fields.sql`
- Types: `packages/core/src/types/index.ts` (SourceEducation, lines ~206-231)
- SDK types: `packages/sdk/src/types.ts` (SourceEducation, lines ~88-112)
- Full spec: `refs/specs/2026-04-03-education-subtype-fields.md`

---

### 2. Education -> Organization FK (Migration 010)

**What changed:**

- **Schema:** Added `organization_id TEXT REFERENCES organizations(id) ON DELETE SET NULL` column to `source_education`. Created index `idx_source_education_org` on the FK column.
- **Data migration:** The migration auto-creates Organization rows from existing `institution` and `issuing_body` text values (deduplicating against existing orgs by name), then links each `source_education` row to the matching org by name. Education-type sources get `org_type='education'`, cert/other sources get `org_type='company'`. Migration 010 maps `degree` and `course` sources to `org_type = 'education'`, all others (including `self_taught` and `certificate`) to `org_type = 'company'`.
- **Types:** `SourceEducation` gained `organization_id: string | null`. The `institution` and `issuing_body` fields were annotated `@deprecated` but retained in the interface for legacy data reads.
- **UI:** The education form in `SourcesView.svelte` uses an org `<select>` dropdown (bound to `formEduOrgId`) instead of free-text institution/issuing_body fields. A "+" button opens a quick-create org modal for inline org creation.

**Why:** Free-text `institution` and `issuing_body` fields had no referential integrity. The same institution could be spelled differently across education sources ("WGU" vs "Western Governors University"). By linking to the `organizations` table via FK, education sources share the same normalized org record that roles and projects use, enabling cross-entity queries like "show everything associated with this organization."

**Current state:**
- Migration: `packages/core/src/db/migrations/010_education_org_fk.sql`
- Deprecated columns `institution` and `issuing_body` remain in schema (SQLite cannot conditionally drop columns; see Part B item 1)

---

### 3. Organization Tags (Migration 011)

**What changed:**

- **Schema:** New `org_tags` junction table with `(organization_id, tag)` composite PK, CASCADE delete, and a CHECK constraint on `tag` allowing: company, vendor, platform, university, school, nonprofit, government, military, conference, volunteer, freelance, other. Index on `tag` column.
- **Data migration:** Seeds tags from each org's existing `org_type` value. Maps `org_type='education'` to the `university` tag. Tags like `vendor`, `platform`, `school`, `conference` are never auto-seeded from `org_type` -- they were added manually via the UI or data fixes. Only the base `org_type` value is seeded as the initial tag.
- **Repository:** `organization-repository.ts` gained tag helper functions:
  - `getTagsForOrg()` — reads tags for a single org
  - `setTags()` — replaces all tags for an org (delete + re-insert)
  - `withTags()` / `withTagsBatch()` — attaches tags to org rows after SELECT
  - `list()` supports `tag` filter via JOIN on `org_tags`
  - `create()` defaults tags to `[org_type]` if no tags provided
  - `update()` replaces tags when `input.tags` is provided
- **Types:** `OrgTag` union type added to core and SDK types. `Organization` interface gained `tags: OrgTag[]`.
- **UI:** The org editor at `/data/organizations` shows a checkbox grid of all tags. The list panel filters by tag via a `<select>` dropdown.

**Why:** A single `org_type` enum is too coarse. An organization can be both a `company` and a `vendor` (e.g., AWS). A university might also be tagged `school`. Tags enable the `eduFilteredOrgs` pattern in SourcesView — when the user selects education_type=degree, the org dropdown shows only orgs tagged `university` or `school`; when they select certificate_subtype=vendor, it shows orgs tagged `vendor` or `company`.

**Current state:**
- Migration: `packages/core/src/db/migrations/011_org_tags.sql`
- Repository: `packages/core/src/db/repositories/organization-repository.ts` (tag helpers at lines ~50-84)
- Types: `OrgTag` at `packages/core/src/types/index.ts` line ~79, `packages/sdk/src/types.ts` line ~201

---

### 4. Kanban Pipeline (Migration 012)

**What changed:**

- **Schema:** Table rebuild of `organizations` to expand the `status` CHECK constraint. Old values (`interested`, `review`, `targeting`) remapped to new values via CASE WHEN during data migration:
  - `interested` -> `backlog`
  - `review` -> `researching`
  - `targeting` -> `interested`
  - `excluded` -> `excluded` (unchanged)

  New valid statuses: `backlog | researching | exciting | interested | acceptable | excluded`. Added `idx_organizations_name` index for picker search performance.
- **Types:** `OrganizationStatus` type updated. `CreateOrganization.status` and `UpdateOrganization.status` in SDK use the new union.
- **UI:** New kanban board at `/opportunities/organizations` with 4 columns:
  - **Backlog** (status: backlog) -- purple accent
  - **Researching** (status: researching) -- amber accent
  - **Targeting** (statuses: exciting/interested/acceptable) -- green accent, color-coded interest levels
  - **Excluded** (status: excluded) -- gray, collapsed by default

  Components: `KanbanBoard.svelte`, `KanbanColumn.svelte`, `KanbanCard.svelte`, `OrgPickerModal.svelte`, `OrgDetailModal.svelte`. Drag-and-drop via `svelte-dnd-action`. Dropping into Targeting defaults to `interested`. Interest level changed via dropdown in OrgDetailModal.

**Why:** The organization vetting workflow is a pipeline: discover -> research -> decide interest level -> exclude. A kanban board models this naturally. The three interest levels within Targeting (exciting/interested/acceptable) let the user prioritize without needing a separate ranking system.

**Current state:**
- Migration: `packages/core/src/db/migrations/012_org_kanban_statuses.sql`
- Board: `packages/webui/src/lib/components/kanban/KanbanBoard.svelte`
- Full spec: `refs/specs/2026-04-03-org-kanban-board.md`

---

### 5. Campuses (Migrations 013-014)

**What changed:**

- **Schema (013):** New `org_campuses` table:
  ```
  id TEXT PK (UUID)
  organization_id TEXT NOT NULL FK -> organizations(id) CASCADE
  name TEXT NOT NULL
  modality TEXT NOT NULL DEFAULT 'in_person' CHECK (in_person|remote|hybrid)
  address TEXT
  city TEXT
  state TEXT
  country TEXT
  created_at TEXT NOT NULL DEFAULT now
  ```
  Index on `organization_id`. Also added `campus_id TEXT REFERENCES org_campuses(id) ON DELETE SET NULL` to `source_education`.

- **Schema (014):** Added `zipcode TEXT` and `is_headquarters INTEGER NOT NULL DEFAULT 0` to `org_campuses`.

- **Repository:** `campus-repository.ts` with `create()`, `listByOrg()`, `get()`, `del()`. No `update()` method yet (see Part B item 2).

- **Types:** `OrgCampus` interface in core and SDK types with all columns. `CampusModality` union type. `SourceEducation` gained `campus_id: string | null`.

- **API:** Routes in `packages/core/src/routes/campuses.ts`:
  - `GET /organizations/:orgId/campuses` -- list campuses for an org
  - `POST /organizations/:orgId/campuses` -- create campus
  - `DELETE /campuses/:id` -- delete campus

- **UI:** The org editor at `/data/organizations` includes a "Campuses / Locations" section (visible for existing orgs only). Users can add campuses with name, modality, address, city, state, zip, country, and HQ flag. Each campus renders as a card showing name, HQ badge, modality pill, and city/state/zip. Delete is supported. The education form in `SourcesView.svelte` has a campus dropdown that loads campuses for the selected org.

**Why:** Organizations have multiple locations. A university has campuses (main campus, online). A company has offices (HQ, remote). Structured location data enables campus-specific education entries (e.g., "WGU - Online Campus") and future features like proximity search. The `is_headquarters` flag identifies the primary office. Zipcode was added in 014 because it is more useful than full address for commute-distance calculations and job search filtering.

**Current state:**
- Migrations: `013_org_campuses.sql`, `014_campus_zipcode_hq.sql`
- Repository: `packages/core/src/db/repositories/campus-repository.ts`
- Routes: `packages/core/src/routes/campuses.ts` (campus and alias routes combined)
- Types: `OrgCampus` at `packages/core/src/types/index.ts` lines ~57-69

---

### 6. Aliases (Migration 015)

**What changed:**

- **Schema:** New `org_aliases` table:
  ```
  id TEXT PK (UUID)
  organization_id TEXT NOT NULL FK -> organizations(id) CASCADE
  alias TEXT NOT NULL COLLATE NOCASE
  UNIQUE(organization_id, alias)
  ```
  Indexes on `alias COLLATE NOCASE` and `organization_id`.

- **API:** Routes in `packages/core/src/routes/campuses.ts` (alias routes share the file):
  - `GET /organizations/:orgId/aliases` -- list aliases
  - `POST /organizations/:orgId/aliases` -- create alias (validates non-empty, catches UNIQUE violations)
  - `DELETE /aliases/:id` -- delete alias

- **Repository (search integration):** The `organization-repository.ts` `list()` function's `search` filter was updated to search aliases:
  ```sql
  WHERE o.name LIKE ? OR o.id IN (SELECT organization_id FROM org_aliases WHERE alias LIKE ?)
  ```
  This means searching for "WGU" matches an org named "Western Governors University" if "WGU" is registered as an alias.

- **UI:** The org editor shows an "Aliases" section with an inline text input. Aliases render as pill badges with a delete button. Adding is done on Enter or click.

**Why:** Organizations are commonly known by abbreviations, acronyms, or informal names. "WGU" = Western Governors University, "USAF" = United States Air Force. Without aliases, the user must search by full name. Aliases enable fuzzy matching in any org search/picker context. `COLLATE NOCASE` ensures case-insensitive matching.

**Current state:**
- Migration: `packages/core/src/db/migrations/015_org_aliases.sql`
- Routes: `packages/core/src/routes/campuses.ts` (lines ~42-73)
- Search: `packages/core/src/db/repositories/organization-repository.ts` (line ~165)

---

### 7. Source Extension Mapping (mapExtension)

**What changed:**

- **API:** The `mapExtension()` helper in `packages/core/src/routes/sources.ts` converts the internal `SourceWithExtension` shape (which has a flat `extension` field) to the SDK's expected shape where extension data lives under a typed key:
  ```
  { extension: {...} }  -->  { education: {...} }   (when source_type='education')
  { extension: {...} }  -->  { role: {...} }         (when source_type='role')
  { extension: {...} }  -->  { project: {...} }      (when source_type='project')
  { extension: {...} }  -->  { clearance: {...} }    (when source_type='clearance')
  ```
  The `mapExtension` helper is applied to source responses in three places: POST (create), GET (single), and PATCH (update) handlers. The list endpoint applies it via `.map(mapExtension)` on the array. All four endpoints transform `extension` to the typed key.

- **API (reverse direction):** On POST and PATCH, the route handler spreads the SDK's nested keys back into a flat structure for the core:
  ```typescript
  const input = { ...body, ...(body.education ?? {}), ...(body.role ?? {}), ...(body.project ?? {}), ...(body.clearance ?? {}) }
  ```

**Why:** The core repository uses a flat `extension` field internally (the polymorphic extension row is just a single object). The SDK models extensions as typed keys on the `Source` interface (`source.education`, `source.role`, etc.) for better TypeScript ergonomics. `mapExtension` bridges this gap at the HTTP boundary so neither the core nor the SDK must change their representation.

**Current state:**
- File: `packages/core/src/routes/sources.ts` (lines ~11-21 for mapExtension, lines ~27-35 for reverse spread)

---

### 8. List API Extension Enrichment

**What changed:**

- **Service:** `SourceService.listSources()` in `packages/core/src/services/source-service.ts` enriches each source from the list query with its extension data by calling `SourceRepo.getExtension()` per source:
  ```typescript
  const enriched = result.data.map(source => {
    const extension = SourceRepo.getExtension(this.db, source.id, source.source_type)
    return { ...source, extension }
  })
  ```
  This ensures the `GET /api/sources` list endpoint returns full extension data (role/project/education/clearance fields) with each source, not just the base `sources` columns.

**Why:** Before this change, listing sources returned only base fields. The UI needs extension data to show org names, education types, dates, etc. in the source list panel without making N+1 GET requests. The `mapExtension` helper (item 7) then converts these to SDK-shaped responses.

**Current state:**
- File: `packages/core/src/services/source-service.ts` (lines ~36-52)

---

### 9. Nav Restructuring

**What changed:**

- **UI:** `packages/webui/src/lib/nav.ts` reorganized the sidebar navigation into 5 groups:
  1. **Experience** (`/experience`): Roles, Projects, Education, Clearances, General -- per-type filtered views of SourcesView
  2. **Data** (`/data`): Bullets, Skills, Organizations, Domains, Notes -- entity management
  3. **Opportunities** (`/opportunities`): Organizations (kanban board), Job Descriptions
  4. **Resumes** (`/resumes`): Builder, Summaries, Templates
  5. **Config** (`/config`): Profile, Export, Debug (Logs)

**Why:** As the app grew from a simple source editor to a multi-entity system with kanban boards, resume builders, and opportunity tracking, the flat nav became unwieldy. The new structure groups by workflow concern: "Experience" is about the user's history, "Data" is about shared entities, "Opportunities" is about target companies/jobs, "Resumes" is about output generation.

Note: Organizations appear in two nav groups -- `/data/organizations` is the master org editor (CRUD, campuses, aliases, tags), and `/opportunities/organizations` is the kanban pipeline board. They share the same data but serve different purposes.

**Current state:**
- File: `packages/webui/src/lib/nav.ts`

---

## Part B: Remaining Cleanup / Polish (Forward)

### 1. Legacy Columns Not Removed

**What's missing:** `institution` and `issuing_body` on `source_education` are deprecated (marked `@deprecated` in types) but still present in the schema. `location` and `headquarters` on `organizations` are hidden in the UI form but still exist as schema columns and are still listed in the `Organization` interface.

**Why it matters:** Deprecated columns create confusion for future development. New code might accidentally read/write them instead of using the proper FK relationships. The `Organization` type exposes `location` and `headquarters` fields that should not be used — campuses replace them.

**Suggested approach:** Write table rebuild migrations (following the pattern from migration 012) that reconstruct `source_education` without `institution`/`issuing_body`, and `organizations` without `location`/`headquarters`. Must verify no active code reads these columns first. For `institution`/`issuing_body`, verify all education sources have a valid `organization_id` before dropping. For `location`/`headquarters`, verify the data has been migrated to campuses.

**Priority:** P2 -- not blocking but accumulates tech debt.

---

### 2. No Campus Editing

**What's missing:** The `campus-repository.ts` has `create()`, `listByOrg()`, `get()`, and `del()` but no `update()` method. The API has no `PATCH /campuses/:id` endpoint. The UI shows campuses as read-only cards with only a delete button -- there is no inline edit or edit modal.

**Why it matters:** If a campus address changes or the user makes a typo in the name, the only recourse is delete and re-create. This loses any `campus_id` references on `source_education` rows.

**Suggested approach:**
1. Add `update()` to `campus-repository.ts` (partial update, same pattern as org update)
2. Add `PATCH /campuses/:id` route in `campuses.ts`
3. Add inline edit to the campus card in the org editor -- click the campus name or a pencil icon to toggle edit mode on that card

**Priority:** P1 -- basic CRUD completeness.

**Done when:** `campus-repository.ts` has an `update()` method, `PATCH /campuses/:id` route exists and returns the updated campus, and the org editor UI allows inline editing of campus name/address/modality fields.

---

### 3. No Alias Search in Education Dropdown

**What's missing:** The API supports alias search via `?search=wgu` (the `list()` function in `organization-repository.ts` searches both `o.name LIKE ?` and `org_aliases.alias LIKE ?`). However, the education org dropdown in `SourcesView.svelte` is a plain `<select>` element. It uses the `eduFilteredOrgs` derived list but has no search/filter input. The user must scroll through all matching organizations to find the one they want.

**Why it matters:** With many organizations, scrolling through a `<select>` is slow. The whole point of aliases is to enable quick lookup by shorthand, but the dropdown does not expose search.

**Suggested approach:** Replace the `<select>` with a searchable combobox/autocomplete component. On keystroke, filter the `eduFilteredOrgs` list client-side (name match + alias match). Consider fetching aliases for all orgs at load time (they are lightweight strings) or making the combobox call the API with `?search=` on each keystroke (debounced). A shared `OrgCombobox.svelte` component could be reused across all org dropdowns (education, role, project, kanban picker).

**Priority:** P1 -- critical UX improvement, especially as the org count grows.

**Done when:** The education org dropdown is a searchable combobox that matches on both org name and aliases, and typing "wgu" surfaces "Western Governors University" in the filtered results.

---

### 4. Org Card in List Views Missing Detail

**What's missing:** The org cards in the left panel of `/data/organizations` show name, worked badge, tag pills, industry, and location. They do not show aliases, campus count, or HQ location.

**Why it matters:** When browsing the org list, the user cannot see at a glance whether an org has aliases (e.g., "WGU" next to "Western Governors University") or how many campuses are defined. This information would reduce the need to click into each org.

**Suggested approach:** Add a line under the tag pills showing alias pills (smaller/muted) and a campus count indicator (e.g., "3 campuses" or just an icon with count). Show HQ city/state if a campus is flagged `is_headquarters`.

**Priority:** P3 -- cosmetic enhancement, not blocking any workflow.

---

### 5. `location`/`headquarters` Still in Save Payload

**What's missing:** The org editor UI removed the `location` and `headquarters` form input fields (replaced by campuses section) and added a comment `<!-- Location and Headquarters are now managed via Campuses section below -->`. However, the `saveOrg()` function still includes `location: formLocation || undefined` and `headquarters: formHeadquarters || undefined` in the PATCH payload. Since `formLocation` and `formHeadquarters` are initialized from the existing org data via `populateForm()` but never modified by the user (no form fields exist), they will send stale values or empty strings.

**Why it matters:** Sends unnecessary/misleading data in the API payload. If a future migration removes the `location`/`headquarters` columns, the PATCH would silently fail or error depending on how the service handles unknown fields.

**Suggested approach:** Remove `location` and `headquarters` from the `saveOrg()` payload object. Remove the `formLocation` and `formHeadquarters` state variables and their initialization in `populateForm()` and `startNew()`.

**Priority:** P1 -- trivial fix, prevents confusion.

**Done when:** `saveOrg()` payload no longer includes `location` or `headquarters`, `formLocation` and `formHeadquarters` variables are removed, and saving an org does not send stale location data.

---

### 6. `org_type` vs Tags Redundancy

**What's missing:** The `org_type` column still exists on the `organizations` table alongside the `org_tags` junction table. The `org_type` column uses a CHECK constraint with values: company, nonprofit, government, military, education, volunteer, freelance, other. Tags use a different set: company, vendor, platform, university, school, nonprofit, government, military, conference, volunteer, freelance, other. The tags are authoritative (migration 011 comment: "tags are authoritative"), but `org_type` is still written on create (defaults to 'company') and can be set via the "Primary Type" dropdown in the UI.

**Why it matters:** Two sources of truth for the same concept. `org_type` is used as the seed for the initial tag on create (if no tags provided, `[org_type]` becomes the tag set). But `org_type='education'` maps to tag `university`, meaning the values are not even 1:1. Code that reads `org_type` vs `tags` will get different answers.

**Suggested approach:** Decision needed:
- **Option A (remove org_type):** Table rebuild migration to drop the column. Set the first tag as the "primary" type for display. Breaking change for any code that reads `org_type`.
- **Option B (keep as primary tag indicator):** Document that `org_type` is the "primary classification" and tags are "additional labels." Update the `org_type` CHECK to match the tag set. Update the UI to enforce that `org_type`'s value is always present in the tags array.
- **Recommendation:** Option A (remove) is cleaner long-term, but Option B is acceptable near-term. Document whichever is chosen.

**Priority:** P2 -- functional but confusing.

**Acceptance:** Either (a) `org_type` column is removed and all consumers read from `tags` array, or (b) `org_type` is documented as 'primary type' with tags as supplementary labels and both are kept in sync.

---

### 7. Campus in IR Compiler

**What's missing:** The compiler already JOINs `organizations` via `organization_id` for the org name. The gap is specifically the missing `campus_id` JOIN to `org_campuses` -- the rendered resume doesn't show campus city/state. `EducationItem` in core types also needs `campus_name`, `campus_city`, `campus_state` fields added. Resume rendering falls back to the deprecated `location` text column on `source_education`.

**Why it matters:** Structured campus data exists but is not used in the final resume output. A degree from WGU's "Online" campus or a role at a company's "Arlington, VA" office should render the campus location, not require the user to manually type it in a deprecated field.

**Suggested approach:**
1. Add `LEFT JOIN org_campuses oc ON oc.id = se.campus_id` to the education compiler query
2. Select `oc.city`, `oc.state`, `oc.name AS campus_name` (or a formatted string)
3. Extend `EducationItem` with `campus_name`, `campus_city`, `campus_state`
4. Update `renderEducationSection` in `sb2nov.ts` to use campus city/state for the location field, falling back to `se.location` for legacy data

**Priority:** P1 -- without this, campus data is informational only and does not flow to resumes.

**Done when:** The education compiler query JOINs `org_campuses`, `EducationItem` includes `campus_name`/`campus_city`/`campus_state` fields, and the rendered resume shows campus location for education entries that have a `campus_id` set.

---

### 8. Tag-Based Org Filtering in Opportunities Kanban

**What's missing:** The `OrgPickerModal` in the kanban board shows all orgs without a status. It has a search bar but no tag filter. Since the kanban is primarily for vetting prospective employers, it would benefit from filtering out irrelevant org types (e.g., universities, conferences).

**Why it matters:** As the org count grows, the picker list gets noisy with organizations the user would never add to an employer pipeline. Tag filtering (e.g., "show only companies") would reduce scrolling.

**Suggested approach:** Add a tag filter dropdown to `OrgPickerModal`, matching the pattern already used in the `/data/organizations` list panel. Default to "company" tag filter since the kanban is about employment opportunities.

**Priority:** P3 -- nice UX improvement but not critical.

---

### 9. Data Integrity: Stale Text Fields

**What's missing:** Some `source_education` rows may still have stale `institution` and `issuing_body` text values that do not match the linked org's `name`. This can happen if the org was renamed after migration 010 linked the FK. The deprecated text columns are not updated when the org name changes.

**Why it matters:** If any code or export reads the deprecated columns instead of joining through `organization_id`, it will show stale data.

**Suggested approach:**
- **Short-term:** Run a one-time data cleanup query that NULLs out `institution` and `issuing_body` on all rows where `organization_id IS NOT NULL`. This makes it clear that the FK is authoritative.
- **Long-term:** Drop the columns entirely (see item 1).

**Priority:** P2 -- data hygiene, not user-facing unless someone reads raw DB.

---

### 10. SDK Types for Campuses and Aliases

**What's missing:** `OrgCampus` and `OrgAlias` types are defined in `packages/sdk/src/types.ts` but are not exported from `packages/sdk/src/index.ts`. There are no SDK resource methods for campus or alias CRUD -- the UI calls `fetch()` directly against the API (e.g., `fetch('/api/organizations/${orgId}/campuses')`). This bypasses the SDK's error handling and type safety patterns.

**Why it matters:** Inconsistency with the rest of the SDK, which provides typed resource methods for organizations, sources, bullets, etc. If a second UI or CLI consumer is added, they would need to duplicate the raw fetch logic.

**Suggested approach:**
1. Export `OrgCampus` and `OrgAlias` from `packages/sdk/src/index.ts`
2. Add SDK resource methods: `forge.organizations.listCampuses(orgId)`, `forge.organizations.createCampus(orgId, input)`, `forge.organizations.deleteCampus(campusId)`, and the same for aliases
3. Update `SourcesView.svelte` and the org editor to use the SDK methods instead of raw `fetch()`

**Priority:** P1 -- build-time error: `OrgCampus`, `OrgAlias`, and `CampusModality` are defined in `packages/sdk/src/types.ts` but not exported from `packages/sdk/src/index.ts`. Any consumer importing these from `@forge/sdk` will fail to compile.

**Done when:** `OrgCampus`, `OrgAlias`, and `CampusModality` are exported from `packages/sdk/src/index.ts`, SDK resource methods exist for campus and alias CRUD, and at least one UI call site has been migrated from raw `fetch()` to the SDK method.

---

### 11. `OrganizationStatus` Type Stale in Core

**What's missing:** `packages/core/src/types/index.ts` has `OrganizationStatus` type with old values (`interested | review | targeting | excluded`). Must be updated to `backlog | researching | exciting | interested | acceptable | excluded` to match migration 012. This is a code fix, not a spec issue.

**Why it matters:** Any TypeScript code referencing `OrganizationStatus` will accept stale values (`review`, `targeting`) and reject valid values (`backlog`, `researching`, `exciting`, `acceptable`). The runtime SQL CHECK constraint catches invalid writes, but type safety is broken at compile time.

**Suggested approach:** Update the `OrganizationStatus` union type in `packages/core/src/types/index.ts` to `'backlog' | 'researching' | 'exciting' | 'interested' | 'acceptable' | 'excluded'`.

**Priority:** P1 -- type mismatch with migration 012.

**Done when:** `OrganizationStatus` type in `packages/core/src/types/index.ts` matches the 6 values in migration 012's CHECK constraint, and `tsc --noEmit` passes.

---

### 12. `CreateOrganizationInput.status` Typed as Raw `string`

**What's missing:** In `packages/core/src/db/repositories/organization-repository.ts`, the `status` field on `CreateOrganizationInput` is `string` not the kanban status union. The SQL CHECK constraint provides runtime safety but TypeScript type safety is absent.

**Why it matters:** Callers can pass any string as `status` without a compile-time error. Typos like `'intrested'` or `'backlogg'` would compile but fail at runtime with a CHECK constraint violation.

**Suggested approach:** Change `status` on `CreateOrganizationInput` (and `UpdateOrganizationInput`) from `string` to `OrganizationStatus`. Import the type from `packages/core/src/types/index.ts`.

**Priority:** P2 -- runtime safety exists via SQL CHECK, but TypeScript safety is absent.

---

### 13. `roleFilteredOrgs` Missing Project Source Type

**What's missing:** Currently `roleFilteredOrgs` only checks `source_type === 'role'` -- orgs linked exclusively via project sources will not appear in the project org dropdown. Fix: extend the filter to include `source_type === 'project'` matches as well.

**Why it matters:** If the user has a project source linked to an org but no role source for that same org, the org will not appear in the project dropdown. The user would need to also create a role source to make the org visible.

**Suggested approach:** Update the `roleFilteredOrgs` derived computation in `SourcesView.svelte` to check `source_type === 'role' || source_type === 'project'` when building the set of orgs to show.

**Priority:** P1 -- functional bug in project org dropdown.

**Done when:** Creating a project source and linking it to an org that has no role sources causes that org to appear in the project org dropdown.

---

### Parallel Tracks

**Parallel tracks:** Items 1+9 (legacy column removal + stale text cleanup) form one track. Item 10 (SDK exports) is independent. Items 3+8 (searchable dropdown) share an `OrgCombobox.svelte` component -- build it once for both education and kanban picker. Item 7 (campus in compiler) is independent. Item 5 (save payload cleanup) is a quick fix independent of all others.

### Testing Notes

Add test cases for alias search in `organization-repository.ts`: verify `list(db, { search: 'wgu' })` returns Western Governors University when 'wgu' is an alias.

---

## Part C: Architecture Notes

### Relationship Diagram

```
Organization
  |
  |-- 1:N -- org_tags (junction: organization_id, tag)
  |           Tags: company, vendor, platform, university, school,
  |                 nonprofit, government, military, conference,
  |                 volunteer, freelance, other
  |
  |-- 1:N -- org_campuses
  |           Columns: name, modality, address, city, state, zipcode,
  |                    country, is_headquarters
  |
  |-- 1:N -- org_aliases
  |           Columns: alias (COLLATE NOCASE)
  |
  |-- 1:N -- source_roles (via organization_id FK)
  |           Employee work history
  |
  |-- 1:N -- source_projects (via organization_id FK)
  |           Project work / contract history
  |
  |-- 1:N -- source_education (via organization_id FK)
                Education credentials

org_campuses
  |
  |-- 1:N -- source_education (via campus_id FK)
              Which campus the education was at
```

Key constraints:
- `org_tags`, `org_campuses`, `org_aliases` all CASCADE on organization delete
- `source_education.organization_id` SET NULL on organization delete
- `source_education.campus_id` SET NULL on campus delete
- `org_aliases.alias` has COLLATE NOCASE for case-insensitive matching
- `org_aliases` has UNIQUE(organization_id, alias) to prevent duplicate aliases per org

### Type Convention: SQLite INTEGER vs SDK boolean

`OrgCampus.is_headquarters` is `number` in core types (SQLite INTEGER) and `boolean` in SDK types. This follows the same convention as `Organization.worked` and `SourceEducation.is_in_progress`.

### Tag-Based Filtering Pattern

The UI uses `$derived` computations to filter the organization list by tags based on context:

**`eduFilteredOrgs`** -- filters the org dropdown in the education form based on `formEducationType` and `formCertificateSubtype`:

| Education Type | Certificate Subtype | Tags Shown |
|---------------|-------------------|------------|
| degree | -- | university, school |
| certificate | vendor | vendor, company |
| certificate | professional | nonprofit, government, company |
| certificate | completion | platform, company, university |
| course | -- | university, platform, conference, company |
| self_taught | -- | all orgs (no filter) |

**`roleFilteredOrgs`** -- for role and project dropdowns, currently only checks `source_type === 'role'` -- orgs linked exclusively via project sources will not appear in the project org dropdown. Fix: extend the filter to include `source_type === 'project'` matches as well. This ensures the dropdown shows "places I've worked or done projects for" rather than only role-linked orgs.

Both filters operate client-side on the full org list (fetched once with `limit: 500`). The API is not involved in the filtering -- the UI loads all orgs and derives filtered subsets reactively.

**New org creation context:** When the user clicks "+" to create a new org from the education form, the `openOrgModal()` function infers default `org_type` and tags from the current education context (e.g., creating from a degree form defaults to `org_type='education'` with tags `['university']`).

### The `mapExtension` Pattern

The core stores polymorphic extension data as a flat `extension` field on `SourceWithExtension`:
```typescript
interface SourceWithExtension extends Source {
  extension: SourceRole | SourceProject | SourceEducation | SourceClearance | null
}
```

The SDK expects extension data under typed keys:
```typescript
interface Source {
  role?: SourceRole
  project?: SourceProject
  education?: SourceEducation
  clearance?: SourceClearance
}
```

`mapExtension()` at the HTTP boundary converts outbound responses:
```typescript
function mapExtension(source: SourceWithExtension): Record<string, unknown> {
  const { extension, ...rest } = source
  if (!extension) return rest
  const key = source.source_type  // 'role' | 'project' | 'education' | 'clearance'
  return { ...rest, [key]: extension }
}
```

On inbound requests, the route handler spreads the typed keys back to flat:
```typescript
const input = { ...body, ...(body.education ?? {}), ...(body.role ?? {}), ...(body.project ?? {}), ...(body.clearance ?? {}) }
```

This pattern keeps the core simple (single `extension` field) while giving SDK consumers type-safe access via discriminated keys.

### Kanban Status Model

The organization kanban models a vetting pipeline with 4 visual columns mapping to 6 status values:

```
  [Backlog]          [Researching]          [Targeting]              [Excluded]
  status: backlog    status: researching    status: exciting         status: excluded
                                            status: interested       (collapsed by default)
                                            status: acceptable
```

**Flow:**
1. Org enters pipeline via OrgPickerModal -- status set to `backlog`
2. User drags to Researching -- status set to `researching`
3. User drags to Targeting -- status set to `interested` (default interest level)
4. User adjusts interest level via OrgDetailModal dropdown (exciting/interested/acceptable)
5. User drags to Excluded if red flags found -- status set to `excluded`
6. "Remove from Pipeline" sets status to `null` (org leaves the board but is not deleted)

**Interest levels within Targeting (color-coded cards):**
- **Exciting** (green bg, green border): Top-tier target
- **Interested** (blue bg, blue border): Strong candidate
- **Acceptable** (gray bg, gray border): Would consider

**Status transitions are unrestricted** -- any column can move to any other column. The kanban is a visual organizer, not a workflow engine.

**Data loading:** The board fetches all orgs (`limit: 500`) and filters to those with non-null status. Orgs without status are not on the board but are available in the OrgPickerModal.

### Decision Log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-04-03 | FK over text fields for education institutions | Normalizes org references; enables cross-entity queries; eliminates spelling inconsistencies |
| 2026-04-03 | Multi-label tags over single org_type enum | An org can be both a company and a vendor; tags enable context-sensitive dropdown filtering |
| 2026-04-03 | Keep org_type alongside tags (for now) | Backward compatibility; used as default tag seed on create; removal deferred to avoid breaking changes |
| 2026-04-03 | Separate campuses table over location text field | Structured location enables modality tracking, HQ identification, zipcode-based search, and multi-location orgs |
| 2026-04-03 | Aliases with COLLATE NOCASE | Case-insensitive matching is essential for abbreviations (WGU, wgu, Wgu all match) |
| 2026-04-03 | Alias search integrated into org list() not a separate endpoint | Search is the primary use case for aliases; integrating into list() means all org queries benefit |
| 2026-04-03 | Zipcode on campus (014) as follow-up migration | Initial campus design missed zipcode; added in a subsequent migration rather than rebuilding 013 |
| 2026-04-03 | Campus/alias routes share one file (campuses.ts) | Both are organization sub-resources; keeping them together reduces file count and groups related endpoints |
| 2026-04-03 | mapExtension at HTTP boundary, not in repository | The core's flat extension model is simpler for internal use; translation to SDK shape is a presentation concern |
| 2026-04-03 | List API enriches with extension data per-source | Enables the UI to show extension fields in list views without N+1 requests; acceptable perf for <500 sources |
| 2026-04-03 | 6 kanban statuses across 4 columns | Targeting column aggregates 3 interest levels; avoids 6 narrow columns while preserving fine-grained status |
| 2026-04-03 | No SDK methods for campus/alias CRUD (deferred) | UI was the only consumer; raw fetch was faster to implement; SDK methods are a polish item |
| 2026-04-03 | Deprecated columns kept, not dropped | SQLite cannot drop columns conditionally; table rebuild migrations are heavyweight; deferred until the columns cause an actual problem |
| 2026-04-03 | `education_organization_id` naming in flat source types | The flat `CreateSource`/`UpdateSource` types use `education_organization_id` (not `organization_id`) to avoid collision with the `organization_id` field used by role and project extension types. The source route handler spreads nested SDK objects into flat fields |
