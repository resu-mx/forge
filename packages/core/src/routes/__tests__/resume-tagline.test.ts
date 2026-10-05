/**
 * Route tests for the resume tagline endpoints (Phase 92). The contract is
 * packages/core/src/routes/resumes.ts:222-307.
 *
 * Seeds and checks go through ctx.db with plain SQL, so this file runs unchanged
 * against forge-server under `just parity`.
 */
import { describe, test, expect, beforeEach, afterEach } from 'bun:test'
import { createTestApp, apiRequest, type TestContext } from './helpers'
import { seedResume } from '../../db/__tests__/helpers'

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
