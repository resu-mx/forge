# Phase 3: Core Services & AI Module

**Goal:** Implement business logic services and the Claude Code CLI wrapper.

**Non-Goals:** No HTTP routes yet. Services are tested directly, not through HTTP.

**Depends on:** Phase 2 (repositories), Phase 1 Task 1.4 (constants), Phase 0 Task 0.1 (AI spike results)
**Blocks:** Phase 4 (routes call services)

### Service Instantiation Pattern

All services are **classes instantiated once at startup** and injected into route handlers via closure. The database connection is injected into each service's constructor. This makes services testable (inject mock db) and ensures the in-memory derivation lock Set is a singleton.

```typescript
// packages/core/src/services/index.ts
export function createServices(db: Database) {
  const sourceRepo = new SourceRepository(db)
  const bulletRepo = new BulletRepository(db)
  // ... all repos

  const aiModule = new ClaudeCliModule(config)
  const derivingBullets = new Set<string>()  // in-memory lock for bullet derivation

  return {
    sources: new SourceService(sourceRepo),
    bullets: new BulletService(bulletRepo),
    perspectives: new PerspectiveService(perspectiveRepo),
    derivation: new DerivationService(sourceRepo, bulletRepo, perspectiveRepo, promptLogRepo, aiModule, derivingBullets),
    resumes: new ResumeService(resumeRepo, perspectiveRepo, bulletRepo),
    audit: new AuditService(perspectiveRepo, bulletRepo, sourceRepo),
    review: new ReviewService(bulletRepo, perspectiveRepo),
  }
}
```

This factory is called once in the server entrypoint. Route handlers receive the services object.

---

## Task 3.1: AI Module — Claude Code CLI Wrapper

**Goal:** Implement the AI module that shells out to Claude Code.

**Files:**
- `packages/core/src/ai/claude-cli.ts` — low-level process wrapper
- `packages/core/src/ai/prompts.ts` — prompt templates
- `packages/core/src/ai/validator.ts` — output validation
- `packages/core/src/ai/index.ts` — public interface

**Reference:**
- `refs/examples/prompts/source-to-bullet.md`
- `refs/examples/prompts/bullet-to-perspective.md`
- `refs/strategies/output-validation.md`

**Steps:**

### 3.1a: Claude CLI Wrapper
```typescript
interface ClaudeOptions {
  prompt: string
  timeout?: number   // default: FORGE_CLAUDE_TIMEOUT
  claudePath?: string // default: FORGE_CLAUDE_PATH
}

interface ClaudeResult {
  ok: true; data: unknown  // parsed JSON
} | {
  ok: false; error: 'TIMEOUT' | 'PARSE_ERROR' | 'PROCESS_ERROR'; raw: string
}

async function invokeClaude(options: ClaudeOptions): Promise<ClaudeResult>
```

Implementation:
- Uses `Bun.spawn(['claude', '-p', prompt, '--output-format', 'json'])`
- Reads stdout as text, parses as JSON
- AbortSignal with timeout for kill
- Captures stderr for error reporting

### 3.1b: Prompt Templates
- `renderSourceToBulletPrompt(source: Source): string`
- `renderBulletToPerspectivePrompt(bullet: Bullet, params: DerivePerspectiveInput): string`
- Templates from refs, rendered with variable interpolation

### 3.1c: Output Validator
- `validateBulletDerivation(response: unknown): BulletDerivationResponse | ValidationError`
- `validatePerspectiveDerivation(response: unknown): PerspectiveDerivationResponse | ValidationError`
- Schema checks: required fields, types, non-empty content
- Provenance heuristic: flag proper nouns/technologies not in input (warning, not blocking)

**Acceptance Criteria:**
- [ ] `invokeClaude` successfully spawns process and captures JSON output
- [ ] Timeout kills process and returns TIMEOUT error
- [ ] Malformed JSON returns PARSE_ERROR with raw output
- [ ] Prompt templates render correctly with source/bullet data
- [ ] Validator rejects missing required fields
- [ ] Validator flags provenance warnings without blocking
- [ ] All validation errors include the raw response for debugging

**Failure Criteria:**
- Claude CLI not found → clear error: "Claude Code CLI not found at {path}. Install from https://claude.ai/claude-code"
- Claude CLI behavior differs from spike results → update wrapper based on actual behavior

**Fallback Strategy:**
If `--output-format json` wraps output in unexpected structure, extract the JSON payload from the response. Document the extraction logic.

**Testing:**
- Unit: Mock `Bun.spawn` to test wrapper logic without actual Claude invocation
- Unit: Prompt template rendering with fixture data
- Unit: Validator with valid response → success
- Unit: Validator with missing fields → error
- Unit: Validator with extra fields → warning
- Unit: Validator with unknown proper nouns → provenance warning
- Integration: One real Claude invocation with test prompt (manual/CI-excluded test)

---

## Task 3.2: SourceService

**File:** `packages/core/src/services/source-service.ts`

**Methods:**
- `createSource(input: CreateSource): Result<Source>`
- `getSource(id: string): Result<Source>`
- `listSources(filter, offset, limit): Result<PaginatedResult<Source>>`
- `updateSource(id: string, input: UpdateSource): Result<Source>`
- `deleteSource(id: string): Result<void>`

**Business logic:**
- Validates input (non-empty title, non-empty description)
- Sets `updated_by = 'human'` on create/update
- Delete checks for dependent bullets via repository (RESTRICT)
- No status transition logic beyond what the repo handles

**Acceptance Criteria:**
- [ ] Validation rejects empty title/description
- [ ] CRUD delegates to repository correctly
- [ ] Errors are wrapped in Result type

**Testing:**
- Unit: Valid create → success
- Unit: Empty title → validation error
- Unit: Delete with bullets → conflict error
- Unit: Mocked repository for all tests

---

## Task 3.2b: BulletService

**File:** `packages/core/src/services/bullet-service.ts`

**Methods:**
- `getBullet(id): Result<Bullet>`
- `listBullets(filter, offset, limit): PaginatedResult<Bullet>`
- `updateBullet(id, input): Result<Bullet>`
- `deleteBullet(id): Result<void>`
- `approveBullet(id): Result<Bullet>` — validates: status must be `pending_review`
- `rejectBullet(id, reason): Result<Bullet>` — validates: status must be `pending_review`, reason non-empty
- `reopenBullet(id): Result<Bullet>` — validates: status must be `rejected`

**Business logic:**
- Status transition validation: `pending_review → approved`, `pending_review → rejected`, `rejected → pending_review` (reopen). All other transitions return VALIDATION_ERROR.
- `approved → pending_review` is blocked (approved is terminal for MVP)
- `approved → rejected` is blocked
- `draft → approved` is blocked (must go through pending_review)
- Delete checks for dependent perspectives

**Acceptance Criteria:**
- [ ] Approve from `pending_review` → success
- [ ] Approve from `draft` → VALIDATION_ERROR
- [ ] Approve from `approved` → VALIDATION_ERROR (already approved)
- [ ] Approve from `rejected` → VALIDATION_ERROR (must reopen first)
- [ ] Reject without reason → VALIDATION_ERROR
- [ ] Reopen from `rejected` → success, preserves rejection_reason
- [ ] Reopen from `approved` → VALIDATION_ERROR

**Testing:**
- Unit: All valid status transitions succeed
- Unit: All invalid status transitions return typed error
- Unit: Reject without reason returns error
- Unit: Delete with perspectives returns CONFLICT

---

## Task 3.2c: PerspectiveService

**File:** `packages/core/src/services/perspective-service.ts`

**Methods:** Mirror of BulletService with perspective-specific fields.
- Same approve/reject/reopen with identical transition rules
- `getPerspectiveWithChain(id)` — delegates to repository's `getWithChain`
- Delete checks: blocked if perspective is in a resume

**Acceptance Criteria:**
- [ ] Same status transition rules as BulletService
- [ ] `getWithChain` returns nested bullet + source with snapshots
- [ ] Delete blocked if in resume

**Testing:**
- Unit: All status transitions
- Unit: Chain retrieval with snapshot comparison

---

## Task 3.3: DerivationService

**File:** `packages/core/src/services/derivation-service.ts`

**This is the most critical service. It orchestrates the entire derivation chain.**

**Methods:**
- `deriveBulletsFromSource(sourceId: string): Result<Bullet[]>`
- `derivePerspectivesFromBullet(bulletId: string, params: DerivePerspectiveInput): Result<Perspective>`

**deriveBulletsFromSource flow:**
1. Get source (404 if not found)
2. Acquire deriving lock (409 if already deriving)
3. Capture `source.description` as snapshot
4. Render source-to-bullet prompt
5. Invoke Claude Code CLI
6. Validate response
7. On success:
   a. Create PromptLog entry
   b. Create Bullet entities (status: pending_review, source_content_snapshot set)
   c. Create bullet_technologies junction rows
   d. Release deriving lock (restore previous status, set last_derived_at)
   e. Return created bullets
8. On failure:
   a. Release deriving lock (restore previous status, no last_derived_at update)
   b. Return typed error (AI_ERROR, GATEWAY_TIMEOUT, etc.)

All of steps 7 or 8 must be in a single SQLite transaction.

**derivePerspectivesFromBullet flow:**
Similar pattern but for bullet → perspective. Uses in-memory lock (Set of bullet IDs currently deriving) since bullets don't have a `deriving` status. Validates that the bullet status is `approved` before proceeding — rejected/draft bullets cannot have perspectives derived.

**Stale lock recovery on startup:**
```typescript
static recoverStaleLocks(sourceRepo: SourceRepository): void {
  // Reset any sources stuck in 'deriving' status (server crashed during derivation)
  // Uses 5-minute threshold to avoid resetting legitimate in-progress derivations
  sourceRepo.resetStaleLocks(300_000) // 5 minutes
}
```
This is called once during server startup (Phase 4 Task 4.1 entrypoint), before the HTTP server begins accepting requests. The in-memory bullet derivation Set is empty on startup (no recovery needed — if perspective derivation was interrupted, no persistent state was corrupted).

**Acceptance Criteria:**
- [ ] Full happy path: source → lock → AI → validate → create bullets → unlock
- [ ] Lock prevents concurrent derivation on same source
- [ ] Content snapshot captured before AI invocation
- [ ] PromptLog entry created with full prompt and response
- [ ] Technologies extracted and stored in junction table
- [ ] On AI failure: lock released, no partial records, typed error returned
- [ ] On validation failure: same cleanup as AI failure
- [ ] Transaction wraps all writes (atomic)

**Failure Criteria:**
- Partial records on failure → transaction not wrapping correctly
- Lock not released on error → need try/finally

**Fallback Strategy:**
If transactional write of multiple entities is complex in bun:sqlite, write a single `db.transaction()` wrapper that handles the multi-table insert.

**Testing:**
- Unit (mocked AI): Happy path → bullets created in pending_review
- Unit (mocked AI): AI returns malformed JSON → error, no records
- Unit (mocked AI): AI times out → error, lock released
- Unit: Concurrent derivation on same source → second call gets 409
- Unit: Concurrent derivation of perspectives on same bullet → second call gets 409 (in-memory lock)
- Unit: Derivation on rejected bullet → VALIDATION_ERROR (only approved bullets)
- Unit: AI returns empty `bullets: []` array → VALIDATION_ERROR (no bullets produced)
- Unit: Verify source_content_snapshot matches source.description at call time
- Unit: Stale lock recovery on startup resets sources stuck in `deriving` for >5 minutes
- Unit: Verify prompt_log entry created with correct template name and content
- Integration: Full flow with real repositories (in-memory db)

---

## Task 3.4: ResumeService

**File:** `packages/core/src/services/resume-service.ts`

**Methods:**
- `createResume(input: CreateResume): Result<Resume>`
- `getResume(id: string): Result<ResumeWithPerspectives>`
- `listResumes(offset, limit): Result<PaginatedResult<Resume>>`
- `updateResume(id, input): Result<Resume>`
- `deleteResume(id): Result<void>`
- `addPerspective(resumeId, input: AddResumePerspective): Result<void>`
- `removePerspective(resumeId, perspectiveId): Result<void>`
- `reorderPerspectives(resumeId, input: ReorderPerspectives): Result<void>`
- `analyzeGaps(resumeId: string): Result<GapAnalysis>`

**Gap analysis implementation:**
Reference: `refs/examples/gap-analysis/algorithm.md`

1. Get resume with perspectives
2. Compute domain coverage from included perspectives
3. Compare against expected domains for the archetype
4. Find unused approved bullets with no perspective for this archetype
5. Return structured gap report

**Acceptance Criteria:**
- [ ] Only approved perspectives can be added to resumes
- [ ] Gap analysis correctly identifies missing domains
- [ ] Gap analysis correctly identifies thin coverage
- [ ] Gap analysis correctly identifies unused bullets
- [ ] Reorder validates all perspective IDs belong to the resume

**Testing:**
- Unit: Add non-approved perspective → validation error
- Unit: Gap analysis with full coverage → empty gaps
- Unit: Gap analysis with missing domain → gap reported
- Unit: Gap analysis with unused bullet → unused_bullet gap reported
- Unit: Reorder with invalid perspective ID → error

---

## Task 3.5: AuditService

**File:** `packages/core/src/services/audit-service.ts`

**Methods:**
- `traceChain(perspectiveId: string): Result<ChainTrace>`
- `checkIntegrity(perspectiveId: string): Result<IntegrityReport>`

**ChainTrace:** Full chain from perspective → bullet → source, including all content snapshots.

**IntegrityReport:** Compares snapshots to current content, flags divergences.

```typescript
interface IntegrityReport {
  perspective_id: string
  bullet_snapshot_matches: boolean
  source_snapshot_matches: boolean
  bullet_diff?: { snapshot: string, current: string }
  source_diff?: { snapshot: string, current: string }
}
```

**Acceptance Criteria:**
- [ ] traceChain returns complete chain for valid perspective
- [ ] traceChain returns NOT_FOUND for invalid perspective
- [ ] checkIntegrity correctly identifies matching snapshots
- [ ] checkIntegrity correctly identifies divergent snapshots

**Testing:**
- Unit: Clean chain (no edits) → all snapshots match
- Unit: Edit source after derivation → source snapshot diverges
- Unit: Edit bullet after perspective derivation → bullet snapshot diverges

---

## Task 3.6: Review Queue Service

**File:** `packages/core/src/services/review-service.ts`

**Methods:**
- `getPendingReview(): Result<ReviewQueue>`

Returns counts and items from bullets and perspectives in `pending_review` status.

**Acceptance Criteria:**
- [ ] Returns correct counts for bullets and perspectives
- [ ] Items include relevant context (source title for bullets, bullet content for perspectives)

**Testing:**
- Unit: No pending items → empty queue
- Unit: Mix of pending bullets and perspectives → correct counts and items

---

## Parallelization

```
Task 3.1 (AI module) ─────────────────────────────┐
Task 3.2 (SourceService) ─────────────────────────┤
Task 3.2b (BulletService) ────────────────────────┤
Task 3.2c (PerspectiveService) ───────────────────┤ (all need repos from Phase 2)
Task 3.4 (ResumeService) ─────────────────────────┤ (also needs Phase 1 Task 1.4 constants)
Task 3.5 (AuditService) ──────────────────────────┤
Task 3.6 (ReviewService) ─────────────────────────┤
                                                   │
Task 3.3 (DerivationService) ◄────────────────────┘ (needs AI module 3.1 + all repos from Phase 2 + PromptLogRepo 2.6)
```

Tasks 3.1, 3.2, 3.2b, 3.2c, 3.4, 3.5, 3.6 can run in parallel. Task 3.3 depends on 3.1 (AI module) + Phase 2 completion (especially PromptLogRepository 2.6).

## Documentation

- `docs/src/lib/services.md` — service patterns, transaction handling, error propagation
- `docs/src/lib/ai-module.md` — Claude CLI integration, prompt templates, validation pipeline
