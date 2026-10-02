# Phase 50: Contacts Support (Spec G)

**Status:** Planning
**Date:** 2026-04-03
**Spec:** [2026-04-03-contacts-support.md](../refs/specs/2026-04-03-contacts-support.md)
**Depends on:** None (but Spec E1 should exist for JD contact linking to be meaningful)
**Blocks:** None currently

> **Dependency on Phase 49:** Phase 49's JD detail page is incomplete until Phase 50 provides `ContactLinkSection.svelte` and cross-entity endpoints.
**Parallelizable with:** Phase 49 (JD Detail Page), Phase 48 (Generic GraphView) -- no file conflicts except Phase 49's JD routes file (which gets a contacts reverse lookup added here)

## Goal

Introduce a `contacts` entity for tracking people at organizations -- recruiters, hiring managers, interviewers, references, and peers. Create the full vertical slice: migration 019 (contacts table + 3 junction tables + note_references rebuild), repository, service, API routes, SDK resource, navigation entry, and a split-panel UI at `/data/contacts`. Add cross-entity reference sections on JD, organization, and resume detail views via a reusable `ContactLinkSection.svelte` component.

## Non-Goals

- Contact import from LinkedIn, email, or vCard
- Contact deduplication / merge tooling
- Communication tracking (emails sent, calls logged)
- Calendar integration (interview scheduling)
- Contact activity timeline
- Bulk contact operations
- Contact profile photos / avatars
- Social media integration beyond LinkedIn URL
- Contact search by relationship type across entities
- Contact export (CSV, vCard)

## Context

The Forge resume builder tracks organizations, job descriptions, and resumes. People (recruiters, hiring managers, references) are currently tracked only in free-text notes fields. This phase adds a structured `contacts` entity with typed relationships to all three entity types. The `contact_organizations`, `contact_job_descriptions`, and `contact_resumes` junction tables use three-column primary keys `(contact_id, entity_id, relationship)` to allow one contact to have multiple relationship types with the same entity.

The UI follows the same split-panel pattern used by organizations (`/data/organizations`), sources (`/data/sources`), and other data pages. Cross-entity reference sections use a reusable `ContactLinkSection.svelte` component that accepts entity type/ID and relationship type options.

## Scope

| Spec section | Covered here? |
|-------------|---------------|
| 1. Data model (contacts table + 3 junction tables) | Yes |
| 2. Migration (019_contacts.sql) | Yes |
| 3. Types (Contact, ContactWithOrg, CreateContact, UpdateContact, relationship types, ContactLink, ContactFilter) | Yes |
| 4. API routes (CRUD + relationship management + reverse lookups) | Yes |
| 5. Repository (CRUD + relationship methods + reverse lookups) | Yes |
| 6. Service (validation layer) | Yes |
| 7. UI layout (split-panel at /data/contacts) | Yes |
| 8. Cross-entity reference sections (JD, org, resume) | Yes |
| 9. SDK resource | Yes |
| 10. Navigation | Yes |
| 11. Files to create | Yes |
| 12. Files to modify | Yes |
| 13. Testing | Yes |
| 14. Acceptance criteria | Yes |

## Files to Create

| File | Description |
|------|-------------|
| `packages/core/src/db/migrations/019_contacts.sql` | contacts table + 3 junction tables + note_references rebuild |
| `packages/core/src/db/repositories/contact-repository.ts` | Contact CRUD + relationship management + reverse lookups |
| `packages/core/src/services/contact-service.ts` | Validation layer over repository |
| `packages/core/src/routes/contacts.ts` | API route handlers for contacts CRUD + relationships |
| `packages/sdk/src/resources/contacts.ts` | SDK resource class |
| `packages/webui/src/routes/data/contacts/+page.svelte` | Contacts page (split-panel) |
| `packages/webui/src/lib/components/contacts/ContactCard.svelte` | Contact card for list panel |
| `packages/webui/src/lib/components/contacts/ContactEditor.svelte` | Contact editor form |
| `packages/webui/src/lib/components/contacts/ContactLinkSection.svelte` | Reusable cross-entity contact linking section |
| `packages/webui/src/lib/components/contacts/LinkContactDialog.svelte` | Dialog for linking a contact to an entity |

## Files to Modify

| File | Change |
|------|--------|
| `packages/core/src/types/index.ts` | Add Contact types, relationship types, ContactLink, ContactFilter. Add `'contact'` to `NoteReferenceEntityType`. |
| `packages/sdk/src/types.ts` | Mirror type additions from core |
| `packages/sdk/src/client.ts` | Register `ContactsResource` on `ForgeClient` |
| `packages/core/src/services/index.ts` | Register `ContactService` in the `Services` object |
| `packages/core/src/routes/server.ts` | Mount contact routes at `/api/contacts` |
| `packages/core/src/routes/organizations.ts` | Add `GET /api/organizations/:id/contacts` reverse lookup endpoint |
| `packages/core/src/routes/job-descriptions.ts` | Add `GET /api/job-descriptions/:id/contacts` reverse lookup endpoint |
| `packages/core/src/routes/resumes.ts` | Add `GET /api/resumes/:id/contacts` reverse lookup endpoint |
| `packages/webui/src/lib/nav.ts` | Add "Contacts" nav entry under Data section |

## Fallback Strategies

- **Migration 019 runs before migration 018:** Migration 019 depends on `job_descriptions` (007) and `resumes` (001) existing. It does not depend on migration 018 (job_description_skills). Safe to run in any order relative to 018.
- **note_references rebuild fails mid-migration:** The migration uses `PRAGMA foreign_keys = OFF` during the table rebuild. If the migration fails partway through, the `note_references_new` table may exist alongside the original. Recovery: drop `note_references_new` and re-run. SQLite transactions protect against partial execution.
- **Contact with deleted organization:** `organization_id` uses `ON DELETE SET NULL`. If the linked org is deleted, the contact's primary org becomes null. Junction table entries in `contact_organizations` use `ON DELETE CASCADE` -- the relationship rows are removed.
- **Relationship type mismatch on API call:** CHECK constraints enforce valid relationship values at the DB level. The service layer validates before the INSERT, returning 400 with a descriptive error message.
- **Cross-entity section on page without Phase 49 (JD detail):** If Phase 49 is not yet implemented, the JD contact section simply will not be added until that page exists. The `ContactLinkSection.svelte` component is generic and works on any entity page.

---

## Tasks

### T50.1: Add Types to Core and SDK

**File:** `packages/core/src/types/index.ts`

[CRITICAL] Add `'contact'` to `NoteReferenceEntityType` union. The migration rebuilds the `note_references` table CHECK constraint to include this value.

[IMPORTANT] All relationship type unions must match the CHECK constraints in migration 019 exactly.

Add the following types after the existing JD types section:

```typescript
// ── Contact Entity ─────────────────────────────────────────────────────

/** A contact person tracked in the job hunting process. */
export interface Contact {
  id: string
  name: string
  title: string | null
  email: string | null
  phone: string | null
  linkedin: string | null
  team: string | null
  dept: string | null
  notes: string | null
  organization_id: string | null
  created_at: string
  updated_at: string
}

/** Contact with computed organization_name from JOIN. Used in API responses. */
export interface ContactWithOrg extends Contact {
  organization_name: string | null
}

/** Input for creating a new Contact. */
export interface CreateContact {
  name: string
  title?: string
  email?: string
  phone?: string
  linkedin?: string
  team?: string
  dept?: string
  notes?: string
  organization_id?: string
}

/** Input for partially updating a Contact. */
export interface UpdateContact {
  name?: string
  title?: string | null
  email?: string | null
  phone?: string | null
  linkedin?: string | null
  team?: string | null
  dept?: string | null
  notes?: string | null
  organization_id?: string | null
}

/** Valid relationship types for contact-organization links. */
export type ContactOrgRelationship = 'recruiter' | 'hr' | 'referral' | 'peer' | 'manager' | 'other'

/** Valid relationship types for contact-job description links. */
export type ContactJDRelationship = 'hiring_manager' | 'recruiter' | 'interviewer' | 'referral' | 'other'

/** Valid relationship types for contact-resume links. */
export type ContactResumeRelationship = 'reference' | 'recommender' | 'other'

/** A contact linked to an entity with a typed relationship. */
export interface ContactLink {
  contact_id: string
  contact_name: string
  contact_title: string | null
  contact_email: string | null
  relationship: string
}

/** Filter parameters for listing contacts. */
export interface ContactFilter {
  organization_id?: string
  search?: string
}
```

Also update `NoteReferenceEntityType`:

```typescript
// Before:
export type NoteReferenceEntityType =
  | 'source'
  | 'bullet'
  | 'perspective'
  | 'resume_entry'
  | 'resume'
  | 'skill'
  | 'organization'
  | 'job_description'

// After:
export type NoteReferenceEntityType =
  | 'source'
  | 'bullet'
  | 'perspective'
  | 'resume_entry'
  | 'resume'
  | 'skill'
  | 'organization'
  | 'job_description'
  | 'contact'
```

**File:** `packages/sdk/src/types.ts`

Mirror all type additions from core. Add `'contact'` to `NoteReferenceEntityType`.

**Acceptance criteria:**
- All Contact types compile with strict TypeScript.
- `NoteReferenceEntityType` includes `'contact'` in both core and SDK.
- Relationship type unions match CHECK constraints exactly.

**Failure criteria:**
- Types in SDK diverge from core types.
- `NoteReferenceEntityType` missing `'contact'`.
- Relationship type values do not match migration CHECK constraints.

---

### T50.2: Write Migration 019 -- Contacts

**File:** `packages/core/src/db/migrations/019_contacts.sql`

[CRITICAL] The migration must handle the `note_references` table rebuild correctly. `PRAGMA foreign_keys = OFF` must be set before the rebuild and `PRAGMA foreign_keys = ON` after. The index `idx_note_refs_entity` must be recreated.

[SEVERE] Three-column primary keys `(contact_id, entity_id, relationship)` are intentional. They allow one contact to have multiple relationship types at the same entity (e.g., both `peer` and `referral` at the same org).

[IMPORTANT] `contacts.organization_id` uses `ON DELETE SET NULL` (not CASCADE) -- the contact survives if their primary org is deleted. Junction tables use `ON DELETE CASCADE` -- link rows are removed when either side is deleted.

```sql
-- Contacts Entity
-- Migration: 019_contacts
-- Adds contacts table and three junction tables for linking contacts
-- to organizations, job descriptions, and resumes with relationship types.

-- Step 1: Create contacts table
CREATE TABLE contacts (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  name TEXT NOT NULL,
  title TEXT,
  email TEXT,
  phone TEXT,
  linkedin TEXT,
  team TEXT,
  dept TEXT,
  notes TEXT,
  organization_id TEXT REFERENCES organizations(id) ON DELETE SET NULL,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

CREATE INDEX idx_contacts_org ON contacts(organization_id);
CREATE INDEX idx_contacts_name ON contacts(name);

-- Step 2: Contact-Organization junction
CREATE TABLE contact_organizations (
  contact_id TEXT NOT NULL REFERENCES contacts(id) ON DELETE CASCADE,
  organization_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
  relationship TEXT NOT NULL CHECK (relationship IN (
    'recruiter', 'hr', 'referral', 'peer', 'manager', 'other'
  )),
  PRIMARY KEY (contact_id, organization_id, relationship)
) STRICT;

CREATE INDEX idx_contact_orgs_contact ON contact_organizations(contact_id);
CREATE INDEX idx_contact_orgs_org ON contact_organizations(organization_id);

-- Step 3: Contact-JobDescription junction
CREATE TABLE contact_job_descriptions (
  contact_id TEXT NOT NULL REFERENCES contacts(id) ON DELETE CASCADE,
  job_description_id TEXT NOT NULL REFERENCES job_descriptions(id) ON DELETE CASCADE,
  relationship TEXT NOT NULL CHECK (relationship IN (
    'hiring_manager', 'recruiter', 'interviewer', 'referral', 'other'
  )),
  PRIMARY KEY (contact_id, job_description_id, relationship)
) STRICT;

CREATE INDEX idx_contact_jds_contact ON contact_job_descriptions(contact_id);
CREATE INDEX idx_contact_jds_jd ON contact_job_descriptions(job_description_id);

-- Step 4: Contact-Resume junction
CREATE TABLE contact_resumes (
  contact_id TEXT NOT NULL REFERENCES contacts(id) ON DELETE CASCADE,
  resume_id TEXT NOT NULL REFERENCES resumes(id) ON DELETE CASCADE,
  relationship TEXT NOT NULL CHECK (relationship IN (
    'reference', 'recommender', 'other'
  )),
  PRIMARY KEY (contact_id, resume_id, relationship)
) STRICT;

CREATE INDEX idx_contact_resumes_contact ON contact_resumes(contact_id);
CREATE INDEX idx_contact_resumes_resume ON contact_resumes(resume_id);

-- Step 5: Extend note_references to include 'contact'
PRAGMA foreign_keys = OFF;

CREATE TABLE note_references_new (
  note_id TEXT NOT NULL CHECK(typeof(note_id) = 'text' AND length(note_id) = 36)
    REFERENCES user_notes(id) ON DELETE CASCADE,
  entity_type TEXT NOT NULL CHECK (entity_type IN (
    'source', 'bullet', 'perspective', 'resume_entry',
    'resume', 'skill', 'organization', 'job_description', 'contact'
  )),
  entity_id TEXT NOT NULL,
  PRIMARY KEY (note_id, entity_type, entity_id)
) STRICT;

INSERT INTO note_references_new SELECT * FROM note_references;
DROP TABLE note_references;
ALTER TABLE note_references_new RENAME TO note_references;
CREATE INDEX idx_note_refs_entity ON note_references(entity_type, entity_id);

PRAGMA foreign_keys = ON;

-- Step 6: Register migration
INSERT INTO _migrations (name) VALUES ('019_contacts');
```

**Acceptance criteria:**
- Migration creates `contacts`, `contact_organizations`, `contact_job_descriptions`, `contact_resumes` tables.
- `contacts.id` has UUID CHECK constraint (typeof + length = 36).
- `contacts.organization_id` FK uses `ON DELETE SET NULL`.
- All junction table FKs use `ON DELETE CASCADE`.
- Three-column PKs on all junction tables prevent exact-duplicate rows but allow same contact+entity with different relationships.
- CHECK constraints on `relationship` columns reject invalid values.
- `note_references` table accepts `'contact'` as `entity_type` after rebuild.
- All indexes created.
- Migration registered in `_migrations`.

**Failure criteria:**
- `PRAGMA foreign_keys = ON` not restored after rebuild.
- `note_references` data lost during rebuild.
- `idx_note_refs_entity` not recreated.
- Junction table PK is two columns instead of three (would prevent multiple relationship types).

---

### T50.3: Write Contact Repository

**File:** `packages/core/src/db/repositories/contact-repository.ts`

[CRITICAL] The `list()` method must support server-side search across `name`, `title`, and `email` (case-insensitive). This differs from JD/org list endpoints which do client-side filtering.

[IMPORTANT] Relationship management methods (`addOrganization`, `removeOrganization`, etc.) use `INSERT OR IGNORE` for idempotent linking and `DELETE` for unlinking.

[IMPORTANT] Reverse lookup methods (`listByOrganization`, `listByJobDescription`, `listByResume`) return `ContactLink[]` with contact details + relationship type.

```typescript
/**
 * ContactRepository -- CRUD operations for the contacts table and
 * relationship management for contact junction tables.
 *
 * All functions take a `Database` instance as the first parameter,
 * keeping the repository stateless and testable.
 */

import type { Database } from 'bun:sqlite'
import type {
  Contact,
  ContactWithOrg,
  CreateContact,
  UpdateContact,
  ContactFilter,
  ContactLink,
  ContactOrgRelationship,
  ContactJDRelationship,
  ContactResumeRelationship,
} from '../../types'

// ---------------------------------------------------------------------------
// Internal: base SELECT with organization JOIN
// ---------------------------------------------------------------------------

const SELECT_WITH_ORG = `
  SELECT c.*,
         o.name AS organization_name
  FROM contacts c
  LEFT JOIN organizations o ON o.id = c.organization_id
`

// ---------------------------------------------------------------------------
// CRUD
// ---------------------------------------------------------------------------

/** Insert a new contact and return the created row with org name. */
export function create(
  db: Database,
  input: CreateContact,
): ContactWithOrg {
  const id = crypto.randomUUID()
  db.query(
    `INSERT INTO contacts (id, name, title, email, phone, linkedin, team, dept, notes, organization_id)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
  ).run(
    id,
    input.name,
    input.title ?? null,
    input.email ?? null,
    input.phone ?? null,
    input.linkedin ?? null,
    input.team ?? null,
    input.dept ?? null,
    input.notes ?? null,
    input.organization_id ?? null,
  )

  return get(db, id)!
}

/** Retrieve a contact by ID with org name, or null if not found. */
export function get(
  db: Database,
  id: string,
): ContactWithOrg | null {
  return (
    (db
      .query(`${SELECT_WITH_ORG} WHERE c.id = ?`)
      .get(id) as ContactWithOrg | null) ?? null
  )
}

/**
 * List contacts with optional filters: organization_id, search.
 * Search is case-insensitive substring match on name, title, and email.
 * Results are ordered alphabetically by name.
 */
export function list(
  db: Database,
  filter?: ContactFilter,
  offset = 0,
  limit = 50,
): { data: ContactWithOrg[]; total: number } {
  const conditions: string[] = []
  const params: unknown[] = []

  if (filter?.organization_id !== undefined) {
    conditions.push('c.organization_id = ?')
    params.push(filter.organization_id)
  }
  if (filter?.search !== undefined && filter.search.trim()) {
    const searchTerm = `%${filter.search.trim()}%`
    conditions.push(
      `(c.name LIKE ? COLLATE NOCASE OR c.title LIKE ? COLLATE NOCASE OR c.email LIKE ? COLLATE NOCASE)`
    )
    params.push(searchTerm, searchTerm, searchTerm)
  }

  const where =
    conditions.length > 0 ? `WHERE ${conditions.join(' AND ')}` : ''

  const countRow = db
    .query(
      `SELECT COUNT(*) AS total FROM contacts c ${where}`,
    )
    .get(...params) as { total: number }

  const dataParams = [...params, limit, offset]
  const rows = db
    .query(
      `${SELECT_WITH_ORG} ${where} ORDER BY c.name ASC LIMIT ? OFFSET ?`,
    )
    .all(...dataParams) as ContactWithOrg[]

  return { data: rows, total: countRow.total }
}

/**
 * Partially update a contact.
 * Only the fields present in `input` are changed. `updated_at` is
 * always refreshed. Returns null if the contact does not exist.
 */
export function update(
  db: Database,
  id: string,
  input: UpdateContact,
): ContactWithOrg | null {
  const existing = get(db, id)
  if (!existing) return null

  const sets: string[] = []
  const params: unknown[] = []

  if (input.name !== undefined) {
    sets.push('name = ?')
    params.push(input.name)
  }
  if (input.title !== undefined) {
    sets.push('title = ?')
    params.push(input.title)
  }
  if (input.email !== undefined) {
    sets.push('email = ?')
    params.push(input.email)
  }
  if (input.phone !== undefined) {
    sets.push('phone = ?')
    params.push(input.phone)
  }
  if (input.linkedin !== undefined) {
    sets.push('linkedin = ?')
    params.push(input.linkedin)
  }
  if (input.team !== undefined) {
    sets.push('team = ?')
    params.push(input.team)
  }
  if (input.dept !== undefined) {
    sets.push('dept = ?')
    params.push(input.dept)
  }
  if (input.notes !== undefined) {
    sets.push('notes = ?')
    params.push(input.notes)
  }
  if (input.organization_id !== undefined) {
    sets.push('organization_id = ?')
    params.push(input.organization_id)
  }

  sets.push("updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')")
  params.push(id)

  db.query(
    `UPDATE contacts SET ${sets.join(', ')} WHERE id = ?`,
  ).run(...params)

  return get(db, id)
}

/** Delete a contact by ID. Returns true if a row was deleted. */
export function del(db: Database, id: string): boolean {
  const result = db.run('DELETE FROM contacts WHERE id = ?', [id])
  return result.changes > 0
}

// ---------------------------------------------------------------------------
// Organization relationships
// ---------------------------------------------------------------------------

/** Link a contact to an organization with a typed relationship. */
export function addOrganization(
  db: Database,
  contactId: string,
  orgId: string,
  relationship: ContactOrgRelationship,
): void {
  db.run(
    'INSERT OR IGNORE INTO contact_organizations (contact_id, organization_id, relationship) VALUES (?, ?, ?)',
    [contactId, orgId, relationship],
  )
}

/** Remove a specific contact-organization link by relationship. */
export function removeOrganization(
  db: Database,
  contactId: string,
  orgId: string,
  relationship: ContactOrgRelationship,
): void {
  db.run(
    'DELETE FROM contact_organizations WHERE contact_id = ? AND organization_id = ? AND relationship = ?',
    [contactId, orgId, relationship],
  )
}

/** List organizations linked to a contact, with relationship type. */
export function listOrganizations(
  db: Database,
  contactId: string,
): Array<{ id: string; name: string; relationship: ContactOrgRelationship }> {
  return db.query(
    `SELECT o.id, o.name, co.relationship
     FROM organizations o
     JOIN contact_organizations co ON co.organization_id = o.id
     WHERE co.contact_id = ?
     ORDER BY o.name ASC`
  ).all(contactId) as Array<{ id: string; name: string; relationship: ContactOrgRelationship }>
}

// ---------------------------------------------------------------------------
// Job Description relationships
// ---------------------------------------------------------------------------

/** Link a contact to a job description with a typed relationship. */
export function addJobDescription(
  db: Database,
  contactId: string,
  jdId: string,
  relationship: ContactJDRelationship,
): void {
  db.run(
    'INSERT OR IGNORE INTO contact_job_descriptions (contact_id, job_description_id, relationship) VALUES (?, ?, ?)',
    [contactId, jdId, relationship],
  )
}

/** Remove a specific contact-job description link by relationship. */
export function removeJobDescription(
  db: Database,
  contactId: string,
  jdId: string,
  relationship: ContactJDRelationship,
): void {
  db.run(
    'DELETE FROM contact_job_descriptions WHERE contact_id = ? AND job_description_id = ? AND relationship = ?',
    [contactId, jdId, relationship],
  )
}

/** List job descriptions linked to a contact, with relationship type and org name. */
export function listJobDescriptions(
  db: Database,
  contactId: string,
): Array<{ id: string; title: string; organization_name: string | null; relationship: ContactJDRelationship }> {
  return db.query(
    `SELECT jd.id, jd.title, o.name AS organization_name, cjd.relationship
     FROM job_descriptions jd
     JOIN contact_job_descriptions cjd ON cjd.job_description_id = jd.id
     LEFT JOIN organizations o ON o.id = jd.organization_id
     WHERE cjd.contact_id = ?
     ORDER BY jd.title ASC`
  ).all(contactId) as Array<{ id: string; title: string; organization_name: string | null; relationship: ContactJDRelationship }>
}

// ---------------------------------------------------------------------------
// Resume relationships
// ---------------------------------------------------------------------------

/** Link a contact to a resume with a typed relationship. */
export function addResume(
  db: Database,
  contactId: string,
  resumeId: string,
  relationship: ContactResumeRelationship,
): void {
  db.run(
    'INSERT OR IGNORE INTO contact_resumes (contact_id, resume_id, relationship) VALUES (?, ?, ?)',
    [contactId, resumeId, relationship],
  )
}

/** Remove a specific contact-resume link by relationship. */
export function removeResume(
  db: Database,
  contactId: string,
  resumeId: string,
  relationship: ContactResumeRelationship,
): void {
  db.run(
    'DELETE FROM contact_resumes WHERE contact_id = ? AND resume_id = ? AND relationship = ?',
    [contactId, resumeId, relationship],
  )
}

/** List resumes linked to a contact, with relationship type. */
export function listResumes(
  db: Database,
  contactId: string,
): Array<{ id: string; name: string; relationship: ContactResumeRelationship }> {
  return db.query(
    `SELECT r.id, r.name, cr.relationship
     FROM resumes r
     JOIN contact_resumes cr ON cr.resume_id = r.id
     WHERE cr.contact_id = ?
     ORDER BY r.name ASC`
  ).all(contactId) as Array<{ id: string; name: string; relationship: ContactResumeRelationship }>
}

// ---------------------------------------------------------------------------
// Reverse lookups (contacts linked TO an entity)
// ---------------------------------------------------------------------------

/** List contacts linked to an organization with relationship type. */
export function listByOrganization(
  db: Database,
  orgId: string,
): ContactLink[] {
  return db.query(
    `SELECT c.id AS contact_id, c.name AS contact_name, c.title AS contact_title,
            c.email AS contact_email, co.relationship
     FROM contacts c
     JOIN contact_organizations co ON co.contact_id = c.id
     WHERE co.organization_id = ?
     ORDER BY c.name ASC`
  ).all(orgId) as ContactLink[]
}

/** List contacts linked to a job description with relationship type. */
export function listByJobDescription(
  db: Database,
  jdId: string,
): ContactLink[] {
  return db.query(
    `SELECT c.id AS contact_id, c.name AS contact_name, c.title AS contact_title,
            c.email AS contact_email, cjd.relationship
     FROM contacts c
     JOIN contact_job_descriptions cjd ON cjd.contact_id = c.id
     WHERE cjd.job_description_id = ?
     ORDER BY c.name ASC`
  ).all(jdId) as ContactLink[]
}

/** List contacts linked to a resume with relationship type. */
export function listByResume(
  db: Database,
  resumeId: string,
): ContactLink[] {
  return db.query(
    `SELECT c.id AS contact_id, c.name AS contact_name, c.title AS contact_title,
            c.email AS contact_email, cr.relationship
     FROM contacts c
     JOIN contact_resumes cr ON cr.contact_id = c.id
     WHERE cr.resume_id = ?
     ORDER BY c.name ASC`
  ).all(resumeId) as ContactLink[]
}
```

**Acceptance criteria:**
- CRUD operations work: create returns ContactWithOrg, get returns with org_name, list supports search + org filter, update is partial, delete cascades.
- `list()` search matches case-insensitively on name, title, and email using LIKE with COLLATE NOCASE.
- `list()` orders by name ASC (alphabetical).
- All relationship add/remove/list methods work for orgs, JDs, and resumes.
- Reverse lookups return `ContactLink[]` with contact details + relationship.
- `addOrganization` / `addJobDescription` / `addResume` use `INSERT OR IGNORE` (idempotent).

**Failure criteria:**
- Search uses exact match instead of LIKE substring.
- Missing COLLATE NOCASE on search queries.
- `removeOrganization` deletes a specific relationship between contact and org. The three-column DELETE (`WHERE contact_id = ? AND organization_id = ? AND relationship = ?`) targets only the specified relationship, preserving other relationship types between the same contact and org.
- Reverse lookup queries missing JOIN to contacts table.

---

### T50.4: Write Contact Service

**File:** `packages/core/src/services/contact-service.ts`

[IMPORTANT] Validates `name` is required and non-empty. Basic format checks on `email` and `linkedin` if provided. Relationship values are validated against allowed sets.

```typescript
/**
 * ContactService -- business logic for contact entities.
 *
 * Validates input before delegating to the ContactRepository.
 * All methods return Result<T> (never throw).
 */

import type { Database } from 'bun:sqlite'
import type {
  ContactWithOrg,
  CreateContact,
  UpdateContact,
  ContactFilter,
  ContactLink,
  ContactOrgRelationship,
  ContactJDRelationship,
  ContactResumeRelationship,
  Result,
  PaginatedResult,
} from '../types'
import * as ContactRepo from '../db/repositories/contact-repository'

const VALID_ORG_RELATIONSHIPS: ContactOrgRelationship[] = [
  'recruiter', 'hr', 'referral', 'peer', 'manager', 'other',
]

const VALID_JD_RELATIONSHIPS: ContactJDRelationship[] = [
  'hiring_manager', 'recruiter', 'interviewer', 'referral', 'other',
]

const VALID_RESUME_RELATIONSHIPS: ContactResumeRelationship[] = [
  'reference', 'recommender', 'other',
]

export class ContactService {
  constructor(private db: Database) {}

  create(input: CreateContact): Result<ContactWithOrg> {
    if (!input.name || input.name.trim().length === 0) {
      return {
        ok: false,
        error: { code: 'VALIDATION_ERROR', message: 'Name must not be empty' },
      }
    }
    if (input.email && !this.isValidEmail(input.email)) {
      return {
        ok: false,
        error: { code: 'VALIDATION_ERROR', message: 'Invalid email format' },
      }
    }
    if (input.linkedin && !this.isValidUrl(input.linkedin)) {
      return {
        ok: false,
        error: { code: 'VALIDATION_ERROR', message: 'LinkedIn must be a valid URL' },
      }
    }

    const contact = ContactRepo.create(this.db, input)
    return { ok: true, data: contact }
  }

  get(id: string): Result<ContactWithOrg> {
    const contact = ContactRepo.get(this.db, id)
    if (!contact) {
      return {
        ok: false,
        error: { code: 'NOT_FOUND', message: `Contact ${id} not found` },
      }
    }
    return { ok: true, data: contact }
  }

  list(
    filter?: ContactFilter,
    offset?: number,
    limit?: number,
  ): PaginatedResult<ContactWithOrg> {
    const result = ContactRepo.list(this.db, filter, offset, limit)
    return {
      ok: true,
      data: result.data,
      pagination: {
        total: result.total,
        offset: offset ?? 0,
        limit: limit ?? 50,
      },
    }
  }

  update(id: string, input: UpdateContact): Result<ContactWithOrg> {
    if (input.name !== undefined && input.name.trim().length === 0) {
      return {
        ok: false,
        error: { code: 'VALIDATION_ERROR', message: 'Name must not be empty' },
      }
    }
    if (input.email !== undefined && input.email !== null && !this.isValidEmail(input.email)) {
      return {
        ok: false,
        error: { code: 'VALIDATION_ERROR', message: 'Invalid email format' },
      }
    }
    if (input.linkedin !== undefined && input.linkedin !== null && !this.isValidUrl(input.linkedin)) {
      return {
        ok: false,
        error: { code: 'VALIDATION_ERROR', message: 'LinkedIn must be a valid URL' },
      }
    }

    const contact = ContactRepo.update(this.db, id, input)
    if (!contact) {
      return {
        ok: false,
        error: { code: 'NOT_FOUND', message: `Contact ${id} not found` },
      }
    }
    return { ok: true, data: contact }
  }

  delete(id: string): Result<void> {
    const deleted = ContactRepo.del(this.db, id)
    if (!deleted) {
      return {
        ok: false,
        error: { code: 'NOT_FOUND', message: `Contact ${id} not found` },
      }
    }
    return { ok: true, data: undefined }
  }

  // ── Organization relationships ────────────────────────────────────

  linkOrganization(
    contactId: string,
    orgId: string,
    relationship: string,
  ): Result<void> {
    if (!VALID_ORG_RELATIONSHIPS.includes(relationship as ContactOrgRelationship)) {
      return {
        ok: false,
        error: {
          code: 'VALIDATION_ERROR',
          message: `Invalid relationship: ${relationship}. Must be one of: ${VALID_ORG_RELATIONSHIPS.join(', ')}`,
        },
      }
    }
    ContactRepo.addOrganization(this.db, contactId, orgId, relationship as ContactOrgRelationship)
    return { ok: true, data: undefined }
  }

  unlinkOrganization(contactId: string, orgId: string, relationship: string): Result<void> {
    if (!VALID_ORG_RELATIONSHIPS.includes(relationship as ContactOrgRelationship)) {
      return {
        ok: false,
        error: {
          code: 'VALIDATION_ERROR',
          message: `Invalid relationship: ${relationship}. Must be one of: ${VALID_ORG_RELATIONSHIPS.join(', ')}`,
        },
      }
    }
    ContactRepo.removeOrganization(this.db, contactId, orgId, relationship as ContactOrgRelationship)
    return { ok: true, data: undefined }
  }

  listOrganizations(contactId: string): Result<Array<{ id: string; name: string; relationship: ContactOrgRelationship }>> {
    const orgs = ContactRepo.listOrganizations(this.db, contactId)
    return { ok: true, data: orgs }
  }

  // ── Job Description relationships ─────────────────────────────────

  linkJobDescription(
    contactId: string,
    jdId: string,
    relationship: string,
  ): Result<void> {
    if (!VALID_JD_RELATIONSHIPS.includes(relationship as ContactJDRelationship)) {
      return {
        ok: false,
        error: {
          code: 'VALIDATION_ERROR',
          message: `Invalid relationship: ${relationship}. Must be one of: ${VALID_JD_RELATIONSHIPS.join(', ')}`,
        },
      }
    }
    ContactRepo.addJobDescription(this.db, contactId, jdId, relationship as ContactJDRelationship)
    return { ok: true, data: undefined }
  }

  unlinkJobDescription(contactId: string, jdId: string, relationship: string): Result<void> {
    if (!VALID_JD_RELATIONSHIPS.includes(relationship as ContactJDRelationship)) {
      return {
        ok: false,
        error: {
          code: 'VALIDATION_ERROR',
          message: `Invalid relationship: ${relationship}. Must be one of: ${VALID_JD_RELATIONSHIPS.join(', ')}`,
        },
      }
    }
    ContactRepo.removeJobDescription(this.db, contactId, jdId, relationship as ContactJDRelationship)
    return { ok: true, data: undefined }
  }

  listJobDescriptions(contactId: string): Result<Array<{ id: string; title: string; organization_name: string | null; relationship: ContactJDRelationship }>> {
    const jds = ContactRepo.listJobDescriptions(this.db, contactId)
    return { ok: true, data: jds }
  }

  // ── Resume relationships ──────────────────────────────────────────

  linkResume(
    contactId: string,
    resumeId: string,
    relationship: string,
  ): Result<void> {
    if (!VALID_RESUME_RELATIONSHIPS.includes(relationship as ContactResumeRelationship)) {
      return {
        ok: false,
        error: {
          code: 'VALIDATION_ERROR',
          message: `Invalid relationship: ${relationship}. Must be one of: ${VALID_RESUME_RELATIONSHIPS.join(', ')}`,
        },
      }
    }
    ContactRepo.addResume(this.db, contactId, resumeId, relationship as ContactResumeRelationship)
    return { ok: true, data: undefined }
  }

  unlinkResume(contactId: string, resumeId: string, relationship: string): Result<void> {
    if (!VALID_RESUME_RELATIONSHIPS.includes(relationship as ContactResumeRelationship)) {
      return {
        ok: false,
        error: {
          code: 'VALIDATION_ERROR',
          message: `Invalid relationship: ${relationship}. Must be one of: ${VALID_RESUME_RELATIONSHIPS.join(', ')}`,
        },
      }
    }
    ContactRepo.removeResume(this.db, contactId, resumeId, relationship as ContactResumeRelationship)
    return { ok: true, data: undefined }
  }

  listResumes(contactId: string): Result<Array<{ id: string; name: string; relationship: ContactResumeRelationship }>> {
    const resumes = ContactRepo.listResumes(this.db, contactId)
    return { ok: true, data: resumes }
  }

  // ── Reverse lookups ───────────────────────────────────────────────

  listByOrganization(orgId: string): Result<ContactLink[]> {
    return { ok: true, data: ContactRepo.listByOrganization(this.db, orgId) }
  }

  listByJobDescription(jdId: string): Result<ContactLink[]> {
    return { ok: true, data: ContactRepo.listByJobDescription(this.db, jdId) }
  }

  listByResume(resumeId: string): Result<ContactLink[]> {
    return { ok: true, data: ContactRepo.listByResume(this.db, resumeId) }
  }

  // ── Private helpers ───────────────────────────────────────────────

  private isValidEmail(email: string): boolean {
    return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email)
  }

  private isValidUrl(url: string): boolean {
    try {
      new URL(url)
      return true
    } catch {
      return false
    }
  }
}
```

**Acceptance criteria:**
- `create()` validates name is required, email format, linkedin URL format.
- `update()` validates same fields when provided.
- All relationship methods validate relationship type against allowed values.
- Invalid relationship type returns `VALIDATION_ERROR` with descriptive message.
- All CRUD methods return `Result<T>`.
- `list()` returns `PaginatedResult<ContactWithOrg>`.

**Failure criteria:**
- Email validation rejects valid emails (e.g., `user+tag@example.com`).
- LinkedIn validation requires specific LinkedIn URL format (should accept any URL).
- Missing validation on relationship type before DB insert.

---

### T50.5: Write Contact API Routes

**File:** `packages/core/src/routes/contacts.ts`

[CRITICAL] Contact routes need both `services` and `db`. The route function signature follows the `sourceRoutes` pattern: `contactRoutes(services: Services, db: Database)`. Direct DB access is needed for reverse lookup queries that go through the repository directly.

```typescript
/**
 * Contact routes -- CRUD, relationship management, and reverse lookups.
 */

import { Hono } from 'hono'
import type { Database } from 'bun:sqlite'
import type { Services } from '../services'
import { mapStatusCode } from './server'

export function contactRoutes(services: Services, db: Database) {
  const app = new Hono()

  // ── CRUD ──────────────────────────────────────────────────────────

  app.post('/contacts', async (c) => {
    const body = await c.req.json()
    const result = services.contacts.create(body)
    if (!result.ok)
      return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data }, 201)
  })

  app.get('/contacts', (c) => {
    const offset = Math.max(
      0,
      parseInt(c.req.query('offset') ?? '0', 10) || 0,
    )
    const limit = Math.min(
      200,
      Math.max(1, parseInt(c.req.query('limit') ?? '50', 10) || 50),
    )
    const filter: Record<string, string> = {}
    if (c.req.query('search')) filter.search = c.req.query('search')!
    if (c.req.query('organization_id'))
      filter.organization_id = c.req.query('organization_id')!

    const result = services.contacts.list(filter, offset, limit)
    if (!result.ok)
      return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data, pagination: result.pagination })
  })

  app.get('/contacts/:id', (c) => {
    const result = services.contacts.get(c.req.param('id'))
    if (!result.ok)
      return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.patch('/contacts/:id', async (c) => {
    const body = await c.req.json()
    const result = services.contacts.update(c.req.param('id'), body)
    if (!result.ok)
      return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.delete('/contacts/:id', (c) => {
    const result = services.contacts.delete(c.req.param('id'))
    if (!result.ok)
      return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.body(null, 204)
  })

  // ── Organization relationships ────────────────────────────────────

  app.get('/contacts/:id/organizations', (c) => {
    const result = services.contacts.listOrganizations(c.req.param('id'))
    if (!result.ok)
      return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.post('/contacts/:id/organizations', async (c) => {
    const body = await c.req.json()
    const result = services.contacts.linkOrganization(
      c.req.param('id'),
      body.organization_id,
      body.relationship,
    )
    if (!result.ok)
      return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.body(null, 201)
  })

  app.delete('/contacts/:contactId/organizations/:orgId/:relationship', (c) => {
    const result = services.contacts.unlinkOrganization(
      c.req.param('contactId'),
      c.req.param('orgId'),
      c.req.param('relationship'),
    )
    if (!result.ok)
      return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.body(null, 204)
  })

  // ── Job Description relationships ─────────────────────────────────

  app.get('/contacts/:id/job-descriptions', (c) => {
    const result = services.contacts.listJobDescriptions(c.req.param('id'))
    if (!result.ok)
      return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.post('/contacts/:id/job-descriptions', async (c) => {
    const body = await c.req.json()
    const result = services.contacts.linkJobDescription(
      c.req.param('id'),
      body.job_description_id,
      body.relationship,
    )
    if (!result.ok)
      return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.body(null, 201)
  })

  app.delete('/contacts/:contactId/job-descriptions/:jdId/:relationship', (c) => {
    const result = services.contacts.unlinkJobDescription(
      c.req.param('contactId'),
      c.req.param('jdId'),
      c.req.param('relationship'),
    )
    if (!result.ok)
      return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.body(null, 204)
  })

  // ── Resume relationships ──────────────────────────────────────────

  app.get('/contacts/:id/resumes', (c) => {
    const result = services.contacts.listResumes(c.req.param('id'))
    if (!result.ok)
      return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.post('/contacts/:id/resumes', async (c) => {
    const body = await c.req.json()
    const result = services.contacts.linkResume(
      c.req.param('id'),
      body.resume_id,
      body.relationship,
    )
    if (!result.ok)
      return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.body(null, 201)
  })

  app.delete('/contacts/:contactId/resumes/:resumeId/:relationship', (c) => {
    const result = services.contacts.unlinkResume(
      c.req.param('contactId'),
      c.req.param('resumeId'),
      c.req.param('relationship'),
    )
    if (!result.ok)
      return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.body(null, 204)
  })

  return app
}
```

**Acceptance criteria:**
- All CRUD endpoints work: POST (201), GET (200), PATCH (200), DELETE (204).
- List supports `?search=`, `?organization_id=`, `?limit=`, `?offset=`.
- POST without `name` returns 400.
- Relationship endpoints link/unlink/list for all three entity types.
- Invalid relationship type returns 400.
- DELETE relationship endpoints return 204 unconditionally.

---

### T50.6: Add Reverse Lookup Endpoints to Existing Routes

[IMPORTANT] These are small additions to existing route files. Each adds a single GET endpoint.

**File:** `packages/core/src/routes/organizations.ts` -- add after existing endpoints:

```typescript
  // ── Contact reverse lookup ──────────────────────────────────────────
  app.get('/organizations/:id/contacts', (c) => {
    const result = services.contacts.listByOrganization(c.req.param('id'))
    if (!result.ok)
      return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })
```

**File:** `packages/core/src/routes/job-descriptions.ts` -- add before `return app`:

```typescript
  // ── Contact reverse lookup ──────────────────────────────────────────
  app.get('/job-descriptions/:id/contacts', (c) => {
    const result = services.contacts.listByJobDescription(c.req.param('id'))
    if (!result.ok)
      return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })
```

**File:** `packages/core/src/routes/resumes.ts` -- add before `return app`:

```typescript
  // ── Contact reverse lookup ──────────────────────────────────────────
  app.get('/resumes/:id/contacts', (c) => {
    const result = services.contacts.listByResume(c.req.param('id'))
    if (!result.ok)
      return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })
```

[GAP] The organization routes currently take `(services: Services)` as their only parameter. The `services.contacts` reference will work because `contacts` is now on the `Services` interface. No signature change needed -- the reverse lookup goes through the service layer.

**Acceptance criteria:**
- `GET /api/organizations/:id/contacts` returns `{ data: ContactLink[] }`.
- `GET /api/job-descriptions/:id/contacts` returns `{ data: ContactLink[] }`.
- `GET /api/resumes/:id/contacts` returns `{ data: ContactLink[] }`.
- All three return empty arrays when no contacts are linked.

---

### T50.7: Register Contact Service and Routes

**File:** `packages/core/src/services/index.ts`

Add `ContactService` import and registration:

```typescript
import { ContactService } from './contact-service'

// In Services interface, add:
contacts: ContactService

// In createServices function, add:
contacts: new ContactService(db),

// At bottom, add re-export:
export { ContactService } from './contact-service'
```

**File:** `packages/core/src/routes/server.ts`

Add import and route mounting:

```typescript
import { contactRoutes } from './contacts'

// In createApp, add after other route registrations:
app.route('/', contactRoutes(services, db))
```

**Acceptance criteria:**
- `services.contacts` is available in all route handlers.
- `/api/contacts` endpoints are accessible.
- TypeScript compiles without errors.

---

### T50.8: Write SDK Contacts Resource

**File:** `packages/sdk/src/resources/contacts.ts`

```typescript
import type {
  ContactWithOrg,
  ContactFilter,
  ContactLink,
  ContactOrgRelationship,
  ContactJDRelationship,
  ContactResumeRelationship,
  CreateContact,
  PaginatedResult,
  PaginationParams,
  RequestFn,
  RequestListFn,
  Result,
  UpdateContact,
} from '../types'

function toParams(
  filter?: object,
): Record<string, string> | undefined {
  if (!filter) return undefined
  const out: Record<string, string> = {}
  for (const [k, v] of Object.entries(filter)) {
    if (v !== undefined && v !== null) out[k] = String(v)
  }
  return Object.keys(out).length > 0 ? out : undefined
}

export class ContactsResource {
  constructor(
    private request: RequestFn,
    private requestList: RequestListFn,
  ) {}

  // ── CRUD ──────────────────────────────────────────────────────────

  create(input: CreateContact): Promise<Result<ContactWithOrg>> {
    return this.request<ContactWithOrg>('POST', '/api/contacts', input)
  }

  list(
    filter?: ContactFilter & PaginationParams,
  ): Promise<PaginatedResult<ContactWithOrg>> {
    return this.requestList<ContactWithOrg>(
      'GET',
      '/api/contacts',
      toParams(filter),
    )
  }

  get(id: string): Promise<Result<ContactWithOrg>> {
    return this.request<ContactWithOrg>('GET', `/api/contacts/${id}`)
  }

  update(
    id: string,
    input: UpdateContact,
  ): Promise<Result<ContactWithOrg>> {
    return this.request<ContactWithOrg>('PATCH', `/api/contacts/${id}`, input)
  }

  delete(id: string): Promise<Result<void>> {
    return this.request<void>('DELETE', `/api/contacts/${id}`)
  }

  // ── Organization relationships ────────────────────────────────────

  listOrganizations(
    contactId: string,
  ): Promise<Result<Array<{ id: string; name: string; relationship: string }>>> {
    return this.request('GET', `/api/contacts/${contactId}/organizations`)
  }

  linkOrganization(
    contactId: string,
    orgId: string,
    relationship: ContactOrgRelationship,
  ): Promise<Result<void>> {
    return this.request<void>('POST', `/api/contacts/${contactId}/organizations`, {
      organization_id: orgId,
      relationship,
    })
  }

  unlinkOrganization(contactId: string, orgId: string, relationship: string): Promise<Result<void>> {
    return this.request<void>(
      'DELETE',
      `/api/contacts/${contactId}/organizations/${orgId}/${encodeURIComponent(relationship)}`,
    )
  }

  // ── Job Description relationships ─────────────────────────────────

  listJobDescriptions(
    contactId: string,
  ): Promise<Result<Array<{ id: string; title: string; organization_name: string | null; relationship: string }>>> {
    return this.request('GET', `/api/contacts/${contactId}/job-descriptions`)
  }

  linkJobDescription(
    contactId: string,
    jdId: string,
    relationship: ContactJDRelationship,
  ): Promise<Result<void>> {
    return this.request<void>('POST', `/api/contacts/${contactId}/job-descriptions`, {
      job_description_id: jdId,
      relationship,
    })
  }

  unlinkJobDescription(contactId: string, jdId: string, relationship: string): Promise<Result<void>> {
    return this.request<void>(
      'DELETE',
      `/api/contacts/${contactId}/job-descriptions/${jdId}/${encodeURIComponent(relationship)}`,
    )
  }

  // ── Resume relationships ──────────────────────────────────────────

  listResumes(
    contactId: string,
  ): Promise<Result<Array<{ id: string; name: string; relationship: string }>>> {
    return this.request('GET', `/api/contacts/${contactId}/resumes`)
  }

  linkResume(
    contactId: string,
    resumeId: string,
    relationship: ContactResumeRelationship,
  ): Promise<Result<void>> {
    return this.request<void>('POST', `/api/contacts/${contactId}/resumes`, {
      resume_id: resumeId,
      relationship,
    })
  }

  unlinkResume(contactId: string, resumeId: string, relationship: string): Promise<Result<void>> {
    return this.request<void>(
      'DELETE',
      `/api/contacts/${contactId}/resumes/${resumeId}/${encodeURIComponent(relationship)}`,
    )
  }

  // ── Reverse lookups ───────────────────────────────────────────────

  /** List contacts linked to an organization. Call from org context. */
  listByOrganization(orgId: string): Promise<Result<ContactLink[]>> {
    return this.request<ContactLink[]>(
      'GET',
      `/api/organizations/${orgId}/contacts`,
    )
  }

  /** List contacts linked to a job description. Call from JD context. */
  listByJobDescription(jdId: string): Promise<Result<ContactLink[]>> {
    return this.request<ContactLink[]>(
      'GET',
      `/api/job-descriptions/${jdId}/contacts`,
    )
  }

  /** List contacts linked to a resume. Call from resume context. */
  listByResume(resumeId: string): Promise<Result<ContactLink[]>> {
    return this.request<ContactLink[]>(
      'GET',
      `/api/resumes/${resumeId}/contacts`,
    )
  }
}
```

**File:** `packages/sdk/src/client.ts` -- register the resource:

```typescript
import { ContactsResource } from './resources/contacts'

// Add to ForgeClient class:
/** Contact CRUD + relationship management. */
public contacts: ContactsResource

// In constructor, add:
this.contacts = new ContactsResource(req, reqList)
```

**Acceptance criteria:**
- All CRUD methods work.
- All relationship methods (link/unlink/list) work for orgs, JDs, and resumes.
- Reverse lookup methods work.
- `forge.contacts` is accessible on the client.

---

### T50.9: Add Navigation Entry

**File:** `packages/webui/src/lib/nav.ts`

[MINOR] Add "Contacts" entry after "Skills" in the Data section.

```typescript
  {
    label: 'Data',
    prefix: '/data',
    children: [
      { href: '/data/bullets', label: 'Bullets' },
      { href: '/data/skills', label: 'Skills' },
      { href: '/data/contacts', label: 'Contacts' },
      { href: '/data/organizations', label: 'Organizations' },
      { href: '/data/domains', label: 'Domains' },
      { href: '/data/notes', label: 'Notes' },
    ],
  },
```

**Acceptance criteria:**
- "Contacts" appears in the sidebar under Data, between Skills and Organizations.
- Clicking navigates to `/data/contacts`.

---

### T50.10: Write ContactCard Component

**File:** `packages/webui/src/lib/components/contacts/ContactCard.svelte`

```svelte
<!--
  ContactCard.svelte -- Card for the contact list panel.
  Displays name, title, organization, and email.
-->
<script lang="ts">
  import type { ContactWithOrg } from '@forge/sdk'

  let {
    contact,
    selected = false,
    onclick,
  }: {
    contact: ContactWithOrg
    selected?: boolean
    onclick: () => void
  } = $props()
</script>

<button
  class="contact-card"
  class:selected
  onclick={onclick}
  type="button"
>
  <span class="name">{contact.name}</span>
  {#if contact.title}
    <span class="title">{contact.title}</span>
  {/if}
  {#if contact.organization_name}
    <span class="org">{contact.organization_name}</span>
  {/if}
  {#if contact.email}
    <span class="email">{contact.email}</span>
  {/if}
</button>

<style>
  .contact-card {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    width: 100%;
    text-align: left;
    padding: 0.75rem;
    border: 1px solid #e5e7eb;
    border-radius: 0.5rem;
    background: #fff;
    cursor: pointer;
    transition: border-color 0.15s, background-color 0.15s;
  }

  .contact-card:hover {
    border-color: #93c5fd;
    background: #f0f9ff;
  }

  .contact-card.selected {
    border-color: #3b82f6;
    background: #eff6ff;
  }

  .name {
    font-weight: 600;
    font-size: 0.9rem;
    color: #1a1a2e;
  }

  .title {
    font-size: 0.8rem;
    color: #6b7280;
  }

  .org {
    font-size: 0.8rem;
    color: #6b7280;
  }

  .email {
    font-size: 0.75rem;
    color: #9ca3af;
  }
</style>
```

---

### T50.11: Write LinkContactDialog Component

**File:** `packages/webui/src/lib/components/contacts/LinkContactDialog.svelte`

[IMPORTANT] This dialog is used by `ContactLinkSection.svelte` to link a contact to an entity. It needs two dropdowns: one for the target entity and one for the relationship type.

```svelte
<!--
  LinkContactDialog.svelte -- Dialog for linking a contact to an entity.
  Shows a contact dropdown and a relationship type dropdown.
-->
<script lang="ts">
  import type { ContactWithOrg } from '@forge/sdk'

  let {
    title = 'Link Contact',
    contacts = [],
    relationships = [],
    onlink,
    oncancel,
  }: {
    title?: string
    contacts: ContactWithOrg[]
    relationships: { value: string; label: string }[]
    onlink: (contactId: string, relationship: string) => void
    oncancel: () => void
  } = $props()

  let selectedContactId = $state('')
  let selectedRelationship = $state(relationships[0]?.value ?? '')

  function handleLink() {
    if (!selectedContactId || !selectedRelationship) return
    onlink(selectedContactId, selectedRelationship)
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="overlay" onclick={oncancel}>
  <div class="dialog" onclick|stopPropagation>
    <div class="dialog-header">
      <h3>{title}</h3>
      <button class="close-btn" onclick={oncancel} type="button">&times;</button>
    </div>

    <div class="dialog-body">
      <div class="field">
        <label for="link-contact">Contact</label>
        <select id="link-contact" bind:value={selectedContactId}>
          <option value="" disabled>Select a contact...</option>
          {#each contacts.sort((a, b) => a.name.localeCompare(b.name)) as c (c.id)}
            <option value={c.id}>{c.name}{c.title ? ` -- ${c.title}` : ''}</option>
          {/each}
        </select>
      </div>

      <div class="field">
        <label for="link-relationship">Relationship</label>
        <select id="link-relationship" bind:value={selectedRelationship}>
          {#each relationships as rel}
            <option value={rel.value}>{rel.label}</option>
          {/each}
        </select>
      </div>
    </div>

    <div class="dialog-footer">
      <button class="btn-cancel" onclick={oncancel} type="button">Cancel</button>
      <button
        class="btn-link"
        onclick={handleLink}
        disabled={!selectedContactId || !selectedRelationship}
        type="button"
      >
        Link
      </button>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    display: flex;
    justify-content: center;
    align-items: center;
    z-index: 100;
  }

  .dialog {
    background: #fff;
    border-radius: 0.5rem;
    width: 420px;
    max-width: 90vw;
    box-shadow: 0 8px 30px rgba(0, 0, 0, 0.15);
  }

  .dialog-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem 1.25rem;
    border-bottom: 1px solid #e5e7eb;
  }

  .dialog-header h3 {
    margin: 0;
    font-size: 1rem;
    font-weight: 700;
    color: #1a1a2e;
  }

  .close-btn {
    background: none;
    border: none;
    font-size: 1.5rem;
    cursor: pointer;
    color: #6b7280;
    line-height: 1;
  }

  .dialog-body {
    padding: 1rem 1.25rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .field label {
    font-size: 0.8rem;
    font-weight: 600;
    color: #374151;
  }

  .field select {
    padding: 0.5rem 0.6rem;
    border: 1px solid #d1d5db;
    border-radius: 0.375rem;
    font-size: 0.9rem;
    outline: none;
  }

  .field select:focus {
    border-color: #3b82f6;
    box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.15);
  }

  .dialog-footer {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    padding: 0.75rem 1.25rem;
    border-top: 1px solid #e5e7eb;
  }

  .btn-cancel {
    padding: 0.4rem 1rem;
    background: none;
    border: 1px solid #d1d5db;
    border-radius: 0.375rem;
    font-size: 0.85rem;
    cursor: pointer;
    color: #374151;
  }

  .btn-link {
    padding: 0.4rem 1rem;
    background: #3b82f6;
    color: #fff;
    border: none;
    border-radius: 0.375rem;
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
  }

  .btn-link:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-link:hover:not(:disabled) {
    background: #2563eb;
  }
</style>
```

---

### T50.12: Write ContactLinkSection Component

**File:** `packages/webui/src/lib/components/contacts/ContactLinkSection.svelte`

[CRITICAL] This is the reusable component used across JD, org, and resume detail views. It accepts entity-specific props and handles link/unlink operations via the SDK.

```svelte
<!--
  ContactLinkSection.svelte -- Reusable cross-entity contact linking section.
  Shows contacts linked to an entity and allows link/unlink operations.
-->
<script lang="ts">
  import { forge, friendlyError } from '$lib/sdk'
  import { addToast } from '$lib/stores/toast.svelte'
  import LinkContactDialog from './LinkContactDialog.svelte'
  import type { ContactLink, ContactWithOrg } from '@forge/sdk'

  let {
    sectionTitle = 'Contacts',
    entityType,
    entityId,
    relationships,
    linkedContacts = $bindable([]),
  }: {
    sectionTitle?: string
    entityType: 'organization' | 'job_description' | 'resume'
    entityId: string
    relationships: { value: string; label: string }[]
    linkedContacts: ContactLink[]
  } = $props()

  let showLinkDialog = $state(false)
  let allContacts = $state<ContactWithOrg[]>([])

  async function openLinkDialog() {
    const res = await forge.contacts.list({ limit: 500 })
    if (res.ok) {
      allContacts = res.data
    }
    showLinkDialog = true
  }

  async function handleLink(contactId: string, relationship: string) {
    let res: any
    if (entityType === 'organization') {
      res = await forge.contacts.linkOrganization(contactId, entityId, relationship as any)
    } else if (entityType === 'job_description') {
      res = await forge.contacts.linkJobDescription(contactId, entityId, relationship as any)
    } else if (entityType === 'resume') {
      res = await forge.contacts.linkResume(contactId, entityId, relationship as any)
    }

    if (res?.ok) {
      await refreshLinkedContacts()
      addToast({ type: 'success', message: 'Contact linked' })
    } else if (res) {
      addToast({ type: 'error', message: friendlyError(res.error) })
    }
    showLinkDialog = false
  }

  async function handleUnlink(contactId: string, relationship: string) {
    let res: any
    if (entityType === 'organization') {
      res = await forge.contacts.unlinkOrganization(contactId, entityId, relationship)
    } else if (entityType === 'job_description') {
      res = await forge.contacts.unlinkJobDescription(contactId, entityId, relationship)
    } else if (entityType === 'resume') {
      res = await forge.contacts.unlinkResume(contactId, entityId, relationship)
    }

    if (res?.ok) {
      linkedContacts = linkedContacts.filter(c => c.contact_id !== contactId)
      addToast({ type: 'success', message: 'Contact unlinked' })
    } else if (res) {
      addToast({ type: 'error', message: friendlyError(res.error) })
    }
  }

  async function refreshLinkedContacts() {
    let res: any
    if (entityType === 'organization') {
      res = await forge.contacts.listByOrganization(entityId)
    } else if (entityType === 'job_description') {
      res = await forge.contacts.listByJobDescription(entityId)
    } else if (entityType === 'resume') {
      res = await forge.contacts.listByResume(entityId)
    }

    if (res?.ok) {
      linkedContacts = res.data
    }
  }

  function formatRelationship(rel: string): string {
    return rel.replace(/_/g, ' ').replace(/\b\w/g, c => c.toUpperCase())
  }
</script>

<div class="link-section">
  <div class="section-header">
    <span class="section-title">{sectionTitle}</span>
    <button class="btn-add" onclick={openLinkDialog} type="button">
      + Link Contact
    </button>
  </div>

  {#if linkedContacts.length === 0}
    <p class="empty-text">No contacts linked.</p>
  {:else}
    <div class="linked-list">
      {#each linkedContacts as link (link.contact_id + link.relationship)}
        <div class="linked-item">
          <span class="contact-info">
            {link.contact_name}
            {#if link.contact_title}
              <span class="contact-role"> -- {link.contact_title}</span>
            {/if}
          </span>
          <span class="relationship-badge">{formatRelationship(link.relationship)}</span>
          <button
            class="unlink-btn"
            onclick={() => handleUnlink(link.contact_id)}
            type="button"
            aria-label="Unlink {link.contact_name}"
          >&times;</button>
        </div>
      {/each}
    </div>
  {/if}
</div>

{#if showLinkDialog}
  <LinkContactDialog
    title="Link Contact to {sectionTitle}"
    contacts={allContacts}
    {relationships}
    onlink={handleLink}
    oncancel={() => (showLinkDialog = false)}
  />
{/if}

<style>
  .link-section {
    border-top: 1px solid #e5e7eb;
    padding-top: 0.75rem;
    margin-top: 0.5rem;
  }

  .section-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.5rem;
  }

  .section-title {
    font-size: 0.8rem;
    font-weight: 700;
    color: #374151;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .btn-add {
    padding: 0.25rem 0.5rem;
    background: none;
    border: 1px solid #d1d5db;
    border-radius: 0.375rem;
    font-size: 0.75rem;
    color: #3b82f6;
    cursor: pointer;
    font-weight: 500;
  }

  .btn-add:hover {
    background: #f0f9ff;
    border-color: #93c5fd;
  }

  .empty-text {
    font-size: 0.8rem;
    color: #9ca3af;
    font-style: italic;
  }

  .linked-list {
    display: flex;
    flex-direction: column;
    gap: 0.375rem;
  }

  .linked-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.35rem 0.5rem;
    background: #f9fafb;
    border-radius: 0.375rem;
    font-size: 0.85rem;
  }

  .contact-info {
    flex: 1;
    color: #1a1a2e;
    font-weight: 500;
  }

  .contact-role {
    font-weight: 400;
    color: #6b7280;
  }

  .relationship-badge {
    font-size: 0.7rem;
    padding: 0.15em 0.5em;
    background: #e0e7ff;
    color: #3730a3;
    border-radius: 999px;
    font-weight: 500;
    white-space: nowrap;
  }

  .unlink-btn {
    background: none;
    border: none;
    color: #9ca3af;
    cursor: pointer;
    font-size: 1.1rem;
    line-height: 1;
    padding: 0 0.25rem;
  }

  .unlink-btn:hover {
    color: #ef4444;
  }
</style>
```

---

### T50.13: Write ContactEditor Component

**File:** `packages/webui/src/lib/components/contacts/ContactEditor.svelte`

[IMPORTANT] The editor includes relationship sections at the bottom for linked orgs, JDs, and resumes.

```svelte
<!--
  ContactEditor.svelte -- Contact editor form with relationship sections.
-->
<script lang="ts">
  import { forge, friendlyError } from '$lib/sdk'
  import { addToast } from '$lib/stores/toast.svelte'
  import { ConfirmDialog } from '$lib/components'
  import ContactLinkSection from './ContactLinkSection.svelte'
  import type { ContactWithOrg, Organization, ContactLink } from '@forge/sdk'

  const ORG_RELATIONSHIPS = [
    { value: 'recruiter', label: 'Recruiter' },
    { value: 'hr', label: 'HR' },
    { value: 'referral', label: 'Referral' },
    { value: 'peer', label: 'Peer' },
    { value: 'manager', label: 'Manager' },
    { value: 'other', label: 'Other' },
  ]

  const JD_RELATIONSHIPS = [
    { value: 'hiring_manager', label: 'Hiring Manager' },
    { value: 'recruiter', label: 'Recruiter' },
    { value: 'interviewer', label: 'Interviewer' },
    { value: 'referral', label: 'Referral' },
    { value: 'other', label: 'Other' },
  ]

  const RESUME_RELATIONSHIPS = [
    { value: 'reference', label: 'Reference' },
    { value: 'recommender', label: 'Recommender' },
    { value: 'other', label: 'Other' },
  ]

  let {
    contact = null,
    organizations = [],
    createMode = false,
    oncreated,
    onupdated,
    ondeleted,
  }: {
    contact: ContactWithOrg | null
    organizations: Organization[]
    createMode?: boolean
    oncreated: (c: ContactWithOrg) => void
    onupdated: (c: ContactWithOrg) => void
    ondeleted: (id: string) => void
  } = $props()

  // Form state
  let name = $state('')
  let title = $state('')
  let organizationId = $state<string | null>(null)
  let email = $state('')
  let phone = $state('')
  let linkedin = $state('')
  let team = $state('')
  let dept = $state('')
  let notes = $state('')

  let saving = $state(false)
  let confirmDeleteOpen = $state(false)

  // Linked entities for relationship sections
  let linkedOrgs = $state<ContactLink[]>([])
  let linkedJds = $state<ContactLink[]>([])
  let linkedResumes = $state<ContactLink[]>([])

  let isDirty = $derived.by(() => {
    if (createMode || !contact) return false
    return (
      name !== contact.name ||
      title !== (contact.title ?? '') ||
      organizationId !== (contact.organization_id ?? null) ||
      email !== (contact.email ?? '') ||
      phone !== (contact.phone ?? '') ||
      linkedin !== (contact.linkedin ?? '') ||
      team !== (contact.team ?? '') ||
      dept !== (contact.dept ?? '') ||
      notes !== (contact.notes ?? '')
    )
  })

  $effect(() => {
    if (contact && !createMode) {
      name = contact.name
      title = contact.title ?? ''
      organizationId = contact.organization_id ?? null
      email = contact.email ?? ''
      phone = contact.phone ?? ''
      linkedin = contact.linkedin ?? ''
      team = contact.team ?? ''
      dept = contact.dept ?? ''
      notes = contact.notes ?? ''
      loadRelationships(contact.id)
    } else if (createMode) {
      name = ''
      title = ''
      organizationId = null
      email = ''
      phone = ''
      linkedin = ''
      team = ''
      dept = ''
      notes = ''
      linkedOrgs = []
      linkedJds = []
      linkedResumes = []
    }
  })

  async function loadRelationships(contactId: string) {
    const [orgRes, jdRes, resumeRes] = await Promise.all([
      forge.contacts.listByOrganization(contactId),
      forge.contacts.listByJobDescription(contactId),
      forge.contacts.listByResume(contactId),
    ])
    // Note: These are reverse lookups which return contacts linked TO an entity.
    // For the contact editor, we need forward lookups (entities linked FROM this contact).
    // We use the contact-specific relationship endpoints instead.
    const [orgList, jdList, resumeList] = await Promise.all([
      forge.contacts.listOrganizations(contactId),
      forge.contacts.listJobDescriptions(contactId),
      forge.contacts.listResumes(contactId),
    ])
    if (orgList.ok) {
      linkedOrgs = orgList.data.map(o => ({
        contact_id: contactId,
        contact_name: name,
        contact_title: title || null,
        contact_email: email || null,
        relationship: o.relationship,
      }))
    }
    if (jdList.ok) {
      linkedJds = jdList.data.map(j => ({
        contact_id: contactId,
        contact_name: j.title,
        contact_title: j.organization_name,
        contact_email: null,
        relationship: j.relationship,
      }))
    }
    if (resumeList.ok) {
      linkedResumes = resumeList.data.map(r => ({
        contact_id: contactId,
        contact_name: r.name,
        contact_title: null,
        contact_email: null,
        relationship: r.relationship,
      }))
    }
  }

  async function handleSave() {
    if (!name.trim()) {
      addToast({ type: 'error', message: 'Name is required' })
      return
    }

    saving = true
    const payload = {
      name: name.trim(),
      title: title.trim() || null,
      organization_id: organizationId,
      email: email.trim() || null,
      phone: phone.trim() || null,
      linkedin: linkedin.trim() || null,
      team: team.trim() || null,
      dept: dept.trim() || null,
      notes: notes.trim() || null,
    }

    if (createMode) {
      const res = await forge.contacts.create(payload as any)
      if (res.ok) {
        oncreated(res.data)
        addToast({ type: 'success', message: 'Contact created' })
      } else {
        addToast({ type: 'error', message: friendlyError(res.error) })
      }
    } else if (contact) {
      const res = await forge.contacts.update(contact.id, payload)
      if (res.ok) {
        onupdated(res.data)
        addToast({ type: 'success', message: 'Contact updated' })
      } else {
        addToast({ type: 'error', message: friendlyError(res.error) })
      }
    }
    saving = false
  }

  async function handleDelete() {
    if (!contact) return
    const res = await forge.contacts.delete(contact.id)
    if (res.ok) {
      ondeleted(contact.id)
      addToast({ type: 'success', message: 'Contact deleted' })
    } else {
      addToast({ type: 'error', message: friendlyError(res.error) })
    }
    confirmDeleteOpen = false
  }
</script>

<div class="editor">
  <div class="field">
    <label for="contact-name">Name <span class="required">*</span></label>
    <input id="contact-name" type="text" bind:value={name} placeholder="Full name" />
  </div>

  <div class="field">
    <label for="contact-title">Title</label>
    <input id="contact-title" type="text" bind:value={title} placeholder="Job title" />
  </div>

  <div class="field">
    <label for="contact-org">Organization</label>
    <select id="contact-org" bind:value={organizationId}>
      <option value={null}>None</option>
      {#each organizations.sort((a, b) => a.name.localeCompare(b.name)) as org (org.id)}
        <option value={org.id}>{org.name}</option>
      {/each}
    </select>
  </div>

  <div class="field-row">
    <div class="field half">
      <label for="contact-email">Email</label>
      <input id="contact-email" type="email" bind:value={email} placeholder="email@example.com" />
    </div>
    <div class="field half">
      <label for="contact-phone">Phone</label>
      <input id="contact-phone" type="tel" bind:value={phone} placeholder="+1-555-0123" />
    </div>
  </div>

  <div class="field">
    <label for="contact-linkedin">LinkedIn</label>
    <div class="url-field">
      <input id="contact-linkedin" type="url" bind:value={linkedin} placeholder="https://linkedin.com/in/..." />
      {#if linkedin.trim() && !createMode}
        <a href={linkedin} target="_blank" rel="noopener noreferrer" class="url-link">
          Open &#8599;
        </a>
      {/if}
    </div>
  </div>

  <div class="field-row">
    <div class="field half">
      <label for="contact-team">Team</label>
      <input id="contact-team" type="text" bind:value={team} placeholder="Platform Security" />
    </div>
    <div class="field half">
      <label for="contact-dept">Dept</label>
      <input id="contact-dept" type="text" bind:value={dept} placeholder="Engineering" />
    </div>
  </div>

  <div class="field">
    <label for="contact-notes">Notes</label>
    <textarea id="contact-notes" bind:value={notes} placeholder="Your private notes..." rows="4"></textarea>
  </div>

  {#if !createMode && contact}
    <ContactLinkSection
      sectionTitle="Linked Organizations"
      entityType="organization"
      entityId={contact.id}
      relationships={ORG_RELATIONSHIPS}
      bind:linkedContacts={linkedOrgs}
    />

    <ContactLinkSection
      sectionTitle="Linked Job Descriptions"
      entityType="job_description"
      entityId={contact.id}
      relationships={JD_RELATIONSHIPS}
      bind:linkedContacts={linkedJds}
    />

    <ContactLinkSection
      sectionTitle="Linked Resumes"
      entityType="resume"
      entityId={contact.id}
      relationships={RESUME_RELATIONSHIPS}
      bind:linkedContacts={linkedResumes}
    />
  {/if}

  <div class="actions">
    <button
      class="btn-primary"
      onclick={handleSave}
      disabled={saving || (!createMode && !isDirty)}
    >
      {saving ? 'Saving...' : createMode ? 'Create' : 'Save'}
    </button>
    {#if !createMode && contact}
      <button
        class="btn-danger"
        onclick={() => (confirmDeleteOpen = true)}
        type="button"
      >
        Delete
      </button>
    {/if}
  </div>
</div>

{#if confirmDeleteOpen}
  <ConfirmDialog
    title="Delete Contact"
    message="Are you sure you want to delete this contact? This cannot be undone."
    onconfirm={handleDelete}
    oncancel={() => (confirmDeleteOpen = false)}
  />
{/if}

<style>
  .editor {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    padding: 1rem;
    overflow-y: auto;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .field-row {
    display: flex;
    gap: 1rem;
  }

  .half {
    flex: 1;
  }

  label {
    font-size: 0.8rem;
    font-weight: 600;
    color: #374151;
  }

  .required {
    color: #ef4444;
  }

  input[type="text"],
  input[type="email"],
  input[type="tel"],
  input[type="url"],
  select,
  textarea {
    padding: 0.5rem 0.6rem;
    border: 1px solid #d1d5db;
    border-radius: 0.375rem;
    font-size: 0.9rem;
    outline: none;
    font-family: inherit;
  }

  input:focus,
  select:focus,
  textarea:focus {
    border-color: #3b82f6;
    box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.15);
  }

  .url-field {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .url-field input {
    flex: 1;
  }

  .url-link {
    font-size: 0.8rem;
    color: #3b82f6;
    text-decoration: none;
    white-space: nowrap;
  }

  .url-link:hover {
    text-decoration: underline;
  }

  .actions {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding-top: 0.5rem;
    border-top: 1px solid #e5e7eb;
  }

  .btn-primary {
    padding: 0.5rem 1.25rem;
    background: #3b82f6;
    color: #fff;
    border: none;
    border-radius: 0.375rem;
    font-weight: 600;
    cursor: pointer;
    font-size: 0.9rem;
  }

  .btn-primary:hover:not(:disabled) {
    background: #2563eb;
  }

  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-danger {
    padding: 0.5rem 1.25rem;
    background: none;
    color: #ef4444;
    border: 1px solid #ef4444;
    border-radius: 0.375rem;
    font-weight: 600;
    cursor: pointer;
    font-size: 0.9rem;
  }

  .btn-danger:hover {
    background: #fef2f2;
  }
</style>
```

---

### T50.14: Write Contacts Page

**File:** `packages/webui/src/routes/data/contacts/+page.svelte`

```svelte
<!--
  Contacts Page -- split-panel layout with list + editor.
-->
<script lang="ts">
  import { forge, friendlyError } from '$lib/sdk'
  import { addToast } from '$lib/stores/toast.svelte'
  import { LoadingSpinner, EmptyState } from '$lib/components'
  import ContactCard from '$lib/components/contacts/ContactCard.svelte'
  import ContactEditor from '$lib/components/contacts/ContactEditor.svelte'
  import type { ContactWithOrg, Organization } from '@forge/sdk'

  let contacts = $state<ContactWithOrg[]>([])
  let organizations = $state<Organization[]>([])
  let selectedId = $state<string | null>(null)
  let createMode = $state(false)
  let searchText = $state('')
  let loading = $state(true)

  let filteredContacts = $derived.by(() => {
    if (!searchText.trim()) return contacts
    const q = searchText.toLowerCase()
    return contacts.filter(c =>
      c.name.toLowerCase().includes(q) ||
      c.title?.toLowerCase().includes(q) ||
      c.email?.toLowerCase().includes(q)
    )
  })

  let selectedContact = $derived(contacts.find(c => c.id === selectedId) ?? null)

  $effect(() => {
    loadData()
  })

  async function loadData() {
    loading = true
    const [contactRes, orgRes] = await Promise.all([
      forge.contacts.list({ limit: 500 }),
      forge.organizations.list({ limit: 500 }),
    ])

    if (contactRes.ok) {
      contacts = contactRes.data
    } else {
      addToast({ type: 'error', message: friendlyError(contactRes.error) })
    }

    if (orgRes.ok) {
      organizations = orgRes.data
    }
    loading = false
  }

  async function refreshContacts() {
    const res = await forge.contacts.list({ limit: 500 })
    if (res.ok) {
      contacts = res.data
    }
  }

  function selectContact(id: string) {
    createMode = false
    selectedId = id
  }

  function startCreate() {
    selectedId = null
    createMode = true
  }

  function handleCreated(c: ContactWithOrg) {
    createMode = false
    selectedId = c.id
    refreshContacts()
  }

  function handleUpdated(c: ContactWithOrg) {
    refreshContacts()
  }

  function handleDeleted(id: string) {
    selectedId = null
    refreshContacts()
  }
</script>

<div class="contacts-page">
  {#if loading}
    <div class="loading-container">
      <LoadingSpinner />
    </div>
  {:else}
    <div class="split-panel">
      <!-- List Panel -->
      <div class="list-panel">
        <div class="list-header">
          <h2 class="panel-title">Contacts</h2>
          <button class="btn-new" onclick={startCreate} type="button">
            + New Contact
          </button>
        </div>

        <div class="list-filters">
          <input
            type="text"
            class="search-input"
            placeholder="Search name, title, email..."
            bind:value={searchText}
          />
        </div>

        <div class="card-list">
          {#if filteredContacts.length === 0}
            <p class="empty-list">No contacts found.</p>
          {:else}
            {#each filteredContacts as contact (contact.id)}
              <ContactCard
                {contact}
                selected={selectedId === contact.id}
                onclick={() => selectContact(contact.id)}
              />
            {/each}
          {/if}
        </div>
      </div>

      <!-- Editor Panel -->
      <div class="editor-panel">
        {#if createMode}
          <ContactEditor
            contact={null}
            {organizations}
            createMode={true}
            oncreated={handleCreated}
            onupdated={handleUpdated}
            ondeleted={handleDeleted}
          />
        {:else if selectedContact}
          {#key selectedContact.id}
            <ContactEditor
              contact={selectedContact}
              {organizations}
              createMode={false}
              oncreated={handleCreated}
              onupdated={handleUpdated}
              ondeleted={handleDeleted}
            />
          {/key}
        {:else}
          <div class="empty-editor">
            <EmptyState
              title="No contact selected"
              description="Select a contact or create a new one"
            />
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .contacts-page {
    height: 100%;
    display: flex;
    flex-direction: column;
  }

  .loading-container {
    display: flex;
    justify-content: center;
    align-items: center;
    height: 300px;
  }

  .split-panel {
    display: flex;
    height: 100%;
    min-height: 0;
  }

  .list-panel {
    width: 320px;
    min-width: 280px;
    border-right: 1px solid #e5e7eb;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .list-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem 1rem;
    border-bottom: 1px solid #e5e7eb;
  }

  .panel-title {
    font-size: 1rem;
    font-weight: 700;
    color: #1a1a2e;
    margin: 0;
  }

  .btn-new {
    padding: 0.35rem 0.75rem;
    background: #3b82f6;
    color: #fff;
    border: none;
    border-radius: 0.375rem;
    font-size: 0.8rem;
    font-weight: 600;
    cursor: pointer;
  }

  .btn-new:hover {
    background: #2563eb;
  }

  .list-filters {
    padding: 0.5rem 1rem;
    border-bottom: 1px solid #f3f4f6;
  }

  .search-input {
    width: 100%;
    padding: 0.35rem 0.5rem;
    border: 1px solid #d1d5db;
    border-radius: 0.375rem;
    font-size: 0.8rem;
    outline: none;
  }

  .search-input:focus {
    border-color: #3b82f6;
  }

  .card-list {
    flex: 1;
    overflow-y: auto;
    padding: 0.5rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .empty-list {
    text-align: center;
    color: #9ca3af;
    padding: 2rem 1rem;
    font-size: 0.85rem;
  }

  .editor-panel {
    flex: 1;
    overflow-y: auto;
    min-width: 0;
  }

  .empty-editor {
    display: flex;
    justify-content: center;
    align-items: center;
    height: 100%;
  }
</style>
```

---

## Testing Support

### Unit Tests

| Test file | Tests |
|-----------|-------|
| `packages/core/src/db/repositories/__tests__/contact-repository.test.ts` | `create()` inserts contact and returns with org_name |
| | `list()` returns contacts sorted by name ASC |
| | `list()` with search filter matches on name, title, email (case-insensitive) |
| | `list()` with organization_id filter returns only that org's contacts |
| | `get()` returns contact with organization_name |
| | `update()` updates specified fields, leaves others unchanged |
| | `del()` removes contact and cascades to junction tables |
| | `addOrganization()` / `removeOrganization()` / `listOrganizations()` work |
| | `addJobDescription()` / `removeJobDescription()` / `listJobDescriptions()` work |
| | `addResume()` / `removeResume()` / `listResumes()` work |
| | Reverse lookups return correct ContactLink data |
| | Duplicate junction row INSERT OR IGNORE is idempotent |

### Integration Tests (API)

| Test file | Tests |
|-----------|-------|
| `packages/core/src/routes/__tests__/contacts.test.ts` | POST /api/contacts with valid data returns 201 |
| | POST /api/contacts without name returns 400 |
| | GET /api/contacts returns paginated list with organization_name |
| | GET /api/contacts?search=smith filters correctly |
| | GET /api/contacts/:id returns contact with org name |
| | PATCH /api/contacts/:id updates fields |
| | DELETE /api/contacts/:id returns 204 |
| | Relationship endpoints (link/unlink/list) all work |
| | Reverse lookup endpoints return correct data |
| | Invalid relationship type returns 400 |

### Migration Tests

| Test file | Tests |
|-----------|-------|
| `packages/core/src/db/__tests__/migration-019.test.ts` | Migration creates all 4 tables |
| | contacts.id CHECK constraint (UUID format) |
| | organization_id FK ON DELETE SET NULL |
| | Junction table CASCADE deletes |
| | CHECK constraints reject invalid relationship values |
| | Three-column PK allows same contact+entity with different relationships |
| | note_references accepts 'contact' as entity_type |

### Component Tests

| Component | Tests |
|-----------|-------|
| `ContactCard.svelte` | Renders name, title, org, email |
| | Selected state applies correct CSS |
| `ContactEditor.svelte` | Create mode: empty form, "Create" button |
| | Edit mode: populated form, dirty tracking |
| | Delete shows ConfirmDialog |
| | Relationship sections render for linked entities |
| `ContactLinkSection.svelte` | Shows linked contacts with relationship badges |
| | "Link Contact" opens dialog |
| | Unlink removes item |
| `LinkContactDialog.svelte` | Shows contact dropdown and relationship dropdown |
| | Link button disabled until both selected |
| | Cancel closes dialog |

### SDK Tests

| Test file | Tests |
|-----------|-------|
| `packages/sdk/src/__tests__/contacts.test.ts` | All CRUD methods |
| | All relationship methods (link/unlink/list) for orgs, JDs, resumes |
| | Reverse lookup methods |

### Smoke Tests

- Navigate to `/data/contacts` and see split-panel layout.
- Create a contact with name; verify it appears in the list.
- Click a contact card; verify the editor populates.
- Edit fields and save; verify changes persist.
- Delete a contact; verify it is removed from the list.
- Link a contact to an organization via the editor's relationship section.
- Search contacts by name/title/email.

### Contract Tests

- Verify `ContactLink` response shape matches the interface: `contact_id`, `contact_name`, `contact_title`, `contact_email`, `relationship`.
- Verify reverse lookup endpoints return the same shape across all three entity types.

---

## Documentation Requirements

- No new documentation files. The split-panel pattern is consistent with other data pages.
- Update the spec tracker if one exists to mark Spec G as in-progress or complete.

---

## Parallelization Notes

- **T50.1** (types) must complete first -- all other tasks depend on it.
- **T50.2** (migration) can run in parallel with T50.1 (types are for TypeScript, migration is SQL).
- **T50.3** (repository) depends on T50.1 (type imports) and T50.2 (table must exist for tests).
- **T50.4** (service) depends on T50.3 (repository import).
- **T50.5** (routes) depends on T50.4 (service import).
- **T50.6** (reverse lookups) depends on T50.4 (service method calls).
- **T50.7** (registration) depends on T50.4 and T50.5.
- **T50.8** (SDK) depends on T50.1 (type imports). Can run in parallel with T50.3-T50.7.
- **T50.9** (nav) can run in parallel with everything.
- **T50.10** (ContactCard) can run in parallel with T50.11-T50.14.
- **T50.11** (LinkContactDialog) can run in parallel with T50.10.
- **T50.12** (ContactLinkSection) depends on T50.11 (imports LinkContactDialog).
- **T50.13** (ContactEditor) depends on T50.12 (imports ContactLinkSection).
- **T50.14** (Contacts page) depends on T50.10 and T50.13.

Dependency graph:

```
T50.1 (types) ────────────────────────────┐
T50.2 (migration) ─ parallel with T50.1   │
T50.9 (nav) ────── parallel with all      │
                                          │
T50.3 (repository) ─── depends on T50.1, T50.2
T50.4 (service) ─── depends on T50.3
T50.5 (routes) ─── depends on T50.4
T50.6 (reverse lookups) ─── depends on T50.4
T50.7 (registration) ─── depends on T50.4, T50.5

T50.8 (SDK) ─── depends on T50.1 (parallel with T50.3-T50.7)

T50.10 (ContactCard) ─── parallel start
T50.11 (LinkContactDialog) ─── parallel start
T50.12 (ContactLinkSection) ─── depends on T50.11
T50.13 (ContactEditor) ─── depends on T50.12
T50.14 (Contacts page) ─── depends on T50.10, T50.13
```
