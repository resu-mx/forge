# Phase 12: API Routes + SDK Updates

**Goal:** Update all HTTP route handlers and SDK types/resources for the evolved schema: polymorphic sources, bullet many-to-many, resume entries (replacing perspectives), organizations, user notes, and integrity drift.

**Non-Goals:** No business logic in routes. Routes parse requests, call services, format responses. No UI work.

**Depends on:** Phase 11 (services)
**Blocks:** Phase 13 (CLI/Import), Phase 14 (UI)

---

## Task 12.1: Update Source Routes

**Goal:** Make source routes handle polymorphic `source_type` + extension data, remove `employer_id`/`project_id` filters.

**File:** `packages/core/src/routes/sources.ts`

**Changes:**

1. `POST /sources` accepts `source_type` + extension fields in body. The service creates base + extension atomically.
2. `GET /sources` supports `?source_type=role` filter. Removes `?employer_id` and `?project_id` query params. Returns `source_type` + extension data on each source.
3. `GET /sources/:id` response includes extension data (role fields, education fields, etc.) based on `source_type`.
4. `PATCH /sources/:id` can update extension fields alongside base fields.

**Implementation:**

```typescript
/**
 * Source routes — thin HTTP layer over SourceService and DerivationService.
 * Updated for polymorphic source_type + extension tables.
 */

import { Hono } from 'hono'
import type { Services } from '../services'
import { mapStatusCode } from './server'
import type { CreateSource, UpdateSource } from '../types'

export function sourceRoutes(services: Services) {
  const app = new Hono()

  app.post('/sources', async (c) => {
    const body = await c.req.json<CreateSource>()
    const result = services.sources.createSource(body)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data }, 201)
  })

  app.get('/sources', (c) => {
    const offset = Math.max(0, parseInt(c.req.query('offset') ?? '0', 10) || 0)
    const limit = Math.min(200, Math.max(1, parseInt(c.req.query('limit') ?? '50', 10) || 50))
    const filter: Record<string, string> = {}
    if (c.req.query('source_type')) filter.source_type = c.req.query('source_type')!
    if (c.req.query('status')) filter.status = c.req.query('status')!

    const result = services.sources.listSources(filter, offset, limit)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data, pagination: result.pagination })
  })

  app.get('/sources/:id', (c) => {
    const result = services.sources.getSource(c.req.param('id'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.patch('/sources/:id', async (c) => {
    const body = await c.req.json<UpdateSource>()
    const result = services.sources.updateSource(c.req.param('id'), body)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.delete('/sources/:id', (c) => {
    const result = services.sources.deleteSource(c.req.param('id'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.body(null, 204)
  })

  app.post('/sources/:id/derive-bullets', async (c) => {
    const result = await services.derivation.deriveBulletsFromSource(c.req.param('id'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data }, 201)
  })

  return app
}
```

**Key differences from current code:**
- `GET /sources` filter removes `employer_id`, `project_id`; adds `source_type`
- `CreateSource` type now includes `source_type` + optional extension objects (`role`, `project`, `education`, `clearance`)
- `UpdateSource` type now includes optional extension updates
- Response payloads include `source_type` and the matching extension object

**Acceptance Criteria:**
- [ ] `POST /sources` with `source_type: 'role'` + `role: { organization_id, start_date, ... }` creates source + source_roles row atomically, returns 201 with extension data
- [ ] `POST /sources` with `source_type: 'education'` + `education: { ... }` creates source + source_education row atomically
- [ ] `POST /sources` without `source_type` defaults to `'general'`
- [ ] `GET /sources?source_type=role` returns only role sources with extension data
- [ ] `GET /sources` (no filter) returns all sources, each with their extension data
- [ ] `GET /sources/:id` includes full extension data
- [ ] `PATCH /sources/:id` can update extension fields (e.g., `role.is_current`)
- [ ] No `employer_id` or `project_id` query params accepted (old params are silently ignored, not errored)
- [ ] Derive-bullets still works — creates `bullet_sources` row (via service)

**Tests:**
```typescript
// packages/core/src/routes/__tests__/sources.test.ts

test('POST /sources with role type → 201 with extension', async () => {
  // Create org first for FK
  const orgRes = await apiRequest(ctx.app, 'POST', '/organizations', { name: 'Acme' })
  const org = (await orgRes.json()).data

  const res = await apiRequest(ctx.app, 'POST', '/sources', {
    title: 'Senior Engineer',
    description: 'Led platform team.',
    source_type: 'role',
    role: {
      organization_id: org.id,
      start_date: '2023-01-01',
      is_current: true,
    },
  })

  expect(res.status).toBe(201)
  const body = await res.json()
  expect(body.data.source_type).toBe('role')
  expect(body.data.role.organization_id).toBe(org.id)
  expect(body.data.role.is_current).toBe(true)
})

test('POST /sources defaults to general → 201', async () => {
  const res = await apiRequest(ctx.app, 'POST', '/sources', {
    title: 'General Note',
    description: 'Some context.',
  })

  expect(res.status).toBe(201)
  const body = await res.json()
  expect(body.data.source_type).toBe('general')
})

test('GET /sources?source_type=role filters correctly', async () => {
  // Seed one role source and one general source
  seedSource(ctx.db, { sourceType: 'role' })
  seedSource(ctx.db, { sourceType: 'general' })

  const res = await apiRequest(ctx.app, 'GET', '/sources?source_type=role')

  expect(res.status).toBe(200)
  const body = await res.json()
  expect(body.data.length).toBe(1)
  expect(body.data[0].source_type).toBe('role')
})

test('GET /sources/:id includes extension data', async () => {
  const id = seedSource(ctx.db, { sourceType: 'education' })

  const res = await apiRequest(ctx.app, 'GET', `/sources/${id}`)

  expect(res.status).toBe(200)
  const body = await res.json()
  expect(body.data.source_type).toBe('education')
  expect(body.data.education).toBeTruthy()
})

test('PATCH /sources/:id updates extension fields', async () => {
  const id = seedSource(ctx.db, { sourceType: 'role' })

  const res = await apiRequest(ctx.app, 'PATCH', `/sources/${id}`, {
    role: { is_current: false },
  })

  expect(res.status).toBe(200)
  const body = await res.json()
  expect(body.data.role.is_current).toBe(false)
})
```

---

## Task 12.2: Update Bullet Routes

**Goal:** Bullet responses return `sources` array instead of `source_id`. The `?source_id=X` filter works via `bullet_sources` JOIN in the service layer.

**File:** `packages/core/src/routes/bullets.ts`

**Changes:**

1. Response for `GET /bullets` and `GET /bullets/:id` includes `sources: Array<{ id, title, is_primary }>` instead of `source_id`.
2. `?source_id=X` filter still accepted — the service handles the JOIN through `bullet_sources`.
3. Response includes `domain` field (nullable TEXT, from v1 framing import).

**Implementation:**

The route file itself changes minimally — the service layer handles the JOIN. The key change is that the response shape is different (no `source_id`, has `sources` array and `domain`).

```typescript
/**
 * Bullet routes — thin HTTP layer over BulletService and DerivationService.
 * Updated: responses use `sources` array (not source_id), includes `domain`.
 */

import { Hono } from 'hono'
import type { Services } from '../services'
import { mapStatusCode } from './server'

export function bulletRoutes(services: Services) {
  const app = new Hono()

  app.get('/bullets', (c) => {
    const offset = Math.max(0, parseInt(c.req.query('offset') ?? '0', 10) || 0)
    const limit = Math.min(200, Math.max(1, parseInt(c.req.query('limit') ?? '50', 10) || 50))
    const filter: Record<string, string> = {}
    if (c.req.query('source_id')) filter.source_id = c.req.query('source_id')!
    if (c.req.query('status')) filter.status = c.req.query('status')!
    if (c.req.query('technology')) filter.technology = c.req.query('technology')!

    const result = services.bullets.listBullets(filter, offset, limit)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data, pagination: result.pagination })
  })

  // ... remaining routes unchanged in structure (same as current)

  return app
}
```

**Acceptance Criteria:**
- [ ] `GET /bullets` response items have `sources: [{ id, title, is_primary }]` array, no `source_id` field
- [ ] `GET /bullets` response items have `domain: string | null`
- [ ] `GET /bullets?source_id=X` returns only bullets associated with that source via `bullet_sources`
- [ ] `GET /bullets/:id` response has `sources` array and `domain`
- [ ] Derive-perspectives still works (unchanged route, service handles `bullet_sources`)

**Tests:**
```typescript
test('GET /bullets → response has sources array, no source_id', async () => {
  const sourceId = seedSource(ctx.db)
  seedBullet(ctx.db, [{ id: sourceId, isPrimary: true }])

  const res = await apiRequest(ctx.app, 'GET', '/bullets')

  expect(res.status).toBe(200)
  const body = await res.json()
  expect(body.data[0].sources).toBeArray()
  expect(body.data[0].sources[0].is_primary).toBe(true)
  expect(body.data[0].source_id).toBeUndefined()
})

test('GET /bullets?source_id=X filters via junction', async () => {
  const s1 = seedSource(ctx.db)
  const s2 = seedSource(ctx.db)
  seedBullet(ctx.db, [{ id: s1, isPrimary: true }])
  seedBullet(ctx.db, [{ id: s2, isPrimary: true }])

  const res = await apiRequest(ctx.app, 'GET', `/bullets?source_id=${s1}`)

  expect(res.status).toBe(200)
  const body = await res.json()
  expect(body.data.length).toBe(1)
  expect(body.data[0].sources[0].id).toBe(s1)
})

test('GET /bullets/:id has domain field', async () => {
  const sourceId = seedSource(ctx.db)
  const bulletId = seedBullet(ctx.db, [{ id: sourceId, isPrimary: true }], { domain: 'devops' })

  const res = await apiRequest(ctx.app, 'GET', `/bullets/${bulletId}`)

  expect(res.status).toBe(200)
  const body = await res.json()
  expect(body.data.domain).toBe('devops')
})
```

---

## Task 12.3: Resume Entry Routes (Replace Perspective Routes)

**Goal:** Replace resume perspective endpoints with resume entry endpoints. Entries support copy-on-write (content = NULL = reference mode, content = string = clone mode, explicit `content: null` in PATCH = reset to reference).

**File:** `packages/core/src/routes/resumes.ts`

**Changes:**

1. Remove `POST /resumes/:id/perspectives`, `DELETE /resumes/:id/perspectives/:pid`, `PATCH /resumes/:id/reorder`
2. Add `POST /resumes/:id/entries` — create resume entry (perspective_id, section, position)
3. Add `GET /resumes/:id/entries` — list entries for a resume
4. Add `PATCH /resumes/:id/entries/:entryId` — update entry (content, section, position, notes). `content: null` resets to reference mode.
5. Add `DELETE /resumes/:id/entries/:entryId` — remove entry
6. `GET /resumes/:id` returns `ResumeWithEntries` (entries grouped by section)

**Implementation:**

```typescript
/**
 * Resume routes — thin HTTP layer over ResumeService.
 * Updated: uses resume_entries instead of resume_perspectives.
 */

import { Hono } from 'hono'
import type { Services } from '../services'
import { mapStatusCode } from './server'
import type { CreateResume, UpdateResume } from '../types'

export function resumeRoutes(services: Services) {
  const app = new Hono()

  // ── Resume CRUD (unchanged structure) ─────────────────────────────

  app.post('/resumes', async (c) => {
    const body = await c.req.json<CreateResume>()
    const result = services.resumes.createResume(body)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data }, 201)
  })

  app.get('/resumes', (c) => {
    const offset = Math.max(0, parseInt(c.req.query('offset') ?? '0', 10) || 0)
    const limit = Math.min(200, Math.max(1, parseInt(c.req.query('limit') ?? '50', 10) || 50))

    const result = services.resumes.listResumes(offset, limit)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data, pagination: result.pagination })
  })

  app.get('/resumes/:id', (c) => {
    // Returns ResumeWithEntries — entries grouped by section
    const result = services.resumes.getResume(c.req.param('id'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.patch('/resumes/:id', async (c) => {
    const body = await c.req.json<UpdateResume>()
    const result = services.resumes.updateResume(c.req.param('id'), body)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.delete('/resumes/:id', (c) => {
    const result = services.resumes.deleteResume(c.req.param('id'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.body(null, 204)
  })

  // ── Resume Entries (NEW — replaces perspective endpoints) ─────────

  app.post('/resumes/:id/entries', async (c) => {
    const body = await c.req.json<{
      perspective_id: string
      section: string
      position: number
    }>()
    const result = services.resumes.addEntry(c.req.param('id'), body)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data }, 201)
  })

  app.get('/resumes/:id/entries', (c) => {
    const result = services.resumes.listEntries(c.req.param('id'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.patch('/resumes/:id/entries/:entryId', async (c) => {
    // Distinguish between "content key absent" (no change) and "content: null" (reset)
    const raw = await c.req.text()
    const body = JSON.parse(raw)
    const hasContentKey = 'content' in body

    const result = services.resumes.updateEntry(
      c.req.param('id'),
      c.req.param('entryId'),
      body,
      hasContentKey,
    )
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.delete('/resumes/:id/entries/:entryId', (c) => {
    const result = services.resumes.removeEntry(c.req.param('id'), c.req.param('entryId'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.body(null, 204)
  })

  // ── Gaps + Export (unchanged) ─────────────────────────────────────

  app.get('/resumes/:id/gaps', (c) => {
    const result = services.resumes.analyzeGaps(c.req.param('id'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.post('/resumes/:id/export', (c) => {
    return c.json(
      { error: { code: 'NOT_IMPLEMENTED', message: 'Resume export is not yet implemented' } },
      501,
    )
  })

  return app
}
```

**Copy-on-write PATCH semantics:**
- `PATCH { section: 'education' }` — updates section, does NOT touch content
- `PATCH { content: 'Edited text' }` — clones: sets content + captures `perspective_content_snapshot`
- `PATCH { content: null }` — resets to reference mode: sets content = NULL
- Absent `content` key in PATCH body = no change to content

The route uses `'content' in body` check (parsing raw text then JSON.parse) to distinguish absent vs explicit null. The `hasContentKey` flag is passed to the service.

**Acceptance Criteria:**
- [ ] `POST /resumes/:id/entries` with `{ perspective_id, section, position }` creates entry, returns 201 with entry data
- [ ] `POST /resumes/:id/entries` with non-existent perspective_id returns 400
- [ ] `GET /resumes/:id/entries` returns array of entries with resolved content (reference or cloned)
- [ ] `PATCH /resumes/:id/entries/:entryId` with `{ content: "Edited" }` clones — sets content + perspective_content_snapshot
- [ ] `PATCH /resumes/:id/entries/:entryId` with `{ content: null }` resets to reference mode — content becomes NULL
- [ ] `PATCH` with absent `content` key does NOT modify content
- [ ] `PATCH` with `{ section: 'education', position: 2 }` updates section and position
- [ ] `DELETE /resumes/:id/entries/:entryId` removes entry, returns 204
- [ ] `GET /resumes/:id` returns `ResumeWithEntries` (sections: Record<string, ResumeEntry[]>)
- [ ] Old perspective endpoints (`POST /resumes/:id/perspectives`, `DELETE .../perspectives/:pid`, `PATCH .../reorder`) are removed — return 404
- [ ] Gap analysis still works (source title lookup via `bullet_sources` JOIN in service)

**Tests:**
```typescript
test('POST /resumes/:id/entries → 201', async () => {
  const resumeId = seedResume(ctx.db)
  const srcId = seedSource(ctx.db)
  const bulletId = seedBullet(ctx.db, [{ id: srcId }])
  const perspId = seedPerspective(ctx.db, bulletId)

  const res = await apiRequest(ctx.app, 'POST', `/resumes/${resumeId}/entries`, {
    perspective_id: perspId,
    section: 'work_history',
    position: 0,
  })

  expect(res.status).toBe(201)
  const body = await res.json()
  expect(body.data.perspective_id).toBe(perspId)
  expect(body.data.content).toBeNull() // reference mode
})

test('PATCH entry content: null resets to reference mode', async () => {
  const { resumeId, entryId } = seedResumeEntry(ctx.db)

  // First clone
  await apiRequest(ctx.app, 'PATCH', `/resumes/${resumeId}/entries/${entryId}`, {
    content: 'Edited version',
  })

  // Then reset
  const res = await apiRequest(ctx.app, 'PATCH', `/resumes/${resumeId}/entries/${entryId}`, {
    content: null,
  })

  expect(res.status).toBe(200)
  const body = await res.json()
  expect(body.data.content).toBeNull()
})

test('GET /resumes/:id returns entries grouped by section', async () => {
  const resumeId = seedResume(ctx.db)
  seedResumeEntryForSection(ctx.db, resumeId, 'work_history', 0)
  seedResumeEntryForSection(ctx.db, resumeId, 'work_history', 1)
  seedResumeEntryForSection(ctx.db, resumeId, 'education', 0)

  const res = await apiRequest(ctx.app, 'GET', `/resumes/${resumeId}`)

  expect(res.status).toBe(200)
  const body = await res.json()
  expect(body.data.sections.work_history.length).toBe(2)
  expect(body.data.sections.education.length).toBe(1)
})

test('old perspective endpoints return 404', async () => {
  const resumeId = seedResume(ctx.db)

  const res = await apiRequest(ctx.app, 'POST', `/resumes/${resumeId}/perspectives`, {
    perspective_id: 'fake',
    section: 'work_history',
    position: 0,
  })

  expect(res.status).toBe(404)
})
```

---

## Task 12.4: Organization Routes

**Goal:** Full CRUD for organizations. Filter by `org_type` and `worked`.

**File:** `packages/core/src/routes/organizations.ts` (NEW)

**Implementation:**

```typescript
/**
 * Organization routes — CRUD for organizations (replaces employers).
 */

import { Hono } from 'hono'
import type { Services } from '../services'
import { mapStatusCode } from './server'

export function organizationRoutes(services: Services) {
  const app = new Hono()

  app.post('/organizations', async (c) => {
    const body = await c.req.json()
    const result = services.organizations.create(body)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data }, 201)
  })

  app.get('/organizations', (c) => {
    const offset = Math.max(0, parseInt(c.req.query('offset') ?? '0', 10) || 0)
    const limit = Math.min(200, Math.max(1, parseInt(c.req.query('limit') ?? '50', 10) || 50))
    const filter: Record<string, string> = {}
    if (c.req.query('org_type')) filter.org_type = c.req.query('org_type')!
    if (c.req.query('worked')) filter.worked = c.req.query('worked')!

    const result = services.organizations.list(filter, offset, limit)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data, pagination: result.pagination })
  })

  app.get('/organizations/:id', (c) => {
    const result = services.organizations.get(c.req.param('id'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.patch('/organizations/:id', async (c) => {
    const body = await c.req.json()
    const result = services.organizations.update(c.req.param('id'), body)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.delete('/organizations/:id', (c) => {
    // Delete sets NULL on source_roles.organization_id (FK ON DELETE SET NULL)
    const result = services.organizations.delete(c.req.param('id'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.body(null, 204)
  })

  return app
}
```

**Registration in `server.ts`:**
```typescript
import { organizationRoutes } from './organizations'
// ...
app.route('/', organizationRoutes(services))
```

**Acceptance Criteria:**
- [ ] `POST /organizations` with `{ name: 'Anthropic', org_type: 'company', worked: true }` returns 201
- [ ] `POST /organizations` with empty name returns 400 VALIDATION_ERROR
- [ ] `POST /organizations` without `org_type` defaults to `'company'`
- [ ] `GET /organizations` returns paginated list
- [ ] `GET /organizations?org_type=military` filters by org type
- [ ] `GET /organizations?worked=1` filters to orgs where user worked
- [ ] `GET /organizations/:id` returns full organization
- [ ] `GET /organizations/:nonexistent` returns 404
- [ ] `PATCH /organizations/:id` updates fields
- [ ] `DELETE /organizations/:id` returns 204, sets NULL on `source_roles.organization_id`

**Tests:**
```typescript
// packages/core/src/routes/__tests__/organizations.test.ts

describe('Organization Routes', () => {
  let ctx: TestContext

  beforeEach(() => { ctx = createTestApp() })
  afterEach(() => { ctx.db.close() })

  test('POST /organizations → 201', async () => {
    const res = await apiRequest(ctx.app, 'POST', '/organizations', {
      name: 'Anthropic',
      org_type: 'company',
      industry: 'AI',
      worked: true,
    })

    expect(res.status).toBe(201)
    const body = await res.json()
    expect(body.data.name).toBe('Anthropic')
    expect(body.data.org_type).toBe('company')
    expect(body.data.worked).toBe(true)
    expect(body.data.id).toBeTruthy()
  })

  test('POST /organizations empty name → 400', async () => {
    const res = await apiRequest(ctx.app, 'POST', '/organizations', { name: '' })
    expect(res.status).toBe(400)
  })

  test('GET /organizations?org_type=company filters', async () => {
    await apiRequest(ctx.app, 'POST', '/organizations', { name: 'Acme', org_type: 'company' })
    await apiRequest(ctx.app, 'POST', '/organizations', { name: 'Army', org_type: 'military' })

    const res = await apiRequest(ctx.app, 'GET', '/organizations?org_type=company')

    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.length).toBe(1)
    expect(body.data[0].name).toBe('Acme')
  })

  test('GET /organizations?worked=1 filters', async () => {
    await apiRequest(ctx.app, 'POST', '/organizations', { name: 'Worked', worked: true })
    await apiRequest(ctx.app, 'POST', '/organizations', { name: 'NotWorked', worked: false })

    const res = await apiRequest(ctx.app, 'GET', '/organizations?worked=1')

    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.length).toBe(1)
    expect(body.data[0].name).toBe('Worked')
  })

  test('DELETE /organizations/:id cascades SET NULL on source_roles', async () => {
    const orgRes = await apiRequest(ctx.app, 'POST', '/organizations', { name: 'ToDelete' })
    const orgId = (await orgRes.json()).data.id

    // Create a role source referencing this org
    await apiRequest(ctx.app, 'POST', '/sources', {
      title: 'Engineer',
      description: 'Role at ToDelete',
      source_type: 'role',
      role: { organization_id: orgId },
    })

    const delRes = await apiRequest(ctx.app, 'DELETE', `/organizations/${orgId}`)
    expect(delRes.status).toBe(204)

    // Source still exists, but org reference is NULL
    const sourcesRes = await apiRequest(ctx.app, 'GET', '/sources?source_type=role')
    const sources = (await sourcesRes.json()).data
    expect(sources[0].role.organization_id).toBeNull()
  })
})
```

---

## Task 12.5: Note Routes

**Goal:** CRUD for user notes with cross-entity reference linking.

**File:** `packages/core/src/routes/notes.ts` (NEW)

**Implementation:**

```typescript
/**
 * User note routes — CRUD for user_notes + note_references.
 */

import { Hono } from 'hono'
import type { Services } from '../services'
import { mapStatusCode } from './server'

export function noteRoutes(services: Services) {
  const app = new Hono()

  app.post('/notes', async (c) => {
    const body = await c.req.json<{ title?: string; content: string }>()
    const result = services.notes.create(body)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data }, 201)
  })

  app.get('/notes', (c) => {
    const offset = Math.max(0, parseInt(c.req.query('offset') ?? '0', 10) || 0)
    const limit = Math.min(200, Math.max(1, parseInt(c.req.query('limit') ?? '50', 10) || 50))
    const search = c.req.query('search')

    const result = services.notes.list({ search }, offset, limit)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data, pagination: result.pagination })
  })

  app.get('/notes/:id', (c) => {
    const result = services.notes.get(c.req.param('id'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.patch('/notes/:id', async (c) => {
    const body = await c.req.json<{ title?: string; content?: string }>()
    const result = services.notes.update(c.req.param('id'), body)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.delete('/notes/:id', (c) => {
    // Cascades to note_references
    const result = services.notes.delete(c.req.param('id'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.body(null, 204)
  })

  // ── Note References ───────────────────────────────────────────────

  app.post('/notes/:id/references', async (c) => {
    const body = await c.req.json<{ entity_type: string; entity_id: string }>()
    const result = services.notes.addReference(c.req.param('id'), body)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: null }, 201)
  })

  app.delete('/notes/:id/references/:entityType/:entityId', (c) => {
    const result = services.notes.removeReference(
      c.req.param('id'),
      c.req.param('entityType'),
      c.req.param('entityId'),
    )
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.body(null, 204)
  })

  return app
}
```

**Registration in `server.ts`:**
```typescript
import { noteRoutes } from './notes'
// ...
app.route('/', noteRoutes(services))
```

**Acceptance Criteria:**
- [ ] `POST /notes` with `{ content: 'Remember to check...' }` returns 201
- [ ] `POST /notes` with `{ title: 'Interview prep', content: '...' }` returns 201
- [ ] `POST /notes` with empty content returns 400 VALIDATION_ERROR
- [ ] `GET /notes` returns paginated list, each note includes its `references` array
- [ ] `GET /notes?search=kubernetes` returns notes containing "kubernetes" in content
- [ ] `GET /notes/:id` returns note with full `references` array
- [ ] `PATCH /notes/:id` updates title and/or content
- [ ] `DELETE /notes/:id` returns 204, cascades to `note_references`
- [ ] `POST /notes/:id/references` with `{ entity_type: 'source', entity_id: 'uuid' }` links entity, returns 201
- [ ] `POST /notes/:id/references` with invalid `entity_type` returns 400
- [ ] `DELETE /notes/:id/references/:entityType/:entityId` unlinks entity, returns 204

**Tests:**
```typescript
// packages/core/src/routes/__tests__/notes.test.ts

describe('Note Routes', () => {
  let ctx: TestContext

  beforeEach(() => { ctx = createTestApp() })
  afterEach(() => { ctx.db.close() })

  test('POST /notes → 201', async () => {
    const res = await apiRequest(ctx.app, 'POST', '/notes', {
      title: 'Interview prep',
      content: 'Remember to mention Kubernetes experience.',
    })

    expect(res.status).toBe(201)
    const body = await res.json()
    expect(body.data.title).toBe('Interview prep')
    expect(body.data.content).toContain('Kubernetes')
  })

  test('GET /notes?search=kubernetes returns matching notes', async () => {
    await apiRequest(ctx.app, 'POST', '/notes', { content: 'Kubernetes is key' })
    await apiRequest(ctx.app, 'POST', '/notes', { content: 'Python is also useful' })

    const res = await apiRequest(ctx.app, 'GET', '/notes?search=kubernetes')

    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.length).toBe(1)
    expect(body.data[0].content).toContain('Kubernetes')
  })

  test('POST /notes/:id/references links entity', async () => {
    const noteRes = await apiRequest(ctx.app, 'POST', '/notes', { content: 'Related to source' })
    const noteId = (await noteRes.json()).data.id
    const sourceId = seedSource(ctx.db)

    const res = await apiRequest(ctx.app, 'POST', `/notes/${noteId}/references`, {
      entity_type: 'source',
      entity_id: sourceId,
    })

    expect(res.status).toBe(201)

    // Verify reference shows up on GET
    const getRes = await apiRequest(ctx.app, 'GET', `/notes/${noteId}`)
    const note = (await getRes.json()).data
    expect(note.references.length).toBe(1)
    expect(note.references[0].entity_type).toBe('source')
    expect(note.references[0].entity_id).toBe(sourceId)
  })

  test('DELETE /notes/:id/references/:entityType/:entityId unlinks', async () => {
    const noteRes = await apiRequest(ctx.app, 'POST', '/notes', { content: 'test' })
    const noteId = (await noteRes.json()).data.id
    const sourceId = seedSource(ctx.db)

    await apiRequest(ctx.app, 'POST', `/notes/${noteId}/references`, {
      entity_type: 'source',
      entity_id: sourceId,
    })

    const res = await apiRequest(ctx.app, 'DELETE', `/notes/${noteId}/references/source/${sourceId}`)
    expect(res.status).toBe(204)
  })

  test('POST /notes/:id/references with invalid entity_type → 400', async () => {
    const noteRes = await apiRequest(ctx.app, 'POST', '/notes', { content: 'test' })
    const noteId = (await noteRes.json()).data.id

    const res = await apiRequest(ctx.app, 'POST', `/notes/${noteId}/references`, {
      entity_type: 'invalid_type',
      entity_id: 'some-id',
    })

    expect(res.status).toBe(400)
  })
})
```

---

## Task 12.6: Integrity Drift Route

**Goal:** Single endpoint returning all entities with stale content snapshots.

**File:** `packages/core/src/routes/integrity.ts` (NEW)

**Implementation:**

```typescript
/**
 * Integrity routes — content drift detection.
 */

import { Hono } from 'hono'
import type { Services } from '../services'
import { mapStatusCode } from './server'

export function integrityRoutes(services: Services) {
  const app = new Hono()

  app.get('/integrity/drift', (c) => {
    const result = services.integrity.getDriftedEntities()
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  return app
}
```

**Registration in `server.ts`:**
```typescript
import { integrityRoutes } from './integrity'
// ...
app.route('/', integrityRoutes(services))
```

**Response shape:**
```typescript
interface DriftReport {
  bullets: Array<{
    id: string
    content: string
    source_content_snapshot: string
    current_source_description: string
    source_id: string
    source_title: string
  }>
  perspectives: Array<{
    id: string
    content: string
    bullet_content_snapshot: string
    current_bullet_content: string
    bullet_id: string
  }>
  resume_entries: Array<{
    id: string
    perspective_content_snapshot: string | null
    current_perspective_content: string
    perspective_id: string
    resume_id: string
  }>
}
```

**Acceptance Criteria:**
- [ ] `GET /integrity/drift` returns 200 with `{ bullets: [...], perspectives: [...], resume_entries: [...] }`
- [ ] Only entities where snapshot differs from current content are included
- [ ] Empty arrays when nothing has drifted
- [ ] Each drifted entity includes enough context to display a meaningful UI alert (IDs, titles, both snapshot and current values)

**Tests:**
```typescript
// packages/core/src/routes/__tests__/integrity.test.ts

describe('Integrity Routes', () => {
  let ctx: TestContext

  beforeEach(() => { ctx = createTestApp() })
  afterEach(() => { ctx.db.close() })

  test('GET /integrity/drift → 200 with empty arrays when no drift', async () => {
    const res = await apiRequest(ctx.app, 'GET', '/integrity/drift')

    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.bullets).toEqual([])
    expect(body.data.perspectives).toEqual([])
    expect(body.data.resume_entries).toEqual([])
  })

  test('GET /integrity/drift detects bullet snapshot mismatch', async () => {
    const sourceId = seedSource(ctx.db)
    seedBullet(ctx.db, [{ id: sourceId, isPrimary: true }])

    // Update source description (causes drift)
    await apiRequest(ctx.app, 'PATCH', `/sources/${sourceId}`, {
      description: 'Updated description that differs from snapshot',
    })

    const res = await apiRequest(ctx.app, 'GET', '/integrity/drift')

    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.bullets.length).toBe(1)
    expect(body.data.bullets[0].source_id).toBe(sourceId)
  })
})
```

---

## Task 12.7: Update SDK Types

**Goal:** Update all SDK type definitions for the evolved schema.

**Files to modify:**
- `packages/sdk/src/types.ts` -- add Organization, ResumeEntry, UserNote, NoteReference, SourceRole, SourceProject, SourceEducation, SourceClearance, DriftReport types. Update Source (drop employer_id/project_id, add source_type + extensions), Bullet (drop source_id, add sources array + domain), GapAnalysis (fix pre-existing type mismatch -- see below).

**Pre-existing GapAnalysis type mismatch fix:** The SDK `GapAnalysis` type must match the core service return type. If the core returns `{ archetype, domain, coverage, gaps, suggestions }` but the SDK defines different field names or missing fields, align them. Specifically, ensure both sides agree on the shape of gap items (domain coverage percentages, unused bullet arrays, etc.).

**Changes -- Modified types:**

```typescript
// Source gains source_type + extension data, loses employer_id/project_id
export interface Source {
  id: string
  title: string
  description: string
  source_type: 'role' | 'project' | 'education' | 'clearance' | 'general'
  notes: string | null
  status: 'draft' | 'approved' | 'deriving'
  updated_by: 'human' | 'ai'
  last_derived_at: string | null
  created_at: string
  updated_at: string
  // Extension data — present when source_type matches
  role?: SourceRole
  project?: SourceProject
  education?: SourceEducation
  clearance?: SourceClearance
}

// Bullet loses source_id, gains sources array + domain
export interface Bullet {
  id: string
  content: string
  source_content_snapshot: string
  technologies: string[]
  metrics: string | null
  domain: string | null
  notes: string | null
  status: 'draft' | 'pending_review' | 'approved' | 'rejected'
  rejection_reason: string | null
  prompt_log_id: string | null
  approved_at: string | null
  approved_by: string | null
  created_at: string
  sources: Array<{ id: string; title: string; is_primary: boolean }>
}

// Perspective gains notes
export interface Perspective {
  // ... existing fields ...
  notes: string | null
}

// Resume gains notes
export interface Resume {
  // ... existing fields ...
  notes: string | null
}

// ResumeWithEntries replaces ResumeWithPerspectives
export interface ResumeWithEntries extends Resume {
  sections: Record<string, ResumeEntry[]>
}

// SourceFilter changes
export interface SourceFilter {
  status?: string
  source_type?: string
  // employer_id and project_id REMOVED
}

// CreateSource changes
export interface CreateSource {
  title: string
  description: string
  source_type?: 'role' | 'project' | 'education' | 'clearance' | 'general'
  role?: Partial<SourceRole>
  project?: Partial<SourceProject>
  education?: Partial<SourceEducation>
  clearance?: Partial<SourceClearance>
}

// UpdateSource changes
export interface UpdateSource {
  title?: string
  description?: string
  role?: Partial<SourceRole>
  project?: Partial<SourceProject>
  education?: Partial<SourceEducation>
  clearance?: Partial<SourceClearance>
}

// AddResumeEntry replaces AddResumePerspective
export interface AddResumeEntry {
  perspective_id: string
  section: string
  position: number
}

// UpdateResumeEntry (new)
export interface UpdateResumeEntry {
  content?: string | null  // null = reset to reference mode
  section?: string
  position?: number
  notes?: string | null
}
```

**Changes — New types:**

```typescript
// Extension types
export interface SourceRole {
  organization_id: string | null
  start_date: string | null
  end_date: string | null
  is_current: boolean
  work_arrangement: string | null
  base_salary: number | null
  total_comp_notes: string | null
}

export interface SourceProject {
  organization_id: string | null
  is_personal: boolean
  url: string | null
  start_date: string | null
  end_date: string | null
}

export interface SourceEducation {
  education_type: 'degree' | 'certificate' | 'course' | 'self_taught'
  institution: string | null
  field: string | null
  start_date: string | null
  end_date: string | null
  is_in_progress: boolean
  credential_id: string | null
  expiration_date: string | null
  issuing_body: string | null
  url: string | null
}

export interface SourceClearance {
  level: string
  polygraph: string | null
  status: string | null
  sponsoring_agency: string | null
  investigation_date: string | null
  adjudication_date: string | null
  reinvestigation_date: string | null
  read_on: string | null
}

// Organization
export interface Organization {
  id: string
  name: string
  org_type: 'company' | 'nonprofit' | 'government' | 'military' | 'education' | 'volunteer' | 'freelance' | 'other'
  industry: string | null
  size: string | null
  worked: boolean
  employment_type: 'civilian' | 'contractor' | 'military_active' | 'military_reserve' | 'volunteer' | 'intern' | null
  location: string | null
  headquarters: string | null
  website: string | null
  linkedin_url: string | null
  glassdoor_url: string | null
  glassdoor_rating: number | null
  reputation_notes: string | null
  notes: string | null
  created_at: string
  updated_at: string
}

export interface CreateOrganization {
  name: string
  org_type?: string
  industry?: string
  size?: string
  worked?: boolean
  employment_type?: string
  location?: string
  headquarters?: string
  website?: string
  linkedin_url?: string
  glassdoor_url?: string
  glassdoor_rating?: number
  reputation_notes?: string
  notes?: string
}

export interface UpdateOrganization {
  name?: string
  org_type?: string
  industry?: string
  size?: string
  worked?: boolean
  employment_type?: string | null
  location?: string | null
  headquarters?: string | null
  website?: string | null
  linkedin_url?: string | null
  glassdoor_url?: string | null
  glassdoor_rating?: number | null
  reputation_notes?: string | null
  notes?: string | null
}

export interface OrganizationFilter {
  org_type?: string
  worked?: string  // '1' or '0' as query param
}

// Resume Entry
export interface ResumeEntry {
  id: string
  resume_id: string
  perspective_id: string
  content: string | null           // null = reference mode
  perspective_content_snapshot: string | null
  section: string
  position: number
  notes: string | null
  created_at: string
  updated_at: string
}

// User Notes
export interface UserNote {
  id: string
  title: string | null
  content: string
  references: NoteReference[]
  created_at: string
  updated_at: string
}

export interface NoteReference {
  entity_type: 'source' | 'bullet' | 'perspective' | 'resume_entry' | 'resume' | 'skill' | 'organization'
  entity_id: string
}

export interface CreateNote {
  title?: string
  content: string
}

export interface UpdateNote {
  title?: string | null
  content?: string
}

// Integrity drift
export interface DriftReport {
  bullets: DriftedBullet[]
  perspectives: DriftedPerspective[]
  resume_entries: DriftedResumeEntry[]
}

export interface DriftedBullet {
  id: string
  content: string
  source_content_snapshot: string
  current_source_description: string
  source_id: string
  source_title: string
}

export interface DriftedPerspective {
  id: string
  content: string
  bullet_content_snapshot: string
  current_bullet_content: string
  bullet_id: string
}

export interface DriftedResumeEntry {
  id: string
  perspective_content_snapshot: string | null
  current_perspective_content: string
  perspective_id: string
  resume_id: string
}
```

**Removed types:**
- `ResumeWithPerspectives` (replaced by `ResumeWithEntries`)
- `AddResumePerspective` (replaced by `AddResumeEntry`)
- `ReorderPerspectives` (individual entry updates replace bulk reorder)
- `Source.employer_id`, `Source.project_id` fields
- `Bullet.source_id` field
- `SourceFilter.employer_id`, `SourceFilter.project_id`
- `CreateSource.employer_id`, `CreateSource.project_id`
- `UpdateSource.employer_id`, `UpdateSource.project_id`

**Acceptance Criteria:**
- [ ] `Source` type has `source_type` discriminator and optional extension objects
- [ ] `Source` type does NOT have `employer_id` or `project_id`
- [ ] `Bullet` type has `sources` array and `domain`, does NOT have `source_id`
- [ ] `Organization` type has all fields from the schema
- [ ] `ResumeEntry` type has `content`, `perspective_content_snapshot`, `section`, `position`, `notes`
- [ ] `UserNote` type has `references` array of `NoteReference`
- [ ] `DriftReport` type with typed arrays for each entity type
- [ ] All input types (`Create*`, `Update*`) match the route bodies
- [ ] `SourceFilter` has `source_type`, not `employer_id`/`project_id`
- [ ] TypeScript strict mode passes (`tsc --noEmit`)
- [ ] `ResumeWithPerspectives` removed, `ResumeWithEntries` in its place

---

## Task 12.8: Update SDK Resources

**Goal:** Update existing SDK resource classes and create new ones.

**Files to modify:**
- `packages/sdk/src/resources/sources.ts` -- update for polymorphic response (source_type + extension objects in filter/responses)
- `packages/sdk/src/resources/bullets.ts` -- update for sources array in responses (no more source_id)
- `packages/sdk/src/resources/resumes.ts` -- update for entries instead of perspectives (addEntry, updateEntry, removeEntry, listEntries)

**Files to create:**
- `packages/sdk/src/resources/organizations.ts` -- new CRUD resource for organizations
- `packages/sdk/src/resources/notes.ts` -- new CRUD + reference resource for user notes
- `packages/sdk/src/resources/integrity.ts` -- new getDrift resource

**Updated SourcesResource:**

```typescript
import type {
  CreateSource,
  PaginatedResult,
  PaginationParams,
  RequestFn,
  RequestListFn,
  Result,
  Source,
  SourceFilter,
  SourceWithBullets,
  UpdateSource,
  Bullet,
} from '../types'

function toParams(filter?: Record<string, unknown>): Record<string, string> | undefined {
  if (!filter) return undefined
  const out: Record<string, string> = {}
  for (const [k, v] of Object.entries(filter)) {
    if (v !== undefined && v !== null) out[k] = String(v)
  }
  return Object.keys(out).length > 0 ? out : undefined
}

export class SourcesResource {
  constructor(
    private request: RequestFn,
    private requestList: RequestListFn,
  ) {}

  create(input: CreateSource): Promise<Result<Source>> {
    return this.request<Source>('POST', '/api/sources', input)
  }

  list(filter?: SourceFilter & PaginationParams): Promise<PaginatedResult<Source>> {
    // SourceFilter now has source_type (not employer_id/project_id)
    return this.requestList<Source>('GET', '/api/sources', toParams(filter))
  }

  get(id: string): Promise<Result<SourceWithBullets>> {
    return this.request<SourceWithBullets>('GET', `/api/sources/${id}`)
  }

  update(id: string, input: UpdateSource): Promise<Result<Source>> {
    return this.request<Source>('PATCH', `/api/sources/${id}`, input)
  }

  delete(id: string): Promise<Result<void>> {
    return this.request<void>('DELETE', `/api/sources/${id}`)
  }

  deriveBullets(id: string): Promise<Result<Bullet[]>> {
    return this.request<Bullet[]>('POST', `/api/sources/${id}/derive-bullets`)
  }
}
```

**Updated ResumesResource:**

```typescript
import type {
  AddResumeEntry,
  CreateResume,
  GapAnalysis,
  PaginatedResult,
  PaginationParams,
  RequestFn,
  RequestListFn,
  ResumeEntry,
  ResumeWithEntries,
  Result,
  Resume,
  UpdateResume,
  UpdateResumeEntry,
} from '../types'

export class ResumesResource {
  constructor(
    private request: RequestFn,
    private requestList: RequestListFn,
  ) {}

  create(input: CreateResume): Promise<Result<Resume>> {
    return this.request<Resume>('POST', '/api/resumes', input)
  }

  list(pagination?: PaginationParams): Promise<PaginatedResult<Resume>> {
    return this.requestList<Resume>('GET', '/api/resumes', toParams(pagination))
  }

  get(id: string): Promise<Result<ResumeWithEntries>> {
    return this.request<ResumeWithEntries>('GET', `/api/resumes/${id}`)
  }

  update(id: string, input: UpdateResume): Promise<Result<Resume>> {
    return this.request<Resume>('PATCH', `/api/resumes/${id}`, input)
  }

  delete(id: string): Promise<Result<void>> {
    return this.request<void>('DELETE', `/api/resumes/${id}`)
  }

  // ── Entry methods (replace perspective methods) ─────────────────

  addEntry(resumeId: string, input: AddResumeEntry): Promise<Result<ResumeEntry>> {
    return this.request<ResumeEntry>('POST', `/api/resumes/${resumeId}/entries`, input)
  }

  listEntries(resumeId: string): Promise<Result<ResumeEntry[]>> {
    return this.request<ResumeEntry[]>('GET', `/api/resumes/${resumeId}/entries`)
  }

  updateEntry(resumeId: string, entryId: string, input: UpdateResumeEntry): Promise<Result<ResumeEntry>> {
    return this.request<ResumeEntry>('PATCH', `/api/resumes/${resumeId}/entries/${entryId}`, input)
  }

  removeEntry(resumeId: string, entryId: string): Promise<Result<void>> {
    return this.request<void>('DELETE', `/api/resumes/${resumeId}/entries/${entryId}`)
  }

  gaps(id: string): Promise<Result<GapAnalysis>> {
    return this.request<GapAnalysis>('GET', `/api/resumes/${id}/gaps`)
  }

  export(id: string): Promise<Result<never>> {
    return this.request<never>('GET', `/api/resumes/${id}/export`)
  }
}
```

**New OrganizationsResource:**

```typescript
import type {
  CreateOrganization,
  Organization,
  OrganizationFilter,
  PaginatedResult,
  PaginationParams,
  RequestFn,
  RequestListFn,
  Result,
  UpdateOrganization,
} from '../types'

function toParams(filter?: Record<string, unknown>): Record<string, string> | undefined {
  if (!filter) return undefined
  const out: Record<string, string> = {}
  for (const [k, v] of Object.entries(filter)) {
    if (v !== undefined && v !== null) out[k] = String(v)
  }
  return Object.keys(out).length > 0 ? out : undefined
}

export class OrganizationsResource {
  constructor(
    private request: RequestFn,
    private requestList: RequestListFn,
  ) {}

  create(input: CreateOrganization): Promise<Result<Organization>> {
    return this.request<Organization>('POST', '/api/organizations', input)
  }

  list(filter?: OrganizationFilter & PaginationParams): Promise<PaginatedResult<Organization>> {
    return this.requestList<Organization>('GET', '/api/organizations', toParams(filter))
  }

  get(id: string): Promise<Result<Organization>> {
    return this.request<Organization>('GET', `/api/organizations/${id}`)
  }

  update(id: string, input: UpdateOrganization): Promise<Result<Organization>> {
    return this.request<Organization>('PATCH', `/api/organizations/${id}`, input)
  }

  delete(id: string): Promise<Result<void>> {
    return this.request<void>('DELETE', `/api/organizations/${id}`)
  }
}
```

**New NotesResource:**

```typescript
import type {
  CreateNote,
  NoteReference,
  PaginatedResult,
  PaginationParams,
  RequestFn,
  RequestListFn,
  Result,
  UpdateNote,
  UserNote,
} from '../types'

function toParams(filter?: Record<string, unknown>): Record<string, string> | undefined {
  if (!filter) return undefined
  const out: Record<string, string> = {}
  for (const [k, v] of Object.entries(filter)) {
    if (v !== undefined && v !== null) out[k] = String(v)
  }
  return Object.keys(out).length > 0 ? out : undefined
}

export class NotesResource {
  constructor(
    private request: RequestFn,
    private requestList: RequestListFn,
  ) {}

  create(input: CreateNote): Promise<Result<UserNote>> {
    return this.request<UserNote>('POST', '/api/notes', input)
  }

  list(filter?: { search?: string } & PaginationParams): Promise<PaginatedResult<UserNote>> {
    return this.requestList<UserNote>('GET', '/api/notes', toParams(filter))
  }

  get(id: string): Promise<Result<UserNote>> {
    return this.request<UserNote>('GET', `/api/notes/${id}`)
  }

  update(id: string, input: UpdateNote): Promise<Result<UserNote>> {
    return this.request<UserNote>('PATCH', `/api/notes/${id}`, input)
  }

  delete(id: string): Promise<Result<void>> {
    return this.request<void>('DELETE', `/api/notes/${id}`)
  }

  addReference(noteId: string, input: { entity_type: string; entity_id: string }): Promise<Result<void>> {
    return this.request<void>('POST', `/api/notes/${noteId}/references`, input)
  }

  removeReference(noteId: string, entityType: string, entityId: string): Promise<Result<void>> {
    return this.request<void>('DELETE', `/api/notes/${noteId}/references/${entityType}/${entityId}`)
  }
}
```

**New IntegrityResource:**

```typescript
import type { DriftReport, RequestFn, Result } from '../types'

export class IntegrityResource {
  constructor(private request: RequestFn) {}

  drift(): Promise<Result<DriftReport>> {
    return this.request<DriftReport>('GET', '/api/integrity/drift')
  }
}
```

**Acceptance Criteria:**
- [ ] `SourcesResource.list` passes `source_type` filter (not `employer_id`/`project_id`)
- [ ] `SourcesResource.create` sends `source_type` + extension in body
- [ ] `BulletsResource` methods work with new `Bullet` type (sources array, domain)
- [ ] `ResumesResource` has `addEntry`, `listEntries`, `updateEntry`, `removeEntry` (no perspective methods)
- [ ] `ResumesResource.get` returns `ResumeWithEntries`
- [ ] `OrganizationsResource` has `create`, `list`, `get`, `update`, `delete`
- [ ] `NotesResource` has CRUD + `addReference`, `removeReference`
- [ ] `IntegrityResource` has `drift()` method
- [ ] All methods produce correct HTTP requests (method, path, body)

---

## Task 12.9: Update SDK Client + Index

**Goal:** Wire new resources into ForgeClient and export from barrel.

**Files to modify:**
- `packages/sdk/src/client.ts` -- add `organizations`, `notes`, `integrity` properties
- `packages/sdk/src/index.ts` -- export new types and resources

**Changes:**

```typescript
import { BulletsResource } from './resources/bullets'
import { IntegrityResource } from './resources/integrity'
import { NotesResource } from './resources/notes'
import { OrganizationsResource } from './resources/organizations'
import { PerspectivesResource } from './resources/perspectives'
import { ResumesResource } from './resources/resumes'
import { ReviewResource } from './resources/review'
import { SourcesResource } from './resources/sources'

export class ForgeClient {
  private baseUrl: string

  public sources: SourcesResource
  public bullets: BulletsResource
  public perspectives: PerspectivesResource
  public resumes: ResumesResource
  public review: ReviewResource
  public organizations: OrganizationsResource   // NEW
  public notes: NotesResource                   // NEW
  public integrity: IntegrityResource           // NEW

  constructor(options: ForgeClientOptions) {
    this.baseUrl = options.baseUrl.replace(/\/+$/, '')

    const req = this.request.bind(this)
    const reqList = this.requestList.bind(this)

    this.sources = new SourcesResource(req, reqList)
    this.bullets = new BulletsResource(req, reqList)
    this.perspectives = new PerspectivesResource(req, reqList)
    this.resumes = new ResumesResource(req, reqList)
    this.review = new ReviewResource(req)
    this.organizations = new OrganizationsResource(req, reqList)  // NEW
    this.notes = new NotesResource(req, reqList)                  // NEW
    this.integrity = new IntegrityResource(req)                   // NEW
  }

  // ... request/requestList methods unchanged ...
}
```

**File:** `packages/sdk/src/index.ts`

**Changes — add new exports:**

```typescript
export { ForgeClient } from './client'
export type { ForgeClientOptions } from './client'

// Result & error types
export type { ForgeError, Pagination, PaginatedResult, PaginationParams, Result } from './types'

// Core entity types
export type { Source, Bullet, Perspective, Resume, Organization, ResumeEntry, UserNote } from './types'

// Extension types
export type { SourceRole, SourceProject, SourceEducation, SourceClearance } from './types'

// Rich response types
export type { SourceWithBullets, BulletWithRelations, PerspectiveWithChain, ResumeWithEntries } from './types'

// Review queue types
export type { BulletReviewItem, PerspectiveReviewItem, ReviewQueue, GapAnalysis } from './types'

// Drift types
export type { DriftReport, DriftedBullet, DriftedPerspective, DriftedResumeEntry } from './types'

// Note types
export type { NoteReference, CreateNote, UpdateNote } from './types'

// Input types
export type {
  CreateSource, UpdateSource,
  UpdateBullet, UpdatePerspective,
  RejectInput, DerivePerspectiveInput,
  CreateResume, UpdateResume,
  AddResumeEntry, UpdateResumeEntry,
  CreateOrganization, UpdateOrganization,
} from './types'

// Filter types
export type { SourceFilter, BulletFilter, PerspectiveFilter, OrganizationFilter } from './types'

// Resource classes
export { SourcesResource } from './resources/sources'
export { BulletsResource } from './resources/bullets'
export { PerspectivesResource } from './resources/perspectives'
export { ResumesResource } from './resources/resumes'
export { ReviewResource } from './resources/review'
export { OrganizationsResource } from './resources/organizations'
export { NotesResource } from './resources/notes'
export { IntegrityResource } from './resources/integrity'
```

**Removed exports:**
- `ResumeWithPerspectives` (replaced by `ResumeWithEntries`)
- `AddResumePerspective` (replaced by `AddResumeEntry`)
- `ReorderPerspectives`

**Acceptance Criteria:**
- [ ] `ForgeClient` has `organizations`, `notes`, `integrity` public properties
- [ ] `import { type Organization, type ResumeEntry, type UserNote } from '@forge/sdk'` works
- [ ] `import { type ResumeWithEntries } from '@forge/sdk'` works
- [ ] `ResumeWithPerspectives` no longer exported (compile error if referenced)
- [ ] `AddResumePerspective` no longer exported
- [ ] `tsc --noEmit` passes for the SDK package

---

## Task 12.10: Update Route Tests

**Goal:** Fix all existing route tests for new schema, add tests for new routes.

**Files:**
- `packages/core/src/routes/__tests__/sources.test.ts` — update for polymorphic sources
- `packages/core/src/routes/__tests__/bullets.test.ts` — update for sources array
- `packages/core/src/routes/__tests__/resumes.test.ts` — replace perspective endpoints with entry endpoints
- `packages/core/src/routes/__tests__/review.test.ts` — update for bullet_sources JOIN
- `packages/core/src/routes/__tests__/contracts.test.ts` — update response shape contracts
- `packages/core/src/routes/__tests__/organizations.test.ts` (NEW)
- `packages/core/src/routes/__tests__/notes.test.ts` (NEW)
- `packages/core/src/routes/__tests__/integrity.test.ts` (NEW)
- `packages/core/src/routes/__tests__/helpers.ts` — update `createTestApp` if server.ts wiring changed

**Key test updates:**
- All `seedBullet` calls updated for junction-only pattern (must specify `sourceId` + `isPrimary` for `bullet_sources` row)
- All `seedSource` calls support `source_type` parameter
- Response assertions updated: `source_id` → `sources[0].id`, etc.
- Resume tests use entry endpoints instead of perspective endpoints

**Acceptance Criteria:**
- [ ] All existing route tests pass with updated schema
- [ ] New organization route tests pass (CRUD + filter + cascade)
- [ ] New note route tests pass (CRUD + references + search)
- [ ] New integrity route tests pass (drift detection)
- [ ] New resume entry route tests pass (CRUD + copy-on-write)
- [ ] Test helpers updated for new seed patterns
- [ ] No leftover references to `employer_id`, `project_id`, `source_id` on bullets, or old perspective endpoints

---

## Task 12.11: Update SDK Tests

**Goal:** Fix existing SDK tests for type changes, add tests for new resources.

**Files:**
- `packages/sdk/src/__tests__/` — existing tests updated
- `packages/sdk/src/__tests__/organizations.test.ts` (NEW)
- `packages/sdk/src/__tests__/notes.test.ts` (NEW)
- `packages/sdk/src/__tests__/integrity.test.ts` (NEW)

**Test approach:** Unit tests mock `fetch` to verify correct HTTP request construction (method, path, body) and response parsing.

**Acceptance Criteria:**
- [ ] Source resource tests updated — `create` sends `source_type` + extension, `list` sends `source_type` filter
- [ ] Bullet resource tests verify response has `sources` array, not `source_id`
- [ ] Resume resource tests use entry methods (`addEntry`, `updateEntry`, `removeEntry`)
- [ ] Organization resource tests verify CRUD methods construct correct requests
- [ ] Notes resource tests verify CRUD + reference methods
- [ ] Integrity resource tests verify `drift()` makes GET to `/api/integrity/drift`
- [ ] `tsc --noEmit` passes for all SDK test files

---

## Task 12.6b: Update `server.ts` Route Registration

**Goal:** `server.ts` (route registration + imports) is a shared file modified by multiple route tasks. This task handles ALL new route registrations in a single coordinated step, to be executed AFTER tasks 12.1-12.6.

**File:** `packages/core/src/routes/server.ts`

**Changes -- add ALL new route registrations:**

```typescript
// Add these imports:
import { organizationRoutes } from './organizations'
import { noteRoutes } from './notes'
import { integrityRoutes } from './integrity'

// In createApp(), add these registrations alongside existing ones:
app.route('/', sourceRoutes(services))      // existing (updated in T12.1)
app.route('/', bulletRoutes(services))      // existing (updated in T12.2)
app.route('/', resumeRoutes(services))      // existing (updated in T12.3)
app.route('/', organizationRoutes(services)) // NEW
app.route('/', noteRoutes(services))         // NEW
app.route('/', integrityRoutes(services))    // NEW
```

**Additional cleanup:**
- Remove employer and project routes from `supportingRoutes` (organizations replace employers; projects are now source_projects)
- The skills routes remain but should gain `GET /skills/:id`, `PATCH /skills/:id`, `DELETE /skills/:id` if not already present

> **Parallelization note:** Route tasks (T12.1-T12.6) create independent route files and can run in parallel. However, this task (T12.6b) modifies the shared `server.ts` file and MUST execute after all route file tasks are complete. Alternatively, each route task can include its own `server.ts` import+registration, but they must be executed sequentially for the `server.ts` modifications.

**Acceptance Criteria:**
- [ ] All new route modules imported in `server.ts`
- [ ] All new route modules registered with `app.route()`
- [ ] Employer/project routes removed from `supportingRoutes`
- [ ] Server starts without errors: `bun run packages/core/src/routes/server.ts`
- [ ] All registered routes respond (verified by route tests in T12.10)

---

## Parallelization

```
Task 12.7 (SDK types) ──────────────────────────────────────────────────────┐
                                                                            │
Task 12.1 (source routes) ──┐                                              │
Task 12.2 (bullet routes) ──┤                                              ├──► Task 12.10 (route tests)
Task 12.3 (entry routes)  ──┤  All routes can run in parallel              │
Task 12.4 (org routes)    ──┤  (each independent)                          ├──► Task 12.11 (SDK tests)
Task 12.5 (note routes)   ──┤                                              │
Task 12.6 (integrity)     ──┘                                              │
                                                                            │
Task 12.8 (SDK resources) ──► depends on 12.7 (types)                      │
Task 12.9 (SDK client)    ──► depends on 12.8 (resources) ─────────────────┘
```

Route tasks (12.1-12.6) can all run in parallel. SDK type updates (12.7) can run in parallel with routes. SDK resources (12.8) and client (12.9) are sequential after 12.7. Tests (12.10, 12.11) run last after everything else.
