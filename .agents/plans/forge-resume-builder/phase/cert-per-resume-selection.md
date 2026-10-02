# Per-Resume Certification Selection — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rework certifications schema (short_name/long_name/cert_id/issuer_id/credly_url/in_progress) and add per-resume cert selection via a resume_certifications junction table with picker + remove UI.

**Architecture:** Table rebuild migration for certifications + new junction table. Compiler reads from junction per-section. Cert picker modal for adding, per-entry × for removing. Follows the resume_skills junction pattern already in the codebase.

**Tech Stack:** SQLite, TypeScript, Svelte 5, Hono

**Spec:** `.claude/plans/forge-resume-builder/refs/specs/2026-04-05-cert-per-resume-selection.md`

---

## CRITICAL: All work in the worktree

```
/Users/adam/notes/job-hunting/.claude/worktrees/cert-rework
```

Branch: `phase/cert-per-resume`. ALL commands run from this directory.

## Files to Create

| File | Purpose |
|------|---------|
| `packages/core/src/db/migrations/041_cert_schema_rework.sql` | Rebuild certifications + create resume_certifications junction |
| `packages/webui/src/lib/components/resume/CertPickerModal.svelte` | Picker for adding certs to a resume |

## Files to Modify

| File | Change |
|------|--------|
| `packages/core/src/types/index.ts` | Certification, Create/Update types; ResumeCertification; AddResumeCertification |
| `packages/sdk/src/types.ts` | Mirror |
| `packages/core/src/db/repositories/certification-repository.ts` | CertificationRow, rowTo, create, update for new fields |
| `packages/core/src/services/certification-service.ts` | Field passthrough |
| `packages/core/src/db/repositories/resume-repository.ts` | addCertification, removeCertification, listCertificationsForSection |
| `packages/core/src/routes/resumes.ts` | POST/DELETE/GET cert junction endpoints |
| `packages/sdk/src/resources/resumes.ts` | addCertification, removeCertification, listCertifications SDK methods |
| `packages/core/src/services/resume-compiler.ts` | buildCertificationItems reads from junction |
| `packages/webui/src/routes/qualifications/certifications/+page.svelte` | Form fields for new schema |
| `packages/webui/src/routes/resumes/+page.svelte` | Picker routing for certifications |
| `packages/core/src/db/__tests__/migrate.test.ts` | Migration count |

---

## Tasks

### Task 1: Migration 041 — certifications rebuild + junction table

**Files:**
- Create: `packages/core/src/db/migrations/041_cert_schema_rework.sql`
- Modify: `packages/core/src/db/__tests__/migrate.test.ts`

Before starting: verify `ls packages/core/src/db/migrations/ | sort -V | tail -3`. Highest should be 040. If 041 exists, bump.

- [ ] **Step 1: Create migration file**

Write to `packages/core/src/db/migrations/041_cert_schema_rework.sql`:

```sql
-- Forge Resume Builder — Certification Schema Rework + Per-Resume Selection
-- Migration: 041_cert_schema_rework
-- Date: 2026-04-05
--
-- Rebuilds the certifications table with:
--   - name → short_name + long_name split
--   - cert_id (exam version code, nullable)
--   - issuer → issuer_id FK to organizations
--   - credly_url
--   - in_progress flag
--   - Drops education_source_id (UI already removed)
--
-- Creates resume_certifications junction table for per-resume cert selection.
-- Cleans up orphaned resume_entries that were created by the old SourcePicker
-- cert flow (before Phase 88 moved certs to their own entity).
--
-- PRAGMA foreign_keys = OFF

-- Step 1: Rebuild certifications table
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

-- Migrate existing data
INSERT INTO certifications_new (
  id, short_name, long_name, cert_id, issuer_id,
  date_earned, expiry_date, credential_id, credential_url, credly_url,
  in_progress, created_at, updated_at
)
SELECT
  c.id,
  c.name,
  c.name,
  NULL,
  (SELECT o.id FROM organizations o WHERE o.name = c.issuer LIMIT 1),
  c.date_earned,
  c.expiry_date,
  c.credential_id,
  c.credential_url,
  NULL,
  0,
  c.created_at,
  c.updated_at
FROM certifications c;

DROP TABLE certifications;
ALTER TABLE certifications_new RENAME TO certifications;
CREATE INDEX idx_certifications_issuer ON certifications(issuer_id);

-- Step 2: Create resume_certifications junction table
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

-- Step 3: Clean up orphaned resume_entries for cert sections
-- These were created by the old SourcePicker cert flow. The compiler
-- no longer reads resume_entries for certifications — it reads from
-- resume_certifications. These orphans waste space and confuse the IR.
DELETE FROM resume_entries
WHERE section_id IN (
  SELECT rs.id FROM resume_sections rs WHERE rs.entry_type = 'certifications'
);
```

- [ ] **Step 2: Update migration test** — bump count, add assertion for 041.

- [ ] **Step 3: Run migration tests**

```bash
bun test packages/core/src/db/__tests__/migrate.test.ts 2>&1 | tail -8
```

- [ ] **Step 4: Commit**

```bash
git add packages/core/src/db/migrations/041_cert_schema_rework.sql packages/core/src/db/__tests__/migrate.test.ts
git commit -m "feat(core): migration 041 — certifications schema rework + resume_certifications junction"
```

---

### Task 2: Core + SDK types

**Files:** `packages/core/src/types/index.ts`, `packages/sdk/src/types.ts`

- [ ] **Step 1: Update `Certification` interface** in both files. Replace old fields with:

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
```

- [ ] **Step 2: Update `CreateCertification`** — replace old fields:

```ts
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
```

- [ ] **Step 3: Update `UpdateCertification`** — replace old fields:

```ts
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
```

- [ ] **Step 4: Add junction types** (in both files):

```ts
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
  position?: number
}
```

- [ ] **Step 5: Mirror in SDK types**

- [ ] **Step 6: Run core tests, fix any shape mismatches**

- [ ] **Step 7: Commit**

```bash
git commit -m "feat(core): certification + resume_certifications junction types"
```

---

### Task 3: Certification repository + service

**Files:** `packages/core/src/db/repositories/certification-repository.ts`, `packages/core/src/services/certification-service.ts`

- [ ] **Step 1: Update `CertificationRow` + `rowToCertification`** to match new schema fields (short_name, long_name, cert_id, issuer_id, credly_url, in_progress; remove name, issuer, education_source_id).

- [ ] **Step 2: Update `create()` INSERT** to use new columns.

- [ ] **Step 3: Update `update()` set-builder** to handle new fields.

- [ ] **Step 4: Update `findById` and list queries** — SELECT new columns. If there's a `linked_resume_count` subquery, update it to count from `resume_certifications` instead of the old `resumes.summary_id`-style pattern.

- [ ] **Step 5: Update certification service** to pass new fields through.

- [ ] **Step 6: Run cert-specific tests** (if they exist in the test suite). Fix any that reference old field names.

- [ ] **Step 7: Commit**

```bash
git commit -m "feat(core): certification repository + service for new schema fields"
```

---

### Task 4: Resume repository + routes — cert junction CRUD

**Files:** `packages/core/src/db/repositories/resume-repository.ts`, `packages/core/src/routes/resumes.ts`

- [ ] **Step 1: Add junction methods to resume-repository.ts** following the `addSkill/removeSkill/listSkillsForSection` pattern:

```ts
addCertification(db: Database, resumeId: string, input: AddResumeCertification): ResumeCertification {
  const id = crypto.randomUUID()
  const position = input.position ?? (
    (db.query('SELECT COALESCE(MAX(position), -1) + 1 AS next FROM resume_certifications WHERE section_id = ?')
      .get(input.section_id) as { next: number }).next
  )
  return db.query(
    `INSERT INTO resume_certifications (id, resume_id, certification_id, section_id, position)
     VALUES (?, ?, ?, ?, ?) RETURNING *`
  ).get(id, resumeId, input.certification_id, input.section_id, position) as ResumeCertification
}

removeCertification(db: Database, resumeId: string, rcId: string): boolean {
  const result = db.run(
    'DELETE FROM resume_certifications WHERE id = ? AND resume_id = ?',
    [rcId, resumeId]
  )
  return result.changes > 0
}

listCertificationsForSection(db: Database, sectionId: string): ResumeCertification[] {
  return db.query(
    'SELECT * FROM resume_certifications WHERE section_id = ? ORDER BY position'
  ).all(sectionId) as ResumeCertification[]
}
```

- [ ] **Step 2: Add routes to resumes.ts**

```ts
// Resume certifications (per-resume cert selection)
app.post('/resumes/:id/certifications', async (c) => {
  const body = await c.req.json<AddResumeCertification>()
  const rc = ResumeRepository.addCertification(db, c.req.param('id'), body)
  return c.json({ data: rc }, 201)
})

app.delete('/resumes/:id/certifications/:rcId', (c) => {
  const ok = ResumeRepository.removeCertification(db, c.req.param('id'), c.req.param('rcId'))
  if (!ok) return c.json({ error: { code: 'NOT_FOUND', message: 'Not found' } }, 404)
  return c.body(null, 204)
})

app.get('/resumes/:id/certifications', (c) => {
  // List all certifications linked to this resume (across all cert sections)
  const rows = db.query(
    'SELECT * FROM resume_certifications WHERE resume_id = ? ORDER BY position'
  ).all(c.req.param('id'))
  return c.json({ data: rows })
})
```

- [ ] **Step 3: Run core tests**

- [ ] **Step 4: Commit**

```bash
git commit -m "feat(core): resume cert junction CRUD + routes"
```

---

### Task 5: SDK types + resources

**Files:** `packages/sdk/src/types.ts`, `packages/sdk/src/resources/resumes.ts`, `packages/sdk/src/resources/certifications.ts` (if exists)

- [ ] **Step 1: Ensure SDK types mirror core** (done partially in Task 2).

- [ ] **Step 2: Add cert junction methods to resumes SDK resource:**

```ts
addCertification(resumeId: string, input: AddResumeCertification): Promise<Result<ResumeCertification>> {
  return this.request<ResumeCertification>('POST', `/api/resumes/${resumeId}/certifications`, input)
}

removeCertification(resumeId: string, rcId: string): Promise<Result<void>> {
  return this.request<void>('DELETE', `/api/resumes/${resumeId}/certifications/${rcId}`)
}

listCertifications(resumeId: string): Promise<Result<ResumeCertification[]>> {
  return this.request<ResumeCertification[]>('GET', `/api/resumes/${resumeId}/certifications`)
}
```

- [ ] **Step 3: Commit**

```bash
git commit -m "feat(sdk): resume certification junction SDK methods"
```

---

### Task 6: Compiler — buildCertificationItems from junction

**Files:** `packages/core/src/services/resume-compiler.ts`, `packages/core/src/services/__tests__/resume-compiler.test.ts`

- [ ] **Step 1: Rewrite buildCertificationItems** to read from `resume_certifications` junction:

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

- [ ] **Step 2: Write compiler test** for the new junction-based flow:

```ts
test('certification section reads from resume_certifications junction', () => {
  const resumeId = seedResume(db)
  // Create a certification directly in the certifications table
  const certId = crypto.randomUUID()
  db.run(
    `INSERT INTO certifications (id, short_name, long_name, in_progress)
     VALUES (?, ?, ?, ?)`,
    [certId, 'Security+', 'CompTIA Security+', 0]
  )
  const secId = seedResumeSection(db, resumeId, 'Certifications', 'certifications', 0)
  // Link via junction
  const rcId = crypto.randomUUID()
  db.run(
    `INSERT INTO resume_certifications (id, resume_id, certification_id, section_id, position)
     VALUES (?, ?, ?, ?, ?)`,
    [rcId, resumeId, certId, secId, 0]
  )

  const result = compileResumeIR(db, resumeId)!
  const certSection = result.sections.find(s => s.type === 'certifications')!
  expect(certSection.items).toHaveLength(1)
  const group = certSection.items[0]
  expect(group.kind).toBe('certification_group')
  if (group.kind === 'certification_group') {
    expect(group.categories[0].certs[0].name).toBe('Security+')
    expect(group.categories[0].certs[0].entry_id).toBe(rcId)
  }
})

test('certification section returns empty when no junction entries', () => {
  const resumeId = seedResume(db)
  const secId = seedResumeSection(db, resumeId, 'Certifications', 'certifications', 0)
  const result = compileResumeIR(db, resumeId)!
  const certSection = result.sections.find(s => s.type === 'certifications')!
  expect(certSection.items).toHaveLength(0)
})
```

- [ ] **Step 3: Run compiler tests, fix any that asserted old behavior**

- [ ] **Step 4: Commit**

```bash
git commit -m "feat(core): compiler reads certifications from resume_certifications junction"
```

---

### Task 7: WebUI — /qualifications/certifications form update

**Files:** `packages/webui/src/routes/qualifications/certifications/+page.svelte`

- [ ] **Step 1: Update form state variables** to match new fields:
  - Replace `formName` with `formShortName` + `formLongName`
  - Add `formCertId`, `formCredlyUrl`, `formInProgress`
  - Replace `formIssuer` (text) with `formIssuerId` (org FK) using an OrgCombobox
  - Remove `formEducationSourceId` references (already removed in prior cleanup; verify)

- [ ] **Step 2: Update populateForm** to read new fields from cert object.

- [ ] **Step 3: Update save/create** to send new field names.

- [ ] **Step 4: Update the form template** with new inputs:
  - Short Name (text, required)
  - Long Name (text, required)
  - Cert ID (text, nullable placeholder "e.g. SY0-701")
  - Issuer (OrgCombobox — import from `$lib/components`)
  - Date Earned, Expiry Date
  - Credential ID, Credential URL, Credly URL
  - In Progress (checkbox)

- [ ] **Step 5: Update card/list display** to show `short_name` + issuer org name.

- [ ] **Step 6: Commit**

```bash
git commit -m "feat(webui): certifications page form for new schema fields"
```

---

### Task 8: WebUI — resume cert picker + DragNDropView

**Files:**
- Create: `packages/webui/src/lib/components/resume/CertPickerModal.svelte`
- Modify: `packages/webui/src/routes/resumes/+page.svelte`
- Modify: `packages/webui/src/lib/components/resume/DragNDropView.svelte`

- [ ] **Step 1: Create CertPickerModal.svelte** — wraps base Modal, lists all certifications via `forge.certifications.list()`, excludes certs already linked to this resume. Shows `short_name` + issuer name. Click fires `onselect(certificationId)`.

Pattern: similar to `SummaryPickerModal.svelte` (pick mode + close).

- [ ] **Step 2: Update openPicker in +page.svelte** — `case 'certifications':` opens the CertPickerModal instead of SourcePicker. Pass the current resume's linked cert IDs so the picker can exclude them.

Wire the `onselect` callback to call `forge.resumes.addCertification(resumeId, { certification_id, section_id })` then reload IR.

- [ ] **Step 3: Update DragNDropView cert template** — the existing cert template renders from `CertificationGroup` IR. The `entry_id` on each cert now maps to `resume_certifications.id`. The existing `onRemoveEntry` callback should work IF the remove handler is updated to call `forge.resumes.removeCertification` instead of `forge.resumes.removeEntry`.

Add a new `onRemoveCertification?: (rcId: string) => Promise<void>` prop to DragNDropView, and wire the cert template's × button to it (separate from `onRemoveEntry` which handles resume_entries).

Wire in +page.svelte:
```ts
onRemoveCertification={async (rcId) => {
  const result = await forge.resumes.removeCertification(selectedResumeId, rcId)
  if (result.ok) await loadIR(selectedResumeId)
  else addToast({ message: friendlyError(result.error), type: 'error' })
}}
```

- [ ] **Step 4: Commit**

```bash
git commit -m "feat(webui): CertPickerModal + resume cert picker routing + remove wiring"
```

---

### Task 9: Verification + merge

- [ ] **Step 1: Full test suites**

```bash
bun test packages/core/src/ 2>&1 | tail -5
bun test packages/webui/src/__tests__/ 2>&1 | tail -5
```

- [ ] **Step 2: svelte-check** on touched files

- [ ] **Step 3: Check file overlap + merge to main**

From main repo:
```bash
cd /Users/adam/notes/job-hunting
# Check overlap, rebase if needed, fast-forward merge
git merge --ff-only phase/cert-per-resume
```

- [ ] **Step 4: Cleanup worktree + branch**

- [ ] **Step 5: Close beads**

```bash
bd update job-hunting-89l --status closed
```

- [ ] **Step 6: Manual smoke test**

1. `/qualifications/certifications` → verify new form fields (short_name, long_name, cert_id, issuer OrgCombobox, credly_url, in_progress checkbox)
2. Edit a cert → set short_name, issuer org, verify round-trip
3. `/resumes` → select resume → Certifications section should be EMPTY (junction empty)
4. Click "+Add Entry" → CertPickerModal shows all 19 certs
5. Add a few → they appear in the section, grouped by issuer, showing `short_name`
6. Click × on one → removed from this resume (but still in `/qualifications/certifications`)

---

## Acceptance Criteria

- [ ] `certifications` table has new schema (short_name, long_name, cert_id, issuer_id FK, credly_url, in_progress)
- [ ] `resume_certifications` junction table exists
- [ ] Orphaned resume_entries for cert sections cleaned up
- [ ] Compiler reads from junction, not from global certifications table
- [ ] Per-resume cert selection works (add via picker, remove via ×)
- [ ] `/qualifications/certifications` form has all new fields + OrgCombobox for issuer
- [ ] Resume displays `short_name` only, grouped by issuer org name
- [ ] All tests pass
