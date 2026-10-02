# Phase 21: SDK Debug Store + Client Logging

**Goal:** Add structured logging and a debug ring buffer to the ForgeClient SDK, enabling immediate diagnosis of client-side rendering issues (notably the infinite loading bug on the resumes page) without external tooling.

**Non-Goals:** Server-side logging changes (Phase 23). Svelte component debug helpers (Phase 22). Devtools panel UI. Log aggregation or persistence. OpenTelemetry integration.

**Depends on:** Nothing (can start immediately)
**Blocks:** Phase 22 (component helpers import `isDevMode` from `@forge/sdk`), Phase 23 (server logger is independent but shares patterns with the spec)
**Parallelizable with:** Phase 23 (server logging)

**Internal task parallelization:** T21.1 must complete first. T21.2 depends on T21.1. T21.3 depends on T21.2. T21.4 depends on T21.3.

**Tech Stack:** TypeScript, `bun:test`, `performance.now()`, `console.debug`

**Reference:** `refs/specs/2026-03-30-observability-structured-logging.md` sections 1.1-1.6

**Architecture:**
- `SDKLogEntry` type, `DebugOptions` interface, `DebugStore` class, `isDevMode()` utility live in `packages/sdk/src/debug.ts`
- `ForgeClient` wraps `request()` and `requestList()` with logging in `packages/sdk/src/client.ts`
- Barrel exports from `packages/sdk/src/index.ts`
- WebUI client (`packages/webui/src/lib/sdk.ts`) enables debug in dev mode
- CLI client (`packages/cli/src/client.ts`) keeps debug off by default

**Fallback Strategies:**
- If `import.meta.env` throws in Bun, `isDevMode()` catches and falls back to `process.env` -- already designed into the function
- If `performance.now()` is unavailable (very old runtimes), fall back to `Date.now()` -- unlikely but trivial guard
- If request/response header access fails, populate `request_id` as `undefined` rather than crashing

---

## Context

The ForgeClient SDK (`packages/sdk/src/client.ts`) currently has no observability. When a request fails silently or a component enters an infinite loading state, the only diagnostic tool is adding ad-hoc `console.log` calls. The resumes page (`packages/webui/src/routes/resumes/+page.svelte`) has a known infinite loading bug that cannot be diagnosed without seeing what SDK requests fire and how they resolve.

This phase adds:
1. A `DebugStore` ring buffer that captures the last N SDK request/response log entries
2. Console logging of every `request()` and `requestList()` call with timing and status
3. A public `forge.debug` property for programmatic inspection from browser devtools
4. An `isDevMode()` utility that works across Vite (browser), Bun (CLI/tests), and Node.js

---

## Tasks

### Task 21.1: Create DebugStore + SDKLogEntry types + isDevMode

**Files to create:** `packages/sdk/src/debug.ts`, `packages/sdk/src/__tests__/debug.test.ts`

**Goal:** Implement the debug store and dev mode detection as standalone utilities with no dependencies on ForgeClient.

#### Steps

- [ ] **Create `packages/sdk/src/debug.ts`** with the following exports:

```typescript
// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface SDKLogEntry {
  timestamp: string          // ISO 8601
  direction: 'response' | 'error'  // Request-direction entries are only logged to console.debug (the → line). They are NOT pushed to the ring buffer since they contain no response data. Only response and error entries are stored.
  method: string             // GET, POST, PATCH, DELETE
  path: string               // /api/resumes/abc-123
  status?: number            // HTTP status (response only)
  duration_ms?: number       // Time from request start to response
  ok?: boolean               // Result.ok value
  error_code?: string        // ForgeError.code if error
  error_message?: string     // ForgeError.message if error
  request_id?: string        // X-Request-Id from response header
  payload_size?: number      // Response body size in bytes (approximate)
  request_body_size?: number // Request body size in bytes (POST/PATCH/PUT only)
  pagination_total?: number    // Total count from paginated response (requestList only)
  pagination_offset?: number   // Offset used in paginated request (requestList only)
  pagination_limit?: number    // Limit used in paginated request (requestList only)
  request_body?: unknown       // Request body (when logPayloads: true, POST/PATCH only)
  response_body?: unknown      // Response body (when logPayloads: true, truncated to 10KB)
}

export interface DebugOptions {
  logToConsole?: boolean    // Default: true (uses console.debug)
  storeSize?: number        // Default: 100 (ring buffer max entries)
  logPayloads?: boolean     // Default: false (log request/response bodies)
}

// ---------------------------------------------------------------------------
// DebugStore
// ---------------------------------------------------------------------------

/**
 * Ring buffer that captures SDK request/response log entries for debugging.
 * Access via `forge.debug` on a ForgeClient instance.
 */
export class DebugStore {
  private entries: SDKLogEntry[] = []
  readonly maxSize: number
  readonly logToConsole: boolean
  readonly logPayloads: boolean
  enabled: boolean

  constructor(options?: DebugOptions | boolean) {
    if (typeof options === 'boolean') {
      this.enabled = options
      this.maxSize = 100
      this.logToConsole = true
      this.logPayloads = false
    } else if (options) {
      this.enabled = true
      this.maxSize = options.storeSize ?? 100
      this.logToConsole = options.logToConsole ?? true
      this.logPayloads = options.logPayloads ?? false
    } else {
      // undefined — auto-detect
      this.enabled = isDevMode()
      this.maxSize = 100
      this.logToConsole = true
      this.logPayloads = false
    }
  }

  /** Push a log entry to the ring buffer. Evicts oldest entry if at capacity (FIFO). */
  push(entry: SDKLogEntry): void {
    if (!this.enabled) return
    this.entries.push(entry)
    if (this.entries.length > this.maxSize) {
      this.entries.shift()
    }
  }

  /** Remove all entries from the buffer. */
  clear(): void {
    this.entries = []
  }

  /** Get all entries in chronological order (oldest first). */
  getAll(): SDKLogEntry[] {
    return [...this.entries]
  }

  /** Get only error entries (where ok === false). */
  getErrors(): SDKLogEntry[] {
    return this.entries.filter((e) => e.ok === false)
  }

  /** Get entries matching a path prefix (e.g., '/api/resumes'). */
  getByPath(pathPrefix: string): SDKLogEntry[] {
    return this.entries.filter((e) => e.path.startsWith(pathPrefix))
  }

  /** Get entries slower than the given threshold in milliseconds. */
  getSlow(thresholdMs: number): SDKLogEntry[] {
    return this.entries.filter(
      (e) => e.duration_ms !== undefined && e.duration_ms > thresholdMs,
    )
  }
}

// ---------------------------------------------------------------------------
// Dev mode detection
// ---------------------------------------------------------------------------

/** Detect dev mode across Vite (browser/SvelteKit) and Bun (CLI/tests). */
export function isDevMode(): boolean {
  try {
    // Vite injects import.meta.env at build time
    if (typeof import.meta.env !== 'undefined') {
      return import.meta.env.DEV === true
    }
  } catch {}
  // Fallback for Bun, Node.js, and other non-Vite runtimes
  if (typeof process !== 'undefined' && process.env) {
    return process.env.NODE_ENV !== 'production' || process.env.FORGE_DEBUG === 'true'
  }
  return false
}
```

- [ ] **Create `packages/sdk/src/__tests__/debug.test.ts`** with tests:

```typescript
import { describe, expect, it, beforeEach } from 'bun:test'
import { DebugStore, isDevMode } from '../debug'
import type { SDKLogEntry } from '../debug'

function makeEntry(overrides: Partial<SDKLogEntry> = {}): SDKLogEntry {
  return {
    timestamp: new Date().toISOString(),
    direction: 'response',
    method: 'GET',
    path: '/api/resumes',
    status: 200,
    duration_ms: 10,
    ok: true,
    ...overrides,
  }
}

describe('DebugStore', () => {
  let store: DebugStore

  beforeEach(() => {
    store = new DebugStore(true)
  })

  it('push + getAll returns entries in chronological order', () => {
    store.push(makeEntry({ path: '/api/a' }))
    store.push(makeEntry({ path: '/api/b' }))
    const all = store.getAll()
    expect(all).toHaveLength(2)
    expect(all[0].path).toBe('/api/a')
    expect(all[1].path).toBe('/api/b')
  })

  it('ring buffer overflow evicts oldest entries (FIFO)', () => {
    const small = new DebugStore({ storeSize: 100, logToConsole: false })
    for (let i = 0; i < 105; i++) {
      small.push(makeEntry({ path: `/api/item-${i}` }))
    }
    const all = small.getAll()
    expect(all).toHaveLength(100)
    // First 5 should be gone
    expect(all[0].path).toBe('/api/item-5')
    expect(all[99].path).toBe('/api/item-104')
  })

  it('clear empties the buffer', () => {
    store.push(makeEntry())
    store.push(makeEntry())
    store.clear()
    expect(store.getAll()).toHaveLength(0)
  })

  it('getErrors returns only entries with ok === false', () => {
    store.push(makeEntry({ ok: true }))
    store.push(makeEntry({ ok: false, error_code: 'NOT_FOUND' }))
    store.push(makeEntry({ ok: true }))
    store.push(makeEntry({ ok: false, error_code: 'NETWORK_ERROR' }))
    const errors = store.getErrors()
    expect(errors).toHaveLength(2)
    expect(errors[0].error_code).toBe('NOT_FOUND')
    expect(errors[1].error_code).toBe('NETWORK_ERROR')
  })

  it('getByPath filters by path prefix', () => {
    store.push(makeEntry({ path: '/api/resumes/abc' }))
    store.push(makeEntry({ path: '/api/bullets/def' }))
    store.push(makeEntry({ path: '/api/resumes/xyz' }))
    const resumes = store.getByPath('/api/resumes')
    expect(resumes).toHaveLength(2)
    expect(resumes[0].path).toBe('/api/resumes/abc')
    expect(resumes[1].path).toBe('/api/resumes/xyz')
  })

  it('getSlow returns entries above threshold', () => {
    store.push(makeEntry({ duration_ms: 10 }))
    store.push(makeEntry({ duration_ms: 600 }))
    store.push(makeEntry({ duration_ms: 200 }))
    store.push(makeEntry({ duration_ms: 501 }))
    const slow = store.getSlow(500)
    expect(slow).toHaveLength(2)
    expect(slow[0].duration_ms).toBe(600)
    expect(slow[1].duration_ms).toBe(501)
  })

  it('push is a no-op when disabled', () => {
    const disabled = new DebugStore(false)
    disabled.push(makeEntry())
    expect(disabled.getAll()).toHaveLength(0)
  })

  it('getAll returns a copy, not the internal array', () => {
    store.push(makeEntry())
    const all = store.getAll()
    all.length = 0
    expect(store.getAll()).toHaveLength(1)
  })

  it('auto-detect mode when no options provided', () => {
    // In Bun test environment, isDevMode() likely returns true (NODE_ENV != production)
    const store = new DebugStore()
    expect(store.enabled).toBe(true) // auto-detected
  })
})

describe('isDevMode', () => {
  it('returns true when NODE_ENV is not production', () => {
    const original = process.env.NODE_ENV
    process.env.NODE_ENV = 'development'
    expect(isDevMode()).toBe(true)
    process.env.NODE_ENV = original
  })

  it('returns false when NODE_ENV is production and FORGE_DEBUG is not set', () => {
    const origNode = process.env.NODE_ENV
    const origDebug = process.env.FORGE_DEBUG
    process.env.NODE_ENV = 'production'
    delete process.env.FORGE_DEBUG
    expect(isDevMode()).toBe(false)
    process.env.NODE_ENV = origNode
    if (origDebug !== undefined) process.env.FORGE_DEBUG = origDebug
  })

  it('returns true when NODE_ENV is production but FORGE_DEBUG=true', () => {
    const origNode = process.env.NODE_ENV
    const origDebug = process.env.FORGE_DEBUG
    process.env.NODE_ENV = 'production'
    process.env.FORGE_DEBUG = 'true'
    expect(isDevMode()).toBe(true)
    process.env.NODE_ENV = origNode
    if (origDebug !== undefined) {
      process.env.FORGE_DEBUG = origDebug
    } else {
      delete process.env.FORGE_DEBUG
    }
  })
})
```

#### Acceptance Criteria
- [ ] `DebugStore` constructor accepts `boolean | DebugOptions | undefined`
- [ ] `push()` adds entries; no-ops when `enabled` is false
- [ ] Ring buffer overflow: 105 entries with maxSize=100 yields 100 entries, first 5 evicted
- [ ] `clear()` empties the buffer
- [ ] `getAll()` returns a shallow copy in chronological order
- [ ] `getErrors()` filters where `ok === false`
- [ ] `getByPath(prefix)` filters by `path.startsWith(prefix)`
- [ ] `getSlow(ms)` filters where `duration_ms > threshold`
- [ ] `isDevMode()` correctly detects dev mode from `import.meta.env.DEV` and `process.env.NODE_ENV` / `process.env.FORGE_DEBUG`
- [ ] Auto-detect constructor path (undefined options) works correctly
- [ ] All 12+ tests pass

---

### Task 21.2: Wrap ForgeClient.request() with logging

**Files to modify:** `packages/sdk/src/client.ts`

**Goal:** Every `request()` and `requestList()` call produces a console.debug log line and pushes an SDKLogEntry to the debug store.

#### Steps

- [ ] **Import DebugStore and types at top of client.ts:**

```typescript
import { DebugStore } from './debug'
import type { DebugOptions, SDKLogEntry } from './debug'
```

- [ ] **Extend `ForgeClientOptions`** to add the debug field:

```typescript
export interface ForgeClientOptions {
  /** Base URL of the Forge API server, e.g. "http://localhost:3000" or "/api". */
  baseUrl: string
  /** Enable debug logging and ring buffer. true = on, false = off, undefined = auto-detect. */
  debug?: boolean | DebugOptions
}
```

- [ ] **Add `debug` property to `ForgeClient`** and initialize it in the constructor BEFORE binding request methods:

```typescript
export class ForgeClient {
  private baseUrl: string

  /** Debug store for programmatic inspection of SDK requests. */
  public debug: DebugStore

  // ... resource properties unchanged ...

  constructor(options: ForgeClientOptions) {
    this.baseUrl = options.baseUrl.replace(/\/+$/, '')

    // Initialize debug store BEFORE binding request methods
    if (typeof options.debug === 'boolean' || typeof options.debug === 'object') {
      this.debug = new DebugStore(options.debug)
    } else {
      this.debug = new DebugStore() // auto-detect
    }

    const req = this.request.bind(this)
    const reqList = this.requestList.bind(this)
    // ... resource initialization unchanged ...
  }
```

- [ ] **Wrap `request()` method** with logging before and after fetch:

```typescript
async request<T>(
  method: string,
  path: string,
  body?: unknown,
): Promise<Result<T>> {
  const start = performance.now()
  const bodySize = body !== undefined ? JSON.stringify(body).length : undefined

  // Log outgoing request
  if (this.debug.enabled && this.debug.logToConsole) {
    console.debug(`[forge:sdk] → ${method} ${path}`)
  }

  try {
    const headers: Record<string, string> = {}
    if (body !== undefined) {
      headers['Content-Type'] = 'application/json'
    }

    const response = await fetch(`${this.baseUrl}${path}`, {
      method,
      headers,
      body: body !== undefined ? JSON.stringify(body) : undefined,
    })

    const duration = performance.now() - start
    const requestId = response.headers.get('X-Request-Id') ?? undefined

    // 204 No Content
    if (response.status === 204) {
      const entry: SDKLogEntry = {
        timestamp: new Date().toISOString(),
        direction: 'response',
        method,
        path,
        status: 204,
        duration_ms: Math.round(duration * 10) / 10,
        ok: true,
        request_id: requestId,
        request_body_size: bodySize,
      }
      this.logResponse(entry)
      return { ok: true, data: undefined as T }
    }

    let json: Record<string, unknown>
    let rawText: string | undefined
    try {
      rawText = await response.text()
      json = JSON.parse(rawText) as Record<string, unknown>
    } catch {
      const entry: SDKLogEntry = {
        timestamp: new Date().toISOString(),
        direction: 'response',
        method,
        path,
        status: response.status,
        duration_ms: Math.round(duration * 10) / 10,
        ok: false,
        error_code: 'UNKNOWN_ERROR',
        error_message: `HTTP ${response.status}: non-JSON response`,
        request_id: requestId,
        payload_size: rawText?.length,
        request_body_size: bodySize,
      }
      this.logResponse(entry)
      return {
        ok: false,
        error: { code: 'UNKNOWN_ERROR', message: `HTTP ${response.status}: non-JSON response` },
      }
    }

    if (!response.ok) {
      const error = json.error as ForgeError | undefined
      const errorObj = error ?? { code: 'UNKNOWN_ERROR', message: `HTTP ${response.status}` }
      const entry: SDKLogEntry = {
        timestamp: new Date().toISOString(),
        direction: 'response',
        method,
        path,
        status: response.status,
        duration_ms: Math.round(duration * 10) / 10,
        ok: false,
        error_code: errorObj.code,
        error_message: errorObj.message,
        request_id: requestId,
        payload_size: rawText?.length,
        request_body_size: bodySize,
        ...(this.debug.logPayloads ? { request_body: body, response_body: rawText && rawText.length <= 10240 ? json : undefined } : {}),
      }
      this.logResponse(entry)
      return { ok: false, error: errorObj }
    }

    const entry: SDKLogEntry = {
      timestamp: new Date().toISOString(),
      direction: 'response',
      method,
      path,
      status: response.status,
      duration_ms: Math.round(duration * 10) / 10,
      ok: true,
      request_id: requestId,
      payload_size: rawText?.length,
      request_body_size: bodySize,
      ...(this.debug.logPayloads ? { request_body: body, response_body: rawText && rawText.length <= 10240 ? json : undefined } : {}),
    }
    this.logResponse(entry)
    return { ok: true, data: json.data as T }
  } catch (err) {
    const duration = performance.now() - start
    const entry: SDKLogEntry = {
      timestamp: new Date().toISOString(),
      direction: 'error',
      method,
      path,
      duration_ms: Math.round(duration * 10) / 10,
      ok: false,
      error_code: 'NETWORK_ERROR',
      error_message: String(err),
      request_body_size: bodySize,
    }
    if (this.debug.enabled && this.debug.logToConsole) {
      console.debug(`[forge:sdk] ✗ ${method} ${path} NETWORK_ERROR (${(entry.error_message ?? '').slice(0, 80)})`)
    }
    this.debug.push(entry)
    return { ok: false, error: { code: 'NETWORK_ERROR', message: String(err) } }
  }
}
```

- [ ] **Add private `logResponse` helper:**

```typescript
private logResponse(entry: SDKLogEntry): void {
  if (this.debug.enabled && this.debug.logToConsole) {
    const status = entry.ok ? 'ok' : `ERROR ${entry.error_code}`
    const rid = entry.request_id ? ` [${entry.request_id}]` : ''
    console.debug(
      `[forge:sdk] ← ${entry.method} ${entry.path} ${entry.status} ${entry.duration_ms}ms${rid} ${status}`,
    )
  }
  this.debug.push(entry)
}
```

- [ ] **Wrap `requestList()` method** with the same pattern, adding pagination fields:

```typescript
async requestList<T>(
  method: string,
  path: string,
  params?: Record<string, string>,
): Promise<PaginatedResult<T>> {
  const start = performance.now()

  if (this.debug.enabled && this.debug.logToConsole) {
    const qs = params && Object.keys(params).length > 0
      ? `?${new URLSearchParams(params).toString()}`
      : ''
    console.debug(`[forge:sdk] → ${method} ${path}${qs}`)
  }

  try {
    let url = `${this.baseUrl}${path}`
    if (params && Object.keys(params).length > 0) {
      const qs = new URLSearchParams(params).toString()
      url += `?${qs}`
    }

    const response = await fetch(url, { method })
    const duration = performance.now() - start
    const requestId = response.headers.get('X-Request-Id') ?? undefined

    let json: Record<string, unknown>
    let rawText: string | undefined
    try {
      rawText = await response.text()
      json = JSON.parse(rawText) as Record<string, unknown>
    } catch {
      const entry: SDKLogEntry = {
        timestamp: new Date().toISOString(),
        direction: 'response',
        method,
        path,
        status: response.status,
        duration_ms: Math.round(duration * 10) / 10,
        ok: false,
        error_code: 'UNKNOWN_ERROR',
        error_message: `HTTP ${response.status}: non-JSON response`,
        request_id: requestId,
        payload_size: rawText?.length,
      }
      this.logResponse(entry)
      return {
        ok: false,
        error: { code: 'UNKNOWN_ERROR', message: `HTTP ${response.status}: non-JSON response` },
      }
    }

    if (!response.ok) {
      const error = json.error as ForgeError | undefined
      const errorObj = error ?? { code: 'UNKNOWN_ERROR', message: `HTTP ${response.status}` }
      const entry: SDKLogEntry = {
        timestamp: new Date().toISOString(),
        direction: 'response',
        method,
        path,
        status: response.status,
        duration_ms: Math.round(duration * 10) / 10,
        ok: false,
        error_code: errorObj.code,
        error_message: errorObj.message,
        request_id: requestId,
        payload_size: rawText?.length,
      }
      this.logResponse(entry)
      return { ok: false, error: errorObj }
    }

    const pagination = json.pagination as { total: number; offset: number; limit: number }
    const entry: SDKLogEntry = {
      timestamp: new Date().toISOString(),
      direction: 'response',
      method,
      path,
      status: response.status,
      duration_ms: Math.round(duration * 10) / 10,
      ok: true,
      request_id: requestId,
      payload_size: rawText?.length,
      pagination_total: pagination?.total,
      pagination_offset: pagination?.offset,
      pagination_limit: pagination?.limit,
    }
    this.logResponse(entry)
    return { ok: true, data: json.data as T[], pagination }
  } catch (err) {
    const duration = performance.now() - start
    const entry: SDKLogEntry = {
      timestamp: new Date().toISOString(),
      direction: 'error',
      method,
      path,
      duration_ms: Math.round(duration * 10) / 10,
      ok: false,
      error_code: 'NETWORK_ERROR',
      error_message: String(err),
    }
    if (this.debug.enabled && this.debug.logToConsole) {
      console.debug(`[forge:sdk] ✗ ${method} ${path} NETWORK_ERROR (${String(err).slice(0, 80)})`)
    }
    this.debug.push(entry)
    return { ok: false, error: { code: 'NETWORK_ERROR', message: String(err) } }
  }
}
```

- [ ] **Add tests to `packages/sdk/src/__tests__/client.test.ts`** (append to existing file):

```typescript
describe('ForgeClient debug logging', () => {
  let client: ForgeClient

  beforeEach(() => {
    fetchMock = mock(() =>
      Promise.resolve(
        jsonResponse({ data: { id: '1' } }, { status: 200 }),
      ),
    )
    globalThis.fetch = fetchMock as typeof fetch
    client = new ForgeClient({ baseUrl: 'http://localhost:3000', debug: true })
  })

  it('captures successful request in debug store', async () => {
    await client.sources.get('abc')
    const entries = client.debug.getAll()
    expect(entries.length).toBeGreaterThanOrEqual(1)
    const last = entries[entries.length - 1]
    expect(last.method).toBe('GET')
    expect(last.path).toBe('/api/sources/abc')
    expect(last.ok).toBe(true)
    expect(last.duration_ms).toBeGreaterThanOrEqual(0)
  })

  it('captures error response in debug store', async () => {
    fetchMock = mock(() =>
      Promise.resolve(
        jsonResponse(
          { error: { code: 'NOT_FOUND', message: 'Source not found' } },
          { status: 404 },
        ),
      ),
    )
    globalThis.fetch = fetchMock as typeof fetch
    await client.sources.get('missing')
    const errors = client.debug.getErrors()
    expect(errors).toHaveLength(1)
    expect(errors[0].error_code).toBe('NOT_FOUND')
    expect(errors[0].status).toBe(404)
  })

  it('captures network error in debug store', async () => {
    fetchMock = mock(() => Promise.reject(new Error('fetch failed')))
    globalThis.fetch = fetchMock as typeof fetch
    await client.sources.get('abc')
    const errors = client.debug.getErrors()
    expect(errors).toHaveLength(1)
    expect(errors[0].direction).toBe('error')
    expect(errors[0].error_code).toBe('NETWORK_ERROR')
  })

  it('requestList() captures pagination fields in debug store', async () => {
    globalThis.fetch = mock(() => Promise.resolve(
      jsonResponse({ data: [{ id: '1' }], pagination: { total: 42, offset: 0, limit: 50 } })
    ))

    const client = new ForgeClient({ baseUrl: 'http://test', debug: true })
    await client.sources.list()

    const entries = client.debug.getAll()
    expect(entries.length).toBe(1)
    expect(entries[0].pagination_total).toBe(42)
    expect(entries[0].pagination_offset).toBe(0)
    expect(entries[0].pagination_limit).toBe(50)
  })

  it('captures X-Request-Id from response header', async () => {
    globalThis.fetch = mock(() => Promise.resolve(
      new Response(JSON.stringify({ data: { id: '1' } }), {
        status: 200,
        headers: { 'Content-Type': 'application/json', 'X-Request-Id': 'test-req-123' },
      })
    ))

    const client = new ForgeClient({ baseUrl: 'http://test', debug: true })
    await client.sources.get('1')

    const entries = client.debug.getAll()
    expect(entries[0].request_id).toBe('test-req-123')
  })
})
```

#### Acceptance Criteria
- [ ] `ForgeClient` constructor accepts `debug?: boolean | DebugOptions`
- [ ] `ForgeClient.debug` is a public `DebugStore` instance
- [ ] `this.debug` is initialized before `this.request.bind(this)`
- [ ] Every `request()` call logs `[forge:sdk] -> METHOD /path` and `[forge:sdk] <- METHOD /path STATUS DURATIONms` to `console.debug`
- [ ] Every `requestList()` call logs the same format with query string appended
- [ ] Network errors log as `[forge:sdk] ✗ METHOD /path NETWORK_ERROR`
- [ ] SDKLogEntry pushed to debug store with all fields populated
- [ ] `request_body_size` populated for POST/PATCH/PUT bodies
- [ ] `pagination_total/offset/limit` populated for `requestList()` responses
- [ ] `request_id` captured from `X-Request-Id` response header
- [ ] Existing `client.test.ts` tests still pass (backward compatible -- debug defaults to auto-detect)
- [ ] 5+ new client logging tests pass

**Note:** `logPayloads: true` is a documented option but is not tested in this phase -- the test assertions for request/response body capture would be verbose and the feature is straightforward. Add to Future Work if coverage is desired.

---

### Task 21.3: Export from SDK barrel + update WebUI/CLI clients

**Files to modify:**
- `packages/sdk/src/index.ts`
- `packages/webui/src/lib/sdk.ts`
- `packages/cli/src/client.ts`

**Goal:** Export the new types and classes from the SDK barrel, and wire up debug mode in the WebUI and CLI client instantiations.

#### Steps

- [ ] **Add exports to `packages/sdk/src/index.ts`:**

```typescript
// Debug store + utilities
export { DebugStore, isDevMode } from './debug'
export type { SDKLogEntry, DebugOptions } from './debug'
```

Place this after the `ForgeClientOptions` export and before the Result types.

- [ ] **Update `packages/webui/src/lib/sdk.ts`** to enable debug in dev mode:

```typescript
import { ForgeClient, isDevMode } from '@forge/sdk'
import type { ForgeError } from '@forge/sdk'

export const forge = new ForgeClient({ baseUrl: '', debug: true })

// Expose forge on window in dev mode for console debugging
// Usage: forge.debug.getAll() in browser console
if (typeof window !== 'undefined' && isDevMode()) {
  ;(window as unknown as Record<string, unknown>).forge = forge
}

/**
 * Convert an API error to a user-friendly message.
 * Detects when the API server is unreachable and shows a helpful hint.
 */
export function friendlyError(error: ForgeError, fallback?: string): string {
  if (
    error.code === 'NETWORK_ERROR' ||
    error.code === 'UNKNOWN_ERROR' &&
    (error.message.includes('non-JSON') || error.message.includes('502'))
  ) {
    return 'Cannot connect to the Forge API server. Start it with: just api'
  }
  return fallback ? `${fallback}: ${error.message}` : error.message
}
```

Note: `debug: true` is always on in the WebUI build because this is a single-user local tool. The `console.debug` logs are hidden by default in Chrome DevTools (must enable 'Verbose' level), so there is no noise penalty. If we later want auto-detection, change to `debug: undefined` and let `isDevMode()` handle it.

- [ ] **Update `packages/cli/src/client.ts`** to document FORGE_DEBUG support without enabling it by default:

```typescript
import { ForgeClient } from '@forge/sdk'
import type { ForgeError } from '@forge/sdk'

// ---------------------------------------------------------------------------
// SDK client singleton
// ---------------------------------------------------------------------------

const baseUrl = process.env.FORGE_API_URL ?? 'http://localhost:3000'

// Debug logging is OFF by default in CLI to avoid interleaving debug output
// with user-facing command output. Enable for a single command with:
//   FORGE_DEBUG=true forge source list
export const forge = new ForgeClient({
  baseUrl,
  debug: process.env.FORGE_DEBUG === 'true',
})

// ... rest of file unchanged ...
```

#### Acceptance Criteria
- [ ] `import { DebugStore, isDevMode } from '@forge/sdk'` works
- [ ] `import type { SDKLogEntry, DebugOptions } from '@forge/sdk'` works
- [ ] WebUI ForgeClient created with `debug: true`
- [ ] CLI ForgeClient created with `debug: process.env.FORGE_DEBUG === 'true'`
- [ ] CLI debug remains off by default (no interleaved output with normal CLI usage)

---

### Task 21.4: Verify with the actual bug

**Files:** None created or modified. This is a diagnostic task.

**Goal:** Use the new debug logging to identify the root cause of the infinite loading bug on the resumes page.

#### Steps

- [ ] Start the API server and WebUI dev server
- [ ] Navigate to `http://localhost:5173/resumes`
- [ ] Open browser DevTools, enable 'Verbose' log level
- [ ] Observe `[forge:sdk]` logs in the console
- [ ] If the page loads: select a resume, observe the request flow
- [ ] Check: do requests fire? Do they complete? Do they error?
- [ ] Open browser console and inspect `forge.debug.getAll()` for the full request history (available because `forge` is exposed on `window` in dev mode -- see T21.3)
- [ ] Document findings: which request fails, what status/error, timing

#### Acceptance Criteria
- [ ] `[forge:sdk]` logs appear in browser console when navigating to resumes page
- [ ] `forge.debug.getAll()` returns the captured request entries in the browser console
- [ ] Root cause of infinite loading identified (or confirmed it's not an SDK-level issue)

---

## Testing Requirements

| Category | Test | Location |
|----------|------|----------|
| Unit | DebugStore push/getAll chronological order | `packages/sdk/src/__tests__/debug.test.ts` |
| Unit | Ring buffer overflow (105 items, maxSize=100, FIFO) | `packages/sdk/src/__tests__/debug.test.ts` |
| Unit | DebugStore.clear() empties buffer | `packages/sdk/src/__tests__/debug.test.ts` |
| Unit | DebugStore.getErrors() filters ok===false | `packages/sdk/src/__tests__/debug.test.ts` |
| Unit | DebugStore.getByPath() filters by prefix | `packages/sdk/src/__tests__/debug.test.ts` |
| Unit | DebugStore.getSlow() filters by threshold | `packages/sdk/src/__tests__/debug.test.ts` |
| Unit | DebugStore disabled = push is no-op | `packages/sdk/src/__tests__/debug.test.ts` |
| Unit | getAll returns copy, not internal array | `packages/sdk/src/__tests__/debug.test.ts` |
| Unit | Auto-detect mode (undefined constructor) | `packages/sdk/src/__tests__/debug.test.ts` |
| Unit | isDevMode with NODE_ENV=development | `packages/sdk/src/__tests__/debug.test.ts` |
| Unit | isDevMode with NODE_ENV=production | `packages/sdk/src/__tests__/debug.test.ts` |
| Unit | isDevMode with FORGE_DEBUG=true override | `packages/sdk/src/__tests__/debug.test.ts` |
| Unit | ForgeClient captures successful request | `packages/sdk/src/__tests__/client.test.ts` |
| Unit | ForgeClient captures error response | `packages/sdk/src/__tests__/client.test.ts` |
| Unit | ForgeClient captures network error | `packages/sdk/src/__tests__/client.test.ts` |
| Unit | ForgeClient requestList() captures pagination fields | `packages/sdk/src/__tests__/client.test.ts` |
| Unit | ForgeClient captures X-Request-Id header | `packages/sdk/src/__tests__/client.test.ts` |
| Smoke | WebUI shows [forge:sdk] logs in browser console | Manual |
| Smoke | CLI with FORGE_DEBUG=true shows logs | Manual |

**Total:** 17 automated tests, 2 manual smoke tests

---

## Documentation Requirements

- [ ] Add JSDoc to `DebugStore` class and all public methods
- [ ] Add JSDoc to `isDevMode()` explaining the three runtime paths
- [ ] Add inline comment in CLI client explaining FORGE_DEBUG usage
- [ ] Add a JSDoc comment to `ResumesResource.pdf()` noting that debug store entries for PDF requests may have inaccurate `payload_size` and `ok` values due to binary response handling limitations
- [ ] No new documentation files needed -- this is internal SDK infrastructure
