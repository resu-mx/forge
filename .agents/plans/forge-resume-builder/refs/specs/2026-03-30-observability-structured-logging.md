# Observability: Structured Logging + Debug Store

**Date:** 2026-03-30
**Status:** Design
**Builds on:** All previous specs

## Purpose

Add structured logging and a debug store to the Forge SDK client and server, enabling rapid diagnosis of client-side rendering issues, API communication failures, and state management bugs without external tooling.

## Goals

1. Every SDK HTTP request/response logged with method, path, status, duration, error code
2. Debug ring buffer captures last N requests for programmatic inspection
3. Component-level state change logging (opt-in via debug flag)
4. Server-side structured JSON logging (replacing current `console.log` strings)
5. Zero external dependencies — uses only `console`, `performance.now()`, and plain data structures
6. Compatible with future OpenTelemetry upgrade path (see companion spec)

## Non-Goals

- Distributed tracing across services (see OTel spec)
- Log aggregation or shipping to external backends
- Production log levels / log rotation (single-user local tool)
- Performance metrics dashboards
- Alerting

---

## 1. SDK Client Logging

### 1.1 Request/Response Logging

Every `fetch()` call in `ForgeClient.request()` and `ForgeClient.requestList()` is wrapped with structured logging.

```typescript
interface SDKLogEntry {
  timestamp: string          // ISO 8601
  direction: 'request' | 'response' | 'error'
  method: string             // GET, POST, PATCH, DELETE
  path: string               // /api/resumes/abc-123
  status?: number            // HTTP status (response only)
  duration_ms?: number       // Time from request start to response
  ok?: boolean               // Result.ok value
  error_code?: string        // ForgeError.code if error
  error_message?: string     // ForgeError.message if error
  request_id?: string        // X-Request-Id from response header
  payload_size?: number      // Response body size in bytes (approximate)
  request_body_size?: number // Request body size in bytes (POST/PATCH/PUT only, from JSON.stringify(body).length)
  pagination_total?: number    // Total count from paginated response (requestList only)
  pagination_offset?: number   // Offset used in paginated request (requestList only)
  pagination_limit?: number    // Limit used in paginated request (requestList only)
  request_body?: unknown       // Request body (when logPayloads: true, POST/PATCH only)
  response_body?: unknown      // Response body (when logPayloads: true, truncated to 10KB)
}
```

**Notes:**
- `request_id` is `undefined` when the request fails at the network level (no response received). This is expected — the server never generated a request ID.
- `request_body_size` is populated for POST/PATCH/PUT requests: `JSON.stringify(body).length`.
- `pagination_*` fields are populated by `requestList()` only.
- When `logPayloads: true`, request and response bodies are captured in the ring buffer. Response bodies are truncated to 10KB to prevent memory bloat. For the PDF endpoint (binary response), the body is not captured — only `payload_size` is recorded.

**Console output format:**
```
[forge:sdk] → GET /api/resumes/abc-123
[forge:sdk] ← GET /api/resumes/abc-123 200 12.3ms [req-id] ok
[forge:sdk] ← POST /api/resumes/abc-123/pdf 422 45.1ms [req-id] ERROR LATEX_COMPILE_ERROR
[forge:sdk] ✗ GET /api/health NETWORK_ERROR (fetch failed)
```

Arrows: `→` request, `←` response, `✗` network error (no response received).

**Log level:** Uses `console.debug()` which is hidden by default in Chrome/Edge DevTools (user must enable 'Verbose' log level). In Firefox and Safari, `console.debug` is visible by default. In terminal (Bun/Node), `console.debug` outputs the same as `console.log`.

#### Binary Response Limitation (pdf)

`pdf()` currently calls `request()` which will log the POST request, but the response logging will show the JSON-parsed result (which is broken for binary). The POST request IS logged (method, path, status), but `ok` and `payload_size` may be inaccurate for binary responses.

**Recommendation:** Add a `requestBinary()` method to ForgeClient in a future phase, and log it the same way. Until then, document this limitation: for `pdf()` calls, the request line is accurate but the response line's `payload_size` reflects the JSON parse attempt, not the actual binary size.

### 1.2 Debug Store (Ring Buffer)

A configurable ring buffer that captures the last N `SDKLogEntry` objects for programmatic access.

```typescript
interface DebugStore {
  entries: SDKLogEntry[]     // Ring buffer of last maxSize entries
  maxSize: number            // Default: 100
  enabled: boolean           // Default: true in dev, false in prod

  // Methods
  push(entry: SDKLogEntry): void
  clear(): void
  getAll(): SDKLogEntry[]
  getErrors(): SDKLogEntry[]
  getByPath(pathPrefix: string): SDKLogEntry[]
  getSlow(thresholdMs: number): SDKLogEntry[]
}
```

**Storage:** In-memory only. Not persisted. Resets on page reload.

**Ring buffer eviction:** The ring buffer uses FIFO (First In, First Out) eviction. When the buffer reaches `maxSize` and a new entry is pushed, the oldest entry is removed. Implementation: use a plain array with `push()` on add and `shift()` on overflow (when `entries.length > maxSize`). This is O(n) per push at max capacity, but with maxSize=100 the performance impact is negligible.

**Access patterns:**
- `forge.debug.getAll()` — last 100 requests in chronological order
- `forge.debug.getErrors()` — only error responses
- `forge.debug.getSlow(500)` — requests slower than 500ms
- `forge.debug.getByPath('/api/resumes')` — filter by path prefix

### 1.3 Client Configuration

```typescript
interface ForgeClientOptions {
  baseUrl: string
  debug?: boolean | DebugOptions
}

interface DebugOptions {
  logToConsole?: boolean    // Default: true (uses console.debug)
  storeSize?: number        // Default: 100 (ring buffer max entries)
  logPayloads?: boolean     // Default: false (log request/response bodies — verbose)
}
```

**Note:** This extends the existing `ForgeClientOptions` interface (exported from `@forge/sdk`). The `debug` field is optional, so this is a backward-compatible addition.

**Dev mode detection utility:**

```typescript
/** Detect dev mode across Vite (browser/SvelteKit) and Bun (CLI/tests) */
function isDevMode(): boolean {
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

This function is defined in `packages/sdk/src/debug.ts` and used anywhere dev mode auto-detection is needed. It handles three runtimes:
- **Vite (browser/SvelteKit):** `import.meta.env.DEV` is statically replaced at build time.
- **Bun (CLI/tests):** `import.meta.env` may not exist or may throw. Falls back to `process.env.NODE_ENV` and `process.env.FORGE_DEBUG`.
- **Node.js:** Same fallback as Bun.

**Default behavior:**
- `debug: true` → console logging ON, store ON with 100 entries
- `debug: false` → console logging OFF, store OFF
- `debug: undefined` → auto-detect: ON if `isDevMode()` returns true

**When `logPayloads: true`:** Request and response bodies are captured in the ring buffer entries (`request_body` and `response_body` fields on `SDKLogEntry`). Response bodies are truncated to 10KB to prevent memory bloat. For the PDF endpoint (binary response), the body is not captured — only `payload_size` is recorded.

### 1.4 Implementation Location

```
packages/sdk/src/debug.ts       — DebugStore class, SDKLogEntry type, isDevMode()
packages/sdk/src/client.ts      — Wrap request/requestList with logging
packages/sdk/src/index.ts       — Export DebugStore, SDKLogEntry, isDevMode
```

The `ForgeClient` class gains a public `debug: DebugStore` property.

### 1.5 CLI Context

The CLI (`packages/cli/src/client.ts`) instantiates `ForgeClient`. In this context:
- `import.meta.env` does not exist (Bun runtime, not Vite). The `isDevMode()` utility handles this via the `process.env` fallback.
- CLI defaults to debug OFF to avoid mixing debug logs with user-facing output.
- `FORGE_DEBUG=true forge source list` enables debug logging for a single command.
- Debug output goes to `console.debug` which in terminals (Bun/Node) outputs the same as `console.log` — there is no separate "verbose" filter like in browser devtools. This means enabling debug in CLI will interleave debug logs with normal output.

### 1.6 Implementation Notes

**Constructor initialization order:** In the `ForgeClient` constructor, `this.debug` must be initialized BEFORE `this.request.bind(this)` is called, because the `request()` method body references `this.debug`. The constructor order must be:
1. Initialize `this.debug = new DebugStore(options)`
2. Bind request methods: `const req = this.request.bind(this)`
3. Create resource instances with bound methods

---

## 2. Component State Logging

### 2.1 Debug Effect Helper

A reusable utility for Svelte 5 components that logs state changes:

```typescript
// packages/webui/src/lib/debug.svelte.ts
import { isDevMode } from '@forge/sdk'

export function debugState(label: string, stateGetter: () => Record<string, unknown>) {
  if (isDevMode()) {
    $effect(() => {
      console.debug(`[forge:${label}]`, stateGetter())
    })
  }
}
```

> **Why `.svelte.ts`?** Svelte 5 runes (`$effect`, `$state`, etc.) are compiler directives. They only work in `.svelte` or `.svelte.ts` files. The `.svelte.ts` extension tells SvelteKit's Vite plugin to process the file through the Svelte compiler.

**Usage in components:**
```svelte
<script>
  import { debugState } from '$lib/debug.svelte'

  let loading = $state(true)
  let selectedId = $state(null)

  // Logs every time loading or selectedId changes
  debugState('resumes', () => ({ loading, selectedId, hasDetail: !!detail }))
</script>
```

**Output:**
```
[forge:resumes] { loading: true, selectedId: null, hasDetail: false }
[forge:resumes] { loading: false, selectedId: null, hasDetail: false }
[forge:resumes] { loading: false, selectedId: "abc-123", hasDetail: false }
[forge:resumes] { loading: false, selectedId: "abc-123", hasDetail: true }
```

### 2.2 Effect Tracing

A utility to trace when effects fire and why:

```typescript
// packages/webui/src/lib/debug.svelte.ts
import { isDevMode } from '@forge/sdk'

export function tracedEffect(label: string, fn: () => void | (() => void)) {
  if (isDevMode()) {
    $effect(() => {
      console.debug(`[forge:effect] ${label} fired`)
      return fn()
    })
  } else {
    $effect(fn)
  }
}
```

### 2.3 Opt-In Only

Component logging is NOT added to all components by default. Developers add `debugState()` calls when debugging a specific component, then remove them or leave them (they're no-ops in production via the `isDevMode()` check).

---

## 3. Server-Side Structured Logging

### 3.1 Replace String Logs with JSON

Current server logging (in `server.ts` middleware):
```
GET /api/health 200 0.1ms [request-id]
```

Replace with structured JSON:
```json
{"level":"info","ts":"2026-03-30T12:00:00Z","method":"GET","path":"/api/resumes/abc-123","route":"/api/resumes/:id","status":200,"duration_ms":0.1,"request_id":"abc-123"}
```

**Note:** Hono exposes the matched route pattern via `c.req.routePath` after routing completes. The post-response log line (after `await next()`) should use this to populate the `route` field. The pre-request log line (before routing) will not have `routePath` available.

### 3.2 Log Levels

| Level | When | Console method |
|-------|------|----------------|
| `debug` | Verbose details (query params, body preview) | `console.debug` |
| `info` | Normal requests | `console.log` |
| `warn` | 4xx responses, slow queries (>500ms) | `console.warn` |
| `error` | 5xx responses, unhandled errors | `console.error` |

Controlled by `FORGE_LOG_LEVEL` env var (already defined in `.env.example`).

### 3.3 Implementation Location

```
packages/core/src/lib/logger.ts        — Logger class with level filtering
packages/core/src/routes/server.ts      — Replace inline console calls with logger
packages/core/src/index.ts              — Initialize logger from FORGE_LOG_LEVEL
```

Create `packages/core/src/lib/` directory if it doesn't exist. This directory is for shared utilities (escape functions from Phase 19 are also here).

---

## 4. Devtools Panel (Future Enhancement)

A WebUI route at `/devtools` that displays:
- Last N SDK requests from the debug store (table with method, path, status, duration)
- Error requests highlighted in red
- Click to expand and see full request/response details
- Component state timeline (if state logging is enabled)

**This is a future enhancement, not part of the initial implementation.** The debug store API is designed to support it.

---

## 5. Acceptance Criteria

### SDK Logging
- [ ] Every `request()` and `requestList()` call logs to `console.debug` with method, path, status, duration
- [ ] Log format: `[forge:sdk] → METHOD /path` for requests, `[forge:sdk] ← METHOD /path STATUS DURATIONms` for responses
- [ ] Network errors logged as `[forge:sdk] ✗ METHOD /path ERROR_CODE`
- [ ] `ForgeClient` has public `debug: DebugStore` property
- [ ] `DebugStore` captures last 100 entries in ring buffer
- [ ] `debug.getAll()`, `getErrors()`, `getByPath()`, `getSlow()` methods work
- [ ] Debug logging auto-enabled in dev mode via `isDevMode()` (supports Vite, Bun, and Node.js)
- [ ] Debug logging configurable via `ForgeClientOptions.debug`
- [ ] CLI context: debug OFF by default, enabled via `FORGE_DEBUG=true`
- [ ] Zero external dependencies added

### Component Logging
- [ ] `debugState()` helper available in `$lib/debug.svelte.ts`
- [ ] `tracedEffect()` helper available in `$lib/debug.svelte.ts`
- [ ] Both are no-ops when `isDevMode()` returns false
- [ ] Can be added to any Svelte component with a single line

### Server Logging
- [ ] Request/response logs are structured JSON
- [ ] Structured logs include `route` field with matched route pattern (e.g., `/api/resumes/:id`)
- [ ] Log level controlled by `FORGE_LOG_LEVEL` env var
- [ ] Levels: debug, info, warn, error
- [ ] 4xx responses logged at warn, 5xx at error
- [ ] Slow requests (>500ms) logged at warn regardless of status

### Tests
- [ ] DebugStore ring buffer: push, overflow, clear, filter methods
- [ ] DebugStore ring buffer respects maxSize
- [ ] Ring buffer overflow: push 105 entries with maxSize=100, verify `getAll()` returns exactly 100 entries, and the first 5 pushed entries are gone (FIFO)
- [ ] Logger filters by log level
- [ ] SDK logging captures correct method/path/status/duration
- [ ] Server slow request logging: a 200 response taking >500ms is logged at `warn` level, not `info`

---

## 6. Dependencies & Parallelization

### Sequential
1. SDK debug store (must exist before client can use it)
2. SDK client logging (depends on debug store)

### Parallel (after SDK logging)
- Component debug helpers (independent)
- Server-side logger (independent)
- Devtools panel (future, depends on debug store)

### No New Dependencies
This spec adds zero npm packages. Uses only built-in APIs:
- `console.debug`, `console.log`, `console.warn`, `console.error`
- `performance.now()`
- `Date.prototype.toISOString()`
- `Response.headers.get('X-Request-Id')`
