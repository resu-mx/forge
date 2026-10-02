# Config -- Profile

**Date:** 2026-03-30
**Status:** Design
**Builds on:** Resume Sections as Entities (Phases 27-28), Resume IR Compiler

## Purpose

Create a global user profile that stores contact information (name, email, phone, location, links, clearance) in a dedicated `user_profile` table. The IR compiler currently reads contact data from the `resumes.header` JSON blob, meaning every resume stores its own copy of the same contact info. This is redundant for a single-user application and creates drift when the user's phone number or clearance changes -- they'd have to update every resume individually.

The profile is the single source of truth for contact fields. The IR compiler reads from `user_profile` for contact info and from the resume-specific `header` JSON for content-level fields only (tagline, summary title). Profile is edited at `/config/profile`.

## Goals

1. `user_profile` table with contact fields
2. IR compiler reads contact info from profile instead of `resumes.header`
3. Profile editor at `/config/profile`
4. Migration that seeds profile from existing resume header data
5. API endpoints: `GET /api/profile`, `PATCH /api/profile`

## Non-Goals

- Multi-user support (this is a single-user app)
- Profile photo/avatar
- Multiple profiles or profile variants
- Summary/tagline per profile (those stay resume-specific)

---

## 1. Schema (Migration 005)

**File:** `packages/core/src/db/migrations/005_user_profile.sql`

```sql
-- Forge Resume Builder -- User Profile
-- Migration: 005_user_profile
-- Date: 2026-03-30
--
-- Creates a user_profile table for global contact information.
-- Seeds the profile from the first resume's header JSON (if any).
-- Builds on 004_resume_sections.

-- Step 1: Create user_profile table
CREATE TABLE user_profile (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  name TEXT NOT NULL,
  email TEXT,
  phone TEXT,
  location TEXT,
  linkedin TEXT,
  github TEXT,
  website TEXT,
  clearance TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

-- Step 2: Seed profile from first resume with a non-null header
-- Uses json_extract to pull fields from the header JSON blob.
-- If no resume exists, inserts a placeholder profile with name 'User'.
-- Assumption: resumes.name is NOT NULL (enforced by 001_initial schema).
-- The COALESCE chain falls through: header JSON name → resumes.name → 'User'.
INSERT INTO user_profile (id, name, email, phone, location, linkedin, github, website, clearance)
SELECT
  lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)),2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)),2) || '-' || hex(randomblob(6))),
  COALESCE(json_extract(header, '$.name'), resumes.name, 'User'),
  json_extract(header, '$.email'),
  json_extract(header, '$.phone'),
  json_extract(header, '$.location'),
  json_extract(header, '$.linkedin'),
  json_extract(header, '$.github'),
  json_extract(header, '$.website'),
  json_extract(header, '$.clearance')
FROM resumes
WHERE header IS NOT NULL
ORDER BY created_at ASC
LIMIT 1;

-- If no resume existed, insert a placeholder
INSERT OR IGNORE INTO user_profile (id, name)
SELECT
  lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)),2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)),2) || '-' || hex(randomblob(6))),
  'User'
WHERE NOT EXISTS (SELECT 1 FROM user_profile);

-- Step 3: Register migration
INSERT INTO _migrations (name) VALUES ('005_user_profile');
```

**Design notes:**
- Single row enforced at the app level, not the DB level. The table could hold multiple rows in theory, but the repository/service always operates on the one row.
- `INSERT OR IGNORE` handles the case where the first `INSERT` already created a row.
- The `id` is a random UUID (not a fixed constant), generated at migration time.
- **No `updated_at` trigger.** The repository is the sole update path for `user_profile`. No trigger is needed because all writes go through `ProfileRepository.update()` which explicitly sets `updated_at`.

## 2. Repository: `profile-repository.ts`

**File:** `packages/core/src/db/repositories/profile-repository.ts`

**Note:** `UserProfile` and `UpdateProfile` are defined in `packages/core/src/types/index.ts` (see Files to Modify) and imported here. Do not duplicate the type definitions in the repository file.

```typescript
import type { Database } from 'bun:sqlite'
import type { UserProfile, UpdateProfile } from '../types'

/** Get the single user profile row. Returns null only if migration hasn't run. */
export function get(db: Database): UserProfile | null {
  return db.query('SELECT * FROM user_profile LIMIT 1').get() as UserProfile | null
}

const ALLOWED_FIELDS = ['name', 'email', 'phone', 'location', 'linkedin', 'github', 'website', 'clearance']

/** Patch the profile. Only provided fields are updated. */
export function update(db: Database, patch: UpdateProfile): UserProfile | null {
  const profile = get(db)
  if (!profile) return null

  const fields: string[] = []
  const values: unknown[] = []

  for (const [key, value] of Object.entries(patch)) {
    if (!ALLOWED_FIELDS.includes(key)) continue  // skip unknown fields
    fields.push(`${key} = ?`)
    values.push(value)
  }

  if (fields.length === 0) return profile

  fields.push("updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')")
  values.push(profile.id)

  db.run(`UPDATE user_profile SET ${fields.join(', ')} WHERE id = ?`, values)
  return get(db)
}
```

## 2.1 Service: `profile-service.ts`

**File:** `packages/core/src/services/profile-service.ts`

```typescript
import type { Database } from 'bun:sqlite'
import type { UserProfile, UpdateProfile } from '../types'
import type { Result } from '../types'
import * as ProfileRepository from '../db/repositories/profile-repository'

export class ProfileService {
  constructor(private db: Database) {}

  getProfile(): Result<UserProfile> {
    const profile = ProfileRepository.get(this.db)
    if (!profile) return { ok: false, error: { code: 'NOT_FOUND', message: 'Profile not found' } }
    return { ok: true, data: profile }
  }

  updateProfile(patch: UpdateProfile): Result<UserProfile> {
    if (patch.name !== undefined && patch.name.trim() === '') {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: 'Name must not be empty' } }
    }
    const updated = ProfileRepository.update(this.db, patch)
    if (!updated) return { ok: false, error: { code: 'NOT_FOUND', message: 'Profile not found' } }
    return { ok: true, data: updated }
  }
}
```

## 3. IR Compiler Changes

**File:** `packages/core/src/services/resume-compiler.ts`

The `parseHeader()` function currently reads from the resume's `header` JSON blob:

```typescript
// CURRENT:
function parseHeader(resume: ResumeRow): ResumeHeader {
  if (resume.header) {
    try { return JSON.parse(resume.header) as ResumeHeader } catch { /* fall through */ }
  }
  return { name: resume.name, tagline: resume.target_role, ... }
}
```

**Change:** The compiler function receives the profile and merges contact fields from the profile with content fields from the resume header:

```typescript
// NEW:
function parseHeader(resume: ResumeRow, profile: UserProfile | null): ResumeHeader {
  // Content fields from the resume-specific header JSON
  let tagline: string | null = resume.target_role
  if (resume.header) {
    try {
      const parsed = JSON.parse(resume.header)
      tagline = parsed.tagline ?? tagline
    } catch { /* fall through */ }
  }

  // Contact fields from profile (single source of truth)
  return {
    name: profile?.name ?? resume.name,
    tagline,
    location: profile?.location ?? null,
    email: profile?.email ?? null,
    phone: profile?.phone ?? null,
    linkedin: profile?.linkedin ?? null,
    github: profile?.github ?? null,
    website: profile?.website ?? null,
    clearance: profile?.clearance ?? null,
  }
}
```

The `compileResumeIR()` function adds a profile lookup:

```typescript
export function compileResumeIR(db: Database, resumeId: string): ResumeDocument | null {
  const resume = db.query('SELECT id, name, target_role, header FROM resumes WHERE id = ?')
    .get(resumeId) as ResumeRow | null
  if (!resume) return null

  const profile = db.query('SELECT * FROM user_profile LIMIT 1').get() as UserProfile | null
  const header = parseHeader(resume, profile)
  // Replace only the `parseHeader` call and profile query (lines ~80-108 of resume-compiler.ts).
  // Keep all section-building logic below unchanged.
}
```

**The `ResumeHeader` type does not change.** It still has all the same fields. The only change is where the data comes from at compile time.

### 3.1 `parseHeader()` Evolution — Final Target State

This spec adds the `profile` parameter. Spec 2 (Summaries) later adds the `summary` parameter. The intermediate state after only this spec: `parseHeader(resume, profile)` -- tagline comes from `resume.target_role`.

```typescript
// Final state after both Profile (005) and Summaries (006) are implemented:
function parseHeader(
  resume: ResumeRow,
  profile: UserProfile | null,
  summary: SummaryRow | null
): ResumeHeader {
  return {
    name: profile?.name ?? resume.name,
    tagline: summary?.tagline ?? resume.target_role,
    location: profile?.location ?? null,
    email: profile?.email ?? null,
    phone: profile?.phone ?? null,
    linkedin: profile?.linkedin ?? null,
    github: profile?.github ?? null,
    website: profile?.website ?? null,
    clearance: profile?.clearance ?? null,
  }
}
```

## 4. `ResumeHeader` Field Ownership

After this change, the `ResumeHeader` interface fields have two sources:

| Field | Source | Edited at |
|-------|--------|-----------|
| `name` | `user_profile.name` | `/config/profile` |
| `email` | `user_profile.email` | `/config/profile` |
| `phone` | `user_profile.phone` | `/config/profile` |
| `location` | `user_profile.location` | `/config/profile` |
| `linkedin` | `user_profile.linkedin` | `/config/profile` |
| `github` | `user_profile.github` | `/config/profile` |
| `website` | `user_profile.website` | `/config/profile` |
| `clearance` | `user_profile.clearance` | `/config/profile` |
| `tagline` | `resumes.header` JSON | Resume header editor |

This means the `HeaderEditor.svelte` component in the resume builder only edits `tagline` (the role/summary line). All contact fields are read-only in the resume builder and editable only at `/config/profile`.

## 5. API Endpoints

Added to the existing API router:

### `GET /api/profile`

Returns the user profile.

**Response:** `{ ok: true, data: UserProfile }`

### `PATCH /api/profile`

Updates profile fields. Only provided fields are modified.

**Request body:**
```json
{
  "name": "Adam",
  "email": "adam@example.com",
  "phone": "+1-555-0123",
  "location": "Washington, DC",
  "linkedin": "linkedin.com/in/adam",
  "github": "github.com/adam",
  "website": "adam.dev",
  "clearance": "TS/SCI with CI Polygraph - Active"
}
```

**Response:** `{ ok: true, data: UserProfile }`

**Validation:**
- `name` must be a non-empty string if provided. Validation errors return `{ code: 'VALIDATION_ERROR' }` which maps to HTTP 400 via `mapStatusCode()`.
- All other fields are nullable strings with no format validation (the user knows their own contact info)

**No POST or DELETE.** The profile always exists (created at migration time). There is no creation or deletion flow.

## 6. SDK

### 6.1 SDK Types (`packages/sdk/src/types.ts`)

Add `UserProfile` and `UpdateProfile` types to the SDK types file:

```typescript
export interface UserProfile {
  id: string
  name: string
  email: string | null
  phone: string | null
  location: string | null
  linkedin: string | null
  github: string | null
  website: string | null
  clearance: string | null
  created_at: string
  updated_at: string
}

export type UpdateProfile = Partial<Omit<UserProfile, 'id' | 'created_at' | 'updated_at'>>
```

### 6.2 SDK Resource (`packages/sdk/src/resources/profile.ts`)

Follow the same resource class pattern used by other SDK resources:

```typescript
import type { UserProfile, UpdateProfile, ApiResponse } from '../types'

type RequestFn = <T>(method: string, path: string, body?: unknown) => Promise<ApiResponse<T>>

export class ProfileResource {
  constructor(private request: RequestFn) {}

  get(): Promise<ApiResponse<UserProfile>> {
    return this.request<UserProfile>('GET', '/api/profile')
  }

  update(data: UpdateProfile): Promise<ApiResponse<UserProfile>> {
    return this.request<UserProfile>('PATCH', '/api/profile', data)
  }
}
```

### 6.3 SDK Client (`packages/sdk/src/client.ts`)

Import and assign `ProfileResource` in the client constructor:

```typescript
import { ProfileResource } from './resources/profile'

// In the ForgeSDK class constructor:
this.profile = new ProfileResource(this.request.bind(this))
```

## 7. UI: `/config/profile`

**Files:**
- `packages/webui/src/routes/config/+layout.svelte` -- Config section pass-through layout (no sub-nav)
- `packages/webui/src/routes/config/profile/+page.svelte` -- Profile editor

### 7.1 Config Layout

The Config section uses the sidebar accordion from Spec 1 for navigation between Profile/Templates/Export/Debug. There is NO separate config sub-layout with its own horizontal nav -- that would be redundant with the sidebar.

The config layout is a pass-through layout with no additional nav:

```svelte
<script>
  let { children } = $props()
</script>

{@render children()}
```

### 7.2 Profile Editor

A form with labeled text inputs for each field. On change, the form auto-saves via `PATCH /api/profile` (debounced 500ms) or via an explicit "Save" button.

**Fields displayed:**
- Name (required, text input)
- Email (text input)
- Phone (text input)
- Location (text input, e.g. "Washington, DC")
- LinkedIn (text input, URL or username)
- GitHub (text input, URL or username)
- Website (text input, URL)
- Security Clearance (text input, e.g. "TS/SCI with CI Polygraph - Active")

### 7.3 Navigation Update

Config group is already added by Spec 1 (Nav Restructuring). No changes to the top-level nav needed.

## 8. HeaderEditor Impact

**File:** `packages/webui/src/lib/components/resume/HeaderEditor.svelte`

The `HeaderEditor` component currently shows all header fields (name, email, phone, etc.) as editable. After this change:

- Contact fields become read-only display with a "Edit in Profile" link
- Only `tagline` remains editable inline
- The header still renders all fields in the resume preview (data comes from the compiled IR which merges profile + resume header)

## Acceptance Criteria

1. **`user_profile` table** exists after migration 005 with correct schema
2. **Migration seeds profile** from first resume's header JSON (if any)
3. **Placeholder profile** created with `name = 'User'` when no resumes exist
4. **`GET /api/profile`** returns the single profile row
5. **`PATCH /api/profile`** updates only provided fields, returns updated profile
6. **`PATCH /api/profile` with `name: ''`** returns 400
7. **IR compiler** uses profile contact info for name/email/phone/etc.
8. **IR compiler** still uses resume-specific `tagline` from header JSON
9. **Profile editor** at `/config/profile` loads and saves correctly
10. **HeaderEditor** shows "Edit in Profile" link for contact fields
11. **Fresh DB migration** creates placeholder profile with `name = 'User'`
12. **Column allowlist** in `update()` prevents writing to `id`, `created_at`, `updated_at`
13. **`updated_at` is refreshed** when profile is updated via PATCH

---

## Files to Create

| File | Description |
|------|-------------|
| `packages/core/src/db/migrations/005_user_profile.sql` | Schema migration |
| `packages/core/src/db/repositories/profile-repository.ts` | CRUD repository (imports types from `types/index.ts`) |
| `packages/core/src/services/profile-service.ts` | `ProfileService` with `getProfile()` and `updateProfile()` methods. Register in `createServices()` factory. |
| `packages/core/src/routes/profile.ts` | API route handler for `GET /api/profile` and `PATCH /api/profile`. Register in `server.ts`. |
| `packages/sdk/src/resources/profile.ts` | `ProfileResource` class following SDK resource pattern |
| `packages/webui/src/routes/config/+layout.svelte` | Config section pass-through layout (no sub-nav) |
| `packages/webui/src/routes/config/profile/+page.svelte` | Profile editor page |

## Files to Modify

| File | Change |
|------|--------|
| `packages/core/src/services/resume-compiler.ts` | `parseHeader()` reads from profile for contact fields. Add `UserProfile` to the import list from `../types`. |
| `packages/core/src/types/index.ts` | Add `UserProfile`, `UpdateProfile` types (single source of truth -- repository imports from here) |
| `packages/core/src/services/index.ts` | Add `profile: ProfileService` to the `Services` interface. Add `profile: new ProfileService(db)` to `createServices()`. Export `ProfileService`. |
| `packages/core/src/routes/server.ts` | Add `app.route('/', profileRoutes(services))` to `createApp()`. Import `profileRoutes` from `./profile`. |
| `packages/sdk/src/types.ts` | Add `UserProfile`, `UpdateProfile` types for SDK consumers |
| `packages/sdk/src/index.ts` | Re-export `UserProfile` and `UpdateProfile` types. Export `ProfileResource` class. |
| `packages/webui/src/lib/components/resume/HeaderEditor.svelte` | Contact fields become read-only with "Edit in Profile" link |

## Testing

- Verify migration seeds profile from existing resume header data
- Verify migration creates placeholder profile when no resumes exist
- Migrate fresh DB with no resumes -- placeholder profile created
- Migrate DB with resume `header = null` -- placeholder profile created
- Verify `GET /api/profile` returns the profile
- Verify `PATCH /api/profile` updates only the provided fields
- `PATCH /api/profile` with `name: ''` -- 400
- Verify `PATCH /api/profile` with empty body `{}` returns 200 with unchanged profile
- Verify the IR compiler uses profile contact info (not resume header) for name/email/phone/etc.
- Verify the IR compiler still uses resume-specific `tagline` from the header JSON
- Compile IR before and after profile update -- verify contact info changes
- Verify `updated_at` is refreshed when profile is updated via PATCH
- Verify the profile editor at `/config/profile` loads and saves correctly
- Verify the `HeaderEditor` component shows contact fields as read-only
