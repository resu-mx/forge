/**
 * Organization routes: the partial PATCH contract (resu-mx/forge#32).
 *
 * Runs in-process against Hono, and against the Rust forge-server under `just parity`.
 */

import { describe, test, expect, beforeEach, afterEach } from 'bun:test'
import { createTestApp, apiRequest, type TestContext } from './helpers'

describe('PATCH /organizations/:id', () => {
  let ctx: TestContext

  beforeEach(() => {
    ctx = createTestApp()
  })

  afterEach(() => {
    ctx.db.close()
  })

  async function seed(body: Record<string, unknown> = {}) {
    const res = await apiRequest(ctx.app, 'POST', '/organizations', {
      name: 'Acme',
      org_type: 'company',
      website: 'https://acme.test',
      status: 'backlog',
      ...body,
    })
    expect(res.status).toBe(201)
    return (await res.json()).data as { id: string }
  }

  async function patch(id: string, body: unknown) {
    const res = await apiRequest(ctx.app, 'PATCH', `/organizations/${id}`, body)
    return { status: res.status, body: await res.json() }
  }

  test('a body with only status changes the status and nothing else', async () => {
    const org = await seed()
    const { status, body } = await patch(org.id, { status: 'researching' })
    expect(status).toBe(200)
    expect(body.data).toMatchObject({
      name: 'Acme',
      org_type: 'company',
      website: 'https://acme.test',
      status: 'researching',
    })
  })

  test('status: null clears the status', async () => {
    const org = await seed()
    const { status, body } = await patch(org.id, { status: null })
    expect(status).toBe(200)
    expect(body.data.status).toBeNull()
    const got = await apiRequest(ctx.app, 'GET', `/organizations/${org.id}`)
    expect((await got.json()).data.status).toBeNull()
  })

  test('null clears a nullable field and leaves the rest', async () => {
    const org = await seed()
    const { status, body } = await patch(org.id, { website: null })
    expect(status).toBe(200)
    expect(body.data.website).toBeNull()
    expect(body.data.status).toBe('backlog')
  })

  test('tags replace the whole list', async () => {
    const org = await seed()
    const { status, body } = await patch(org.id, { tags: ['vendor', 'platform'] })
    expect(status).toBe(200)
    expect([...body.data.tags].sort()).toEqual(['platform', 'vendor'])
  })

  for (const [name, bad] of [
    ['a blank name', { name: '   ' }],
    ['an unknown org_type', { org_type: 'guild' }],
    ['an unknown status', { status: 'bogus' }],
  ] as const) {
    test(`${name} is a 400 VALIDATION_ERROR`, async () => {
      const org = await seed()
      const { status, body } = await patch(org.id, bad)
      expect(status).toBe(400)
      expect(body.error.code).toBe('VALIDATION_ERROR')
    })
  }

  test('an unknown id is a 404 NOT_FOUND', async () => {
    const { status, body } = await patch(crypto.randomUUID(), { status: 'backlog' })
    expect(status).toBe(404)
    expect(body.error.code).toBe('NOT_FOUND')
  })
})
