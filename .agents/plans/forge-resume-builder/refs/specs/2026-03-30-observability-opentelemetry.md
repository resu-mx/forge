# Observability: OpenTelemetry-Compatible Spans

**Date:** 2026-03-30
**Status:** Design (Future — not for immediate implementation)
**Builds on:** 2026-03-30-observability-structured-logging.md

## Purpose

Define an upgrade path from the structured logging spec to full OpenTelemetry (OTel) instrumentation, enabling distributed tracing across the Forge stack (SDK → HTTP API → services → SQLite) with export to standard backends (Jaeger, Zipkin, OTLP).

This spec is written as a **future reference** — not for immediate implementation. The structured logging spec should be implemented first. This spec documents where OTel would integrate, what spans to create, and how to migrate from console logging to OTel without breaking changes.

## Goals

1. Define OTel span schema for every instrumented operation
2. Show how the structured logging API maps to OTel spans (migration path)
3. Identify OTel-compatible libraries for TypeScript/Bun and Rust
4. Design context propagation across SDK → API → service boundaries
5. Enable opt-in activation without code changes (configuration-only switch)

## Non-Goals

- Implementing OTel in this phase (see structured logging spec for immediate work)
- Running OTel collector or backend infrastructure
- Performance benchmarking of OTel overhead
- Custom OTel exporters

---

## 1. Architecture Overview

```
Browser (SDK Client)                    Server (Bun/Hono)
┌─────────────────────┐                ┌──────────────────────────────────┐
│ ForgeClient         │                │ Hono Middleware                   │
│  ┌───────────────┐  │   HTTP/JSON    │  ┌────────────────────────────┐  │
│  │ OTel Span:    │──┼───────────────►│  │ OTel Span:                │  │
│  │ forge.sdk.req │  │  traceparent   │  │ forge.api.request         │  │
│  └───────────────┘  │  header        │  │  ┌──────────────────────┐ │  │
│                     │                │  │  │ OTel Span:           │ │  │
│                     │                │  │  │ forge.service.method │ │  │
│                     │                │  │  │  ┌────────────────┐  │ │  │
│                     │                │  │  │  │ OTel Span:     │  │ │  │
│                     │                │  │  │  │ forge.db.query │  │ │  │
│                     │                │  │  │  └────────────────┘  │ │  │
│                     │                │  │  └──────────────────────┘ │  │
│                     │◄───────────────┼──│                          │  │
│                     │  traceresponse │  └────────────────────────────┘  │
└─────────────────────┘                └──────────────────────────────────┘
```

**Context propagation:** The SDK client attaches a `traceparent` header (W3C Trace Context format) to every request. The server middleware extracts it, creates a child span, and propagates the trace ID through the service and database layers.

---

## 2. Span Definitions

### 2.1 SDK Client Spans

| Span Name | Kind | Attributes | When |
|-----------|------|------------|------|
| `forge.sdk.request` | CLIENT | `http.method`, `http.url`, `http.status_code`, `forge.error.code` | Every SDK request |
| `forge.sdk.request_list` | CLIENT | Same + `forge.pagination.total`, `forge.pagination.offset` | List/paginated requests |

```typescript
// Span attributes
{
  'http.method': 'GET',
  'http.url': '/api/resumes/abc-123',
  'http.status_code': 200,
  'http.response_content_length': 4096,
  'forge.sdk.ok': true,
  'forge.error.code': undefined,  // or 'NOT_FOUND', 'VALIDATION_ERROR', etc.
}
```

### 2.2 Server Middleware Spans

| Span Name | Kind | Attributes | When |
|-----------|------|------------|------|
| `forge.api.request` | SERVER | `http.method`, `http.route`, `http.status_code`, `forge.request_id` | Every HTTP request |

```typescript
// Span attributes
{
  'http.method': 'GET',
  'http.route': '/api/resumes/:id',  // route template, not actual path
  'http.target': '/api/resumes/abc-123',  // actual path
  'http.status_code': 200,
  'forge.request_id': 'uuid',
}
```

### 2.3 Service Layer Spans

| Span Name | Kind | Attributes | When |
|-----------|------|------------|------|
| `forge.service.{name}.{method}` | INTERNAL | `forge.entity.type`, `forge.entity.id` | Every service method call |

Examples:
- `forge.service.resume.getResume` — `{ forge.entity.type: 'resume', forge.entity.id: 'abc-123' }`
- `forge.service.derivation.deriveBulletsFromSource` — `{ forge.entity.type: 'source', forge.entity.id: 'def-456' }`
- `forge.service.resume_compiler.compileIR` — `{ forge.entity.type: 'resume', forge.entity.id: 'abc-123' }`

### 2.4 Database Spans

| Span Name | Kind | Attributes | When |
|-----------|------|------------|------|
| `forge.db.query` | CLIENT | `db.system: 'sqlite'`, `db.statement` (truncated), `db.operation`, `db.row_count` | Every DB query |

```typescript
{
  'db.system': 'sqlite',
  'db.operation': 'SELECT',
  'db.statement': 'SELECT * FROM resumes WHERE id = ?',  // parameterized, not with values
  'db.row_count': 1,
}
```

### 2.5 AI Module Spans

| Span Name | Kind | Attributes | When |
|-----------|------|------------|------|
| `forge.ai.invoke` | CLIENT | `forge.ai.template`, `forge.ai.timeout_ms`, `forge.ai.ok` | Claude CLI invocation |
| `forge.ai.validate` | INTERNAL | `forge.ai.validator`, `forge.ai.valid`, `forge.ai.warnings` | Output validation |

### 2.6 LaTeX/PDF Spans

| Span Name | Kind | Attributes | When |
|-----------|------|------------|------|
| `forge.tectonic.compile` | CLIENT | `forge.tectonic.ok`, `forge.tectonic.duration_ms`, `forge.tectonic.exit_code` | PDF compilation |

---

## 3. Library Selection

### TypeScript / Bun

| Library | Purpose | Notes |
|---------|---------|-------|
| `@opentelemetry/api` | Core OTel API (tracer, span, context) | Lightweight, API-only |
| `@opentelemetry/sdk-trace-base` | Span processor + exporter interfaces | Needed for configuration |
| `@opentelemetry/sdk-trace-web` | Browser-specific trace provider | For SDK client in browser |
| `@opentelemetry/sdk-trace-node` | Node/Bun trace provider | For server (Bun-compatible) |
| `@opentelemetry/exporter-trace-otlp-http` | OTLP exporter | Sends to collector |
| `@opentelemetry/instrumentation-fetch` | Auto-instrument fetch() | Could replace manual SDK spans |
| `@opentelemetry/context-zone` | Zone.js context propagation for browser | Needed for async context in browser |

**Bun compatibility note:** Bun supports most Node.js APIs that OTel depends on. `@opentelemetry/sdk-trace-node` works with Bun 1.x. Zone.js may not work in Bun — use `AsyncLocalStorage` instead (Bun supports it).

### Rust (Future Migration)

| Crate | Purpose |
|-------|---------|
| `opentelemetry` | Core OTel API |
| `opentelemetry-otlp` | OTLP exporter |
| `tracing` | Rust's structured logging (compatible via `tracing-opentelemetry` bridge) |
| `tracing-opentelemetry` | Bridge from Rust `tracing` spans to OTel |

**Migration path:** Rust's `tracing` crate is the idiomatic logging/tracing framework. The `tracing-opentelemetry` bridge converts `tracing::Span` to OTel spans automatically. This means the Rust code can use `#[instrument]` attributes and `tracing::info!()` macros, and get OTel compatibility for free.

---

## 4. Migration from Structured Logging

The structured logging spec's `SDKLogEntry` maps directly to OTel span attributes:

| SDKLogEntry field | OTel attribute |
|-------------------|----------------|
| `method` | `http.method` |
| `path` | `http.url` |
| `status` | `http.status_code` |
| `duration_ms` | Span duration (calculated from start/end) |
| `ok` | `forge.sdk.ok` |
| `error_code` | `forge.error.code` |
| `request_id` | `forge.request_id` |

**Migration steps:**

1. **Phase 1 (now):** Implement structured logging spec. `console.debug` + ring buffer.
2. **Phase 2 (when needed):** Add `@opentelemetry/api` as a dependency. Create a `ForgeTracer` that wraps OTel's `tracer.startSpan()`. Keep `console.debug` as a fallback when OTel is not configured.
3. **Phase 3 (when needed):** Add OTel SDK + OTLP exporter. Configure via `FORGE_OTEL_ENDPOINT` env var. When set, spans export to a collector. When not set, falls back to console logging.

**No breaking changes:** The `ForgeClient` API doesn't change. The `debug` property still works. OTel adds additional export capability without removing the console/ring-buffer approach.

### 4.1 OTel Activation Configuration

```env
# .env
FORGE_OTEL_ENABLED=false           # Master switch
FORGE_OTEL_ENDPOINT=               # OTLP endpoint (e.g., http://localhost:4318)
FORGE_OTEL_SERVICE_NAME=forge      # Service name in traces
FORGE_OTEL_SAMPLE_RATE=1.0         # 1.0 = trace everything
```

When `FORGE_OTEL_ENABLED=true` and `FORGE_OTEL_ENDPOINT` is set:
- Server creates OTel trace provider with OTLP exporter
- Hono middleware creates server spans with context extraction from `traceparent` header
- SDK client (browser) creates client spans and injects `traceparent` header
- All existing `console.debug` logging continues to work alongside OTel

When `FORGE_OTEL_ENABLED=false` (default):
- No OTel dependencies loaded
- No performance overhead
- Structured logging (console.debug + ring buffer) is the only observability layer

---

## 5. Context Propagation

### 5.1 W3C Trace Context

The SDK client injects the `traceparent` header on every request:
```
traceparent: 00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01
```

Format: `version-trace_id-parent_id-trace_flags`

The server middleware extracts this header, creates a child span, and propagates the trace ID to all service and database spans within that request.

### 5.2 Async Context (Server)

Bun supports `AsyncLocalStorage` (Node.js API). OTel uses this to propagate span context across async boundaries within a single request:

```typescript
import { AsyncLocalStorage } from 'node:async_hooks'

const contextStorage = new AsyncLocalStorage<SpanContext>()

// In middleware:
contextStorage.run(spanContext, async () => {
  // All async code within this request can access the span context
  await next()
})

// In service:
const currentSpan = contextStorage.getStore()
```

### 5.3 Browser Context

In the browser, OTel uses Zone.js or a simpler approach. For Forge, since each SDK request is a single `fetch()` call (no nested async within the client), context propagation is trivial — the span is created before `fetch()` and ended after the response.

---

## 6. Span Visualization

### 6.1 Local Development

For local development, use Jaeger all-in-one:
```bash
docker run -d --name jaeger \
  -p 16686:16686 \  # Jaeger UI
  -p 4318:4318 \    # OTLP HTTP receiver
  jaegertracing/all-in-one:latest
```

Set `FORGE_OTEL_ENDPOINT=http://localhost:4318` and traces appear in the Jaeger UI at `http://localhost:16686`.

### 6.2 Example Trace

A request to `GET /api/resumes/abc-123/ir` would produce:

```
forge.sdk.request (12ms)
  └─ forge.api.request (10ms)
       └─ forge.service.resume_compiler.compileIR (8ms)
            ├─ forge.db.query SELECT resume (0.2ms)
            ├─ forge.db.query SELECT resume_entries (0.5ms)
            ├─ forge.db.query SELECT experience_entries (1.2ms)
            ├─ forge.db.query SELECT bullet_skills (0.8ms)
            └─ forge.db.query SELECT education_entries (0.3ms)
```

This immediately shows where time is spent and whether a request is completing or hanging.

---

## 7. Acceptance Criteria

### OTel Integration (when implemented)
- [ ] `@opentelemetry/api` is the only required dependency (others are optional)
- [ ] OTel is opt-in via `FORGE_OTEL_ENABLED` env var (default: false)
- [ ] When disabled, zero OTel overhead (no spans created, no packages loaded)
- [ ] When enabled, spans created for: SDK requests, API middleware, service methods, DB queries
- [ ] `traceparent` header propagated from SDK → server
- [ ] Spans export to OTLP endpoint when `FORGE_OTEL_ENDPOINT` is set
- [ ] Structured logging (console.debug + ring buffer) continues to work alongside OTel
- [ ] Server spans include `forge.request_id` attribute matching the X-Request-Id header
- [ ] Span names follow the `forge.{layer}.{operation}` convention

### Rust Compatibility
- [ ] Span naming convention is compatible with Rust `tracing` crate
- [ ] `tracing-opentelemetry` bridge can produce equivalent spans
- [ ] No TypeScript-specific abstractions that prevent Rust reimplementation

### Migration Path
- [ ] `SDKLogEntry` fields map 1:1 to OTel span attributes
- [ ] Existing `debug` ring buffer API is preserved when OTel is enabled
- [ ] No breaking changes to `ForgeClient` constructor or public API
- [ ] `console.debug` logging is not removed when OTel is active (dual output)

---

## 8. Dependencies & Sequencing

This spec is **not for immediate implementation.** It documents the upgrade path.

**Prerequisite:** Implement structured logging spec first (console.debug + ring buffer).

**When to implement OTel:**
- When debugging requires cross-boundary tracing (SDK → API → DB in one view)
- When the Rust migration begins (Rust `tracing` is OTel-compatible from day one)
- When multiple services exist (e.g., separate AI service, webhook service)

**Estimated cost:**
- `@opentelemetry/api`: ~50KB minified
- `@opentelemetry/sdk-trace-base` + exporter: ~150KB minified
- Implementation: 2-3 days for full stack instrumentation
- Infrastructure: Jaeger Docker container for local dev

For a single-user local tool, this is overkill until the Rust migration or multi-service architecture is in play. The structured logging spec provides 90% of the debugging value at 5% of the complexity.
