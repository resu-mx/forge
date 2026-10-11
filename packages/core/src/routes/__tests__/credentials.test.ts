/**
 * Credential route acceptance tests (Phase 86 T86.1).
 *
 * Acceptance criteria from the phase plan:
 *   [x] All 5 routes implemented
 *   [x] Proper status codes and error responses (201/200/204 happy path,
 *       400/404 error paths)
 *   [x] Request body validation passthrough (delegated to service)
 */

import { describe, test, expect, beforeEach, afterEach } from 'bun:test'
import { createTestApp, apiRequest, type TestContext } from './helpers'
import { seedOrganization, testUuid } from '../../db/__tests__/helpers'

describe('Credential routes', () => {
  let ctx: TestContext

  beforeEach(() => {
    ctx = createTestApp()
  })

  afterEach(() => {
    ctx.db.close()
  })

  const clearanceDetails = {
    level: 'top_secret',
    polygraph: 'ci',
    clearance_type: 'personnel',
    access_programs: ['sci'],
  }

  // ────────────────────────────────────────────────────────────────
  // POST /credentials
  // ────────────────────────────────────────────────────────────────

  describe('POST /credentials', () => {
    test('201 + data envelope for valid clearance', async () => {
      const res = await apiRequest(ctx.app, 'POST', '/credentials', {
        credential_type: 'clearance',
        label: 'TS/SCI',
        details: clearanceDetails,
      })
      expect(res.status).toBe(201)
      const body = await res.json()
      expect(body.data).toBeDefined()
      expect(body.data.id).toHaveLength(36)
      expect(body.data.credential_type).toBe('clearance')
      expect(body.data.label).toBe('TS/SCI')
      expect(body.data.status).toBe('active') // default
      expect(body.data.details.level).toBe('top_secret')
    })

    test('201 for each of 4 credential types', async () => {
      const cases = [
        { type: 'clearance', details: clearanceDetails },
        { type: 'drivers_license', details: { class: 'A', state: 'VA', endorsements: [] } },
        { type: 'bar_admission', details: { jurisdiction: 'Virginia', bar_number: null } },
        { type: 'medical_license', details: { license_type: 'MD', state: 'VA', license_number: null } },
      ]
      for (const { type, details } of cases) {
        const res = await apiRequest(ctx.app, 'POST', '/credentials', {
          credential_type: type,
          label: `Test ${type}`,
          details,
        })
        expect(res.status).toBe(201)
      }
    })

    test('details is taken and returned as an object, for each type', async () => {
      const cases = [
        { type: 'clearance', details: clearanceDetails },
        { type: 'drivers_license', details: { class: 'C', state: 'OR', endorsements: [] } },
        { type: 'bar_admission', details: { jurisdiction: 'Oregon', bar_number: null } },
        { type: 'medical_license', details: { license_type: 'MD', state: 'OR', license_number: '12345' } },
      ]
      for (const { type, details } of cases) {
        const res = await apiRequest(ctx.app, 'POST', '/credentials', {
          credential_type: type,
          label: `Shape ${type}`,
          details,
        })
        expect(res.status).toBe(201)
        expect((await res.json()).data.details).toEqual(details)
      }

      const list = (await (await apiRequest(ctx.app, 'GET', '/credentials')).json()).data
      expect(list).toHaveLength(4)
      for (const { type, details } of cases) {
        expect(list.find((c: { credential_type: string }) => c.credential_type === type).details).toEqual(details)
      }
    })

    test('400 for details sent as a JSON string', async () => {
      // The type-specific fields are read from the value itself, so a string has none.
      const res = await apiRequest(ctx.app, 'POST', '/credentials', {
        credential_type: 'drivers_license',
        label: 'DL',
        details: JSON.stringify({ class: 'C', state: 'OR' }),
      })
      expect(res.status).toBe(400)
      const body = await res.json()
      expect(body.error.code).toBe('VALIDATION_ERROR')
      expect(body.error.message).toContain('class')
    })

    test('400 for missing details', async () => {
      const res = await apiRequest(ctx.app, 'POST', '/credentials', {
        credential_type: 'bar_admission',
        label: 'No details',
      })
      expect(res.status).toBe(400)
      const body = await res.json()
      expect(body.error.code).toBe('VALIDATION_ERROR')
      expect(body.error.message).toContain('details')
    })

    test('400 for unknown credential_type', async () => {
      const res = await apiRequest(ctx.app, 'POST', '/credentials', {
        credential_type: 'bogus',
        label: 'Bad',
        details: {},
      })
      expect(res.status).toBe(400)
      const body = await res.json()
      expect(body.error).toBeDefined()
      expect(body.error.code).toBe('VALIDATION_ERROR')
    })

    test('400 for missing required detail (clearance without level)', async () => {
      const res = await apiRequest(ctx.app, 'POST', '/credentials', {
        credential_type: 'clearance',
        label: 'Missing',
        details: { clearance_type: 'personnel', access_programs: [] },
      })
      expect(res.status).toBe(400)
      const body = await res.json()
      expect(body.error.code).toBe('VALIDATION_ERROR')
      expect(body.error.message).toContain('level')
    })

    test('400 for empty label', async () => {
      const res = await apiRequest(ctx.app, 'POST', '/credentials', {
        credential_type: 'clearance',
        label: '',
        details: clearanceDetails,
      })
      expect(res.status).toBe(400)
    })

    test('404 for unknown organization_id', async () => {
      const res = await apiRequest(ctx.app, 'POST', '/credentials', {
        credential_type: 'clearance',
        label: 'Ghost Org',
        organization_id: testUuid(),
        details: clearanceDetails,
      })
      expect(res.status).toBe(404)
      const body = await res.json()
      expect(body.error.code).toBe('NOT_FOUND')
    })

    test('preserves organization_id when provided', async () => {
      const orgId = seedOrganization(ctx.db, { name: 'DoD', orgType: 'government' })
      const res = await apiRequest(ctx.app, 'POST', '/credentials', {
        credential_type: 'clearance',
        label: 'DoD TS',
        organization_id: orgId,
        details: clearanceDetails,
      })
      expect(res.status).toBe(201)
      const body = await res.json()
      expect(body.data.organization_id).toBe(orgId)
    })
  })

  // ────────────────────────────────────────────────────────────────
  // GET /credentials
  // ────────────────────────────────────────────────────────────────

  describe('GET /credentials', () => {
    test('returns empty list initially', async () => {
      const res = await apiRequest(ctx.app, 'GET', '/credentials')
      expect(res.status).toBe(200)
      const body = await res.json()
      expect(body.data).toEqual([])
    })

    test('returns all credentials', async () => {
      await apiRequest(ctx.app, 'POST', '/credentials', {
        credential_type: 'clearance',
        label: 'TS',
        details: clearanceDetails,
      })
      await apiRequest(ctx.app, 'POST', '/credentials', {
        credential_type: 'drivers_license',
        label: 'VA CDL',
        details: { class: 'A', state: 'VA', endorsements: [] },
      })

      const res = await apiRequest(ctx.app, 'GET', '/credentials')
      expect(res.status).toBe(200)
      const body = await res.json()
      expect(body.data).toHaveLength(2)
    })

    test('?type=clearance filters by credential_type', async () => {
      await apiRequest(ctx.app, 'POST', '/credentials', {
        credential_type: 'clearance',
        label: 'TS',
        details: clearanceDetails,
      })
      await apiRequest(ctx.app, 'POST', '/credentials', {
        credential_type: 'drivers_license',
        label: 'VA CDL',
        details: { class: 'A', state: 'VA', endorsements: [] },
      })

      const res = await apiRequest(ctx.app, 'GET', '/credentials?type=clearance')
      expect(res.status).toBe(200)
      const body = await res.json()
      expect(body.data).toHaveLength(1)
      expect(body.data[0].credential_type).toBe('clearance')
    })

    test('?type=bogus returns 400', async () => {
      const res = await apiRequest(ctx.app, 'GET', '/credentials?type=bogus')
      expect(res.status).toBe(400)
    })
  })

  // ────────────────────────────────────────────────────────────────
  // GET /credentials/:id
  // ────────────────────────────────────────────────────────────────

  describe('GET /credentials/:id', () => {
    test('200 + data envelope for existing credential', async () => {
      const createRes = await apiRequest(ctx.app, 'POST', '/credentials', {
        credential_type: 'clearance',
        label: 'TS',
        details: clearanceDetails,
      })
      const created = (await createRes.json()).data

      const res = await apiRequest(ctx.app, 'GET', `/credentials/${created.id}`)
      expect(res.status).toBe(200)
      const body = await res.json()
      expect(body.data.id).toBe(created.id)
      expect(body.data.details).toEqual(clearanceDetails)
    })

    test('404 for unknown id', async () => {
      const res = await apiRequest(ctx.app, 'GET', `/credentials/${testUuid()}`)
      expect(res.status).toBe(404)
      const body = await res.json()
      expect(body.error.code).toBe('NOT_FOUND')
    })
  })

  // ────────────────────────────────────────────────────────────────
  // PATCH /credentials/:id
  // ────────────────────────────────────────────────────────────────

  describe('PATCH /credentials/:id', () => {
    test('200 + updated data', async () => {
      const createRes = await apiRequest(ctx.app, 'POST', '/credentials', {
        credential_type: 'clearance',
        label: 'Old',
        details: clearanceDetails,
      })
      const created = (await createRes.json()).data

      const res = await apiRequest(ctx.app, 'PATCH', `/credentials/${created.id}`, {
        label: 'New Label',
      })
      expect(res.status).toBe(200)
      const body = await res.json()
      expect(body.data.label).toBe('New Label')
    })

    test('partial details update merges with existing JSON', async () => {
      const createRes = await apiRequest(ctx.app, 'POST', '/credentials', {
        credential_type: 'clearance',
        label: 'TS',
        details: clearanceDetails, // polygraph: ci, level: top_secret
      })
      const created = (await createRes.json()).data

      // Update only polygraph
      const res = await apiRequest(ctx.app, 'PATCH', `/credentials/${created.id}`, {
        details: { polygraph: 'full_scope' },
      })
      expect(res.status).toBe(200)
      const body = await res.json()
      expect(body.data.details.polygraph).toBe('full_scope') // updated
      expect(body.data.details.level).toBe('top_secret')     // preserved
      expect(body.data.details.access_programs).toEqual(['sci']) // preserved

      const getRes = await apiRequest(ctx.app, 'GET', `/credentials/${created.id}`)
      expect((await getRes.json()).data.details).toEqual({ ...clearanceDetails, polygraph: 'full_scope' })
    })

    test('400 when a details update drops a required field', async () => {
      const createRes = await apiRequest(ctx.app, 'POST', '/credentials', {
        credential_type: 'drivers_license',
        label: 'DL',
        details: { class: 'C', state: 'OR', endorsements: [] },
      })
      const created = (await createRes.json()).data

      const res = await apiRequest(ctx.app, 'PATCH', `/credentials/${created.id}`, {
        details: { state: '' },
      })
      expect(res.status).toBe(400)
      const body = await res.json()
      expect(body.error.code).toBe('VALIDATION_ERROR')
      expect(body.error.message).toContain('state')
    })

    test('404 for unknown id', async () => {
      const res = await apiRequest(ctx.app, 'PATCH', `/credentials/${testUuid()}`, { label: 'x' })
      expect(res.status).toBe(404)
    })

    test('400 for empty label on update', async () => {
      const createRes = await apiRequest(ctx.app, 'POST', '/credentials', {
        credential_type: 'clearance',
        label: 'TS',
        details: clearanceDetails,
      })
      const created = (await createRes.json()).data

      const res = await apiRequest(ctx.app, 'PATCH', `/credentials/${created.id}`, {
        label: '   ',
      })
      expect(res.status).toBe(400)
    })

    test('400 for invalid status on update', async () => {
      const createRes = await apiRequest(ctx.app, 'POST', '/credentials', {
        credential_type: 'clearance',
        label: 'TS',
        details: clearanceDetails,
      })
      const created = (await createRes.json()).data

      const res = await apiRequest(ctx.app, 'PATCH', `/credentials/${created.id}`, {
        status: 'wibble',
      })
      expect(res.status).toBe(400)
    })
  })

  // ────────────────────────────────────────────────────────────────
  // DELETE /credentials/:id
  // ────────────────────────────────────────────────────────────────

  describe('DELETE /credentials/:id', () => {
    test('204 for existing credential', async () => {
      const createRes = await apiRequest(ctx.app, 'POST', '/credentials', {
        credential_type: 'clearance',
        label: 'Temp',
        details: clearanceDetails,
      })
      const created = (await createRes.json()).data

      const res = await apiRequest(ctx.app, 'DELETE', `/credentials/${created.id}`)
      expect(res.status).toBe(204)

      // Verify gone
      const getRes = await apiRequest(ctx.app, 'GET', `/credentials/${created.id}`)
      expect(getRes.status).toBe(404)
    })

    test('404 for unknown id', async () => {
      const res = await apiRequest(ctx.app, 'DELETE', `/credentials/${testUuid()}`)
      expect(res.status).toBe(404)
    })
  })

  // ────────────────────────────────────────────────────────────────
  // Envelope shape — every response follows {data} or {error}
  // ────────────────────────────────────────────────────────────────

  describe('response envelope shape', () => {
    test('success response is {data: ...}', async () => {
      const res = await apiRequest(ctx.app, 'GET', '/credentials')
      const body = await res.json()
      expect(body).toHaveProperty('data')
      expect(body).not.toHaveProperty('error')
    })

    test('error response is {error: {code, message}}', async () => {
      const res = await apiRequest(ctx.app, 'GET', `/credentials/${testUuid()}`)
      const body = await res.json()
      expect(body).toHaveProperty('error')
      expect(body.error).toHaveProperty('code')
      expect(body.error).toHaveProperty('message')
      expect(body).not.toHaveProperty('data')
    })
  })
})
