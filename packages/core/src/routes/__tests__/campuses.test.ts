/**
 * Tests for org location routes (formerly campus routes).
 *
 * Tests both /locations (new) and /campuses (backward-compat) paths.
 */

import { describe, test, expect, beforeEach, afterEach } from 'bun:test'
import type { Database } from 'bun:sqlite'
import { createTestApp, apiRequest, type TestContext } from './helpers'
import type { Organization } from '../../types'

function seedOrg(db: Database, input: { name: string }): Organization {
  const id = crypto.randomUUID()
  return db
    .query('INSERT INTO organizations (id, name) VALUES (?, ?) RETURNING *')
    .get(id, input.name) as Organization
}

function seedAddress(db: Database, input: { name: string; city?: string; state?: string }) {
  const id = crypto.randomUUID()
  return db
    .query('INSERT INTO addresses (id, name, city, state) VALUES (?, ?, ?, ?) RETURNING *')
    .get(id, input.name, input.city ?? null, input.state ?? null) as { id: string; name: string; city: string | null; state: string | null }
}

describe('Org location routes', () => {
  let ctx: TestContext

  beforeEach(() => {
    ctx = createTestApp()
  })

  afterEach(() => {
    ctx.db.close()
  })

  // -- POST + GET /organizations/:orgId/locations --

  test('POST /organizations/:orgId/locations creates a location', async () => {
    const org = seedOrg(ctx.db, { name: 'Test Org' })

    const res = await apiRequest(ctx.app, 'POST', `/organizations/${org.id}/locations`, {
      name: 'HQ',
      modality: 'in_person',
      is_headquarters: true,
    })

    expect(res.status).toBe(201)
    const body = await res.json()
    expect(body.data.name).toBe('HQ')
    expect(body.data.modality).toBe('in_person')
    expect(body.data.is_headquarters).toBe(true)
    expect(body.data.address_id).toBeNull()
  })

  test('POST with address_id links location to address', async () => {
    const org = seedOrg(ctx.db, { name: 'Test Org' })
    const addr = seedAddress(ctx.db, { name: 'Portland Office', city: 'Portland', state: 'OR' })

    const res = await apiRequest(ctx.app, 'POST', `/organizations/${org.id}/locations`, {
      name: 'Portland',
      modality: 'hybrid',
      address_id: addr.id,
    })

    expect(res.status).toBe(201)
    const body = await res.json()
    expect(body.data.address_id).toBe(addr.id)
  })

  test('GET /organizations/:orgId/locations lists locations', async () => {
    const org = seedOrg(ctx.db, { name: 'Test Org' })
    await apiRequest(ctx.app, 'POST', `/organizations/${org.id}/locations`, { name: 'Alpha', modality: 'remote' })
    await apiRequest(ctx.app, 'POST', `/organizations/${org.id}/locations`, { name: 'Beta', modality: 'in_person' })

    const res = await apiRequest(ctx.app, 'GET', `/organizations/${org.id}/locations`)
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.length).toBe(2)
    // Sorted by name
    expect(body.data[0].name).toBe('Alpha')
    expect(body.data[1].name).toBe('Beta')
  })

  // -- PATCH /locations/:id --

  test('PATCH /locations/:id updates fields', async () => {
    const org = seedOrg(ctx.db, { name: 'Test Org' })
    const createRes = await apiRequest(ctx.app, 'POST', `/organizations/${org.id}/locations`, {
      name: 'Main',
      modality: 'in_person',
    })
    const { data: location } = await createRes.json()

    const res = await apiRequest(ctx.app, 'PATCH', `/locations/${location.id}`, {
      name: 'Online Campus',
    })

    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.name).toBe('Online Campus')
    expect(body.data.modality).toBe('in_person') // unchanged
  })

  test('PATCH /locations/:id with nonexistent ID returns 404', async () => {
    const res = await apiRequest(ctx.app, 'PATCH', '/locations/nonexistent', {
      name: 'New Name',
    })
    expect(res.status).toBe(404)
  })

  test('PATCH /locations/:id toggles is_headquarters', async () => {
    const org = seedOrg(ctx.db, { name: 'Test Org' })
    const createRes = await apiRequest(ctx.app, 'POST', `/organizations/${org.id}/locations`, {
      name: 'HQ',
      modality: 'in_person',
    })
    const { data: location } = await createRes.json()

    // Set to true
    const res1 = await apiRequest(ctx.app, 'PATCH', `/locations/${location.id}`, { is_headquarters: true })
    expect((await res1.json()).data.is_headquarters).toBe(true)

    // Set to false
    const res2 = await apiRequest(ctx.app, 'PATCH', `/locations/${location.id}`, { is_headquarters: false })
    expect((await res2.json()).data.is_headquarters).toBe(false)
  })

  test('PATCH /locations/:id updates address_id', async () => {
    const org = seedOrg(ctx.db, { name: 'Test Org' })
    const addr = seedAddress(ctx.db, { name: 'Office', city: 'Austin', state: 'TX' })
    const createRes = await apiRequest(ctx.app, 'POST', `/organizations/${org.id}/locations`, {
      name: 'Austin Office',
      modality: 'hybrid',
    })
    const { data: location } = await createRes.json()

    const res = await apiRequest(ctx.app, 'PATCH', `/locations/${location.id}`, { address_id: addr.id })
    expect(res.status).toBe(200)
    expect((await res.json()).data.address_id).toBe(addr.id)
  })

  // -- DELETE /locations/:id --

  test('DELETE /locations/:id removes location', async () => {
    const org = seedOrg(ctx.db, { name: 'Test Org' })
    const createRes = await apiRequest(ctx.app, 'POST', `/organizations/${org.id}/locations`, {
      name: 'Temp',
      modality: 'remote',
    })
    const { data: location } = await createRes.json()

    const res = await apiRequest(ctx.app, 'DELETE', `/locations/${location.id}`)
    expect(res.status).toBe(204)

    // Verify gone
    const listRes = await apiRequest(ctx.app, 'GET', `/organizations/${org.id}/locations`)
    expect((await listRes.json()).data.length).toBe(0)
  })

  // -- Backward-compat /campuses paths --

  test('GET /organizations/:orgId/campuses works (backward-compat)', async () => {
    const org = seedOrg(ctx.db, { name: 'Test Org' })
    await apiRequest(ctx.app, 'POST', `/organizations/${org.id}/locations`, { name: 'Campus A', modality: 'in_person' })

    const res = await apiRequest(ctx.app, 'GET', `/organizations/${org.id}/campuses`)
    expect(res.status).toBe(200)
    expect((await res.json()).data.length).toBe(1)
  })

  test('PATCH /campuses/:id works (backward-compat)', async () => {
    const org = seedOrg(ctx.db, { name: 'Test Org' })
    const createRes = await apiRequest(ctx.app, 'POST', `/organizations/${org.id}/locations`, {
      name: 'Old Path',
      modality: 'remote',
    })
    const { data: location } = await createRes.json()

    const res = await apiRequest(ctx.app, 'PATCH', `/campuses/${location.id}`, { name: 'Updated via campuses' })
    expect(res.status).toBe(200)
    expect((await res.json()).data.name).toBe('Updated via campuses')
  })

  test('POST /organizations/:orgId/campuses creates a location (backward-compat)', async () => {
    const org = seedOrg(ctx.db, { name: 'Test Org' })
    const res = await apiRequest(ctx.app, 'POST', `/organizations/${org.id}/campuses`, {
      name: 'Via campuses',
      modality: 'remote',
    })
    expect(res.status).toBe(201)
    const body = await res.json()
    expect(body.data.organization_id).toBe(org.id)
    expect(body.data.is_headquarters).toBe(false)
  })

  test('DELETE /campuses/:id removes location (backward-compat)', async () => {
    const org = seedOrg(ctx.db, { name: 'Test Org' })
    const createRes = await apiRequest(ctx.app, 'POST', `/organizations/${org.id}/locations`, {
      name: 'Temp',
      modality: 'remote',
    })
    const { data: location } = await createRes.json()
    const res = await apiRequest(ctx.app, 'DELETE', `/campuses/${location.id}`)
    expect(res.status).toBe(204)
    const listRes = await apiRequest(ctx.app, 'GET', `/organizations/${org.id}/campuses`)
    expect((await listRes.json()).data.length).toBe(0)
  })

  test('GET /organizations/:orgId/locations sorts by name, not headquarters first', async () => {
    const org = seedOrg(ctx.db, { name: 'Test Org' })
    await apiRequest(ctx.app, 'POST', `/organizations/${org.id}/locations`, { name: 'Zeta HQ', is_headquarters: true })
    await apiRequest(ctx.app, 'POST', `/organizations/${org.id}/locations`, { name: 'Alpha' })
    const res = await apiRequest(ctx.app, 'GET', `/organizations/${org.id}/locations`)
    const names = (await res.json()).data.map((l: { name: string }) => l.name)
    expect(names).toEqual(['Alpha', 'Zeta HQ'])
  })
})
