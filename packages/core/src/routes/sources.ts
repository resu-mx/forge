/**
 * Source routes — thin HTTP layer over SourceService and DerivationService.
 */

import { Hono } from 'hono'
import type { Database } from 'bun:sqlite'
import type { Services } from '../services'
import { mapStatusCode } from './server'
import type { CreateSource, UpdateSource, SourceWithExtension } from '../types'

/** Map the internal `extension` field to the SDK's typed keys (role/project/education/presentation). */
function mapExtension(source: SourceWithExtension): Record<string, unknown> {
  const { extension, ...rest } = source
  if (!extension) return rest
  const key = source.source_type === 'role' ? 'role'
    : source.source_type === 'project' ? 'project'
    : source.source_type === 'education' ? 'education'
    : source.source_type === 'presentation' ? 'presentation'
    : null
  if (!key) return rest
  return { ...rest, [key]: extension }
}

export function sourceRoutes(services: Services, db: Database) {
  const app = new Hono()

  app.post('/sources', async (c) => {
    const body = await c.req.json()
    // Spread nested SDK extension objects into flat structure for core
    // Clearance needs explicit mapping: SDK uses status/type but core uses
    // clearance_status/clearance_type to avoid collisions with base fields.
    const clearanceFlat = body.clearance ? {
      level: body.clearance.level,
      polygraph: body.clearance.polygraph,
      clearance_status: body.clearance.status,
      clearance_type: body.clearance.type,
      sponsor_organization_id: body.clearance.sponsor_organization_id,
      continuous_investigation: body.clearance.continuous_investigation,
      access_programs: body.clearance.access_programs,
    } : {}
    const input: CreateSource = {
      ...body,
      ...(body.education ?? {}),
      ...(body.role ?? {}),
      ...(body.project ?? {}),
      ...clearanceFlat,
    }
    const result = await services.sources.createSource(input)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: mapExtension(result.data) }, 201)
  })

  app.get('/sources', async (c) => {
    const offset = Math.max(0, parseInt(c.req.query('offset') ?? '0', 10) || 0)
    const limit = Math.min(200, Math.max(1, parseInt(c.req.query('limit') ?? '50', 10) || 50))
    const filter: Record<string, string> = {}
    if (c.req.query('source_type')) filter.source_type = c.req.query('source_type')!
    if (c.req.query('organization_id')) filter.organization_id = c.req.query('organization_id')!
    if (c.req.query('status')) filter.status = c.req.query('status')!
    if (c.req.query('education_type')) filter.education_type = c.req.query('education_type')!
    if (c.req.query('search')) filter.search = c.req.query('search')!

    const result = await services.sources.listSources(filter, offset, limit)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data.map(mapExtension), pagination: result.pagination })
  })

  app.get('/sources/:id', async (c) => {
    const result = await services.sources.getSource(c.req.param('id'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: mapExtension(result.data) })
  })

  app.patch('/sources/:id', async (c) => {
    const body = await c.req.json()
    // Spread nested SDK extension objects into flat structure for core
    // Clearance needs explicit mapping: SDK uses status/type but core uses
    // clearance_status/clearance_type to avoid collisions with base fields.
    const clearanceFlat = body.clearance ? {
      ...(body.clearance.level !== undefined ? { level: body.clearance.level } : {}),
      ...(body.clearance.polygraph !== undefined ? { polygraph: body.clearance.polygraph } : {}),
      ...(body.clearance.status !== undefined ? { clearance_status: body.clearance.status } : {}),
      ...(body.clearance.type !== undefined ? { clearance_type: body.clearance.type } : {}),
      ...(body.clearance.sponsor_organization_id !== undefined ? { sponsor_organization_id: body.clearance.sponsor_organization_id } : {}),
      ...(body.clearance.continuous_investigation !== undefined ? { continuous_investigation: body.clearance.continuous_investigation } : {}),
      ...(body.clearance.access_programs !== undefined ? { access_programs: body.clearance.access_programs } : {}),
    } : {}
    const input: UpdateSource = {
      ...body,
      ...(body.education ?? {}),
      ...(body.role ?? {}),
      ...(body.project ?? {}),
      ...(body.presentation ?? {}),
      ...clearanceFlat,
    }
    const result = await services.sources.updateSource(c.req.param('id'), input)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: mapExtension(result.data) })
  })

  app.delete('/sources/:id', async (c) => {
    const result = await services.sources.deleteSource(c.req.param('id'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.body(null, 204)
  })

  app.post('/sources/:id/derive-bullets', (c) => {
    return c.json({
      error: {
        code: 'NOT_IMPLEMENTED',
        message: 'This endpoint has been replaced. Use POST /api/derivations/prepare with entity_type "source".',
      },
    }, 501)
  })

  // ── Source Skills ─────────────────────────────────────────────────

  app.get('/sources/:id/skills', (c) => {
    const rows = db.query(
      `SELECT s.* FROM skills s
       JOIN source_skills ss ON ss.skill_id = s.id
       WHERE ss.source_id = ?
       ORDER BY s.name ASC`
    ).all(c.req.param('id'))
    return c.json({ data: rows })
  })

  app.post('/sources/:id/skills', async (c) => {
    const body = await c.req.json()
    const sourceId = c.req.param('id')

    // If skill_id is provided, link existing skill
    if (body.skill_id) {
      try {
        db.run('INSERT OR IGNORE INTO source_skills (source_id, skill_id) VALUES (?, ?)',
          [sourceId, body.skill_id])
      } catch (err: any) {
        if (err.message?.includes('FOREIGN KEY')) {
          return c.json({ error: { code: 'NOT_FOUND', message: 'Source or skill not found' } }, 404)
        }
        throw err
      }
      const skill = db.query('SELECT * FROM skills WHERE id = ?').get(body.skill_id)
      return c.json({ data: skill }, 201)
    }

    // If name is provided, create new skill and link it
    if (body.name?.trim()) {
      // Check the source first. Otherwise the skill is inserted, the link insert then
      // fails the FK, and the request answers 500 with an orphan skill left behind
      // (resu-mx/forge#21).
      if (!db.query('SELECT 1 FROM sources WHERE id = ?').get(sourceId)) {
        return c.json({ error: { code: 'NOT_FOUND', message: 'Source or skill not found' } }, 404)
      }
      // Capitalize first character only, preserve rest (SAFe stays SAFe, foo→Foo)
      const raw = body.name.trim()
      const name = raw.charAt(0).toUpperCase() + raw.slice(1)
      // Check if skill with this name already exists (case-insensitive)
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
      db.run('INSERT OR IGNORE INTO source_skills (source_id, skill_id) VALUES (?, ?)',
        [sourceId, skill.id])
      return c.json({ data: skill }, 201)
    }

    return c.json({ error: { code: 'VALIDATION_ERROR', message: 'skill_id or name is required' } }, 400)
  })

  app.delete('/sources/:sourceId/skills/:skillId', (c) => {
    const result = db.run(
      'DELETE FROM source_skills WHERE source_id = ? AND skill_id = ?',
      [c.req.param('sourceId'), c.req.param('skillId')]
    )
    if (result.changes === 0) {
      return c.json({ error: { code: 'NOT_FOUND', message: 'Skill link not found' } }, 404)
    }
    return c.body(null, 204)
  })

  return app
}
