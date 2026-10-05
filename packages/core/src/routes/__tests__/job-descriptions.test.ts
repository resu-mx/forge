/**
 * Tests for job description API routes.
 */

import { describe, test, expect, beforeEach, afterEach } from 'bun:test'
import { createTestApp, apiRequest, type TestContext } from './helpers'
import { seedOrganization, seedJobDescription } from '../../db/__tests__/helpers'

let ctx: TestContext

beforeEach(() => {
  ctx = createTestApp()
})

afterEach(() => {
  ctx.db.close()
})

describe('Job Description Routes', () => {
  // ── POST /job-descriptions ──────────────────────────────────────────

  test('POST creates a job description and returns 201', async () => {
    const res = await apiRequest(ctx.app, 'POST', '/job-descriptions', {
      title: 'Security Engineer',
      raw_text: 'Full job description text...',
    })
    expect(res.status).toBe(201)
    const body = await res.json()
    expect(body.data.title).toBe('Security Engineer')
    expect(body.data.status).toBe('discovered')
    expect(body.data.id).toHaveLength(36)
  })

  test('POST with organization includes organization_name', async () => {
    const orgId = seedOrganization(ctx.db, { name: 'Anthropic' })
    const res = await apiRequest(ctx.app, 'POST', '/job-descriptions', {
      title: 'ML Engineer',
      raw_text: 'text',
      organization_id: orgId,
    })
    expect(res.status).toBe(201)
    const body = await res.json()
    expect(body.data.organization_name).toBe('Anthropic')
  })

  test('POST rejects empty title with 400', async () => {
    const res = await apiRequest(ctx.app, 'POST', '/job-descriptions', {
      title: '',
      raw_text: 'text',
    })
    expect(res.status).toBe(400)
    const body = await res.json()
    expect(body.error.code).toBe('VALIDATION_ERROR')
  })

  test('POST rejects empty raw_text with 400', async () => {
    const res = await apiRequest(ctx.app, 'POST', '/job-descriptions', {
      title: 'Valid',
      raw_text: '',
    })
    expect(res.status).toBe(400)
  })

  // ── GET /job-descriptions ───────────────────────────────────────────

  test('GET list returns paginated envelope', async () => {
    seedJobDescription(ctx.db, { title: 'JD1' })
    seedJobDescription(ctx.db, { title: 'JD2' })

    const res = await apiRequest(ctx.app, 'GET', '/job-descriptions')
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data).toBeArray()
    expect(body.data).toHaveLength(2)
    expect(body.pagination.total).toBe(2)
    expect(body.pagination.offset).toBe(0)
    expect(body.pagination.limit).toBe(50)
  })

  test('GET list filters by status', async () => {
    seedJobDescription(ctx.db, { title: 'A', status: 'applied' })
    seedJobDescription(ctx.db, { title: 'B', status: 'discovered' })

    const res = await apiRequest(
      ctx.app,
      'GET',
      '/job-descriptions?status=applied',
    )
    const body = await res.json()
    expect(body.data).toHaveLength(1)
    expect(body.data[0].title).toBe('A')
  })

  test('GET list filters by organization_id', async () => {
    const orgId = seedOrganization(ctx.db, { name: 'FilterOrg' })
    seedJobDescription(ctx.db, {
      title: 'Match',
      organizationId: orgId,
    })
    seedJobDescription(ctx.db, { title: 'NoMatch' })

    const res = await apiRequest(
      ctx.app,
      'GET',
      `/job-descriptions?organization_id=${orgId}`,
    )
    const body = await res.json()
    expect(body.data).toHaveLength(1)
    expect(body.data[0].title).toBe('Match')
  })

  test('GET list includes organization_name per item', async () => {
    const orgId = seedOrganization(ctx.db, { name: 'OrgName' })
    seedJobDescription(ctx.db, {
      title: 'With Org',
      organizationId: orgId,
    })
    seedJobDescription(ctx.db, { title: 'No Org' })

    const res = await apiRequest(ctx.app, 'GET', '/job-descriptions')
    const body = await res.json()
    const withOrg = body.data.find(
      (jd: any) => jd.title === 'With Org',
    )
    const noOrg = body.data.find(
      (jd: any) => jd.title === 'No Org',
    )
    expect(withOrg.organization_name).toBe('OrgName')
    expect(noOrg.organization_name).toBeNull()
  })

  // ── GET /job-descriptions/:id ───────────────────────────────────────

  test('GET single returns job description with org name', async () => {
    const orgId = seedOrganization(ctx.db, { name: 'TestCorp' })
    const jdId = seedJobDescription(ctx.db, {
      title: 'Target JD',
      organizationId: orgId,
    })

    const res = await apiRequest(
      ctx.app,
      'GET',
      `/job-descriptions/${jdId}`,
    )
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.title).toBe('Target JD')
    expect(body.data.organization_name).toBe('TestCorp')
  })

  test('GET single returns 404 for nonexistent id', async () => {
    const res = await apiRequest(
      ctx.app,
      'GET',
      `/job-descriptions/${crypto.randomUUID()}`,
    )
    expect(res.status).toBe(404)
  })

  // ── PATCH /job-descriptions/:id ─────────────────────────────────────

  test('PATCH updates fields and returns updated JD', async () => {
    const jdId = seedJobDescription(ctx.db, { title: 'Old' })

    const res = await apiRequest(
      ctx.app,
      'PATCH',
      `/job-descriptions/${jdId}`,
      { title: 'New', status: 'applied' },
    )
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.title).toBe('New')
    expect(body.data.status).toBe('applied')
  })

  test('PATCH returns 404 for nonexistent id', async () => {
    const res = await apiRequest(
      ctx.app,
      'PATCH',
      `/job-descriptions/${crypto.randomUUID()}`,
      { title: 'X' },
    )
    expect(res.status).toBe(404)
  })

  test('PATCH with organization_id: null clears the organization', async () => {
    const orgId = seedOrganization(ctx.db, { name: 'ClearMe' })
    const jdId = seedJobDescription(ctx.db, {
      title: 'Has Org',
      organizationId: orgId,
    })

    const res = await apiRequest(
      ctx.app,
      'PATCH',
      `/job-descriptions/${jdId}`,
      { organization_id: null },
    )
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.organization_id).toBeNull()
  })

  test('PATCH rejects invalid status with 400', async () => {
    const jdId = seedJobDescription(ctx.db)

    const res = await apiRequest(
      ctx.app,
      'PATCH',
      `/job-descriptions/${jdId}`,
      { status: 'invalid' },
    )
    expect(res.status).toBe(400)
  })

  // ── DELETE /job-descriptions/:id ────────────────────────────────────

  test('DELETE returns 204', async () => {
    const jdId = seedJobDescription(ctx.db)

    const res = await apiRequest(
      ctx.app,
      'DELETE',
      `/job-descriptions/${jdId}`,
    )
    expect(res.status).toBe(204)

    // Verify deleted
    const get = await apiRequest(
      ctx.app,
      'GET',
      `/job-descriptions/${jdId}`,
    )
    expect(get.status).toBe(404)
  })

  test('DELETE returns 404 for nonexistent id', async () => {
    const res = await apiRequest(
      ctx.app,
      'DELETE',
      `/job-descriptions/${crypto.randomUUID()}`,
    )
    expect(res.status).toBe(404)
  })

  // ── GET /job-descriptions/:id/skills ──────────────────────────────

  test('GET skills returns empty array for JD with no skills', async () => {
    const jdId = seedJobDescription(ctx.db)
    const res = await apiRequest(ctx.app, 'GET', `/job-descriptions/${jdId}/skills`)
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data).toBeArray()
    expect(body.data).toHaveLength(0)
  })

  test('GET skills returns linked skills sorted by name', async () => {
    const jdId = seedJobDescription(ctx.db)
    const skillId1 = crypto.randomUUID()
    const skillId2 = crypto.randomUUID()
    ctx.db.run("INSERT INTO skills (id, name, category) VALUES (?, ?, ?)", [skillId1, 'Terraform', 'methodology'])
    ctx.db.run("INSERT INTO skills (id, name, category) VALUES (?, ?, ?)", [skillId2, 'Kubernetes', 'methodology'])
    ctx.db.run("INSERT INTO job_description_skills (job_description_id, skill_id) VALUES (?, ?)", [jdId, skillId1])
    ctx.db.run("INSERT INTO job_description_skills (job_description_id, skill_id) VALUES (?, ?)", [jdId, skillId2])

    const res = await apiRequest(ctx.app, 'GET', `/job-descriptions/${jdId}/skills`)
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data).toHaveLength(2)
    expect(body.data[0].name).toBe('Kubernetes')
    expect(body.data[1].name).toBe('Terraform')
  })

  // ── POST /job-descriptions/:id/skills ─────────────────────────────

  test('POST skill with skill_id links existing skill and returns 201', async () => {
    const jdId = seedJobDescription(ctx.db)
    const skillId = crypto.randomUUID()
    ctx.db.run("INSERT INTO skills (id, name, category) VALUES (?, ?, ?)", [skillId, 'Python', 'language'])

    const res = await apiRequest(ctx.app, 'POST', `/job-descriptions/${jdId}/skills`, { skill_id: skillId })
    expect(res.status).toBe(201)
    const body = await res.json()
    expect(body.data.name).toBe('Python')

    // Verify link exists
    const getRes = await apiRequest(ctx.app, 'GET', `/job-descriptions/${jdId}/skills`)
    const getBody = await getRes.json()
    expect(getBody.data).toHaveLength(1)
  })

  test('POST skill with duplicate skill_id is idempotent', async () => {
    const jdId = seedJobDescription(ctx.db)
    const skillId = crypto.randomUUID()
    ctx.db.run("INSERT INTO skills (id, name, category) VALUES (?, ?, ?)", [skillId, 'Go', 'language'])

    await apiRequest(ctx.app, 'POST', `/job-descriptions/${jdId}/skills`, { skill_id: skillId })
    const res = await apiRequest(ctx.app, 'POST', `/job-descriptions/${jdId}/skills`, { skill_id: skillId })
    expect(res.status).toBe(201)

    const getRes = await apiRequest(ctx.app, 'GET', `/job-descriptions/${jdId}/skills`)
    const getBody = await getRes.json()
    expect(getBody.data).toHaveLength(1)
  })

  test('POST skill with name creates new skill and links it', async () => {
    const jdId = seedJobDescription(ctx.db)

    const res = await apiRequest(ctx.app, 'POST', `/job-descriptions/${jdId}/skills`, { name: 'python' })
    expect(res.status).toBe(201)
    const body = await res.json()
    expect(body.data.name).toBe('Python') // capitalizeFirst
    expect(body.data.id).toHaveLength(36)

    const getRes = await apiRequest(ctx.app, 'GET', `/job-descriptions/${jdId}/skills`)
    const getBody = await getRes.json()
    expect(getBody.data).toHaveLength(1)
  })

  test('POST skill with name deduplicates case-insensitively', async () => {
    const jdId = seedJobDescription(ctx.db)
    const skillId = crypto.randomUUID()
    ctx.db.run("INSERT INTO skills (id, name, category) VALUES (?, ?, ?)", [skillId, 'Python', 'language'])

    const res = await apiRequest(ctx.app, 'POST', `/job-descriptions/${jdId}/skills`, { name: 'python' })
    expect(res.status).toBe(201)
    const body = await res.json()
    expect(body.data.id).toBe(skillId) // reuses existing
  })

  test('POST skill with neither skill_id nor name returns 400', async () => {
    const jdId = seedJobDescription(ctx.db)
    const res = await apiRequest(ctx.app, 'POST', `/job-descriptions/${jdId}/skills`, {})
    expect(res.status).toBe(400)
    const body = await res.json()
    expect(body.error.code).toBe('VALIDATION_ERROR')
  })

  test('POST skill with name for unknown JD returns 404 and creates no skill', async () => {
    ctx.db.run("INSERT INTO skills (id, name, category) VALUES (?, ?, ?)", [crypto.randomUUID(), 'Python', 'language'])
    const unknownJd = crypto.randomUUID()

    for (const name of ['Zebra Mesh', 'python']) {
      const res = await apiRequest(ctx.app, 'POST', `/job-descriptions/${unknownJd}/skills`, { name })
      expect(res.status).toBe(404)
      const body = await res.json()
      expect(body.error.code).toBe('NOT_FOUND')
    }
    const row = ctx.db.query('SELECT COUNT(*) AS cnt FROM skills WHERE name = ? COLLATE NOCASE').get('Zebra Mesh') as any
    expect(row.cnt).toBe(0)
  })

  test('POST skill with skill_id returns 404 for an unknown JD or skill', async () => {
    const jdId = seedJobDescription(ctx.db)
    const skillId = crypto.randomUUID()
    ctx.db.run("INSERT INTO skills (id, name, category) VALUES (?, ?, ?)", [skillId, 'Go', 'language'])

    const unknownJd = await apiRequest(ctx.app, 'POST', `/job-descriptions/${crypto.randomUUID()}/skills`, { skill_id: skillId })
    expect(unknownJd.status).toBe(404)
    const unknownSkill = await apiRequest(ctx.app, 'POST', `/job-descriptions/${jdId}/skills`, { skill_id: crypto.randomUUID() })
    expect(unknownSkill.status).toBe(404)
    expect((await unknownSkill.json()).error.code).toBe('NOT_FOUND')
  })

  test('POST skill with name keeps a listed category and maps others to other', async () => {
    const jdId = seedJobDescription(ctx.db)
    const cases: [name: string, category: string, expected: string][] = [
      ['terraform', 'tool', 'tool'],
      ['vector stores', 'ai_ml', 'other'], // a valid skill_categories slug, not in the TS list
      ['cissp', 'certification', 'other'], // an extraction-prompt category, not a slug
    ]
    for (const [name, category, expected] of cases) {
      const res = await apiRequest(ctx.app, 'POST', `/job-descriptions/${jdId}/skills`, { name, category })
      expect(res.status).toBe(201)
      const body = await res.json()
      expect(body.data.category).toBe(expected)
    }
  })

  // ── DELETE /job-descriptions/:jdId/skills/:skillId ────────────────

  test('DELETE skill removes junction row and returns 204', async () => {
    const jdId = seedJobDescription(ctx.db)
    const skillId = crypto.randomUUID()
    ctx.db.run("INSERT INTO skills (id, name, category) VALUES (?, ?, ?)", [skillId, 'Rust', 'language'])
    ctx.db.run("INSERT INTO job_description_skills (job_description_id, skill_id) VALUES (?, ?)", [jdId, skillId])

    const res = await apiRequest(ctx.app, 'DELETE', `/job-descriptions/${jdId}/skills/${skillId}`)
    expect(res.status).toBe(204)

    const getRes = await apiRequest(ctx.app, 'GET', `/job-descriptions/${jdId}/skills`)
    const getBody = await getRes.json()
    expect(getBody.data).toHaveLength(0)
  })

  test('DELETE skill for nonexistent link returns 204 (idempotent)', async () => {
    const jdId = seedJobDescription(ctx.db)
    const res = await apiRequest(ctx.app, 'DELETE', `/job-descriptions/${jdId}/skills/${crypto.randomUUID()}`)
    expect(res.status).toBe(204)
  })

  // ── Cascade tests ─────────────────────────────────────────────────

  test('Deleting a JD cascades to job_description_skills', async () => {
    const jdId = seedJobDescription(ctx.db)
    const skillId = crypto.randomUUID()
    ctx.db.run("INSERT INTO skills (id, name, category) VALUES (?, ?, ?)", [skillId, 'AWS', 'platform'])
    ctx.db.run("INSERT INTO job_description_skills (job_description_id, skill_id) VALUES (?, ?)", [jdId, skillId])

    await apiRequest(ctx.app, 'DELETE', `/job-descriptions/${jdId}`)

    const row = ctx.db.query('SELECT COUNT(*) AS cnt FROM job_description_skills WHERE job_description_id = ?').get(jdId) as any
    expect(row.cnt).toBe(0)
  })

  test('Deleting a skill cascades to job_description_skills', async () => {
    const jdId = seedJobDescription(ctx.db)
    const skillId = crypto.randomUUID()
    ctx.db.run("INSERT INTO skills (id, name, category) VALUES (?, ?, ?)", [skillId, 'Docker', 'methodology'])
    ctx.db.run("INSERT INTO job_description_skills (job_description_id, skill_id) VALUES (?, ?)", [jdId, skillId])

    ctx.db.run('DELETE FROM skills WHERE id = ?', [skillId])

    const row = ctx.db.query('SELECT COUNT(*) AS cnt FROM job_description_skills WHERE skill_id = ?').get(skillId) as any
    expect(row.cnt).toBe(0)
  })

  // ── POST /job-descriptions/lookup-by-url ───────────────────────────

  test('POST lookup-by-url returns JD when URL matches', async () => {
    seedJobDescription(ctx.db, {
      title: 'ML Engineer',
      url: 'https://example.com/jobs/ml-eng',
      rawText: 'Build ML systems',
    })

    const res = await apiRequest(ctx.app, 'POST', '/job-descriptions/lookup-by-url', {
      url: 'https://example.com/jobs/ml-eng',
    })
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.title).toBe('ML Engineer')
    expect(body.data.url).toBe('https://example.com/jobs/ml-eng')
  })

  test('POST lookup-by-url returns 404 when URL not found', async () => {
    const res = await apiRequest(ctx.app, 'POST', '/job-descriptions/lookup-by-url', {
      url: 'https://example.com/jobs/nonexistent',
    })
    expect(res.status).toBe(404)
    const body = await res.json()
    expect(body.error.code).toBe('NOT_FOUND')
  })

  test('POST lookup-by-url returns 400 for empty URL', async () => {
    const res = await apiRequest(ctx.app, 'POST', '/job-descriptions/lookup-by-url', {
      url: '',
    })
    expect(res.status).toBe(400)
    const body = await res.json()
    expect(body.error.code).toBe('VALIDATION_ERROR')
  })

  test('POST lookup-by-url returns 400 for missing URL', async () => {
    const res = await apiRequest(ctx.app, 'POST', '/job-descriptions/lookup-by-url', {})
    expect(res.status).toBe(400)
    const body = await res.json()
    expect(body.error.code).toBe('VALIDATION_ERROR')
  })

  test('POST lookup-by-url hydrates organization_name', async () => {
    const orgId = seedOrganization(ctx.db, { name: 'Anthropic' })
    seedJobDescription(ctx.db, {
      title: 'SWE',
      url: 'https://anthropic.com/careers/swe',
      organizationId: orgId,
    })

    const res = await apiRequest(ctx.app, 'POST', '/job-descriptions/lookup-by-url', {
      url: 'https://anthropic.com/careers/swe',
    })
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.organization_name).toBe('Anthropic')
  })

  // ── Sort order test ───────────────────────────────────────────────

  test('GET list returns JDs ordered by updated_at DESC', async () => {
    const jd1 = seedJobDescription(ctx.db, { title: 'First' })
    const jd2 = seedJobDescription(ctx.db, { title: 'Second' })

    // Update jd1 so it has a newer updated_at
    ctx.db.run(
      "UPDATE job_descriptions SET title = 'First Updated', updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?",
      [jd1],
    )

    const res = await apiRequest(ctx.app, 'GET', '/job-descriptions')
    const body = await res.json()
    expect(body.data[0].title).toBe('First Updated')
  })
})
