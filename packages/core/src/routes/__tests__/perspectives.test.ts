import { describe, test, expect, beforeEach, afterEach } from 'bun:test'
import { Database } from 'bun:sqlite'
import { createTestApp, apiRequest, type TestContext } from './helpers'
import { seedSource, seedBullet, seedPerspective, seedResume, seedResumeEntry, seedResumeSection } from '../../db/__tests__/helpers'

describe('Perspective Routes', () => {
  let ctx: TestContext

  beforeEach(() => {
    ctx = createTestApp()
  })

  afterEach(() => {
    ctx.db.close()
  })

  // ── GET /perspectives ─────────────────────────────────────────────

  test('GET /perspectives returns 200 with pagination', async () => {
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }])
    seedPerspective(ctx.db, bulletId, { content: 'Perspective A' })
    seedPerspective(ctx.db, bulletId, { content: 'Perspective B' })

    const res = await apiRequest(ctx.app, 'GET', '/perspectives')
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data).toBeArray()
    expect(body.data.length).toBe(2)
    expect(body.pagination).toBeDefined()
  })

  // ── GET /perspectives/:id ─────────────────────────────────────────

  test('GET /perspectives/:id returns 200 with chain (bullet + source)', async () => {
    const sourceId = seedSource(ctx.db, { title: 'Chain Source' })
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }], { content: 'Chain Bullet' })
    const perspId = seedPerspective(ctx.db, bulletId, { content: 'Chain Perspective' })

    const res = await apiRequest(ctx.app, 'GET', `/perspectives/${perspId}`)
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.id).toBe(perspId)
    expect(body.data.bullet).toBeDefined()
    expect(body.data.bullet.id).toBe(bulletId)
    expect(body.data.source).toBeDefined()
    expect(body.data.source.id).toBe(sourceId)
  })

  // ── PATCH /perspectives/:id/approve ───────────────────────────────

  test('PATCH /perspectives/:id/approve from in_review returns 200', async () => {
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }])
    const perspId = seedPerspective(ctx.db, bulletId, { status: 'in_review' })

    const res = await apiRequest(ctx.app, 'PATCH', `/perspectives/${perspId}/approve`)
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.status).toBe('approved')
  })

  // ── PATCH /perspectives/:id/reject ────────────────────────────────

  test('PATCH /perspectives/:id/reject with reason returns 200', async () => {
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }])
    const perspId = seedPerspective(ctx.db, bulletId, { status: 'in_review' })

    const res = await apiRequest(ctx.app, 'PATCH', `/perspectives/${perspId}/reject`, {
      rejection_reason: 'Too generic',
    })
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.status).toBe('rejected')
  })

  // ── PATCH /perspectives/:id/reopen ────────────────────────────────

  test('PATCH /perspectives/:id/reopen after reject returns 200 in_review', async () => {
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }])
    const perspId = seedPerspective(ctx.db, bulletId, { status: 'in_review' })

    const rejectRes = await apiRequest(ctx.app, 'PATCH', `/perspectives/${perspId}/reject`, {
      rejection_reason: 'Too generic',
    })
    expect(rejectRes.status).toBe(200)

    const res = await apiRequest(ctx.app, 'PATCH', `/perspectives/${perspId}/reopen`)
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.status).toBe('in_review')
    expect(body.data.rejection_reason).toBeNull()
  })

  test('PATCH /perspectives/:id/reopen from approved returns 400', async () => {
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }])
    const perspId = seedPerspective(ctx.db, bulletId, { status: 'approved' })

    const res = await apiRequest(ctx.app, 'PATCH', `/perspectives/${perspId}/reopen`)
    expect(res.status).toBe(400)
    const body = await res.json()
    expect(body.error.code).toBe('VALIDATION_ERROR')
    expect(body.error.message).toContain('Cannot transition')
  })

  // ── PATCH /perspectives/:id/submit ────────────────────────────────

  test('PATCH /perspectives/:id/submit transitions draft to in_review', async () => {
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }])
    const perspId = seedPerspective(ctx.db, bulletId, { status: 'draft' })

    const res = await apiRequest(ctx.app, 'PATCH', `/perspectives/${perspId}/submit`)
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.id).toBe(perspId)
    expect(body.data.status).toBe('in_review')
  })

  test('PATCH /perspectives/:id/submit on non-draft returns 400', async () => {
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }])
    const perspId = seedPerspective(ctx.db, bulletId, { status: 'rejected' })

    const res = await apiRequest(ctx.app, 'PATCH', `/perspectives/${perspId}/submit`)
    expect(res.status).toBe(400)
    const body = await res.json()
    expect(body.error.code).toBe('VALIDATION_ERROR')
    expect(body.error.message).toContain('Only draft perspectives')
  })

  test('PATCH /perspectives/nonexistent/submit returns 404', async () => {
    const res = await apiRequest(ctx.app, 'PATCH', '/perspectives/nonexistent/submit')
    expect(res.status).toBe(404)
    const body = await res.json()
    expect(body.error.code).toBe('NOT_FOUND')
  })

  // ── PATCH /perspectives/:id/archive, /unarchive ───────────────────

  test('PATCH /perspectives/:id/archive from approved returns 200 archived', async () => {
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }])
    const perspId = seedPerspective(ctx.db, bulletId, { status: 'approved' })

    const res = await apiRequest(ctx.app, 'PATCH', `/perspectives/${perspId}/archive`)
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.id).toBe(perspId)
    expect(body.data.status).toBe('archived')
  })

  test('PATCH /perspectives/:id/unarchive after archive returns 200 draft', async () => {
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }])
    const perspId = seedPerspective(ctx.db, bulletId, { status: 'approved' })

    const archiveRes = await apiRequest(ctx.app, 'PATCH', `/perspectives/${perspId}/archive`)
    expect(archiveRes.status).toBe(200)

    const res = await apiRequest(ctx.app, 'PATCH', `/perspectives/${perspId}/unarchive`)
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.status).toBe('draft')
    expect(body.data.rejection_reason).toBeNull()
  })

  test('PATCH /perspectives/:id/archive from in_review returns 400', async () => {
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }])
    const perspId = seedPerspective(ctx.db, bulletId, { status: 'in_review' })

    const res = await apiRequest(ctx.app, 'PATCH', `/perspectives/${perspId}/archive`)
    expect(res.status).toBe(400)
    const body = await res.json()
    expect(body.error.code).toBe('VALIDATION_ERROR')
    expect(body.error.message).toContain('Cannot transition')
  })

  test('PATCH /perspectives/:id/unarchive from draft returns 400', async () => {
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }])
    const perspId = seedPerspective(ctx.db, bulletId, { status: 'draft' })

    const res = await apiRequest(ctx.app, 'PATCH', `/perspectives/${perspId}/unarchive`)
    expect(res.status).toBe(400)
    const body = await res.json()
    expect(body.error.code).toBe('VALIDATION_ERROR')
    expect(body.error.message).toContain('Cannot transition')
  })

  test('PATCH /perspectives/nonexistent/archive returns 404', async () => {
    const res = await apiRequest(ctx.app, 'PATCH', '/perspectives/nonexistent/archive')
    expect(res.status).toBe(404)
    const body = await res.json()
    expect(body.error.code).toBe('NOT_FOUND')
  })

  // ── DELETE /perspectives/:id ──────────────────────────────────────

  test('DELETE /perspectives/:id returns 204', async () => {
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }])
    const perspId = seedPerspective(ctx.db, bulletId)

    const res = await apiRequest(ctx.app, 'DELETE', `/perspectives/${perspId}`)
    expect(res.status).toBe(204)
  })

  // ── POST /perspectives (direct creation) ──────────────────────────

  test('POST /perspectives creates an auto-approved perspective', async () => {
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }])

    const res = await apiRequest(ctx.app, 'POST', '/perspectives', {
      bullet_id: bulletId,
      content: 'Filler bullet text',
    })
    expect(res.status).toBe(201)
    const body = await res.json()
    expect(body.data.bullet_id).toBe(bulletId)
    expect(body.data.content).toBe('Filler bullet text')
    expect(body.data.status).toBe('approved')
    expect(body.data.approved_by).toBe('direct')
    expect(body.data.framing).toBe('accomplishment')
  })

  test('POST /perspectives with auto_approve=false creates draft', async () => {
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }])

    const res = await apiRequest(ctx.app, 'POST', '/perspectives', {
      bullet_id: bulletId,
      content: 'Draft perspective',
      auto_approve: false,
    })
    expect(res.status).toBe(201)
    const body = await res.json()
    expect(body.data.status).toBe('draft')
  })

  test('POST /perspectives with invalid bullet_id returns 404', async () => {
    const res = await apiRequest(ctx.app, 'POST', '/perspectives', {
      bullet_id: '00000000-0000-0000-0000-000000000000',
      content: 'Should fail',
    })
    expect(res.status).toBe(404)
  })

  test('POST /perspectives without content returns 400', async () => {
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }])

    const res = await apiRequest(ctx.app, 'POST', '/perspectives', {
      bullet_id: bulletId,
      content: '',
    })
    expect(res.status).toBe(400)
  })

  test('DELETE /perspectives/:id in resume returns 409', async () => {
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }])
    const perspId = seedPerspective(ctx.db, bulletId)
    const resumeId = seedResume(ctx.db)
    const secId = seedResumeSection(ctx.db, resumeId, 'Experience', 'experience')
    seedResumeEntry(ctx.db, secId, { perspectiveId: perspId })

    const res = await apiRequest(ctx.app, 'DELETE', `/perspectives/${perspId}`)
    expect(res.status).toBe(409)
    const body = await res.json()
    expect(body.error.code).toBe('CONFLICT')
  })
})
