# Phase 1.2 Resumption Prompt

Copy-paste the block below into a new Claude Code session to pick up
where Phase 1.1 left off.

---

Pick up the Forge storage abstraction work at **Phase 1.2**. Phase 0
(entity lifecycle manager + SQLite adapter + entity map), Phase 1.0
(services container wiring + error mapper), and Phase 1.1 (domain,
industry, role-type migrations + HOWTO doc) are already shipped as
committed branches in `.claude/worktrees/forge-storage`.

**Before starting, check memory:**
- `project_storage_abstraction_2026_04_10.md` — Phase 0 architecture
- `project_storage_phase1_0_2026_04_11.md` — Phase 1.0 context
- `project_storage_phase1_1_2026_04_11.md` — Phase 1.1 context (if
  present — it may be named differently depending on what the previous
  session wrote)
- `.claude/plans/forge-resume-builder/refs/specs/2026-04-10-phase1-service-rewiring-plan.md` — full decomposition
- `.claude/plans/forge-resume-builder/refs/HOWTO-migrate-service.md` —
  the canonical migration recipe from Phase 1.1

**Set up the worktree:**
1. If not already inside, enter the worktree at `.claude/worktrees/forge-storage`
2. The current branch is `worktree-forge-storage-phase1` with Phase 1.1
   commits (1.1.1, 1.1.2, 1.1.3, 1.1-doc) on top of Phase 1.0
3. Verify with `git log --oneline -8` — you should see the three
   `Phase 1.1.N` commits and a `docs(storage): Phase 1.1 complete`
   commit
4. Run the containerized baseline: `docker compose --profile tools run
   --rm shell -c 'cd /app && bun run --filter "@forge/core" test'` —
   expect **1788 pass / 12 fail / 1800 total** (the 12 are pre-existing,
   unrelated)

**Phase 1.2 scope — migrate 9 CRUD-with-a-twist services:**

Each of these services is primarily CRUD plus one or two non-trivial
wrinkles. Migrate in order:

1. **`archetype-service.ts`** — CRUD + junction management for
   `archetype_domains`. Adds the pattern "service writes the parent row
   via `elm.create`, then writes junction rows via
   `elm.create('archetype_domains', {...})` in a loop."

2. **`skill-service.ts`** — CRUD + FK on `skill_categories.slug` (non-id
   FK) + junction for `skill_domains`. First service to exercise the
   non-id FK path in the entity map.

3. **`summary-service.ts`** — CRUD + FKs to `industries` and
   `role_types` + junction for `summary_skills`. The FK path is already
   proven by skill-service; this reuses it.

4. **`profile-service.ts`** — singleton-row pattern. The
   `user_profile` table has at most one row. Service enforces this at
   the app layer; ELM does not know about it. Keep the singleton check
   in the service; only the underlying storage calls change.

5. **`credential-service.ts`** — polymorphic `details` JSON column with
   type-specific validation (education vs clearance vs license). Keep
   the type-discriminated validation in the service; ELM handles the
   storage round-trip.

6. **`certification-service.ts`** — FK to `organizations` + junction
   for `certification_skills`.

7. **`note-service.ts`** — polymorphic `note_references` many-to-many.
   The `note_references` junction has a `referenced_entity_type` column
   and a `referenced_id` column. Verify the entity map models this
   correctly (no FK constraint, just a text column) before migrating.

8. **`contact-service.ts`** — three junction tables
   (`contact_organizations`, `contact_job_descriptions`,
   `contact_resumes`). Most complex junction-heavy service in Phase 1.2.

9. **`organization-service.ts`** — FK to `industries`, cascade to many
   junctions (several `contact_organizations`, tags, campuses,
   aliases). Most complex service in Phase 1.2 due to the cascade
   breadth. Save for last in this sub-phase.

**Per-service workflow:** Follow
`.claude/plans/forge-resume-builder/refs/HOWTO-migrate-service.md`
exactly. It has the canonical skeleton, the checklist, the gotchas, and
the non-goals.

**After each service migration:**
- Run the service's test file:
  `docker compose --profile tools run --rm shell -c 'cd /app && bun
  test packages/core/src/services/__tests__/<service-name>.test.ts'`
- Commit with message `feat(storage): Phase 1.2.N —
  migrate <service-name> to EntityLifecycleManager`.

**After all 9 services done:**
- Run the full core test suite: still **1788 pass / 12 fail / 1800
  total** expected (no regressions)
- Run the storage tests: `bun test packages/core/src/storage/__tests__`
  — 51/51 expected
- Update the HOWTO doc with any new gotchas Phase 1.2 discovers
- Checkpoint commit:
  `docs(storage): Phase 1.2 complete — updated HOWTO`

**Checkpoint when done.** Do NOT proceed to Phase 1.3 in the same
session; leave a new resumption prompt for Phase 1.3.

**New gotchas Phase 1.1 didn't encounter (watch out in 1.2):**

- **Junction table creates.** Junction tables have composite primary
  keys (e.g., `archetype_domains` = `(archetype_id, domain_id)`). The
  ELM's `create` auto-generates an `id` only when the entity map's
  `id` field is required text. For junctions, call
  `elm.create('archetype_domains', { archetype_id, domain_id })` — the
  ELM will skip id auto-generation because the entity has no `id`
  field. Verify with the Phase 0 PRAGMA test that the junction is
  modeled correctly.
- **Junction table deletes.** Use `elm.deleteWhere('archetype_domains',
  { archetype_id: id })` — not `elm.delete(...)` — because there is no
  single id. `deleteWhere` bypasses cascade/restrict/setNull, which is
  fine for junctions because the entity map should not have cascades
  declared on the junction itself.
- **Multi-step transactions.** Services that write a parent row + N
  junction rows should consider using `elm.transaction(async (tx) =>
  {...})` for atomicity. However, `TransactionScope` exposes only
  adapter-level CRUD (no validation), so service-layer validation
  must still run before opening the transaction. For Phase 1.2,
  starting without explicit transactions is fine — each `elm.create`
  is its own transaction — but flag any atomicity-sensitive paths for
  Phase 1.3's review.
- **Services with FK to non-id columns.** The entity map's FK field
  supports `field: 'slug'`, `field: 'id'`, etc. The ELM's
  `validateForeignKeys` correctly handles non-id FKs via
  `adapter.count`. Just make sure the entity map entry for the child
  entity has the right `fk.field` value.
