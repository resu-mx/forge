# Qualifications �� Credentials & Certifications

**Date:** 2026-04-05
**Status:** Design
**Builds on:** User Profile (Migration 005), Organizations (Migration 002), Skills (Migration 002), Resume Sections (Migration 004), Clearance Structured Data (Migration 019), Sources (Migration 002)
**Related specs:**
- [Config Profile](./2026-03-30-config-profile.md)
- [Org Model Evolution](./2026-04-03-org-model-evolution.md)

## Purpose

Security clearance is currently modeled as a Source type (`source_type = 'clearance'`) with a `source_clearances` extension table. This is architecturally wrong — clearance is a boolean qualifier ("do you have it?"), not a source of narrative content. No bullets derive from a clearance. It doesn't participate in the Source → Bullet → Perspective derivation chain.

The same is true for other credentials (driver's licenses, bar admissions, medical licenses) and for certifications (PMP, CISSP, AWS SA Pro). These are qualifications — things a JD requires or prefers — not things you write accomplishment statements about.

This spec introduces two new entities under a "Qualifications" sidebar group:

1. **Credentials** — boolean-gate qualifiers (clearance, licenses, admissions). Single list page with type-specific forms. No skill linkage, no derivation chain. JD matching treats them as pass/fail gates.
2. **Certifications** — earned validations of skill (PMP, CISSP, AWS SA Pro). List page with skill linkage. Certs corroborate skills but don't produce bullets — the training/role that earned the cert is the bullet source.

## Goals

1. `credentials` table with polymorphic `details` JSON for type-specific fields
2. `certifications` table with metadata and skill junction
3. Remove `source_type = 'clearance'` from the source type enum
4. Remove `source_clearances` extension table (migrate data → credentials)
5. Remove `user_profile.clearance` text field (clearance managed from Qualifications page only)
6. New "Qualifications" sidebar group with Credentials and Certifications pages
7. Resume IR compiler pulls credentials and certs directly (no derivation chain)
8. JD matching uses credentials as boolean gates, certs as skill corroboration

## Non-Goals

- Multi-user credential verification / attestation
- Credential expiry alerting or renewal tracking
- Certification exam scheduling or study tracking
- Linking credentials to skills (deferred — credentials are boolean gates for now)
- Automated cert validation against issuer APIs
- Cover letter integration with qualifications
- Bulk import of certifications from LinkedIn or similar

---

## 1. Schema Changes

### 1.1 `credentials` Table

```sql
-- Forge Resume Builder — Credentials Entity
-- Migration: 030_credentials
-- Date: 2026-04-05
--
-- Creates a credentials table for boolean-qualifier credentials
-- (clearance, licenses, admissions). Type-specific data stored in
-- a JSON details column since each type has different fields and
-- there are very few rows per user.
-- Builds on 029_prompt_logs_jd_entity_type.

CREATE TABLE credentials (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  credential_type TEXT NOT NULL CHECK (credential_type IN (
    'clearance', 'drivers_license', 'bar_admission', 'medical_license'
  )),
  label TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'active' CHECK (status IN (
    'active', 'inactive', 'expired'
  )),
  organization_id TEXT REFERENCES organizations(id) ON DELETE SET NULL,
  details TEXT NOT NULL DEFAULT '{}',  -- JSON, type-specific structured fields
  issued_date TEXT,    -- ISO 8601 date, nullable
  expiry_date TEXT,    -- ISO 8601 date, nullable
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

CREATE INDEX idx_credentials_type ON credentials(credential_type);
CREATE INDEX idx_credentials_org ON credentials(organization_id);
```

**Type-specific `details` JSON schemas:**

**Clearance:**
```json
{
  "level": "top_secret",
  "polygraph": "full_scope",
  "clearance_type": "personnel",
  "access_programs": ["sci", "sap"]
}
```

Fields:
- `level`: `'public' | 'confidential' | 'secret' | 'top_secret' | 'q' | 'l'`
- `polygraph`: `'none' | 'ci' | 'full_scope'` (nullable)
- `clearance_type`: `'personnel' | 'facility'`
- `access_programs`: array of `'sci' | 'sap' | 'nato'`

Note: `organization_id` on the parent row serves as the sponsor org (replaces `source_clearances.sponsor_organization_id`).

**Driver's License:**
```json
{
  "class": "C",
  "state": "VA",
  "endorsements": ["motorcycle", "hazmat"]
}
```

**Bar Admission:**
```json
{
  "jurisdiction": "Virginia",
  "bar_number": "12345"
}
```

**Medical License:**
```json
{
  "license_type": "MD",
  "state": "VA",
  "license_number": "ML-12345"
}
```

### 1.2 `certifications` Table

```sql
CREATE TABLE certifications (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  name TEXT NOT NULL,
  issuer TEXT,
  date_earned TEXT,     -- ISO 8601 date
  expiry_date TEXT,     -- ISO 8601 date, nullable
  credential_id TEXT,   -- issuer's credential ID string
  credential_url TEXT,  -- verification URL
  education_source_id TEXT REFERENCES sources(id) ON DELETE SET NULL,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

CREATE INDEX idx_certifications_source ON certifications(education_source_id);
```

### 1.3 `certification_skills` Junction Table

```sql
CREATE TABLE certification_skills (
  certification_id TEXT NOT NULL
    REFERENCES certifications(id) ON DELETE CASCADE,
  skill_id TEXT NOT NULL
    REFERENCES skills(id) ON DELETE CASCADE,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  PRIMARY KEY (certification_id, skill_id)
) STRICT;
```

### 1.4 Data Migration: `source_clearances` → `credentials`

```sql
-- Migrate existing clearance data from source_clearances to credentials.
-- Each source_clearances row becomes a credentials row with credential_type='clearance'.
INSERT INTO credentials (id, credential_type, label, status, organization_id, details, issued_date, created_at, updated_at)
SELECT
  lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' ||
    substr(hex(randomblob(2)),2) || '-' ||
    substr('89ab', abs(random()) % 4 + 1, 1) ||
    substr(hex(randomblob(2)),2) || '-' || hex(randomblob(6))),
  'clearance',
  CASE sc.level
    WHEN 'top_secret' THEN 'Top Secret'
    WHEN 'secret' THEN 'Secret'
    WHEN 'confidential' THEN 'Confidential'
    WHEN 'public' THEN 'Public Trust'
    WHEN 'q' THEN 'DOE Q'
    WHEN 'l' THEN 'DOE L'
    ELSE sc.level
  END,
  sc.status,
  sc.sponsor_organization_id,
  json_object(
    'level', sc.level,
    'polygraph', sc.polygraph,
    'clearance_type', sc.clearance_type,
    'access_programs', (
      SELECT json_group_array(cap.program)
      FROM clearance_access_programs cap
      WHERE cap.source_clearance_id = sc.source_id
    )
  ),
  s.start_date,
  s.created_at,
  s.updated_at
FROM source_clearances sc
JOIN sources s ON s.id = sc.source_id;
```

### 1.5 Schema Cleanup

After data migration, remove the clearance source infrastructure:

```sql
-- Drop clearance access programs junction (no longer needed)
DROP TABLE IF EXISTS clearance_access_programs;

-- Drop source_clearances extension table
DROP TABLE IF EXISTS source_clearances;

-- Remove clearance from user_profile
-- (Requires table rebuild since SQLite can't drop columns with CHECK constraints)
-- Rebuild user_profile WITHOUT the clearance column.

-- Remove source_type='clearance' from sources CHECK constraint
-- (Requires table rebuild — see migration pattern from 024_unified_kanban_statuses.sql)
```

Note: The table rebuilds for `user_profile` and `sources` follow the established pattern from migration 024. The sources CHECK constraint becomes `CHECK (source_type IN ('role', 'project', 'education', 'general'))`.

### 1.6 Update `note_references` Entity Type CHECK

Rebuild `note_references` to add `'credential'` and `'certification'` to the `entity_type` CHECK constraint (same pattern as migration 007 which added `'job_description'`).

---

## 2. Type Definitions

### 2.1 Core Types (`packages/core/src/types/index.ts`)

```typescript
/** Valid credential type discriminator values. */
export type CredentialType = 'clearance' | 'drivers_license' | 'bar_admission' | 'medical_license'

/** Valid credential status values. */
export type CredentialStatus = 'active' | 'inactive' | 'expired'

/** Clearance-specific details stored in credentials.details JSON. */
export interface ClearanceDetails {
  level: ClearanceLevel
  polygraph: ClearancePolygraph | null
  clearance_type: ClearanceType
  access_programs: ClearanceAccessProgram[]
}

/** Driver's license details stored in credentials.details JSON. */
export interface DriversLicenseDetails {
  class: string
  state: string
  endorsements: string[]
}

/** Bar admission details stored in credentials.details JSON. */
export interface BarAdmissionDetails {
  jurisdiction: string
  bar_number: string | null
}

/** Medical license details stored in credentials.details JSON. */
export interface MedicalLicenseDetails {
  license_type: string
  state: string
  license_number: string | null
}

/** Union of all credential detail types. */
export type CredentialDetails =
  | ClearanceDetails
  | DriversLicenseDetails
  | BarAdmissionDetails
  | MedicalLicenseDetails

/** A credential entity (clearance, license, admission). */
export interface Credential {
  id: string
  credential_type: CredentialType
  label: string
  status: CredentialStatus
  organization_id: string | null
  details: CredentialDetails
  issued_date: string | null
  expiry_date: string | null
  created_at: string
  updated_at: string
}

/** Input for creating a credential. */
export interface CreateCredential {
  credential_type: CredentialType
  label: string
  status?: CredentialStatus
  organization_id?: string
  details: CredentialDetails
  issued_date?: string
  expiry_date?: string
}

/** Input for updating a credential. */
export interface UpdateCredential {
  label?: string
  status?: CredentialStatus
  organization_id?: string | null
  details?: Partial<CredentialDetails>
  issued_date?: string | null
  expiry_date?: string | null
}

/** A certification entity. */
export interface Certification {
  id: string
  name: string
  issuer: string | null
  date_earned: string | null
  expiry_date: string | null
  credential_id: string | null
  credential_url: string | null
  education_source_id: string | null
  created_at: string
  updated_at: string
}

/** Input for creating a certification. */
export interface CreateCertification {
  name: string
  issuer?: string
  date_earned?: string
  expiry_date?: string
  credential_id?: string
  credential_url?: string
  education_source_id?: string
}

/** Input for updating a certification. */
export interface UpdateCertification {
  name?: string
  issuer?: string | null
  date_earned?: string | null
  expiry_date?: string | null
  credential_id?: string | null
  credential_url?: string | null
  education_source_id?: string | null
}

/** A certification with its linked skills. */
export interface CertificationWithSkills extends Certification {
  skills: Skill[]
}
```

### 2.2 Source Type Update

```typescript
// Before:
export type SourceType = 'role' | 'project' | 'education' | 'clearance' | 'general'

// After:
export type SourceType = 'role' | 'project' | 'education' | 'general'
```

### 2.3 Remove Clearance Source Types

Remove these types (no longer needed — replaced by `ClearanceDetails`):
- `SourceClearance` interface
- Remove `'clearance'` from `SourceType` union
- Remove clearance extension fields from `CreateSource` and `UpdateSource`
- Remove `SourceClearance` from the `Source.extension` union

### 2.4 Remove Profile Clearance

Remove `clearance: string | null` from the `UserProfile` interface.

---

## 3. Repository Layer

### 3.1 `credential-repository.ts`

**File:** `packages/core/src/db/repositories/credential-repository.ts`

Standard CRUD repository:
- `create(input: CreateCredential): Promise<Credential>`
- `findById(id: string): Promise<Credential | null>`
- `findAll(): Promise<Credential[]>`
- `findByType(type: CredentialType): Promise<Credential[]>`
- `update(id: string, input: UpdateCredential): Promise<Credential>`
- `delete(id: string): Promise<void>`

`details` column: serialize with `JSON.stringify()` on write, `JSON.parse()` on read.

### 3.2 `certification-repository.ts`

**File:** `packages/core/src/db/repositories/certification-repository.ts`

Standard CRUD repository:
- `create(input: CreateCertification): Promise<Certification>`
- `findById(id: string): Promise<Certification | null>`
- `findAll(): Promise<Certification[]>`
- `findByIdWithSkills(id: string): Promise<CertificationWithSkills | null>`
- `findAllWithSkills(): Promise<CertificationWithSkills[]>`
- `update(id: string, input: UpdateCertification): Promise<Certification>`
- `delete(id: string): Promise<void>`
- `addSkill(certId: string, skillId: string): Promise<void>`
- `removeSkill(certId: string, skillId: string): Promise<void>`
- `getSkills(certId: string): Promise<Skill[]>`

---

## 4. Service Layer

### 4.1 `credential-service.ts`

**File:** `packages/core/src/services/credential-service.ts`

Thin service wrapping the repository. Validates `credential_type`-specific `details` fields on create/update (e.g., clearance must have `level`, driver's license must have `class` and `state`).

### 4.2 `certification-service.ts`

**File:** `packages/core/src/services/certification-service.ts`

Thin service wrapping the repository. Validates `education_source_id` references an actual education source (if provided). Manages skill linkage.

---

## 5. HTTP Routes

### 5.1 Credentials Routes

**File:** `packages/core/src/routes/credentials.ts`

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/credentials` | List all credentials |
| GET | `/api/credentials/:id` | Get credential by ID |
| POST | `/api/credentials` | Create credential |
| PATCH | `/api/credentials/:id` | Update credential |
| DELETE | `/api/credentials/:id` | Delete credential |

### 5.2 Certifications Routes

**File:** `packages/core/src/routes/certifications.ts`

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/certifications` | List all certifications (with skills) |
| GET | `/api/certifications/:id` | Get certification by ID (with skills) |
| POST | `/api/certifications` | Create certification |
| PATCH | `/api/certifications/:id` | Update certification |
| DELETE | `/api/certifications/:id` | Delete certification |
| POST | `/api/certifications/:id/skills` | Add skill to certification |
| DELETE | `/api/certifications/:id/skills/:skillId` | Remove skill from certification |

---

## 6. SDK Resources

### 6.1 `CredentialResource`

**File:** `packages/sdk/src/resources/credentials.ts`

Methods mirror the HTTP routes. Returns `Result<Credential>` / `Result<Credential[]>`.

### 6.2 `CertificationResource`

**File:** `packages/sdk/src/resources/certifications.ts`

Methods mirror the HTTP routes. `list()` and `get()` return `CertificationWithSkills`. Skill management via `addSkill()` / `removeSkill()`.

---

## 7. WebUI

### 7.1 Navigation Update

**File:** `packages/webui/src/lib/nav.ts`

```typescript
// Remove from Experience group:
{ href: '/experience/clearances', label: 'Clearances' },

// Add new group:
{
  label: 'Qualifications',
  prefix: '/qualifications',
  children: [
    { href: '/qualifications/credentials', label: 'Credentials' },
    { href: '/qualifications/certifications', label: 'Certifications' },
  ],
},
```

Position: after Experience, before Data.

### 7.2 Credentials Page

**Route:** `/qualifications/credentials/+page.svelte`

Split-panel layout (using `SplitPanel` component):
- **Left panel:** List of all credentials grouped by type. Each shows label, type badge, status badge.
- **Right panel:** Type-specific form when a credential is selected.

**Clearance form fields:**
- Label (text, e.g., "Top Secret / SCI")
- Level (dropdown: public, confidential, secret, top_secret, q, l)
- Polygraph (dropdown: none, ci, full_scope)
- Type (dropdown: personnel, facility)
- Access Programs (multi-select: sci, sap, nato)
- Sponsor Organization (OrgCombobox)
- Status (dropdown: active, inactive, expired)

**Driver's License form fields:**
- Label (text, e.g., "Virginia CDL Class A")
- Class (text)
- State (text)
- Endorsements (multi-input or comma-separated)
- Issuing Organization (OrgCombobox, optional)
- Status (dropdown)

**Bar Admission form fields:**
- Label (text, e.g., "Virginia State Bar")
- Jurisdiction (text)
- Bar Number (text, optional)
- Organization (OrgCombobox, optional)
- Issued Date
- Status (dropdown)

**Medical License form fields:**
- Label (text, e.g., "Virginia MD License")
- License Type (text)
- State (text)
- License Number (text, optional)
- Organization (OrgCombobox, optional)
- Issued Date, Expiry Date
- Status (dropdown)

### 7.3 Certifications Page

**Route:** `/qualifications/certifications/+page.svelte`

Split-panel layout:
- **Left panel:** List of certifications. Each shows name, issuer, status (active/expired based on expiry_date).
- **Right panel:** Certification detail form + skill tags.

**Form fields:**
- Name (text, e.g., "AWS Solutions Architect Professional")
- Issuer (text, e.g., "Amazon Web Services")
- Date Earned (date picker)
- Expiry Date (date picker, optional)
- Credential ID (text, optional)
- Credential URL (text, optional)
- Linked Education Source (dropdown of education-type sources, optional)
- Skills (tag selector, similar to JD skill tagging)

### 7.4 Remove Clearances Source Page

**Delete:** `/experience/clearances/` route (or redirect to `/qualifications/credentials`)

**Update:** Sources views should no longer show `source_type = 'clearance'` as an option in type selectors or filters.

---

## 8. Resume IR Compiler Updates

### 8.1 Clearance Section

The IR compiler currently reads clearance data from sources with `source_type = 'clearance'`. Change to read from `credentials` table where `credential_type = 'clearance'`.

The `ClearanceItem` IR type stays the same — it renders as a flat line item. The data source changes from the derivation chain to a direct credentials query.

```typescript
// Before (in compiler):
const clearanceSources = sources.filter(s => s.source_type === 'clearance')

// After:
const clearanceCredentials = await services.credentials.findByType('clearance')
```

### 8.2 Certifications Section

Add compiler support for a certifications resume section. Pull from `certifications` table. Render as flat line items (name, issuer, date).

```typescript
const certs = await services.certifications.findAll()
// Render: "AWS Solutions Architect Professional — Amazon Web Services (2024)"
```

### 8.3 `ResumeSection` Type

The existing `ResumeSection = 'clearance' | 'certifications' | ...` values remain. The compiler just reads from different tables now.

---

## 9. JD Matching Updates

### 9.1 Credential Gate Matching

When matching a JD against a user's qualifications, treat credentials as boolean gates:

- JD mentions "TS/SCI required" → check if user has a credential with `credential_type = 'clearance'` and `details.level = 'top_secret'` and `details.access_programs` includes `'sci'`
- JD mentions "must have valid driver's license" → check if user has `credential_type = 'drivers_license'` with `status = 'active'`

This is a pass/fail check, separate from the embedding-based skill alignment scoring.

### 9.2 Certification Skill Corroboration

When computing alignment scores, certifications boost matched skills:

- JD requires "network security" → embedding similarity finds matching bullets
- User has CISSP linked to "network security" skill → boost the confidence of that skill match

Implementation deferred to a future phase — the data model supports it via `certification_skills`, but the alignment algorithm change is out of scope for this spec.

---

## 10. MCP Server Updates

### 10.1 New Tools

Add MCP tools for credential and certification CRUD:

**Credentials:**
- `forge_search_credentials` — list/filter credentials
- `forge_create_credential` — create a credential
- `forge_update_credential` — update a credential

**Certifications:**
- `forge_search_certifications` — list/filter certifications
- `forge_create_certification` — create a certification
- `forge_update_certification` — update a certification
- `forge_add_certification_skill` — link a skill to a certification
- `forge_remove_certification_skill` — unlink a skill from a certification

### 10.2 New Resources

- `forge://credentials` — list of all credentials
- `forge://certifications` — list of all certifications with skills

---

## 11. Existing Source Clearance Data Handling

### 11.1 Orphaned Sources

After migrating `source_clearances` data to `credentials`, the parent `sources` rows with `source_type = 'clearance'` become orphaned. These sources likely have no bullets (since clearances don't produce bullets). The migration should:

1. Check for any bullets referencing clearance sources (via `bullet_sources` junction)
2. If bullets exist: log a warning — these are unusual and may need manual review
3. Delete clearance source rows (cascade will clean up `bullet_sources` if empty)

### 11.2 Existing Resume Entries

If any resumes have entries pointing to clearance sources, those entries should be preserved but the resume section should switch to reading from the new `credentials` table. The `entry_type = 'clearance'` on `resume_sections` continues to work — only the data source changes.

---

## Migration Sequence

| Step | Action |
|------|--------|
| 1 | Create `credentials` table |
| 2 | Create `certifications` table |
| 3 | Create `certification_skills` junction table |
| 4 | Migrate `source_clearances` + `clearance_access_programs` → `credentials` |
| 5 | Drop `clearance_access_programs` table |
| 6 | Drop `source_clearances` table |
| 7 | Rebuild `sources` table — remove `'clearance'` from `source_type` CHECK |
| 8 | Rebuild `user_profile` — remove `clearance` column |
| 9 | Rebuild `note_references` — add `'credential'`, `'certification'` to entity_type CHECK |
| 10 | Delete orphaned clearance source rows |
