# Phase 1: Service Rewiring Plan

**Status:** Draft
**Created:** 2026-04-10
**Depends on:** Phase 0 (complete — `worktree-forge-storage-phase0`)
**Branch:** `worktree-forge-storage-phase1` (and stacked sub-phase branches)
**Worktree:** `.claude/worktrees/forge-storage`

## Goal

Rewire all 26 Forge services to use `EntityLifecycleManager` (the integrity layer from Phase 0) instead of calling raw repository functions. Zero behavior change from the user's perspective. Services keep their existing public APIs and `Result<T>` envelope; only the implementation changes.

## Why Decompose

**Blast radius:** A single-shot migration of 26 services across ~6,000 lines of service code has real risk of cascading failures. One broken service → broken routes → broken MCP tools → broken WebUI.

**Context budget:** Full 26-service migration in one session risks context exhaustion mid-work, leaving a half-migrated service layer. That's worse than no migration.

**Review surface:** A focused batch of 3 services lets the user review the pattern once; subsequent batches are mechanical replication. Reviewing 26 services in one PR is impossible.

**Interrupt-friendly:** Each sub-phase lands a working state. If work stops mid-stream, the codebase is in a consistent state with some services migrated and the rest still on the old path. Both paths coexist via the `Services` container.

**Pattern establishment:** The first batch proves the wiring, error translation, and test compatibility. After that, the remaining services are a template-and-repeat exercise that can happen across multiple sessions or in parallel sub-branches.

## Architectural Principle

**Both paths coexist during Phase 1.** Services that have been rewired use `elm`; services that haven't still use raw repositories. The Services container passes both `db` and `elm` to every service constructor; each service decides which to use. This enables:

- Incremental migration (ship one service at a time)
- Easy rollback (revert a single commit, no global state)
- Parallel work (multiple sub-branches migrating different services)
- A/B debugging (if a service misbehaves after migration, compare against the still-raw siblings)

Once all 26 are migrated, the repositories are deleted in a final cleanup pass.

## Sub-Phase Decomposition

### Phase 1.0 — Infrastructure Wiring

**Scope:**
- Wire `SqliteAdapter` + `EntityLifecycleManager` + `SQLITE_NAMED_QUERIES` into `packages/core/src/index.ts`
- Add `elm: EntityLifecycleManager` to the `Services` container
- Pass ELM to every service constructor (alongside existing `db` parameter)
- Create a `storage-error-mapper.ts` helper that translates `StorageError` → the service layer's current error shape
- No service logic changes yet — services still call raw repos, just now have `elm` available

**Why:**
- This is the foundational step that unlocks everything. Without it, no service can be migrated.
- Error translation is critical: the service layer currently uses `{code: 'VALIDATION_ERROR' | 'CONFLICT' | 'NOT_FOUND' | ...}` codes that differ from Phase 0's `StorageErrorCode` union. Routes, MCP tools, and WebUI all depend on the existing codes. The mapper preserves backwards compatibility so no route handler or client needs to change.
- Doing the wiring as a single commit separates "infrastructure" from "migration". If the wiring breaks something, it's obvious what caused it; it's not entangled with a specific service rewrite.
- ELM is added alongside `db` (not replacing it) so every service keeps working without modification. Later sub-phases rewire method-by-method.

**Deliverables:**
- `packages/core/src/index.ts` — construct ELM at startup, pass to `createServices()`
- `packages/core/src/services/index.ts` — extend `Services` interface, update `createServices` signature and each service constructor call
- `packages/core/src/storage/error-mapper.ts` — `storageErrorToServiceError(err)` translation function
- All 26 service constructors updated to accept ELM (but not yet using it)
- All existing tests continue to pass (no behavior change)

**Commit:** `feat(storage): Phase 1.0 — wire EntityLifecycleManager into services container`

---

### Phase 1.1 — Pattern-Setting Migrations (3 simple services)

**Scope:** Migrate three nearly-identical simple services end-to-end:
1. `domain-service` — CRUD + regex validation + delete-protection
2. `industry-service` — CRUD + name validation + delete-protection
3. `role-type-service` — CRUD + name validation + delete-protection

**Why these three:**
- **Structural similarity:** All three are lookup/taxonomy tables with identical CRUD + delete-protection patterns. Migrating them together exercises the same code paths multiple times, making the error translation helper robust across repeated use.
- **Minimum viable proof:** They are the simplest services in the codebase. If the new path doesn't work on these, it definitely won't work on the complex ones. Start with easy wins.
- **Delete-protection validation:** All three have delete-protection via `restrict` rules in the entity map. This exercises the integrity layer's `RESTRICT_VIOLATION → CONFLICT` translation path, which is non-trivial.
- **No cross-entity coordination:** These services don't orchestrate writes across multiple entities. No transactions, no snapshot hooks, no fire-and-forget embeddings. Pure CRUD. Keeps the first batch focused on the core pattern without distracting complexity.
- **Repository deletion:** Once migrated, the raw repository files for these three can be deleted (Phase 1 completion criteria), shrinking the surface area of things to rewire next.

**Why not migrate `skill-service` or `archetype-service` first?**
- `skill-service` has an `FK → skill_categories(slug)` relationship that's non-standard. Worth doing early but not first.
- `archetype-service` has a junction table (`archetype_domains`) for domain linking, adding complexity. Better to establish the pattern first, then apply it.
- `profile-service` has a singleton-row pattern (single row in `user_profile`). Also non-standard. Save for later.

**Deliverables:**
- Three rewritten services, each ~30% shorter than before (validation boilerplate removed — ELM handles it)
- All existing tests for these three services still pass
- A short `HOWTO-migrate-service.md` capturing the exact template/pattern for future sub-phases

**Commits (one per service for clean git history):**
- `feat(storage): Phase 1.1.1 — migrate domain-service to EntityLifecycleManager`
- `feat(storage): Phase 1.1.2 — migrate industry-service to EntityLifecycleManager`
- `feat(storage): Phase 1.1.3 — migrate role-type-service to EntityLifecycleManager`

---

### Phase 1.2 — Simple CRUD Services (future session)

**Scope:** Migrate services that are primarily CRUD with minor extras:
- `archetype-service` — adds junction management for `archetype_domains`
- `skill-service` — adds FK to `skill_categories`, junction for `skill_domains`
- `summary-service` — adds FK to `industries`, `role_types`, junction for `summary_skills`
- `profile-service` — singleton-row pattern (enforced at app layer, not DB)
- `credential-service` — polymorphic JSON `details` column with type-specific validation
- `certification-service` — FK to `organizations`, junction for `certification_skills`
- `note-service` — polymorphic `note_references` many-to-many
- `contact-service` — three junction tables (`contact_organizations`, `contact_job_descriptions`, `contact_resumes`)
- `organization-service` — FK to `industries`, cascade to many junctions

**Why this batch:** All of these are CRUD with a twist. After Phase 1.1 proves the pattern, these services exercise the remaining "easy" complications: junction tables, polymorphic JSON fields, singleton rows, FK-to-lookup-table patterns. No transactions, no complex orchestration.

**Expected commits:** 9 commits, one per service.

---

### Phase 1.3 — Core Content Services (future session)

**Scope:** The services that own Forge's core content model and its coordination:
- `source-service` — source polymorphism (role/project/education/presentation extension tables), embedding lifecycle
- `bullet-service` — status FSM, embedding lifecycle
- `perspective-service` — status FSM, bullet_content_snapshot capture, embedding lifecycle
- `resume-service` — complex multi-table writes (sections, entries, skills, certifications), override management
- `job-description-service` — embedding lifecycle on raw_text, requirement parsing

**Why defer until last:**
- **Source polymorphism** requires coordinated writes across `sources` + one of four extension tables. This needs the ELM's transaction scope, which hasn't been stress-tested beyond the Phase 0 test.
- **Status FSM** logic (bullets, perspectives) is pure business logic that stays in the service layer. But the service still calls the ELM for the underlying writes. These are the first services that need the translation from "business rule validation" + "storage write" to coordinate cleanly.
- **Embedding lifecycle** was moved into the Phase 0 entity map's `afterCreate` hooks. These services currently call `queueMicrotask(() => embeddingService.embed(...))` directly. Migration means removing that code from the service since the hook handles it now.
- **Resume-service** is the single most complex service in Forge — it manages sections, entries, skills, certifications, overrides, and cross-table joins. It should be migrated LAST among writers because getting it wrong breaks the resume rendering path (user-visible).

**Expected commits:** 5 commits, one per service. Each likely touches 100+ lines.

---

### Phase 1.4 — Read-Only and Coordination Services (future session)

**Scope:** Services that don't do much writing but read heavily:
- `audit-service` — calls `traceChain` named query
- `integrity-service` — runs drift detection via SQL JOINs
- `review-service` — lists pending bullets/perspectives with chain context
- `export-service` — serializes entities to JSON/markdown/LaTeX
- `tagline-service` — pure computation from JD text
- `resume-compiler.ts` — builds the resume IR from multi-table joins

**Why last:** These services barely write. They mostly read. Their migration is the simplest (replace `repo.get(...)` with `elm.get(...)` and named queries) but they depend on the write-side migrations being complete so that the data they read is known-consistent. Doing them first would mean they might break if a write-side service was migrated incorrectly.

**Expected commits:** 6 commits.

---

### Phase 1.5 — Derivation Service (future session, standalone)

**Scope:** `derivation-service.ts` — the split-handshake protocol with `pending_derivations` locks.

**Why standalone:**
- This is the only service that manages explicit locks via a dedicated `pending_derivations` table.
- It uses the `prepare` → LLM call → `commit` pattern, with explicit transaction boundaries.
- The Phase 0 integrity layer does NOT specially handle `pending_derivations`; the service will need to use the `transaction()` scope and call the ELM from within.
- Stale lock recovery runs at server startup and must keep working.
- Risk level is high enough that this service deserves its own sub-phase with focused testing.

**Expected commits:** 1 or 2 commits.

---

### Phase 1.6 — Embedding Service (future session, standalone)

**Scope:** `embedding-service.ts` — the vector store and fire-and-forget hooks.

**Why standalone:**
- **Paradigm shift:** In Phase 0, the `afterCreate` hooks in the entity map call `embeddingService.embed(...)`. So the embedding service now receives calls FROM the integrity layer, not the other way around. This inverts a dependency.
- **Hook registration:** The embedding service becomes an injected dependency of `buildEntityMap(deps)`. The existing `queueMicrotask` pattern inside services should be removed since the hook replaces it.
- **No integrity layer usage:** The embedding service itself still needs to manage the `embeddings` table, likely using the ELM for consistency. But its core methods (`findSimilar`, `alignResume`, `matchRequirements`) involve cosine similarity computation that has no integrity-layer equivalent.
- **Semantic question:** Should `findSimilar` become a named query? Or stay as a service method that uses the ELM as a raw read backend? Either works; the decision depends on whether we want vector search to be capability-gated (`VectorCapableAdapter`).

**Expected commits:** 1-2 commits.

---

### Phase 1.7 — Template Service (future session, standalone)

**Scope:** `template-service.ts` — `createResumeFromTemplate` uses a single transaction to create a resume + clone sections atomically.

**Why standalone:**
- **Multi-entity atomic write:** This is the cleanest test of the ELM's `transaction(fn)` scope.
- **Built-in template protection:** The service rejects deletes of `is_builtin: 1` rows. This is pure business logic that stays in the service layer, but the ELM needs to know NOT to cascade delete built-in templates.
- **JSON column handling:** Templates store their sections as a JSON column. Deserialization via the entity map's `lazy` flag should work transparently, but this is the first service to exercise that path in write-heavy mode.

**Expected commits:** 1 commit.

---

### Phase 1.8 — Cleanup (future session)

**Scope:**
- Delete all raw repository files under `packages/core/src/db/repositories/` (28 files)
- Remove the `db: Database` parameter from service constructors (services only need ELM now)
- Update `createServices(db, elm)` to just `createServices(elm)` — DB is now internal to the adapter
- Delete the repository tests that are now redundant (their functionality is covered by the integrity layer tests)
- Update the HOWTO doc with a "Phase 1 complete" note

**Why last:**
- This is the "scorched earth" step that permanently removes the old path. Not reversible without git revert.
- Must run after ALL services are migrated, or you break the ones still on the old path.
- Deleting repository files also means deleting their tests — we need to be confident the integrity layer + migrated services have equivalent coverage.
- A final test run validates the full suite still passes with only the new path.

**Expected commits:** 1 large cleanup commit, or split into "delete repos" + "simplify constructors".

---

## Phase 1 Completion Criteria

Phase 1 is complete when:

- [ ] All 26 services call `this.elm.X(...)` instead of `SomeRepo.X(this.db, ...)`
- [ ] All raw repository files under `packages/core/src/db/repositories/` are deleted
- [ ] Service constructors accept only ELM (no `db` parameter)
- [ ] All existing tests pass (including the 12 pre-existing unrelated failures)
- [ ] All 35 Phase 0 storage tests still pass
- [ ] No behavior changes visible to routes, MCP tools, or WebUI
- [ ] `HOWTO-migrate-service.md` exists as a canonical reference

## Test Strategy

At every sub-phase commit:
1. Run the full core test suite: `just docker-test` (or `bun test packages/core`)
2. Confirm test count doesn't regress
3. Confirm the 12 known pre-existing failures don't grow or change

If a sub-phase adds net failures, that's a regression and the commit is rolled back before proceeding.

## Risk Register

| Risk | Mitigation |
|---|---|
| Error-mapper incomplete → routes return wrong error codes | Phase 1.0 ships with a unit test covering every `StorageErrorCode` → service code mapping |
| ELM rejects data that old path accepted → service tests break | PRAGMA test catches schema drift; integration tests catch semantic drift |
| ELM is slower than raw repos on hot paths (e.g., resume compiler) | Benchmark before/after on representative operations; if slowdown is >20%, add a fast path using `adapter.executeNamedQuery()` |
| Circular dependency between ELM and embedding service | Phase 1.6 is intentionally standalone; the factory pattern (`buildEntityMap(deps)`) already handles the dep injection cleanly |
| Context exhaustion mid-sub-phase | Each sub-phase is scoped to fit comfortably in a single session; if it doesn't, split it further |

## Branching Strategy

All Phase 1 sub-phases land on `worktree-forge-storage-phase1` as stacked commits. Sub-phase identity is preserved in commit messages (`feat(storage): Phase 1.2 — migrate archetype-service`).

If a sub-phase is large enough to warrant isolation during review, it can branch off as `worktree-forge-storage-phase1.N` and merge back into `-phase1` when stable.

## Session Resumption Notes

If Phase 1 is paused and resumed in a later session:
- Check `git log --oneline worktree-forge-storage-phase0..worktree-forge-storage-phase1` to see which sub-phases have landed
- The `HOWTO-migrate-service.md` file captures the exact pattern to replicate
- The storage tests from Phase 0 are the canary; as long as they pass, the foundation is intact
- Each service file shows at the top whether it's migrated (comment: `// Phase 1: uses EntityLifecycleManager`) or still on the raw-repo path
