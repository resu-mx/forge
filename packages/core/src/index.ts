/**
 * @forge/core — Forge Resume Builder API Server entrypoint.
 *
 * Startup sequence:
 * 1. Validate environment variables
 * 2. Connect to SQLite database
 * 3. Run pending migrations
 * 4. Recover stale deriving locks
 * 5. Check Claude CLI availability
 * 6. Create services
 * 7. Mount routes
 * 8. Start HTTP server
 */

import { resolve, dirname } from 'path'
import { mkdirSync } from 'fs'
import { getDatabase, DB_PATH_REQUIRED_MESSAGE } from './db/connection'
import { runMigrations } from './db/migrate'
import { createServices } from './services'
import { DerivationService } from './services/derivation-service'
import { createApp } from './routes/server'
import { logger } from './lib/logger'

// ── 1. Environment variables ─────────────────────────────────────────

const PORT = parseInt(process.env.FORGE_PORT ?? '3000', 10)
const DB_PATH = process.env.FORGE_DB_PATH

if (!DB_PATH) {
  console.error(DB_PATH_REQUIRED_MESSAGE)
  process.exit(1)
}

if (isNaN(PORT) || PORT < 1 || PORT > 65535) {
  console.error(`FORGE_PORT must be a valid port number (1-65535), got: ${process.env.FORGE_PORT}`)
  process.exit(1)
}

// ── 2. Database connection ───────────────────────────────────────────

const dbPath = resolve(DB_PATH)
mkdirSync(dirname(dbPath), { recursive: true })
const db = getDatabase(dbPath)
logger.info({ msg: 'Database connected', path: dbPath })

// ── 3. Migrations ────────────────────────────────────────────────────

const migrationsDir = resolve(import.meta.dir, 'db/migrations')
runMigrations(db, migrationsDir)

// ── 4. Recover stale locks ───────────────────────────────────────────

const recovered = await DerivationService.recoverStaleLocks(db)
if (recovered > 0) {
  logger.warn({ msg: 'Recovered stale deriving locks', count: recovered })
}

// ── 5. Claude CLI check ──────────────────────────────────────────────

const claudePath = process.env.FORGE_CLAUDE_PATH ?? 'claude'
try {
  const proc = Bun.spawnSync([claudePath, '--version'], { stdout: 'pipe', stderr: 'pipe' })
  if (proc.exitCode === 0) {
    const version = proc.stdout.toString().trim()
    logger.info({ msg: 'Claude CLI available', version })
  } else {
    logger.warn({ msg: 'Claude CLI not found', path: claudePath, note: 'AI features will be unavailable' })
  }
} catch {
  logger.warn({ msg: 'Claude CLI not found', path: claudePath, note: 'AI features will be unavailable' })
}

// ── 5b. Tectonic check ─────────────────────────────────────────────

try {
  const tecProc = Bun.spawnSync(['tectonic', '--version'], { stdout: 'pipe', stderr: 'pipe' })
  if (tecProc.exitCode === 0) {
    const version = tecProc.stdout.toString().trim()
    logger.info({ msg: 'Tectonic available', version })
  } else {
    logger.warn({ msg: 'Tectonic not found', note: 'PDF generation will be unavailable' })
  }
} catch {
  logger.warn({ msg: 'Tectonic not found', note: 'PDF generation will be unavailable' })
}

// ── 6. Initialize embedding service (before createServices) ─────────
// EmbeddingService loads the ML model asynchronously (~2-5s on first use).
// Phase 1.0: the embedding service is constructed BEFORE createServices
// so the entity map can close over it in lifecycle hooks (afterCreate
// hooks fire embeddings for bullets/perspectives/sources/JD requirements
// via the integrity layer rather than service-level queueMicrotask calls).
import { EmbeddingService } from './services/embedding-service'
import { buildDefaultElm } from './storage/build-elm'

const embeddingService = new EmbeddingService(buildDefaultElm(db))

// ── 6b. Create services with embedding wired into the entity map ────

const services = createServices(db, dbPath, { embeddingService })
services.embedding = embeddingService

// JD service still uses the embedding service DIRECTLY for parse-and-multi-embed
// semantics (entity-map hooks can't express the jd_requirement:{i} pattern).
// Source service embedding is fully handled by entity-map afterCreate hooks.
services.jobDescriptions.setEmbeddingService(embeddingService)

// ── 7. Mount routes ──────────────────────────────────────────────────

const app = createApp(services, db)

// ── 8. Start server ──────────────────────────────────────────────────

export default {
  port: PORT,
  fetch: app.fetch,
}

logger.info({ msg: 'Forge API server listening', url: `http://localhost:${PORT}`, port: PORT, log_level: logger.getLevel() })
