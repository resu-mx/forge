/**
 * Perspective routes — thin HTTP layer over PerspectiveService.
 */

import { Hono } from 'hono'
import type { Services } from '../services'
import { mapStatusCode } from './server'

export function perspectiveRoutes(services: Services) {
  const app = new Hono()

  app.post('/perspectives', async (c) => {
    const body = await c.req.json<{
      bullet_id: string
      content: string
      target_archetype?: string
      domain?: string
      framing?: string
      auto_approve?: boolean
    }>()
    if (!body.bullet_id) {
      return c.json({ error: { code: 'VALIDATION_ERROR', message: 'bullet_id is required' } }, 400)
    }
    if (!body.content) {
      return c.json({ error: { code: 'VALIDATION_ERROR', message: 'content is required' } }, 400)
    }
    const result = await services.perspectives.createPerspective(body)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data }, 201)
  })

  app.get('/perspectives', async (c) => {
    const offset = Math.max(0, parseInt(c.req.query('offset') ?? '0', 10) || 0)
    const limit = Math.min(200, Math.max(1, parseInt(c.req.query('limit') ?? '50', 10) || 50))
    const filter: Record<string, string> = {}
    if (c.req.query('bullet_id')) filter.bullet_id = c.req.query('bullet_id')!
    if (c.req.query('target_archetype')) filter.target_archetype = c.req.query('target_archetype')!
    if (c.req.query('domain')) filter.domain = c.req.query('domain')!
    if (c.req.query('framing')) filter.framing = c.req.query('framing')!
    if (c.req.query('status')) filter.status = c.req.query('status')!
    if (c.req.query('source_id')) filter.source_id = c.req.query('source_id')!
    if (c.req.query('search')) filter.search = c.req.query('search')!
    if (c.req.query('archetype')) filter.target_archetype = c.req.query('archetype')!

    const result = await services.perspectives.listPerspectives(filter, offset, limit)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data, pagination: result.pagination })
  })

  app.get('/perspectives/:id', async (c) => {
    const result = await services.perspectives.getPerspectiveWithChain(c.req.param('id'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.patch('/perspectives/:id', async (c) => {
    const body = await c.req.json()
    const result = await services.perspectives.updatePerspective(c.req.param('id'), body)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.delete('/perspectives/:id', async (c) => {
    const result = await services.perspectives.deletePerspective(c.req.param('id'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.body(null, 204)
  })

  app.patch('/perspectives/:id/approve', async (c) => {
    const result = await services.perspectives.approvePerspective(c.req.param('id'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.patch('/perspectives/:id/reject', async (c) => {
    const body = await c.req.json<{ rejection_reason?: string }>()
    const result = await services.perspectives.rejectPerspective(c.req.param('id'), body.rejection_reason ?? '')
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.patch('/perspectives/:id/reopen', async (c) => {
    const result = await services.perspectives.reopenPerspective(c.req.param('id'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.patch('/perspectives/:id/submit', async (c) => {
    const result = await services.perspectives.submitPerspective(c.req.param('id'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  return app
}
