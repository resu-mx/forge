/**
 * Hono app setup with middleware.
 *
 * Creates the app instance, adds middleware (CORS, request logging,
 * X-Request-Id, error handler), and exports the createApp factory.
 */

import { Hono } from 'hono'
import { cors } from 'hono/cors'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import type { Database } from 'bun:sqlite'
import type { Services } from '../services'
import { logger } from '../lib/logger'
import { parseCorsOrigins } from './cors-origins'
import { sourceRoutes } from './sources'
import { bulletRoutes } from './bullets'
import { perspectiveRoutes } from './perspectives'
import { resumeRoutes } from './resumes'
import { reviewRoutes } from './review'
import { supportingRoutes } from './supporting'
import { auditRoutes } from './audit'
import { organizationRoutes } from './organizations'
import { noteRoutes } from './notes'
import { integrityRoutes } from './integrity'
import { domainRoutes } from './domains'
import { archetypeRoutes } from './archetypes'
import { jobDescriptionRoutes } from './job-descriptions'
import { templateRoutes } from './templates'
import { profileRoutes } from './profile'
import { exportRoutes } from './export'
import { summaryRoutes } from './summaries'
import { campusRoutes } from './campuses'
import { contactRoutes } from './contacts'
import { industryRoutes } from './industries'
import { roleTypeRoutes } from './role-types'
import { credentialRoutes } from './credentials'
import { certificationRoutes } from './certifications'
import { alignmentRoutes } from './alignment'
import { derivationRoutes } from './derivations'
import { addressRoutes } from './addresses'
import { answerBankRoutes } from './answer-bank'
import { extensionRoutes } from './extension'

/** Map error codes to HTTP status codes. */
// Re-export for backward compatibility — prefer importing from './status-codes' directly
export { mapStatusCode } from './status-codes'

/** Read server version from package.json. Called during createApp(), not at module scope. */
function readServerVersion(): string {
  try {
    const pkg = JSON.parse(readFileSync(join(import.meta.dir, '../../package.json'), 'utf-8'))
    return pkg.version ?? 'unknown'
  } catch {
    return 'unknown'
  }
}

/** Create the Hono app with all middleware and routes mounted. */
export function createApp(services: Services, db: Database) {
  const app = new Hono().basePath('/api')

  // ── Middleware ──────────────────────────────────────────────────────

  // CORS
  app.use(
    '*',
    cors({
      origin: (origin) => {
        // An operator's list replaces the built-in policy below, in dev and in production alike
        // (resu-mx/forge#126). Unset or blank keeps the defaults.
        const configured = parseCorsOrigins(process.env.FORGE_CORS_ORIGINS)
        if (configured === '*') return '*'
        if (configured) return origin && configured.includes(origin) ? origin : undefined

        // Production: same-origin (served from same server)
        if (process.env.NODE_ENV === 'production') {
          return '*'
        }
        // Development: allow webui + Chrome/Firefox extension origins
        if (!origin) return undefined
        const allowedOrigins = ['http://localhost:5173', 'http://127.0.0.1:5173']
        if (allowedOrigins.includes(origin)) return origin
        // Browser extension origins: chrome-extension:// or moz-extension://
        if (origin.startsWith('chrome-extension://') || origin.startsWith('moz-extension://')) {
          return origin
        }
        return undefined
      },
    }),
  )

  // X-Request-Id + structured request logging
  app.use('*', async (c, next) => {
    const requestId = crypto.randomUUID()
    c.set('requestId', requestId)
    c.header('X-Request-Id', requestId)

    const start = performance.now()
    await next()
    const duration_ms = Math.round((performance.now() - start) * 10) / 10

    const fields = {
      method: c.req.method,
      path: c.req.path,
      route: c.req.routePath ?? c.req.path,
      status: c.res.status,
      duration_ms,
      request_id: requestId,
    }

    // Slow request detection: 200 response >500ms is a warn, not info
    if (c.res.status >= 500) {
      logger.error(fields)
    } else if (c.res.status >= 400 || duration_ms > 500) {
      logger.warn(fields)
    } else {
      logger.info(fields)
    }
  })

  // ── Health check ───────────────────────────────────────────────────

  const serverVersion = readServerVersion()
  app.get('/health', (c) => c.json({ data: { server: 'ok', version: serverVersion } }))

  // ── Routes ─────────────────────────────────────────────────────────

  app.route('/', sourceRoutes(services, db))
  app.route('/', bulletRoutes(services, db))
  app.route('/', perspectiveRoutes(services))
  app.route('/', resumeRoutes(services, db))
  app.route('/', reviewRoutes(services))
  app.route('/', supportingRoutes(services, db))
  app.route('/', auditRoutes(services))
  app.route('/', organizationRoutes(services))
  app.route('/', noteRoutes(services))
  app.route('/', integrityRoutes(services))
  app.route('/', domainRoutes(services))
  app.route('/', archetypeRoutes(services))
  app.route('/', jobDescriptionRoutes(services, db))
  app.route('/', templateRoutes(services))
  app.route('/', profileRoutes(services))
  app.route('/', summaryRoutes(services))
  app.route('/', campusRoutes(services, db))
  app.route('/', exportRoutes(services))
  app.route('/', contactRoutes(services, db))
  app.route('/', industryRoutes(services))
  app.route('/', roleTypeRoutes(services))
  app.route('/', credentialRoutes(services))
  app.route('/', certificationRoutes(services))
  app.route('/', alignmentRoutes(services))
  app.route('/', derivationRoutes(services))
  app.route('/', addressRoutes(services))
  app.route('/', answerBankRoutes(services))
  app.route('/', extensionRoutes(services))

  // ── Global error handler ───────────────────────────────────────────

  app.onError((err, c) => {
    logger.error({
      msg: 'Unhandled error',
      error: err.message,
      stack: process.env.NODE_ENV !== 'production' ? err.stack : undefined,
    })
    return c.json(
      {
        error: {
          code: 'INTERNAL_ERROR',
          message: process.env.NODE_ENV === 'production'
            ? 'Internal server error'
            : err.message,
        },
      },
      500,
    )
  })

  // ── 404 fallback ───────────────────────────────────────────────────

  app.notFound((c) =>
    c.json(
      { error: { code: 'NOT_FOUND', message: `Route not found: ${c.req.method} ${c.req.path}` } },
      404,
    ),
  )

  return app
}
