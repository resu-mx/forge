import { describe, test, expect, beforeEach, afterEach } from 'bun:test'
import type { Database } from 'bun:sqlite'
import { createTestApp, apiRequest, type TestContext } from './helpers'
import { seedSource, seedBullet, seedPerspective } from '../../db/__tests__/helpers'

describe('Bullet Routes', () => {
  let ctx: TestContext

  beforeEach(() => {
    ctx = createTestApp()
  })

  afterEach(() => {
    ctx.db.close()
  })

  // ── GET /bullets ───────────────────────────────────────────────────

  test('GET /bullets returns 200 with pagination', async () => {
    const sourceId = seedSource(ctx.db)
    seedBullet(ctx.db, [{ id: sourceId }], { content: 'Bullet A' })
    seedBullet(ctx.db, [{ id: sourceId }], { content: 'Bullet B' })

    const res = await apiRequest(ctx.app, 'GET', '/bullets')
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data).toBeArray()
    expect(body.data.length).toBe(2)
    expect(body.pagination).toBeDefined()
    expect(body.pagination.total).toBe(2)
  })

  test('GET /bullets?status=approved filters by status', async () => {
    const sourceId = seedSource(ctx.db)
    seedBullet(ctx.db, [{ id: sourceId }], { status: 'approved' })
    seedBullet(ctx.db, [{ id: sourceId }], { status: 'in_review' })

    const res = await apiRequest(ctx.app, 'GET', '/bullets?status=approved')
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.length).toBe(1)
    expect(body.data[0].status).toBe('approved')
  })

  // ── GET /bullets/:id ──────────────────────────────────────────────

  test('GET /bullets/:id returns 200', async () => {
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }])

    const res = await apiRequest(ctx.app, 'GET', `/bullets/${bulletId}`)
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.id).toBe(bulletId)
  })

  // ── PATCH /bullets/:id/approve ────────────────────────────────────

  test('PATCH /bullets/:id/approve from in_review returns 200', async () => {
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }], { status: 'in_review' })

    const res = await apiRequest(ctx.app, 'PATCH', `/bullets/${bulletId}/approve`)
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.status).toBe('approved')
  })

  test('PATCH /bullets/:id/approve from draft returns 400', async () => {
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }], { status: 'draft' })

    const res = await apiRequest(ctx.app, 'PATCH', `/bullets/${bulletId}/approve`)
    expect(res.status).toBe(400)
    const body = await res.json()
    expect(body.error.code).toBe('VALIDATION_ERROR')
    expect(body.error.message).toContain('Cannot transition')
  })

  // ── PATCH /bullets/:id/reject ─────────────────────────────────────

  test('PATCH /bullets/:id/reject with reason returns 200', async () => {
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }], { status: 'in_review' })

    const res = await apiRequest(ctx.app, 'PATCH', `/bullets/${bulletId}/reject`, {
      rejection_reason: 'Too vague',
    })
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.status).toBe('rejected')
  })

  test('PATCH /bullets/:id/reject without reason returns 400', async () => {
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }], { status: 'in_review' })

    const res = await apiRequest(ctx.app, 'PATCH', `/bullets/${bulletId}/reject`, {
      rejection_reason: '',
    })
    expect(res.status).toBe(400)
    const body = await res.json()
    expect(body.error.code).toBe('VALIDATION_ERROR')
  })

  // ── PATCH /bullets/:id/reopen ─────────────────────────────────────

  test('PATCH /bullets/:id/reopen from rejected returns 200', async () => {
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }], { status: 'rejected' })

    const res = await apiRequest(ctx.app, 'PATCH', `/bullets/${bulletId}/reopen`)
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.status).toBe('in_review')
  })

  // ── DELETE /bullets/:id ───────────────────────────────────────────

  test('DELETE /bullets/:id returns 204', async () => {
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }])

    const res = await apiRequest(ctx.app, 'DELETE', `/bullets/${bulletId}`)
    expect(res.status).toBe(204)
  })

  test('DELETE /bullets/:id with perspectives returns 409', async () => {
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }])
    seedPerspective(ctx.db, bulletId)

    const res = await apiRequest(ctx.app, 'DELETE', `/bullets/${bulletId}`)
    expect(res.status).toBe(409)
    const body = await res.json()
    expect(body.error.code).toBe('CONFLICT')
  })

  // ── PATCH /bullets/:id (extended fields) ───────────────────────────

  test('PATCH /bullets/:id with domain updates domain', async () => {
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }])

    const res = await apiRequest(ctx.app, 'PATCH', `/bullets/${bulletId}`, { domain: 'security' })
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.domain).toBe('security')
  })

  test('PATCH /bullets/:id with domain null clears domain', async () => {
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }], { domain: 'security' })

    const res = await apiRequest(ctx.app, 'PATCH', `/bullets/${bulletId}`, { domain: null })
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.domain).toBeNull()
  })

  test('PATCH /bullets/:id with technologies replaces all', async () => {
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }])

    const res = await apiRequest(ctx.app, 'PATCH', `/bullets/${bulletId}`, {
      technologies: ['python', 'rust'],
    })
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.technologies).toEqual(['python', 'rust'])
  })

  test('PATCH /bullets/:id with empty technologies clears all', async () => {
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }])
    // Phase 89: technologies are now backed by bullet_skills + skills.
    // Seed via the skill + bullet_skills junction.
    const skillId = crypto.randomUUID()
    ctx.db.run(`INSERT INTO skills (id, name, category) VALUES (?, ?, 'other')`, [skillId, 'ts'])
    ctx.db.run(`INSERT INTO bullet_skills (bullet_id, skill_id) VALUES (?, ?)`, [bulletId, skillId])

    const res = await apiRequest(ctx.app, 'PATCH', `/bullets/${bulletId}`, { technologies: [] })
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.technologies).toEqual([])
  })

  // ── PATCH /bullets/:id/submit ──────────────────────────────────────

  test('PATCH /bullets/:id/submit transitions draft to in_review', async () => {
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }], { status: 'draft' })

    const res = await apiRequest(ctx.app, 'PATCH', `/bullets/${bulletId}/submit`)
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.status).toBe('in_review')
  })

  test('PATCH /bullets/:id/submit on non-draft returns 400', async () => {
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }], { status: 'in_review' })

    const res = await apiRequest(ctx.app, 'PATCH', `/bullets/${bulletId}/submit`)
    expect(res.status).toBe(400)
    const body = await res.json()
    expect(body.error.code).toBe('VALIDATION_ERROR')
  })

  test('PATCH /bullets/nonexistent/submit returns 404', async () => {
    const res = await apiRequest(ctx.app, 'PATCH', '/bullets/nonexistent/submit')
    expect(res.status).toBe(404)
  })

  // ── Bullet Skills ──────────────────────────────────────────────────

  test('GET /bullets/:id/skills returns linked skills', async () => {
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }])

    // Create a skill and link it
    const skillId = crypto.randomUUID()
    ctx.db.run('INSERT INTO skills (id, name, category) VALUES (?, ?, ?)', [skillId, 'Python', 'language'])
    ctx.db.run('INSERT INTO bullet_skills (bullet_id, skill_id) VALUES (?, ?)', [bulletId, skillId])

    const res = await apiRequest(ctx.app, 'GET', `/bullets/${bulletId}/skills`)
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data).toBeArray()
    expect(body.data.length).toBe(1)
    expect(body.data[0].name).toBe('Python')
  })

  test('POST /bullets/:id/skills with skill_id links existing skill', async () => {
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }])

    const skillId = crypto.randomUUID()
    ctx.db.run('INSERT INTO skills (id, name, category) VALUES (?, ?, ?)', [skillId, 'Go', 'language'])

    const res = await apiRequest(ctx.app, 'POST', `/bullets/${bulletId}/skills`, { skill_id: skillId })
    expect(res.status).toBe(201)
    const body = await res.json()
    expect(body.data.id).toBe(skillId)
  })

  test('POST /bullets/:id/skills with name creates new skill (capitalizeFirst)', async () => {
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }])

    const res = await apiRequest(ctx.app, 'POST', `/bullets/${bulletId}/skills`, { name: 'newskill' })
    expect(res.status).toBe(201)
    const body = await res.json()
    expect(body.data.name).toBe('Newskill')
  })

  test('POST /bullets/:id/skills with name case-insensitive dedup links existing', async () => {
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }])

    const skillId = crypto.randomUUID()
    ctx.db.run('INSERT INTO skills (id, name, category) VALUES (?, ?, ?)', [skillId, 'Python', 'language'])

    const res = await apiRequest(ctx.app, 'POST', `/bullets/${bulletId}/skills`, { name: 'python' })
    expect(res.status).toBe(201)
    const body = await res.json()
    expect(body.data.id).toBe(skillId)
    expect(body.data.name).toBe('Python')
  })

  test('POST /bullets/:id/skills duplicate is idempotent', async () => {
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }])

    const skillId = crypto.randomUUID()
    ctx.db.run('INSERT INTO skills (id, name, category) VALUES (?, ?, ?)', [skillId, 'Rust', 'language'])
    ctx.db.run('INSERT INTO bullet_skills (bullet_id, skill_id) VALUES (?, ?)', [bulletId, skillId])

    const res = await apiRequest(ctx.app, 'POST', `/bullets/${bulletId}/skills`, { skill_id: skillId })
    expect(res.status).toBe(201)

    // Verify only one link exists
    const getRes = await apiRequest(ctx.app, 'GET', `/bullets/${bulletId}/skills`)
    const getBody = await getRes.json()
    expect(getBody.data.length).toBe(1)
  })

  test('POST /bullets/:id/skills without name or skill_id returns 400', async () => {
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }])

    const res = await apiRequest(ctx.app, 'POST', `/bullets/${bulletId}/skills`, {})
    expect(res.status).toBe(400)
    const body = await res.json()
    expect(body.error.code).toBe('VALIDATION_ERROR')
  })

  test('DELETE /bullets/:bulletId/skills/:skillId returns 204', async () => {
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }])

    const skillId = crypto.randomUUID()
    ctx.db.run('INSERT INTO skills (id, name, category) VALUES (?, ?, ?)', [skillId, 'Go', 'language'])
    ctx.db.run('INSERT INTO bullet_skills (bullet_id, skill_id) VALUES (?, ?)', [bulletId, skillId])

    const res = await apiRequest(ctx.app, 'DELETE', `/bullets/${bulletId}/skills/${skillId}`)
    expect(res.status).toBe(204)
  })

  test('DELETE /bullets/:bulletId/skills/:skillId not found returns 404', async () => {
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }])

    const res = await apiRequest(ctx.app, 'DELETE', `/bullets/${bulletId}/skills/${crypto.randomUUID()}`)
    expect(res.status).toBe(404)
    const body = await res.json()
    expect(body.error.code).toBe('NOT_FOUND')
  })

  // ── Bullet Sources ─────────────────────────────────────────────────

  test('GET /bullets/:id/sources returns sources with is_primary', async () => {
    const sourceId = seedSource(ctx.db, { title: 'My Source' })
    const bulletId = seedBullet(ctx.db, [{ id: sourceId, isPrimary: true }])

    const res = await apiRequest(ctx.app, 'GET', `/bullets/${bulletId}/sources`)
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data).toBeArray()
    expect(body.data.length).toBe(1)
    expect(body.data[0].title).toBe('My Source')
    expect(body.data[0].is_primary).toBe(1)
  })

  test('POST /bullets with source_ids [{ id, is_primary }] links the source', async () => {
    const sourceId = seedSource(ctx.db, { title: 'Linked Source' })

    const res = await apiRequest(ctx.app, 'POST', '/bullets', {
      content: 'Bullet linked via source_ids',
      source_ids: [{ id: sourceId, is_primary: true }],
    })
    expect(res.status).toBe(201)
    const created = await res.json()

    const srcRes = await apiRequest(ctx.app, 'GET', `/bullets/${created.data.id}/sources`)
    expect(srcRes.status).toBe(200)
    const body = await srcRes.json()
    expect(body.data.length).toBe(1)
    expect(body.data[0].id).toBe(sourceId)
    expect(body.data[0].is_primary).toBeTruthy()
  })

  test('GET /bullets/:id/sources for bullet with no sources returns empty', async () => {
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [])

    const res = await apiRequest(ctx.app, 'GET', `/bullets/${bulletId}/sources`)
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data).toEqual([])
  })
})
