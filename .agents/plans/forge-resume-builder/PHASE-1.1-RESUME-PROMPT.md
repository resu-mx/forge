# Phase 1.1 Resumption Prompt

Copy-paste the block below into a new Claude Code session to pick up where Phase 1.0 left off.

---

Pick up the Forge storage abstraction work at **Phase 1.1**. Phase 0 (entity lifecycle manager + SQLite adapter + entity map) and Phase 1.0 (services container wiring + error mapper) are already shipped as committed branches in `.claude/worktrees/forge-storage`.

**Before starting, check memory:**
- `project_storage_abstraction_2026_04_10.md` — Phase 0 architecture
- `project_storage_phase1_0_2026_04_11.md` — Phase 1.0 context and Phase 1.1 scope
- `.claude/plans/forge-resume-builder/refs/specs/2026-04-10-phase1-service-rewiring-plan.md` — full decomposition

**Set up the worktree:**
1. If not already inside, enter the worktree at `.claude/worktrees/forge-storage` (it already exists — do not create a new one)
2. The current branch should be `worktree-forge-storage-phase1` with the Phase 1.0 commit on top of Phase 0's two commits
3. Verify with `git log --oneline -4` — you should see: Phase 1.0 → Phase 0 storage → Phase 0 docker → (prior main commit)
4. Run the containerized test baseline to confirm green state: `docker compose --profile tools run --rm shell -c 'cd /app && bun run --filter "@forge/core" test'` — expect **1800 pass / 12 fail** (the 12 are pre-existing, unrelated)

**Phase 1.1 scope — migrate three simple services to EntityLifecycleManager:**

1. **`domain-service.ts`** — CRUD + regex name validation + delete-protection via ELM's restrict rules (the DB has no FK from domains, but the entity map declares cascade on archetype_domains and skill_domains; restrict violations come from perspectives referencing the domain name and archetype references)
2. **`industry-service.ts`** — CRUD + empty-name validation + delete-protection (ELM handles organizations/summaries setNull automatically)
3. **`role-type-service.ts`** — CRUD + empty-name validation + delete-protection (ELM handles summaries setNull)

For each service:

1. Replace `import * as FooRepo from '../db/repositories/foo-repository'` with no imports (ELM replaces it)
2. Rewrite CRUD methods to call `this.elm.create/get/list/update/delete('entityType', ...)` instead of `FooRepo.x(this.db, ...)`
3. Use `liftStorageResult` from `../storage/error-mapper` to convert `Result<T>` envelopes
4. Cast the generic row to the domain type: `result.value as Domain`
5. Keep service-level validation that the ELM doesn't handle (e.g. regex format checks for `domain-service`'s name pattern)
6. Remove pre-delete reference counting — the integrity layer's restrict rules handle it
7. Remove error-message string matching (`message.includes('UNIQUE constraint')`) — the error mapper handles UNIQUE_VIOLATION → CONFLICT automatically

**After each service migration:**
- Run `docker compose --profile tools run --rm shell -c 'cd /app && bun test packages/core/src/services/__tests__/<service-name>.test.ts'` to verify that service's existing tests still pass
- Commit with message `feat(storage): Phase 1.1.N — migrate <service-name> to EntityLifecycleManager`

**After all 3 services done:**
- Run the full core test suite: still 1800 pass / 12 fail expected
- Run the storage tests: `bun test packages/core/src/storage/__tests__` — 51/51 expected
- Write a short `HOWTO-migrate-service.md` capturing the exact pattern so Phase 1.2+ can be mechanical
- Final commit: `docs(storage): Phase 1.1 complete — HOWTO for service migrations`

**Checkpoint when done.** Do NOT proceed to Phase 1.2 in the same session; leave a new resumption prompt for Phase 1.2.

**Gotchas to watch for:**
- The service layer uses `Result<T>` with `.data` (not `.value`) — the `liftStorageResult` helper handles the rename
- `PaginatedResult<T>` has a different shape than `Result<ListResult>` — wrap the ELM's list result into `{ ok, data: rows, pagination }` manually
- `DomainRepo.CreateDomainInput` types are still imported for signature compatibility — only the RUNTIME calls change, not the TypeScript input types (yet)
- Tests may construct services directly via `new DomainService(db)` — the `buildDefaultElm` fallback handles this, but be aware that the test ELM has no embedding service so hooks are no-ops
- `error-mapper.ts` exports `liftStorageResult<T>()` which is the primary helper services should use — it converts `StorageResult<T>` → service `Result<T>` in one call

Start by reading `domain-service.ts` and its existing test file, then plan the rewrite.
