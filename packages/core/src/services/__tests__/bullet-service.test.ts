import { describe, test, expect, beforeEach, afterEach } from 'bun:test'
import type { Database } from 'bun:sqlite'
import { BulletService } from '../bullet-service'
import { createTestDb, seedSource, seedBullet, seedPerspective } from '../../db/__tests__/helpers'
import { buildDefaultElm } from '../../storage/build-elm'

describe('BulletService', () => {
  let db: Database
  let service: BulletService

  beforeEach(() => {
    db = createTestDb()
    service = new BulletService(buildDefaultElm(db))
  })

  afterEach(() => db.close())

  // ── getBullet ─────────────────────────────────────────────────────

  test('getBullet returns existing bullet', async () => {
    const srcId = seedSource(db)
    const bulletId = seedBullet(db, [{ id: srcId }])

    const result = await service.getBullet(bulletId)
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data.id).toBe(bulletId)
  })

  test('getBullet returns NOT_FOUND for missing ID', async () => {
    const result = await service.getBullet('nonexistent-id')
    expect(result.ok).toBe(false)
    if (result.ok) return
    expect(result.error.code).toBe('NOT_FOUND')
  })

  // ── listBullets ───────────────────────────────────────────────────

  test('listBullets returns all bullets', async () => {
    const srcId = seedSource(db)
    seedBullet(db, [{ id: srcId }])
    seedBullet(db, [{ id: srcId }], { content: 'Second bullet' })

    const result = await service.listBullets()
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data.length).toBe(2)
  })

  test('listBullets filters by source_id via junction', async () => {
    const src1 = seedSource(db)
    const src2 = seedSource(db, { title: 'Source 2' })
    seedBullet(db, [{ id: src1 }])
    seedBullet(db, [{ id: src2 }])

    const result = await service.listBullets({ source_id: src1 })
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data.length).toBe(1)
  })

  test('listBullets filters by status', async () => {
    const srcId = seedSource(db)
    seedBullet(db, [{ id: srcId }], { status: 'approved' })
    seedBullet(db, [{ id: srcId }], { status: 'draft' })

    const result = await service.listBullets({ status: 'approved' })
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data.length).toBe(1)
  })

  test('listBullets supports pagination', async () => {
    const srcId = seedSource(db)
    for (let i = 0; i < 5; i++) {
      seedBullet(db, [{ id: srcId }], { content: `Bullet ${i}` })
    }

    const result = await service.listBullets({}, 0, 2)
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data.length).toBe(2)
    expect(result.pagination.total).toBe(5)
  })

  // ── updateBullet ──────────────────────────────────────────────────

  test('updateBullet with valid content succeeds', async () => {
    const srcId = seedSource(db)
    const bulletId = seedBullet(db, [{ id: srcId }])

    const result = await service.updateBullet(bulletId, { content: 'Updated content' })
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data.content).toBe('Updated content')
  })

  test('updateBullet rejects empty content', async () => {
    const srcId = seedSource(db)
    const bulletId = seedBullet(db, [{ id: srcId }])

    const result = await service.updateBullet(bulletId, { content: '' })
    expect(result.ok).toBe(false)
    if (result.ok) return
    expect(result.error.code).toBe('VALIDATION_ERROR')
  })

  test('updateBullet returns NOT_FOUND for missing ID', async () => {
    const result = await service.updateBullet('nonexistent', { content: 'New' })
    expect(result.ok).toBe(false)
    if (result.ok) return
    expect(result.error.code).toBe('NOT_FOUND')
  })

  // ── deleteBullet ──────────────────────────────────────────────────

  test('deleteBullet removes bullet', async () => {
    const srcId = seedSource(db)
    const bulletId = seedBullet(db, [{ id: srcId }])

    const result = await service.deleteBullet(bulletId)
    expect(result.ok).toBe(true)

    const check = await service.getBullet(bulletId)
    expect(check.ok).toBe(false)
  })

  test('deleteBullet returns NOT_FOUND for missing ID', async () => {
    const result = await service.deleteBullet('nonexistent')
    expect(result.ok).toBe(false)
    if (result.ok) return
    expect(result.error.code).toBe('NOT_FOUND')
  })

  test('deleteBullet returns CONFLICT when bullet has perspectives', async () => {
    const srcId = seedSource(db)
    const bulletId = seedBullet(db, [{ id: srcId }])
    seedPerspective(db, bulletId)

    const result = await service.deleteBullet(bulletId)
    expect(result.ok).toBe(false)
    if (result.ok) return
    expect(result.error.code).toBe('CONFLICT')
  })

  // ── Status transitions ────────────────────────────────────────────

  test('approveBullet transitions from in_review to approved', async () => {
    const srcId = seedSource(db)
    const bulletId = seedBullet(db, [{ id: srcId }], { status: 'in_review' })

    const result = await service.approveBullet(bulletId)
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data.status).toBe('approved')
    expect(result.data.approved_at).not.toBeNull()
    expect(result.data.approved_by).toBe('human')
  })

  test('approveBullet rejects transition from draft', async () => {
    const srcId = seedSource(db)
    const bulletId = seedBullet(db, [{ id: srcId }], { status: 'draft' })

    const result = await service.approveBullet(bulletId)
    expect(result.ok).toBe(false)
    if (result.ok) return
    expect(result.error.code).toBe('VALIDATION_ERROR')
    expect(result.error.message).toContain('draft')
  })

  test('rejectBullet transitions from in_review to rejected', async () => {
    const srcId = seedSource(db)
    const bulletId = seedBullet(db, [{ id: srcId }], { status: 'in_review' })

    const result = await service.rejectBullet(bulletId, 'Too vague')
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data.status).toBe('rejected')
    expect(result.data.rejection_reason).toBe('Too vague')
  })

  test('rejectBullet requires non-empty reason', async () => {
    const srcId = seedSource(db)
    const bulletId = seedBullet(db, [{ id: srcId }], { status: 'in_review' })

    const result = await service.rejectBullet(bulletId, '')
    expect(result.ok).toBe(false)
    if (result.ok) return
    expect(result.error.code).toBe('VALIDATION_ERROR')
  })

  test('reopenBullet transitions from rejected to in_review', async () => {
    const srcId = seedSource(db)
    const bulletId = seedBullet(db, [{ id: srcId }], { status: 'in_review' })

    // First reject
    await service.rejectBullet(bulletId, 'reason')

    // Then reopen
    const result = await service.reopenBullet(bulletId)
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data.status).toBe('in_review')
  })

  test('approveBullet returns NOT_FOUND for missing ID', async () => {
    const result = await service.approveBullet('nonexistent')
    expect(result.ok).toBe(false)
    if (result.ok) return
    expect(result.error.code).toBe('NOT_FOUND')
  })

  // ── submitBullet ───────────────────────────────────────────────────

  test('submitBullet transitions draft to in_review', async () => {
    const srcId = seedSource(db)
    const bulletId = seedBullet(db, [{ id: srcId }], { status: 'draft' })

    const result = await service.submitBullet(bulletId)
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data.status).toBe('in_review')
  })

  test('submitBullet rejects non-draft bullet (in_review)', async () => {
    const srcId = seedSource(db)
    const bulletId = seedBullet(db, [{ id: srcId }], { status: 'in_review' })

    const result = await service.submitBullet(bulletId)
    expect(result.ok).toBe(false)
    if (result.ok) return
    expect(result.error.code).toBe('VALIDATION_ERROR')
    expect(result.error.message).toContain('Only draft bullets')
  })

  test('submitBullet rejects approved bullet', async () => {
    const srcId = seedSource(db)
    const bulletId = seedBullet(db, [{ id: srcId }], { status: 'approved' })

    const result = await service.submitBullet(bulletId)
    expect(result.ok).toBe(false)
    if (result.ok) return
    expect(result.error.code).toBe('VALIDATION_ERROR')
  })

  test('submitBullet rejects rejected bullet', async () => {
    const srcId = seedSource(db)
    const bulletId = seedBullet(db, [{ id: srcId }], { status: 'rejected' })

    const result = await service.submitBullet(bulletId)
    expect(result.ok).toBe(false)
    if (result.ok) return
    expect(result.error.code).toBe('VALIDATION_ERROR')
  })

  test('submitBullet returns NOT_FOUND for missing ID', async () => {
    const result = await service.submitBullet('nonexistent')
    expect(result.ok).toBe(false)
    if (result.ok) return
    expect(result.error.code).toBe('NOT_FOUND')
  })

  // ── archiveBullet / unarchiveBullet ────────────────────────────────

  test('archiveBullet then unarchiveBullet returns an approved bullet to draft', async () => {
    const srcId = seedSource(db)
    const bulletId = seedBullet(db, [{ id: srcId }], { status: 'approved' })

    const archived = await service.archiveBullet(bulletId)
    expect(archived.ok).toBe(true)
    if (!archived.ok) return
    expect(archived.data.status).toBe('archived')

    const unarchived = await service.unarchiveBullet(bulletId)
    expect(unarchived.ok).toBe(true)
    if (!unarchived.ok) return
    expect(unarchived.data.status).toBe('draft')
  })

  test('archiveBullet rejects a non-approved bullet', async () => {
    const srcId = seedSource(db)
    const bulletId = seedBullet(db, [{ id: srcId }], { status: 'in_review' })

    const result = await service.archiveBullet(bulletId)
    expect(result.ok).toBe(false)
    if (result.ok) return
    expect(result.error.code).toBe('VALIDATION_ERROR')
  })

  test('unarchiveBullet rejects a non-archived bullet', async () => {
    const srcId = seedSource(db)
    const bulletId = seedBullet(db, [{ id: srcId }], { status: 'approved' })

    const result = await service.unarchiveBullet(bulletId)
    expect(result.ok).toBe(false)
    if (result.ok) return
    expect(result.error.code).toBe('VALIDATION_ERROR')
  })
})
