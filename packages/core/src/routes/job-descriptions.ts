/**
 * Job description routes -- thin HTTP layer over JobDescriptionService.
 */

import { Hono } from 'hono'
import type { Database } from 'bun:sqlite'
import type { Services } from '../services'
import { mapStatusCode } from './server'
import { renderJDSkillExtractionPrompt } from '../ai'
import { regenerateResumeTagline } from '../services/tagline-service'

export function jobDescriptionRoutes(services: Services, db: Database) {
  const app = new Hono()

  app.post('/job-descriptions', async (c) => {
    const body = await c.req.json()
    const result = await services.jobDescriptions.create(body)
    if (!result.ok)
      return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data }, 201)
  })

  app.get('/job-descriptions', async (c) => {
    const offset = Math.max(
      0,
      parseInt(c.req.query('offset') ?? '0', 10) || 0,
    )
    const limit = Math.min(
      200,
      Math.max(1, parseInt(c.req.query('limit') ?? '50', 10) || 50),
    )
    const filter: Record<string, string> = {}
    if (c.req.query('status')) filter.status = c.req.query('status')!
    if (c.req.query('organization_id'))
      filter.organization_id = c.req.query('organization_id')!

    const result = await services.jobDescriptions.list(filter, offset, limit)
    if (!result.ok)
      return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data, pagination: result.pagination })
  })

  // Lookup by URL — must come before :id route
  app.post('/job-descriptions/lookup-by-url', async (c) => {
    const body = await c.req.json()
    const url = body?.url
    if (!url || typeof url !== 'string' || url.trim().length === 0) {
      return c.json(
        { error: { code: 'VALIDATION_ERROR', message: 'URL must be a non-empty string' } },
        400,
      )
    }
    const result = await services.jobDescriptions.lookupByUrl(url)
    if (!result.ok) {
      return c.json({ error: result.error }, mapStatusCode(result.error.code))
    }
    return c.json({ data: result.data })
  })

  app.get('/job-descriptions/:id', async (c) => {
    const result = await services.jobDescriptions.get(c.req.param('id'))
    if (!result.ok)
      return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.patch('/job-descriptions/:id', async (c) => {
    const body = await c.req.json()
    const result = await services.jobDescriptions.update(c.req.param('id'), body)
    if (!result.ok)
      return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.delete('/job-descriptions/:id', async (c) => {
    const result = await services.jobDescriptions.delete(c.req.param('id'))
    if (!result.ok)
      return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.body(null, 204)
  })

  // ── JD Skills ───────────────────────────────────────────────────────

  app.get('/job-descriptions/:id/skills', (c) => {
    const rows = db.query(
      `SELECT s.* FROM skills s
       JOIN job_description_skills jds ON jds.skill_id = s.id
       WHERE jds.job_description_id = ?
       ORDER BY s.name ASC`
    ).all(c.req.param('id'))
    return c.json({ data: rows })
  })

  app.post('/job-descriptions/:id/skills', async (c) => {
    const body = await c.req.json()
    const jdId = c.req.param('id')

    // If skill_id is provided, link existing skill
    if (body.skill_id) {
      try {
        db.run('INSERT OR IGNORE INTO job_description_skills (job_description_id, skill_id) VALUES (?, ?)',
          [jdId, body.skill_id])
      } catch (err: any) {
        if (err.message?.includes('FOREIGN KEY')) {
          return c.json({ error: { code: 'NOT_FOUND', message: 'Job description or skill not found' } }, 404)
        }
        throw err
      }
      const skill = db.query('SELECT * FROM skills WHERE id = ?').get(body.skill_id)
      return c.json({ data: skill }, 201)
    }

    // If name is provided, create new skill and link it
    if (body.name?.trim()) {
      // Check the JD before writing anything. Linking to an unknown JD fails the FK after the
      // skill was inserted, which left an orphan skill and answered 500.
      if (!db.query('SELECT 1 FROM job_descriptions WHERE id = ?').get(jdId)) {
        return c.json({ error: { code: 'NOT_FOUND', message: 'Job description or skill not found' } }, 404)
      }
      const raw = body.name.trim()
      const name = raw.charAt(0).toUpperCase() + raw.slice(1)
      let skill = db.query('SELECT * FROM skills WHERE name = ? COLLATE NOCASE').get(name) as any
      if (!skill) {
        const id = crypto.randomUUID()
        // Category is a CHECK enum (Phase 89) — only allow valid values; fall back to 'other'.
        const validCategories = ['language', 'framework', 'platform', 'tool', 'library',
          'methodology', 'protocol', 'concept', 'soft_skill', 'other']
        const category = validCategories.includes(body.category) ? body.category : 'other'
        skill = db.query(
          `INSERT INTO skills (id, name, category) VALUES (?, ?, ?) RETURNING *`
        ).get(id, name, category)
      }
      db.run('INSERT OR IGNORE INTO job_description_skills (job_description_id, skill_id) VALUES (?, ?)',
        [jdId, skill.id])
      return c.json({ data: skill }, 201)
    }

    return c.json({ error: { code: 'VALIDATION_ERROR', message: 'skill_id or name is required' } }, 400)
  })

  app.delete('/job-descriptions/:jdId/skills/:skillId', (c) => {
    db.run(
      'DELETE FROM job_description_skills WHERE job_description_id = ? AND skill_id = ?',
      [c.req.param('jdId'), c.req.param('skillId')]
    )
    return c.body(null, 204)
  })

  // ── JD-Resume Linkage ─────────────────────────────────────────────

  app.get('/job-descriptions/:id/resumes', async (c) => {
    const jdId = c.req.param('id')

    // Verify JD exists
    const jd = await services.jobDescriptions.get(jdId)
    if (!jd.ok) return c.json({ error: jd.error }, mapStatusCode(jd.error.code))

    const rows = db
      .query(`
        SELECT r.id AS resume_id, r.name AS resume_name, r.target_role,
               r.target_employer, r.archetype, r.status,
               jdr.created_at, r.created_at AS resume_created_at
        FROM job_description_resumes jdr
        JOIN resumes r ON r.id = jdr.resume_id
        WHERE jdr.job_description_id = ?
        ORDER BY jdr.created_at DESC
      `)
      .all(jdId)

    return c.json({ data: rows })
  })

  app.post('/job-descriptions/:id/resumes', async (c) => {
    const jdId = c.req.param('id')
    const body = await c.req.json()
    const resumeId = body.resume_id

    if (!resumeId || typeof resumeId !== 'string') {
      return c.json(
        { error: { code: 'VALIDATION_ERROR', message: 'resume_id is required' } },
        400,
      )
    }

    // Verify JD exists
    const jd = await services.jobDescriptions.get(jdId)
    if (!jd.ok) return c.json({ error: jd.error }, mapStatusCode(jd.error.code))

    // Verify resume exists
    const resume = await services.resumes.getResume(resumeId)
    if (!resume.ok) {
      return c.json(
        { error: { code: 'NOT_FOUND', message: `Resume ${resumeId} not found` } },
        404,
      )
    }

    // INSERT OR IGNORE for idempotent linking.
    // Use result.changes from db.run() return value -- NOT a separate
    // SELECT changes() query, which is unreliable under Bun's SQLite driver.
    const result = db.run(
      `INSERT OR IGNORE INTO job_description_resumes (job_description_id, resume_id) VALUES (?, ?)`,
      [jdId, resumeId],
    )

    // Phase 92: regenerate the resume's tagline from all linked JDs. We do
    // this for both new links and re-links (200 path) so the caller can
    // see the current tagline in the response.
    const tagline = await regenerateResumeTagline(db, resumeId)

    // Fetch the link data (whether just created or already existed)
    const link = db
      .query(`
        SELECT r.id AS resume_id, r.name AS resume_name, r.target_role,
               r.target_employer, r.archetype, r.status,
               jdr.created_at, r.created_at AS resume_created_at
        FROM job_description_resumes jdr
        JOIN resumes r ON r.id = jdr.resume_id
        WHERE jdr.job_description_id = ? AND jdr.resume_id = ?
      `)
      .get(jdId, resumeId)

    // Determine status code: 201 if new, 200 if already existed
    const status = result.changes > 0 ? 201 : 200

    return c.json({ data: link, tagline }, status as any)
  })

  app.delete('/job-descriptions/:jdId/resumes/:resumeId', async (c) => {
    const { jdId, resumeId } = c.req.param()

    db.run(
      `DELETE FROM job_description_resumes WHERE job_description_id = ? AND resume_id = ?`,
      [jdId, resumeId],
    )

    // Phase 92: regenerate tagline from remaining linked JDs (if any).
    // If none remain, the tagline is cleared to null.
    // The DELETE contract remains 204 — clients refetch the resume to see
    // the updated generated_tagline.
    await regenerateResumeTagline(db, resumeId)

    return c.body(null, 204)
  })

  // ── Contact reverse lookup ──────────────────────────────────────────
  app.get('/job-descriptions/:id/contacts', async (c) => {
    const result = await services.contacts.listByJobDescription(c.req.param('id'))
    if (!result.ok)
      return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  // ── JD Skill Extraction (Context) ────────────────────────────────
  // Returns a context payload for MCP client to perform extraction.
  // No AI call is made here -- the caller executes the prompt_template
  // and calls POST /:id/skills for each accepted skill individually.

  app.post('/job-descriptions/:id/extract-skills', async (c) => {
    const { id } = c.req.param()

    // 1. Fetch the JD
    const jdResult = await services.jobDescriptions.get(id)
    if (!jdResult.ok) {
      return c.json({ error: jdResult.error }, mapStatusCode(jdResult.error.code))
    }
    const jd = jdResult.data

    // 2. Validate raw_text
    if (!jd.raw_text || jd.raw_text.trim().length === 0) {
      return c.json({
        error: { code: 'VALIDATION_ERROR', message: 'Job description has no text to extract skills from' },
      }, 400)
    }

    // 3. Render prompt template
    const prompt_template = renderJDSkillExtractionPrompt(jd.raw_text)

    // 4. Get existing skills filtered by categories mentioned in JD text
    const rawText = jd.raw_text.toLowerCase()
    const categoryKeywords: Record<string, string[]> = {
      language: ['python', 'java', 'go', 'rust', 'typescript', 'javascript', 'c++', 'scala', 'ruby', 'kotlin'],
      framework: ['fastapi', 'django', 'flask', 'react', 'next.js', 'express', 'spring', 'pytorch', 'tensorflow'],
      tool: ['docker', 'kubernetes', 'terraform', 'helm', 'git', 'jenkins', 'grafana', 'prometheus'],
      platform: ['aws', 'gcp', 'azure', 'vercel', 'heroku', 'cloudflare'],
      methodology: ['agile', 'scrum', 'kanban', 'devops', 'devsecops', 'ci/cd', 'tdd', 'sre'],
      domain: ['machine learning', 'deep learning', 'nlp', 'computer vision', 'distributed systems', 'security'],
      certification: ['cka', 'ckad', 'aws certified', 'security+', 'cissp'],
    }

    const matchedCategories = new Set<string>()
    for (const [category, keywords] of Object.entries(categoryKeywords)) {
      if (keywords.some(kw => rawText.includes(kw))) {
        matchedCategories.add(category)
      }
    }
    matchedCategories.add('other')
    matchedCategories.add('soft_skill')

    let existingSkills: Array<{ id: string; name: string; category: string }> = []
    const allSkills = await services.skills.list()
    if (allSkills.ok) {
      existingSkills = allSkills.data
        .filter(s => s.category && matchedCategories.has(s.category))
        .map(s => ({ id: s.id, name: s.name, category: s.category ?? 'other' }))
    }

    // 5. Return context payload
    return c.json({
      data: {
        jd_raw_text: jd.raw_text,
        existing_skills: existingSkills,
        prompt_template,
        instructions: 'Execute the prompt_template to extract skills from the JD text. For each extracted skill, check existing_skills for a match by name before creating new ones. Call forge_tag_jd_skill (or POST /api/job-descriptions/:id/skills) for each accepted skill.',
      },
    })
  })

  return app
}
