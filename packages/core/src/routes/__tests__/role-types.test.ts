/**
 * Route-level tests for PATCH /api/role-types/:id (resu-mx/forge#19).
 *
 * This is the contract the Rust server is held to in `just parity-tier0`. Setup goes through
 * POST and reads back through GET, so both runtimes run the same calls. (List pagination is
 * tracked in #141 and is not asserted here.)
 */

import { describe, test, expect, beforeEach, afterEach } from 'bun:test'
import { createTestApp, apiRequest, type TestContext } from './helpers'

describe('Role type routes: PATCH /role-types/:id', () => {
  let ctx: TestContext

  beforeEach(() => {
    ctx = createTestApp()
  })

  afterEach(() => {
    ctx.db.close()
  })

  async function create(name: string, description?: string): Promise<string> {
    const res = await apiRequest(ctx.app, 'POST', '/role-types', { name, description })
    expect(res.status).toBe(201)
    return (await res.json()).data.id
  }

  async function read(id: string) {
    const res = await apiRequest(ctx.app, 'GET', `/role-types/${id}`)
    expect(res.status).toBe(200)
    return (await res.json()).data
  }

  test('renames with a trimmed name, clears the description, and stores it', async () => {
    const id = await create('TL', 'Before')
    const res = await apiRequest(ctx.app, 'PATCH', `/role-types/${id}`, {
      name: '  Tech Lead  ',
      description: null,
    })
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.id).toBe(id)
    expect(body.data.name).toBe('Tech Lead')
    expect(body.data.description).toBeNull()

    const stored = await read(id)
    expect(stored.name).toBe('Tech Lead')
    expect(stored.description).toBeNull()
  })

  test('sets only the description and leaves the name', async () => {
    const id = await create('Individual Contributor')
    const res = await apiRequest(ctx.app, 'PATCH', `/role-types/${id}`, {
      description: 'Builds things',
    })
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.name).toBe('Individual Contributor')
    expect(body.data.description).toBe('Builds things')
  })

  test('an empty body changes nothing and returns the row', async () => {
    const id = await create('Architect', 'Kept')
    const res = await apiRequest(ctx.app, 'PATCH', `/role-types/${id}`, {})
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.name).toBe('Architect')
    expect(body.data.description).toBe('Kept')
  })

  test('renaming to its own name returns 200', async () => {
    const id = await create('Manager')
    const res = await apiRequest(ctx.app, 'PATCH', `/role-types/${id}`, {
      name: 'Manager',
      description: 'Edited',
    })
    expect(res.status).toBe(200)
    expect((await res.json()).data.description).toBe('Edited')
  })

  test('a blank name returns 400 and changes nothing', async () => {
    const id = await create('Director', 'People')
    const res = await apiRequest(ctx.app, 'PATCH', `/role-types/${id}`, {
      name: '   ',
      description: 'changed',
    })
    expect(res.status).toBe(400)
    const body = await res.json()
    expect(body.error.code).toBe('VALIDATION_ERROR')
    expect(body.error.message).toContain('Name must not be empty')

    const stored = await read(id)
    expect(stored.name).toBe('Director')
    expect(stored.description).toBe('People')
  })

  test('an unknown id returns 404', async () => {
    const res = await apiRequest(ctx.app, 'PATCH', '/role-types/nonexistent', {
      description: 'x',
    })
    expect(res.status).toBe(404)
    expect((await res.json()).error.code).toBe('NOT_FOUND')
  })

  test('a name another role type has returns 409 and changes nothing', async () => {
    await create('Principal')
    const id = await create('Staff')
    const res = await apiRequest(ctx.app, 'PATCH', `/role-types/${id}`, { name: ' Principal ' })
    expect(res.status).toBe(409)
    expect((await res.json()).error.code).toBe('CONFLICT')
    expect((await read(id)).name).toBe('Staff')
  })
})
