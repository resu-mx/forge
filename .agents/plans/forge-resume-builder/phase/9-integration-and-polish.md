# Phase 9: Integration & Polish

**Goal:** End-to-end testing, documentation completion, and final validation against acceptance criteria.

**Non-Goals:** No new features. This phase is validation and cleanup only.

**Depends on:** All previous phases (Phases 6, 7, 8 can still be finishing — tasks here are independent)
**Blocks:** Nothing — this is the final phase

**Prerequisites:** Install Playwright for WebUI E2E tests: `bun add -D @playwright/test` in the webui package, run `bunx playwright install chromium`. Add `playwright.config.ts` to `packages/webui/`.

---

## Task 9.1: End-to-End Test Suite

**Goal:** Verify the full system works as a connected whole.

**File:** `packages/core/src/__tests__/e2e/` (or a top-level `tests/e2e/` directory)

**Test scenarios (reference: `refs/examples/e2e/derivation-chain.md`):**

### E2E-1: Full Derivation Chain
1. Create employer
2. Create source with employer reference
3. Mark source as approved
4. Derive bullets from source
5. Review bullets: approve 3, reject 1 with reason
6. Derive perspectives from approved bullet (archetype: agentic-ai, domain: ai_ml)
7. Approve perspective
8. Create resume (archetype: agentic-ai)
9. Add perspective to resume
10. Verify chain: `GET /perspectives/:id` returns full chain with correct snapshots
11. Verify gap analysis: `GET /resumes/:id/gaps` returns structured report

### E2E-2: Content Drift Detection
1. Create source → derive bullets → approve bullet
2. Edit source description
3. Verify bullet's `source_content_snapshot` differs from source's current description
4. Verify chain integrity report shows divergence

### E2E-3: Rejection and Reopen Flow
1. Derive bullets → reject all with reasons
2. Reopen one bullet
3. Approve the reopened bullet
4. Derive perspectives from it
5. Verify rejection_reason is preserved after reopen

### E2E-4: Concurrency Protection
1. Trigger derive-bullets on a source
2. Immediately trigger derive-bullets again on the same source
3. Verify first succeeds, second returns 409

### E2E-5: Cascade and Restrict Behavior
1. Create source → derive bullets → derive perspectives → add to resume
2. Attempt to delete perspective → 409 (in resume)
3. Remove perspective from resume → delete perspective → success
4. Attempt to delete bullet → 409 (has other perspectives, if any)
5. Delete all perspectives for bullet → delete bullet → success
6. Attempt to delete source → 409 (has other bullets, if any)
7. Delete all bullets → delete source → success

### E2E-6: CLI + API Consistency
1. Create source via CLI
2. Verify via API
3. Derive bullets via API
4. Review via CLI (`forge review` in non-interactive mode)
5. Verify status changes via API

### E2E-7: WebUI + API Consistency
1. Create source via WebUI (Playwright or similar)
2. Verify via API
3. Full derivation workflow through WebUI
4. Verify all data correct via API

**Database isolation:** Each E2E test scenario uses a fresh temporary database file. Test setup creates the db, runs migrations, seeds data. Test teardown deletes the file. This avoids test-order dependencies.

**Acceptance Criteria:**
- [ ] All 7 E2E scenarios pass
- [ ] Tests run against real (file-based) SQLite databases, not in-memory
- [ ] Each test uses a fresh database (no test-order dependencies)

**Testing infrastructure:**
- Bun test for API-level E2E
- Playwright for WebUI E2E (Task 9.1 + 7.x)
- Both against a running core server

---

## Task 9.2: Contract Tests

**Goal:** Verify SDK correctly handles all API response shapes.

**Steps:**
1. For each API endpoint, verify the SDK correctly deserializes the response
2. For each error code, verify the SDK maps it to the correct ForgeError
3. Verify pagination response shape matches SDK's PaginatedResult type

**Acceptance Criteria:**
- [ ] Every success response shape has a corresponding SDK test
- [ ] Every error code has a corresponding SDK test
- [ ] Pagination works correctly through SDK

---

## Task 9.3: Acceptance Criteria Validation

**Goal:** Walk through every acceptance criterion from the spec and verify it.

**Reference:** Spec section "Acceptance Criteria"

**Steps:**
1. **First:** Generate a checklist from the spec's Acceptance Criteria section (spec lines 867-912) as a markdown file with checkboxes. This is the master checklist — do not rely on manual discovery.
2. For each component (core, SDK, CLI, WebUI, data model):
   a. Work through the checklist
   b. Verify each criterion is met (run command, check output, inspect behavior)
   c. Document any exceptions or deviations
3. Create a checklist report: `docs/src/mvp/acceptance-report.md`

**Acceptance Criteria:**
- [ ] Every acceptance criterion from the spec is verified or documented as deferred
- [ ] Report committed to docs

---

## Task 9.4: Documentation Completion

**Goal:** Fill in all doc stubs created during implementation.

**Files to complete:**
- `docs/src/adrs/001-api-first.md` through `007-uuids.md`
- `docs/src/data/models/entity-types.md`
- `docs/src/data/models/schema.md`
- `docs/src/architecture/monorepo.md`
- `docs/src/architecture/rust-migration.md`
- `docs/src/mvp/scope.md`
- `docs/src/api/routes.md`
- `docs/src/api/errors.md`
- `docs/src/sdk/client.md`
- `docs/src/sdk/examples.md`
- `docs/src/lib/repositories.md`
- `docs/src/lib/services.md`
- `docs/src/lib/ai-module.md`
- `docs/src/lib/migrations.md`
- `docs/src/cli/commands.md`
- `docs/src/cli/usage.md`
- `docs/src/webui/views.md`
- `docs/src/webui/components.md`

**Acceptance Criteria:**
- [ ] All doc files have substantive content (no empty stubs)
- [ ] ADRs match the decisions actually implemented
- [ ] API route docs match actual routes
- [ ] SDK examples are tested and working

---

## Task 9.4b: Backup/Restore Validation

**Goal:** Verify `just dump` produces a restorable SQL backup.

**Steps:**
1. Populate database with test data (sources, bullets, perspectives, resume)
2. Run `just dump`
3. Delete the database
4. Restore from dump: `sqlite3 data/forge-test.db < data/forge-dump-YYYYMMDD.sql`
5. Verify all data present

**Acceptance Criteria:**
- [ ] `just dump` produces valid SQL
- [ ] Restored database has all original data
- [ ] Restored database passes all PRAGMA checks

---

## Task 9.5: Smoke Test & Dev Experience

**Goal:** Verify the new-developer experience.

**Steps:**
1. Clone repo fresh
2. `bun install`
3. `just dev`
4. Open browser to localhost:5173
5. Create a source, derive bullets, approve, derive perspectives, build resume
6. Run `forge review` from CLI
7. Verify everything works from scratch

**Acceptance Criteria:**
- [ ] Fresh clone → `just dev` → working app in under 2 minutes
- [ ] No manual steps beyond `bun install` and `just dev`
- [ ] `.env.example` has correct defaults that work out of the box
- [ ] README.md (if created) has getting-started instructions

---

## Parallelization

```
Task 9.1 (E2E tests) ─────┐
Task 9.2 (Contract tests) ─┤ (all can run in parallel)
Task 9.3 (Acceptance) ─────┤
Task 9.4 (Docs) ───────────┤
Task 9.5 (Smoke test) ─────┘
```

All tasks in this phase are independent reads/verifications.
