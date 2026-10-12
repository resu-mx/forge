/**
 * Route tests for the resume tagline endpoints (Phase 92). The contract is
 * packages/core/src/routes/resumes.ts:222-307.
 *
 * Seeds and checks go through ctx.db with plain SQL, so this file runs unchanged
 * against forge-server under `just parity`.
 */
import { describe, test, expect, beforeEach, afterEach } from 'bun:test'
import { createTestApp, apiRequest, type TestContext } from './helpers'
import { seedResume, seedResumeSection, seedJobDescription, seedSkill } from '../../db/__tests__/helpers'

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

  // resu-mx/forge#72: a blank override (empty or whitespace-only, only possible for a row written
  // directly) is no override. `resolved` and `has_override` agree, and the stored text is untouched.
  test('a whitespace-only override is no override: resolved is the generated tagline', async () => {
    const id = seedResume(ctx.db)
    setTaglines(id, GENERATED, '   ')
    const { data } = await (await apiRequest(ctx.app, 'GET', `/resumes/${id}/tagline`)).json()
    expect(data).toEqual({
      generated_tagline: GENERATED, tagline_override: '   ', resolved: GENERATED, has_override: false,
    })
  })

  test('an empty-string override is no override either', async () => {
    const id = seedResume(ctx.db)
    setTaglines(id, GENERATED, '')
    const { data } = await (await apiRequest(ctx.app, 'GET', `/resumes/${id}/tagline`)).json()
    expect(data.resolved).toBe(GENERATED)
    expect(data.has_override).toBe(false)
  })

  test('a blank override with nothing generated resolves to an empty string, not the whitespace', async () => {
    const id = seedResume(ctx.db)
    setTaglines(id, null, '   ')
    const { data } = await (await apiRequest(ctx.app, 'GET', `/resumes/${id}/tagline`)).json()
    expect(data.resolved).toBe('')
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

describe('POST /resumes/:id/tagline/regenerate', () => {
  const regenerate = (id: string, body?: unknown) =>
    apiRequest(ctx.app, 'POST', `/resumes/${id}/tagline/regenerate`, body)

  /** Link a JD by SQL, so this block doesn't depend on POST /job-descriptions/:id/resumes. */
  function linkJd(resumeId: string, rawText: string): string {
    const jdId = seedJobDescription(ctx.db, { rawText })
    ctx.db.run('INSERT INTO job_description_resumes (job_description_id, resume_id) VALUES (?, ?)', [jdId, resumeId])
    return jdId
  }

  test('regenerates from linked JDs with the target-role prefix and the skill boost', async () => {
    const id = seedResume(ctx.db, { targetRole: 'Senior Platform Engineer' })
    seedSkill(ctx.db, { name: 'Terraform', category: 'tool' })
    linkJd(id, 'Terraform Kubernetes Python Ansible')

    const res = await regenerate(id)
    expect(res.status).toBe(200)
    const { data } = await res.json()
    expect(data).toEqual({
      generated_tagline: 'Senior Platform Engineer -- terraform + ansible + kubernetes',
      has_override: false,
      keywords: [
        { term: 'terraform', score: 2, matchedSkill: true },
        { term: 'ansible', score: 1, matchedSkill: false },
        { term: 'kubernetes', score: 1, matchedSkill: false },
      ],
    })
    expect(row(id).generated_tagline).toBe(data.generated_tagline)
  })

  test('aggregates several linked JDs; ties break by localeCompare', async () => {
    const id = seedResume(ctx.db, { targetRole: 'SRE' })
    linkJd(id, 'kubernetes terraform python')
    linkJd(id, 'kubernetes prometheus grafana')
    const { data } = await (await regenerate(id)).json()
    expect(data.generated_tagline).toBe('SRE -- kubernetes + grafana + prometheus')
    expect(data.keywords.map((k: { term: string }) => k.term)).toEqual(['kubernetes', 'grafana', 'prometheus'])
    expect(data.keywords[0].score).toBeCloseTo(2, 9)
    expect(data.keywords[1].score).toBeCloseTo(Math.log(1.5) + 1, 9)
  })

  test('with no linked JDs it clears generated_tagline and answers ""', async () => {
    const id = seedResume(ctx.db)
    setTaglines(id, 'stale', null)
    const { data } = await (await regenerate(id)).json()
    expect(data).toEqual({ generated_tagline: '', has_override: false, keywords: [] })
    expect(row(id).generated_tagline).toBeNull()
  })

  test('skips linked JDs whose raw_text is empty', async () => {
    const id = seedResume(ctx.db)
    linkJd(id, '')
    const { data } = await (await regenerate(id)).json()
    expect(data.generated_tagline).toBe('')
    expect(row(id).generated_tagline).toBeNull()
  })

  test('keeps an override, reports has_override and still updates generated_tagline', async () => {
    const id = seedResume(ctx.db, { targetRole: 'SRE' })
    setTaglines(id, null, OVERRIDE)
    linkJd(id, 'Kafka Kubernetes Python')
    const { data } = await (await regenerate(id)).json()
    expect(data.has_override).toBe(true)
    expect(data.generated_tagline).toBeTruthy()
    expect(row(id).tagline_override).toBe(OVERRIDE)
    expect(row(id).generated_tagline).toBe(data.generated_tagline)
  })

  test('bumps updated_at', async () => {
    const id = seedResume(ctx.db)
    setTaglines(id, null, null)
    await regenerate(id)
    expect(row(id).updated_at).not.toBe(OLD)
  })

  test('ignores a request body', async () => {
    const id = seedResume(ctx.db)
    expect((await regenerate(id, { anything: true })).status).toBe(200)
  })

  test('unknown resume answers 404 NOT_FOUND', async () => {
    await expectResumeNotFound(await regenerate(MISSING))
  })
})

// resu-mx/forge#72: one rule for blank tagline text. The header the compilers build agrees with
// `resolved` and `has_override`: a blank value is absent, at every step of the fallback chain.
describe('a blank tagline column in the rendered header', () => {
  const headerTagline = async (id: string): Promise<string> => {
    const ir = await (await apiRequest(ctx.app, 'GET', `/export/resume/${id}?format=json`)).json()
    return ir.data.header.tagline
  }

  test('a whitespace-only override falls back to the generated tagline', async () => {
    const id = seedResume(ctx.db, { targetRole: 'Cloud Engineer' })
    seedResumeSection(ctx.db, id, 'Experience', 'experience')
    setTaglines(id, GENERATED, '   ')
    expect(await headerTagline(id)).toBe(GENERATED)
    const md = await (await apiRequest(ctx.app, 'GET', `/export/resume/${id}?format=markdown`)).text()
    expect(md).toContain(GENERATED)
  })

  test('an empty-string override falls back to the generated tagline', async () => {
    const id = seedResume(ctx.db, { targetRole: 'Cloud Engineer' })
    seedResumeSection(ctx.db, id, 'Experience', 'experience')
    setTaglines(id, GENERATED, '')
    expect(await headerTagline(id)).toBe(GENERATED)
  })

  test('a blank generated tagline falls back to the target role', async () => {
    const id = seedResume(ctx.db, { targetRole: 'Cloud Engineer' })
    seedResumeSection(ctx.db, id, 'Experience', 'experience')
    setTaglines(id, '   ', null)
    expect(await headerTagline(id)).toBe('Cloud Engineer')
  })

  test('a real override still wins over the generated tagline', async () => {
    const id = seedResume(ctx.db, { targetRole: 'Cloud Engineer' })
    seedResumeSection(ctx.db, id, 'Experience', 'experience')
    setTaglines(id, GENERATED, OVERRIDE)
    expect(await headerTagline(id)).toBe(OVERRIDE)
  })
})

describe('PATCH /resumes/:id regenerates the tagline when target_role changes', () => {
  /** Link a JD by SQL, so this block doesn't depend on POST /job-descriptions/:id/resumes. */
  function linkJd(resumeId: string, rawText: string): string {
    const jdId = seedJobDescription(ctx.db, { rawText })
    ctx.db.run('INSERT INTO job_description_resumes (job_description_id, resume_id) VALUES (?, ?)', [jdId, resumeId])
    return jdId
  }
  const patchResume = (id: string, body: unknown) => apiRequest(ctx.app, 'PATCH', `/resumes/${id}`, body)

  test('a changed role rebuilds the generated tagline with the new prefix', async () => {
    const id = seedResume(ctx.db, { targetRole: 'Cloud Engineer' })
    linkJd(id, 'Kubernetes Terraform Python')
    await apiRequest(ctx.app, 'POST', `/resumes/${id}/tagline/regenerate`)
    expect(row(id).generated_tagline!.startsWith('Cloud Engineer -- ')).toBe(true)
    ctx.db.run('UPDATE resumes SET tagline_override = ? WHERE id = ?', [OVERRIDE, id])

    const res = await patchResume(id, { target_role: 'Staff SRE' })
    expect(res.status).toBe(200)
    expect((await res.json()).data.target_role).toBe('Staff SRE')

    // (The Resume the route answers with has no tagline columns; read them from the store.)
    const stored = row(id)
    expect(stored.generated_tagline!.startsWith('Staff SRE -- ')).toBe(true)
    // The override is never written by regeneration.
    expect(stored.tagline_override).toBe(OVERRIDE)
  })

  test('the same role, or a different field, leaves the generated tagline alone', async () => {
    const id = seedResume(ctx.db, { targetRole: 'Cloud Engineer' })
    linkJd(id, 'Kubernetes Terraform Python')
    setTaglines(id, 'seeded', null)

    expect((await patchResume(id, { target_role: 'Cloud Engineer' })).status).toBe(200)
    expect(row(id).generated_tagline).toBe('seeded')
    expect((await patchResume(id, { name: 'Renamed' })).status).toBe(200)
    expect(row(id).generated_tagline).toBe('seeded')
  })

  test('a changed role with no linked JD leaves the generated tagline empty', async () => {
    const id = seedResume(ctx.db, { targetRole: 'Cloud Engineer' })
    const res = await patchResume(id, { target_role: 'Staff SRE' })
    expect(res.status).toBe(200)
    expect(row(id).generated_tagline).toBeNull()
  })

  test('an unknown resume answers 404 and writes nothing', async () => {
    const res = await patchResume(MISSING, { target_role: 'Staff SRE' })
    expect(res.status).toBe(404)
  })
})

describe('DELETE /job-descriptions/:id regenerates the taglines of its linked resumes', () => {
  function linkJd(resumeId: string, rawText: string): string {
    const jdId = seedJobDescription(ctx.db, { rawText })
    ctx.db.run('INSERT INTO job_description_resumes (job_description_id, resume_id) VALUES (?, ?)', [jdId, resumeId])
    return jdId
  }
  const regenerate = (id: string) => apiRequest(ctx.app, 'POST', `/resumes/${id}/tagline/regenerate`)

  test('each linked resume is rebuilt from the JDs it still has, and cleared when none remain', async () => {
    const both = seedResume(ctx.db, { targetRole: 'SRE' })
    const only = seedResume(ctx.db, { targetRole: 'SRE' })
    const doomed = linkJd(both, 'kafka kubernetes terraform')
    linkJd(both, 'python django postgres')
    ctx.db.run('INSERT INTO job_description_resumes (job_description_id, resume_id) VALUES (?, ?)', [doomed, only])
    await regenerate(both)
    await regenerate(only)
    const before = row(both).generated_tagline
    expect(before).toContain('kafka')
    expect(row(only).generated_tagline).toContain('kafka')
    ctx.db.run('UPDATE resumes SET tagline_override = ? WHERE id = ?', [OVERRIDE, both])

    const res = await apiRequest(ctx.app, 'DELETE', `/job-descriptions/${doomed}`)
    expect(res.status).toBe(204)

    // `both` still has the second JD: rebuilt from it alone, so it no longer quotes the deleted one.
    const after = row(both)
    expect(after.generated_tagline).not.toContain('kafka')
    expect(after.generated_tagline).toContain('python')
    expect(after.tagline_override).toBe(OVERRIDE)
    // `only` had nothing else: cleared.
    expect(row(only).generated_tagline).toBeNull()
  })

  test('a resume the JD was not linked to is untouched', async () => {
    const linked = seedResume(ctx.db)
    const other = seedResume(ctx.db)
    const jd = linkJd(linked, 'kafka kubernetes')
    setTaglines(other, 'seeded', null)

    expect((await apiRequest(ctx.app, 'DELETE', `/job-descriptions/${jd}`)).status).toBe(204)
    expect(row(other)).toEqual({ generated_tagline: 'seeded', tagline_override: null, updated_at: OLD })
  })

  test('an unknown JD answers 404 and regenerates nothing', async () => {
    const id = seedResume(ctx.db)
    setTaglines(id, 'seeded', null)
    expect((await apiRequest(ctx.app, 'DELETE', `/job-descriptions/${MISSING}`)).status).toBe(404)
    expect(row(id).generated_tagline).toBe('seeded')
  })
})
