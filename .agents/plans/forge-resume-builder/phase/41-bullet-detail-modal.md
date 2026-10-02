# Phase 41: Bullet Detail Modal (Spec 12)

**Status:** Planning
**Date:** 2026-04-03
**Spec:** [2026-04-03-bullet-detail-modal.md](../refs/specs/2026-04-03-bullet-detail-modal.md)
**Depends on:** Phase 28 (stable baseline)
**Blocks:** None currently identified
**Parallelizable with:** Phases 29-40 at the code level (independent feature); no new migrations

## Goal

Add a detail modal for bullets on `/data/bullets`. Clicking a bullet card opens a modal overlay showing all bullet data -- content, status, domain, skills, technologies, sources, notes, and related perspectives. All fields are editable (except sources, which are read-only). The modal includes a "Derive Perspectives" button that opens a separate derivation dialog with archetype/domain/framing dropdowns loaded from the API. The implementation spans backend changes (extending `UpdateBulletInput`, adding bullet skills/sources endpoints, adding a submit endpoint), SDK method additions, two new UI components, and integration into `BulletsView.svelte`. No new migrations are required -- the existing `bullet_skills` and `bullet_technologies` tables provide the schema.

## Non-Goals

- Editing perspectives from within the bullet modal (clicking navigates away in the future)
- Inline derivation parameters (uses a separate dialog)
- Bulk bullet operations
- Reordering perspectives within the bullet
- Source editing from the bullet modal (sources are read-only context)
- Editing the `metrics` field from the bullet modal (metrics data is set during AI derivation and not typically user-edited)

## Context

The current `/data/bullets` page (`BulletsView.svelte`) shows bullet cards in a flat list with inline action buttons for approve/reject/reopen and a card-level "Derive Perspective" button with hardcoded archetype/domain constants. There is no way to view or edit all bullet fields in one place. The "Derive Perspective" button opens a small modal with hardcoded `ARCHETYPES` and `DOMAINS` arrays rather than loading them from the API.

The bullet data model stores `content`, `metrics`, `notes`, `domain`, and `status` on the `bullets` table; technologies in a `bullet_technologies` junction table; skills in a `bullet_skills` junction table; and source associations in a `bullet_sources` junction table. However, the current `UpdateBulletInput` only accepts `content` and `metrics` -- `notes`, `domain`, and `technologies` are not updatable via the API. There is also no API to read/write bullet skills or to read bullet sources. The `draft -> pending_review` status transition has no dedicated endpoint despite being defined in `VALID_TRANSITIONS`.

This phase addresses all of these gaps: extending the update input, adding bullet skills/sources endpoints, adding a submit endpoint, adding SDK methods, and building the modal UI.

## Scope

| Spec section | Covered here? |
|-------------|---------------|
| 1.1 New endpoints (skills, sources, submit) | Yes |
| 1.2 Backend changes (UpdateBulletInput, submitBullet) | Yes |
| 1.3 Existing endpoints used | Acknowledged |
| 2.1 BulletDetailModal | Yes |
| 2.2 DerivePerspectivesDialog | Yes |
| 2.3 Status transitions | Yes |
| 3. BulletsView integration | Yes |
| 4. Files to create | Yes |
| 5. Files to modify | Yes |
| 6. Testing | Yes |
| 7. Non-goals | Acknowledged |
| 8. Acceptance criteria | Yes |

## Files to Create

| File | Description |
|------|-------------|
| `packages/webui/src/lib/components/BulletDetailModal.svelte` | Bullet detail/edit modal with all fields, skill picker, tech input, sources list, perspectives list, status actions, save/delete |
| `packages/webui/src/lib/components/DerivePerspectivesDialog.svelte` | AI derivation parameter dialog with archetype/domain/framing dropdowns loaded from API |

Note: Do NOT add `BulletDetailModal` or `DerivePerspectivesDialog` to `$lib/components/index.ts`. They are imported directly by path from BulletsView and internally.

## Files to Modify

| File | Change |
|------|--------|
| `packages/core/src/db/repositories/bullet-repository.ts` | Extend `UpdateBulletInput` with `notes`, `domain`, `technologies` fields; update `update()` to handle new fields with SET clauses for notes/domain and DELETE-all + INSERT-all for technologies |
| `packages/core/src/services/bullet-service.ts` | Add `submitBullet(id)` method that calls `this.transition(id, 'pending_review')` |
| `packages/core/src/routes/bullets.ts` | Add bullet skills endpoints (GET/POST/DELETE), bullet sources endpoint (GET), `PATCH /api/bullets/:id/submit` route; update function signature to accept `db` parameter |
| `packages/core/src/routes/server.ts` | Pass `db` to `bulletRoutes(services, db)` |
| `packages/sdk/src/resources/bullets.ts` | Add `listSkills`, `addSkill`, `removeSkill`, `listSources`, `submit` methods to `BulletsResource` |
| `packages/webui/src/routes/data/sources/BulletsView.svelte` | Add `detailBulletId` state, click handler on cards, mount `BulletDetailModal`; remove inline derive modal (hardcoded `ARCHETYPES`/`DOMAINS`, `deriveModal` state, `submitDerive` function, derive modal template) |

## Fallback Strategies

- **Skill picker performance:** If loading all skills via `GET /api/skills` is slow for large skill sets, the picker can be lazy-loaded (fetch on first focus of the search input). For now, load all skills on modal mount alongside other data.
- **Technology case sensitivity:** Technologies are lowercased before storage (`tech.toLowerCase().trim()`). If the user expects case-preserved display, the UI can capitalize-first on display -- but per spec, technologies display as-is from the API response (lowercased).
- **Sources endpoint missing data:** The `BulletRepository.getSources()` method already returns `id`, `title`, and `is_primary`. The spec wants full `Source` fields. The new `GET /api/bullets/:id/sources` endpoint will JOIN `sources` to get all source columns plus `is_primary`. If the sources table schema changes in a future migration, the `SELECT s.*` pattern will automatically pick up new columns.
- **Delete with perspectives:** The existing `DELETE /api/bullets/:id` already returns 409 CONFLICT when perspectives exist. The modal will catch this and show an error toast. No backend changes needed.
- **Derive endpoint failure:** The AI derivation can fail due to network/API issues. The DerivePerspectivesDialog stays open on error and shows an error toast, allowing the user to retry.

---

## Tasks

### T41.1: Extend `UpdateBulletInput` and `update()` in BulletRepository

**File:** `packages/core/src/db/repositories/bullet-repository.ts`

Extend `UpdateBulletInput` to include `notes`, `domain`, and `technologies`. Update the `update()` method to handle these new fields: `notes` and `domain` as additional SET clauses (same pattern as `content` and `metrics`), and `technologies` as a DELETE-all + INSERT-all within the same method call.

**Current `UpdateBulletInput` (line 43-46):**
```typescript
export interface UpdateBulletInput {
  content?: string
  metrics?: string | null
}
```

**Replace with:**
```typescript
export interface UpdateBulletInput {
  content?: string
  metrics?: string | null
  notes?: string | null
  domain?: string | null
  technologies?: string[]
}
```

**Current `update()` method (line 258-285):**
```typescript
  update(db: Database, id: string, input: UpdateBulletInput): Bullet | null {
    const sets: string[] = []
    const params: unknown[] = []

    if (input.content !== undefined) {
      sets.push('content = ?')
      params.push(input.content)
    }
    if (input.metrics !== undefined) {
      sets.push('metrics = ?')
      params.push(input.metrics)
    }

    if (sets.length === 0) {
      return BulletRepository.get(db, id)
    }

    params.push(id)

    const row = db
      .query(`UPDATE bullets SET ${sets.join(', ')} WHERE id = ? RETURNING *`)
      .get(...params) as BulletRow | null

    if (!row) return null

    const technologies = getTechnologies(db, id)
    return rowToBullet(row, technologies)
  },
```

**Replace with:**
```typescript
  update(db: Database, id: string, input: UpdateBulletInput): Bullet | null {
    const sets: string[] = []
    const params: unknown[] = []

    if (input.content !== undefined) {
      sets.push('content = ?')
      params.push(input.content)
    }
    if (input.metrics !== undefined) {
      sets.push('metrics = ?')
      params.push(input.metrics)
    }
    if (input.notes !== undefined) {
      sets.push('notes = ?')
      params.push(input.notes)
    }
    if (input.domain !== undefined) {
      sets.push('domain = ?')
      params.push(input.domain)
    }

    // Handle technologies: DELETE all existing, INSERT new list
    const hasTechnologies = input.technologies !== undefined

    if (sets.length === 0 && !hasTechnologies) {
      return BulletRepository.get(db, id)
    }

    let row: BulletRow | null

    if (sets.length > 0) {
      params.push(id)
      row = db
        .query(`UPDATE bullets SET ${sets.join(', ')} WHERE id = ? RETURNING *`)
        .get(...params) as BulletRow | null
    } else {
      // Only technologies changed — verify bullet exists
      row = db
        .query('SELECT * FROM bullets WHERE id = ?')
        .get(id) as BulletRow | null
    }

    if (!row) return null

    if (hasTechnologies) {
      db.run('DELETE FROM bullet_technologies WHERE bullet_id = ?', [id])
      insertTechnologies(db, id, input.technologies!)
    }

    const technologies = getTechnologies(db, id)
    return rowToBullet(row, technologies)
  },
```

**Acceptance criteria:**
- `BulletRepository.update(db, id, { notes: 'my note' })` updates the `notes` column and returns the bullet with `notes === 'my note'`.
- `BulletRepository.update(db, id, { domain: 'security' })` updates the `domain` column.
- `BulletRepository.update(db, id, { domain: null })` clears the domain field.
- `BulletRepository.update(db, id, { technologies: ['python', 'rust'] })` replaces all technologies with `['python', 'rust']`.
- `BulletRepository.update(db, id, { technologies: [] })` clears all technologies.
- `BulletRepository.update(db, id, { technologies: ['Python'] })` stores `['python']` (lowercased by `insertTechnologies`).
- `BulletRepository.update(db, id, { content: 'new', notes: 'note', technologies: ['go'] })` updates all three in one call.
- `BulletRepository.update(db, id, {})` returns the bullet unchanged (no-op).

**Failure criteria:**
- Technologies are not cleared before reinserting (duplicates appear).
- `notes` or `domain` updates fail silently or are ignored.
- Calling `update()` with only `technologies` returns null for an existing bullet.

---

### T41.2: Add `submitBullet()` to BulletService

**File:** `packages/core/src/services/bullet-service.ts`

Add a `submitBullet(id)` method that transitions a bullet from `draft` to `pending_review`. This mirrors the pattern of `approveBullet`, `rejectBullet`, and `reopenBullet` which all delegate to the private `transition()` method.

**Add after `reopenBullet` method (after line 86):**
```typescript
  submitBullet(id: string): Result<Bullet> {
    // Guard: only draft bullets can be submitted for review.
    // Note: transition() checks VALID_TRANSITIONS, but VALID_TRANSITIONS
    // allows rejected -> pending_review. submitBullet is semantically
    // for draft -> pending_review only. Use reopenBullet for rejected bullets.
    const bullet = BulletRepository.get(this.db, id)
    if (!bullet) return { ok: false, error: { code: 'NOT_FOUND', message: 'Bullet not found' } }
    if (bullet.status !== 'draft') {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: 'Only draft bullets can be submitted for review' } }
    }
    return this.transition(id, 'pending_review')
  }
```

**Acceptance criteria:**
- `service.submitBullet(id)` on a `draft` bullet returns `{ ok: true, data: { status: 'pending_review' } }`.
- `service.submitBullet(id)` on a `pending_review` bullet returns `{ ok: false, error: { code: 'VALIDATION_ERROR', message: "Cannot transition from 'pending_review' to 'pending_review'" } }`.
- `service.submitBullet(id)` on an `approved` bullet returns a VALIDATION_ERROR.
- `service.submitBullet(id)` on a `rejected` bullet returns VALIDATION_ERROR -- only `draft` bullets can be submitted for review. `reopenBullet()` is the correct action for rejected bullets.
- `service.submitBullet(id)` on a non-existent bullet returns NOT_FOUND.

**Failure criteria:**
- `submitBullet` allows transitions from non-draft statuses (e.g., rejected bullets pass through because `VALID_TRANSITIONS` permits `rejected -> pending_review`).
- `submitBullet` does not reuse the existing `transition()` method for the actual transition after the guard check.

---

### T41.3: Add Bullet Skills, Sources, and Submit Routes

**Prerequisite:** Verify `bullet_skills` table exists: `SELECT name FROM sqlite_master WHERE type='table' AND name='bullet_skills'`. This table was created in migration 001. If missing, check that all migrations have been applied.

**File:** `packages/core/src/routes/bullets.ts`

Update `bulletRoutes` to accept `db` as a second parameter (matching `sourceRoutes` pattern) and add four new endpoints: GET/POST/DELETE for bullet skills, GET for bullet sources, and PATCH for submit.

**Replace the entire file with:**
```typescript
/**
 * Bullet routes — thin HTTP layer over BulletService and DerivationService.
 */

import { Hono } from 'hono'
import type { Database } from 'bun:sqlite'
import type { Services } from '../services'
import { mapStatusCode } from './server'

export function bulletRoutes(services: Services, db: Database) {
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

  app.get('/bullets/:id', (c) => {
    const result = services.bullets.getBullet(c.req.param('id'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.patch('/bullets/:id', async (c) => {
    const body = await c.req.json()
    const result = services.bullets.updateBullet(c.req.param('id'), body)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.delete('/bullets/:id', (c) => {
    const result = services.bullets.deleteBullet(c.req.param('id'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.body(null, 204)
  })

  app.patch('/bullets/:id/approve', (c) => {
    const result = services.bullets.approveBullet(c.req.param('id'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.patch('/bullets/:id/reject', async (c) => {
    const body = await c.req.json<{ rejection_reason?: string }>()
    const result = services.bullets.rejectBullet(c.req.param('id'), body.rejection_reason ?? '')
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.patch('/bullets/:id/reopen', (c) => {
    const result = services.bullets.reopenBullet(c.req.param('id'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.patch('/bullets/:id/submit', (c) => {
    const result = services.bullets.submitBullet(c.req.param('id'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.post('/bullets/:id/derive-perspectives', async (c) => {
    const body = await c.req.json<{ archetype: string; domain: string; framing: string }>()
    const result = await services.derivation.derivePerspectivesFromBullet(c.req.param('id'), {
      archetype: body.archetype,
      domain: body.domain,
      framing: body.framing as any,
    })
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data }, 201)
  })

  // ── Bullet Skills ───────────────────────────────────────────────────

  app.get('/bullets/:id/skills', (c) => {
    const rows = db.query(
      `SELECT s.* FROM skills s
       JOIN bullet_skills bs ON bs.skill_id = s.id
       WHERE bs.bullet_id = ?
       ORDER BY s.name ASC`
    ).all(c.req.param('id'))
    return c.json({ data: rows })
  })

  app.post('/bullets/:id/skills', async (c) => {
    const body = await c.req.json()
    const bulletId = c.req.param('id')

    // If skill_id is provided, link existing skill
    if (body.skill_id) {
      try {
        db.run('INSERT OR IGNORE INTO bullet_skills (bullet_id, skill_id) VALUES (?, ?)',
          [bulletId, body.skill_id])
      } catch (err: any) {
        if (err.message?.includes('FOREIGN KEY')) {
          return c.json({ error: { code: 'NOT_FOUND', message: 'Bullet or skill not found' } }, 404)
        }
        throw err
      }
      const skill = db.query('SELECT * FROM skills WHERE id = ?').get(body.skill_id)
      return c.json({ data: skill }, 201)
    }

    // If name is provided, create new skill and link it
    if (body.name?.trim()) {
      // capitalizeFirst: uppercase first character, preserve rest (SAFe stays SAFe, foo->Foo)
      const raw = body.name.trim()
      const name = raw.charAt(0).toUpperCase() + raw.slice(1)
      // Case-insensitive dedup: check if skill with this name already exists
      let skill = db.query('SELECT * FROM skills WHERE name = ? COLLATE NOCASE').get(name) as any
      if (!skill) {
        const id = crypto.randomUUID()
        skill = db.query(
          `INSERT INTO skills (id, name, category) VALUES (?, ?, ?) RETURNING *`
        ).get(id, name, body.category ?? 'general')
      }
      db.run('INSERT OR IGNORE INTO bullet_skills (bullet_id, skill_id) VALUES (?, ?)',
        [bulletId, skill.id])
      return c.json({ data: skill }, 201)
    }

    return c.json({ error: { code: 'VALIDATION_ERROR', message: 'skill_id or name is required' } }, 400)
  })

  app.delete('/bullets/:bulletId/skills/:skillId', (c) => {
    const result = db.run(
      'DELETE FROM bullet_skills WHERE bullet_id = ? AND skill_id = ?',
      [c.req.param('bulletId'), c.req.param('skillId')]
    )
    if (result.changes === 0) {
      return c.json({ error: { code: 'NOT_FOUND', message: 'Skill link not found' } }, 404)
    }
    return c.body(null, 204)
  })

  // ── Bullet Sources ──────────────────────────────────────────────────

  // NOTE: `is_primary` is returned as `0` or `1` (SQLite INTEGER).
  // The UI uses truthiness check (`{#if src.is_primary}`) which works for both number and boolean.
  app.get('/bullets/:id/sources', (c) => {
    const rows = db.query(
      `SELECT s.*, bs.is_primary
       FROM bullet_sources bs
       JOIN sources s ON bs.source_id = s.id
       WHERE bs.bullet_id = ?
       ORDER BY bs.is_primary DESC, s.title ASC`
    ).all(c.req.param('id'))
    return c.json({ data: rows })
  })

  return app
}
```

- [ ] **Also update `server.ts`:** Change `bulletRoutes(services)` to `bulletRoutes(services, db)` at the mounting line. This is a one-line change but critical -- without it, the `db` parameter is undefined in the new skill/source endpoints.

**File:** `packages/core/src/routes/server.ts`

Update the `bulletRoutes` call to pass `db`:

**Current (line 103):**
```typescript
  app.route('/', bulletRoutes(services))
```

**Replace with:**
```typescript
  app.route('/', bulletRoutes(services, db))
```

**Acceptance criteria:**
- `GET /api/bullets/:id/skills` returns `{ data: Skill[] }` for a bullet with linked skills.
- `POST /api/bullets/:id/skills` with `{ skill_id }` links an existing skill and returns `{ data: Skill }` with status 201.
- `POST /api/bullets/:id/skills` with `{ name: 'python' }` when `Python` already exists links the existing one (case-insensitive dedup).
- `POST /api/bullets/:id/skills` with `{ name: 'newskill' }` creates `Newskill` (capitalizeFirst) and links it.
- `POST /api/bullets/:id/skills` with duplicate skill is idempotent (INSERT OR IGNORE).
- `DELETE /api/bullets/:bulletId/skills/:skillId` returns 204 on success, 404 if link does not exist.
- `GET /api/bullets/:id/sources` returns `{ data: Array<Source & { is_primary: number }> }` with source fields plus `is_primary`.
- `PATCH /api/bullets/:id/submit` transitions a draft bullet to `pending_review` and returns 200.
- `PATCH /api/bullets/:id/submit` on a non-draft bullet returns 400 with VALIDATION_ERROR.

**Failure criteria:**
- Bullet skills endpoints are not accessible (404 from router).
- `capitalizeFirst` logic differs from source skills pattern.
- Submit endpoint does not call `services.bullets.submitBullet`.

---

### T41.4: Add SDK Methods to `BulletsResource`

**File:** `packages/sdk/src/resources/bullets.ts`

Add five new methods to `BulletsResource`: `listSkills`, `addSkill`, `removeSkill`, `listSources`, and `submit`.

**Replace the entire file with:**
```typescript
import type {
  Bullet,
  BulletFilter,
  BulletWithRelations,
  DerivePerspectiveInput,
  PaginatedResult,
  PaginationParams,
  Perspective,
  RejectInput,
  RequestFn,
  RequestListFn,
  Result,
  Skill,
  Source,
  UpdateBullet,
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

export class BulletsResource {
  constructor(
    private request: RequestFn,
    private requestList: RequestListFn,
  ) {}

  list(
    filter?: BulletFilter & PaginationParams,
  ): Promise<PaginatedResult<Bullet>> {
    return this.requestList<Bullet>('GET', '/api/bullets', toParams(filter))
  }

  get(id: string): Promise<Result<BulletWithRelations>> {
    return this.request<BulletWithRelations>('GET', `/api/bullets/${id}`)
  }

  update(id: string, input: UpdateBullet): Promise<Result<Bullet>> {
    return this.request<Bullet>('PATCH', `/api/bullets/${id}`, input)
  }

  delete(id: string): Promise<Result<void>> {
    return this.request<void>('DELETE', `/api/bullets/${id}`)
  }

  approve(id: string): Promise<Result<Bullet>> {
    return this.request<Bullet>('PATCH', `/api/bullets/${id}/approve`)
  }

  reject(id: string, input: RejectInput): Promise<Result<Bullet>> {
    return this.request<Bullet>('PATCH', `/api/bullets/${id}/reject`, input)
  }

  reopen(id: string): Promise<Result<Bullet>> {
    return this.request<Bullet>('PATCH', `/api/bullets/${id}/reopen`)
  }

  submit(id: string): Promise<Result<Bullet>> {
    return this.request<Bullet>('PATCH', `/api/bullets/${id}/submit`)
  }

  derivePerspectives(
    id: string,
    input: DerivePerspectiveInput,
  ): Promise<Result<Perspective>> {
    return this.request<Perspective>(
      'POST',
      `/api/bullets/${id}/derive-perspectives`,
      input,
    )
  }

  // ── Bullet Skills ───────────────────────────────────────────────────

  listSkills(bulletId: string): Promise<Result<Skill[]>> {
    return this.request<Skill[]>('GET', `/api/bullets/${bulletId}/skills`)
  }

  addSkill(bulletId: string, input: { skill_id: string } | { name: string; category?: string }): Promise<Result<Skill>> {
    return this.request<Skill>('POST', `/api/bullets/${bulletId}/skills`, input)
  }

  removeSkill(bulletId: string, skillId: string): Promise<Result<void>> {
    return this.request<void>('DELETE', `/api/bullets/${bulletId}/skills/${skillId}`)
  }

  // ── Bullet Sources ──────────────────────────────────────────────────

  listSources(bulletId: string): Promise<Result<Array<Source & { is_primary: number }>>> {
    return this.request<Array<Source & { is_primary: number }>>('GET', `/api/bullets/${bulletId}/sources`)
  }
}
```

Note: `UpdateBullet` in `packages/sdk/src/types.ts` already includes `notes`, `domain`, and `technologies` fields -- no type changes needed in the SDK types file. Only the `BulletsResource` methods need updating.

**Acceptance criteria:**
- `forge.bullets.submit(id)` sends `PATCH /api/bullets/:id/submit` and returns `Result<Bullet>`.
- `forge.bullets.listSkills(bulletId)` sends `GET /api/bullets/:id/skills` and returns `Result<Skill[]>`.
- `forge.bullets.addSkill(bulletId, { name: 'Python' })` sends `POST /api/bullets/:id/skills` with the body.
- `forge.bullets.addSkill(bulletId, { skill_id: 'uuid' })` sends `POST /api/bullets/:id/skills` with the body.
- `forge.bullets.removeSkill(bulletId, skillId)` sends `DELETE /api/bullets/:bulletId/skills/:skillId`.
- `forge.bullets.listSources(bulletId)` sends `GET /api/bullets/:id/sources` and returns sources with `is_primary`.
- All existing methods (`list`, `get`, `update`, `delete`, `approve`, `reject`, `reopen`, `derivePerspectives`) remain unchanged.

**Failure criteria:**
- New methods use incorrect HTTP verbs or paths.
- Existing methods are accidentally modified.
- `Source` or `Skill` types are not imported.

---

### T41.5: Create `BulletDetailModal.svelte`

**File:** `packages/webui/src/lib/components/BulletDetailModal.svelte`

This is the main modal component. It loads all bullet data on mount, provides editable fields for content/notes/domain/technologies, a skill picker with search + inline create, read-only sources list, perspectives list, status transition buttons, derive button, and delete with confirmation.

```svelte
<script lang="ts">
  import { forge, friendlyError } from '$lib/sdk'
  import { addToast } from '$lib/stores/toast.svelte'
  import { StatusBadge, LoadingSpinner, ConfirmDialog } from '$lib/components'
  import DerivePerspectivesDialog from './DerivePerspectivesDialog.svelte'
  import type { Bullet, Skill, Source, Perspective } from '@forge/sdk'

  let { bulletId, onclose, onupdate }: {
    bulletId: string
    onclose: () => void
    onupdate: () => void
  } = $props()

  // ── State ────────────────────────────────────────────────────────────

  // Note: `forge.bullets.get()` returns `BulletWithRelations` in the SDK type but the
  // actual API response is `Bullet` (no `perspective_count`). The modal uses `Bullet`
  // state type, so this is handled correctly. `perspective_count` is derived from the
  // perspectives list length.
  let bullet = $state<Bullet | null>(null)
  let bulletSkills = $state<Skill[]>([])
  let bulletSources = $state<Array<Source & { is_primary: number }>>([])
  let perspectives = $state<Perspective[]>([])
  let loading = $state(true)
  let saving = $state(false)

  // Editable fields
  let editContent = $state('')
  let editNotes = $state('')
  let editDomain = $state<string | null>(null)
  let editTechnologies = $state<string[]>([])

  // Skill picker
  let allSkills = $state<Skill[]>([])
  let skillSearch = $state('')
  let showSkillDropdown = $state(false)

  // Technology input
  let newTechInput = $state('')

  // Derivation dialog
  let showDeriveDialog = $state(false)

  // Delete confirmation
  let showDeleteConfirm = $state(false)

  // Reject inline input
  let showRejectInput = $state(false)
  let rejectReason = $state('')

  // Domain dropdown
  let allDomains = $state<Array<{ id: string; name: string }>>([])

  // ── Derived ──────────────────────────────────────────────────────────

  let filteredSkills = $derived.by(() => {
    if (!skillSearch.trim()) return []
    const q = skillSearch.toLowerCase()
    const linkedIds = new Set(bulletSkills.map(s => s.id))
    return allSkills
      .filter(s => !linkedIds.has(s.id) && s.name.toLowerCase().includes(q))
      .slice(0, 10)
  })

  let canCreateSkill = $derived.by(() => {
    if (!skillSearch.trim()) return false
    const q = skillSearch.trim().toLowerCase()
    // Check if any existing skill matches case-insensitively
    return !allSkills.some(s => s.name.toLowerCase() === q)
  })

  // ── Data Loading ─────────────────────────────────────────────────────

  $effect(() => {
    loadAllData()
  })

  async function loadAllData() {
    loading = true

    const [bulletRes, skillsRes, sourcesRes, perspectivesRes, allSkillsRes, domainsRes] = await Promise.all([
      forge.bullets.get(bulletId),
      forge.bullets.listSkills(bulletId),
      forge.bullets.listSources(bulletId),
      forge.perspectives.list({ bullet_id: bulletId, limit: 200 }), // If `limit` is not part of `PerspectiveFilter`, pass it as a second pagination argument: `forge.perspectives.list({ bullet_id: bulletId }, { limit: 200 })`
      forge.skills.list({ limit: 500 }), // Pass limit: 500 to avoid silent truncation on large skill sets
      forge.domains.list({ limit: 200 }),
    ])

    if (bulletRes.ok) {
      bullet = bulletRes.data
      editContent = bulletRes.data.content
      editNotes = bulletRes.data.notes ?? ''
      editDomain = bulletRes.data.domain
      editTechnologies = [...(bulletRes.data.technologies ?? [])]
    } else {
      addToast({ message: friendlyError(bulletRes.error, 'Failed to load bullet'), type: 'error' })
      onclose()
      return
    }

    if (skillsRes.ok) bulletSkills = skillsRes.data
    if (sourcesRes.ok) bulletSources = sourcesRes.data
    if (perspectivesRes.ok) perspectives = perspectivesRes.data
    if (allSkillsRes.ok) allSkills = allSkillsRes.data
    if (domainsRes.ok) allDomains = domainsRes.data

    loading = false
  }

  // ── Save ──────────────────────────────────────────────────────────────

  async function save() {
    if (!bullet) return
    saving = true

    const res = await forge.bullets.update(bulletId, {
      content: editContent,
      notes: editNotes || null, // Normalize empty string to null for notes (avoid storing empty strings in DB)
      domain: editDomain,
      technologies: editTechnologies,
    })

    if (res.ok) {
      bullet = res.data
      addToast({ message: 'Bullet saved', type: 'success' })
      onupdate()
    } else {
      addToast({ message: friendlyError(res.error, 'Failed to save'), type: 'error' })
    }

    saving = false
  }

  // ── Status Transitions ───────────────────────────────────────────────

  async function submitForReview() {
    const res = await forge.bullets.submit(bulletId)
    if (res.ok) {
      bullet = res.data
      addToast({ message: 'Submitted for review', type: 'success' })
      onupdate()
    } else {
      addToast({ message: friendlyError(res.error, 'Submit failed'), type: 'error' })
    }
  }

  async function approve() {
    const res = await forge.bullets.approve(bulletId)
    if (res.ok) {
      bullet = res.data
      addToast({ message: 'Bullet approved', type: 'success' })
      onupdate()
    } else {
      addToast({ message: friendlyError(res.error, 'Approve failed'), type: 'error' })
    }
  }

  async function submitReject() {
    if (!rejectReason.trim()) {
      addToast({ message: 'Please provide a rejection reason.', type: 'error' })
      return
    }
    const res = await forge.bullets.reject(bulletId, { rejection_reason: rejectReason })
    if (res.ok) {
      bullet = res.data
      showRejectInput = false
      rejectReason = ''
      addToast({ message: 'Bullet rejected', type: 'success' })
      onupdate()
    } else {
      addToast({ message: friendlyError(res.error, 'Reject failed'), type: 'error' })
    }
  }

  async function reopen() {
    const res = await forge.bullets.reopen(bulletId)
    if (res.ok) {
      bullet = res.data
      addToast({ message: 'Bullet reopened', type: 'success' })
      onupdate()
    } else {
      addToast({ message: friendlyError(res.error, 'Reopen failed'), type: 'error' })
    }
  }

  // ── Skills ────────────────────────────────────────────────────────────

  async function addExistingSkill(skill: Skill) {
    const res = await forge.bullets.addSkill(bulletId, { skill_id: skill.id })
    if (res.ok) {
      bulletSkills = [...bulletSkills, res.data]
      skillSearch = ''
      showSkillDropdown = false
    } else {
      addToast({ message: friendlyError(res.error, 'Failed to add skill'), type: 'error' })
    }
  }

  async function createAndAddSkill() {
    if (!skillSearch.trim()) return
    const res = await forge.bullets.addSkill(bulletId, { name: skillSearch.trim() })
    if (res.ok) {
      bulletSkills = [...bulletSkills, res.data]
      // Also add to allSkills so it appears in future searches
      if (!allSkills.some(s => s.id === res.data.id)) {
        allSkills = [...allSkills, res.data]
      }
      skillSearch = ''
      showSkillDropdown = false
    } else {
      addToast({ message: friendlyError(res.error, 'Failed to create skill'), type: 'error' })
    }
  }

  async function removeSkill(skillId: string) {
    const res = await forge.bullets.removeSkill(bulletId, skillId)
    if (res.ok) {
      bulletSkills = bulletSkills.filter(s => s.id !== skillId)
    } else {
      addToast({ message: friendlyError(res.error, 'Failed to remove skill'), type: 'error' })
    }
  }

  // ── Technologies ──────────────────────────────────────────────────────

  function addTechnology() {
    const tech = newTechInput.toLowerCase().trim()
    if (tech && !editTechnologies.includes(tech)) {
      editTechnologies = [...editTechnologies, tech]
    }
    newTechInput = ''
  }

  function removeTechnology(tech: string) {
    editTechnologies = editTechnologies.filter(t => t !== tech)
  }

  function handleTechKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault()
      addTechnology()
    }
  }

  // ── Delete ────────────────────────────────────────────────────────────

  async function deleteBullet() {
    const res = await forge.bullets.delete(bulletId)
    if (res.ok) {
      addToast({ message: 'Bullet deleted', type: 'success' })
      onupdate()
      onclose()
    } else {
      showDeleteConfirm = false
      if (res.error.code === 'CONFLICT') {
        addToast({ message: 'Cannot delete bullet with existing perspectives. Delete its perspectives first.', type: 'error' })
      } else {
        addToast({ message: friendlyError(res.error, 'Delete failed'), type: 'error' })
      }
    }
  }

  // ── Derive callback ───────────────────────────────────────────────────

  async function onDeriveComplete() {
    // Refresh perspectives list
    const res = await forge.perspectives.list({ bullet_id: bulletId, limit: 200 })
    if (res.ok) perspectives = res.data
    onupdate()
  }

  // ── Keyboard ──────────────────────────────────────────────────────────

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      // If a sub-dialog is open, let it handle Escape
      if (showDeriveDialog || showDeleteConfirm) return
      onclose()
    }
  }

  function truncateTitle(text: string, max: number = 60): string {
    if (text.length <= max) return text
    return text.slice(0, max) + '...'
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div class="modal-overlay" onclick={onclose} role="presentation">
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="modal-content" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true" aria-label="Bullet Details">
    {#if loading}
      <div class="loading-container">
        <LoadingSpinner size="lg" message="Loading bullet..." />
      </div>
    {:else if bullet}
      <!-- Header -->
      <div class="modal-header">
        <div class="header-left">
          <h3>{truncateTitle(bullet.content)}</h3>
          <StatusBadge status={bullet.status} />
        </div>
        <button class="close-btn" onclick={onclose}>&times;</button>
      </div>

      <div class="modal-body">
        <!-- Status Actions -->
        <div class="status-actions">
          {#if bullet.status === 'draft'}
            <button class="btn btn-submit" onclick={submitForReview}>Submit for Review</button>
          {/if}
          {#if bullet.status === 'pending_review'}
            <button class="btn btn-approve" onclick={approve}>Approve</button>
            {#if showRejectInput}
              <div class="reject-inline">
                <input
                  type="text"
                  class="reject-input"
                  placeholder="Rejection reason..."
                  bind:value={rejectReason}
                  onkeydown={(e) => { if (e.key === 'Enter') submitReject() }}
                />
                <button class="btn btn-danger-sm" onclick={submitReject}>Confirm</button>
                <button class="btn btn-ghost-sm" onclick={() => { showRejectInput = false; rejectReason = '' }}>Cancel</button>
              </div>
            {:else}
              <button class="btn btn-reject" onclick={() => showRejectInput = true}>Reject</button>
            {/if}
          {/if}
          {#if bullet.status === 'rejected'}
            <button class="btn btn-reopen" onclick={reopen}>Reopen</button>
            {#if bullet.rejection_reason}
              <span class="rejection-reason">Rejected: {bullet.rejection_reason}</span>
            {/if}
          {/if}
        </div>

        <!-- Content -->
        <div class="field-group">
          <label class="field-label" for="bullet-content">Content</label>
          <textarea
            id="bullet-content"
            class="field-textarea content-textarea"
            bind:value={editContent}
            rows="4"
          ></textarea>
        </div>

        <!-- Domain -->
        <div class="field-group">
          <label class="field-label" for="bullet-domain">Domain</label>
          <select id="bullet-domain" class="field-select" bind:value={editDomain}>
            <option value={null}>-- No domain --</option>
            {#each allDomains as domain}
              <option value={domain.name}>{domain.name}</option>
            {/each}
          </select>
        </div>

        <!-- Skills -->
        <div class="field-group">
          <label class="field-label">Skills</label>
          <div class="tag-pills">
            {#each bulletSkills as skill (skill.id)}
              <span class="tag-pill skill-pill">
                {skill.name}
                <button class="pill-remove" onclick={() => removeSkill(skill.id)}>&times;</button>
              </span>
            {/each}
          </div>
          <div class="skill-picker">
            <input
              type="text"
              class="skill-search"
              placeholder="Search or create skill..."
              bind:value={skillSearch}
              onfocus={() => showSkillDropdown = true}
              onblur={() => setTimeout(() => showSkillDropdown = false, 200)}
              onkeydown={(e) => {
                if (e.key === 'Enter' && canCreateSkill) {
                  e.preventDefault()
                  createAndAddSkill()
                }
              }}
            />
            {#if showSkillDropdown && (filteredSkills.length > 0 || canCreateSkill)}
              <div class="skill-dropdown">
                {#each filteredSkills as skill (skill.id)}
                  <button class="dropdown-item" onmousedown={() => addExistingSkill(skill)}>
                    {skill.name}
                    {#if skill.category}
                      <span class="dropdown-category">{skill.category}</span>
                    {/if}
                  </button>
                {/each}
                {#if canCreateSkill}
                  <button class="dropdown-item create-item" onmousedown={createAndAddSkill}>
                    Create "{skillSearch.trim()}"
                  </button>
                {/if}
              </div>
            {/if}
          </div>
        </div>

        <!-- Technologies -->
        <div class="field-group">
          <label class="field-label">Technologies</label>
          <div class="tag-pills">
            {#each editTechnologies as tech}
              <span class="tag-pill tech-pill">
                {tech}
                <button class="pill-remove" onclick={() => removeTechnology(tech)}>&times;</button>
              </span>
            {/each}
          </div>
          <input
            type="text"
            class="tech-input"
            placeholder="Type technology and press Enter..."
            bind:value={newTechInput}
            onkeydown={handleTechKeydown}
          />
        </div>

        <!-- Sources (read-only) -->
        <div class="field-group">
          <label class="field-label">Sources</label>
          {#if bulletSources.length === 0}
            <p class="empty-text">No sources linked</p>
          {:else}
            <div class="source-list">
              {#each bulletSources as src}
                <div class="source-item">
                  {#if src.is_primary}
                    <span class="primary-star" title="Primary source">&#9733;</span>
                  {/if}
                  <span class="source-title">{src.title}</span>
                </div>
              {/each}
            </div>
          {/if}
        </div>

        <!-- Notes -->
        <div class="field-group">
          <label class="field-label" for="bullet-notes">Notes</label>
          <textarea
            id="bullet-notes"
            class="field-textarea"
            bind:value={editNotes}
            rows="3"
            placeholder="Add notes..."
          ></textarea>
        </div>

        <!-- Perspectives -->
        <div class="field-group">
          <label class="field-label">Perspectives ({perspectives.length})</label>
          {#if perspectives.length === 0}
            <p class="empty-text">No perspectives derived yet</p>
          {:else}
            <div class="perspective-list">
              {#each perspectives as p (p.id)}
                <div class="perspective-item">
                  <div class="perspective-meta">
                    {#if p.target_archetype}
                      <span class="meta-tag archetype">{p.target_archetype}</span>
                    {/if}
                    {#if p.domain}
                      <span class="meta-tag domain">{p.domain}</span>
                    {/if}
                    <span class="meta-tag framing">{p.framing}</span>
                    <StatusBadge status={p.status} />
                  </div>
                  <p class="perspective-content">{p.content}</p>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      </div>

      <!-- Footer -->
      <div class="modal-footer">
        <div class="footer-left">
          <button class="btn btn-derive" onclick={() => showDeriveDialog = true}>
            Derive Perspectives
          </button>
        </div>
        <div class="footer-right">
          <button class="btn btn-delete" onclick={() => showDeleteConfirm = true}>Delete</button>
          <button class="btn btn-save" onclick={save} disabled={saving}>
            {saving ? 'Saving...' : 'Save'}
          </button>
        </div>
      </div>
    {/if}
  </div>
</div>

<!-- Derive Dialog -->
{#if showDeriveDialog}
  <DerivePerspectivesDialog
    {bulletId}
    onclose={() => showDeriveDialog = false}
    onderive={onDeriveComplete}
  />
{/if}

<!-- Delete Confirmation -->
<ConfirmDialog
  open={showDeleteConfirm}
  title="Delete Bullet"
  message="Are you sure you want to delete this bullet? This cannot be undone."
  confirmLabel="Delete"
  onconfirm={deleteBullet}
  oncancel={() => showDeleteConfirm = false}
/>

<style>
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal-content {
    background: #fff;
    border-radius: 10px;
    width: 90%;
    max-width: 640px;
    max-height: 85vh;
    display: flex;
    flex-direction: column;
    box-shadow: 0 8px 30px rgba(0, 0, 0, 0.15);
  }

  .loading-container {
    display: flex;
    justify-content: center;
    align-items: center;
    padding: 4rem 0;
  }

  .modal-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    padding: 1rem 1.25rem;
    border-bottom: 1px solid #e5e7eb;
    gap: 0.75rem;
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
    min-width: 0;
  }

  .modal-header h3 {
    font-size: 1rem;
    font-weight: 600;
    color: #1a1a2e;
    margin: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .close-btn {
    background: none;
    border: none;
    font-size: 1.3rem;
    color: #9ca3af;
    cursor: pointer;
    padding: 0.2rem;
    line-height: 1;
    flex-shrink: 0;
  }

  .close-btn:hover { color: #374151; }

  .modal-body {
    flex: 1;
    overflow-y: auto;
    padding: 1rem 1.25rem;
  }

  .status-actions {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
    margin-bottom: 1rem;
    padding-bottom: 0.75rem;
    border-bottom: 1px solid #f3f4f6;
  }

  .reject-inline {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    flex: 1;
  }

  .reject-input {
    flex: 1;
    padding: 0.3rem 0.5rem;
    border: 1px solid #fca5a5;
    border-radius: 4px;
    font-size: 0.78rem;
    color: #1a1a1a;
  }

  .reject-input:focus {
    outline: none;
    border-color: #ef4444;
    box-shadow: 0 0 0 2px rgba(239, 68, 68, 0.15);
  }

  .rejection-reason {
    font-size: 0.75rem;
    color: #ef4444;
    font-style: italic;
  }

  .field-group {
    margin-bottom: 1rem;
  }

  .field-label {
    display: block;
    font-size: 0.75rem;
    font-weight: 600;
    color: #6b7280;
    text-transform: uppercase;
    letter-spacing: 0.03em;
    margin-bottom: 0.3rem;
  }

  .field-textarea {
    width: 100%;
    padding: 0.5rem 0.65rem;
    border: 1px solid #d1d5db;
    border-radius: 6px;
    font-size: 0.82rem;
    color: #374151;
    font-family: inherit;
    line-height: 1.5;
    resize: vertical;
    min-height: 60px;
  }

  .content-textarea {
    min-height: 80px;
  }

  .field-textarea:focus {
    outline: none;
    border-color: #6c63ff;
    box-shadow: 0 0 0 2px rgba(108, 99, 255, 0.15);
  }

  .field-select {
    width: 100%;
    padding: 0.45rem 0.65rem;
    border: 1px solid #d1d5db;
    border-radius: 6px;
    font-size: 0.82rem;
    color: #374151;
    background: #fff;
    font-family: inherit;
  }

  .field-select:focus {
    outline: none;
    border-color: #6c63ff;
    box-shadow: 0 0 0 2px rgba(108, 99, 255, 0.15);
  }

  .tag-pills {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
    margin-bottom: 0.35rem;
  }

  .tag-pill {
    display: inline-flex;
    align-items: center;
    gap: 0.2rem;
    padding: 0.15em 0.45em;
    border-radius: 3px;
    font-size: 0.72rem;
    font-weight: 500;
  }

  .skill-pill {
    background: #dbeafe;
    color: #1e40af;
  }

  .tech-pill {
    background: #ede9fe;
    color: #5b21b6;
  }

  .pill-remove {
    background: none;
    border: none;
    font-size: 0.85rem;
    line-height: 1;
    cursor: pointer;
    color: inherit;
    opacity: 0.6;
    padding: 0;
  }

  .pill-remove:hover { opacity: 1; }

  .skill-picker {
    position: relative;
  }

  .skill-search,
  .tech-input {
    width: 100%;
    padding: 0.4rem 0.65rem;
    border: 1px solid #d1d5db;
    border-radius: 6px;
    font-size: 0.8rem;
    color: #1a1a1a;
  }

  .skill-search:focus,
  .tech-input:focus {
    outline: none;
    border-color: #6c63ff;
    box-shadow: 0 0 0 2px rgba(108, 99, 255, 0.15);
  }

  .skill-dropdown {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    background: #fff;
    border: 1px solid #d1d5db;
    border-radius: 6px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
    max-height: 200px;
    overflow-y: auto;
    z-index: 10;
    margin-top: 2px;
  }

  .dropdown-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 0.45rem 0.65rem;
    border: none;
    background: none;
    font-size: 0.8rem;
    color: #374151;
    cursor: pointer;
    text-align: left;
    font-family: inherit;
  }

  .dropdown-item:hover {
    background: #f3f4f6;
  }

  .dropdown-category {
    font-size: 0.68rem;
    color: #9ca3af;
  }

  .create-item {
    color: #6c63ff;
    font-weight: 500;
    border-top: 1px solid #e5e7eb;
  }

  .source-list {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }

  .source-item {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    font-size: 0.82rem;
    color: #374151;
    padding: 0.3rem 0.5rem;
    background: #f9fafb;
    border-radius: 4px;
  }

  .primary-star {
    color: #f59e0b;
    font-size: 0.9rem;
  }

  .source-title {
    flex: 1;
  }

  .empty-text {
    font-size: 0.78rem;
    color: #9ca3af;
    font-style: italic;
  }

  .perspective-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .perspective-item {
    padding: 0.5rem 0.65rem;
    background: #f9fafb;
    border: 1px solid #e5e7eb;
    border-radius: 6px;
  }

  .perspective-meta {
    display: flex;
    gap: 0.3rem;
    flex-wrap: wrap;
    margin-bottom: 0.25rem;
    align-items: center;
  }

  .meta-tag {
    display: inline-block;
    padding: 0.1em 0.4em;
    border-radius: 3px;
    font-size: 0.68rem;
    font-weight: 500;
  }

  .meta-tag.archetype { background: #eef2ff; color: #4f46e5; }
  .meta-tag.domain { background: #d1fae5; color: #065f46; }
  .meta-tag.framing { background: #fefce8; color: #a16207; }

  .perspective-content {
    font-size: 0.78rem;
    color: #4b5563;
    line-height: 1.4;
  }

  .modal-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.75rem 1.25rem;
    border-top: 1px solid #e5e7eb;
  }

  .footer-left {
    display: flex;
    gap: 0.5rem;
  }

  .footer-right {
    display: flex;
    gap: 0.5rem;
  }

  .btn {
    padding: 0.4rem 0.8rem;
    border: none;
    border-radius: 6px;
    font-size: 0.78rem;
    font-weight: 500;
    cursor: pointer;
    font-family: inherit;
    transition: background 0.15s;
  }

  .btn:disabled { opacity: 0.6; cursor: not-allowed; }

  .btn-save { background: #6c63ff; color: #fff; }
  .btn-save:hover:not(:disabled) { background: #5a52e0; }

  .btn-derive { background: #eef2ff; color: #4f46e5; }
  .btn-derive:hover { background: #dbeafe; }

  .btn-delete { background: #fee2e2; color: #dc2626; }
  .btn-delete:hover { background: #fecaca; }

  .btn-submit { background: #dbeafe; color: #1e40af; }
  .btn-submit:hover { background: #bfdbfe; }

  .btn-approve { background: #d1fae5; color: #065f46; }
  .btn-approve:hover { background: #bbf7d0; }

  .btn-reject { background: #fee2e2; color: #dc2626; }
  .btn-reject:hover { background: #fecaca; }

  .btn-reopen { background: #fef3c7; color: #92400e; }
  .btn-reopen:hover { background: #fde68a; }

  .btn-danger-sm {
    padding: 0.25rem 0.5rem;
    border: none;
    border-radius: 4px;
    font-size: 0.72rem;
    font-weight: 500;
    cursor: pointer;
    background: #ef4444;
    color: #fff;
  }

  .btn-danger-sm:hover { background: #dc2626; }

  .btn-ghost-sm {
    padding: 0.25rem 0.5rem;
    border: none;
    border-radius: 4px;
    font-size: 0.72rem;
    font-weight: 500;
    cursor: pointer;
    background: transparent;
    color: #6b7280;
  }

  .btn-ghost-sm:hover { color: #374151; background: #f3f4f6; }
</style>
```

**Acceptance criteria:**
- Modal opens with all data loaded (bullet fields, skills, sources, perspectives, domain list, skill list).
- Loading spinner shows while data is being fetched.
- Content and notes are editable textareas.
- Domain dropdown is populated from `/api/domains` and displays `domain.name`.
- Skills display as tag pills with remove button; skill picker shows search + create.
- Technologies display as tag pills with remove button; text input adds on Enter (lowercased).
- Sources are read-only with primary source marked by a star.
- Perspectives list shows archetype/domain/framing + status badge.
- Perspective items are rendered as non-interactive divs (no click handler). Clicking a perspective does nothing in this phase.
- Status actions match the transition table (draft: Submit, pending_review: Approve/Reject, rejected: Reopen, approved: none).
- Reject shows inline input for reason before confirming.
- Save sends content, notes, domain, technologies via `forge.bullets.update`.
- Delete shows `ConfirmDialog` and calls `forge.bullets.delete`; shows error toast on CONFLICT.
- Modal closes on X click, Escape, backdrop click.
- `onupdate()` is called after any mutation.

**Failure criteria:**
- Missing data loads cause the modal to break (null access errors).
- Status action buttons appear for wrong statuses.
- Technologies are not lowercased before local state.
- Skills are not immediately reflected in the pill list after add/remove.
- Raw `fetch` is used instead of SDK methods.

---

### T41.6: Create `DerivePerspectivesDialog.svelte`

**File:** `packages/webui/src/lib/components/DerivePerspectivesDialog.svelte`

A dialog overlay for selecting archetype, domain, and framing before triggering AI perspective derivation. Dropdowns are loaded from the API (not hardcoded). Derive button is disabled while dropdowns are loading or if either archetype/domain has no options.

```svelte
<script lang="ts">
  import { forge, friendlyError } from '$lib/sdk'
  import { addToast } from '$lib/stores/toast.svelte'
  import { LoadingSpinner } from '$lib/components'

  let { bulletId, onclose, onderive }: {
    bulletId: string
    onclose: () => void
    onderive: () => void
  } = $props()

  let archetypes = $state<Array<{ id: string; name: string }>>([])
  let domains = $state<Array<{ id: string; name: string }>>([])
  let loadingOptions = $state(true)
  let deriving = $state(false)

  let selectedArchetype = $state('')
  let selectedDomain = $state('')
  let selectedFraming = $state<'accomplishment' | 'responsibility' | 'context'>('accomplishment')

  const FRAMING_OPTIONS: Array<{ value: 'accomplishment' | 'responsibility' | 'context'; label: string }> = [
    { value: 'accomplishment', label: 'Accomplishment' },
    { value: 'responsibility', label: 'Responsibility' },
    { value: 'context', label: 'Context' },
  ]

  let canDerive = $derived(
    !loadingOptions &&
    !deriving &&
    archetypes.length > 0 &&
    domains.length > 0 &&
    selectedArchetype !== '' &&
    selectedDomain !== ''
  )

  $effect(() => {
    loadOptions()
  })

  async function loadOptions() {
    loadingOptions = true

    const [archetypeRes, domainRes] = await Promise.all([
      forge.archetypes.list({ limit: 200 }),
      forge.domains.list({ limit: 200 }),
    ])

    if (archetypeRes.ok) {
      archetypes = archetypeRes.data
      if (archetypes.length > 0) selectedArchetype = archetypes[0].name
    }
    if (domainRes.ok) {
      domains = domainRes.data
      if (domains.length > 0) selectedDomain = domains[0].name
    }

    loadingOptions = false
  }

  async function derive() {
    if (!canDerive) return
    deriving = true

    const res = await forge.bullets.derivePerspectives(bulletId, {
      archetype: selectedArchetype,
      domain: selectedDomain,
      framing: selectedFraming,
    })

    if (res.ok) {
      addToast({ message: 'Perspectives derived successfully', type: 'success' })
      onderive()
      onclose()
    } else {
      addToast({ message: friendlyError(res.error, 'Derivation failed'), type: 'error' })
    }

    deriving = false
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.stopPropagation()
      onclose()
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div class="dialog-overlay" onclick={onclose} role="presentation">
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="dialog" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true" aria-label="Derive Perspectives">
    <div class="dialog-header">
      <h3>Derive Perspectives</h3>
      <button class="close-btn" onclick={onclose}>&times;</button>
    </div>

    <div class="dialog-body">
      {#if loadingOptions}
        <div class="loading-container">
          <LoadingSpinner size="md" message="Loading options..." />
        </div>
      {:else}
        <div class="form-group">
          <label for="derive-archetype">Archetype</label>
          <select id="derive-archetype" bind:value={selectedArchetype}>
            {#each archetypes as arch}
              <option value={arch.name}>{arch.name}</option>
            {/each}
          </select>
          {#if archetypes.length === 0}
            <span class="field-hint">No archetypes available</span>
          {/if}
        </div>

        <div class="form-group">
          <label for="derive-domain">Domain</label>
          <select id="derive-domain" bind:value={selectedDomain}>
            {#each domains as dom}
              <option value={dom.name}>{dom.name}</option>
            {/each}
          </select>
          {#if domains.length === 0}
            <span class="field-hint">No domains available</span>
          {/if}
        </div>

        <div class="form-group">
          <label for="derive-framing">Framing</label>
          <select id="derive-framing" bind:value={selectedFraming}>
            {#each FRAMING_OPTIONS as opt}
              <option value={opt.value}>{opt.label}</option>
            {/each}
          </select>
        </div>
      {/if}
    </div>

    <div class="dialog-actions">
      <button class="btn btn-ghost" onclick={onclose}>Cancel</button>
      <button class="btn btn-primary" onclick={derive} disabled={!canDerive}>
        {#if deriving}
          Deriving...
        {:else}
          Derive
        {/if}
      </button>
    </div>
  </div>
</div>

<style>
  .dialog-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 10000;
  }

  .dialog {
    background: #fff;
    border-radius: 8px;
    width: 90%;
    max-width: 420px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.2);
  }

  .dialog-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 1rem 1.25rem;
    border-bottom: 1px solid #e5e7eb;
  }

  .dialog-header h3 {
    font-size: 1rem;
    font-weight: 600;
    color: #1a1a2e;
    margin: 0;
  }

  .close-btn {
    background: none;
    border: none;
    font-size: 1.2rem;
    color: #9ca3af;
    cursor: pointer;
    padding: 0.2rem;
    line-height: 1;
  }

  .close-btn:hover { color: #374151; }

  .dialog-body {
    padding: 1.25rem;
  }

  .loading-container {
    display: flex;
    justify-content: center;
    padding: 2rem 0;
  }

  .form-group {
    margin-bottom: 1rem;
  }

  .form-group label {
    display: block;
    font-size: 0.8rem;
    font-weight: 500;
    color: #374151;
    margin-bottom: 0.35rem;
  }

  .form-group select {
    width: 100%;
    padding: 0.5rem 0.65rem;
    border: 1px solid #d1d5db;
    border-radius: 6px;
    font-size: 0.875rem;
    color: #1a1a1a;
    background: #fff;
    font-family: inherit;
  }

  .form-group select:focus {
    outline: none;
    border-color: #6c63ff;
    box-shadow: 0 0 0 2px rgba(108, 99, 255, 0.15);
  }

  .field-hint {
    font-size: 0.7rem;
    color: #ef4444;
    font-style: italic;
    margin-top: 0.2rem;
    display: block;
  }

  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.75rem;
    padding: 0.75rem 1.25rem;
    border-top: 1px solid #e5e7eb;
  }

  .btn {
    padding: 0.45rem 0.9rem;
    border: none;
    border-radius: 6px;
    font-size: 0.8rem;
    font-weight: 500;
    cursor: pointer;
    font-family: inherit;
    transition: background 0.15s;
  }

  .btn:disabled { opacity: 0.6; cursor: not-allowed; }
  .btn-primary { background: #6c63ff; color: #fff; }
  .btn-primary:hover:not(:disabled) { background: #5a52e0; }
  .btn-ghost { background: transparent; color: #6b7280; }
  .btn-ghost:hover { color: #374151; background: #f3f4f6; }
</style>
```

**Acceptance criteria:**
- Dialog loads archetypes and domains from the API (not hardcoded).
- Shows loading spinner while dropdown data loads.
- Derive button is disabled while loading or if archetypes/domains have no options.
- Framing dropdown has three hardcoded options: accomplishment, responsibility, context.
- Archetype and domain dropdowns display `name` and send `name` (not ID).
- Successful derivation calls `onderive()`, shows success toast, and closes.
- Failed derivation shows error toast and stays open.
- Dialog closes on X, Escape, or backdrop click.
- Escape in dialog does not close the parent BulletDetailModal (stopPropagation).

**Failure criteria:**
- Hardcoded archetype/domain arrays instead of API-loaded.
- Derive button enabled before dropdowns finish loading.
- Dialog does not close on success.
- Parent modal closes when pressing Escape in the dialog.

---

### T41.7: Integrate Modal into `BulletsView.svelte`

**File:** `packages/webui/src/routes/data/sources/BulletsView.svelte`

Add click handler to bullet cards to open `BulletDetailModal`. Remove the existing inline derive modal (hardcoded `ARCHETYPES`, `DOMAINS`, `deriveModal` state, `openDeriveModal`, `submitDerive`, and the derive modal template). The card-level "Derive Perspective" button is removed -- derivation is now accessed through the modal.

**Changes to the `<script>` block:**

1. Add import for `BulletDetailModal`:
```typescript
  import BulletDetailModal from '$lib/components/BulletDetailModal.svelte'
```

2. Add `detailBulletId` state:
```typescript
  let detailBulletId = $state<string | null>(null)
```

3. Remove the following (lines 16-23):
```typescript
  // Derive perspective modal
  let deriveModal = $state({
    open: false,
    bulletId: '',
    archetype: 'agentic-ai',
    domain: 'ai_ml',
    framing: 'accomplishment' as 'accomplishment' | 'responsibility' | 'context',
    submitting: false,
  })
```

4. Remove `openDeriveModal` function (lines 145-154):
```typescript
  function openDeriveModal(bulletId: string) {
    deriveModal = {
      open: true,
      bulletId,
      archetype: 'agentic-ai',
      domain: 'ai_ml',
      framing: 'accomplishment',
      submitting: false,
    }
  }
```

5. Remove `submitDerive` function (lines 156-170):
```typescript
  async function submitDerive() {
    deriveModal.submitting = true

    const res = await forge.bullets.derivePerspectives(deriveModal.bulletId, {
      archetype: deriveModal.archetype,
      domain: deriveModal.domain,
      framing: deriveModal.framing,
    })
    if (res.ok) {
      addToast({ message: 'Perspective derived successfully', type: 'success' })
      deriveModal.open = false
    } else {
      addToast({ message: `Derive failed: ${res.error.message}`, type: 'error' })
    }

    deriveModal.submitting = false
  }
```

6. Remove `ARCHETYPES` and `DOMAINS` constants (lines 179-180):
```typescript
  const ARCHETYPES = ['agentic-ai', 'infrastructure', 'security-engineer', 'solutions-architect', 'public-sector', 'hft']
  const DOMAINS = ['systems_engineering', 'software_engineering', 'security', 'devops', 'ai_ml', 'leadership']
```

**Changes to the template:**

7. Add `onclick` and `cursor: pointer` style to the `.item-card` div (line 233):
```svelte
        <div class="item-card" style="cursor: pointer;" onclick={() => detailBulletId = item.id}>
```

8. Add `e.stopPropagation()` to all existing action buttons inside bullet cards (Approve, Reject, Reopen, Derive) to prevent the card click handler from also triggering. Example: change `onclick={() => approveBullet(item.id)}` to `onclick={(e) => { e.stopPropagation(); approveBullet(item.id) }}`.

9. Remove the card-level "Derive Perspective" button (lines 286-289):
```svelte
            {#if contentType === 'bullet' && item.status === 'approved'}
              <button class="btn btn-derive-action" onclick={() => openDeriveModal(item.id)}>
                Derive Perspective
              </button>
            {/if}
```

10. Remove the entire Derive Perspective Modal template (lines 327-371) -- the `{#if deriveModal.open}...{/if}` block.

11. Add `BulletDetailModal` mount after the closing `</div>` of the `.bullets-page` container:
```svelte
{#if detailBulletId}
  <BulletDetailModal
    bulletId={detailBulletId}
    onclose={() => detailBulletId = null}
    onupdate={() => loadItems()}
  />
{/if}
```

12. Remove unused CSS classes: `.btn-derive-action` and `.btn-derive-action:hover`.

**Design note:** The card-level Approve/Reject/Reopen buttons are intentionally RETAINED in BulletsView for quick status actions without opening the modal. The modal provides the same status actions for users who prefer the detail view. Both UX paths coexist -- the modal is not the only way to change status. This is an intentional design choice, not a conflict.

**Acceptance criteria:**
- Clicking a bullet card sets `detailBulletId` and opens `BulletDetailModal`.
- The card-level "Derive Perspective" button is removed from all bullet cards.
- The card-level Approve/Reject/Reopen buttons are retained for quick review workflows.
- The inline derive modal (with hardcoded `ARCHETYPES`/`DOMAINS`) is completely removed.
- The reject modal still works as before (it is independent of these changes).
- After the modal triggers `onupdate`, the bullet list refreshes.
- After the modal closes, `detailBulletId` resets to null.

**Failure criteria:**
- Old derive modal remnants cause TypeScript errors.
- Card click propagation issues (e.g., clicking approve/reject buttons inside the card also opens the modal).
- Bullet list does not refresh after modal mutations.

---

## Testing Support

### Test Fixtures

The existing `createTestDb()` helper runs all migrations. No changes to helpers are needed. The `seedBullet()` helper creates bullets with source associations. For skill-related tests, skills are created inline via the API (`POST /api/skills`).

For bullet skills tests, seed data like this:
```typescript
// Create a skill via direct SQL (no seedSkill helper exists)
const skillId = crypto.randomUUID()
ctx.db.run('INSERT INTO skills (id, name, category) VALUES (?, ?, ?)', [skillId, 'Python', 'language'])

// Link it to a bullet
ctx.db.run('INSERT INTO bullet_skills (bullet_id, skill_id) VALUES (?, ?)', [bulletId, skillId])
```

### Unit Tests

**File:** `packages/core/src/db/repositories/__tests__/bullet-repository.test.ts` (add or update)

| Test | Assertion |
|------|-----------|
| `update()` with `{ notes: 'test note' }` sets notes | `bullet.notes === 'test note'` |
| `update()` with `{ notes: null }` clears notes | `bullet.notes === null` |
| `update()` with `{ domain: 'security' }` sets domain | `bullet.domain === 'security'` |
| `update()` with `{ domain: null }` clears domain | `bullet.domain === null` |
| `update()` with `{ technologies: ['python', 'rust'] }` replaces technologies | `bullet.technologies === ['python', 'rust']` |
| `update()` with `{ technologies: [] }` clears technologies | `bullet.technologies === []` |
| `update()` with `{ technologies: ['Python'] }` lowercases | `bullet.technologies === ['python']` |
| `update()` with `{}` returns bullet unchanged | No changes to any field |
| `update()` with only `technologies` returns bullet | `bullet !== null`, technologies replaced |
| `update()` with `{ content: 'x', notes: 'y', technologies: ['go'] }` updates all | All three fields updated |

### Integration Tests

**File:** `packages/core/src/routes/__tests__/bullets.test.ts` (new file). Use `packages/core/src/routes/__tests__/sources.test.ts` as the structural template.

| Test | Route | Assertion |
|------|-------|-----------|
| GET bullet skills returns linked skills | `GET /api/bullets/:id/skills` | 200, `data` is array of Skill objects |
| POST bullet skills with `{ skill_id }` links skill | `POST /api/bullets/:id/skills` | 201, `data.id === skillId` |
| POST bullet skills with `{ name: 'python' }` creates Newskill | `POST /api/bullets/:id/skills` | 201, `data.name === 'Python'` (capitalizeFirst) |
| POST bullet skills with duplicate is idempotent | `POST /api/bullets/:id/skills` | 201, no error; GET returns skill once |
| POST bullet skills case-insensitive dedup | `POST /api/bullets/:id/skills` | 201, existing `Python` skill is linked when `{ name: 'python' }` sent |
| POST bullet skills without name or skill_id returns 400 | `POST /api/bullets/:id/skills` | 400, `error.code === 'VALIDATION_ERROR'` |
| DELETE bullet skill link returns 204 | `DELETE /api/bullets/:bulletId/skills/:skillId` | 204 |
| DELETE non-existent bullet skill link returns 404 | `DELETE /api/bullets/:bulletId/skills/:skillId` | 404, `error.code === 'NOT_FOUND'` |
| GET bullet sources returns sources with is_primary | `GET /api/bullets/:id/sources` | 200, `data[0].is_primary` is 0 or 1, `data[0].title` exists |
| GET bullet sources for bullet with no sources returns empty | `GET /api/bullets/:id/sources` | 200, `data === []` |
| PATCH bullet update with `{ notes: 'new' }` updates notes | `PATCH /api/bullets/:id` | 200, `data.notes === 'new'` |
| PATCH bullet update with `{ domain: 'security' }` updates domain | `PATCH /api/bullets/:id` | 200, `data.domain === 'security'` |
| PATCH bullet update with `{ domain: null }` clears domain | `PATCH /api/bullets/:id` | 200, `data.domain === null` |
| PATCH bullet update with `{ technologies: ['python', 'rust'] }` replaces | `PATCH /api/bullets/:id` | 200, `data.technologies` equals `['python', 'rust']` |
| PATCH bullet update with `{ technologies: [] }` clears | `PATCH /api/bullets/:id` | 200, `data.technologies === []` |
| PATCH submit transitions draft to pending_review | `PATCH /api/bullets/:id/submit` | 200, `data.status === 'pending_review'` |
| PATCH submit on non-draft returns 400 | `PATCH /api/bullets/:id/submit` | 400, `error.code === 'VALIDATION_ERROR'` |
| PATCH submit on non-existent bullet returns 404 | `PATCH /api/bullets/nonexistent/submit` | 404 |

### Component Smoke Tests

These are manual acceptance checks or future browser automation tests.

| Test | What to verify |
|------|---------------|
| Modal opens on card click | Clicking a bullet card opens the detail modal |
| All fields populate correctly | Content, notes, domain, skills, technologies, sources, perspectives load |
| Loading spinner during fetch | Spinner visible while data loads |
| Content textarea is editable | Typing in the content area changes text |
| Status buttons match transitions | draft: Submit; pending_review: Approve/Reject; rejected: Reopen; approved: none |
| Skills add via search | Searching "Py" shows Python; clicking adds pill |
| Skills create inline | Typing "NewSkill" + Enter creates and adds pill |
| Skills remove via X | Clicking X on skill pill removes it |
| Technologies add via Enter | Typing "docker" + Enter adds lowercased pill |
| Technologies remove via X | Clicking X on tech pill removes it |
| Sources display read-only | Sources list shows titles, primary marked with star, no edit controls |
| Perspectives list shows metadata | Each perspective shows archetype/domain/framing + status badge |
| Derive button opens dialog | "Derive Perspectives" opens DerivePerspectivesDialog |
| Derive dialog loads from API | Archetype/domain dropdowns populated from API, not hardcoded |
| Derive button disabled while loading | Button grayed out until dropdowns finish loading |
| Delete shows ConfirmDialog | "Delete" button opens confirmation |
| Delete with perspectives shows error | CONFLICT response shows toast, modal stays open |
| Modal closes on X/Escape/backdrop | All three close methods work |
| Old derive modal removed | No "Derive Perspective" button on cards; no inline derive modal in DOM |

### Contract Tests

| Test | What to verify |
|------|---------------|
| SDK `UpdateBullet` type includes notes, domain, technologies | `forge.bullets.update(id, { notes: 'x', domain: 'y', technologies: ['z'] })` compiles |
| SDK `BulletsResource.submit` exists | `forge.bullets.submit(id)` compiles and calls correct endpoint |
| SDK `BulletsResource.listSkills` returns `Skill[]` | Return type matches `Result<Skill[]>` |
| SDK `BulletsResource.addSkill` accepts both input shapes | `{ skill_id }` and `{ name }` both compile |
| SDK `BulletsResource.listSources` returns sources with `is_primary` | Return type matches `Result<Array<Source & { is_primary: number }>>` |

---

## Documentation Requirements

- No new documentation files required (non-goal).
- The spec file serves as the design document.
- This plan file serves as the implementation reference.
- Inline TSDoc comments in SDK `BulletsResource` methods explain what each endpoint does.
- Inline code comments in `bullet-repository.ts` explain the technology DELETE-all + INSERT-all pattern.

---

## Parallelization Notes

**Within this phase:**
- T41.1 (repository) and T41.2 (service) are independent of each other -- they modify different files with no cross-dependencies. They can be developed in parallel.
- T41.3 (routes + server.ts) depends on T41.2 (needs `submitBullet` on the service) and T41.1 (needs the extended `UpdateBulletInput` to flow through the PATCH handler). Implement T41.1 and T41.2 first, then T41.3.
- T41.4 (SDK) CAN be developed and committed in parallel with T41.1-T41.3 (no build-time dependency). However, end-to-end testing of the SDK methods requires the backend endpoints from T41.1-T41.3 to be complete.
- T41.5 (BulletDetailModal) and T41.6 (DerivePerspectivesDialog) can be developed in parallel -- T41.5 imports T41.6 but only conditionally renders it. Both depend on T41.4 (SDK methods must exist for the UI to call them).
- T41.7 (BulletsView integration) depends on T41.5 (imports BulletDetailModal). It should be the last UI task.

**Recommended execution order:**
1. T41.1 + T41.2 (repository + service -- foundational, parallel)
2. T41.3 (routes -- depends on T41.1 + T41.2)
3. T41.4 (SDK -- can parallel with steps 1-2 in development, commit together)
4. T41.5 + T41.6 (UI components -- parallel, depend on T41.4)
5. T41.7 (BulletsView integration -- depends on T41.5, final step)

**Cross-phase:**
- This phase has no migration dependencies and does not conflict with Phases 29-40 at the code level. The only shared files are `bullet-repository.ts`, `bullet-service.ts`, `bullets.ts` (routes), and `server.ts`, which are not modified by other parallel phases.
- The `bullet_skills` and `bullet_technologies` tables already exist from migration 001. No migration coordination is needed.
