import { describe, expect, it, beforeEach, afterEach, mock } from 'bun:test'
import { ForgeClient } from '../client'

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function jsonResponse(
  body: unknown,
  init: { status?: number } = {},
): Response {
  const status = init.status ?? 200
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  })
}

function noContentResponse(): Response {
  return new Response(null, { status: 204 })
}

/** Extract the URL string from the first fetch call. */
function calledUrl(fetchMock: ReturnType<typeof mock>): string {
  return (fetchMock.mock.calls[0] as [string])[0]
}

/** Extract the RequestInit from the first fetch call. */
function calledInit(fetchMock: ReturnType<typeof mock>): RequestInit {
  return (fetchMock.mock.calls[0] as [string, RequestInit])[1]
}

// ---------------------------------------------------------------------------
// Suite
// ---------------------------------------------------------------------------

describe('Resource clients', () => {
  const originalFetch = globalThis.fetch
  let fetchMock: ReturnType<typeof mock>
  let client: ForgeClient

  beforeEach(() => {
    fetchMock = mock(() => Promise.resolve(new Response()))
    globalThis.fetch = fetchMock as typeof fetch
    client = new ForgeClient({ baseUrl: 'http://localhost:3000' })
  })

  afterEach(() => {
    globalThis.fetch = originalFetch
  })

  // -----------------------------------------------------------------------
  // sources
  // -----------------------------------------------------------------------

  describe('sources', () => {
    it('create sends POST /api/sources with body', async () => {
      const created = { id: 's1', title: 'My Source', source_type: 'general' }
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: created }, { status: 201 })),
      )

      const result = await client.sources.create({
        title: 'My Source',
        description: 'desc',
      })

      expect(calledUrl(fetchMock)).toBe('http://localhost:3000/api/sources')
      expect(calledInit(fetchMock).method).toBe('POST')
      expect(JSON.parse(calledInit(fetchMock).body as string)).toEqual({
        title: 'My Source',
        description: 'desc',
      })
      expect(result).toEqual({ ok: true, data: created })
    })

    it('create sends source_type and extension data', async () => {
      const created = { id: 's1', title: 'Engineer', source_type: 'role' }
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: created }, { status: 201 })),
      )

      await client.sources.create({
        title: 'Engineer',
        description: 'Role at Acme',
        source_type: 'role',
        role: { organization_id: 'org-1', is_current: true },
      })

      const body = JSON.parse(calledInit(fetchMock).body as string)
      expect(body.source_type).toBe('role')
      expect(body.role.organization_id).toBe('org-1')
    })

    it('list sends GET /api/sources with filter params', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(
          jsonResponse({
            data: [],
            pagination: { total: 0, offset: 0, limit: 50 },
          }),
        ),
      )

      await client.sources.list({ status: 'approved', source_type: 'role', offset: 10 })

      const url = calledUrl(fetchMock)
      expect(url).toContain('/api/sources')
      expect(url).toContain('status=approved')
      expect(url).toContain('source_type=role')
      expect(url).toContain('offset=10')
    })

    it('get sends GET /api/sources/:id', async () => {
      const source = { id: 's1', title: 'Test', source_type: 'general', bullet_count: 3 }
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: source })),
      )

      const result = await client.sources.get('s1')

      expect(calledUrl(fetchMock)).toBe('http://localhost:3000/api/sources/s1')
      expect(calledInit(fetchMock).method).toBe('GET')
      expect(result).toEqual({ ok: true, data: source })
    })

    it('update sends PATCH /api/sources/:id', async () => {
      const updated = { id: 's1', title: 'Updated' }
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: updated })),
      )

      await client.sources.update('s1', { title: 'Updated' })

      expect(calledUrl(fetchMock)).toBe('http://localhost:3000/api/sources/s1')
      expect(calledInit(fetchMock).method).toBe('PATCH')
      expect(JSON.parse(calledInit(fetchMock).body as string)).toEqual({
        title: 'Updated',
      })
    })

    it('delete sends DELETE /api/sources/:id', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(noContentResponse()),
      )

      const result = await client.sources.delete('s1')

      expect(calledUrl(fetchMock)).toBe('http://localhost:3000/api/sources/s1')
      expect(calledInit(fetchMock).method).toBe('DELETE')
      expect(result.ok).toBe(true)
    })

  })

  // -----------------------------------------------------------------------
  // bullets
  // -----------------------------------------------------------------------

  describe('bullets', () => {
    it('list sends GET /api/bullets with filter params', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(
          jsonResponse({
            data: [],
            pagination: { total: 0, offset: 0, limit: 50 },
          }),
        ),
      )

      await client.bullets.list({ source_id: 's1', status: 'approved' })

      const url = calledUrl(fetchMock)
      expect(url).toContain('/api/bullets')
      expect(url).toContain('source_id=s1')
      expect(url).toContain('status=approved')
    })

    it('get sends GET /api/bullets/:id', async () => {
      const bullet = {
        id: 'b1',
        content: 'test',
        sources: [{ id: 's1', title: 'Source', is_primary: true }],
        perspective_count: 2,
      }
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: bullet })),
      )

      const result = await client.bullets.get('b1')

      expect(calledUrl(fetchMock)).toBe('http://localhost:3000/api/bullets/b1')
      expect(calledInit(fetchMock).method).toBe('GET')
      expect(result).toEqual({ ok: true, data: bullet })
    })

    it('update sends PATCH /api/bullets/:id with body', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: { id: 'b1' } })),
      )

      await client.bullets.update('b1', {
        content: 'updated',
        technologies: ['TypeScript'],
      })

      expect(calledInit(fetchMock).method).toBe('PATCH')
      expect(JSON.parse(calledInit(fetchMock).body as string)).toEqual({
        content: 'updated',
        technologies: ['TypeScript'],
      })
    })

    it('delete sends DELETE /api/bullets/:id', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(noContentResponse()),
      )

      await client.bullets.delete('b1')

      expect(calledUrl(fetchMock)).toBe('http://localhost:3000/api/bullets/b1')
      expect(calledInit(fetchMock).method).toBe('DELETE')
    })

    it('approve sends PATCH /api/bullets/:id/approve with no body', async () => {
      const bullet = { id: 'b1', status: 'approved' }
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: bullet })),
      )

      const result = await client.bullets.approve('b1')

      expect(calledUrl(fetchMock)).toBe(
        'http://localhost:3000/api/bullets/b1/approve',
      )
      expect(calledInit(fetchMock).method).toBe('PATCH')
      expect(calledInit(fetchMock).body).toBeUndefined()
      expect(result).toEqual({ ok: true, data: bullet })
    })

    it('reject sends PATCH /api/bullets/:id/reject with rejection_reason body', async () => {
      const bullet = { id: 'b1', status: 'rejected', rejection_reason: 'Too vague' }
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: bullet })),
      )

      const result = await client.bullets.reject('b1', {
        rejection_reason: 'Too vague',
      })

      expect(calledUrl(fetchMock)).toBe(
        'http://localhost:3000/api/bullets/b1/reject',
      )
      expect(calledInit(fetchMock).method).toBe('PATCH')
      expect(JSON.parse(calledInit(fetchMock).body as string)).toEqual({
        rejection_reason: 'Too vague',
      })
      expect(result).toEqual({ ok: true, data: bullet })
    })

    it('reopen sends PATCH /api/bullets/:id/reopen', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: { id: 'b1', status: 'draft' } })),
      )

      await client.bullets.reopen('b1')

      expect(calledUrl(fetchMock)).toBe(
        'http://localhost:3000/api/bullets/b1/reopen',
      )
      expect(calledInit(fetchMock).method).toBe('PATCH')
    })

  })

  // -----------------------------------------------------------------------
  // perspectives
  // -----------------------------------------------------------------------

  describe('perspectives', () => {
    it('list sends GET /api/perspectives with filter params', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(
          jsonResponse({
            data: [],
            pagination: { total: 0, offset: 0, limit: 50 },
          }),
        ),
      )

      await client.perspectives.list({
        bullet_id: 'b1',
        archetype: 'sre',
        framing: 'accomplishment',
      })

      const url = calledUrl(fetchMock)
      expect(url).toContain('bullet_id=b1')
      expect(url).toContain('archetype=sre')
      expect(url).toContain('framing=accomplishment')
    })

    it('get sends GET /api/perspectives/:id and returns chain', async () => {
      const perspective = {
        id: 'p1',
        content: 'perspective content',
        bullet: { id: 'b1', content: 'bullet' },
        source: { id: 's1', title: 'source' },
      }
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: perspective })),
      )

      const result = await client.perspectives.get('p1')

      expect(calledUrl(fetchMock)).toBe(
        'http://localhost:3000/api/perspectives/p1',
      )
      expect(result).toEqual({ ok: true, data: perspective })
    })

    it('approve sends PATCH /api/perspectives/:id/approve', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(
          jsonResponse({ data: { id: 'p1', status: 'approved' } }),
        ),
      )

      await client.perspectives.approve('p1')

      expect(calledUrl(fetchMock)).toBe(
        'http://localhost:3000/api/perspectives/p1/approve',
      )
      expect(calledInit(fetchMock).method).toBe('PATCH')
    })

    it('reject sends PATCH /api/perspectives/:id/reject with body', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(
          jsonResponse({ data: { id: 'p1', status: 'rejected' } }),
        ),
      )

      await client.perspectives.reject('p1', {
        rejection_reason: 'Not relevant',
      })

      expect(calledUrl(fetchMock)).toBe(
        'http://localhost:3000/api/perspectives/p1/reject',
      )
      expect(JSON.parse(calledInit(fetchMock).body as string)).toEqual({
        rejection_reason: 'Not relevant',
      })
    })

    it('reopen sends PATCH /api/perspectives/:id/reopen', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(
          jsonResponse({ data: { id: 'p1', status: 'draft' } }),
        ),
      )

      await client.perspectives.reopen('p1')

      expect(calledUrl(fetchMock)).toBe(
        'http://localhost:3000/api/perspectives/p1/reopen',
      )
    })

    it('update sends PATCH /api/perspectives/:id', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: { id: 'p1' } })),
      )

      await client.perspectives.update('p1', { framing: 'responsibility' })

      expect(calledInit(fetchMock).method).toBe('PATCH')
      expect(calledUrl(fetchMock)).toBe(
        'http://localhost:3000/api/perspectives/p1',
      )
    })

    it('delete sends DELETE /api/perspectives/:id', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(noContentResponse()),
      )

      await client.perspectives.delete('p1')

      expect(calledInit(fetchMock).method).toBe('DELETE')
      expect(calledUrl(fetchMock)).toBe(
        'http://localhost:3000/api/perspectives/p1',
      )
    })
  })

  // -----------------------------------------------------------------------
  // resumes
  // -----------------------------------------------------------------------

  describe('resumes', () => {
    it('create sends POST /api/resumes with body', async () => {
      const resume = { id: 'r1', name: 'SRE Resume' }
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: resume }, { status: 201 })),
      )

      const result = await client.resumes.create({
        name: 'SRE Resume',
        target_role: 'SRE',
        target_employer: 'Acme',
        archetype: 'sre',
      })

      expect(calledUrl(fetchMock)).toBe('http://localhost:3000/api/resumes')
      expect(calledInit(fetchMock).method).toBe('POST')
      expect(result).toEqual({ ok: true, data: resume })
    })

    it('list sends GET /api/resumes with pagination', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(
          jsonResponse({
            data: [],
            pagination: { total: 0, offset: 0, limit: 50 },
          }),
        ),
      )

      await client.resumes.list({ limit: 10 })

      const url = calledUrl(fetchMock)
      expect(url).toContain('limit=10')
    })

    it('get sends GET /api/resumes/:id and returns ResumeWithEntries', async () => {
      const resume = { id: 'r1', sections: { work_history: [] } }
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: resume })),
      )

      const result = await client.resumes.get('r1')

      expect(calledUrl(fetchMock)).toBe('http://localhost:3000/api/resumes/r1')
      expect(result).toEqual({ ok: true, data: resume })
    })

    it('update sends PATCH /api/resumes/:id', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: { id: 'r1' } })),
      )

      await client.resumes.update('r1', { name: 'Updated' })

      expect(calledInit(fetchMock).method).toBe('PATCH')
    })

    it('delete sends DELETE /api/resumes/:id', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(noContentResponse()),
      )

      await client.resumes.delete('r1')

      expect(calledInit(fetchMock).method).toBe('DELETE')
      expect(calledUrl(fetchMock)).toBe('http://localhost:3000/api/resumes/r1')
    })

    it('addEntry sends POST /api/resumes/:id/entries', async () => {
      const entry = { id: 'e1', perspective_id: 'p1', section_id: 'sec1', position: 0 }
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: entry }, { status: 201 })),
      )

      const result = await client.resumes.addEntry('r1', {
        perspective_id: 'p1',
        section_id: 'sec1',
        position: 0,
      })

      expect(calledUrl(fetchMock)).toBe(
        'http://localhost:3000/api/resumes/r1/entries',
      )
      expect(calledInit(fetchMock).method).toBe('POST')
      expect(JSON.parse(calledInit(fetchMock).body as string)).toEqual({
        perspective_id: 'p1',
        section_id: 'sec1',
        position: 0,
      })
      expect(result).toEqual({ ok: true, data: entry })
    })

    it('listEntries sends GET /api/resumes/:id/entries', async () => {
      const entries = [{ id: 'e1' }, { id: 'e2' }]
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: entries })),
      )

      const result = await client.resumes.listEntries('r1')

      expect(calledUrl(fetchMock)).toBe(
        'http://localhost:3000/api/resumes/r1/entries',
      )
      expect(calledInit(fetchMock).method).toBe('GET')
      expect(result).toEqual({ ok: true, data: entries })
    })

    it('updateEntry sends PATCH /api/resumes/:id/entries/:entryId', async () => {
      const entry = { id: 'e1', content: 'Edited version' }
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: entry })),
      )

      const result = await client.resumes.updateEntry('r1', 'e1', {
        content: 'Edited version',
      })

      expect(calledUrl(fetchMock)).toBe(
        'http://localhost:3000/api/resumes/r1/entries/e1',
      )
      expect(calledInit(fetchMock).method).toBe('PATCH')
      expect(JSON.parse(calledInit(fetchMock).body as string)).toEqual({
        content: 'Edited version',
      })
      expect(result).toEqual({ ok: true, data: entry })
    })

    it('updateEntry with content: null resets to reference mode', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: { id: 'e1', content: null } })),
      )

      await client.resumes.updateEntry('r1', 'e1', { content: null })

      const body = JSON.parse(calledInit(fetchMock).body as string)
      expect(body.content).toBeNull()
    })

    it('removeEntry sends DELETE /api/resumes/:id/entries/:entryId', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(noContentResponse()),
      )

      await client.resumes.removeEntry('r1', 'e1')

      expect(calledUrl(fetchMock)).toBe(
        'http://localhost:3000/api/resumes/r1/entries/e1',
      )
      expect(calledInit(fetchMock).method).toBe('DELETE')
    })

    it('gaps sends GET /api/resumes/:id/gaps', async () => {
      const gapAnalysis = {
        resume_id: 'r1',
        archetype: 'sre',
        target_role: 'SRE',
        target_employer: 'Acme',
        gaps: [],
        coverage_summary: {
          perspectives_included: 5,
          total_approved_perspectives_for_archetype: 10,
          domains_represented: ['cloud', 'linux'],
          domains_missing: ['kubernetes'],
        },
      }
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: gapAnalysis })),
      )

      const result = await client.resumes.gaps('r1')

      expect(calledUrl(fetchMock)).toBe(
        'http://localhost:3000/api/resumes/r1/gaps',
      )
      expect(calledInit(fetchMock).method).toBe('GET')
      expect(result).toEqual({ ok: true, data: gapAnalysis })
    })

  })

  // -----------------------------------------------------------------------
  // organizations
  // -----------------------------------------------------------------------

  describe('contacts', () => {
    beforeEach(() => {
      // The contact link routes answer 201 with an empty body.
      fetchMock.mockImplementation(() => Promise.resolve(new Response(null, { status: 201 })))
    })

    it('linkOrganization POSTs the target id and resolves ok on an empty 201', async () => {
      const result = await client.contacts.linkOrganization('c1', 'o1', 'recruiter')

      expect(result.ok).toBe(true)
      expect(calledUrl(fetchMock)).toBe('http://localhost:3000/api/contacts/c1/organizations')
      expect(calledInit(fetchMock).method).toBe('POST')
      expect(JSON.parse(calledInit(fetchMock).body as string)).toEqual({
        organization_id: 'o1',
        relationship: 'recruiter',
      })
    })

    it('linkJobDescription POSTs the target id and resolves ok on an empty 201', async () => {
      const result = await client.contacts.linkJobDescription('c1', 'j1', 'hiring_manager')

      expect(result.ok).toBe(true)
      expect(calledUrl(fetchMock)).toBe('http://localhost:3000/api/contacts/c1/job-descriptions')
      expect(calledInit(fetchMock).method).toBe('POST')
      expect(JSON.parse(calledInit(fetchMock).body as string)).toEqual({
        job_description_id: 'j1',
        relationship: 'hiring_manager',
      })
    })

    it('linkResume POSTs the target id and resolves ok on an empty 201', async () => {
      const result = await client.contacts.linkResume('c1', 'r1', 'reference')

      expect(result.ok).toBe(true)
      expect(calledUrl(fetchMock)).toBe('http://localhost:3000/api/contacts/c1/resumes')
      expect(calledInit(fetchMock).method).toBe('POST')
      expect(JSON.parse(calledInit(fetchMock).body as string)).toEqual({
        resume_id: 'r1',
        relationship: 'reference',
      })
    })
  })

  describe('organizations', () => {
    it('create sends POST /api/organizations with body', async () => {
      const org = { id: 'org1', name: 'Anthropic', org_type: 'company' }
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: org }, { status: 201 })),
      )

      const result = await client.organizations.create({
        name: 'Anthropic',
        org_type: 'company',
        industry: 'AI',
      })

      expect(calledUrl(fetchMock)).toBe('http://localhost:3000/api/organizations')
      expect(calledInit(fetchMock).method).toBe('POST')
      expect(JSON.parse(calledInit(fetchMock).body as string)).toEqual({
        name: 'Anthropic',
        org_type: 'company',
        industry: 'AI',
      })
      expect(result).toEqual({ ok: true, data: org })
    })

    it('list sends GET /api/organizations with filter', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(
          jsonResponse({
            data: [],
            pagination: { total: 0, offset: 0, limit: 50 },
          }),
        ),
      )

      await client.organizations.list({ org_type: 'company', worked: '1' })

      const url = calledUrl(fetchMock)
      expect(url).toContain('org_type=company')
      expect(url).toContain('worked=1')
    })

    it('get sends GET /api/organizations/:id', async () => {
      const org = { id: 'org1', name: 'Anthropic' }
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: org })),
      )

      const result = await client.organizations.get('org1')

      expect(calledUrl(fetchMock)).toBe('http://localhost:3000/api/organizations/org1')
      expect(result).toEqual({ ok: true, data: org })
    })

    it('update sends PATCH /api/organizations/:id', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: { id: 'org1' } })),
      )

      await client.organizations.update('org1', { name: 'Updated Corp' })

      expect(calledUrl(fetchMock)).toBe('http://localhost:3000/api/organizations/org1')
      expect(calledInit(fetchMock).method).toBe('PATCH')
      expect(JSON.parse(calledInit(fetchMock).body as string)).toEqual({
        name: 'Updated Corp',
      })
    })

    it('delete sends DELETE /api/organizations/:id', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(noContentResponse()),
      )

      await client.organizations.delete('org1')

      expect(calledUrl(fetchMock)).toBe('http://localhost:3000/api/organizations/org1')
      expect(calledInit(fetchMock).method).toBe('DELETE')
    })
  })

  // -----------------------------------------------------------------------
  // notes
  // -----------------------------------------------------------------------

  describe('notes', () => {
    it('create sends POST /api/notes with body', async () => {
      const note = { id: 'n1', title: 'Test', content: 'My note', references: [] }
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: note }, { status: 201 })),
      )

      const result = await client.notes.create({
        title: 'Test',
        content: 'My note',
      })

      expect(calledUrl(fetchMock)).toBe('http://localhost:3000/api/notes')
      expect(calledInit(fetchMock).method).toBe('POST')
      expect(result).toEqual({ ok: true, data: note })
    })

    it('list sends GET /api/notes with search filter', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(
          jsonResponse({
            data: [],
            pagination: { total: 0, offset: 0, limit: 50 },
          }),
        ),
      )

      await client.notes.list({ search: 'kubernetes', limit: 10 })

      const url = calledUrl(fetchMock)
      expect(url).toContain('search=kubernetes')
      expect(url).toContain('limit=10')
    })

    it('get sends GET /api/notes/:id', async () => {
      const note = { id: 'n1', content: 'test', references: [] }
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: note })),
      )

      const result = await client.notes.get('n1')

      expect(calledUrl(fetchMock)).toBe('http://localhost:3000/api/notes/n1')
      expect(result).toEqual({ ok: true, data: note })
    })

    it('update sends PATCH /api/notes/:id', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: { id: 'n1' } })),
      )

      await client.notes.update('n1', { content: 'Updated content' })

      expect(calledUrl(fetchMock)).toBe('http://localhost:3000/api/notes/n1')
      expect(calledInit(fetchMock).method).toBe('PATCH')
    })

    it('delete sends DELETE /api/notes/:id', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(noContentResponse()),
      )

      await client.notes.delete('n1')

      expect(calledUrl(fetchMock)).toBe('http://localhost:3000/api/notes/n1')
      expect(calledInit(fetchMock).method).toBe('DELETE')
    })

    it('addReference sends POST /api/notes/:id/references', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(noContentResponse()),
      )

      await client.notes.addReference('n1', {
        entity_type: 'source',
        entity_id: 's1',
      })

      expect(calledUrl(fetchMock)).toBe(
        'http://localhost:3000/api/notes/n1/references',
      )
      expect(calledInit(fetchMock).method).toBe('POST')
      expect(JSON.parse(calledInit(fetchMock).body as string)).toEqual({
        entity_type: 'source',
        entity_id: 's1',
      })
    })

    it('removeReference sends DELETE /api/notes/:id/references/:entityType/:entityId', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(noContentResponse()),
      )

      await client.notes.removeReference('n1', 'source', 's1')

      expect(calledUrl(fetchMock)).toBe(
        'http://localhost:3000/api/notes/n1/references/source/s1',
      )
      expect(calledInit(fetchMock).method).toBe('DELETE')
    })

    it('getNotesForEntity sends GET /api/notes/by-entity/:type/:id', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: [] })),
      )

      const result = await client.notes.getNotesForEntity('source', 's1')

      expect(calledUrl(fetchMock)).toBe(
        'http://localhost:3000/api/notes/by-entity/source/s1',
      )
      expect(calledInit(fetchMock).method).toBe('GET')
      expect(result.ok).toBe(true)
    })
  })

  // -----------------------------------------------------------------------
  // integrity
  // -----------------------------------------------------------------------

  describe('integrity', () => {
    it('drift sends GET /api/integrity/drift', async () => {
      const report = {
        bullets: [],
        perspectives: [],
        resume_entries: [],
      }
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: report })),
      )

      const result = await client.integrity.drift()

      expect(calledUrl(fetchMock)).toBe(
        'http://localhost:3000/api/integrity/drift',
      )
      expect(calledInit(fetchMock).method).toBe('GET')
      expect(result).toEqual({ ok: true, data: report })
    })

    it('drift returns drifted entities', async () => {
      const report = {
        bullets: [{
          id: 'b1',
          content: 'bullet',
          source_content_snapshot: 'old desc',
          current_source_description: 'new desc',
          source_id: 's1',
          source_title: 'My Source',
        }],
        perspectives: [],
        resume_entries: [],
      }
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: report })),
      )

      const result = await client.integrity.drift()

      expect(result.ok).toBe(true)
      if (result.ok) {
        expect(result.data.bullets.length).toBe(1)
        expect(result.data.bullets[0].source_id).toBe('s1')
      }
    })
  })

  // -----------------------------------------------------------------------
  // review
  // -----------------------------------------------------------------------

  describe('review', () => {
    it('pending sends GET /api/review/pending', async () => {
      const queue = {
        bullets: { count: 2, items: [{ id: 'b1' }, { id: 'b2' }] },
        perspectives: { count: 1, items: [{ id: 'p1' }] },
      }
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: queue })),
      )

      const result = await client.review.pending()

      expect(calledUrl(fetchMock)).toBe(
        'http://localhost:3000/api/review/pending',
      )
      expect(calledInit(fetchMock).method).toBe('GET')
      expect(result).toEqual({ ok: true, data: queue })
    })
  })

  // -----------------------------------------------------------------------
  // Edge cases
  // -----------------------------------------------------------------------

  describe('edge cases', () => {
    it('list with no filter does not append query string', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(
          jsonResponse({
            data: [],
            pagination: { total: 0, offset: 0, limit: 50 },
          }),
        ),
      )

      await client.sources.list()

      expect(calledUrl(fetchMock)).toBe('http://localhost:3000/api/sources')
    })

    it('list with all-undefined filter values does not append query string', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(
          jsonResponse({
            data: [],
            pagination: { total: 0, offset: 0, limit: 50 },
          }),
        ),
      )

      await client.bullets.list({
        source_id: undefined,
        status: undefined,
      })

      expect(calledUrl(fetchMock)).toBe('http://localhost:3000/api/bullets')
    })

    it('error response propagates through resource method', async () => {
      const error = { code: 'NOT_FOUND', message: 'Source not found' }
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ error }, { status: 404 })),
      )

      const result = await client.sources.get('nonexistent')

      expect(result.ok).toBe(false)
      if (!result.ok) {
        expect(result.error).toEqual(error)
      }
    })

    it('network error propagates through resource method', async () => {
      fetchMock.mockImplementation(() =>
        Promise.reject(new TypeError('Failed to fetch')),
      )

      const result = await client.bullets.approve('b1')

      expect(result.ok).toBe(false)
      if (!result.ok) {
        expect(result.error.code).toBe('NETWORK_ERROR')
      }
    })
  })

  // -----------------------------------------------------------------------
  // profile
  // -----------------------------------------------------------------------

  describe('profile', () => {
    it('get sends GET /api/profile', async () => {
      const profileData = { id: 'p1', name: 'Adam', email: 'adam@test.com' }
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: profileData })),
      )

      const result = await client.profile.get()

      expect(calledUrl(fetchMock)).toBe('http://localhost:3000/api/profile')
      expect(calledInit(fetchMock).method).toBe('GET')
      expect(result).toEqual({ ok: true, data: profileData })
    })

    it('update sends PATCH /api/profile with body', async () => {
      const updated = { id: 'p1', name: 'Updated', email: 'new@test.com' }
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: updated })),
      )

      const result = await client.profile.update({ name: 'Updated', email: 'new@test.com' })

      expect(calledUrl(fetchMock)).toBe('http://localhost:3000/api/profile')
      expect(calledInit(fetchMock).method).toBe('PATCH')
      expect(JSON.parse(calledInit(fetchMock).body as string)).toEqual({
        name: 'Updated',
        email: 'new@test.com',
      })
      expect(result).toEqual({ ok: true, data: updated })
    })
  })

  // -----------------------------------------------------------------------
  // summaries
  // -----------------------------------------------------------------------

  describe('summaries', () => {
    it('create sends POST /api/summaries with body', async () => {
      const created = { id: 'sum1', title: 'Test Summary', is_template: false }
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: created }, { status: 201 })),
      )

      const result = await client.summaries.create({ title: 'Test Summary' })

      expect(calledUrl(fetchMock)).toBe('http://localhost:3000/api/summaries')
      expect(calledInit(fetchMock).method).toBe('POST')
      expect(result).toEqual({ ok: true, data: created })
    })

    it('list sends GET /api/summaries with no params', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(
          jsonResponse({ data: [], pagination: { total: 0, offset: 0, limit: 50 } }),
        ),
      )

      await client.summaries.list()

      expect(calledUrl(fetchMock)).toBe('http://localhost:3000/api/summaries')
    })

    it('list with is_template filter sends ?is_template=1', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(
          jsonResponse({ data: [], pagination: { total: 0, offset: 0, limit: 50 } }),
        ),
      )

      await client.summaries.list({ is_template: true })

      expect(calledUrl(fetchMock)).toContain('is_template=1')
    })

    it('list with is_template=false sends ?is_template=0', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(
          jsonResponse({ data: [], pagination: { total: 0, offset: 0, limit: 50 } }),
        ),
      )

      await client.summaries.list({ is_template: false })

      expect(calledUrl(fetchMock)).toContain('is_template=0')
    })

    it('list with pagination sends offset and limit', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(
          jsonResponse({ data: [], pagination: { total: 0, offset: 10, limit: 5 } }),
        ),
      )

      await client.summaries.list({ offset: 10, limit: 5 })

      expect(calledUrl(fetchMock)).toContain('offset=10')
      expect(calledUrl(fetchMock)).toContain('limit=5')
    })

    it('get sends GET /api/summaries/:id', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: { id: 'sum1' } })),
      )

      await client.summaries.get('sum1')

      expect(calledUrl(fetchMock)).toBe('http://localhost:3000/api/summaries/sum1')
      expect(calledInit(fetchMock).method).toBe('GET')
    })

    it('update sends PATCH /api/summaries/:id', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: { id: 'sum1', title: 'Updated' } })),
      )

      await client.summaries.update('sum1', { title: 'Updated' })

      expect(calledUrl(fetchMock)).toBe('http://localhost:3000/api/summaries/sum1')
      expect(calledInit(fetchMock).method).toBe('PATCH')
    })

    it('delete sends DELETE /api/summaries/:id', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(noContentResponse()),
      )

      await client.summaries.delete('sum1')

      expect(calledUrl(fetchMock)).toBe('http://localhost:3000/api/summaries/sum1')
      expect(calledInit(fetchMock).method).toBe('DELETE')
    })

    it('clone sends POST /api/summaries/:id/clone', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: { id: 'sum2', title: 'Copy of Test' } }, { status: 201 })),
      )

      const result = await client.summaries.clone('sum1')

      expect(calledUrl(fetchMock)).toBe('http://localhost:3000/api/summaries/sum1/clone')
      expect(calledInit(fetchMock).method).toBe('POST')
      expect(result).toEqual({ ok: true, data: { id: 'sum2', title: 'Copy of Test' } })
    })
  })
})
