import { describe, test, expect, beforeEach, afterEach } from 'bun:test'
import { Database } from 'bun:sqlite'
import { createTestApp, apiRequest, type TestContext } from './helpers'
import { seedSource } from '../../db/__tests__/helpers'

describe('Source Routes', () => {
  let ctx: TestContext

  beforeEach(() => {
    ctx = createTestApp()
  })

  afterEach(() => {
    ctx.db.close()
  })

  // ── POST /sources ──────────────────────────────────────────────────

  test('POST /sources creates a source and returns 201', async () => {
    const res = await apiRequest(ctx.app, 'POST', '/sources', {
      title: 'Cloud Migration Lead',
      description: 'Led cloud migration for forensics platform.',
      source_type: 'role',
    })
    expect(res.status).toBe(201)
    const body = await res.json()
    expect(body.data).toBeDefined()
    expect(body.data.id).toHaveLength(36)
    expect(body.data.title).toBe('Cloud Migration Lead')
    expect(body.data.source_type).toBe('role')
  })

  test('POST /sources with empty title returns 400', async () => {
    const res = await apiRequest(ctx.app, 'POST', '/sources', {
      title: '',
      description: 'Some description.',
    })
    expect(res.status).toBe(400)
    const body = await res.json()
    expect(body.error).toBeDefined()
    expect(body.error.code).toBe('VALIDATION_ERROR')
  })

  // ── GET /sources ───────────────────────────────────────────────────

  test('GET /sources returns 200 with pagination', async () => {
    seedSource(ctx.db, { title: 'Source A' })
    seedSource(ctx.db, { title: 'Source B' })

    const res = await apiRequest(ctx.app, 'GET', '/sources')
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data).toBeArray()
    expect(body.data.length).toBe(2)
    expect(body.pagination).toBeDefined()
    expect(body.pagination.total).toBe(2)
    expect(body.pagination.offset).toBe(0)
    expect(body.pagination.limit).toBe(50)
  })

  test('GET /sources?source_type=role filters correctly', async () => {
    seedSource(ctx.db, { title: 'Role Source', sourceType: 'role' })
    seedSource(ctx.db, { title: 'General Source', sourceType: 'general' })

    const res = await apiRequest(ctx.app, 'GET', '/sources?source_type=role')
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.length).toBe(1)
    expect(body.data[0].title).toBe('Role Source')
  })

  // ── GET /sources/:id ──────────────────────────────────────────────

  test('GET /sources/:id returns 200 for existing source', async () => {
    const sourceId = seedSource(ctx.db, { title: 'My Source' })

    const res = await apiRequest(ctx.app, 'GET', `/sources/${sourceId}`)
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.id).toBe(sourceId)
    expect(body.data.title).toBe('My Source')
  })

  test('GET /sources/:id returns 404 for non-existent source', async () => {
    const res = await apiRequest(ctx.app, 'GET', '/sources/00000000-0000-0000-0000-000000000000')
    expect(res.status).toBe(404)
    const body = await res.json()
    expect(body.error.code).toBe('NOT_FOUND')
  })

  // ── PATCH /sources/:id ────────────────────────────────────────────

  test('PATCH /sources/:id updates and returns 200', async () => {
    const sourceId = seedSource(ctx.db, { title: 'Old Title' })

    const res = await apiRequest(ctx.app, 'PATCH', `/sources/${sourceId}`, {
      title: 'New Title',
    })
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.title).toBe('New Title')
  })

  // ── PATCH /sources/:id: status (board moves) ──────────────────────

  test('PATCH /sources/:id moves a source to Approved and it stays there', async () => {
    const sourceId = seedSource(ctx.db, { status: 'draft' })

    const res = await apiRequest(ctx.app, 'PATCH', `/sources/${sourceId}`, { status: 'approved' })
    expect(res.status).toBe(200)
    expect((await res.json()).data.status).toBe('approved')

    // Not just echoed back: a fresh read sees it.
    const after = await apiRequest(ctx.app, 'GET', `/sources/${sourceId}`)
    expect((await after.json()).data.status).toBe('approved')
  })

  for (const target of ['draft', 'in_review', 'approved', 'rejected', 'archived']) {
    test(`PATCH /sources/:id accepts status "${target}" from any other column`, async () => {
      const from = target === 'draft' ? 'archived' : 'draft'
      const sourceId = seedSource(ctx.db, { status: from })

      const res = await apiRequest(ctx.app, 'PATCH', `/sources/${sourceId}`, { status: target })
      expect(res.status).toBe(200)
      expect((await res.json()).data.status).toBe(target)
    })
  }

  for (const bad of ['deriving', 'bogus']) {
    test(`PATCH /sources/:id rejects status "${bad}" with 400 and leaves the source alone`, async () => {
      const sourceId = seedSource(ctx.db, { status: 'draft', title: 'Keep Me' })

      const res = await apiRequest(ctx.app, 'PATCH', `/sources/${sourceId}`, {
        status: bad,
        title: 'Changed Too',
      })
      expect(res.status).toBe(400)
      expect((await res.json()).error.code).toBe('VALIDATION_ERROR')

      const after = (await (await apiRequest(ctx.app, 'GET', `/sources/${sourceId}`)).json()).data
      expect(after.status).toBe('draft')
      expect(after.title).toBe('Keep Me')
    })
  }

  test('PATCH /sources/:id without status leaves the status unchanged', async () => {
    const sourceId = seedSource(ctx.db, { status: 'in_review' })

    const res = await apiRequest(ctx.app, 'PATCH', `/sources/${sourceId}`, { title: 'Renamed' })
    expect(res.status).toBe(200)
    expect((await res.json()).data.status).toBe('in_review')
  })

  // ── DELETE /sources/:id ───────────────────────────────────────────

  // ── Education sub-type round-trips ──────────────────────────────────

  test('POST source with education_type=degree round-trips all fields', async () => {
    const res = await apiRequest(ctx.app, 'POST', '/sources', {
      title: 'PhD Physics',
      description: 'Doctoral program.',
      source_type: 'education',
      education: {
        education_type: 'degree',
        degree_level: 'doctoral',
        degree_type: 'PhD',
        field: 'Physics',
        gpa: '4.0/4.0',
        location: 'Pasadena, CA',
        edu_description: 'Research in quantum computing.',
      },
    })

    expect(res.status).toBe(201)
    const body = await res.json()
    expect(body.data).toBeDefined()
    // Verify the extension data is returned
    const fetched = await apiRequest(ctx.app, 'GET', `/sources/${body.data.id}`)
    const fetchedBody = await fetched.json()
    expect(fetchedBody.data.education.degree_level).toBe('doctoral')
    expect(fetchedBody.data.education.degree_type).toBe('PhD')
    expect(fetchedBody.data.education.gpa).toBe('4.0/4.0')
    expect(fetchedBody.data.education.location).toBe('Pasadena, CA')
    expect(fetchedBody.data.education.edu_description).toBe('Research in quantum computing.')
  })

  test('PATCH source changes certificate_subtype via nested education payload', async () => {
    const created = await apiRequest(ctx.app, 'POST', '/sources', {
      title: 'Cert',
      description: 'A cert.',
      source_type: 'education',
      education: {
        education_type: 'certificate',
        certificate_subtype: 'vendor',
      },
    })
    expect(created.status).toBe(201)
    const createdBody = await created.json()

    const updated = await apiRequest(ctx.app, 'PATCH', `/sources/${createdBody.data.id}`, {
      education: { certificate_subtype: 'professional' },
    })
    expect(updated.status).toBe(200)
    const updatedBody = await updated.json()
    expect(updatedBody.data.education.certificate_subtype).toBe('professional')
  })

  test('GET source includes all new education fields', async () => {
    const created = await apiRequest(ctx.app, 'POST', '/sources', {
      title: 'Course',
      description: 'A course.',
      source_type: 'education',
      education: {
        education_type: 'course',
        location: 'Virtual',
        edu_description: 'Security training.',
      },
    })
    expect(created.status).toBe(201)
    const createdBody = await created.json()

    const fetched = await apiRequest(ctx.app, 'GET', `/sources/${createdBody.data.id}`)
    expect(fetched.status).toBe(200)
    const fetchedBody = await fetched.json()
    expect(fetchedBody.data.education.location).toBe('Virtual')
    expect(fetchedBody.data.education.edu_description).toBe('Security training.')
    // Null fields are returned, not omitted
    expect(fetchedBody.data.education.degree_level).toBeNull()
    expect(fetchedBody.data.education.gpa).toBeNull()
  })

  // ── DELETE /sources/:id ───────────────────────────────────────────

  test('DELETE /sources/:id returns 204', async () => {
    const sourceId = seedSource(ctx.db)

    const res = await apiRequest(ctx.app, 'DELETE', `/sources/${sourceId}`)
    expect(res.status).toBe(204)

    // Verify deleted
    const getRes = await apiRequest(ctx.app, 'GET', `/sources/${sourceId}`)
    expect(getRes.status).toBe(404)
  })
})
