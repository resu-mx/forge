# ADR: R1 Server Architecture

**Date**: 2026-04-23
**Status**: Accepted
**Context**: Standing up the Axum HTTP server (R1) to replace the TS Hono API via strangler fig migration.

## Decisions

### 1. Thread Safety: `Mutex<Forge>` + `spawn_blocking`

**Chosen**: `std::sync::Mutex<Forge>` wrapped in `Arc<AppState>`, all DB ops via `spawn_blocking`
**Rejected**: Connection pool (r2d2-sqlite, deadpool-sqlite), `tokio::sync::Mutex`

**Rationale**: Forge is single-user with SQLite WAL mode. A connection pool adds complexity
with no benefit — SQLite is fundamentally single-writer. `std::sync::Mutex` is appropriate
because we never hold the lock across await points (all DB access is in `spawn_blocking`).
The `with_conn` helper encapsulates this pattern so handlers don't repeat boilerplate.

If perf becomes a bottleneck (unlikely for a personal tool), upgrade path is
`r2d2::Pool<SqliteConnectionManager>` with read-pool + write-pool separation.

### 2. Routes Call Repos Directly (Bypass Service Layer)

**Chosen**: Route handlers call `XRepository::method(conn, ...)` directly
**Rejected**: Implementing the 125 `todo!()` service methods first

**Rationale**: The forge-sdk service layer is all stubs. The repos already handle
validation, FK checks, and data access. The service layer adds value only when
there's cross-entity business logic (e.g., cascade operations, workflow rules).
Implementing 125 passthrough stubs just to delegate to repos is waste.

**When to add services**: When a route handler needs to coordinate across multiple
repos, apply business rules beyond CRUD, or integrate with the AI layer.

### 3. Crate Layout: lib + bin

**Chosen**: `forge-server` has both `src/lib.rs` and `src/main.rs`
**Rejected**: Binary-only crate

**Rationale**: The lib crate exports `routes::api_router()`, `state::AppState`, etc.
This enables:
- Integration tests (`tests/api_tests.rs`) that build a test server with in-memory SQLite
- Future R2 Tauri app embedding the Axum router directly (no subprocess)
- Other binaries reusing the route definitions

### 4. JSON Contract Compatibility

**Requirement**: Exact same JSON shapes as the TS server

- Success: `{ data: T }` or `{ data: T[], pagination: { total, offset, limit } }`
- Error: `{ error: { code: string, message: string } }`
- Status codes: 201 Created, 204 No Content, 400/404/409/500

**Implementation**: `ApiData<T>`, `ApiList<T>`, `Created<T>`, `NoContent` response wrappers.
`ApiError(ForgeError)` maps `ForgeError` variants to HTTP status codes via `IntoResponse`.

### 5. Middleware Stack

- **CORS**: `tower-http::CorsLayer` — permissive (Any) in dev, same-origin in prod
- **Tracing**: `tower-http::TraceLayer` — request/response logging via `tracing`
- **Request ID**: `tower-http::SetRequestIdLayer` + `PropagateRequestIdLayer` — UUID per request

No auth middleware needed (single-user local app).

### 6. Port Allocation

- TS Hono server: port 3000 (unchanged)
- Rust Axum server: port 3001 (via `FORGE_PORT` env var)

During strangler fig migration, both can run simultaneously behind a reverse proxy.

## Dependencies Added

| Crate | Version | Purpose |
|-------|---------|---------|
| tower | 0.5 | Middleware composition |
| tower-http | 0.6 | CORS, tracing, request-id |
| axum-test | 20 | Integration testing (dev-dep) |
| rusqlite | 0.32 | Re-exported for `with_conn` helper |
