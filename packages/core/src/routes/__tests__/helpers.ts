/**
 * Test helpers for route tests.
 *
 * By default `createTestApp()` creates a fully wired Hono app backed by an
 * in-memory database and requests are served in-process.
 *
 * Parity mode: set `FORGE_TEST_SERVER_BIN` to the path of the Rust `forge-server`
 * binary and the same tests run against it instead. Each `createTestApp()` call
 * migrates a fresh temp database file with the TS runner, spawns the Rust server
 * on it, and `app.request()` becomes a real HTTP request. `ctx.db` is a second
 * connection to the same file, so tests can still seed and inspect rows directly.
 * Run it with `just parity`.
 */

import { Database } from 'bun:sqlite'
import { rmSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { createTestDb } from '../../db/__tests__/helpers'
import { createServices } from '../../services'
import { createApp } from '../server'

/** The subset of Hono's `app.request` that the route tests use. */
export interface AppLike {
  request(input: string | URL | Request, init?: RequestInit): Response | Promise<Response>
}

export interface TestContext {
  app: AppLike
  db: Database
}

// ── Parity mode: Rust server ────────────────────────────────────────

let counter = 0
let current: { proc: { kill(): void }; dbPath: string } | undefined

function stopCurrent() {
  if (!current) return
  current.proc.kill()
  for (const suffix of ['', '-wal', '-shm']) rmSync(current.dbPath + suffix, { force: true })
  current = undefined
}
process.on('exit', stopCurrent)

function createRustTestApp(bin: string): TestContext {
  // Tests run sequentially and each wants a fresh database, so the previous
  // server is no longer needed.
  stopCurrent()

  const dbPath = join(tmpdir(), `forge-parity-${process.pid}-${++counter}.db`)
  const db = createTestDb(dbPath)
  const port = 30000 + Math.floor(Math.random() * 30000)
  const proc = Bun.spawn([bin], {
    env: { ...process.env, FORGE_DB_PATH: dbPath, FORGE_PORT: String(port), FORGE_LOG_LEVEL: 'warn' },
    stdout: 'ignore',
    stderr: 'inherit',
  })
  current = { proc, dbPath }

  const base = `http://127.0.0.1:${port}`
  let ready: Promise<void> | undefined
  const waitReady = () =>
    (ready ??= (async () => {
      for (let i = 0; i < 400; i++) {
        try {
          if ((await fetch(`${base}/api/health`)).ok) return
        } catch {
          // not listening yet
        }
        await Bun.sleep(25)
      }
      throw new Error(`forge-server did not start on port ${port}`)
    })())

  const app: AppLike = {
    async request(input, init) {
      await waitReady()
      const url = new URL(typeof input === 'string' || input instanceof URL ? input : input.url, 'http://localhost')
      const rest = typeof input === 'object' && 'method' in input ? input : undefined
      return fetch(base + url.pathname + url.search, rest ?? init)
    },
  }
  return { app, db }
}

// ── Default: in-process Hono ────────────────────────────────────────

/** Create a test app with in-memory database, migrations applied. */
export function createTestApp(): TestContext {
  const bin = process.env.FORGE_TEST_SERVER_BIN
  if (bin) return createRustTestApp(bin)

  const db = createTestDb()
  const services = createServices(db, ':memory:')
  const app = createApp(services, db)
  return { app, db }
}

/** Helper to make JSON requests against the test app. */
export async function apiRequest(
  app: AppLike,
  method: string,
  path: string,
  body?: unknown,
): Promise<Response> {
  const url = `http://localhost/api${path}`
  const init: RequestInit = { method }
  if (body !== undefined) {
    init.body = JSON.stringify(body)
    init.headers = { 'Content-Type': 'application/json' }
  }
  return app.request(url, init)
}
