/**
 * Resume routes — thin HTTP layer over ResumeService.
 */

import { Hono } from 'hono'
import type { Database } from 'bun:sqlite'
import type { Services } from '../services'
import { mapStatusCode } from './server'
import { regenerateResumeTagline, resolveTagline } from '../services/tagline-service'
import type { CreateResume, UpdateResume, AddResumeEntry } from '../types'

export function resumeRoutes(services: Services, db: Database) {
  const app = new Hono()

  app.post('/resumes', async (c) => {
    const body = await c.req.json<CreateResume & { template_id?: string }>()

    if (body.template_id) {
      const result = await services.templates.createResumeFromTemplate({
        ...body,
        template_id: body.template_id,
      })
      if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
      return c.json({ data: result.data }, 201)
    }

    const result = await services.resumes.createResume(body)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data }, 201)
  })

  app.get('/resumes', async (c) => {
    const offset = Math.max(0, parseInt(c.req.query('offset') ?? '0', 10) || 0)
    const limit = Math.min(200, Math.max(1, parseInt(c.req.query('limit') ?? '50', 10) || 50))

    const result = await services.resumes.listResumes(offset, limit)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data, pagination: result.pagination })
  })

  app.get('/resumes/:id', async (c) => {
    const result = await services.resumes.getResume(c.req.param('id'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.patch('/resumes/:id', async (c) => {
    const id = c.req.param('id')
    const body = await c.req.json<UpdateResume>()
    // The generated tagline starts with the resume's target role, so a role that actually
    // changes regenerates it (resu-mx/forge#72). Read the old role before the update.
    const before =
      body.target_role !== undefined
        ? (db.query('SELECT target_role FROM resumes WHERE id = ?').get(id) as {
            target_role: string
          } | null)
        : null
    const result = await services.resumes.updateResume(id, body)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    if (before && before.target_role !== result.data.target_role) {
      await regenerateResumeTagline(db, id)
    }
    return c.json({ data: result.data })
  })

  app.delete('/resumes/:id', async (c) => {
    const result = await services.resumes.deleteResume(c.req.param('id'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.body(null, 204)
  })

  app.post('/resumes/:id/entries', async (c) => {
    const body = await c.req.json<AddResumeEntry>()
    const result = await services.resumes.addEntry(c.req.param('id'), body)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data }, 201)
  })

  // Reorder must come before :entryId to avoid "reorder" matching as a param
  app.patch('/resumes/:id/entries/reorder', async (c) => {
    const body = await c.req.json<{ entries: Array<{ id: string; section_id: string; position: number }> }>()
    const result = await services.resumes.reorderEntries(c.req.param('id'), body.entries)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: null })
  })

  app.patch('/resumes/:id/entries/:entryId', async (c) => {
    const body = await c.req.json<{ content?: string | null; section_id?: string; position?: number }>()
    const result = await services.resumes.updateEntry(c.req.param('id'), c.req.param('entryId'), body)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.delete('/resumes/:id/entries/:entryId', async (c) => {
    const result = await services.resumes.removeEntry(c.req.param('id'), c.req.param('entryId'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.body(null, 204)
  })

  // ── Section CRUD ────────────────────────────────────────────────────

  app.post('/resumes/:id/sections', async (c) => {
    const body = await c.req.json<{ title: string; entry_type: string; position?: number }>()
    const result = await services.resumes.createSection(c.req.param('id'), body)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data }, 201)
  })

  app.get('/resumes/:id/sections', async (c) => {
    const result = await services.resumes.listSections(c.req.param('id'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.patch('/resumes/:id/sections/:sectionId', async (c) => {
    const body = await c.req.json<{ title?: string; position?: number }>()
    const result = await services.resumes.updateSection(c.req.param('id'), c.req.param('sectionId'), body)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.delete('/resumes/:id/sections/:sectionId', async (c) => {
    const result = await services.resumes.deleteSection(c.req.param('id'), c.req.param('sectionId'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.body(null, 204)
  })

  // ── Resume Skills (within a section) ───────────────────────────────

  app.post('/resumes/:id/sections/:sectionId/skills', async (c) => {
    const body = await c.req.json<{ skill_id: string }>()
    const result = await services.resumes.addSkill(c.req.param('id'), c.req.param('sectionId'), body.skill_id)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data }, 201)
  })

  app.get('/resumes/:id/sections/:sectionId/skills', async (c) => {
    const result = await services.resumes.listSkillsForSection(c.req.param('id'), c.req.param('sectionId'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.delete('/resumes/:id/sections/:sectionId/skills/:skillId', async (c) => {
    const result = await services.resumes.removeSkill(c.req.param('id'), c.req.param('sectionId'), c.req.param('skillId'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.body(null, 204)
  })

  app.patch('/resumes/:id/sections/:sectionId/skills/reorder', async (c) => {
    const body = await c.req.json<{ skills: Array<{ skill_id: string; position: number }> }>()
    const result = await services.resumes.reorderSkills(c.req.param('id'), c.req.param('sectionId'), body.skills)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: null })
  })

  // ── Resume Certifications (per-resume cert selection) ─────────────────

  app.post('/resumes/:id/certifications', async (c) => {
    const body = await c.req.json<{ certification_id: string; section_id: string; position?: number }>()
    const result = await services.resumes.addCertification(c.req.param('id'), body)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data }, 201)
  })

  app.delete('/resumes/:id/certifications/:rcId', async (c) => {
    const result = await services.resumes.removeCertification(c.req.param('id'), c.req.param('rcId'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.body(null, 204)
  })

  app.get('/resumes/:id/certifications', async (c) => {
    const result = await services.resumes.listCertificationsForResume(c.req.param('id'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  // -- Save as Template ---------------------------------------------------

  app.post('/resumes/:id/save-as-template', async (c) => {
    const body = await c.req.json<{ name: string; description?: string }>()
    const result = await services.templates.saveAsTemplate(
      c.req.param('id'),
      body.name,
      body.description,
    )
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data }, 201)
  })

  app.get('/resumes/:id/gaps', async (c) => {
    const result = await services.resumes.analyzeGaps(c.req.param('id'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  // ── IR endpoint ──────────────────────────────────────────────────

  app.get('/resumes/:id/ir', (c) => {
    const result = services.resumes.getIR(c.req.param('id'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  // ── Header & Override endpoints ────────────────────────────────────

  app.patch('/resumes/:id/header', async (c) => {
    const body = await c.req.json()
    const result = await services.resumes.updateHeader(c.req.param('id'), body)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.patch('/resumes/:id/markdown-override', async (c) => {
    const body = await c.req.json<{ content: string | null }>()
    const result = await services.resumes.updateMarkdownOverride(c.req.param('id'), body.content)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.patch('/resumes/:id/latex-override', async (c) => {
    const body = await c.req.json<{ content: string | null }>()
    const result = await services.resumes.updateLatexOverride(c.req.param('id'), body.content)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  // ── Tagline endpoints (Phase 92) ──────────────────────────────────

  /**
   * Return the resume's current tagline state: the generated tagline
   * (from linked JDs), the user's override (if any), the resolved value
   * (override takes precedence), and a flag indicating whether an override
   * is present.
   */
  app.get('/resumes/:id/tagline', (c) => {
    const row = db
      .query(
        'SELECT generated_tagline, tagline_override FROM resumes WHERE id = ?',
      )
      .get(c.req.param('id')) as {
        generated_tagline: string | null
        tagline_override: string | null
      } | null

    if (!row) {
      return c.json(
        { error: { code: 'NOT_FOUND', message: 'Resume not found' } },
        404,
      )
    }

    const { resolved, has_override } = resolveTagline(row.tagline_override, row.generated_tagline)

    return c.json({
      data: {
        generated_tagline: row.generated_tagline,
        tagline_override: row.tagline_override,
        resolved,
        has_override,
      },
    })
  })

  /** Force a tagline regeneration from currently linked JDs. */
  app.post('/resumes/:id/tagline/regenerate', async (c) => {
    const result = await regenerateResumeTagline(db, c.req.param('id'))
    if (!result) {
      return c.json(
        { error: { code: 'NOT_FOUND', message: 'Resume not found' } },
        404,
      )
    }
    return c.json({ data: result })
  })

  /**
   * Set or clear the tagline override. Pass `{ content: string }` to set,
   * `{ content: null }` to clear the override (falls back to generated).
   */
  app.patch('/resumes/:id/tagline-override', async (c) => {
    const body = await c.req.json<{ content: string | null }>()
    const resumeId = c.req.param('id')

    const exists = db
      .query('SELECT id FROM resumes WHERE id = ?')
      .get(resumeId) as { id: string } | null
    if (!exists) {
      return c.json(
        { error: { code: 'NOT_FOUND', message: 'Resume not found' } },
        404,
      )
    }

    const normalized = body.content && body.content.trim().length > 0 ? body.content : null
    db.run(
      `UPDATE resumes
       SET tagline_override = ?,
           updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
       WHERE id = ?`,
      [normalized, resumeId],
    )

    const row = db
      .query('SELECT generated_tagline, tagline_override FROM resumes WHERE id = ?')
      .get(resumeId) as { generated_tagline: string | null; tagline_override: string | null }

    const { resolved, has_override } = resolveTagline(row.tagline_override, row.generated_tagline)

    return c.json({
      data: {
        generated_tagline: row.generated_tagline,
        tagline_override: row.tagline_override,
        resolved,
        has_override,
      },
    })
  })

  // ── PDF endpoint ──────────────────────────────────────────────────

  app.post('/resumes/:id/pdf', async (c) => {
    let latex: string | undefined
    const bust = c.req.query('bust') === '1'
    try {
      const body = await c.req.json()
      latex = body.latex
    } catch {
      // No body is fine -- compile from IR
    }

    const result = await services.resumes.generatePDF(c.req.param('id'), latex, bust)
    if (!result.ok) {
      const code = result.error.code
      const status = code === 'TECTONIC_NOT_AVAILABLE' ? 501
                   : code === 'TECTONIC_TIMEOUT' ? 504
                   : code === 'LATEX_COMPILE_ERROR' ? 422
                   : mapStatusCode(code)
      return c.json({ error: result.error }, status as any)
    }

    return new Response(result.data, {
      status: 200,
      headers: {
        'Content-Type': 'application/pdf',
        'Content-Disposition': 'inline; filename="resume.pdf"',
        'X-Forge-Pdf-Cache': result._cacheHit ? 'hit' : 'miss',
      },
    })
  })

  // ── Linked JDs (reverse lookup) ──────────────────────────────────

  app.get('/resumes/:id/job-descriptions', async (c) => {
    const resumeId = c.req.param('id')

    // Verify resume exists
    const resume = await services.resumes.getResume(resumeId)
    if (!resume.ok) return c.json({ error: resume.error }, mapStatusCode(resume.error.code))

    const rows = db
      .query(`
        SELECT jd.id AS job_description_id, jd.title, o.name AS organization_name,
               jd.status, jd.location, jd.salary_range,
               jdr.created_at, jd.created_at AS jd_created_at
        FROM job_description_resumes jdr
        JOIN job_descriptions jd ON jd.id = jdr.job_description_id
        LEFT JOIN organizations o ON o.id = jd.organization_id
        WHERE jdr.resume_id = ?
        ORDER BY jdr.created_at DESC
      `)
      .all(resumeId)

    return c.json({ data: rows })
  })

  // ── Contact reverse lookup ──────────────────────────────────────────
  app.get('/resumes/:id/contacts', async (c) => {
    const result = await services.contacts.listByResume(c.req.param('id'))
    if (!result.ok)
      return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  return app
}
