# Phase 29: Config Profile (Migration 005 + Core + API + SDK + UI)

**Status:** Planning
**Date:** 2026-03-31
**Spec:** [2026-03-30-config-profile.md](../refs/specs/2026-03-30-config-profile.md)
**Depends on:** Phase 28 (resume sections — stable baseline)
**Blocks:** Phase 30 (Summaries Entity), Phase 35 (Config Export)
**Parallelizable with:** Phase 31 (JD), Phase 32 (Nav), Phase 36 (Resume Templates)

## Goal

Create a global `user_profile` table that stores contact information (name, email, phone, location, links, clearance) as a single source of truth for a single-user application. The IR compiler switches from reading contact data out of each resume's `header` JSON blob to reading it from `user_profile`, eliminating redundant copies and preventing drift. A profile editor at `/config/profile` lets the user manage their contact info, and the `HeaderEditor` component in the resume builder is updated to show contact fields as read-only with an "Edit in Profile" link. The full vertical slice includes migration 005, repository, service, API routes, SDK resource, and Svelte UI.

## Non-Goals

- Multi-user support (single-user app)
- Profile photo / avatar
- Multiple profiles or profile variants
- Summary / tagline per profile (those stay resume-specific)
- Config sub-navigation layout (handled by Phase 32 Nav Restructuring)

## Context

The resume `header` JSON blob currently holds both content-level fields (tagline) and contact-level fields (name, email, phone, etc.). Every resume stores its own copy of the same contact info. When the user's phone number or clearance changes, they'd have to update every resume individually. The profile table is the single source of truth for contact fields, eliminating this duplication.

## Scope

| Spec section | Covered here? |
|-------------|---------------|
| 1. Schema (migration 005) | Yes |
| 2. Repository (`profile-repository.ts`) | Yes |
| 2.1 Service (`profile-service.ts`) | Yes |
| 3. IR compiler changes | Yes |
| 4. ResumeHeader field ownership | Yes |
| 5. API endpoints | Yes |
| 6. SDK resource | Yes |
| 7. UI: `/config/profile` | Yes |
| 8. HeaderEditor impact | Yes |

## Files to Create

| File | Description |
|------|-------------|
| `packages/core/src/db/migrations/005_user_profile.sql` | Schema migration |
| `packages/core/src/db/repositories/profile-repository.ts` | Repository with `get()` and `update()` |
| `packages/core/src/services/profile-service.ts` | `ProfileService` with `getProfile()` and `updateProfile()` |
| `packages/core/src/routes/profile.ts` | API route handler for `GET /api/profile` and `PATCH /api/profile` |
| `packages/sdk/src/resources/profile.ts` | `ProfileResource` class |
| `packages/webui/src/routes/config/+layout.svelte` | Config section pass-through layout |
| `packages/webui/src/routes/config/profile/+page.svelte` | Profile editor page |
| `packages/core/src/db/repositories/__tests__/profile-repository.test.ts` | Repository unit tests |
| `packages/core/src/services/__tests__/profile-service.test.ts` | Service unit tests |
| `packages/core/src/routes/__tests__/profile.test.ts` | API integration tests |

## Files to Modify

| File | Change |
|------|--------|
| `packages/core/src/types/index.ts` | Add `UserProfile`, `UpdateProfile` types |
| `packages/core/src/services/index.ts` | Add `profile: ProfileService` to `Services` interface and `createServices()` |
| `packages/core/src/routes/server.ts` | Mount `profileRoutes` |
| `packages/core/src/services/resume-compiler.ts` | `parseHeader()` reads contact fields from profile |
| `packages/sdk/src/types.ts` | Add `UserProfile`, `UpdateProfile` types |
| `packages/sdk/src/client.ts` | Add `profile: ProfileResource` to `ForgeClient` |
| `packages/sdk/src/index.ts` | Re-export `UserProfile`, `UpdateProfile`, `ProfileResource` |
| `packages/webui/src/lib/components/resume/HeaderEditor.svelte` | Contact fields become read-only with "Edit in Profile" link |
| `packages/webui/src/routes/+layout.svelte` | Add Config nav item |
| `packages/core/src/db/__tests__/helpers.ts` | Add `seedProfile()` helper |
| `packages/core/src/services/__tests__/resume-compiler.test.ts` | Update tests to account for profile-based contact fields |
| `packages/core/src/routes/__tests__/contracts.test.ts` | Add profile contract test |

## Fallback Strategies

- **Migration seed fails (no resumes exist):** The migration includes an `INSERT OR IGNORE` fallback that creates a placeholder profile with `name = 'User'`. This is tested explicitly.
- **Profile row somehow deleted:** `ProfileService.getProfile()` returns `NOT_FOUND`. The IR compiler falls back to resume-level header data when `profile` is null. The UI shows an error toast.
- **`header` JSON parsing fails:** The compiler already has a try/catch; with the profile change, only `tagline` comes from the header JSON, so a parse failure just means `tagline` defaults to `resume.target_role`.

---

## Tasks

### T29.1: Write Migration `005_user_profile.sql`

**File:** `packages/core/src/db/migrations/005_user_profile.sql`

Creates the `user_profile` table and seeds it from the first resume's header JSON (or creates a placeholder).

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
-- The COALESCE chain falls through: header JSON name -> resumes.name -> 'User'.
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

-- Step 3: If no resume existed, insert a placeholder
INSERT OR IGNORE INTO user_profile (id, name)
SELECT
  lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)),2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)),2) || '-' || hex(randomblob(6))),
  'User'
WHERE NOT EXISTS (SELECT 1 FROM user_profile);

-- Step 4: Register migration
INSERT INTO _migrations (name) VALUES ('005_user_profile');
```

**Key points:**
- Single row enforced at the app level, not DB level. The repository always operates on one row.
- `INSERT OR IGNORE` in step 3 handles the case where step 2 already created a row.
- The `id` is a random UUID generated at migration time.
- No `updated_at` trigger needed — the repository sets it explicitly on every update.

**Acceptance criteria:**
- After migration, `SELECT COUNT(*) FROM user_profile` returns exactly 1.
- If a resume with `header = '{"name":"Adam","email":"adam@test.com"}'` exists, the profile row has `name = 'Adam'` and `email = 'adam@test.com'`.
- If no resumes exist, the profile row has `name = 'User'` and all other fields are NULL.
- The `_migrations` table contains `005_user_profile`.

**Failure criteria:**
- Migration fails with a SQL error.
- Zero rows in `user_profile` after migration.
- More than one row in `user_profile` after migration.

---

### T29.2: Add Core Types (`UserProfile`, `UpdateProfile`)

#### `packages/core/src/types/index.ts`

Add after the `Archetype`/`ArchetypeDomain` block (before the `// -- Input Types` section):

```typescript
// ── User Profile ──────────────────────────────────────────────────────

/** Global user profile — single source of truth for contact information. */
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

/** Input for partially updating the user profile. */
export type UpdateProfile = Partial<Omit<UserProfile, 'id' | 'created_at' | 'updated_at'>>
```

**Acceptance criteria:**
- `UserProfile` and `UpdateProfile` are importable from `../types` in both repository and service files.
- The interface matches the `user_profile` table schema column-for-column.

**Failure criteria:**
- TypeScript compilation fails when importing these types.
- A field name mismatch between the type and the DB column causes runtime data loss.

---

### T29.3: Write Profile Repository

**File:** `packages/core/src/db/repositories/profile-repository.ts`

**Note:** The spec shows `import from '../types'` for the repository -- the correct path is `'../../types'` since the file is at `packages/core/src/db/repositories/profile-repository.ts`.

```typescript
/**
 * ProfileRepository — data access for the user_profile table.
 *
 * Single-row table: get() returns the one profile, update() patches it.
 */

import type { Database } from 'bun:sqlite'
import type { UserProfile, UpdateProfile } from '../../types'

/** Get the single user profile row. Returns null only if migration hasn't run. */
export function get(db: Database): UserProfile | null {
  return db.query('SELECT * FROM user_profile LIMIT 1').get() as UserProfile | null
}

const ALLOWED_FIELDS = ['name', 'email', 'phone', 'location', 'linkedin', 'github', 'website', 'clearance']

/** Patch the profile. Only provided fields are updated. Returns updated profile or null if not found. */
export function update(db: Database, patch: UpdateProfile): UserProfile | null {
  const profile = get(db)
  if (!profile) return null

  const fields: string[] = []
  const values: unknown[] = []

  for (const [key, value] of Object.entries(patch)) {
    if (!ALLOWED_FIELDS.includes(key)) continue
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

**Expected inputs/outputs:**

- `get(db)` with a migrated DB returns `{ id: '...', name: 'User', email: null, ... }`.
- `update(db, { name: 'Adam', email: 'adam@test.com' })` returns the updated profile with `name = 'Adam'`, `email = 'adam@test.com'`, and `updated_at` refreshed.
- `update(db, {})` returns the unchanged profile (no-op).
- `update(db, { id: 'hacked', name: 'Evil' })` ignores the `id` field and only updates `name`.

**Acceptance criteria:**
- `get()` returns a single `UserProfile` object after migration.
- `update()` modifies only the specified fields and refreshes `updated_at`.
- Fields not in `ALLOWED_FIELDS` are silently ignored.
- Passing an empty patch returns the unchanged profile without running an UPDATE.

**Failure criteria:**
- `get()` returns null on a migrated database.
- `update()` overwrites the `id` or `created_at` fields.
- `update()` does not refresh `updated_at`.

---

### T29.4: Write Profile Service

**File:** `packages/core/src/services/profile-service.ts`

```typescript
/**
 * ProfileService — business logic for user profile management.
 *
 * Validates input, delegates to ProfileRepository, returns Result<T>.
 */

import type { Database } from 'bun:sqlite'
import type { UserProfile, UpdateProfile, Result } from '../types'
import * as ProfileRepository from '../db/repositories/profile-repository'

export class ProfileService {
  constructor(private db: Database) {}

  getProfile(): Result<UserProfile> {
    const profile = ProfileRepository.get(this.db)
    if (!profile) {
      return { ok: false, error: { code: 'NOT_FOUND', message: 'Profile not found' } }
    }
    return { ok: true, data: profile }
  }

  updateProfile(patch: UpdateProfile): Result<UserProfile> {
    if (patch.name !== undefined && patch.name.trim() === '') {
      return {
        ok: false,
        error: { code: 'VALIDATION_ERROR', message: 'Name must not be empty' },
      }
    }
    if (patch.name === null) {
      return {
        ok: false,
        error: { code: 'VALIDATION_ERROR', message: 'Name cannot be null' },
      }
    }
    const updated = ProfileRepository.update(this.db, patch)
    if (!updated) {
      return { ok: false, error: { code: 'NOT_FOUND', message: 'Profile not found' } }
    }
    return { ok: true, data: updated }
  }
}
```

**Expected inputs/outputs:**

- `getProfile()` on a migrated DB returns `{ ok: true, data: { id: '...', name: 'User', ... } }`.
- `updateProfile({ name: '' })` returns `{ ok: false, error: { code: 'VALIDATION_ERROR', message: 'Name must not be empty' } }`.
- `updateProfile({ name: '  ' })` also returns a validation error (whitespace-only name).
- `updateProfile({ email: 'adam@test.com' })` returns `{ ok: true, data: { ..., email: 'adam@test.com' } }`.

**Acceptance criteria:**
- Returns `Result<UserProfile>` discriminated union.
- Empty or whitespace-only `name` is rejected with `VALIDATION_ERROR`.
- All other fields accept any string value including null (clearing a field).
- Returns `NOT_FOUND` if the profile table is empty.

**Failure criteria:**
- Throws an exception instead of returning a `Result`.
- Allows empty name to be persisted.

---

### T29.5: Register Service in `createServices()`

#### `packages/core/src/services/index.ts`

Add import at the top:

```typescript
import { ProfileService } from './profile-service'
```

Add to `Services` interface:

```typescript
export interface Services {
  sources: SourceService
  bullets: BulletService
  perspectives: PerspectiveService
  derivation: DerivationService
  resumes: ResumeService
  audit: AuditService
  review: ReviewService
  organizations: OrganizationService
  notes: NoteService
  integrity: IntegrityService
  domains: DomainService
  archetypes: ArchetypeService
  profile: ProfileService
}
```

Add to `createServices()` return object:

```typescript
    profile: new ProfileService(db),
```

Add re-export at the bottom:

```typescript
export { ProfileService } from './profile-service'
```

**Acceptance criteria:**
- `services.profile` is accessible in route handlers.
- `ProfileService` is re-exported from `@forge/core`.

**Failure criteria:**
- TypeScript error about missing `profile` property on `Services`.

---

### T29.6: Write API Routes

**File:** `packages/core/src/routes/profile.ts`

```typescript
/**
 * Profile routes — thin HTTP layer over ProfileService.
 *
 * GET  /profile    — returns the single user profile
 * PATCH /profile   — updates profile fields (partial patch)
 */

import { Hono } from 'hono'
import type { Services } from '../services'
import { mapStatusCode } from './server'

export function profileRoutes(services: Services) {
  const app = new Hono()

  app.get('/profile', (c) => {
    const result = services.profile.getProfile()
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  // Note: c.req.json() will throw if the request body is empty or not valid JSON.
  // This is consistent with the existing pattern across all PATCH routes in the codebase.
  app.patch('/profile', async (c) => {
    const body = await c.req.json()
    const result = services.profile.updateProfile(body)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  return app
}
```

**Expected inputs/outputs:**

- `GET /api/profile` returns `200 { data: { id, name, email, ... } }`.
- `PATCH /api/profile` with `{ "name": "Adam", "email": "adam@test.com" }` returns `200 { data: { ..., name: "Adam", email: "adam@test.com" } }`.
- `PATCH /api/profile` with `{ "name": "" }` returns `400 { error: { code: "VALIDATION_ERROR", message: "Name must not be empty" } }`.
- `PATCH /api/profile` with `{}` returns `200 { data: { ... } }` (unchanged).

**Acceptance criteria:**
- Routes are mounted under `/api/profile` (the Hono basePath adds `/api`).
- GET returns 200 with `{ data: UserProfile }`.
- PATCH returns 200 with updated profile or 400 for validation errors.
- No POST or DELETE routes exist.

**Failure criteria:**
- Routes are mounted at wrong path.
- Validation errors return 500 instead of 400.

---

### T29.7: Mount Routes in Server

#### `packages/core/src/routes/server.ts`

Add import:

```typescript
import { profileRoutes } from './profile'
```

Add route mount after the existing routes (before the global error handler):

```typescript
  app.route('/', profileRoutes(services))
```

**Acceptance criteria:**
- `GET /api/profile` and `PATCH /api/profile` are reachable through the server.

**Failure criteria:**
- Import error or missing route registration causes 404 on `/api/profile`.

---

### T29.8: Update IR Compiler

#### `packages/core/src/services/resume-compiler.ts`

**Add `UserProfile` to imports:**

```typescript
import type {
  ResumeDocument,
  ResumeHeader,
  IRSection,
  IRSectionType,
  IRSectionItem,
  ExperienceGroup,
  ExperienceSubheading,
  ExperienceBullet,
  SkillGroup,
  EducationItem,
  ProjectItem,
  ClearanceItem,
  PresentationItem,
  SummaryItem,
  UserProfile,
} from '../types'
```

**Update `compileResumeIR()`** to fetch the profile:

Replace:
```typescript
  // 2. Parse header (JSON blob or build default)
  const header = parseHeader(resume)
```

With:
```typescript
  // 2. Fetch user profile for contact fields
  const profile = db
    .query('SELECT * FROM user_profile LIMIT 1')
    .get() as UserProfile | null

  // 3. Parse header — contact fields from profile, content fields from resume
  const header = parseHeader(resume, profile)
```

Update the subsequent section numbering comments accordingly (3 becomes 4, 4 becomes 5).

**Update `parseHeader()` signature and implementation:**

Replace:
```typescript
function parseHeader(resume: ResumeRow): ResumeHeader {
  if (resume.header) {
    try {
      return JSON.parse(resume.header) as ResumeHeader
    } catch {
      // Fall through to default
    }
  }
  return {
    name: resume.name,
    tagline: resume.target_role,
    location: null,
    email: null,
    phone: null,
    linkedin: null,
    github: null,
    website: null,
    clearance: null,
  }
}
```

With:
```typescript
function parseHeader(resume: ResumeRow, profile: UserProfile | null): ResumeHeader {
  // Content fields from the resume-specific header JSON
  let tagline: string | null = resume.target_role
  if (resume.header) {
    try {
      const parsed = JSON.parse(resume.header)
      tagline = parsed.tagline ?? tagline
    } catch {
      // Fall through — tagline defaults to target_role
    }
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

**Acceptance criteria:**
- `compileResumeIR()` reads contact info from `user_profile` table.
- `tagline` still comes from the resume's header JSON or `target_role`.
- If `user_profile` is empty (null), contact fields fall back to resume name + nulls.
- No change to the `ResumeHeader` type or the `ResumeDocument` shape.

**Failure criteria:**
- Compiler crashes when `user_profile` table has no rows.
- Contact fields still come from the resume header JSON.
- Tagline is sourced from profile instead of resume.

---

### T29.9: Add SDK Types

#### `packages/sdk/src/types.ts`

Add after the `ArchetypeWithDomains` block (before the `// -- Rich response types` section):

```typescript
// ---------------------------------------------------------------------------
// User Profile
// ---------------------------------------------------------------------------

/** Global user profile — single source of truth for contact information. */
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

/** Input for partially updating the user profile. */
export type UpdateProfile = Partial<Omit<UserProfile, 'id' | 'created_at' | 'updated_at'>>
```

**Acceptance criteria:**
- Types are standalone (no imports from `@forge/core`).
- Match the core types exactly.

**Failure criteria:**
- Type shape mismatch between SDK and core causes runtime contract violations.

---

### T29.10: Write SDK Resource

**File:** `packages/sdk/src/resources/profile.ts`

```typescript
import type { UserProfile, UpdateProfile, RequestFn, Result } from '../types'

export class ProfileResource {
  constructor(private request: RequestFn) {}

  /** Get the user profile. */
  get(): Promise<Result<UserProfile>> {
    return this.request<UserProfile>('GET', '/api/profile')
  }

  /** Update profile fields. Only provided fields are modified. */
  update(data: UpdateProfile): Promise<Result<UserProfile>> {
    return this.request<UserProfile>('PATCH', '/api/profile', data)
  }
}
```

**Expected inputs/outputs:**

- `client.profile.get()` issues `GET /api/profile` and returns `Result<UserProfile>`.
- `client.profile.update({ name: 'Adam' })` issues `PATCH /api/profile` with body `{ name: 'Adam' }` and returns `Result<UserProfile>`.

**Note:** The spec references `ApiResponse<T>` which does not exist in the SDK. The correct type is `RequestFn` from `../types`, as used here.

**Acceptance criteria:**
- Uses the `RequestFn` type (not `RequestListFn`) since profile is not paginated.
- Only `get()` and `update()` methods exist (no `create()` or `delete()`).

**Failure criteria:**
- Wrong HTTP methods or paths.
- Uses `RequestListFn` for a non-paginated resource.

---

### T29.11: Register SDK Resource in Client

#### `packages/sdk/src/client.ts`

Add import:

```typescript
import { ProfileResource } from './resources/profile'
```

Add public property declaration (after `skills`):

```typescript
  /** User profile (contact info). */
  public profile: ProfileResource
```

Add initialization in constructor (after `this.skills = ...`):

```typescript
    this.profile = new ProfileResource(req)
```

**Acceptance criteria:**
- `client.profile.get()` and `client.profile.update()` work from SDK consumers.

**Failure criteria:**
- TypeScript error about missing `profile` property.
- `profile` is undefined at runtime because it was not initialized in constructor.

---

### T29.12: Update SDK Barrel Exports

#### `packages/sdk/src/index.ts`

Add to the core entity types export block:

```typescript
export type {
  Source,
  Bullet,
  Perspective,
  Resume,
  Organization,
  ResumeEntry,
  ResumeSectionEntity,
  ResumeSkill,
  Skill,
  UserNote,
  UserProfile,
} from './types'
```

Add a new export block for profile input types (after the note types block):

```typescript
// Profile types
export type {
  UpdateProfile,
} from './types'
```

Add to the resource classes block:

```typescript
export { ProfileResource } from './resources/profile'
```

**Acceptance criteria:**
- `import type { UserProfile, UpdateProfile } from '@forge/sdk'` works.
- `import { ProfileResource } from '@forge/sdk'` works.

**Failure criteria:**
- Missing re-export causes "not exported" TypeScript error in consuming packages.

---

### T29.13: Write Config Layout

**File:** `packages/webui/src/routes/config/+layout.svelte`

Pass-through layout — no additional nav (the sidebar handles Config sub-items once Phase 32 Nav Restructuring lands):

```svelte
<script>
  let { children } = $props()
</script>

{@render children()}
```

**Acceptance criteria:**
- SvelteKit routes under `/config/*` render correctly.
- No duplicate navigation chrome.

**Failure criteria:**
- Layout wrapping causes SvelteKit hydration error.
- Content is not rendered because `children` is not invoked.

---

### T29.14: Write Profile Editor Page

**File:** `packages/webui/src/routes/config/profile/+page.svelte`

```svelte
<script lang="ts">
  import { onDestroy } from 'svelte'
  import type { UserProfile } from '@forge/sdk'
  import { forge, friendlyError } from '$lib/sdk'
  import { addToast } from '$lib/stores/toast.svelte'

  let loading = $state(true)
  let saving = $state(false)
  let profile = $state<UserProfile | null>(null)
  let form = $state<Record<string, string | null>>({
    name: '',
    email: null,
    phone: null,
    location: null,
    linkedin: null,
    github: null,
    website: null,
    clearance: null,
  })

  let saveTimeout: ReturnType<typeof setTimeout> | null = null

  onDestroy(() => {
    if (saveTimeout) clearTimeout(saveTimeout)
  })

  async function loadProfile() {
    loading = true
    const result = await forge.profile.get()
    if (result.ok) {
      profile = result.data
      form = {
        name: result.data.name,
        email: result.data.email,
        phone: result.data.phone,
        location: result.data.location,
        linkedin: result.data.linkedin,
        github: result.data.github,
        website: result.data.website,
        clearance: result.data.clearance,
      }
    } else {
      addToast({ message: friendlyError(result.error), type: 'error' })
    }
    loading = false
  }

  async function handleSave() {
    saving = true
    try {
      const patch: Record<string, string | null> = {}
      for (const [key, value] of Object.entries(form)) {
        if (profile && value !== (profile as Record<string, unknown>)[key]) {
          if (key === 'name') {
            patch[key] = value  // name: send as-is (empty string triggers server validation)
          } else {
            patch[key] = value === '' ? null : value
          }
        }
      }

      if (Object.keys(patch).length === 0) {
        saving = false
        return
      }

      const result = await forge.profile.update(patch)
      if (result.ok) {
        profile = result.data
        addToast({ message: 'Profile saved', type: 'success' })
      } else {
        addToast({ message: friendlyError(result.error), type: 'error' })
      }
    } finally {
      saving = false
    }
  }

  function scheduleAutosave() {
    if (saveTimeout) clearTimeout(saveTimeout)
    saveTimeout = setTimeout(() => handleSave(), 500)
  }

  // Load on mount
  $effect(() => {
    loadProfile()
  })
</script>

<div class="page-header">
  <h1>Profile</h1>
  <p class="page-subtitle">Your contact information, shared across all resumes.</p>
</div>

{#if loading}
  <div class="loading">Loading profile...</div>
{:else if profile}
  <form class="profile-form" onsubmit={(e) => { e.preventDefault(); handleSave() }}>
    <div class="form-grid">
      <div class="form-field full-width">
        <label for="pf-name">Name <span class="required">*</span></label>
        <input id="pf-name" type="text" bind:value={form.name} required
               oninput={scheduleAutosave} />
      </div>

      <div class="form-field">
        <label for="pf-email">Email</label>
        <input id="pf-email" type="email" bind:value={form.email}
               placeholder="adam@example.com"
               oninput={scheduleAutosave} />
      </div>

      <div class="form-field">
        <label for="pf-phone">Phone</label>
        <input id="pf-phone" type="tel" bind:value={form.phone}
               placeholder="+1-555-0123"
               oninput={scheduleAutosave} />
      </div>

      <div class="form-field">
        <label for="pf-location">Location</label>
        <input id="pf-location" type="text" bind:value={form.location}
               placeholder="Washington, DC"
               oninput={scheduleAutosave} />
      </div>

      <div class="form-field">
        <label for="pf-linkedin">LinkedIn</label>
        <input id="pf-linkedin" type="text" bind:value={form.linkedin}
               placeholder="linkedin.com/in/username"
               oninput={scheduleAutosave} />
      </div>

      <div class="form-field">
        <label for="pf-github">GitHub</label>
        <input id="pf-github" type="text" bind:value={form.github}
               placeholder="github.com/username"
               oninput={scheduleAutosave} />
      </div>

      <div class="form-field">
        <label for="pf-website">Website</label>
        <input id="pf-website" type="text" bind:value={form.website}
               placeholder="yoursite.dev"
               oninput={scheduleAutosave} />
      </div>

      <div class="form-field full-width">
        <label for="pf-clearance">Security Clearance</label>
        <input id="pf-clearance" type="text" bind:value={form.clearance}
               placeholder="TS/SCI with CI Polygraph - Active"
               oninput={scheduleAutosave} />
      </div>
    </div>

    <div class="form-actions">
      <button class="btn btn-primary" type="submit" disabled={saving}>
        {saving ? 'Saving...' : 'Save Profile'}
      </button>
    </div>
  </form>
{:else}
  <div class="error">Failed to load profile. Check that the API server is running.</div>
{/if}

<style>
  .page-header {
    margin-bottom: 2rem;
  }

  .page-header h1 {
    font-size: 1.75rem;
    font-weight: 700;
    color: #1a1a2e;
  }

  .page-subtitle {
    font-size: 0.9rem;
    color: #6b7280;
    margin-top: 0.25rem;
  }

  .profile-form {
    background: #fff;
    border: 1px solid #e5e7eb;
    border-radius: 8px;
    padding: 1.5rem;
    max-width: 640px;
  }

  .form-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
  }

  .full-width {
    grid-column: 1 / -1;
  }

  .form-field label {
    display: block;
    font-size: 0.8rem;
    font-weight: 600;
    color: #374151;
    margin-bottom: 0.35rem;
  }

  .required {
    color: #ef4444;
  }

  .form-field input {
    width: 100%;
    padding: 0.5rem 0.75rem;
    border: 1px solid #d1d5db;
    border-radius: 6px;
    font-size: 0.875rem;
    color: #1a1a1a;
    background: #fff;
    transition: border-color 0.15s;
  }

  .form-field input:focus {
    outline: none;
    border-color: #6c63ff;
    box-shadow: 0 0 0 2px rgba(108, 99, 255, 0.15);
  }

  .form-actions {
    display: flex;
    justify-content: flex-end;
    margin-top: 1.5rem;
  }

  .btn {
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 6px;
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.15s, opacity 0.15s;
    white-space: nowrap;
    font-family: inherit;
  }

  .btn:disabled { opacity: 0.6; cursor: not-allowed; }
  .btn-primary { background: #6c63ff; color: #fff; }
  .btn-primary:hover:not(:disabled) { background: #5a52e0; }

  .loading, .error {
    text-align: center;
    padding: 3rem;
    color: #6b7280;
  }

  .error {
    color: #ef4444;
  }
</style>
```

**Acceptance criteria:**
- Page loads at `/config/profile`.
- Form populates from `GET /api/profile`.
- Autosaves on input after 500ms debounce.
- Manual "Save Profile" button also works.
- Empty name shows validation error toast.
- Empty optional fields are sent as `null` (clearing the field).

**Failure criteria:**
- Page crashes on load because SDK has no `profile` resource.
- Autosave fires on every keystroke (no debounce).
- Empty strings are persisted instead of nulls for optional fields.

---

### T29.15: Update HeaderEditor Component

#### `packages/webui/src/lib/components/resume/HeaderEditor.svelte`

Replace the entire file with the updated version where contact fields are read-only and only `tagline` is editable:

```svelte
<script lang="ts">
  import type { ResumeHeader } from '@forge/sdk'
  import { forge, friendlyError } from '$lib/sdk'
  import { addToast } from '$lib/stores/toast.svelte'

  let {
    header,
    resumeId,
    onSave,
  }: {
    header: ResumeHeader
    resumeId: string
    onSave: () => Promise<void>
  } = $props()

  let editingTagline = $state(false)
  let saving = $state(false)
  let taglineValue = $state(header.tagline ?? '')

  async function handleSaveTagline() {
    saving = true
    try {
      // Update the resume header JSON — only tagline is resume-specific
      const currentHeader = header
      const headerJson = JSON.stringify({
        ...currentHeader,
        tagline: taglineValue || null,
      })
      const result = await forge.resumes.updateHeader(resumeId, { header: headerJson } as unknown as Record<string, unknown>)
      if (result.ok) {
        addToast({ message: 'Tagline updated', type: 'success' })
        editingTagline = false
        await onSave()
      } else {
        addToast({ message: friendlyError(result.error), type: 'error' })
      }
    } finally {
      saving = false
    }
  }
</script>

<div class="header-editor">
  <div class="header-display">
    <div class="header-display-top">
      <h2 class="header-name">{header.name}</h2>
    </div>

    {#if editingTagline}
      <form class="tagline-edit" onsubmit={(e) => { e.preventDefault(); handleSaveTagline() }}>
        <input type="text" bind:value={taglineValue}
               placeholder="Security Engineer | Cloud + DevSecOps"
               class="tagline-input" />
        <div class="tagline-actions">
          <button class="btn btn-ghost btn-sm" type="button"
                  onclick={() => { editingTagline = false; taglineValue = header.tagline ?? '' }}>
            Cancel
          </button>
          <button class="btn btn-primary btn-sm" type="submit" disabled={saving}>
            {saving ? 'Saving...' : 'Save'}
          </button>
        </div>
      </form>
    {:else}
      <div class="tagline-row">
        {#if header.tagline}
          <p class="header-tagline">{header.tagline}</p>
        {:else}
          <p class="header-tagline placeholder">No tagline set</p>
        {/if}
        <button class="btn btn-sm btn-ghost" onclick={() => { editingTagline = true; taglineValue = header.tagline ?? '' }}>
          Edit Tagline
        </button>
      </div>
    {/if}

    <div class="header-contact">
      {#if header.location}<span>{header.location}</span>{/if}
      {#if header.email}<span>{header.email}</span>{/if}
      {#if header.phone}<span>{header.phone}</span>{/if}
      {#if header.linkedin}<a href={header.linkedin} target="_blank" rel="noopener">LinkedIn</a>{/if}
      {#if header.github}<a href={header.github} target="_blank" rel="noopener">GitHub</a>{/if}
      {#if header.website}<a href={header.website} target="_blank" rel="noopener">Website</a>{/if}
    </div>
    {#if header.clearance}
      <p class="header-clearance">{header.clearance}</p>
    {/if}

    <a href="/config/profile" class="edit-profile-link">
      Edit contact info in Profile
    </a>
  </div>
</div>

<style>
  .header-display {
    padding: 1.25rem;
    background: #fff;
    border: 1px solid #e5e7eb;
    border-radius: 8px;
    margin-bottom: 1rem;
    text-align: center;
  }

  .header-display-top {
    display: flex;
    justify-content: center;
    align-items: flex-start;
  }

  .header-name {
    font-size: 1.5rem;
    font-weight: 700;
    color: #1a1a2e;
  }

  .tagline-row {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    margin-top: 0.25rem;
  }

  .header-tagline {
    font-size: 0.95rem;
    color: #374151;
  }

  .header-tagline.placeholder {
    color: #9ca3af;
    font-style: italic;
  }

  .tagline-edit {
    margin-top: 0.5rem;
  }

  .tagline-input {
    width: 100%;
    max-width: 400px;
    padding: 0.5rem 0.75rem;
    border: 1px solid #d1d5db;
    border-radius: 6px;
    font-size: 0.875rem;
    color: #1a1a1a;
    text-align: center;
  }

  .tagline-input:focus {
    outline: none;
    border-color: #6c63ff;
    box-shadow: 0 0 0 2px rgba(108, 99, 255, 0.15);
  }

  .tagline-actions {
    display: flex;
    justify-content: center;
    gap: 0.5rem;
    margin-top: 0.5rem;
  }

  .header-contact {
    display: flex;
    gap: 1rem;
    justify-content: center;
    flex-wrap: wrap;
    margin-top: 0.5rem;
    font-size: 0.85rem;
    color: #6b7280;
  }

  .header-contact a {
    color: #6c63ff;
    text-decoration: none;
  }

  .header-contact a:hover {
    text-decoration: underline;
  }

  .header-clearance {
    margin-top: 0.5rem;
    font-size: 0.8rem;
    font-weight: 600;
    color: #059669;
  }

  .edit-profile-link {
    display: inline-block;
    margin-top: 0.75rem;
    font-size: 0.8rem;
    color: #6c63ff;
    text-decoration: none;
  }

  .edit-profile-link:hover {
    text-decoration: underline;
  }

  .btn {
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 6px;
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.15s, opacity 0.15s;
    white-space: nowrap;
    font-family: inherit;
  }

  .btn:disabled { opacity: 0.6; cursor: not-allowed; }
  .btn-primary { background: #6c63ff; color: #fff; }
  .btn-primary:hover:not(:disabled) { background: #5a52e0; }
  .btn-ghost { background: transparent; color: #6b7280; }
  .btn-ghost:hover { color: #374151; background: #f3f4f6; }
  .btn-sm { padding: 0.3rem 0.6rem; font-size: 0.75rem; }
</style>
```

**Acceptance criteria:**
- Contact fields (name, email, phone, location, links, clearance) are read-only display.
- Only `tagline` has an inline edit flow.
- "Edit contact info in Profile" link navigates to `/config/profile`.

**Failure criteria:**
- User can still edit contact fields inline in the resume builder.
- Link points to wrong URL.

---

### T29.16: Add Config Nav Item

#### `packages/webui/src/routes/+layout.svelte`

Add a "Config" entry to the `navItems` array:

```typescript
  const navItems = [
    { href: '/', label: 'Dashboard' },
    { href: '/sources', label: 'Sources' },
    { href: '/bullets', label: 'Bullets' },
    { href: '/resumes', label: 'Resumes' },
    { href: '/organizations', label: 'Organizations' },
    { href: '/domains', label: 'Domains' },
    { href: '/skills', label: 'Skills' },
    { href: '/archetypes', label: 'Archetypes' },
    { href: '/chain', label: 'Chain View' },
    { href: '/logs', label: 'Logs' },
    { href: '/notes', label: 'Notes' },
    { href: '/config/profile', label: 'Config' },
  ]
```

**Implementation guidance:** The active state detection should use `page.url.pathname.startsWith('/config')` to highlight the Config nav item when on any `/config/*` path, matching the existing pattern in `+layout.svelte`.

**Acceptance criteria:**
- "Config" appears in the sidebar.
- Clicking it navigates to `/config/profile`.
- The active state highlights correctly when on any `/config/*` path.

**Failure criteria:**
- Nav item is missing or points to wrong path.

---

### T29.17: Add Test Helper

#### `packages/core/src/db/__tests__/helpers.ts`

Add after the `seedArchetypeDomain` function:

```typescript
/** Seed a test user profile and return its ID */
export function seedProfile(db: Database, opts: {
  name?: string
  email?: string | null
  phone?: string | null
  location?: string | null
  linkedin?: string | null
  github?: string | null
  website?: string | null
  clearance?: string | null
} = {}): string {
  const id = testUuid()
  // Delete any existing profile (single-row table)
  db.run('DELETE FROM user_profile')
  db.run(
    `INSERT INTO user_profile (id, name, email, phone, location, linkedin, github, website, clearance)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)`,
    [
      id,
      opts.name ?? 'Test User',
      opts.email ?? null,
      opts.phone ?? null,
      opts.location ?? null,
      opts.linkedin ?? null,
      opts.github ?? null,
      opts.website ?? null,
      opts.clearance ?? null,
    ]
  )
  return id
}
```

**Acceptance criteria:**
- `seedProfile(db)` creates a profile row and returns the ID.
- `seedProfile(db, { name: 'Adam', email: 'adam@test.com' })` creates a profile with those values.
- Clears any existing profile row first (single-row constraint).

**Failure criteria:**
- Leaves multiple rows in `user_profile`.
- Throws if called on a DB where migration 005 hasn't run (but `createTestDb()` runs all migrations, so this is unlikely).

---

## Testing Support

### Fixtures / Test Cases

**Profile fixture data:**

```typescript
const PROFILE_FIXTURES = {
  minimal: { name: 'User' },
  full: {
    name: 'Adam',
    email: 'adam@example.com',
    phone: '+1-555-0123',
    location: 'Washington, DC',
    linkedin: 'linkedin.com/in/adam',
    github: 'github.com/adam',
    website: 'adam.dev',
    clearance: 'TS/SCI with CI Polygraph - Active',
  },
  partial_update: {
    email: 'newemail@example.com',
    phone: null,  // clear phone
  },
  invalid_name: { name: '' },
  whitespace_name: { name: '   ' },
}
```

**Resume with header JSON for migration testing:**

```typescript
const HEADER_JSON = JSON.stringify({
  name: 'Adam',
  tagline: 'Security Engineer',
  email: 'adam@test.com',
  phone: '+1-555-0123',
  location: 'Reston, VA',
  linkedin: 'linkedin.com/in/adam',
  github: 'github.com/adam',
  website: 'adam.dev',
  clearance: 'TS/SCI - Active',
})
```

### Test Kinds

#### Unit Tests: Repository

**File:** `packages/core/src/db/repositories/__tests__/profile-repository.test.ts`

```typescript
import { describe, it, expect, beforeEach, afterEach } from 'bun:test'
import { Database } from 'bun:sqlite'
import { createTestDb, seedProfile, seedResume } from '../../__tests__/helpers'
import * as ProfileRepository from '../profile-repository'

describe('ProfileRepository', () => {
  let db: Database

  beforeEach(() => {
    db = createTestDb()
  })

  afterEach(() => {
    db.close()
  })

  describe('get()', () => {
    it('returns the profile row after migration', () => {
      const profile = ProfileRepository.get(db)
      expect(profile).not.toBeNull()
      expect(profile!.name).toBe('User')
      expect(profile!.id).toHaveLength(36)
      expect(profile!.email).toBeNull()
    })

    it('returns seeded profile with custom data', () => {
      seedProfile(db, { name: 'Adam', email: 'adam@test.com' })
      const profile = ProfileRepository.get(db)
      expect(profile!.name).toBe('Adam')
      expect(profile!.email).toBe('adam@test.com')
    })
  })

  describe('update()', () => {
    it('updates only provided fields', () => {
      const result = ProfileRepository.update(db, { name: 'Adam', email: 'adam@test.com' })
      expect(result).not.toBeNull()
      expect(result!.name).toBe('Adam')
      expect(result!.email).toBe('adam@test.com')
      expect(result!.phone).toBeNull()
    })

    it('returns unchanged profile for empty patch', () => {
      const before = ProfileRepository.get(db)
      const result = ProfileRepository.update(db, {})
      expect(result).not.toBeNull()
      expect(result!.name).toBe(before!.name)
      expect(result!.updated_at).toBe(before!.updated_at)
    })

    it('refreshes updated_at on update', () => {
      const before = ProfileRepository.get(db)
      // Force a small delay so timestamps differ
      const result = ProfileRepository.update(db, { name: 'Updated' })
      expect(result!.name).toBe('Updated')
      // updated_at should be >= before (same second is acceptable)
      expect(result!.updated_at >= before!.updated_at).toBe(true)
    })

    it('ignores disallowed fields (id, created_at, updated_at)', () => {
      const before = ProfileRepository.get(db)
      const result = ProfileRepository.update(db, {
        name: 'Safe',
      } as any)
      expect(result!.id).toBe(before!.id)
      expect(result!.created_at).toBe(before!.created_at)
    })

    it('can clear a nullable field by setting to null', () => {
      ProfileRepository.update(db, { email: 'adam@test.com' })
      const result = ProfileRepository.update(db, { email: null as unknown as string })
      expect(result!.email).toBeNull()
    })

    it('returns null when profile table is empty', () => {
      db.run('DELETE FROM user_profile')
      const result = ProfileRepository.update(db, { name: 'Test' })
      expect(result).toBeNull()
    })
  })
})
```

#### Unit Tests: Service

**File:** `packages/core/src/services/__tests__/profile-service.test.ts`

```typescript
import { describe, it, expect, beforeEach, afterEach } from 'bun:test'
import { Database } from 'bun:sqlite'
import { createTestDb, seedProfile } from '../../db/__tests__/helpers'
import { ProfileService } from '../profile-service'

describe('ProfileService', () => {
  let db: Database
  let service: ProfileService

  beforeEach(() => {
    db = createTestDb()
    service = new ProfileService(db)
  })

  afterEach(() => {
    db.close()
  })

  describe('getProfile()', () => {
    it('returns ok with the profile', () => {
      const result = service.getProfile()
      expect(result.ok).toBe(true)
      if (result.ok) {
        expect(result.data.name).toBe('User')
        expect(result.data.id).toHaveLength(36)
      }
    })

    it('returns NOT_FOUND when profile is missing', () => {
      db.run('DELETE FROM user_profile')
      const result = service.getProfile()
      expect(result.ok).toBe(false)
      if (!result.ok) {
        expect(result.error.code).toBe('NOT_FOUND')
      }
    })
  })

  describe('updateProfile()', () => {
    it('updates name and returns ok', () => {
      const result = service.updateProfile({ name: 'Adam' })
      expect(result.ok).toBe(true)
      if (result.ok) {
        expect(result.data.name).toBe('Adam')
      }
    })

    it('rejects empty name with VALIDATION_ERROR', () => {
      const result = service.updateProfile({ name: '' })
      expect(result.ok).toBe(false)
      if (!result.ok) {
        expect(result.error.code).toBe('VALIDATION_ERROR')
        expect(result.error.message).toContain('Name must not be empty')
      }
    })

    it('rejects whitespace-only name', () => {
      const result = service.updateProfile({ name: '   ' })
      expect(result.ok).toBe(false)
      if (!result.ok) {
        expect(result.error.code).toBe('VALIDATION_ERROR')
      }
    })

    it('rejects null name with VALIDATION_ERROR', () => {
      const result = service.updateProfile({ name: null as unknown as string })
      expect(result.ok).toBe(false)
      if (!result.ok) {
        expect(result.error.code).toBe('VALIDATION_ERROR')
        expect(result.error.message).toContain('Name cannot be null')
      }
    })

    it('allows updating optional fields to null', () => {
      service.updateProfile({ email: 'adam@test.com' })
      const result = service.updateProfile({ email: null as unknown as string })
      expect(result.ok).toBe(true)
      if (result.ok) {
        expect(result.data.email).toBeNull()
      }
    })

    it('empty patch returns unchanged profile', () => {
      const result = service.updateProfile({})
      expect(result.ok).toBe(true)
    })

    it('returns NOT_FOUND when profile is missing', () => {
      db.run('DELETE FROM user_profile')
      const result = service.updateProfile({ name: 'Test' })
      expect(result.ok).toBe(false)
      if (!result.ok) {
        expect(result.error.code).toBe('NOT_FOUND')
      }
    })

    it('updates multiple fields at once', () => {
      const result = service.updateProfile({
        name: 'Adam',
        email: 'adam@test.com',
        phone: '+1-555-0123',
        location: 'Washington, DC',
        linkedin: 'linkedin.com/in/adam',
        github: 'github.com/adam',
        website: 'adam.dev',
        clearance: 'TS/SCI with CI Polygraph - Active',
      })
      expect(result.ok).toBe(true)
      if (result.ok) {
        expect(result.data.name).toBe('Adam')
        expect(result.data.email).toBe('adam@test.com')
        expect(result.data.clearance).toBe('TS/SCI with CI Polygraph - Active')
      }
    })
  })
})
```

#### Integration Tests: API Routes

**File:** `packages/core/src/routes/__tests__/profile.test.ts`

```typescript
import { describe, test, expect, beforeEach, afterEach } from 'bun:test'
import { createTestApp, apiRequest, type TestContext } from './helpers'
import { seedProfile } from '../../db/__tests__/helpers'

describe('Profile routes', () => {
  let ctx: TestContext

  beforeEach(() => {
    ctx = createTestApp()
  })

  afterEach(() => {
    ctx.db.close()
  })

  // -- GET /profile ---------------------------------------------------------

  test('GET /profile returns the profile', async () => {
    const res = await apiRequest(ctx.app, 'GET', '/profile')
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data).toBeDefined()
    expect(body.data.id).toHaveLength(36)
    expect(body.data.name).toBe('User')
    expect(body.data.created_at).toBeDefined()
    expect(body.data.updated_at).toBeDefined()
  })

  test('GET /profile returns seeded profile data', async () => {
    seedProfile(ctx.db, { name: 'Adam', email: 'adam@test.com', clearance: 'TS/SCI' })
    const res = await apiRequest(ctx.app, 'GET', '/profile')
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.name).toBe('Adam')
    expect(body.data.email).toBe('adam@test.com')
    expect(body.data.clearance).toBe('TS/SCI')
  })

  // -- PATCH /profile -------------------------------------------------------

  test('PATCH /profile updates provided fields', async () => {
    const res = await apiRequest(ctx.app, 'PATCH', '/profile', {
      name: 'Adam',
      email: 'adam@example.com',
    })
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.name).toBe('Adam')
    expect(body.data.email).toBe('adam@example.com')
  })

  test('PATCH /profile with empty name returns 400', async () => {
    const res = await apiRequest(ctx.app, 'PATCH', '/profile', { name: '' })
    expect(res.status).toBe(400)
    const body = await res.json()
    expect(body.error.code).toBe('VALIDATION_ERROR')
  })

  test('PATCH /profile with whitespace-only name returns 400', async () => {
    const res = await apiRequest(ctx.app, 'PATCH', '/profile', { name: '   ' })
    expect(res.status).toBe(400)
    const body = await res.json()
    expect(body.error.code).toBe('VALIDATION_ERROR')
  })

  test('PATCH /profile with empty body returns 200 unchanged', async () => {
    const getRes = await apiRequest(ctx.app, 'GET', '/profile')
    const before = (await getRes.json()).data

    const res = await apiRequest(ctx.app, 'PATCH', '/profile', {})
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.name).toBe(before.name)
  })

  test('PATCH /profile updates all fields at once', async () => {
    const res = await apiRequest(ctx.app, 'PATCH', '/profile', {
      name: 'Adam',
      email: 'adam@example.com',
      phone: '+1-555-0123',
      location: 'Washington, DC',
      linkedin: 'linkedin.com/in/adam',
      github: 'github.com/adam',
      website: 'adam.dev',
      clearance: 'TS/SCI with CI Polygraph - Active',
    })
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.name).toBe('Adam')
    expect(body.data.email).toBe('adam@example.com')
    expect(body.data.phone).toBe('+1-555-0123')
    expect(body.data.location).toBe('Washington, DC')
    expect(body.data.linkedin).toBe('linkedin.com/in/adam')
    expect(body.data.github).toBe('github.com/adam')
    expect(body.data.website).toBe('adam.dev')
    expect(body.data.clearance).toBe('TS/SCI with CI Polygraph - Active')
  })

  test('PATCH /profile refreshes updated_at', async () => {
    const getRes = await apiRequest(ctx.app, 'GET', '/profile')
    const before = (await getRes.json()).data

    const res = await apiRequest(ctx.app, 'PATCH', '/profile', { name: 'Updated' })
    const body = await res.json()
    expect(body.data.updated_at >= before.updated_at).toBe(true)
  })

  test('PATCH /profile ignores unknown fields', async () => {
    const res = await apiRequest(ctx.app, 'PATCH', '/profile', {
      name: 'Safe',
      evil_field: 'should be ignored',
    })
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.name).toBe('Safe')
    expect(body.data.evil_field).toBeUndefined()
  })

  // -- Contract shape -------------------------------------------------------

  test('GET /profile follows { data: entity } contract', async () => {
    const res = await apiRequest(ctx.app, 'GET', '/profile')
    const body = await res.json()
    expect(body).toHaveProperty('data')
    expect(body).not.toHaveProperty('error')
    expect(body).not.toHaveProperty('pagination')
    expect(body.data).toHaveProperty('id')
    expect(body.data).toHaveProperty('name')
    expect(body.data).toHaveProperty('created_at')
    expect(body.data).toHaveProperty('updated_at')
  })
})
```

#### Contract Tests: SDK Shape Validation

Add to `packages/core/src/routes/__tests__/contracts.test.ts`:

```typescript
  // ── Profile Contract ──────────────────────────────────────────────

  test('GET /profile returns { data: UserProfile } contract shape', async () => {
    const res = await apiRequest(ctx.app, 'GET', '/profile')
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body).toHaveProperty('data')
    expect(body.data).toHaveProperty('id')
    expect(body.data).toHaveProperty('name')
    expect(body.data).toHaveProperty('email')
    expect(body.data).toHaveProperty('phone')
    expect(body.data).toHaveProperty('location')
    expect(body.data).toHaveProperty('linkedin')
    expect(body.data).toHaveProperty('github')
    expect(body.data).toHaveProperty('website')
    expect(body.data).toHaveProperty('clearance')
    expect(body.data).toHaveProperty('created_at')
    expect(body.data).toHaveProperty('updated_at')
    expect(body).not.toHaveProperty('pagination')
    expect(body).not.toHaveProperty('error')
  })

  test('PATCH /profile returns { data: UserProfile } contract shape', async () => {
    const res = await apiRequest(ctx.app, 'PATCH', '/profile', { name: 'Contract Test' })
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body).toHaveProperty('data')
    expect(body.data.name).toBe('Contract Test')
    expect(body).not.toHaveProperty('error')
  })

  test('PATCH /profile validation error returns { error } contract shape', async () => {
    const res = await apiRequest(ctx.app, 'PATCH', '/profile', { name: '' })
    expect(res.status).toBe(400)
    const body = await res.json()
    expect(body).toHaveProperty('error')
    expect(body.error).toHaveProperty('code')
    expect(body.error).toHaveProperty('message')
    expect(body).not.toHaveProperty('data')
  })
```

#### SDK Resource Tests

Add to `packages/sdk/src/__tests__/resources.test.ts`:

```typescript
  // -----------------------------------------------------------------------
  // profile
  // -----------------------------------------------------------------------

  describe('profile', () => {
    it('get sends GET /api/profile', async () => {
      const profileData = { id: 'p1', name: 'Adam', email: 'adam@test.com' }
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: profileData })),
      )

      const result = await client.profile.get()

      expect(calledUrl(fetchMock)).toBe('http://localhost:3000/api/profile')
      expect(calledInit(fetchMock).method).toBe('GET')
      expect(result).toEqual({ ok: true, data: profileData })
    })

    it('update sends PATCH /api/profile with body', async () => {
      const updated = { id: 'p1', name: 'Updated', email: 'new@test.com' }
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: updated })),
      )

      const result = await client.profile.update({ name: 'Updated', email: 'new@test.com' })

      expect(calledUrl(fetchMock)).toBe('http://localhost:3000/api/profile')
      expect(calledInit(fetchMock).method).toBe('PATCH')
      expect(JSON.parse(calledInit(fetchMock).body as string)).toEqual({
        name: 'Updated',
        email: 'new@test.com',
      })
      expect(result).toEqual({ ok: true, data: updated })
    })
  })
```

#### Compiler Tests Update

**File:** `packages/core/src/services/__tests__/resume-compiler.test.ts`

Add or update tests to verify profile-based contact fields:

```typescript
  describe('parseHeader with profile', () => {
    it('uses profile contact fields instead of resume header', () => {
      const resumeId = seedResume(db, { name: 'Resume Name' })
      seedProfile(db, {
        name: 'Adam',
        email: 'adam@test.com',
        phone: '+1-555-0123',
        location: 'DC',
      })
      // Set resume header JSON with a tagline
      db.run("UPDATE resumes SET header = ? WHERE id = ?", [
        JSON.stringify({ tagline: 'Security Engineer' }),
        resumeId,
      ])

      const ir = compileResumeIR(db, resumeId)
      expect(ir).not.toBeNull()
      expect(ir!.header.name).toBe('Adam')  // from profile
      expect(ir!.header.email).toBe('adam@test.com')  // from profile
      expect(ir!.header.phone).toBe('+1-555-0123')  // from profile
      expect(ir!.header.location).toBe('DC')  // from profile
      expect(ir!.header.tagline).toBe('Security Engineer')  // from resume header
    })

    it('falls back to resume name when profile is missing', () => {
      const resumeId = seedResume(db, { name: 'Fallback Resume' })
      db.run('DELETE FROM user_profile')

      const ir = compileResumeIR(db, resumeId)
      expect(ir).not.toBeNull()
      expect(ir!.header.name).toBe('Fallback Resume')
    })

    it('tagline defaults to target_role when header JSON has no tagline', () => {
      const resumeId = seedResume(db, { targetRole: 'AI Engineer' })
      seedProfile(db, { name: 'Adam' })

      const ir = compileResumeIR(db, resumeId)
      expect(ir!.header.tagline).toBe('AI Engineer')
    })

    it('contact info changes when profile is updated', () => {
      const resumeId = seedResume(db)
      seedProfile(db, { name: 'Before', email: 'old@test.com' })

      const before = compileResumeIR(db, resumeId)
      expect(before!.header.email).toBe('old@test.com')

      // Update profile
      db.run("UPDATE user_profile SET email = 'new@test.com'")

      const after = compileResumeIR(db, resumeId)
      expect(after!.header.email).toBe('new@test.com')
    })
  })
```

#### Smoke Tests

```typescript
  test('migration 005 creates user_profile table', () => {
    const db = createTestDb()
    const tables = db.query(
      "SELECT name FROM sqlite_master WHERE type='table' AND name='user_profile'"
    ).all() as { name: string }[]
    expect(tables).toHaveLength(1)
    expect(tables[0].name).toBe('user_profile')
    db.close()
  })

  test('migration 005 seeds exactly one profile row', () => {
    const db = createTestDb()
    const count = db.query('SELECT COUNT(*) as n FROM user_profile').get() as { n: number }
    expect(count.n).toBe(1)
    db.close()
  })

  test('migration 005 seeds profile from existing resume header JSON', () => {
    // Create a DB with only migrations 001-004, insert a resume with header blob,
    // then run migration 005 and verify the profile was seeded from that header.
    const db = createTestDb({ throughMigration: '004_resume_sections' })
    const resumeId = testUuid()
    db.run(
      `INSERT INTO resumes (id, name, target_role, header, created_at, updated_at)
       VALUES (?, ?, ?, ?, datetime('now'), datetime('now'))`,
      [resumeId, 'Test Resume', 'Engineer', JSON.stringify({
        name: 'Adam',
        email: 'adam@test.com',
        phone: '+1-555-0123',
        location: 'DC',
        linkedin: 'linkedin.com/in/adam',
        github: 'github.com/adam',
        website: 'adam.dev',
        clearance: 'TS/SCI',
      })]
    )
    // Now run migration 005
    runMigration(db, '005_user_profile')
    const profile = db.query('SELECT * FROM user_profile LIMIT 1').get() as any
    expect(profile).not.toBeNull()
    expect(profile.name).toBe('Adam')
    expect(profile.email).toBe('adam@test.com')
    expect(profile.phone).toBe('+1-555-0123')
    expect(profile.location).toBe('DC')
    expect(profile.clearance).toBe('TS/SCI')
    db.close()
  })
```

### Expected Test Commands and Output

```bash
# Run all profile-related tests
bun test packages/core/src/db/repositories/__tests__/profile-repository.test.ts
bun test packages/core/src/services/__tests__/profile-service.test.ts
bun test packages/core/src/routes/__tests__/profile.test.ts
bun test packages/core/src/services/__tests__/resume-compiler.test.ts

# Run SDK resource tests
bun test packages/sdk/src/__tests__/resources.test.ts

# Run all tests to verify no regressions
bun test

# Expected: all tests pass, no regressions
```

---

## Documentation Requirements

- **Repository file:** JSDoc on `get()` and `update()` functions describing single-row semantics and the ALLOWED_FIELDS allowlist. Document the single-row invariant in JSDoc on `ProfileRepository`: "The user_profile table is expected to contain exactly one row. This is enforced at the application level (not the DB level) via the migration seeding logic."
- **Service file:** JSDoc on class and methods describing validation rules (name must not be empty, name cannot be null).
- **Route file:** Module-level comment listing the two endpoints.
- **Migration file:** Step-by-step comments explaining the seed logic.
- **SDK resource:** JSDoc on `get()` and `update()` methods.
- **No README or external docs** -- inline documentation only.

---

## Parallelization Notes

Within this phase, the following tasks can run in parallel:

**Group A (can run in parallel -- no dependencies between them):**
- T29.1 (migration SQL)
- T29.9 (SDK types)
- T29.13 (config layout)

**Group A.1 (depends on T29.9):**
- T29.10 (SDK resource) -- depends on T29.9 (SDK types must exist before the resource can import them)

**Group B (depends on T29.2 core types):**
- T29.3 (repository) -- depends on T29.2
- T29.4 (service) -- depends on T29.2, T29.3
- T29.5 (service registration) -- depends on T29.4

**Group C (depends on Group B):**
- T29.6 (API routes) -- depends on T29.4, T29.5
- T29.7 (route mounting) -- depends on T29.6

**Group D (depends on T29.2):**
- T29.8 (IR compiler) -- depends on T29.2

**Group E (depends on Groups A-C):**
- T29.11 (SDK client registration) -- depends on T29.10
- T29.12 (SDK exports) -- depends on T29.9, T29.10, T29.11
- T29.14 (profile editor page) -- depends on T29.12, T29.13
- T29.15 (HeaderEditor update) -- depends on T29.12
- T29.16 (nav update) -- no code dependency, just needs T29.13 to exist
- T29.17 (test helper) -- depends on T29.1

**Recommended execution order:**
1. T29.1 + T29.2 + T29.9 + T29.13 (parallel)
2. T29.3 + T29.10 + T29.17 (parallel, after step 1 -- T29.10 depends on T29.9 from step 1)
3. T29.4 + T29.8 + T29.11 (parallel, after step 2)
4. T29.5 + T29.12 (parallel, after step 3)
5. T29.6 + T29.16 (parallel, after step 4)
6. T29.7 + T29.14 + T29.15 (parallel, after step 5)
7. All tests (after step 6)
