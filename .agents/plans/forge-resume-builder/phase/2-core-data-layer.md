# Phase 2: Core Data Layer (Repositories)

**Goal:** Implement all repository classes — pure data access with minimal validation.

**Non-Goals:** No derivation logic, no AI. Just CRUD + queries. Repositories perform field-level validation (non-null checks, FK existence) but NOT business rule validation (status transitions, derivation eligibility). Business rules live in services (Phase 3).

**Context:** Bun's `db.exec(sql)` handles multiple semicolon-separated statements in a single call. `db.run(sql, params)` handles single parameterized statements. The migration runner uses `db.exec()` for multi-statement SQL files. Repositories use `db.run()` and `db.query()` for parameterized queries. Bun bundles SQLite 3.45+ which supports `RETURNING` clauses.

**Depends on:** Phase 1 (types + schema + migration runner)
**Blocks:** Phase 3 (services depend on repositories)

---

## Task 2.1: Database Connection Helper

**Goal:** Create a reusable database connection factory.

**File:** `packages/core/src/db/connection.ts`

**Steps:**
1. Create `getDatabase(dbPath: string): Database` using `bun:sqlite`
2. Set PRAGMAs: `foreign_keys = ON`, `journal_mode = WAL`
3. Run migrations on first connection
4. Export a singleton for the application (configurable via env var `FORGE_DB_PATH`)

**Acceptance Criteria:**
- [ ] Returns a configured `bun:sqlite` Database instance
- [ ] Foreign keys are enforced (test with an FK violation)
- [ ] WAL mode is enabled
- [ ] Migrations run automatically on first connection
- [ ] In-memory mode (`:memory:`) works for testing

**Testing:**
- Unit: Create in-memory db, verify PRAGMAs
- Unit: Insert FK-violating row, verify error

---

## Task 2.2: SourceRepository

**File:** `packages/core/src/db/repositories/source-repository.ts`

**Methods:**
- `create(input: CreateSource): Source`
- `get(id: string): Source | null`
- `list(filter: SourceFilter, offset: number, limit: number): { data: Source[], total: number }`
- `update(id: string, input: UpdateSource): Source`
- `delete(id: string): void` (throws if has bullets)
- `acquireDerivingLock(id: string): Source | null` (atomic status update)
- `releaseDerivingLock(id: string, restoreStatus: SourceStatus, derived: boolean): void`

**Implementation Details:**
- UUID generation: `crypto.randomUUID()`
- `updated_at` set on every update
- `acquireDerivingLock` uses `UPDATE ... WHERE status != 'deriving' RETURNING *`
- Filter supports: `employer_id`, `project_id`, `status`
- Pagination: two queries (COUNT + SELECT with LIMIT/OFFSET)

**Acceptance Criteria:**
- [ ] CRUD operations work correctly
- [ ] `delete` throws when source has bullets (FK RESTRICT)
- [ ] `acquireDerivingLock` returns null if already deriving
- [ ] `list` pagination returns correct `total` count
- [ ] `list` filters work independently and combined
- [ ] Generated UUIDs are valid v4 format

**Testing:**
- Unit: Create source, get by ID, verify fields match
- Unit: Create source, update description, verify updated_at changes
- Unit: Create source + bullet, attempt delete → error
- Unit: Create source, acquire lock, attempt second lock → null
- Unit: List with filters, verify correct results
- Unit: List with pagination, verify offset/limit/total

---

## Task 2.3: BulletRepository

**File:** `packages/core/src/db/repositories/bullet-repository.ts`

**Methods:**
- `create(input: CreateBullet): Bullet` (includes source_content_snapshot, technologies)
- `get(id: string): Bullet | null` (includes technologies from junction table)
- `list(filter: BulletFilter, offset, limit): { data: Bullet[], total }`
- `update(id: string, input: UpdateBullet): Bullet`
- `delete(id: string): void` (throws if has perspectives)
- `updateStatus(id: string, status: BulletStatus, opts?: { rejection_reason?: string }): Bullet` — sets status, approved_at, approved_by, rejection_reason as appropriate. Does NOT validate transitions — that's the service's job.

**Implementation Details:**
- Technologies stored via `bullet_technologies` junction table
- Technology names lowercased and trimmed on insert for consistency
- `get` and `list` JOIN bullet_technologies and aggregate as array
- `updateStatus` is a generic status setter. The BulletService (Phase 3) validates transitions before calling this.
- `approved_at` and `approved_by = 'human'` set when status becomes `approved`
- Filter supports: `source_id`, `status`, `technology`
- Technology filter uses: `JOIN bullet_technologies bt ON ... WHERE bt.technology = ?`

**Acceptance Criteria:**
- [ ] Create bullet with technologies → technologies in junction table
- [ ] Get bullet → technologies array populated from junction table
- [ ] Delete bullet → cascade deletes from bullet_technologies
- [ ] Delete bullet with perspectives → error (RESTRICT)
- [ ] Approve from pending_review → success
- [ ] Approve from draft → error
- [ ] Reject without reason → error
- [ ] Reopen from rejected → status = pending_review
- [ ] Filter by technology returns correct results

**Testing:**
- Unit: Full CRUD lifecycle
- Unit: Technology junction table operations
- Unit: Technology normalization: "Kubernetes", "kubernetes", " kubernetes " all stored as "kubernetes"
- Unit: `updateStatus` sets `approved_at`/`approved_by` on approve, `rejection_reason` on reject
- Unit: Rejection with reason preserved
- Unit: Reopen preserves rejection_reason (history)
- Unit: Filter by technology across multiple bullets
- Unit: List with pagination where offset >= total → empty data, correct total
- Unit: `last_derived_at` not set by repository (set by service on lock release)

---

## Task 2.4: PerspectiveRepository

**File:** `packages/core/src/db/repositories/perspective-repository.ts`

**Methods:** Mirror of BulletRepository with perspective-specific fields.
- `create(input: CreatePerspective): Perspective`
- `get(id: string): Perspective | null`
- `getWithChain(id: string): PerspectiveWithChain | null` (JOINs bullet + source)
- `list(filter: PerspectiveFilter, offset, limit): { data, total }`
- `update`, `delete`, `approve`, `reject`, `reopen`

**Implementation Details:**
- `getWithChain` performs a 3-table JOIN: perspectives → bullets → sources
- Includes content snapshots in the response
- Filter supports: `bullet_id`, `target_archetype`, `domain`, `framing`, `status`

**Acceptance Criteria:**
- [ ] `getWithChain` returns nested bullet + source with correct data
- [ ] Content snapshots are preserved and returned
- [ ] All status transitions work correctly
- [ ] Filter by archetype + domain combination works
- [ ] Delete blocked if perspective is in a resume

**Testing:**
- Unit: Full CRUD lifecycle
- Unit: `getWithChain` returns correct nested structure
- Unit: All status transitions
- Unit: Multi-field filter combinations

---

## Task 2.5: ResumeRepository

**File:** `packages/core/src/db/repositories/resume-repository.ts`

**Methods:**
- `create(input: CreateResume): Resume`
- `get(id: string): Resume | null`
- `getWithPerspectives(id: string): ResumeWithPerspectives | null`
- `list(offset, limit): { data, total }`
- `update(id: string, input: UpdateResume): Resume`
- `delete(id: string): void` (cascades to resume_perspectives)
- `addPerspective(resumeId, input: AddResumePerspective): void`
- `removePerspective(resumeId, perspectiveId): void` — returns false/throws if perspective not in resume (404)
- `reorderPerspectives(resumeId, input: ReorderPerspectives): void` — wrapped in a transaction (DELETE all positions + INSERT new positions atomically)

**Implementation Details:**
- `getWithPerspectives` JOINs resume_perspectives → perspectives, groups by section, orders by position
- `reorderPerspectives` takes full list of `{ perspective_id, section, position }` and replaces all
- Delete cascades to resume_perspectives (but RESTRICT on perspectives themselves)

**Acceptance Criteria:**
- [ ] Resume CRUD works
- [ ] Add/remove perspectives updates join table
- [ ] Reorder replaces positions correctly
- [ ] Get with perspectives returns sections with correct ordering
- [ ] Delete resume cascades join table, does not delete perspectives

**Testing:**
- Unit: Full CRUD lifecycle
- Unit: Add multiple perspectives in different sections
- Unit: Reorder within section
- Unit: Move perspective between sections
- Unit: Delete resume, verify perspectives still exist

---

## Task 2.6: Supporting Repositories

**Files:**
- `packages/core/src/db/repositories/employer-repository.ts`
- `packages/core/src/db/repositories/project-repository.ts`
- `packages/core/src/db/repositories/skill-repository.ts`
- `packages/core/src/db/repositories/prompt-log-repository.ts`

**These are simpler — basic CRUD:**
- EmployerRepository: create, get, list, delete
- ProjectRepository: create, get, list, delete (filter by employer)
- SkillRepository: create, get, list (filter by category), getOrCreate (upsert by name)
- PromptLogRepository: create (append-only), getByEntity(type, id)

**Acceptance Criteria:**
- [ ] All basic CRUD operations work
- [ ] PromptLogRepository is append-only (no update, no delete)
- [ ] SkillRepository getOrCreate is idempotent

**Testing:**
- Unit: CRUD for each repository
- Unit: PromptLog create + query by entity

---

## Parallelization

All repositories can be developed in parallel once the connection helper exists. They share no state.

```
Task 2.1 (connection) ──┬──► Task 2.2 (sources)
                        ├──► Task 2.3 (bullets)
                        ├──► Task 2.4 (perspectives)
                        ├──► Task 2.5 (resumes)
                        └──► Task 2.6 (supporting)
```

## Test Fixtures

Create a shared test helper: `packages/core/src/db/__tests__/helpers.ts`
- `createTestDb()`: returns in-memory database with migrations applied
- `seedTestData(db)`: inserts a standard set of employers, sources, bullets, perspectives for tests
- Fixture data should mirror the e2e example from `refs/examples/e2e/derivation-chain.md`

## Documentation

- `docs/src/lib/repositories.md` — repository pattern, method signatures, query patterns
- `docs/src/data/models/schema.md` — update with any implementation discoveries
