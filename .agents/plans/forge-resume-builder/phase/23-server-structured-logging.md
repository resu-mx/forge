# Phase 23: Server Structured Logging

**Goal:** Replace string-based `console.log` calls in the Hono server with structured JSON logging and level filtering, controlled by the `FORGE_LOG_LEVEL` environment variable.

**Non-Goals:** Log aggregation, log shipping to external backends, log rotation, distributed tracing (see OTel spec). Client-side logging (Phase 21). Component debug helpers (Phase 22). Production log management.

**Depends on:** Nothing (independent)
**Blocks:** Nothing
**Parallelizable with:** Phase 21, Phase 22

**Internal task parallelization:** T23.1 must complete first. T23.2 and T23.3 depend on T23.1 but can run in parallel with each other. T23.4 depends on T23.2 and T23.3.

**Tech Stack:** TypeScript, Hono, `bun:test`, `console.debug/log/warn/error`

**Reference:** `refs/specs/2026-03-30-observability-structured-logging.md` section 3

**Architecture:**
- Logger class lives in `packages/core/src/lib/logger.ts` (the `lib/` directory already exists with escape utilities and compilers)
- Server middleware in `packages/core/src/routes/server.ts` uses the logger
- Startup code in `packages/core/src/index.ts` uses the logger
- Logger tests in `packages/core/src/lib/__tests__/logger.test.ts`

**Fallback Strategies:**
- If `c.req.routePath` is unavailable in some Hono versions, fall back to `c.req.path` for the `route` field
- If `FORGE_LOG_LEVEL` is set to an invalid value, default to `'info'` and log a warning at startup

---

## Context

The current server logging is string-based:
```
GET /api/health 200 0.1ms [request-id]
```

This is adequate for human reading but makes it difficult to:
- Filter logs by level (debug vs info vs error)
- Parse logs programmatically (e.g., to find slow requests)
- Distinguish between the matched route pattern (`/api/resumes/:id`) and the actual path (`/api/resumes/abc-123`)
- Detect slow requests that return 200 but took >500ms

This phase replaces the inline logging with a `Logger` class that outputs structured JSON and supports level filtering via `FORGE_LOG_LEVEL`.

Current logging locations:
1. **Request middleware** (`packages/core/src/routes/server.ts` line 71): `console.log/warn/error` based on status code
2. **Error handler** (`packages/core/src/routes/server.ts` line 96): `console.error('Unhandled error:', err)`
3. **Startup** (`packages/core/src/index.ts`): `console.log` for database path, Claude CLI version, Tectonic version, server URL; `console.warn` for recovered locks, missing CLI tools

---

## Tasks

### Task 23.1: Create Logger class

**Files to create:** `packages/core/src/lib/logger.ts`, `packages/core/src/lib/__tests__/logger.test.ts`

**Goal:** Implement a structured JSON logger with level filtering and zero external dependencies.

#### Steps

- [ ] **Create `packages/core/src/lib/logger.ts`:**

```typescript
/**
 * Structured JSON logger with level filtering.
 *
 * Outputs JSON objects to the appropriate console method based on log level.
 * Level is controlled by the FORGE_LOG_LEVEL env var (default: 'info').
 *
 * Log levels (in order of severity):
 *   debug < info < warn < error
 *
 * Setting FORGE_LOG_LEVEL=debug enables all levels.
 * Setting FORGE_LOG_LEVEL=warn suppresses debug and info.
 */

export type LogLevel = 'debug' | 'info' | 'warn' | 'error'

const LEVEL_ORDER: Record<LogLevel, number> = {
  debug: 0,
  info: 1,
  warn: 2,
  error: 3,
}

function isValidLevel(level: string): level is LogLevel {
  return level in LEVEL_ORDER
}

/**
 * Structured JSON logger with level filtering.
 *
 * Outputs JSON objects to the appropriate console method based on log level.
 * Level hierarchy: debug < info < warn < error.
 * Messages below the configured level are suppressed.
 */
export class Logger {
  private level: LogLevel
  private levelNum: number

  constructor(level?: LogLevel | string) {
    if (level && isValidLevel(level)) {
      this.level = level
    } else {
      this.level = 'info'
    }
    this.levelNum = LEVEL_ORDER[this.level]
  }

  /** Current configured log level. */
  getLevel(): LogLevel {
    return this.level
  }

  /** Log at debug level. Suppressed when level >= info. Uses console.debug. */
  debug(fields: Record<string, unknown>): void {
    if (this.levelNum > LEVEL_ORDER.debug) return
    console.debug(JSON.stringify({ level: 'debug', ts: new Date().toISOString(), ...fields }))
  }

  /** Log at info level. Suppressed when level >= warn. Uses console.log. */
  info(fields: Record<string, unknown>): void {
    if (this.levelNum > LEVEL_ORDER.info) return
    console.log(JSON.stringify({ level: 'info', ts: new Date().toISOString(), ...fields }))
  }

  /** Log at warn level. Suppressed when level >= error. Uses console.warn. */
  warn(fields: Record<string, unknown>): void {
    if (this.levelNum > LEVEL_ORDER.warn) return
    console.warn(JSON.stringify({ level: 'warn', ts: new Date().toISOString(), ...fields }))
  }

  /** Log at error level. Never suppressed (highest level). Uses console.error. */
  error(fields: Record<string, unknown>): void {
    // error is never filtered (highest level)
    console.error(JSON.stringify({ level: 'error', ts: new Date().toISOString(), ...fields }))
  }
}

// ---------------------------------------------------------------------------
// Singleton — initialized from FORGE_LOG_LEVEL env var
// ---------------------------------------------------------------------------

const envLevel = (typeof process !== 'undefined' && process.env?.FORGE_LOG_LEVEL) || 'info'
export const logger = new Logger(envLevel)
```

- [ ] **Create `packages/core/src/lib/__tests__/logger.test.ts`:**

```typescript
import { describe, expect, it, beforeEach, afterEach, spyOn } from 'bun:test'
import { Logger } from '../logger'
import type { LogLevel } from '../logger'

describe('Logger', () => {
  let debugSpy: ReturnType<typeof spyOn>
  let logSpy: ReturnType<typeof spyOn>
  let warnSpy: ReturnType<typeof spyOn>
  let errorSpy: ReturnType<typeof spyOn>

  beforeEach(() => {
    debugSpy = spyOn(console, 'debug').mockImplementation(() => {})
    logSpy = spyOn(console, 'log').mockImplementation(() => {})
    warnSpy = spyOn(console, 'warn').mockImplementation(() => {})
    errorSpy = spyOn(console, 'error').mockImplementation(() => {})
  })

  afterEach(() => {
    debugSpy.mockRestore()
    logSpy.mockRestore()
    warnSpy.mockRestore()
    errorSpy.mockRestore()
  })

  it('debug level enables all log methods', () => {
    const logger = new Logger('debug')
    logger.debug({ msg: 'test' })
    logger.info({ msg: 'test' })
    logger.warn({ msg: 'test' })
    logger.error({ msg: 'test' })
    expect(debugSpy).toHaveBeenCalledTimes(1)
    expect(logSpy).toHaveBeenCalledTimes(1)
    expect(warnSpy).toHaveBeenCalledTimes(1)
    expect(errorSpy).toHaveBeenCalledTimes(1)
  })

  it('info level suppresses debug', () => {
    const logger = new Logger('info')
    logger.debug({ msg: 'suppressed' })
    logger.info({ msg: 'visible' })
    logger.warn({ msg: 'visible' })
    logger.error({ msg: 'visible' })
    expect(debugSpy).not.toHaveBeenCalled()
    expect(logSpy).toHaveBeenCalledTimes(1)
    expect(warnSpy).toHaveBeenCalledTimes(1)
    expect(errorSpy).toHaveBeenCalledTimes(1)
  })

  it('warn level suppresses debug and info', () => {
    const logger = new Logger('warn')
    logger.debug({ msg: 'suppressed' })
    logger.info({ msg: 'suppressed' })
    logger.warn({ msg: 'visible' })
    logger.error({ msg: 'visible' })
    expect(debugSpy).not.toHaveBeenCalled()
    expect(logSpy).not.toHaveBeenCalled()
    expect(warnSpy).toHaveBeenCalledTimes(1)
    expect(errorSpy).toHaveBeenCalledTimes(1)
  })

  it('error level suppresses everything except error', () => {
    const logger = new Logger('error')
    logger.debug({ msg: 'suppressed' })
    logger.info({ msg: 'suppressed' })
    logger.warn({ msg: 'suppressed' })
    logger.error({ msg: 'visible' })
    expect(debugSpy).not.toHaveBeenCalled()
    expect(logSpy).not.toHaveBeenCalled()
    expect(warnSpy).not.toHaveBeenCalled()
    expect(errorSpy).toHaveBeenCalledTimes(1)
  })

  it('outputs structured JSON with level and ts fields', () => {
    const logger = new Logger('debug')
    logger.info({ method: 'GET', path: '/api/health', status: 200 })
    expect(logSpy).toHaveBeenCalledTimes(1)
    const output = logSpy.mock.calls[0][0] as string
    const parsed = JSON.parse(output)
    expect(parsed.level).toBe('info')
    expect(parsed.ts).toBeDefined()
    expect(parsed.method).toBe('GET')
    expect(parsed.path).toBe('/api/health')
    expect(parsed.status).toBe(200)
  })

  it('defaults to info when given an invalid level', () => {
    const logger = new Logger('banana' as LogLevel)
    expect(logger.getLevel()).toBe('info')
    logger.debug({ msg: 'suppressed' })
    logger.info({ msg: 'visible' })
    expect(debugSpy).not.toHaveBeenCalled()
    expect(logSpy).toHaveBeenCalledTimes(1)
  })

  it('defaults to info when no level is provided', () => {
    const logger = new Logger()
    expect(logger.getLevel()).toBe('info')
  })

  it('uses console.debug for debug, console.log for info, console.warn for warn, console.error for error', () => {
    const logger = new Logger('debug')
    logger.debug({ msg: 'd' })
    logger.info({ msg: 'i' })
    logger.warn({ msg: 'w' })
    logger.error({ msg: 'e' })
    expect(debugSpy).toHaveBeenCalledTimes(1)
    expect(logSpy).toHaveBeenCalledTimes(1)
    expect(warnSpy).toHaveBeenCalledTimes(1)
    expect(errorSpy).toHaveBeenCalledTimes(1)
  })
})
```

#### Acceptance Criteria
- [ ] `Logger` constructor takes an optional level (default: `'info'`)
- [ ] Invalid level strings fall back to `'info'`
- [ ] `debug()` suppressed when level >= info
- [ ] `info()` suppressed when level >= warn
- [ ] `warn()` suppressed when level >= error
- [ ] `error()` never suppressed
- [ ] Output is structured JSON with `level`, `ts`, and caller-provided fields
- [ ] Each method uses the correct `console` method: `debug`/`log`/`warn`/`error`
- [ ] Singleton `logger` export reads from `FORGE_LOG_LEVEL` env var
- [ ] All 8 tests pass

---

### Task 23.2: Update server middleware

**File to modify:** `packages/core/src/routes/server.ts`

**Goal:** Replace inline `console.log/warn/error` calls with structured `logger` calls, add route pattern field, and add slow request detection.

#### Steps

- [ ] **Add import at top of file:**

```typescript
import { logger } from '../lib/logger'
```

- [ ] **Replace the request logging middleware** (lines 60-72). Replace the current X-Request-Id + logging middleware with:

```typescript
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
```

Key changes from the current implementation:
1. Uses `logger.info/warn/error` instead of `console.log/warn/error`
2. Adds `route` field from `c.req.routePath` (Hono provides the matched route pattern after `await next()`)
3. Adds slow request detection: any request taking >500ms is logged at `warn` regardless of status code
4. Duration rounded to 1 decimal place for consistency

- [ ] **Replace the global error handler** (lines 95-108):

```typescript
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
```

- [ ] **Add automated test for slow request warn** to `packages/core/src/routes/__tests__/server.test.ts` (or a new test file alongside it):

```typescript
test('slow request (>500ms) logged at warn level', async () => {
  // Use createTestApp which applies all middleware
  const { app, db } = createTestApp()

  // Mock performance.now to simulate a slow response
  const originalNow = performance.now
  let callCount = 0
  performance.now = () => {
    callCount++
    // First call (start): return 0
    // Second call (end): return 600 (>500ms)
    return callCount === 1 ? 0 : 600
  }

  const warnSpy = spyOn(console, 'warn').mockImplementation(() => {})

  const res = await app.request('http://localhost/api/health')
  expect(res.status).toBe(200)

  // Should have logged at warn level due to >500ms duration
  expect(warnSpy).toHaveBeenCalled()

  warnSpy.mockRestore()
  performance.now = originalNow
  db.close()
})
```

#### Acceptance Criteria
- [ ] Request logging uses `logger.info/warn/error` instead of `console.log/warn/error`
- [ ] Output is structured JSON: `{"level":"info","ts":"...","method":"GET","path":"/api/resumes/abc","route":"/api/resumes/:id","status":200,"duration_ms":1.2,"request_id":"..."}`
- [ ] 4xx responses logged at `warn`
- [ ] 5xx responses logged at `error`
- [ ] 200 responses taking >500ms logged at `warn` (slow request detection)
- [ ] 200 responses under 500ms logged at `info`
- [ ] `route` field populated with Hono route pattern (e.g., `/api/resumes/:id`) not the actual path
- [ ] Error handler uses `logger.error` with structured fields including `error` and optional `stack`
- [ ] All existing request-logging behavior preserved (X-Request-Id header set, timing measured)
- [ ] Automated test verifies slow request warn behavior

**Implementation Note:** For 404 responses from the `notFound` handler, `c.req.routePath` may be undefined or the raw path (no route pattern matched). The structured log entry will have `route: null` for these cases, which is expected behavior.

---

### Task 23.3: Update startup logging

**File to modify:** `packages/core/src/index.ts`

**Goal:** Replace `console.log` and `console.warn` startup messages with structured `logger` calls.

#### Steps

- [ ] **Add import at top of file:**

```typescript
import { logger } from './lib/logger'
```

- [ ] **Replace startup logging calls.** Replace each `console.log`/`console.warn` with the appropriate `logger` method. The startup section should become:

```typescript
// ── 2. Database connection ───────────────────────────────────────────

const dbPath = resolve(DB_PATH)
mkdirSync(dirname(dbPath), { recursive: true })
const db = getDatabase(dbPath)
logger.info({ msg: 'Database connected', path: dbPath })

// ── 3. Migrations ────────────────────────────────────────────────────

const migrationsDir = resolve(import.meta.dir, 'db/migrations')
runMigrations(db, migrationsDir)

// ── 4. Recover stale locks ───────────────────────────────────────────

const recovered = DerivationService.recoverStaleLocks(db)
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
```

- [ ] **Replace the final startup message:**

```typescript
logger.info({ msg: 'Forge API server listening', url: `http://localhost:${PORT}`, port: PORT, log_level: logger.getLevel() })
```

Note: The logger's `getLevel()` method is included in the startup log so operators can see what log level is active.

Wait -- the singleton logger is imported. We need to verify it reads `FORGE_LOG_LEVEL` from the env. The singleton in `logger.ts` reads `process.env.FORGE_LOG_LEVEL` at module load time, which happens during the `import` at the top of `index.ts`. Since env vars are set before the process starts, this works correctly.

#### Acceptance Criteria
- [ ] All `console.log` calls in `index.ts` replaced with `logger.info`
- [ ] All `console.warn` calls in `index.ts` replaced with `logger.warn`
- [ ] The `console.error` for missing `FORGE_DB_PATH` and invalid `FORGE_PORT` remain as `console.error` + `process.exit(1)` (these fire before the logger is meaningful -- the process is about to exit)
- [ ] Startup messages include structured fields (path, version, count, url, port)
- [ ] Log level is included in the server startup message

---

### Task 23.4: Documentation

**File to create:** `docs/src/architecture/logging.md`

**Goal:** Document the structured log format, level configuration, and examples.

#### Steps

- [ ] **Create `docs/src/architecture/logging.md`:**

```markdown
# Structured Logging

Forge uses structured JSON logging on the server side, controlled by the `FORGE_LOG_LEVEL` environment variable.

## Configuration

Set `FORGE_LOG_LEVEL` in your `.env` file or environment:

| Value   | Output                              |
|---------|-------------------------------------|
| `debug` | All logs (verbose request details)  |
| `info`  | Normal requests + warnings + errors (default) |
| `warn`  | Warnings and errors only            |
| `error` | Errors only                         |

## Log Format

All server logs are JSON objects with at minimum `level` and `ts` fields:

```json
{"level":"info","ts":"2026-03-30T12:00:00.000Z","method":"GET","path":"/api/resumes/abc-123","route":"/api/resumes/:id","status":200,"duration_ms":1.2,"request_id":"a1b2c3d4-..."}
```

### Request Log Fields

| Field        | Type   | Description                                    |
|--------------|--------|------------------------------------------------|
| `level`      | string | `info`, `warn`, or `error`                     |
| `ts`         | string | ISO 8601 timestamp                             |
| `method`     | string | HTTP method (GET, POST, etc.)                  |
| `path`       | string | Actual request path                            |
| `route`      | string | Matched route pattern (e.g., `/api/resumes/:id`) |
| `status`     | number | HTTP response status code                      |
| `duration_ms`| number | Request duration in milliseconds               |
| `request_id` | string | X-Request-Id header value                      |

### Level Assignment

- **info**: 2xx/3xx responses under 500ms
- **warn**: 4xx responses, or any response taking > 500ms
- **error**: 5xx responses and unhandled exceptions

### Startup Log Fields

| Field       | Type   | Description                  |
|-------------|--------|------------------------------|
| `msg`       | string | Human-readable description   |
| `path`      | string | Database file path           |
| `version`   | string | Tool version (Claude, Tectonic) |
| `url`       | string | Server listen URL            |
| `port`      | number | Server listen port           |
| `log_level` | string | Active log level             |

## SDK Client Logging

The SDK client (`@forge/sdk`) has its own logging system using `console.debug`. See the SDK debug store documentation for details. SDK logging is separate from server logging and is configured via `ForgeClientOptions.debug`.
```

#### Acceptance Criteria
- [ ] File created at `docs/src/architecture/logging.md`
- [ ] Documents `FORGE_LOG_LEVEL` values and defaults
- [ ] Shows example JSON log output
- [ ] Lists all request log fields with types and descriptions
- [ ] Explains level assignment rules (4xx=warn, 5xx=error, slow=warn)
- [ ] Mentions SDK client logging as a separate system

---

## Testing Requirements

| Category | Test | Location |
|----------|------|----------|
| Unit | Logger debug level enables all methods | `packages/core/src/lib/__tests__/logger.test.ts` |
| Unit | Logger info level suppresses debug | `packages/core/src/lib/__tests__/logger.test.ts` |
| Unit | Logger warn level suppresses debug + info | `packages/core/src/lib/__tests__/logger.test.ts` |
| Unit | Logger error level suppresses debug + info + warn | `packages/core/src/lib/__tests__/logger.test.ts` |
| Unit | Logger outputs structured JSON with level + ts | `packages/core/src/lib/__tests__/logger.test.ts` |
| Unit | Logger invalid level defaults to info | `packages/core/src/lib/__tests__/logger.test.ts` |
| Unit | Logger no-arg constructor defaults to info | `packages/core/src/lib/__tests__/logger.test.ts` |
| Unit | Logger uses correct console methods | `packages/core/src/lib/__tests__/logger.test.ts` |
| Unit | Slow request (>500ms) logged at warn level | `packages/core/src/routes/__tests__/server.test.ts` |
| Smoke | Server outputs JSON logs on requests | Manual |
| Smoke | 404 request logged at warn | Manual |
| Smoke | Startup logs are structured JSON | Manual |

**Total:** 9 automated tests, 3 manual smoke tests

---

## Documentation Requirements

- [ ] JSDoc on `Logger` class and all public methods
- [ ] JSDoc on `LogLevel` type
- [ ] `docs/src/architecture/logging.md` created with format, fields, and configuration
