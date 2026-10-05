/**
 * Route tests for the resume tagline endpoints (Phase 92). The contract is
 * packages/core/src/routes/resumes.ts:222-307.
 *
 * Seeds and checks go through ctx.db with plain SQL, so this file runs unchanged
 * against forge-server under `just parity`.
 */
import { describe, test, expect, beforeEach, afterEach } from 'bun:test'
import { createTestApp, apiRequest, type TestContext } from './helpers'
import { seedResume, seedResumeSection } from '../../db/__tests__/helpers'

const MISSING = '00000000-0000-4000-8000-000000000000'
const OLD = '2000-01-01T00:00:00Z'
const GENERATED = 'Cloud Engineer -- kubernetes + aws + docker'
const OVERRIDE = 'Platform engineer for regulated clouds'

let ctx: TestContext
beforeEach(() => { ctx = createTestApp() })
afterEach(() => { ctx.db.close() })

type Row = { generated_tagline: string | null; tagline_override: string | null; updated_at: string }
function row(id: string): Row {
  return ctx.db
    .query('SELECT generated_tagline, tagline_override, updated_at FROM resumes WHERE id = ?')
    .get(id) as Row
}

/** Set both tagline columns, and age updated_at so that a bump is observable. */
function setTaglines(id: string, generated: string | null, override: string | null) {
  ctx.db.run(
    'UPDATE resumes SET generated_tagline = ?, tagline_override = ?, updated_at = ? WHERE id = ?',
    [generated, override, OLD, id],
  )
}

async function expectResumeNotFound(res: Response) {
  expect(res.status).toBe(404)
  expect(await res.json()).toEqual({ error: { code: 'NOT_FOUND', message: 'Resume not found' } })
}

describe('GET /resumes/:id/tagline', () => {
  test('returns the generated tagline when no override is set', async () => {
    const id = seedResume(ctx.db)
    setTaglines(id, GENERATED, null)
    const res = await apiRequest(ctx.app, 'GET', `/resumes/${id}/tagline`)
    expect(res.status).toBe(200)
    expect((await res.json()).data).toEqual({
      generated_tagline: GENERATED, tagline_override: null, resolved: GENERATED, has_override: false,
    })
  })

  test('an override wins resolved and sets has_override', async () => {
    const id = seedResume(ctx.db)
    setTaglines(id, GENERATED, OVERRIDE)
    const { data } = await (await apiRequest(ctx.app, 'GET', `/resumes/${id}/tagline`)).json()
    expect(data).toEqual({
      generated_tagline: GENERATED, tagline_override: OVERRIDE, resolved: OVERRIDE, has_override: true,
    })
  })

  test('with neither column set, resolved is an empty string', async () => {
    const id = seedResume(ctx.db)
    const { data } = await (await apiRequest(ctx.app, 'GET', `/resumes/${id}/tagline`)).json()
    expect(data).toEqual({ generated_tagline: null, tagline_override: null, resolved: '', has_override: false })
  })

  // Pins today's TS behaviour; resu-mx/forge#72 decides whether resolved should trim.
  test('a whitespace-only override is resolved as-is but has_override is false', async () => {
    const id = seedResume(ctx.db)
    setTaglines(id, GENERATED, '   ')
    const { data } = await (await apiRequest(ctx.app, 'GET', `/resumes/${id}/tagline`)).json()
    expect(data.resolved).toBe('   ')
    expect(data.has_override).toBe(false)
  })

  test('does not write: updated_at is unchanged', async () => {
    const id = seedResume(ctx.db)
    setTaglines(id, GENERATED, null)
    await apiRequest(ctx.app, 'GET', `/resumes/${id}/tagline`)
    expect(row(id).updated_at).toBe(OLD)
  })

  test('unknown resume answers 404 NOT_FOUND', async () => {
    await expectResumeNotFound(await apiRequest(ctx.app, 'GET', `/resumes/${MISSING}/tagline`))
  })
})

describe('PATCH /resumes/:id/tagline-override', () => {
  const patch = (id: string, body: unknown) =>
    apiRequest(ctx.app, 'PATCH', `/resumes/${id}/tagline-override`, body)

  test('sets the override verbatim and answers the tagline state', async () => {
    const id = seedResume(ctx.db)
    setTaglines(id, GENERATED, null)
    const res = await patch(id, { content: OVERRIDE })
    expect(res.status).toBe(200)
    expect((await res.json()).data).toEqual({
      generated_tagline: GENERATED, tagline_override: OVERRIDE, resolved: OVERRIDE, has_override: true,
    })
    expect(row(id).tagline_override).toBe(OVERRIDE)
  })

  test('does not trim a non-blank override', async () => {
    const id = seedResume(ctx.db)
    const { data } = await (await patch(id, { content: '  Padded  ' })).json()
    expect(data.tagline_override).toBe('  Padded  ')
    expect(row(id).tagline_override).toBe('  Padded  ')
  })

  for (const [label, body] of [
    ['null', { content: null }],
    ['a missing content', {}],
    ['an empty string', { content: '' }],
    ['whitespace only', { content: '   ' }],
  ] as const) {
    test(`clears the override with ${label}`, async () => {
      const id = seedResume(ctx.db)
      setTaglines(id, GENERATED, OVERRIDE)
      const { data } = await (await patch(id, body)).json()
      expect(data).toEqual({
        generated_tagline: GENERATED, tagline_override: null, resolved: GENERATED, has_override: false,
      })
      expect(row(id).tagline_override).toBeNull()
    })
  }

  test('bumps updated_at and never touches generated_tagline', async () => {
    const id = seedResume(ctx.db)
    setTaglines(id, GENERATED, null)
    await patch(id, { content: OVERRIDE })
    const r = row(id)
    expect(r.generated_tagline).toBe(GENERATED)
    expect(r.updated_at).not.toBe(OLD)
    expect(r.updated_at).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/)
  })

  test('the override reaches the JSON IR and the Markdown export', async () => {
    const id = seedResume(ctx.db)
    seedResumeSection(ctx.db, id, 'Experience', 'experience')
    await patch(id, { content: OVERRIDE })
    const ir = await (await apiRequest(ctx.app, 'GET', `/export/resume/${id}?format=json`)).json()
    expect(ir.data.header.tagline).toBe(OVERRIDE)
    const md = await (await apiRequest(ctx.app, 'GET', `/export/resume/${id}?format=markdown`)).text()
    expect(md).toContain(OVERRIDE)
  })

  test('unknown resume answers 404 NOT_FOUND', async () => {
    await expectResumeNotFound(await patch(MISSING, { content: OVERRIDE }))
  })
})
