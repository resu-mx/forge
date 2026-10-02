# Contacts Support

**Date:** 2026-04-03
**Spec:** G (Contacts)
**Phase:** TBD (next available)
**Builds on:** Organizations (migration 002), Job Descriptions (migration 007, Spec E1), Resumes (migration 001)
**Dependencies:** None (but Spec E1 should exist for JD contact linking to be meaningful)
**Blocks:** None currently

## Overview

Introduce a new `contacts` entity for tracking people at organizations — recruiters, hiring managers, interviewers, references, and peers. Contacts are a first-class entity with their own CRUD page under `/data/contacts` (split-panel layout), and they are referenced as sub-resources from job descriptions, organizations, and resumes via junction tables with typed relationships.

This spec covers:
1. Data model: `contacts` table + three junction tables with relationship types
2. Full CRUD API with relationship management endpoints
3. Split-panel UI at `/data/contacts`
4. Cross-entity reference sections on JD, Org, and Resume detail views
5. SDK resource class

## Non-Goals

- Contact import from LinkedIn, email, or vCard
- Contact deduplication / merge tooling
- Communication tracking (emails sent, calls logged)
- Calendar integration (interview scheduling)
- Contact activity timeline
- Bulk contact operations
- Contact profile photos / avatars
- Social media integration beyond LinkedIn URL
- Contact search by relationship type across entities (e.g., "all hiring managers")

---

## 1. Data Model

### 1.1 `contacts` Table

```sql
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
```

**Column semantics:**

| Column | Purpose | Example |
|--------|---------|---------|
| `name` | Full name | "Jane Smith" |
| `title` | Job title / role | "VP Engineering", "Technical Recruiter" |
| `email` | Email address | "jane@cloudflare.com" |
| `phone` | Phone number (free text) | "+1-555-0123" |
| `linkedin` | LinkedIn profile URL | "https://linkedin.com/in/janesmith" |
| `team` | Team within the organization | "Platform Security" |
| `dept` | Department | "Engineering" |
| `notes` | User's private notes about this contact | "Met at KubeCon, very responsive" |
| `organization_id` | Primary organization FK (nullable) | Links to "Cloudflare" org |

**`organization_id` purpose:** This is the contact's *primary* organization — the one they currently work at. The junction table `contact_organizations` tracks additional/historical org relationships (e.g., a recruiter who works for a staffing firm but recruits for multiple companies). When `organization_id` is set, it provides a default org for display in the list panel.

### 1.2 `contact_organizations` Junction Table

Links contacts to organizations with a typed relationship.

```sql
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
```

The three-column PK `(contact_id, organization_id, relationship)` allows one contact to have multiple relationship types at the same entity (e.g., a contact who is both a `peer` and `referral` at the same org).

**Relationship types:**

| Relationship | Meaning |
|-------------|---------|
| `recruiter` | External or internal recruiter for this org |
| `hr` | HR / people operations contact |
| `referral` | Someone who can refer you at this org |
| `peer` | Colleague or industry peer at this org |
| `manager` | Current or potential manager |
| `other` | Catch-all for unlisted relationships |

### 1.3 `contact_job_descriptions` Junction Table

Links contacts to job descriptions with a typed relationship.

```sql
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
```

The three-column PK `(contact_id, job_description_id, relationship)` allows one contact to have multiple relationship types at the same JD (e.g., a contact who is both a `recruiter` and `interviewer` for the same role).

**Relationship types:**

| Relationship | Meaning |
|-------------|---------|
| `hiring_manager` | The hiring manager for this role |
| `recruiter` | Recruiter handling this position |
| `interviewer` | Someone who will interview you for this role |
| `referral` | Person who referred you to this posting |
| `other` | Catch-all |

### 1.4 `contact_resumes` Junction Table

Links contacts to resumes with a typed relationship.

```sql
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
```

**Relationship types:**

| Relationship | Meaning |
|-------------|---------|
| `reference` | Professional reference listed on or associated with this resume |
| `recommender` | Someone who wrote or can write a recommendation |
| `other` | Catch-all |

---

## 2. Migration

### 2.1 Migration: `019_contacts.sql`

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

**`updated_at` management:** The repository manually sets `updated_at` in the UPDATE query (following the existing pattern used by other repositories in this codebase, e.g., `organization-repository.ts`).

**Migration dependency:** Requires migrations 017 and 018 to exist before migration 019. Migration ordering: 017 (prompt_logs expansion), 018 (job_description_skills), 019 (contacts). Also requires migration 007 (`job_descriptions` table) for the `contact_job_descriptions` FK.

---

## 3. Types

### 3.1 Core Types (`packages/core/src/types/index.ts`)

```typescript
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

### 3.2 SDK Types (`packages/sdk/src/types.ts`)

Mirror all types from 3.1 into the SDK types file.

### 3.3 NoteReferenceEntityType Update

Add `'contact'` to the `NoteReferenceEntityType` union in both core and SDK types:

```typescript
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

---

## 4. API Routes

### 4.1 Contacts CRUD

New file: `packages/core/src/routes/contacts.ts`

| Method | Path | Description | Request Body | Response |
|--------|------|-------------|-------------|----------|
| `POST` | `/api/contacts` | Create a contact | `CreateContact` | `Result<ContactWithOrg>` (201) |
| `GET` | `/api/contacts` | List contacts | Query: `?organization_id=...&search=...&limit=50&offset=0` | `PaginatedResult<ContactWithOrg>` |
| `GET` | `/api/contacts/:id` | Get a contact | — | `Result<ContactWithOrg>` |
| `PATCH` | `/api/contacts/:id` | Update a contact | `UpdateContact` | `Result<ContactWithOrg>` |
| `DELETE` | `/api/contacts/:id` | Delete a contact | — | 204 |

**List endpoint:**
- `search` parameter: case-insensitive substring match on `name`, `title`, and `email`
- `organization_id` parameter: filter by primary org FK
- JOINs `organizations` to include `organization_name`

### 4.2 Contact Relationship Management

| Method | Path | Description | Request Body | Response |
|--------|------|-------------|-------------|----------|
| `GET` | `/api/contacts/:id/organizations` | List orgs linked to this contact | — | `{ data: Array<Organization & { relationship: ContactOrgRelationship }> }` |
| `POST` | `/api/contacts/:id/organizations` | Link contact to org | `{ organization_id, relationship }` | 201 |
| `DELETE` | `/api/contacts/:contactId/organizations/:orgId` | Unlink contact from org | — | 204 |
| `GET` | `/api/contacts/:id/job-descriptions` | List JDs linked to this contact | — | `{ data: Array<JobDescriptionWithOrg & { relationship: ContactJDRelationship }> }` |
| `POST` | `/api/contacts/:id/job-descriptions` | Link contact to JD | `{ job_description_id, relationship }` | 201 |
| `DELETE` | `/api/contacts/:contactId/job-descriptions/:jdId` | Unlink contact from JD | — | 204 |
| `GET` | `/api/contacts/:id/resumes` | List resumes linked to this contact | — | `{ data: Array<Resume & { relationship: ContactResumeRelationship }> }` |
| `POST` | `/api/contacts/:id/resumes` | Link contact to resume | `{ resume_id, relationship }` | 201 |
| `DELETE` | `/api/contacts/:contactId/resumes/:resumeId` | Unlink contact from resume | — | 204 |

### 4.3 Reverse Lookups (for cross-entity reference sections)

These endpoints return contacts linked to a given entity. They are added to the existing route files for each entity.

| Method | Path | Description | Response |
|--------|------|-------------|----------|
| `GET` | `/api/organizations/:id/contacts` | Contacts linked to this org | `{ data: ContactLink[] }` |
| `GET` | `/api/job-descriptions/:id/contacts` | Contacts linked to this JD | `{ data: ContactLink[] }` |
| `GET` | `/api/resumes/:id/contacts` | Contacts linked to this resume | `{ data: ContactLink[] }` |

**`ContactLink` response shape:**
```json
{
  "data": [
    {
      "contact_id": "uuid",
      "contact_name": "Jane Smith",
      "contact_title": "VP Engineering",
      "contact_email": "jane@cloudflare.com",
      "relationship": "hiring_manager"
    }
  ]
}
```

---

## 5. Repository

### 5.1 New File: `packages/core/src/db/repositories/contact-repository.ts`

Follows the same pattern as `organization-repository.ts` and `job-description-repository.ts`:

- `create(input: CreateContact): Contact`
- `list(filter: ContactFilter, offset: number, limit: number): { items: ContactWithOrg[], total: number }`
- `get(id: string): ContactWithOrg | null`
- `update(id: string, input: UpdateContact): ContactWithOrg | null`
- `delete(id: string): boolean`
- `addOrganization(contactId: string, orgId: string, relationship: ContactOrgRelationship): void`
- `removeOrganization(contactId: string, orgId: string): void`
- `listOrganizations(contactId: string): Array<Organization & { relationship: ContactOrgRelationship }>`
- `addJobDescription(contactId: string, jdId: string, relationship: ContactJDRelationship): void`
- `removeJobDescription(contactId: string, jdId: string): void`
- `listJobDescriptions(contactId: string): Array<JobDescriptionWithOrg & { relationship: ContactJDRelationship }>`
- `addResume(contactId: string, resumeId: string, relationship: ContactResumeRelationship): void`
- `removeResume(contactId: string, resumeId: string): void`
- `listResumes(contactId: string): Array<Resume & { relationship: ContactResumeRelationship }>`
- `listByOrganization(orgId: string): ContactLink[]` (reverse lookup)
- `listByJobDescription(jdId: string): ContactLink[]` (reverse lookup)
- `listByResume(resumeId: string): ContactLink[]` (reverse lookup)

### 5.2 New File: `packages/core/src/services/contact-service.ts`

Thin service layer over the repository with validation:
- `name` is required and non-empty
- `email` is validated with a basic format check if provided
- `linkedin` is validated as a URL if provided
- `relationship` values are validated against the allowed CHECK constraint values

---

## 6. UI Layout

### 6.1 Contacts Page (`/data/contacts`)

Split-panel layout consistent with other data pages.

```
┌──────────────────────┬──────────────────────────────────────┐
│ Contacts             │ [Editor Panel]                       │
│ ┌──────────────────┐ │                                      │
│ │ [+ New Contact]  │ │  Name: [________________________]    │
│ │ [🔍 search]      │ │  Title: [________________________]   │
│ ├──────────────────┤ │  Organization: [dropdown________]    │
│ │ ┌──────────────┐ │ │  Email: [________________________]   │
│ │ │ Jane Smith   │ │ │  Phone: [________________________]   │
│ │ │ VP Eng       │ │ │  LinkedIn: [____________________]   │
│ │ │ Cloudflare   │ │ │  Team: [________________________]   │
│ │ │ jane@cf.com  │ │ │  Dept: [________________________]   │
│ │ └──────────────┘ │ │                                      │
│ │ ┌──────────────┐ │ │  Notes:                              │
│ │ │ John Doe     │ │ │  ┌──────────────────────────────┐   │
│ │ │ Recruiter    │ │ │  │ (textarea)                   │   │
│ │ │ Staffing Co  │ │ │  └──────────────────────────────┘   │
│ │ └──────────────┘ │ │                                      │
│ │                  │ │  ── Linked Organizations ──────────   │
│ │ ...              │ │  [Cloudflare (manager) ×]             │
│ └──────────────────┘ │  [+ Link Organization]               │
│                      │                                      │
│                      │  ── Linked Job Descriptions ────────  │
│                      │  [Sr Security Eng @ CF (hiring_mgr) ×]│
│                      │  [+ Link JD]                         │
│                      │                                      │
│                      │  ── Linked Resumes ─────────────────  │
│                      │  [CloudSec Resume (reference) ×]      │
│                      │  [+ Link Resume]                     │
│                      │                                      │
│                      │     [Save]              [Delete]     │
└──────────────────────┴──────────────────────────────────────┘
```

### 6.2 List Panel

**Card content:**
- **Name** (bold, primary text)
- **Title** (muted text, if present)
- **Organization name** (muted text, from primary `organization_id` JOIN)
- **Email** (small, muted text, if present)

**Controls:**
- "New Contact" button
- Search input (filters on name, title, email — server-side via API `?search=` parameter)

**Sorting:** Alphabetical by name.

### 6.3 Editor Panel

**Fields (top section):**

| Field | Input Type | Required | Notes |
|-------|-----------|----------|-------|
| Name | Text input | Yes | Full name |
| Title | Text input | No | Job title |
| Organization | Dropdown (all orgs + "None") | No | Primary org FK. Same pattern as JD org dropdown. |
| Email | Text input (type=email) | No | Email address |
| Phone | Text input (type=tel) | No | Free text phone number |
| LinkedIn | Text input (type=url) | No | LinkedIn profile URL. Rendered as clickable link when populated. |
| Team | Text input | No | Team within org |
| Dept | Text input | No | Department |
| Notes | Textarea | No | Private notes |

**Relationship sections (bottom section):**

Each relationship section has:
- A header label ("Linked Organizations", "Linked Job Descriptions", "Linked Resumes")
- A list of linked entities as pills/rows: `[Entity name (relationship type) ×]`
- A "+ Link" button that opens a linking dialog

**Linking dialog pattern (reusable):**
```
┌─ Link Organization ──────────── × ─┐
│                                      │
│ Organization: [dropdown___________]  │
│ Relationship: [recruiter ▾]         │
│                                      │
│              [Cancel]  [Link]       │
└──────────────────────────────────────┘
```

Each dialog has a dropdown for the target entity and a dropdown for the relationship type (populated from the CHECK constraint values). The target entity dropdown loads from the appropriate list endpoint.

---

## 7. Cross-Entity Reference Sections

These are small read-only + link/unlink sections added to existing entity detail views. They show contacts associated with that entity and allow linking/unlinking.

### 7.1 JD Detail (Spec E1 page)

Add a "Contacts" section to the JD editor panel (below skills, above notes):

```
── Contacts ──────────────────────────
[Jane Smith — Hiring Manager ×]
[John Doe — Recruiter ×]
[+ Link Contact]
```

- Data: `GET /api/job-descriptions/:id/contacts`
- Link: opens dialog with contact dropdown + relationship type dropdown
- Unlink: `DELETE /api/contacts/:contactId/job-descriptions/:jdId`

### 7.2 Organization Detail (existing org page)

Add a "People I Know" section to the org detail view:

```
── People I Know ─────────────────────
[Jane Smith — Manager]
[John Doe — Recruiter]
[+ Link Contact]
```

- Data: `GET /api/organizations/:id/contacts`
- Link/Unlink: same pattern as JD contacts

### 7.3 Resume Detail (existing resume editor)

Add a "References" section to the resume editor:

```
── References ────────────────────────
[Alice Johnson — Reference]
[Bob Lee — Recommender]
[+ Link Contact]
```

- Data: `GET /api/resumes/:id/contacts`
- Link/Unlink: same pattern

**Note:** These cross-entity sections are implemented as a reusable `ContactLinkSection.svelte` component that accepts: the entity type, entity ID, available relationship types, and the link/unlink endpoints.

---

## 8. SDK Resource

### 8.1 New File: `packages/sdk/src/resources/contacts.ts`

```typescript
export class ContactsResource {
  constructor(
    private request: RequestFn,
    private requestList: RequestListFn,
  ) {}

  // CRUD
  create(input: CreateContact): Promise<Result<ContactWithOrg>> { ... }
  list(filter?: ContactFilter & PaginationParams): Promise<PaginatedResult<ContactWithOrg>> { ... }
  get(id: string): Promise<Result<ContactWithOrg>> { ... }
  update(id: string, input: UpdateContact): Promise<Result<ContactWithOrg>> { ... }
  delete(id: string): Promise<Result<void>> { ... }

  // Organization relationships
  listOrganizations(contactId: string): Promise<Result<Array<Organization & { relationship: string }>>> { ... }
  linkOrganization(contactId: string, orgId: string, relationship: ContactOrgRelationship): Promise<Result<void>> { ... }
  unlinkOrganization(contactId: string, orgId: string): Promise<Result<void>> { ... }

  // JD relationships
  listJobDescriptions(contactId: string): Promise<Result<Array<JobDescriptionWithOrg & { relationship: string }>>> { ... }
  linkJobDescription(contactId: string, jdId: string, relationship: ContactJDRelationship): Promise<Result<void>> { ... }
  unlinkJobDescription(contactId: string, jdId: string): Promise<Result<void>> { ... }

  // Resume relationships
  listResumes(contactId: string): Promise<Result<Array<Resume & { relationship: string }>>> { ... }
  linkResume(contactId: string, resumeId: string, relationship: ContactResumeRelationship): Promise<Result<void>> { ... }
  unlinkResume(contactId: string, resumeId: string): Promise<Result<void>> { ... }
}
```

### 8.2 Client Registration

Add to `ForgeClient`:

```typescript
contacts = new ContactsResource(this.request, this.requestList)
```

---

## 9. Navigation

Add "Contacts" entry under the Data section in the sidebar navigation (`packages/webui/src/lib/nav.ts`):

```typescript
{ label: 'Contacts', href: '/data/contacts', icon: 'users' }
```

Position it after "Skills" in the Data section.

---

## 10. Files to Create

| File | Purpose |
|------|---------|
| `packages/core/src/db/migrations/019_contacts.sql` | contacts table + 3 junction tables + note_references rebuild |
| `packages/core/src/db/repositories/contact-repository.ts` | Contact CRUD + relationship management |
| `packages/core/src/services/contact-service.ts` | Validation layer over repository |
| `packages/core/src/routes/contacts.ts` | API route handlers |
| `packages/sdk/src/resources/contacts.ts` | SDK resource class |
| `packages/webui/src/routes/data/contacts/+page.svelte` | Contacts page (split-panel) |
| `packages/webui/src/lib/components/contacts/ContactCard.svelte` | Contact card for list panel |
| `packages/webui/src/lib/components/contacts/ContactEditor.svelte` | Contact editor form |
| `packages/webui/src/lib/components/contacts/ContactLinkSection.svelte` | Reusable cross-entity contact linking section |
| `packages/webui/src/lib/components/contacts/LinkContactDialog.svelte` | Dialog for linking a contact to an entity |

## 11. Files to Modify

| File | Change |
|------|--------|
| `packages/core/src/types/index.ts` | Add `Contact`, `ContactWithOrg`, `CreateContact`, `UpdateContact`, relationship types, `ContactLink`, `ContactFilter`. Add `'contact'` to `NoteReferenceEntityType`. |
| `packages/sdk/src/types.ts` | Mirror type additions from core |
| `packages/sdk/src/client.ts` | Register `ContactsResource` on `ForgeClient` |
| `packages/core/src/services/index.ts` | Register `ContactService` in the `Services` object |
| `packages/core/src/routes/index.ts` (or server setup) | Mount contact routes at `/api/contacts` |
| `packages/core/src/routes/organizations.ts` | Add `GET /api/organizations/:id/contacts` reverse lookup endpoint |
| `packages/core/src/routes/job-descriptions.ts` | Add `GET /api/job-descriptions/:id/contacts` reverse lookup endpoint |
| `packages/core/src/routes/resumes.ts` | Add `GET /api/resumes/:id/contacts` reverse lookup endpoint |
| `packages/webui/src/lib/nav.ts` | Add "Contacts" nav entry under Data |
| `packages/webui/src/routes/opportunities/job-descriptions/+page.svelte` | Add ContactLinkSection to JD editor (after Spec E1 lands) |
| `packages/webui/src/routes/opportunities/organizations/+page.svelte` | Add ContactLinkSection ("People I Know") to org detail view |

---

## 12. Testing

### 12.1 Migration Tests

- Migration 019 creates `contacts`, `contact_organizations`, `contact_job_descriptions`, `contact_resumes` tables
- `INSERT INTO contacts` with valid UUID and name succeeds
- `organization_id` FK to organizations works; deleting org sets `organization_id = NULL`
- Junction table PRIMARY KEYs prevent duplicate links
- Deleting a contact cascades to all three junction tables
- Deleting an org cascades to `contact_organizations` rows
- Deleting a JD cascades to `contact_job_descriptions` rows
- Deleting a resume cascades to `contact_resumes` rows
- CHECK constraints on relationship columns reject invalid values
- `note_references` accepts `'contact'` as `entity_type`

### 12.2 Repository Tests

- `create()` inserts a contact and returns it
- `list()` returns contacts with `organization_name` JOIN
- `list()` with search filter matches on name, title, email (case-insensitive)
- `list()` with `organization_id` filter returns only that org's contacts
- `get()` returns contact with `organization_name`
- `update()` updates specified fields, leaves others unchanged
- `delete()` removes contact and cascades to junction tables
- `addOrganization()` / `removeOrganization()` / `listOrganizations()` work correctly
- `addJobDescription()` / `removeJobDescription()` / `listJobDescriptions()` work correctly
- `addResume()` / `removeResume()` / `listResumes()` work correctly
- Reverse lookups (`listByOrganization`, `listByJobDescription`, `listByResume`) return correct `ContactLink` data
- Duplicate junction row insertion is rejected (PRIMARY KEY)

### 12.3 API Tests

- `POST /api/contacts` with valid data returns 201
- `POST /api/contacts` without `name` returns 400
- `GET /api/contacts` returns paginated list with `organization_name`
- `GET /api/contacts?search=smith` filters correctly
- `GET /api/contacts/:id` returns contact with org name
- `PATCH /api/contacts/:id` updates fields
- `DELETE /api/contacts/:id` returns 204
- Relationship endpoints (link/unlink/list for orgs, JDs, resumes) all work
- Reverse lookup endpoints return correct data
- Invalid relationship type returns 400

### 12.4 UI Component Tests

- Split-panel renders with contact list on left, editor on right
- "New Contact" opens editor in create mode
- Creating a contact adds it to the list
- Clicking a contact card populates the editor
- Editing fields and saving updates the contact
- Delete shows ConfirmDialog; confirming removes contact
- Search filters contacts by name/title/email
- Organization dropdown shows all orgs + "None"
- Relationship sections show linked entities with relationship type
- "Link" button opens linking dialog with entity + relationship dropdowns
- × button on linked entity removes the link

### 12.5 SDK Tests

- All CRUD methods work
- All relationship methods (link/unlink/list) work for orgs, JDs, and resumes

---

## 13. Acceptance Criteria

1. Migration 019 creates `contacts` table with all columns and correct constraints
2. Three junction tables (`contact_organizations`, `contact_job_descriptions`, `contact_resumes`) exist with typed relationship CHECK constraints
3. All CASCADE deletes work: deleting a contact cascades to junctions; deleting an org/JD/resume cascades to relevant junction rows
4. `ON DELETE SET NULL` on `contacts.organization_id` works correctly when the linked org is deleted
5. Contact CRUD API (`POST/GET/PATCH/DELETE /api/contacts`) works with `organization_name` JOIN
6. List endpoint supports `search` and `organization_id` filter parameters
7. Relationship management endpoints (link/unlink/list) work for all three entity types
8. Reverse lookup endpoints on orgs, JDs, and resumes return correct `ContactLink` data
9. Invalid relationship types are rejected with 400
10. SDK `ContactsResource` has all CRUD and relationship methods
11. `/data/contacts` renders a split-panel with contact list and editor
12. Contact editor form handles all fields: name, title, org dropdown, email, phone, linkedin, team, dept, notes
13. Editor shows relationship sections for linked orgs, JDs, and resumes with link/unlink capability
14. Linking dialog shows entity dropdown + relationship type dropdown
15. Cross-entity reference sections appear on JD detail (Contacts), org detail (People I Know), and resume detail (References)
16. `ContactLinkSection.svelte` is reusable across all three entity views
17. "Contacts" appears in the Data section of the sidebar navigation
18. `NoteReferenceEntityType` includes `'contact'` in both core and SDK types
19. `note_references` table accepts `entity_type = 'contact'` after migration
