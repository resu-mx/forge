import { describe, test, expect, beforeEach, afterEach } from 'bun:test'
import type { Database } from 'bun:sqlite'
import { PerspectiveService } from '../perspective-service'
import { createTestDb, seedSource, seedBullet, seedPerspective, seedResume, seedResumeEntry, seedResumeSection } from '../../db/__tests__/helpers'
import { buildDefaultElm } from '../../storage/build-elm'

describe('PerspectiveService', () => {
  let db: Database
  let service: PerspectiveService

  beforeEach(() => {
    db = createTestDb()
    service = new PerspectiveService(buildDefaultElm(db))
  })

  afterEach(() => db.close())

  // ── getPerspective ────────────────────────────────────────────────

  test('getPerspective returns existing perspective', async () => {
    const srcId = seedSource(db)
    const bulletId = seedBullet(db, [{ id: srcId }])
    const perspId = seedPerspective(db, bulletId)

    const result = await service.getPerspective(perspId)
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data.id).toBe(perspId)
  })

  test('getPerspective returns NOT_FOUND for missing ID', async () => {
    const result = await service.getPerspective('nonexistent')
    expect(result.ok).toBe(false)
    if (result.ok) return
    expect(result.error.code).toBe('NOT_FOUND')
  })

  // ── getPerspectiveWithChain ───────────────────────────────────────

  test('getPerspectiveWithChain returns full chain', async () => {
    const srcId = seedSource(db)
    const bulletId = seedBullet(db, [{ id: srcId }])
    const perspId = seedPerspective(db, bulletId)

    const result = await service.getPerspectiveWithChain(perspId)
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data.id).toBe(perspId)
    expect(result.data.bullet.id).toBe(bulletId)
    expect(result.data.source.id).toBe(srcId)
  })

  test('getPerspectiveWithChain returns NOT_FOUND for missing ID', async () => {
    const result = await service.getPerspectiveWithChain('nonexistent')
    expect(result.ok).toBe(false)
    if (result.ok) return
    expect(result.error.code).toBe('NOT_FOUND')
  })

  // ── listPerspectives ──────────────────────────────────────────────

  test('listPerspectives returns all perspectives', async () => {
    const srcId = seedSource(db)
    const bulletId = seedBullet(db, [{ id: srcId }])
    seedPerspective(db, bulletId)
    seedPerspective(db, bulletId, { domain: 'security' })

    const result = await service.listPerspectives()
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data.length).toBe(2)
  })

  test('listPerspectives filters by bullet_id', async () => {
    const srcId = seedSource(db)
    const b1 = seedBullet(db, [{ id: srcId }])
    const b2 = seedBullet(db, [{ id: srcId }], { content: 'Other bullet' })
    seedPerspective(db, b1)
    seedPerspective(db, b2)

    const result = await service.listPerspectives({ bullet_id: b1 })
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data.length).toBe(1)
  })

  test('listPerspectives filters by archetype and status', async () => {
    const srcId = seedSource(db)
    const bulletId = seedBullet(db, [{ id: srcId }])
    seedPerspective(db, bulletId, { archetype: 'agentic-ai', status: 'approved' })
    seedPerspective(db, bulletId, { archetype: 'infrastructure', status: 'approved' })
    seedPerspective(db, bulletId, { archetype: 'agentic-ai', status: 'draft' })

    const result = await service.listPerspectives({ target_archetype: 'agentic-ai', status: 'approved' })
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data.length).toBe(1)
  })

  // ── updatePerspective ─────────────────────────────────────────────

  test('updatePerspective with valid content succeeds', async () => {
    const srcId = seedSource(db)
    const bulletId = seedBullet(db, [{ id: srcId }])
    const perspId = seedPerspective(db, bulletId)

    const result = await service.updatePerspective(perspId, { content: 'Updated perspective' })
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data.content).toBe('Updated perspective')
  })

  test('updatePerspective rejects empty content', async () => {
    const srcId = seedSource(db)
    const bulletId = seedBullet(db, [{ id: srcId }])
    const perspId = seedPerspective(db, bulletId)

    const result = await service.updatePerspective(perspId, { content: '' })
    expect(result.ok).toBe(false)
    if (result.ok) return
    expect(result.error.code).toBe('VALIDATION_ERROR')
  })

  test('updatePerspective returns NOT_FOUND for missing ID', async () => {
    const result = await service.updatePerspective('nonexistent', { content: 'New' })
    expect(result.ok).toBe(false)
    if (result.ok) return
    expect(result.error.code).toBe('NOT_FOUND')
  })

  // ── deletePerspective ─────────────────────────────────────────────

  test('deletePerspective removes perspective', async () => {
    const srcId = seedSource(db)
    const bulletId = seedBullet(db, [{ id: srcId }])
    const perspId = seedPerspective(db, bulletId)

    const result = await service.deletePerspective(perspId)
    expect(result.ok).toBe(true)

    const check = await service.getPerspective(perspId)
    expect(check.ok).toBe(false)
  })

  test('deletePerspective returns NOT_FOUND for missing ID', async () => {
    const result = await service.deletePerspective('nonexistent')
    expect(result.ok).toBe(false)
    if (result.ok) return
    expect(result.error.code).toBe('NOT_FOUND')
  })

  test('deletePerspective returns CONFLICT when in a resume', async () => {
    const srcId = seedSource(db)
    const bulletId = seedBullet(db, [{ id: srcId }])
    const perspId = seedPerspective(db, bulletId)
    const resumeId = seedResume(db)
    const secId = seedResumeSection(db, resumeId, 'Experience', 'experience')
    seedResumeEntry(db, secId, { perspectiveId: perspId })

    const result = await service.deletePerspective(perspId)
    expect(result.ok).toBe(false)
    if (result.ok) return
    expect(result.error.code).toBe('CONFLICT')
  })

  // ── Status transitions ────────────────────────────────────────────

  test('approvePerspective transitions from in_review to approved', async () => {
    const srcId = seedSource(db)
    const bulletId = seedBullet(db, [{ id: srcId }])
    const perspId = seedPerspective(db, bulletId, { status: 'in_review' })

    const result = await service.approvePerspective(perspId)
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data.status).toBe('approved')
    expect(result.data.approved_at).not.toBeNull()
    expect(result.data.approved_by).toBe('human')
  })

  test('approvePerspective rejects transition from draft', async () => {
    const srcId = seedSource(db)
    const bulletId = seedBullet(db, [{ id: srcId }])
    const perspId = seedPerspective(db, bulletId, { status: 'draft' })

    const result = await service.approvePerspective(perspId)
    expect(result.ok).toBe(false)
    if (result.ok) return
    expect(result.error.code).toBe('VALIDATION_ERROR')
  })

  test('rejectPerspective transitions from in_review to rejected', async () => {
    const srcId = seedSource(db)
    const bulletId = seedBullet(db, [{ id: srcId }])
    const perspId = seedPerspective(db, bulletId, { status: 'in_review' })

    const result = await service.rejectPerspective(perspId, 'Does not match domain')
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data.status).toBe('rejected')
    expect(result.data.rejection_reason).toBe('Does not match domain')
  })

  test('rejectPerspective requires non-empty reason', async () => {
    const srcId = seedSource(db)
    const bulletId = seedBullet(db, [{ id: srcId }])
    const perspId = seedPerspective(db, bulletId, { status: 'in_review' })

    const result = await service.rejectPerspective(perspId, '')
    expect(result.ok).toBe(false)
    if (result.ok) return
    expect(result.error.code).toBe('VALIDATION_ERROR')
  })

  test('reopenPerspective transitions from rejected to in_review', async () => {
    const srcId = seedSource(db)
    const bulletId = seedBullet(db, [{ id: srcId }])
    const perspId = seedPerspective(db, bulletId, { status: 'in_review' })

    // First reject
    await service.rejectPerspective(perspId, 'reason')

    // Then reopen
    const result = await service.reopenPerspective(perspId)
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data.status).toBe('in_review')
  })

  // ── submitPerspective ─────────────────────────────────────────────

  test('submitPerspective transitions draft to in_review', async () => {
    const srcId = seedSource(db)
    const bulletId = seedBullet(db, [{ id: srcId }])
    const perspId = seedPerspective(db, bulletId, { status: 'draft' })

    const result = await service.submitPerspective(perspId)
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data.status).toBe('in_review')
  })

  test('submitPerspective rejects non-draft perspective', async () => {
    for (const status of ['in_review', 'approved', 'rejected', 'archived']) {
      const srcId = seedSource(db)
      const bulletId = seedBullet(db, [{ id: srcId }])
      const perspId = seedPerspective(db, bulletId, { status })

      const result = await service.submitPerspective(perspId)
      expect(result.ok).toBe(false)
      if (result.ok) return
      expect(result.error.code).toBe('VALIDATION_ERROR')
      expect(result.error.message).toContain('Only draft perspectives')
    }
  })

  test('submitPerspective returns NOT_FOUND for missing ID', async () => {
    const result = await service.submitPerspective('nonexistent')
    expect(result.ok).toBe(false)
    if (result.ok) return
    expect(result.error.code).toBe('NOT_FOUND')
  })

  // ── archivePerspective / unarchivePerspective ─────────────────────

  test('archivePerspective then unarchivePerspective returns an approved perspective to draft', async () => {
    const srcId = seedSource(db)
    const bulletId = seedBullet(db, [{ id: srcId }])
    const perspId = seedPerspective(db, bulletId, { status: 'approved' })

    const archived = await service.archivePerspective(perspId)
    expect(archived.ok).toBe(true)
    if (!archived.ok) return
    expect(archived.data.status).toBe('archived')

    const unarchived = await service.unarchivePerspective(perspId)
    expect(unarchived.ok).toBe(true)
    if (!unarchived.ok) return
    expect(unarchived.data.status).toBe('draft')
  })

  test('archivePerspective rejects a non-approved perspective', async () => {
    const srcId = seedSource(db)
    const bulletId = seedBullet(db, [{ id: srcId }])
    const perspId = seedPerspective(db, bulletId, { status: 'draft' })

    const result = await service.archivePerspective(perspId)
    expect(result.ok).toBe(false)
    if (result.ok) return
    expect(result.error.code).toBe('VALIDATION_ERROR')
  })

  test('unarchivePerspective rejects a non-archived perspective', async () => {
    const srcId = seedSource(db)
    const bulletId = seedBullet(db, [{ id: srcId }])
    const perspId = seedPerspective(db, bulletId, { status: 'approved' })

    const result = await service.unarchivePerspective(perspId)
    expect(result.ok).toBe(false)
    if (result.ok) return
    expect(result.error.code).toBe('VALIDATION_ERROR')
  })

  test('approvePerspective returns NOT_FOUND for missing ID', async () => {
    const result = await service.approvePerspective('nonexistent')
    expect(result.ok).toBe(false)
    if (result.ok) return
    expect(result.error.code).toBe('NOT_FOUND')
  })
})
