import { describe, test, expect, beforeEach, afterEach } from 'bun:test'
import { Database } from 'bun:sqlite'
import { createTestApp, apiRequest, type TestContext } from './helpers'
import { seedSource, seedBullet, seedPerspective, seedResume, seedResumeEntry, seedResumeSection, seedSkill, seedResumeSkill } from '../../db/__tests__/helpers'

describe('Resume Routes', () => {
  let ctx: TestContext

  beforeEach(() => {
    ctx = createTestApp()
  })

  afterEach(() => {
    ctx.db.close()
  })

  // ── POST /resumes ─────────────────────────────────────────────────

  test('POST /resumes creates a resume and returns 201', async () => {
    const res = await apiRequest(ctx.app, 'POST', '/resumes', {
      name: 'AI Engineer Resume',
      target_role: 'AI Engineer',
      target_employer: 'Anthropic',
      archetype: 'agentic-ai',
    })
    expect(res.status).toBe(201)
    const body = await res.json()
    expect(body.data).toBeDefined()
    expect(body.data.id).toHaveLength(36)
    expect(body.data.name).toBe('AI Engineer Resume')
    expect(body.data.archetype).toBe('agentic-ai')
  })

  // ── GET /resumes ──────────────────────────────────────────────────

  test('GET /resumes returns 200 with pagination', async () => {
    seedResume(ctx.db, { name: 'Resume A' })
    seedResume(ctx.db, { name: 'Resume B' })

    const res = await apiRequest(ctx.app, 'GET', '/resumes')
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data).toBeArray()
    expect(body.data.length).toBe(2)
    expect(body.pagination).toBeDefined()
    expect(body.pagination.total).toBe(2)
  })

  // ── GET /resumes/:id ──────────────────────────────────────────────

  test('GET /resumes/:id returns 200 with sections', async () => {
    const resumeId = seedResume(ctx.db)
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }])
    const perspId = seedPerspective(ctx.db, bulletId)
    const secId = seedResumeSection(ctx.db, resumeId, 'Experience', 'experience')
    seedResumeEntry(ctx.db, secId, { perspectiveId: perspId })

    const res = await apiRequest(ctx.app, 'GET', `/resumes/${resumeId}`)
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.id).toBe(resumeId)
    expect(body.data.sections).toBeDefined()
    expect(body.data.sections).toBeArray()
    expect(body.data.sections.length).toBe(1)
    expect(body.data.sections[0].entry_type).toBe('experience')
    expect(body.data.sections[0].entries).toBeArray()
    expect(body.data.sections[0].entries.length).toBe(1)
  })

  // ── POST /resumes/:id/entries ─────────────────────────────────────

  test('POST /resumes/:id/entries with approved perspective returns 201', async () => {
    const resumeId = seedResume(ctx.db)
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }])
    const perspId = seedPerspective(ctx.db, bulletId, { status: 'approved' })
    const secId = seedResumeSection(ctx.db, resumeId, 'Experience', 'experience')

    const res = await apiRequest(ctx.app, 'POST', `/resumes/${resumeId}/entries`, {
      perspective_id: perspId,
      section_id: secId,
      position: 0,
    })
    expect(res.status).toBe(201)
    const body = await res.json()
    expect(body.data).toBeDefined()
    expect(body.data.perspective_id).toBe(perspId)
  })

  test('POST /resumes/:id/entries with non-approved perspective returns 400', async () => {
    const resumeId = seedResume(ctx.db)
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }])
    const perspId = seedPerspective(ctx.db, bulletId, { status: 'in_review' })
    const secId = seedResumeSection(ctx.db, resumeId, 'Experience', 'experience')

    const res = await apiRequest(ctx.app, 'POST', `/resumes/${resumeId}/entries`, {
      perspective_id: perspId,
      section_id: secId,
      position: 0,
    })
    expect(res.status).toBe(400)
    const body = await res.json()
    expect(body.error.code).toBe('VALIDATION_ERROR')
    expect(body.error.message).toContain('approved')
  })

  test('POST /resumes/:id/entries with a draft perspective gives the approved-only message', async () => {
    const resumeId = seedResume(ctx.db)
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }])
    const perspId = seedPerspective(ctx.db, bulletId, { status: 'draft' })
    const secId = seedResumeSection(ctx.db, resumeId, 'Experience', 'experience')

    const res = await apiRequest(ctx.app, 'POST', `/resumes/${resumeId}/entries`, {
      perspective_id: perspId,
      section_id: secId,
    })
    expect(res.status).toBe(400)
    const body = await res.json()
    expect(body.error.code).toBe('VALIDATION_ERROR')
    // toContain, not toBe: forge-server renders ForgeError's Display
    // ("validation error: <msg>"), a prefix gap that is not part of this case.
    expect(body.error.message).toContain('Only approved perspectives can be added to resumes')
  })

  test('POST /resumes/:id/entries with an archived perspective says to unarchive it', async () => {
    const resumeId = seedResume(ctx.db)
    const sourceId = seedSource(ctx.db)
    const bulletId = seedBullet(ctx.db, [{ id: sourceId }])
    const perspId = seedPerspective(ctx.db, bulletId, { status: 'archived' })
    const secId = seedResumeSection(ctx.db, resumeId, 'Experience', 'experience')

    const res = await apiRequest(ctx.app, 'POST', `/resumes/${resumeId}/entries`, {
      perspective_id: perspId,
      section_id: secId,
    })
    expect(res.status).toBe(400)
    const body = await res.json()
    expect(body.error.code).toBe('VALIDATION_ERROR')
    expect(body.error.message).toContain('Cannot add archived perspective to resume. Unarchive it first.')
  })

  // ── perspective_content_snapshot on add / update ──────────────────

  describe('perspective_content_snapshot', () => {
    const PERSPECTIVE_TEXT = 'Perspective text frozen at add time'
    let resumeId: string
    let secId: string
    let perspId: string

    beforeEach(() => {
      resumeId = seedResume(ctx.db)
      const sourceId = seedSource(ctx.db)
      const bulletId = seedBullet(ctx.db, [{ id: sourceId }])
      perspId = seedPerspective(ctx.db, bulletId, { status: 'approved', content: PERSPECTIVE_TEXT })
      secId = seedResumeSection(ctx.db, resumeId, 'Experience', 'experience')
    })

    async function addEntry(extra: Record<string, unknown>) {
      const res = await apiRequest(ctx.app, 'POST', `/resumes/${resumeId}/entries`, {
        section_id: secId,
        perspective_id: perspId,
        ...extra,
      })
      expect(res.status).toBe(201)
      return (await res.json()).data
    }

    test('perspective plus non-empty content stores no snapshot', async () => {
      const entry = await addEntry({ content: 'Rewritten for this resume' })
      expect(entry.content).toBe('Rewritten for this resume')
      expect(entry.perspective_content_snapshot).toBeNull()
    })

    test('whitespace-only content counts as content, so no snapshot', async () => {
      const entry = await addEntry({ content: '  ' })
      expect(entry.content).toBe('  ')
      expect(entry.perspective_content_snapshot).toBeNull()
    })

    test('perspective only captures the perspective content, frozen against later edits', async () => {
      const entry = await addEntry({})
      expect(entry.perspective_content_snapshot).toBe(PERSPECTIVE_TEXT)

      ctx.db.run('UPDATE perspectives SET content = ? WHERE id = ?', ['Edited after the fact', perspId])

      const res = await apiRequest(ctx.app, 'GET', `/resumes/${resumeId}`)
      expect(res.status).toBe(200)
      const body = await res.json()
      const stored = body.data.sections[0].entries.find((e: any) => e.id === entry.id)
      expect(stored.perspective_content_snapshot).toBe(PERSPECTIVE_TEXT)
    })

    test('perspective plus content: null captures the snapshot', async () => {
      const entry = await addEntry({ content: null })
      expect(entry.perspective_content_snapshot).toBe(PERSPECTIVE_TEXT)
    })

    test('perspective plus content: "" captures the snapshot', async () => {
      const entry = await addEntry({ content: '' })
      expect(entry.perspective_content_snapshot).toBe(PERSPECTIVE_TEXT)
    })

    test('freeform entry without a perspective has no snapshot', async () => {
      const res = await apiRequest(ctx.app, 'POST', `/resumes/${resumeId}/entries`, {
        section_id: secId,
        content: 'Freeform text',
      })
      expect(res.status).toBe(201)
      const body = await res.json()
      expect(body.data.perspective_content_snapshot).toBeNull()
    })

    test('PATCH content: null clears the snapshot', async () => {
      const entry = await addEntry({})
      expect(entry.perspective_content_snapshot).toBe(PERSPECTIVE_TEXT)

      const res = await apiRequest(ctx.app, 'PATCH', `/resumes/${resumeId}/entries/${entry.id}`, {
        content: null,
      })
      expect(res.status).toBe(200)
      const body = await res.json()
      expect(body.data.content).toBeNull()
      expect(body.data.perspective_content_snapshot).toBeNull()
    })

    test('PATCH position only leaves the snapshot unchanged', async () => {
      const entry = await addEntry({})

      const res = await apiRequest(ctx.app, 'PATCH', `/resumes/${resumeId}/entries/${entry.id}`, {
        position: 5,
      })
      expect(res.status).toBe(200)
      const body = await res.json()
      expect(body.data.position).toBe(5)
      expect(body.data.perspective_content_snapshot).toBe(PERSPECTIVE_TEXT)
    })

    test('PATCH content to a non-null value leaves the snapshot unchanged', async () => {
      const entry = await addEntry({})

      const res = await apiRequest(ctx.app, 'PATCH', `/resumes/${resumeId}/entries/${entry.id}`, {
        content: 'Now with its own text',
      })
      expect(res.status).toBe(200)
      const body = await res.json()
      expect(body.data.content).toBe('Now with its own text')
      expect(body.data.perspective_content_snapshot).toBe(PERSPECTIVE_TEXT)
    })
  })

  // ── GET /resumes/:id/gaps ─────────────────────────────────────────

  test('GET /resumes/:id/gaps returns 200 with gap analysis', async () => {
    const resumeId = seedResume(ctx.db, { archetype: 'agentic-ai' })

    const res = await apiRequest(ctx.app, 'GET', `/resumes/${resumeId}/gaps`)
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data).toBeDefined()
    expect(body.data.resume_id).toBe(resumeId)
    expect(body.data.archetype).toBe('agentic-ai')
    expect(body.data.gaps).toBeArray()
    expect(body.data.coverage_summary).toBeDefined()
  })

  // ── POST /resumes/:id/export (removed) ──────────────────────────

  test('POST /resumes/:id/export returns 404 (legacy stub removed)', async () => {
    const resumeId = seedResume(ctx.db)

    const res = await apiRequest(ctx.app, 'POST', `/resumes/${resumeId}/export`)
    expect(res.status).toBe(404)
  })

  // ── DELETE /resumes/:id ───────────────────────────────────────────

  test('DELETE /resumes/:id returns 204', async () => {
    const resumeId = seedResume(ctx.db)

    const res = await apiRequest(ctx.app, 'DELETE', `/resumes/${resumeId}`)
    expect(res.status).toBe(204)

    // Verify deleted
    const getRes = await apiRequest(ctx.app, 'GET', `/resumes/${resumeId}`)
    expect(getRes.status).toBe(404)
  })

  // ── Section CRUD routes ────────────────────────────────────────────

  describe('Section CRUD routes', () => {
    test('POST /resumes/:id/sections creates a section', async () => {
      const resumeId = seedResume(ctx.db)
      const res = await apiRequest(ctx.app, 'POST', `/resumes/${resumeId}/sections`, {
        title: 'Work History',
        entry_type: 'experience',
        position: 0,
      })
      expect(res.status).toBe(201)
      const { data } = await res.json()
      expect(data.title).toBe('Work History')
      expect(data.entry_type).toBe('experience')
      expect(data.id).toHaveLength(36)
    })

    test('GET /resumes/:id/sections lists sections ordered by position', async () => {
      const resumeId = seedResume(ctx.db)
      seedResumeSection(ctx.db, resumeId, 'Skills', 'skills', 1)
      seedResumeSection(ctx.db, resumeId, 'Summary', 'freeform', 0)

      const res = await apiRequest(ctx.app, 'GET', `/resumes/${resumeId}/sections`)
      expect(res.status).toBe(200)
      const { data } = await res.json()
      expect(data).toHaveLength(2)
      expect(data[0].title).toBe('Summary')
      expect(data[1].title).toBe('Skills')
    })

    test('PATCH /resumes/:id/sections/:sectionId updates title', async () => {
      const resumeId = seedResume(ctx.db)
      const sectionId = seedResumeSection(ctx.db, resumeId, 'Old Title', 'experience')

      const res = await apiRequest(ctx.app, 'PATCH', `/resumes/${resumeId}/sections/${sectionId}`, {
        title: 'Professional Experience',
      })
      expect(res.status).toBe(200)
      const { data } = await res.json()
      expect(data.title).toBe('Professional Experience')
      expect(data.entry_type).toBe('experience')
    })

    test('DELETE /resumes/:id/sections/:sectionId deletes with cascade', async () => {
      const resumeId = seedResume(ctx.db)
      const sectionId = seedResumeSection(ctx.db, resumeId, 'Experience', 'experience')
      const sourceId = seedSource(ctx.db)
      const bulletId = seedBullet(ctx.db, [{ id: sourceId }])
      const perspId = seedPerspective(ctx.db, bulletId)
      seedResumeEntry(ctx.db, sectionId, { perspectiveId: perspId })

      const res = await apiRequest(ctx.app, 'DELETE', `/resumes/${resumeId}/sections/${sectionId}`)
      expect(res.status).toBe(204)

      // Entries should be gone
      const entries = ctx.db.query('SELECT * FROM resume_entries WHERE section_id = ?').all(sectionId) as any[]
      expect(entries).toHaveLength(0)
    })

    test('returns 404 for section on wrong resume', async () => {
      const resume1 = seedResume(ctx.db)
      const resume2 = seedResume(ctx.db)
      const sectionId = seedResumeSection(ctx.db, resume1, 'Experience', 'experience')

      const res = await apiRequest(ctx.app, 'DELETE', `/resumes/${resume2}/sections/${sectionId}`)
      expect(res.status).toBe(404)
    })
  })

  // ── Resume Skills routes ───────────────────────────────────────────

  describe('Resume Skills routes', () => {
    test('POST /resumes/:id/sections/:sectionId/skills adds a skill', async () => {
      const resumeId = seedResume(ctx.db)
      const sectionId = seedResumeSection(ctx.db, resumeId, 'Skills', 'skills')
      const skillId = seedSkill(ctx.db, { name: 'Python' })

      const res = await apiRequest(
        ctx.app, 'POST',
        `/resumes/${resumeId}/sections/${sectionId}/skills`,
        { skill_id: skillId },
      )
      expect(res.status).toBe(201)
      const { data } = await res.json()
      expect(data.skill_id).toBe(skillId)
    })

    test('GET /resumes/:id/sections/:sectionId/skills lists skills', async () => {
      const resumeId = seedResume(ctx.db)
      const sectionId = seedResumeSection(ctx.db, resumeId, 'Skills', 'skills')
      const skill1 = seedSkill(ctx.db, { name: 'Python' })
      const skill2 = seedSkill(ctx.db, { name: 'TypeScript' })
      seedResumeSkill(ctx.db, sectionId, skill1, 0)
      seedResumeSkill(ctx.db, sectionId, skill2, 1)

      const res = await apiRequest(ctx.app, 'GET', `/resumes/${resumeId}/sections/${sectionId}/skills`)
      expect(res.status).toBe(200)
      const { data } = await res.json()
      expect(data).toHaveLength(2)
    })

    test('DELETE /resumes/:id/sections/:sectionId/skills/:skillId removes a skill', async () => {
      const resumeId = seedResume(ctx.db)
      const sectionId = seedResumeSection(ctx.db, resumeId, 'Skills', 'skills')
      const skillId = seedSkill(ctx.db, { name: 'Go' })
      seedResumeSkill(ctx.db, sectionId, skillId)

      const res = await apiRequest(
        ctx.app, 'DELETE',
        `/resumes/${resumeId}/sections/${sectionId}/skills/${skillId}`,
      )
      expect(res.status).toBe(204)

      // Verify removed
      const skills = ctx.db.query('SELECT * FROM resume_skills WHERE section_id = ?').all(sectionId) as any[]
      expect(skills).toHaveLength(0)
    })

    test('rejects adding skill to non-skills section', async () => {
      const resumeId = seedResume(ctx.db)
      const sectionId = seedResumeSection(ctx.db, resumeId, 'Experience', 'experience')
      const skillId = seedSkill(ctx.db, { name: 'Python' })

      const res = await apiRequest(
        ctx.app, 'POST',
        `/resumes/${resumeId}/sections/${sectionId}/skills`,
        { skill_id: skillId },
      )
      expect(res.status).toBe(400)
      const body = await res.json()
      expect(body.error.code).toBe('VALIDATION_ERROR')
    })

    test('duplicate skill returns 409', async () => {
      const resumeId = seedResume(ctx.db)
      const sectionId = seedResumeSection(ctx.db, resumeId, 'Skills', 'skills')
      const skillId = seedSkill(ctx.db, { name: 'Rust' })
      seedResumeSkill(ctx.db, sectionId, skillId)

      const res = await apiRequest(
        ctx.app, 'POST',
        `/resumes/${resumeId}/sections/${sectionId}/skills`,
        { skill_id: skillId },
      )
      expect(res.status).toBe(409)
    })
  })

  // ── Updated entry endpoints ────────────────────────────────────────

  describe('Updated entry endpoints', () => {
    test('POST /resumes/:id/entries with section_id', async () => {
      const resumeId = seedResume(ctx.db)
      const sectionId = seedResumeSection(ctx.db, resumeId, 'Experience', 'experience')
      const sourceId = seedSource(ctx.db)
      const bulletId = seedBullet(ctx.db, [{ id: sourceId }])
      const perspId = seedPerspective(ctx.db, bulletId)

      const res = await apiRequest(ctx.app, 'POST', `/resumes/${resumeId}/entries`, {
        section_id: sectionId,
        perspective_id: perspId,
        position: 0,
      })
      expect(res.status).toBe(201)
      const { data } = await res.json()
      expect(data.section_id).toBe(sectionId)
    })

    test('POST /resumes/:id/entries freeform (no perspective_id)', async () => {
      const resumeId = seedResume(ctx.db)
      const sectionId = seedResumeSection(ctx.db, resumeId, 'Summary', 'freeform')

      const res = await apiRequest(ctx.app, 'POST', `/resumes/${resumeId}/entries`, {
        section_id: sectionId,
        content: 'Hello freeform',
        position: 0,
      })
      expect(res.status).toBe(201)
      const { data } = await res.json()
      expect(data.perspective_id).toBeNull()
      expect(data.content).toBe('Hello freeform')
    })

    test('reorder entries with section_id', async () => {
      const resumeId = seedResume(ctx.db)
      const sectionId = seedResumeSection(ctx.db, resumeId, 'Experience', 'experience')
      const sourceId = seedSource(ctx.db)
      const bulletId = seedBullet(ctx.db, [{ id: sourceId }])
      const p1 = seedPerspective(ctx.db, bulletId, { content: 'First' })
      const p2 = seedPerspective(ctx.db, bulletId, { content: 'Second' })
      const e1 = seedResumeEntry(ctx.db, sectionId, { perspectiveId: p1, position: 0 })
      const e2 = seedResumeEntry(ctx.db, sectionId, { perspectiveId: p2, position: 1 })

      const res = await apiRequest(ctx.app, 'PATCH', `/resumes/${resumeId}/entries/reorder`, {
        entries: [
          { id: e2, section_id: sectionId, position: 0 },
          { id: e1, section_id: sectionId, position: 1 },
        ],
      })
      expect(res.status).toBe(200)
    })
  })

  // ── End-to-end smoke tests ─────────────────────────────────────────

  describe('End-to-end smoke: section lifecycle', () => {
    test('add section -> add skills -> verify IR', async () => {
      const resumeId = seedResume(ctx.db)
      const skillId = seedSkill(ctx.db, { name: 'TypeScript', category: 'language' })

      // Create skills section
      const createRes = await apiRequest(ctx.app, 'POST', `/resumes/${resumeId}/sections`, {
        title: 'Technical Skills',
        entry_type: 'skills',
        position: 0,
      })
      expect(createRes.status).toBe(201)
      const { data: section } = await createRes.json()

      // Add skill
      const addRes = await apiRequest(
        ctx.app, 'POST',
        `/resumes/${resumeId}/sections/${section.id}/skills`,
        { skill_id: skillId },
      )
      expect(addRes.status).toBe(201)

      // Get IR
      const irRes = await apiRequest(ctx.app, 'GET', `/resumes/${resumeId}/ir`)
      expect(irRes.status).toBe(200)
      const { data: ir } = await irRes.json()
      expect(ir.sections).toHaveLength(1)
      expect(ir.sections[0].type).toBe('skills')
      expect(ir.sections[0].items[0].kind).toBe('skill_group')
      expect(ir.sections[0].items[0].categories[0].skills).toContain('TypeScript')
    })

    // Renderers (LaTeX, Markdown, Typst) draw only `ir.sections`, so a linked summary must
    // reach them through exactly one section: the synthetic one at the top. `ir.summary`
    // is the same text for the editor. Empty sections stay in the IR; renderers omit them.
    test('linked summary -> IR carries it in exactly one section', async () => {
      const text = 'Builds the runtime that agents run on.'
      const sumRes = await apiRequest(ctx.app, 'POST', '/summaries', {
        title: 'Platform engineer',
        description: text,
      })
      expect(sumRes.status).toBe(201)
      const { data: summary } = await sumRes.json()

      const createRes = await apiRequest(ctx.app, 'POST', '/resumes', {
        name: 'Linked summary',
        target_role: 'Engineer',
        target_employer: 'Acme',
        archetype: 'agentic-ai',
      })
      expect(createRes.status).toBe(201)
      const { data: resume } = await createRes.json()
      const linkRes = await apiRequest(ctx.app, 'PATCH', `/resumes/${resume.id}`, {
        summary_id: summary.id,
      })
      expect(linkRes.status).toBe(200)
      const secRes = await apiRequest(ctx.app, 'POST', `/resumes/${resume.id}/sections`, {
        title: 'Experience',
        entry_type: 'experience',
        position: 0,
      })
      expect(secRes.status).toBe(201)

      const irRes = await apiRequest(ctx.app, 'GET', `/resumes/${resume.id}/ir`)
      expect(irRes.status).toBe(200)
      const { data: ir } = await irRes.json()
      expect(ir.summary.content).toBe(text)
      expect(ir.sections.map((s: { type: string }) => s.type)).toEqual(['summary', 'experience'])
      expect(ir.sections[0].title).toBe('Summary')
      expect(ir.sections[0].items).toHaveLength(1)
      expect(ir.sections[0].items[0].kind).toBe('summary')
      expect(ir.sections[0].items[0].content).toBe(text)
      expect(JSON.stringify(ir.sections).split(text)).toHaveLength(2)
      expect(ir.sections[1].items).toEqual([])
    })

    // An entry added by source_id alone has no bullet text of its own; the IR carries the
    // source's description for it, which the Typst renderer draws (resu-mx/forge#204).
    test('source-only presentation and project entries -> IR carries the source description', async () => {
      const createRes = await apiRequest(ctx.app, 'POST', '/resumes', {
        name: 'Source-only entries',
        target_role: 'Researcher',
        target_employer: 'Acme',
        archetype: 'agentic-ai',
      })
      expect(createRes.status).toBe(201)
      const { data: resume } = await createRes.json()

      const cases = [
        { source_type: 'presentation', entry_type: 'presentations', kind: 'presentation', description: 'Panelist on audit trails for autonomous agents.' },
        { source_type: 'project', entry_type: 'projects', kind: 'project', description: 'A laptop-scale robot arm simulator.' },
      ]
      for (const [i, c] of cases.entries()) {
        const srcRes = await apiRequest(ctx.app, 'POST', '/sources', {
          title: `Source-only ${c.kind}`,
          description: c.description,
          source_type: c.source_type,
        })
        expect(srcRes.status).toBe(201)
        const { data: source } = await srcRes.json()
        const secRes = await apiRequest(ctx.app, 'POST', `/resumes/${resume.id}/sections`, {
          title: c.entry_type,
          entry_type: c.entry_type,
          position: i,
        })
        expect(secRes.status).toBe(201)
        const { data: section } = await secRes.json()
        const entryRes = await apiRequest(ctx.app, 'POST', `/resumes/${resume.id}/entries`, {
          section_id: section.id,
          source_id: source.id,
        })
        expect(entryRes.status).toBe(201)
      }

      const irRes = await apiRequest(ctx.app, 'GET', `/resumes/${resume.id}/ir`)
      expect(irRes.status).toBe(200)
      const { data: ir } = await irRes.json()
      for (const c of cases) {
        const section = ir.sections.find((s: { type: string }) => s.type === c.entry_type)
        expect(section.items).toHaveLength(1)
        expect(section.items[0].kind).toBe(c.kind)
        expect(section.items[0].description).toBe(c.description)
      }
    })

    test('add freeform section -> add entry -> verify IR', async () => {
      const resumeId = seedResume(ctx.db)

      // Create freeform section
      const createRes = await apiRequest(ctx.app, 'POST', `/resumes/${resumeId}/sections`, {
        title: 'Summary',
        entry_type: 'freeform',
        position: 0,
      })
      expect(createRes.status).toBe(201)
      const { data: section } = await createRes.json()

      // Add freeform entry
      const addRes = await apiRequest(ctx.app, 'POST', `/resumes/${resumeId}/entries`, {
        section_id: section.id,
        content: 'Experienced engineer.',
        position: 0,
      })
      expect(addRes.status).toBe(201)

      // Get IR
      const irRes = await apiRequest(ctx.app, 'GET', `/resumes/${resumeId}/ir`)
      expect(irRes.status).toBe(200)
      const { data: ir } = await irRes.json()
      expect(ir.sections).toHaveLength(1)
      expect(ir.sections[0].type).toBe('freeform')
      expect(ir.sections[0].items[0].kind).toBe('summary')
      expect(ir.sections[0].items[0].content).toBe('Experienced engineer.')
    })

    // The PDF, Markdown and LaTeX outputs all mark an in-progress certification from this flag.
    // (PDF text itself is not asserted here: the TS PDF needs tectonic.)
    test('GET /resumes/:id/ir flags an in-progress certification and keeps its name plain', async () => {
      const resumeId = seedResume(ctx.db)
      const sectionId = seedResumeSection(ctx.db, resumeId, 'Certifications', 'certifications', 0)
      const orgId = crypto.randomUUID()
      ctx.db.run(`INSERT INTO organizations (id, name, org_type) VALUES (?, 'ISC2', 'company')`, [orgId])
      const pin = (short: string, inProgress: number, position: number) => {
        const certId = crypto.randomUUID()
        ctx.db.run(
          `INSERT INTO certifications (id, short_name, long_name, issuer_id, in_progress)
           VALUES (?, ?, ?, ?, ?)`,
          [certId, short, short, orgId, inProgress],
        )
        ctx.db.run(
          `INSERT INTO resume_certifications (id, resume_id, certification_id, section_id, position)
           VALUES (?, ?, ?, ?, ?)`,
          [crypto.randomUUID(), resumeId, certId, sectionId, position],
        )
      }
      pin('CISSP', 0, 0)
      pin('CCSP', 1, 1)

      const irRes = await apiRequest(ctx.app, 'GET', `/resumes/${resumeId}/ir`)
      expect(irRes.status).toBe(200)
      const { data: ir } = await irRes.json()
      const group = ir.sections.find((s: { type: string }) => s.type === 'certifications').items[0]
      expect(group.kind).toBe('certification_group')
      expect(group.categories[0].certs.map((c: { name: string; in_progress: boolean }) => [c.name, c.in_progress])).toEqual([
        ['CISSP', false],
        ['CCSP', true],
      ])
    })
  })
})
