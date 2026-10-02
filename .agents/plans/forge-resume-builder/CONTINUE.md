# Forge Resume Builder — Continuation Prompt

Use this prompt to resume implementation in a new session.

---

## Prompt

Continue implementing the Forge resume builder. Pick up where the last session left off.

### Context Recovery

1. **Read the memory:** Check `.claude/projects/-Users-adam-notes-job-hunting/memory/project_forge_session_2026_03_28.md` for detailed session state.

2. **Read the plan:** `.claude/plans/forge-resume-builder/PLAN.md` is the top-level plan. Phase plans are in `.claude/plans/forge-resume-builder/phase/`. Reference materials (contracts, schemas, strategies, examples) are in `.claude/plans/forge-resume-builder/refs/`.

3. **Read the spec:** `docs/superpowers/specs/2026-03-28-forge-resume-builder-design.md` is the design spec.

4. **Check beads for current state:**
   ```bash
   bd ready -n 20          # What's unblocked
   bd list --status closed  # What's done
   ```

5. **Verify the test suite still passes:**
   ```bash
   bun test packages/       # Should be 294 tests, 0 failures
   ```

### What's Done

- **Phase 0** ✓ — Monorepo scaffolded, Claude CLI risk gate passed
- **Phase 1** ✓ — Types (`packages/core/src/types/`), schema (`001_initial.sql`), migration runner (`migrate.ts` + `connection.ts`), constants (`archetypes.ts`)
- **Phase 2** ✓ — All 8 repositories (`packages/core/src/db/repositories/`): source, bullet, perspective, resume, employer, project, skill, prompt-log
- **T3.1** ✓ — AI module (`packages/core/src/ai/`): Claude CLI wrapper, prompt templates, output validator
- **T5.1** ✓ — SDK base (`packages/sdk/src/client.ts`): ForgeClient with request<T>/requestList<T>
- **T5.2** ✓ — SDK resources (`packages/sdk/src/resources/`): sources, bullets, perspectives, resumes, review

### What's Next

**Phase 3: Core Services** — 7 tasks, all unblocked, all parallelizable except T3.3:

| Task | Title | Key Reference | Dependencies |
|------|-------|---------------|-------------|
| T3.2 | SourceService | phase/3 | T2.2 (SourceRepo) ✓ |
| T3.2b | BulletService | phase/3 | T2.3 (BulletRepo) ✓ |
| T3.2c | PerspectiveService | phase/3 | T2.4 (PerspectiveRepo) ✓ |
| T3.3 | DerivationService | phase/3 (CRITICAL) | T3.1 (AI) ✓ + T2.2, T2.3, T2.6 ✓ |
| T3.4 | ResumeService | phase/3 + refs/examples/gap-analysis/ | T2.5 ✓ + T1.4 (constants) ✓ |
| T3.5 | AuditService | phase/3 | T2.4 ✓ |
| T3.6 | ReviewService | phase/3 | T2.3, T2.4 ✓ |

T3.2, T3.2b, T3.2c, T3.4, T3.5, T3.6 can all run in parallel. T3.3 can also run in parallel (its deps are all complete).

**After Phase 3:** T4.1 (Hono HTTP server setup) unblocks all Phase 4 routes.

### How to Execute

1. Claim tasks in beads: `bd update <id> --claim`
2. Launch parallel agents in worktrees for each task
3. Copy files from worktrees to main after completion (worktree agents can't run `bun test` — verify on main)
4. Run `bun test packages/` to verify all tests pass
5. Close tasks in beads: `bd close <id> --reason "..."`
6. Check `bd ready` for next unblocked work

### Service Pattern

All services follow this pattern (established in the plan):

```typescript
// Services are classes instantiated once via createServices(db)
// They take repositories as constructor dependencies
// All methods return Result<T> (never throw)
// Status transitions are validated in services, not repositories
// See phase/3 for the createServices() factory pattern
```

The service instantiation pattern is documented in `.claude/plans/forge-resume-builder/phase/3-core-services-and-ai-module.md` under "Service Instantiation Pattern".

### Important Notes

- **Beads global server:** 127.0.0.1:13306, database `beads_job-hunting`. Verify `bd list` works before starting.
- **`bd create --deps blocks:X`** means "this issue BLOCKS X" not "depends on X". See `memory/feedback_beads_deps.md`.
- **Worktree agents** write to isolated branches. Copy files to main with `cp .claude/worktrees/<agent>/packages/... packages/...` then verify.
- **Claude CLI envelope:** `claude -p "..." --output-format json` returns `{ result: "```json\n{...}\n```" }` — extraction logic is in `packages/core/src/ai/claude-cli.ts`.
