/**
 * Source skill routes — the contract for resu-mx/forge#20, #21 and #22:
 *   GET    /sources/:id/skills
 *   POST   /sources/:id/skills
 *   DELETE /sources/:sourceId/skills/:skillId
 *
 * Test names start with the method, so `-t GET`, `-t POST` or `-t DELETE` selects one
 * endpoint. Each test seeds and inspects through `ctx.db` and calls only the endpoint
 * under test, so each endpoint's cases can pass under `just parity` on their own.
 * Assert codes, not messages: Rust renders NOT_FOUND messages differently.
 */
import { describe, test, expect, beforeEach, afterEach } from 'bun:test'
import { createTestApp, apiRequest, type TestContext } from './helpers'
import { seedSource, seedSkill } from '../../db/__tests__/helpers'

const ROW_KEYS = ['category', 'created_at', 'id', 'name']

describe('Source Skill Routes', () => {
  let ctx: TestContext

  beforeEach(() => {
    ctx = createTestApp()
  })

  afterEach(() => {
    ctx.db.close()
  })

  const link = (sourceId: string, skillId: string) =>
    ctx.db.run('INSERT INTO source_skills (source_id, skill_id) VALUES (?, ?)', [sourceId, skillId])
  const linkedIds = (sourceId: string) =>
    (ctx.db.query('SELECT skill_id FROM source_skills WHERE source_id = ?').all(sourceId) as { skill_id: string }[])
      .map((r) => r.skill_id)
  const linkCount = () =>
    (ctx.db.query('SELECT COUNT(*) AS n FROM source_skills').get() as { n: number }).n
  const skillCount = () =>
    (ctx.db.query('SELECT COUNT(*) AS n FROM skills').get() as { n: number }).n
  const skillByName = (name: string) =>
    ctx.db.query('SELECT * FROM skills WHERE name = ? COLLATE NOCASE').all(name) as
      { id: string; name: string; category: string }[]

  // ── GET /sources/:id/skills ────────────────────────────────────────

  test('GET /sources/:id/skills returns full skill rows ordered by name', async () => {
    const sourceId = seedSource(ctx.db)
    link(sourceId, seedSkill(ctx.db, { name: 'Zeta Lang', category: 'language' }))
    link(sourceId, seedSkill(ctx.db, { name: 'Alpha Tool', category: 'tool' }))

    const res = await apiRequest(ctx.app, 'GET', `/sources/${sourceId}/skills`)
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.map((s: { name: string }) => s.name)).toEqual(['Alpha Tool', 'Zeta Lang'])
    expect(Object.keys(body.data[0]).sort()).toEqual(ROW_KEYS)
    expect(body.data[0].category).toBe('tool')
    expect(body.data[0].created_at).toMatch(/^\d{4}-\d{2}-\d{2}T/)
  })

  test('GET /sources/:id/skills for a source with no links returns 200 with []', async () => {
    const sourceId = seedSource(ctx.db)
    const res = await apiRequest(ctx.app, 'GET', `/sources/${sourceId}/skills`)
    expect(res.status).toBe(200)
    expect(await res.json()).toEqual({ data: [] })
  })

  test('GET /sources/:id/skills for an unknown source returns 200 with []', async () => {
    const res = await apiRequest(ctx.app, 'GET', `/sources/${crypto.randomUUID()}/skills`)
    expect(res.status).toBe(200)
    expect((await res.json()).data).toEqual([])
  })

  // ── POST /sources/:id/skills ───────────────────────────────────────

  test('POST /sources/:id/skills with skill_id links it and returns 201 with the full row', async () => {
    const sourceId = seedSource(ctx.db)
    const skillId = seedSkill(ctx.db, { name: 'Go', category: 'language' })

    const res = await apiRequest(ctx.app, 'POST', `/sources/${sourceId}/skills`, { skill_id: skillId })
    expect(res.status).toBe(201)
    const body = await res.json()
    expect(body.data.id).toBe(skillId)
    expect(Object.keys(body.data).sort()).toEqual(ROW_KEYS)
    expect(linkedIds(sourceId)).toEqual([skillId])
  })

  test('POST /sources/:id/skills with skill_id twice returns 201 both times and keeps one link', async () => {
    const sourceId = seedSource(ctx.db)
    const skillId = seedSkill(ctx.db, { name: 'Go', category: 'language' })

    for (let i = 0; i < 2; i++) {
      const res = await apiRequest(ctx.app, 'POST', `/sources/${sourceId}/skills`, { skill_id: skillId })
      expect(res.status).toBe(201)
      expect((await res.json()).data.id).toBe(skillId)
    }
    expect(linkedIds(sourceId)).toEqual([skillId])
  })

  test('POST /sources/:id/skills with skill_id and name uses skill_id and creates no skill', async () => {
    const sourceId = seedSource(ctx.db)
    const skillId = seedSkill(ctx.db, { name: 'Go', category: 'language' })

    const res = await apiRequest(ctx.app, 'POST', `/sources/${sourceId}/skills`, {
      skill_id: skillId,
      name: 'Ignored Name Xyz',
    })
    expect(res.status).toBe(201)
    expect((await res.json()).data.id).toBe(skillId)
    expect(skillByName('Ignored Name Xyz')).toEqual([])
    expect(linkedIds(sourceId)).toEqual([skillId])
  })

  test('POST /sources/:id/skills with name creates the skill with its first letter capitalised', async () => {
    const sourceId = seedSource(ctx.db)
    for (const [input, stored] of [['kubernetes', 'Kubernetes'], ['sAFe', 'SAFe']]) {
      const res = await apiRequest(ctx.app, 'POST', `/sources/${sourceId}/skills`, { name: input })
      expect(res.status).toBe(201)
      const { data } = await res.json()
      expect(data.name).toBe(stored)
      expect(linkedIds(sourceId)).toContain(data.id)
    }
  })

  test('POST /sources/:id/skills with a name matching an existing skill case-insensitively reuses it', async () => {
    const sourceId = seedSource(ctx.db)
    const skillId = seedSkill(ctx.db, { name: 'Zeta Lang', category: 'language' })
    const before = skillCount()

    const res = await apiRequest(ctx.app, 'POST', `/sources/${sourceId}/skills`, {
      name: 'zeta lang',
      category: 'tool',
    })
    expect(res.status).toBe(201)
    const { data } = await res.json()
    expect(data.id).toBe(skillId)
    expect(data.name).toBe('Zeta Lang')
    expect(data.category).toBe('language')
    expect(skillCount()).toBe(before)
    expect(linkedIds(sourceId)).toEqual([skillId])
  })

  test('POST /sources/:id/skills with name maps category: tool stays, ai_ml and bogus become other', async () => {
    const sourceId = seedSource(ctx.db)
    const cases: [string, string | undefined, string][] = [
      ['Cat Tool Probe', 'tool', 'tool'],
      ['Cat Aiml Probe', 'ai_ml', 'other'],
      ['Cat Bogus Probe', 'bogus', 'other'],
    ]
    for (const [name, category, expected] of cases) {
      const res = await apiRequest(ctx.app, 'POST', `/sources/${sourceId}/skills`, { name, category })
      expect(res.status).toBe(201)
      expect((await res.json()).data.category).toBe(expected)
    }
  })

  test('POST /sources/:id/skills with an empty, blank or null body value returns 400 VALIDATION_ERROR', async () => {
    const sourceId = seedSource(ctx.db)
    const before = skillCount()
    for (const payload of [{}, { name: '   ' }, { skill_id: null }]) {
      const res = await apiRequest(ctx.app, 'POST', `/sources/${sourceId}/skills`, payload)
      expect(res.status).toBe(400)
      expect((await res.json()).error.code).toBe('VALIDATION_ERROR')
    }
    expect(skillCount()).toBe(before)
    expect(linkedIds(sourceId)).toEqual([])
  })

  test('POST /sources/:id/skills with skill_id for an unknown source or skill returns 404 and links nothing', async () => {
    const sourceId = seedSource(ctx.db)
    const skillId = seedSkill(ctx.db, { name: 'Go', category: 'language' })
    const before = linkCount()

    const unknownSource = await apiRequest(ctx.app, 'POST', `/sources/${crypto.randomUUID()}/skills`, {
      skill_id: skillId,
    })
    expect(unknownSource.status).toBe(404)
    expect((await unknownSource.json()).error.code).toBe('NOT_FOUND')

    const unknownSkill = await apiRequest(ctx.app, 'POST', `/sources/${sourceId}/skills`, {
      skill_id: crypto.randomUUID(),
    })
    expect(unknownSkill.status).toBe(404)
    expect((await unknownSkill.json()).error.code).toBe('NOT_FOUND')

    expect(linkCount()).toBe(before)
  })

  test('POST /sources/:id/skills with name for an unknown source returns 404 and creates no skill', async () => {
    const before = skillCount()
    const res = await apiRequest(ctx.app, 'POST', `/sources/${crypto.randomUUID()}/skills`, {
      name: 'Orphan Check',
    })
    expect(res.status).toBe(404)
    expect((await res.json()).error.code).toBe('NOT_FOUND')
    expect(skillCount()).toBe(before)
    expect(skillByName('Orphan Check')).toEqual([])
  })

  // ── DELETE /sources/:sourceId/skills/:skillId ──────────────────────

  test('DELETE /sources/:sourceId/skills/:skillId unlinks and returns 204 without deleting the skill', async () => {
    const a = seedSource(ctx.db, { title: 'A' })
    const b = seedSource(ctx.db, { title: 'B' })
    const skillId = seedSkill(ctx.db, { name: 'Rust', category: 'language' })
    link(a, skillId)
    link(b, skillId)

    const res = await apiRequest(ctx.app, 'DELETE', `/sources/${a}/skills/${skillId}`)
    expect(res.status).toBe(204)
    expect(await res.text()).toBe('')
    expect(linkedIds(a)).toEqual([])
    expect(linkedIds(b)).toEqual([skillId])
    expect(ctx.db.query('SELECT 1 FROM skills WHERE id = ?').get(skillId)).not.toBeNull()
  })

  test('DELETE /sources/:sourceId/skills/:skillId a second time returns 404 NOT_FOUND', async () => {
    const sourceId = seedSource(ctx.db)
    const skillId = seedSkill(ctx.db, { name: 'Rust', category: 'language' })
    link(sourceId, skillId)

    const first = await apiRequest(ctx.app, 'DELETE', `/sources/${sourceId}/skills/${skillId}`)
    expect(first.status).toBe(204)
    const second = await apiRequest(ctx.app, 'DELETE', `/sources/${sourceId}/skills/${skillId}`)
    expect(second.status).toBe(404)
    expect((await second.json()).error.code).toBe('NOT_FOUND')
  })

  test('DELETE /sources/:sourceId/skills/:skillId with an unknown source or skill returns 404 NOT_FOUND', async () => {
    const sourceId = seedSource(ctx.db)
    const skillId = seedSkill(ctx.db, { name: 'Rust', category: 'language' })
    link(sourceId, skillId)

    const unknownSource = await apiRequest(ctx.app, 'DELETE', `/sources/${crypto.randomUUID()}/skills/${skillId}`)
    expect(unknownSource.status).toBe(404)
    expect((await unknownSource.json()).error.code).toBe('NOT_FOUND')

    const unknownSkill = await apiRequest(ctx.app, 'DELETE', `/sources/${sourceId}/skills/${crypto.randomUUID()}`)
    expect(unknownSkill.status).toBe(404)
    expect((await unknownSkill.json()).error.code).toBe('NOT_FOUND')

    expect(linkedIds(sourceId)).toEqual([skillId])
  })
})
