Observability Analysis

1. Full Scope — What COULD Be Added

┌────────────────────┬──────────────────────────┬─────────────────────────────────────────────────────────────────────┐
│       Layer        │           What           │                              Examples                               │
├────────────────────┼──────────────────────────┼─────────────────────────────────────────────────────────────────────┤
│ SDK Client (fetch) │ Request/response logging │ Method, URL, status, duration, error code, payload size             │
├────────────────────┼──────────────────────────┼─────────────────────────────────────────────────────────────────────┤
│ SDK Client (fetch) │ Request tracing          │ Correlation IDs propagated via X-Request-Id header                  │
├────────────────────┼──────────────────────────┼─────────────────────────────────────────────────────────────────────┤
│ SDK Client (fetch) │ Metrics                  │ Request count, error rate, latency percentiles                      │
├────────────────────┼──────────────────────────┼─────────────────────────────────────────────────────────────────────┤
│ Svelte Components  │ Lifecycle tracing        │ Component mount/destroy, effect fire counts, state change log       │
├────────────────────┼──────────────────────────┼─────────────────────────────────────────────────────────────────────┤
│ Svelte Components  │ Render timing            │ How long each render takes, what triggered it                       │
├────────────────────┼──────────────────────────┼─────────────────────────────────────────────────────────────────────┤
│ Svelte Components  │ Error boundaries         │ Catch and display unhandled errors instead of silent fail           │
├────────────────────┼──────────────────────────┼─────────────────────────────────────────────────────────────────────┤
│ Hono Server        │ Structured JSON logs     │ Already has request logging, could be structured (JSON, log levels) │
├────────────────────┼──────────────────────────┼─────────────────────────────────────────────────────────────────────┤
│ Hono Server        │ Request tracing          │ X-Request-Id already exists, could propagate to DB queries          │
├────────────────────┼──────────────────────────┼─────────────────────────────────────────────────────────────────────┤
│ Database           │ Query logging            │ SQL queries, bind params, duration, row count                       │
├────────────────────┼──────────────────────────┼─────────────────────────────────────────────────────────────────────┤
│ Database           │ Slow query detection     │ Flag queries > Nms                                                  │
├────────────────────┼──────────────────────────┼─────────────────────────────────────────────────────────────────────┤
│ AI Module          │ Invocation logging       │ Already has prompt_logs table, could add timing                     │
└────────────────────┴──────────────────────────┴─────────────────────────────────────────────────────────────────────┘

- Console-based structured logging
- Event emitter / observable
- Middleware/interceptor chain
- Debug flag with structured log store (debug store/ring buffer) adds programmatic access: devtools panel in the UI can show "last 20 API calls" with timing.
    - SDK client: console.debug on every request/response with method, path, status, duration
    - Debug ring buffer: last 100 entries, queryable (getErrors(), getSlow(), getByPath())
    - Component helpers: debugState() and tracedEffect() for Svelte 5 state tracing
    - Server: structured JSON logs with level filtering via FORGE_LOG_LEVEL
    - Zero dependencies, auto-enabled in dev mode
- OpenTelemetry-compatible spans; 2026-03-30-observability-opentelemetry.md
    - Full span schema: SDK → API middleware → services → DB → AI → tectonic
    - W3C Trace Context propagation via traceparent header
    - Library selection for TypeScript/Bun and Rust
    - Migration path from structured logging (1:1 field mapping to OTel attributes)
    - Opt-in via FORGE_OTEL_ENABLED env var, Jaeger for local dev
    - Explicitly marked as "future — not for immediate implementation"

Observability GOALS:
- Did the SDK fetch() fire? — We don't know if the browser is even making the request
- Did the SDK fetch() complete? — Maybe it's hanging on response parsing
- Did the component state actually change? — Maybe detailLoading = false fires but Svelte doesn't re-render
- Is the component being re-mounted/destroyed? — SvelteKit navigation might be destroying and recreating it
