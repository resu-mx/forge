import { describe, test, expect, beforeEach, afterEach } from 'bun:test'
import { createTestApp, apiRequest, type TestContext } from './helpers'
import { seedProfile } from '../../db/__tests__/helpers'

describe('Profile routes', () => {
  let ctx: TestContext

  beforeEach(() => {
    ctx = createTestApp()
  })

  afterEach(() => {
    ctx.db.close()
  })

  // -- GET /profile ---------------------------------------------------------

  test('GET /profile returns the profile', async () => {
    const res = await apiRequest(ctx.app, 'GET', '/profile')
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data).toBeDefined()
    expect(body.data.id).toHaveLength(36)
    expect(body.data.name).toBe('User')
    expect(body.data.created_at).toBeDefined()
    expect(body.data.updated_at).toBeDefined()
  })

  test('GET /profile returns seeded profile data', async () => {
    seedProfile(ctx.db, { name: 'Adam', email: 'adam@test.com' })
    const res = await apiRequest(ctx.app, 'GET', '/profile')
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.name).toBe('Adam')
    expect(body.data.email).toBe('adam@test.com')
    // clearance moved to credentials entity in migration 037 (Phase 84)
    expect(body.data.clearance).toBeUndefined()
  })

  // -- PATCH /profile -------------------------------------------------------

  test('PATCH /profile updates provided fields', async () => {
    const res = await apiRequest(ctx.app, 'PATCH', '/profile', {
      name: 'Adam',
      email: 'adam@example.com',
    })
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.name).toBe('Adam')
    expect(body.data.email).toBe('adam@example.com')
  })

  test('PATCH /profile with empty name returns 400', async () => {
    const res = await apiRequest(ctx.app, 'PATCH', '/profile', { name: '' })
    expect(res.status).toBe(400)
    const body = await res.json()
    expect(body.error.code).toBe('VALIDATION_ERROR')
  })

  test('PATCH /profile with whitespace-only name returns 400', async () => {
    const res = await apiRequest(ctx.app, 'PATCH', '/profile', { name: '   ' })
    expect(res.status).toBe(400)
    const body = await res.json()
    expect(body.error.code).toBe('VALIDATION_ERROR')
  })

  test('PATCH /profile with empty body returns 200 unchanged', async () => {
    const getRes = await apiRequest(ctx.app, 'GET', '/profile')
    const before = (await getRes.json()).data

    const res = await apiRequest(ctx.app, 'PATCH', '/profile', {})
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.name).toBe(before.name)
  })

  test('PATCH /profile updates all fields at once', async () => {
    // Post-migration 046: location/linkedin/github/website moved to addresses + profile_urls
    const res = await apiRequest(ctx.app, 'PATCH', '/profile', {
      name: 'Adam',
      email: 'adam@example.com',
      phone: '+1-555-0123',
      address: { name: 'Washington, DC', city: 'Washington', state: 'DC' },
      urls: [
        { key: 'linkedin', url: 'linkedin.com/in/adam' },
        { key: 'github', url: 'github.com/adam' },
        { key: 'blog', url: 'adam.dev' },
      ],
    })
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.name).toBe('Adam')
    expect(body.data.email).toBe('adam@example.com')
    expect(body.data.phone).toBe('+1-555-0123')
    expect(body.data.address.name).toBe('Washington, DC')
    expect(body.data.urls).toHaveLength(3)
    expect(body.data.urls[0].key).toBe('linkedin')
    expect(body.data.urls[0].url).toBe('linkedin.com/in/adam')
  })

  test('PATCH /profile refreshes updated_at', async () => {
    const getRes = await apiRequest(ctx.app, 'GET', '/profile')
    const before = (await getRes.json()).data

    const res = await apiRequest(ctx.app, 'PATCH', '/profile', { name: 'Updated' })
    const body = await res.json()
    expect(body.data.updated_at >= before.updated_at).toBe(true)
  })

  // Strict updated_at checks. The case above uses `>=`, which a missed bump
  // satisfies. Here the old value is seeded through SQL, so only a real write
  // of the current time can pass. TS refreshes updated_at on every PATCH
  // because ProfileService always calls elm.update('user_profile', ...), even
  // when only the address or the URL list changed.
  describe('PATCH /profile updated_at', () => {
    const OLD = '2020-01-01T00:00:00Z'
    const ISO_Z = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/

    function seedOldUpdatedAt(): void {
      const result = ctx.db.run('UPDATE user_profile SET updated_at = ?', [OLD])
      expect(result.changes).toBe(1)
    }

    function expectBumped(data: { updated_at: string }): void {
      expect(data.updated_at).toMatch(ISO_Z)
      expect(data.updated_at > OLD).toBe(true)
    }

    test('scalar-only PATCH bumps updated_at', async () => {
      seedOldUpdatedAt()
      const res = await apiRequest(ctx.app, 'PATCH', '/profile', { name: 'Scalar' })
      expect(res.status).toBe(200)
      expectBumped((await res.json()).data)
    })

    test('address-only PATCH creating a new address bumps updated_at', async () => {
      seedOldUpdatedAt()
      const res = await apiRequest(ctx.app, 'PATCH', '/profile', {
        address: { name: 'Home', city: 'Washington', state: 'DC' },
      })
      expect(res.status).toBe(200)
      const data = (await res.json()).data
      expect(data.address.city).toBe('Washington')
      expectBumped(data)
    })

    test('address-only PATCH updating an existing address bumps updated_at', async () => {
      seedProfile(ctx.db, { addressName: 'Old address' })
      seedOldUpdatedAt()
      const res = await apiRequest(ctx.app, 'PATCH', '/profile', {
        address: { name: 'New address', city: 'Boston', state: 'MA' },
      })
      expect(res.status).toBe(200)
      const data = (await res.json()).data
      expect(data.address.name).toBe('New address')
      expectBumped(data)
    })

    test('empty-body PATCH bumps updated_at', async () => {
      seedOldUpdatedAt()
      const res = await apiRequest(ctx.app, 'PATCH', '/profile', {})
      expect(res.status).toBe(200)
      expectBumped((await res.json()).data)
    })

    test('urls-only PATCH bumps updated_at', async () => {
      seedOldUpdatedAt()
      const res = await apiRequest(ctx.app, 'PATCH', '/profile', {
        urls: [{ key: 'github', url: 'github.com/adam' }],
      })
      expect(res.status).toBe(200)
      const data = (await res.json()).data
      expect(data.urls).toHaveLength(1)
      expectBumped(data)
    })
  })

  test('PATCH /profile ignores unknown fields', async () => {
    const res = await apiRequest(ctx.app, 'PATCH', '/profile', {
      name: 'Safe',
      evil_field: 'should be ignored',
    })
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.name).toBe('Safe')
    expect(body.data.evil_field).toBeUndefined()
  })

  // -- Contract shape -------------------------------------------------------

  test('GET /profile follows { data: entity } contract', async () => {
    const res = await apiRequest(ctx.app, 'GET', '/profile')
    const body = await res.json()
    expect(body).toHaveProperty('data')
    expect(body).not.toHaveProperty('error')
    expect(body).not.toHaveProperty('pagination')
    expect(body.data).toHaveProperty('id')
    expect(body.data).toHaveProperty('name')
    expect(body.data).toHaveProperty('created_at')
    expect(body.data).toHaveProperty('updated_at')
  })
})
