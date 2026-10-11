/**
 * Bullet routes — thin HTTP layer over BulletService and DerivationService.
 */

import { Hono } from 'hono'
import type { Database } from 'bun:sqlite'
import type { Services } from '../services'
import { mapStatusCode } from './server'

export function bulletRoutes(services: Services, db: Database) {
  const app = new Hono()

  app.post('/bullets', async (c) => {
    const body = await c.req.json()
    const result = await services.bullets.createBullet(body)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data }, 201)
  })

  app.get('/bullets', async (c) => {
    const offset = Math.max(0, parseInt(c.req.query('offset') ?? '0', 10) || 0)
    const limit = Math.min(200, Math.max(1, parseInt(c.req.query('limit') ?? '50', 10) || 50))
    const filter: Record<string, string> = {}
    if (c.req.query('source_id')) filter.source_id = c.req.query('source_id')!
    if (c.req.query('status')) filter.status = c.req.query('status')!
    if (c.req.query('technology')) filter.technology = c.req.query('technology')!

    const result = await services.bullets.listBullets(filter, offset, limit)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data, pagination: result.pagination })
  })

  app.get('/bullets/:id', async (c) => {
    const result = await services.bullets.getBullet(c.req.param('id'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.patch('/bullets/:id', async (c) => {
    const body = await c.req.json()
    const result = await services.bullets.updateBullet(c.req.param('id'), body)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.delete('/bullets/:id', async (c) => {
    const result = await services.bullets.deleteBullet(c.req.param('id'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.body(null, 204)
  })

  app.patch('/bullets/:id/approve', async (c) => {
    const result = await services.bullets.approveBullet(c.req.param('id'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.patch('/bullets/:id/reject', async (c) => {
    const body = await c.req.json<{ rejection_reason?: string }>()
    const result = await services.bullets.rejectBullet(c.req.param('id'), body.rejection_reason ?? '')
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.patch('/bullets/:id/reopen', async (c) => {
    const result = await services.bullets.reopenBullet(c.req.param('id'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.patch('/bullets/:id/submit', async (c) => {
    const result = await services.bullets.submitBullet(c.req.param('id'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.patch('/bullets/:id/archive', async (c) => {
    const result = await services.bullets.archiveBullet(c.req.param('id'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.patch('/bullets/:id/unarchive', async (c) => {
    const result = await services.bullets.unarchiveBullet(c.req.param('id'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.post('/bullets/:id/derive-perspectives', (c) => {
    return c.json({
      error: {
        code: 'NOT_IMPLEMENTED',
        message: 'This endpoint has been replaced. Use POST /api/derivations/prepare with entity_type "bullet".',
      },
    }, 501)
  })

  // ── Bullet Skills ───────────────────────────────────────────────────

  app.get('/bullets/:id/skills', (c) => {
    const rows = db.query(
      `SELECT s.* FROM skills s
       JOIN bullet_skills bs ON bs.skill_id = s.id
       WHERE bs.bullet_id = ?
       ORDER BY s.name ASC`
    ).all(c.req.param('id'))
    return c.json({ data: rows })
  })

  app.post('/bullets/:id/skills', async (c) => {
    const body = await c.req.json()
    const bulletId = c.req.param('id')

    // If skill_id is provided, link existing skill
    if (body.skill_id) {
      try {
        db.run('INSERT OR IGNORE INTO bullet_skills (bullet_id, skill_id) VALUES (?, ?)',
          [bulletId, body.skill_id])
      } catch (err: any) {
        if (err.message?.includes('FOREIGN KEY')) {
          return c.json({ error: { code: 'NOT_FOUND', message: 'Bullet or skill not found' } }, 404)
        }
        throw err
      }
      const skill = db.query('SELECT * FROM skills WHERE id = ?').get(body.skill_id)
      return c.json({ data: skill }, 201)
    }

    // If name is provided, create new skill and link it
    if (body.name?.trim()) {
      // capitalizeFirst: uppercase first character, preserve rest (SAFe stays SAFe, foo->Foo)
      const raw = body.name.trim()
      const name = raw.charAt(0).toUpperCase() + raw.slice(1)
      // Case-insensitive dedup: check if skill with this name already exists
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
      db.run('INSERT OR IGNORE INTO bullet_skills (bullet_id, skill_id) VALUES (?, ?)',
        [bulletId, skill.id])
      return c.json({ data: skill }, 201)
    }

    return c.json({ error: { code: 'VALIDATION_ERROR', message: 'skill_id or name is required' } }, 400)
  })

  app.delete('/bullets/:bulletId/skills/:skillId', (c) => {
    const result = db.run(
      'DELETE FROM bullet_skills WHERE bullet_id = ? AND skill_id = ?',
      [c.req.param('bulletId'), c.req.param('skillId')]
    )
    if (result.changes === 0) {
      return c.json({ error: { code: 'NOT_FOUND', message: 'Skill link not found' } }, 404)
    }
    return c.body(null, 204)
  })

  // ── Bullet Sources ──────────────────────────────────────────────────

  // NOTE: `is_primary` is returned as `0` or `1` (SQLite INTEGER).
  // The UI uses truthiness check (`{#if src.is_primary}`) which works for both number and boolean.
  app.get('/bullets/:id/sources', (c) => {
    const rows = db.query(
      `SELECT s.*, bs.is_primary
       FROM bullet_sources bs
       JOIN sources s ON bs.source_id = s.id
       WHERE bs.bullet_id = ?
       ORDER BY bs.is_primary DESC, s.title ASC`
    ).all(c.req.param('id'))
    return c.json({ data: rows })
  })

  return app
}
