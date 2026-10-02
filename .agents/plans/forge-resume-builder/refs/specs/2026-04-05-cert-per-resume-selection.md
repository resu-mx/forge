# Per-Resume Certification Selection + Schema Rework — Design Spec

**Date:** 2026-04-05
**Beads:** `job-hunting-89l` (Per-resume certification selection)
**Status:** Approved, ready for implementation plan

## Summary

Rework the `certifications` table schema to add `short_name`/`long_name` split, `cert_id` (exam version), `issuer_id` org FK (replacing free-text `issuer`), and `credly_url`. Add a `resume_certifications` junction table for per-resume cert selection with position ordering. Rewrite the compiler's `buildCertificationItems` to read from the junction instead of auto-including all certs. Update the resume editor with a cert picker and per-entry remove buttons.

## Goals

1. Per-resume cert selection: each resume chooses which certs to include via a junction table. No more auto-include-all.
2. Schema improvements: `short_name`/`long_name` split, `cert_id`, org-linked `issuer_id`, `credly_url`.
3. Resume display uses `short_name` only (compact).
4. Issuer links to organizations (select/create pattern).

## Non-goals

- Tag/keyword-based cert filtering (YAGNI for 19 certs).
- Cert ordering within issuer groups by date (position-based ordering is sufficient).
- Credly badge rendering/preview in the UI (just store the URL).
- Migration of `education_source_id` data (UI already removed, column dropped in this migration).

## Schema

### Migration (one file, table rebuild + new junction)

**Rebuild `certifications`:**

```sql
-- PRAGMA foreign_keys = OFF

CREATE TABLE certifications_new (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  short_name TEXT NOT NULL,
  long_name TEXT NOT NULL,
  cert_id TEXT,
  issuer_id TEXT REFERENCES organizations(id) ON DELETE SET NULL,
  date_earned TEXT,
  expiry_date TEXT,
  credential_id TEXT,
  credential_url TEXT,
  credly_url TEXT,
  in_progress INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

-- Migrate data: name → short_name AND long_name
-- issuer (free text) → best-effort org lookup → issuer_id
INSERT INTO certifications_new (
  id, short_name, long_name, cert_id, issuer_id,
  date_earned, expiry_date, credential_id, credential_url, credly_url,
  in_progress, created_at, updated_at
)
SELECT
  c.id,
  c.name,                    -- current name becomes short_name
  c.name,                    -- also becomes long_name (user edits later)
  NULL,                       -- cert_id not previously tracked
  (SELECT o.id FROM organizations o WHERE o.name = c.issuer LIMIT 1),
  c.date_earned,
  c.expiry_date,
  c.credential_id,
  c.credential_url,
  NULL,                       -- credly_url not previously tracked
  0,                          -- in_progress defaults to false
  c.created_at,
  c.updated_at
FROM certifications c;

DROP TABLE certifications;
ALTER TABLE certifications_new RENAME TO certifications;

CREATE INDEX idx_certifications_issuer ON certifications(issuer_id);
```

**New junction table:**

```sql
CREATE TABLE resume_certifications (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  resume_id TEXT NOT NULL REFERENCES resumes(id) ON DELETE CASCADE,
  certification_id TEXT NOT NULL REFERENCES certifications(id) ON DELETE CASCADE,
  section_id TEXT NOT NULL REFERENCES resume_sections(id) ON DELETE CASCADE,
  position INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  UNIQUE(resume_id, certification_id)
) STRICT;

CREATE INDEX idx_resume_certs_resume ON resume_certifications(resume_id);
CREATE INDEX idx_resume_certs_section ON resume_certifications(section_id);
```

### Field mapping

| Field | Old | New | Notes |
|---|---|---|---|
| Display name | `name` | `short_name` | "Security+", "GCFA" |
| Full name | (same as name) | `long_name` | "CompTIA Security+" |
| Exam version | (not tracked) | `cert_id` | "SY0-701", nullable |
| Issuer | `issuer TEXT` | `issuer_id TEXT FK` | Org link, not free text |
| Credly | (not tracked) | `credly_url` | Badge link |
| In progress | (not tracked) | `in_progress` | 0/1 flag for certs being pursued |
| Education link | `education_source_id` | (dropped) | Removed |

## Types

### Core (`packages/core/src/types/index.ts`)

```ts
export interface Certification {
  id: string
  short_name: string
  long_name: string
  cert_id: string | null
  issuer_id: string | null
  date_earned: string | null
  expiry_date: string | null
  credential_id: string | null
  credential_url: string | null
  credly_url: string | null
  in_progress: boolean
  created_at: string
  updated_at: string
}

export interface CreateCertification {
  short_name: string
  long_name: string
  cert_id?: string
  issuer_id?: string
  date_earned?: string
  expiry_date?: string
  credential_id?: string
  credential_url?: string
  credly_url?: string
  in_progress?: boolean
}

export interface UpdateCertification {
  short_name?: string
  long_name?: string
  cert_id?: string | null
  issuer_id?: string | null
  date_earned?: string | null
  expiry_date?: string | null
  credential_id?: string | null
  credential_url?: string | null
  credly_url?: string | null
  in_progress?: boolean
}

export interface ResumeCertification {
  id: string
  resume_id: string
  certification_id: string
  section_id: string
  position: number
  created_at: string
}

export interface AddResumeCertification {
  certification_id: string
  section_id: string
  position?: number  // defaults to next-available, same pattern as AddResumeEntry
}
```

### IR type (`CertificationGroup`)

The existing `CertificationGroup` shape stays the same (categories → label + certs array). The cert `name` field now comes from `short_name`. The `entry_id` maps to `resume_certifications.id` (for per-entry delete). The `source_id` is no longer used (certs aren't sources) — set to `null`.

### SDK mirror

All types mirrored in `packages/sdk/src/types.ts`.

## Compiler

`buildCertificationItems(db, sectionId)` rewritten:

```ts
function buildCertificationItems(db: Database, sectionId: string): CertificationGroup[] {
  const rows = db.query(`
    SELECT rc.id AS entry_id, c.short_name, c.cert_id,
           o.name AS issuer_name, c.date_earned
    FROM resume_certifications rc
    JOIN certifications c ON c.id = rc.certification_id
    LEFT JOIN organizations o ON o.id = c.issuer_id
    WHERE rc.section_id = ?
    ORDER BY rc.position ASC
  `).all(sectionId) as Array<{
    entry_id: string
    short_name: string
    cert_id: string | null
    issuer_name: string | null
    date_earned: string | null
  }>

  if (rows.length === 0) return []

  const catMap = new Map<string, Array<{ name: string; entry_id: string | null; source_id: string | null }>>()

  for (const row of rows) {
    const label = row.issuer_name ?? 'Other'
    if (!catMap.has(label)) catMap.set(label, [])
    catMap.get(label)!.push({
      name: row.short_name,
      entry_id: row.entry_id,
      source_id: null,
    })
  }

  return [{
    kind: 'certification_group',
    categories: Array.from(catMap.entries()).map(([label, certs]) => ({ label, certs })),
  }]
}
```

Key changes from Phase 88's version:
- Reads from `resume_certifications` junction (per-resume), NOT from global `certifications` table
- Uses `sectionId` parameter (no longer ignored with `_` prefix)
- Display name from `c.short_name`
- Groups by org name via `issuer_id` FK join

## API

### Existing certification CRUD (updated fields)

`POST/GET/PATCH/DELETE /api/certifications` — update to use new column names. The service/repository need the new fields.

### New resume-certification endpoints

```
POST   /api/resumes/:id/certifications    — { certification_id, section_id, position? }
DELETE /api/resumes/:id/certifications/:rcId  — remove from junction
GET    /api/resumes/:id/certifications    — list for this resume
```

Follow the `resume_skills` pattern (similar junction CRUD already exists at `/api/resumes/:id/skills`).

### Resume repository additions

```ts
addCertification(db, resumeId, input: AddResumeCertification): ResumeCertification
removeCertification(db, resumeId, rcId: string): boolean
listCertifications(db, resumeId): ResumeCertification[]
```

Position defaulting uses the same `COALESCE(MAX(position), -1) + 1` pattern as `addEntry`.

## WebUI

### `/qualifications/certifications` page — form update

Replace current fields with:
- **Short Name** (text, required) — "Security+"
- **Long Name** (text, required) — "CompTIA Security+"
- **Cert ID** (text, nullable) — "SY0-701"
- **Issuer** (OrgCombobox — select existing org or create new)
- **Date Earned** (date)
- **Expiry Date** (date)
- **Credential ID** (text) — personal verification ID
- **Credential URL** (url)
- **Credly URL** (url)
- **In Progress** (checkbox) — for certs being actively pursued

The card/list display shows `short_name` with `issuer` org name.

### Resume editor Certifications section

**"+Add Entry" button:** opens a `CertPickerModal` listing all certifications not already on this resume. Rows show `short_name` + issuer. Click adds to the junction.

**Each cert entry:** shows `short_name` only, grouped by issuer. Has "×" remove button (deletes from junction via `DELETE /api/resumes/:id/certifications/:rcId`).

**DragNDropView template:** the existing `{:else if section.type === 'certifications'}` block renders from the IR's `CertificationGroup` — no template change needed since the IR shape is unchanged. The `entry_id` on each cert maps to `resume_certifications.id` so the existing `onRemoveEntry` callback works.

**Picker routing (`openPicker`):** the `case 'certifications'` opens the cert picker modal instead of SourcePicker.

### `CertPickerModal.svelte` — new component

Wraps base `Modal`. Lists all certifications (from `forge.certifications.list()`), excludes certs already linked to this resume. Shows `short_name` + issuer name. Click fires `onselect(certificationId)`. Parent adds to junction via the new API endpoint.

## Testing

### Repository
- CRUD for certifications with new fields (short_name, long_name, cert_id, issuer_id, credly_url)
- addCertification / removeCertification / listCertifications for the junction

### Compiler
- `buildCertificationItems` reads from junction, groups by issuer_name, uses short_name
- Empty junction → empty array
- Multiple certs from different issuers → multiple categories

### Structural (webui)
- CertPickerModal source assertions
- DragNDropView cert section renders from IR (existing)
- openPicker has certifications case

## Files affected

### Create
- Migration file
- `packages/webui/src/lib/components/resume/CertPickerModal.svelte`

### Modify
- Core + SDK types (Certification, CreateCertification, UpdateCertification, ResumeCertification, AddResumeCertification)
- Certification repository/service
- Certification routes (field changes)
- Resume repository (addCertification, removeCertification, listCertifications)
- Resume routes (new cert endpoints)
- Resume compiler (`buildCertificationItems`)
- `/qualifications/certifications/+page.svelte` (form fields)
- `resumes/+page.svelte` (picker routing)
- SDK resources (certifications + resumes)
- Migration test (count + assertion)

## Risks

1. **Table rebuild on certifications** — requires `PRAGMA foreign_keys = OFF`. Same proven pattern used for sources (migration 039). Data migration uses `INSERT ... SELECT` with best-effort issuer matching.
2. **Issuer matching** — `SELECT o.id FROM organizations WHERE o.name = c.issuer LIMIT 1` may not match if the issuer name in the old data doesn't exactly match an org name. Unmatched issuers get `issuer_id = NULL`. User fixes via the updated form.
3. **Existing resume_entries for certs** — the old cert entries (created via SourcePicker before Phase 88) are orphaned in `resume_entries`. They point at cert sources via `source_id` but the compiler ignores them. These can be cleaned up (DELETE from resume_entries where the section is certifications) as part of this migration.
