/**
 * Route-level tests for /api/domains.
 */

import { describe, test, expect, beforeEach, afterEach } from 'bun:test'
import { createTestApp, apiRequest, type TestContext } from './helpers'
import { seedDomain, seedArchetype, seedArchetypeDomain, seedSource, seedBullet, seedPerspective } from '../../db/__tests__/helpers'

describe('Domain routes', () => {
  let ctx: TestContext

  beforeEach(() => {
    ctx = createTestApp()
  })

  afterEach(() => {
    ctx.db.close()
  })

  // -- POST /domains --------------------------------------------------------

  test('POST /domains with valid body returns 201', async () => {
    const res = await apiRequest(ctx.app, 'POST', '/domains', {
      name: 'new_domain',
      description: 'A test domain',
    })
    expect(res.status).toBe(201)
    const body = await res.json()
    expect(body.data.name).toBe('new_domain')
    expect(body.data.description).toBe('A test domain')
    expect(body.data.id).toHaveLength(36)
  })

  test('POST /domains with empty name returns 400', async () => {
    const res = await apiRequest(ctx.app, 'POST', '/domains', { name: '' })
    expect(res.status).toBe(400)
    const body = await res.json()
    expect(body.error.code).toBe('VALIDATION_ERROR')
  })

  test('POST /domains with invalid format returns 400', async () => {
    const res = await apiRequest(ctx.app, 'POST', '/domains', { name: 'Bad Name' })
    expect(res.status).toBe(400)
  })

  test('POST /domains with duplicate name returns 409', async () => {
    await apiRequest(ctx.app, 'POST', '/domains', { name: 'dup_test' })
    const res = await apiRequest(ctx.app, 'POST', '/domains', { name: 'dup_test' })
    expect(res.status).toBe(409)
    const body = await res.json()
    expect(body.error.code).toBe('CONFLICT')
  })

  // -- GET /domains ---------------------------------------------------------

  test('GET /domains returns array with pagination', async () => {
    const res = await apiRequest(ctx.app, 'GET', '/domains')
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data).toBeArray()
    expect(body.pagination).toBeDefined()
    expect(typeof body.pagination.total).toBe('number')
  })

  test('GET /domains items include usage counts', async () => {
    const res = await apiRequest(ctx.app, 'GET', '/domains')
    const body = await res.json()
    if (body.data.length > 0) {
      expect(typeof body.data[0].perspective_count).toBe('number')
      expect(typeof body.data[0].archetype_count).toBe('number')
    }
  })

  // -- GET /domains/:id -----------------------------------------------------

  test('GET /domains/:id returns domain', async () => {
    const domainId = seedDomain(ctx.db, { name: 'get_test' })
    const res = await apiRequest(ctx.app, 'GET', `/domains/${domainId}`)
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.name).toBe('get_test')
  })

  test('GET /domains/:id with invalid id returns 404', async () => {
    const res = await apiRequest(ctx.app, 'GET', '/domains/nonexistent')
    expect(res.status).toBe(404)
  })

  // -- PATCH /domains/:id ---------------------------------------------------

  test('PATCH /domains/:id updates domain', async () => {
    const domainId = seedDomain(ctx.db, { name: 'patch_test' })
    const res = await apiRequest(ctx.app, 'PATCH', `/domains/${domainId}`, {
      description: 'Updated desc',
    })
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.description).toBe('Updated desc')
  })

  test('PATCH /domains/:id renames and clears the description', async () => {
    const domainId = seedDomain(ctx.db, { name: 'patch_rename', description: 'Before' })
    const res = await apiRequest(ctx.app, 'PATCH', `/domains/${domainId}`, {
      name: 'patch_renamed',
      description: null,
    })
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.id).toBe(domainId)
    expect(body.data.name).toBe('patch_renamed')
    expect(body.data.description).toBeNull()
  })

  test('PATCH /domains/:id with its own name returns 200', async () => {
    const domainId = seedDomain(ctx.db, { name: 'patch_same' })
    const res = await apiRequest(ctx.app, 'PATCH', `/domains/${domainId}`, {
      name: 'patch_same',
      description: 'Edited',
    })
    expect(res.status).toBe(200)
    expect((await res.json()).data.description).toBe('Edited')
  })

  test('PATCH /domains/:id with empty name returns 400', async () => {
    const domainId = seedDomain(ctx.db, { name: 'patch_blank' })
    const res = await apiRequest(ctx.app, 'PATCH', `/domains/${domainId}`, { name: '   ' })
    expect(res.status).toBe(400)
    const body = await res.json()
    expect(body.error.code).toBe('VALIDATION_ERROR')
    expect(body.error.message).toContain('Name must not be empty')
  })

  test('PATCH /domains/:id with invalid format returns 400 and changes nothing', async () => {
    const domainId = seedDomain(ctx.db, { name: 'patch_format' })
    const res = await apiRequest(ctx.app, 'PATCH', `/domains/${domainId}`, { name: 'Bad Name' })
    expect(res.status).toBe(400)
    expect((await res.json()).error.code).toBe('VALIDATION_ERROR')
    const get = await apiRequest(ctx.app, 'GET', `/domains/${domainId}`)
    expect((await get.json()).data.name).toBe('patch_format')
  })

  test('PATCH /domains/:id with unknown id returns 404', async () => {
    const res = await apiRequest(ctx.app, 'PATCH', '/domains/nonexistent', { description: 'x' })
    expect(res.status).toBe(404)
  })

  test('PATCH /domains/:id to a taken name returns 409', async () => {
    seedDomain(ctx.db, { name: 'patch_taken' })
    const domainId = seedDomain(ctx.db, { name: 'patch_other' })
    const res = await apiRequest(ctx.app, 'PATCH', `/domains/${domainId}`, { name: 'patch_taken' })
    expect(res.status).toBe(409)
    expect((await res.json()).error.code).toBe('CONFLICT')
  })

  // -- DELETE /domains/:id --------------------------------------------------

  test('DELETE /domains/:id with no references returns 204', async () => {
    const domainId = seedDomain(ctx.db, { name: 'delete_test' })
    const res = await apiRequest(ctx.app, 'DELETE', `/domains/${domainId}`)
    expect(res.status).toBe(204)
  })

  test('DELETE /domains/:id with perspective references returns 409', async () => {
    const domainId = seedDomain(ctx.db, { name: 'block_delete' })
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }])
    seedPerspective(ctx.db, bulletId, { domain: 'block_delete' })

    const res = await apiRequest(ctx.app, 'DELETE', `/domains/${domainId}`)
    expect(res.status).toBe(409)
  })

  test('DELETE /domains/:id with archetype references returns 409', async () => {
    const domainId = seedDomain(ctx.db, { name: 'arch_block' })
    const archId = seedArchetype(ctx.db, { name: 'blocker' })
    seedArchetypeDomain(ctx.db, archId, domainId)

    const res = await apiRequest(ctx.app, 'DELETE', `/domains/${domainId}`)
    expect(res.status).toBe(409)
  })
})
