/**
 * Route-level tests for PATCH /api/role-types/:id (resu-mx/forge#19) and the paginated
 * GET /api/role-types list (resu-mx/forge#141).
 *
 * This is the contract the Rust server is held to in `just parity-tier0`. Setup goes through
 * POST and reads back through GET, so both runtimes run the same calls.
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
    expect(body.error.message).toBe('Name must not be empty')

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

describe('Role type routes: GET /role-types pagination', () => {
  let ctx: TestContext

  beforeEach(() => {
    ctx = createTestApp()
  })

  afterEach(() => {
    ctx.db.close()
  })

  async function create(name: string): Promise<void> {
    const res = await apiRequest(ctx.app, 'POST', '/role-types', { name })
    expect(res.status).toBe(201)
  }

  // Zero-padded, so name order is numeric order. Created out of order, so the order the page
  // comes back in is the sort, not insertion.
  const nameOf = (i: number) => `Role ${String(i).padStart(3, '0')}`

  async function seed(count: number): Promise<void> {
    for (let i = count - 1; i >= 0; i--) await create(nameOf(i))
  }

  async function list(query = '') {
    const res = await apiRequest(ctx.app, 'GET', `/role-types${query}`)
    expect(res.status).toBe(200)
    return res.json()
  }

  test('an empty list has the data and pagination shape', async () => {
    const body = await list()
    expect(body.data).toEqual([])
    expect(body.pagination).toEqual({ total: 0, offset: 0, limit: 50 })
  })

  test('returns the default page of 50, ordered by name, with the full total', async () => {
    await seed(60)
    const body = await list()
    expect(body.data).toHaveLength(50)
    expect(body.data.map((r: { name: string }) => r.name)).toEqual(
      Array.from({ length: 50 }, (_, i) => nameOf(i)),
    )
    expect(body.pagination).toEqual({ total: 60, offset: 0, limit: 50 })
  })

  test('offset and limit select a window and are echoed back', async () => {
    await seed(10)
    const body = await list('?offset=3&limit=4')
    expect(body.data.map((r: { name: string }) => r.name)).toEqual([3, 4, 5, 6].map(nameOf))
    expect(body.pagination).toEqual({ total: 10, offset: 3, limit: 4 })
  })

  test('an offset past the end returns no rows and the full total', async () => {
    await seed(3)
    const body = await list('?offset=10')
    expect(body.data).toEqual([])
    expect(body.pagination).toEqual({ total: 3, offset: 10, limit: 50 })
  })

  test('a negative offset is clamped to 0', async () => {
    await seed(3)
    const body = await list('?offset=-3')
    expect(body.data).toHaveLength(3)
    expect(body.pagination).toEqual({ total: 3, offset: 0, limit: 50 })
  })

  test('limit=500 is clamped to 200, reports the full total, and pages to it', async () => {
    await seed(250)

    const first = await list('?limit=500')
    expect(first.data).toHaveLength(200)
    expect(first.pagination).toEqual({ total: 250, offset: 0, limit: 200 })

    const second = await list('?limit=200&offset=200')
    expect(second.data).toHaveLength(50)
    expect(second.pagination).toEqual({ total: 250, offset: 200, limit: 200 })

    const names = [...first.data, ...second.data].map((r: { name: string }) => r.name)
    expect(new Set(names).size).toBe(250)
    expect(names).toEqual(Array.from({ length: 250 }, (_, i) => nameOf(i)))
  })
})
