# Phase 4: HTTP API

**Goal:** Implement all HTTP route handlers as a thin layer over services.

**Non-Goals:** No business logic in routes. Routes parse requests, call services, format responses.

**Depends on:** Phase 3 (services)
**Blocks:** Phase 5 (SDK needs a running API to integration-test against)

---

## Task 4.1: HTTP Server Setup

**Goal:** Create the Bun HTTP server with middleware and routing infrastructure.

**Files:**
- `packages/core/src/routes/server.ts` — Bun.serve setup
- `packages/core/src/routes/middleware.ts` — request logging, error handling, CORS
- `packages/core/src/routes/router.ts` — route matching (or use a lightweight library like Hono)
- `packages/core/src/index.ts` — entrypoint: init db, start server

**Framework decision:** Use [Hono](https://hono.dev/) — lightweight, Bun-native, typed routes, middleware support.

**API path prefix:** All API routes are mounted under `/api/` in the Hono app (e.g., `/api/sources`, `/api/bullets`). In development, the Vite proxy maps `/api/*` to `http://localhost:3000/api/*`. In production, core serves both `/api/*` routes and `/*` static assets. This avoids dev/prod path discrepancies.

**Steps:**
1. Create Hono app instance with `/api` base path
2. Add middleware: JSON body parsing, CORS, request logging, X-Request-Id generation, error handler
3. CORS: in dev allow `localhost:5173`; in production allow same-origin (no cross-origin needed since core serves static assets)
4. X-Request-Id: generate UUID per request, include in response header, pass to logger
5. Error handler catches all thrown errors and returns the standard error envelope
6. Add health check: `GET /api/health` → `{ status: "ok" }`
7. Wire up database connection (singleton)
8. Entrypoint startup sequence:
   a. Load and validate environment variables (FORGE_PORT, FORGE_DB_PATH, FORGE_CLAUDE_PATH, FORGE_CLAUDE_TIMEOUT, FORGE_LOG_LEVEL) — exit with clear error if required vars are missing or invalid
   b. Connect to SQLite database
   c. Run pending migrations
   d. Recover stale deriving locks (`DerivationService.recoverStaleLocks`)
   e. Pre-flight check: verify `claude` binary exists at configured path (`claude --version`), log result (warn if not found, don't crash — server should work for data management without AI)
   f. Create services via `createServices(db)`
   g. Mount routes with services
   h. Start HTTP server on `FORGE_PORT`

**Acceptance Criteria:**
- [ ] Server starts on configured port
- [ ] `GET /api/health` returns 200
- [ ] Unknown routes return 404 with error envelope
- [ ] Malformed JSON body returns 400
- [ ] Unhandled errors return 500 with error envelope (no stack trace in production)
- [ ] CORS allows requests from localhost:5173 in development
- [ ] X-Request-Id header generated and returned on every response
- [ ] Request logging shows method, path, status code, duration, request ID
- [ ] Startup validates environment variables — clear error on missing/invalid
- [ ] Startup runs migrations before accepting requests
- [ ] Startup recovers stale deriving locks
- [ ] Startup logs Claude CLI availability (warn if not found, don't crash)
- [ ] Missing FORGE_DB_PATH → clear error message, exit 1

**Testing:**
- Smoke: Start server, hit health endpoint
- Unit: Error handler formats errors correctly
- Unit: CORS headers present on responses

---

## Task 4.1b: Employer, Project, and Skills Routes

**Goal:** Basic CRUD routes for supporting entities. Required for the WebUI employer dropdown and source creation.

**File:** `packages/core/src/routes/supporting.ts`

**Routes:**
- `POST /api/employers` → create employer → 201
- `GET /api/employers` → list employers → 200
- `DELETE /api/employers/:id` → delete employer → 204
- `POST /api/projects` → create project → 201
- `GET /api/projects` → list projects (filterable by employer_id) → 200
- `DELETE /api/projects/:id` → delete project → 204
- `POST /api/skills` → create skill → 201
- `GET /api/skills` → list skills (filterable by category) → 200
- `POST /api/bullets/:id/skills` → associate skill with bullet → 201
- `POST /api/perspectives/:id/skills` → associate skill with perspective → 201

**Acceptance Criteria:**
- [ ] Employer CRUD works
- [ ] Project CRUD with employer filter works
- [ ] Skill CRUD and association endpoints work
- [ ] Delete employer with projects → 409 or cascade (follow existing FK behavior)

**Testing:**
- Integration: Create employer → create project for employer → create source with employer_id
- Integration: Create skill → associate with bullet → verify in gap analysis skills_covered

---

## Task 4.2: Source Routes

**File:** `packages/core/src/routes/sources.ts`

**Routes:**
- `POST /api/sources` → `sourceService.createSource(body)` → 201
- `GET /api/sources` → `sourceService.listSources(query)` → 200
- `GET /api/sources/:id` → `sourceService.getSource(id)` → 200
- `PATCH /api/sources/:id` → `sourceService.updateSource(id, body)` → 200. Supports `{ status: "approved" }` and `{ status: "draft" }` to explicitly transition source status.
- `DELETE /api/sources/:id` → `sourceService.deleteSource(id)` → 204
- `POST /api/sources/:id/derive-bullets` → `derivationService.deriveBulletsFromSource(id)` → 201. Source must be in `approved` status.

**Implementation pattern (all routes follow this):**
```typescript
app.post('/sources', async (c) => {
  const body = await c.req.json<CreateSource>()
  const result = await sourceService.createSource(body)
  if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
  return c.json({ data: result.data }, 201)
})
```

**Reference:** `refs/strategies/http/status-codes.md`, `refs/strategies/http/response-shapes.md`

**Acceptance Criteria:**
- [ ] All 6 source routes implemented
- [ ] Correct status codes for success and each error type
- [ ] Pagination params parsed from query string for list endpoint
- [ ] derive-bullets returns 409 if already deriving
- [ ] derive-bullets returns 504 on timeout, 502 on AI error

**Testing:**
- Integration: POST create → GET verify → PATCH update → DELETE
- Integration: derive-bullets happy path (mocked AI)
- Integration: derive-bullets conflict (double call)
- Contract: Response shapes match `refs/strategies/http/response-shapes.md`

---

## Task 4.3: Bullet Routes

**File:** `packages/core/src/routes/bullets.ts`

**Routes:**
- `GET /bullets` → list (filterable by source_id, status, technology)
- `GET /bullets/:id` → get with source and perspective count
- `PATCH /bullets/:id` → update content
- `DELETE /bullets/:id` → delete (409 if has perspectives)
- `PATCH /bullets/:id/approve` → approve (400 if wrong status)
- `PATCH /bullets/:id/reject` → reject with required `rejection_reason` body
- `PATCH /bullets/:id/reopen` → reopen from rejected
- `POST /bullets/:id/derive-perspectives` → derive with `{ archetype, domain, framing }` body

**Acceptance Criteria:**
- [ ] All 8 bullet routes implemented
- [ ] Reject requires non-empty rejection_reason in body → 400 otherwise
- [ ] Approve from non-pending_review status → 400 with clear message
- [ ] Technology filter works via query param `?technology=kubernetes`
- [ ] derive-perspectives accepts and validates archetype/domain/framing

**Testing:**
- Integration: Full bullet lifecycle: create via derivation → approve → derive perspectives
- Integration: Rejection flow with reason
- Integration: Reopen flow
- Contract: Error shapes match contract

---

## Task 4.4: Perspective Routes

**File:** `packages/core/src/routes/perspectives.ts`

**Routes:**
- `GET /perspectives` → list (filterable by bullet_id, archetype, domain, framing, status)
- `GET /perspectives/:id` → get with full chain (bullet + source + snapshots)
- `PATCH /perspectives/:id` → update content
- `DELETE /perspectives/:id` → delete (409 if in a resume)
- `PATCH /perspectives/:id/approve` → approve
- `PATCH /perspectives/:id/reject` → reject with reason
- `PATCH /perspectives/:id/reopen` → reopen

**Acceptance Criteria:**
- [ ] All 7 perspective routes implemented
- [ ] GET :id returns nested chain (perspective → bullet → source)
- [ ] Delete blocked if perspective is in a resume

**Testing:**
- Integration: Full perspective lifecycle
- Integration: GET with chain returns correct nested data
- Contract: Chain response shape verified

---

## Task 4.5: Resume Routes

**File:** `packages/core/src/routes/resumes.ts`

**Routes:**
- `POST /resumes` → create
- `GET /resumes` → list
- `GET /resumes/:id` → get with perspectives grouped by section
- `PATCH /resumes/:id` → update metadata
- `DELETE /resumes/:id` → delete (cascades join table)
- `POST /resumes/:id/perspectives` → add perspective
- `DELETE /resumes/:id/perspectives/:perspectiveId` → remove perspective
- `PATCH /resumes/:id/reorder` → reorder perspectives
- `GET /resumes/:id/gaps` → gap analysis
- `POST /resumes/:id/export` → **501 Not Implemented**

**Acceptance Criteria:**
- [ ] All 10 resume routes implemented
- [ ] Export returns 501 with helpful message
- [ ] Gap analysis returns structured gap report
- [ ] Add perspective validates perspective is approved
- [ ] Reorder validates all IDs belong to the resume

**Testing:**
- Integration: Resume assembly workflow
- Integration: Gap analysis with known test data
- Integration: Export returns 501
- Contract: Gap analysis response matches `refs/examples/gap-analysis/response-shape.md`

---

## Task 4.6: Review Queue Route

**File:** `packages/core/src/routes/review.ts`

**Route:**
- `GET /review/pending` → review queue with counts and items

**Acceptance Criteria:**
- [ ] Returns correct counts for pending bullets and perspectives
- [ ] Items include context (source title, bullet content)

**Testing:**
- Integration: Create pending items, verify queue response

---

## Task 4.7: Static Asset Serving (Production Mode)

**Goal:** In production mode, core serves the built WebUI static assets.

**Implementation:**
- Check for `packages/webui/dist/` directory
- If exists, serve files at `/*` (catch-all after API routes)
- If not, skip (dev mode uses Vite dev server)

**Acceptance Criteria:**
- [ ] API routes take priority over static file serving
- [ ] index.html served for SPA client-side routes (fallback)
- [ ] Works without webui dist/ directory (graceful skip)

**Testing:**
- Integration: Build webui, start core, verify static files served

---

## Parallelization

All route files can be developed in parallel (each is independent):

```
Task 4.1 (server setup) ──┬──► Task 4.1b (employers/projects/skills)
                           ├──► Task 4.2 (sources)
                           ├──► Task 4.3 (bullets)
                           ├──► Task 4.4 (perspectives)
                           ├──► Task 4.5 (resumes)
                           ├──► Task 4.6 (review)
                           └──► Task 4.7 (static assets — test in Phase 9)
```

Note: Task 4.7 (static asset serving) depends on `packages/webui/dist/` from Phase 7. The code is written in Phase 4 but full testing happens in Phase 9.

## Documentation

- `docs/src/api/routes.md` — route table with methods, paths, status codes
- `docs/src/api/errors.md` — error code reference
