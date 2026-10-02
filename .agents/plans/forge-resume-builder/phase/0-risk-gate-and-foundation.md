# Phase 0: Risk Gate & Foundation

**Goal:** Verify the Claude Code CLI invocation pattern is stable, and scaffold the monorepo with all workspace configuration.

**Non-Goals:** No business logic, no database, no UI. This phase is purely structural.

**Duration:** Short — this is a gating phase.

**Depends on:** Nothing
**Blocks:** All other phases (monorepo structure), Phase 3 specifically (AI module risk gate)

---

## Task 0.1: Verify Claude Code CLI Invocation (RISK GATE)

**Goal:** Confirm `claude -p "prompt" --output-format json` works reliably for programmatic use.

**Steps:**
1. Run `claude -p "Respond with JSON: {\"test\": true}" --output-format json` from a Bun script via `Bun.spawn()`
2. Verify: stdout is parseable JSON
3. Verify: process exits cleanly with exit code 0
4. Verify: timeout behavior — what happens when killed after N seconds
5. Verify: error output — what does stderr contain on failure
6. Test with a realistic prompt (the source-to-bullet template from `refs/examples/prompts/source-to-bullet.md`)
7. Document findings in `docs/src/lib/ai-module-spike.md`

**Acceptance Criteria:**
- [ ] `Bun.spawn()` successfully invokes `claude` and captures stdout
- [ ] JSON output is parseable
- [ ] Process can be killed via timeout (Bun's AbortSignal or process.kill)
- [ ] Findings documented with exact flags and behavior notes

**Failure Criteria:**
- `claude -p` does not exist or is not stable → STOP. Reevaluate AI module approach.
- JSON output is inconsistent or wrapped in extra formatting → document workarounds

**Fallback Strategy:**
If `claude -p --output-format json` is unreliable:
1. Try `claude -p "prompt"` without `--output-format json` and extract JSON from freeform output
2. If that fails, switch AI module to use Anthropic API directly (changes ADR-004 scope)
3. Document the decision as an ADR amendment

**Testing:**
- Smoke test: single invocation, parse result
- Timeout test: prompt that takes >5s, verify kill behavior
- Error test: invalid prompt, verify error handling

**Documentation:** `docs/src/lib/ai-module-spike.md` — findings, exact flags, example output, edge cases

---

## Task 0.2: Scaffold Monorepo

**Goal:** Create the full directory structure with workspace configuration.

**Can parallelize with:** Task 0.1 (independent)

**Post-scaffold:** Initialize beads issue tracker (`bd init forge`) in the new repo root, then create all plan issues as beads (phases as epics, tasks as issues with `blocks` dependencies matching the plan's dependency graph). This converts the plan into a trackable, multi-session work graph.

**Steps:**

1. Create new repository `forge/` (or subdirectory — confirm with user)
2. Initialize Bun workspace:
   ```
   forge/
   ├── package.json          (workspaces: ["packages/*"])
   ├── bunfig.toml
   ├── tsconfig.json          (base config)
   ├── .gitignore
   ├── .env.example
   ├── justfile
   └── packages/
       ├── core/
       │   ├── package.json   (name: @forge/core)
       │   ├── tsconfig.json
       │   └── src/
       │       ├── index.ts
       │       ├── db/
       │       │   ├── migrations/
       │       │   ├── migrate.ts
       │       │   └── repositories/
       │       ├── services/
       │       ├── ai/
       │       ├── routes/
       │       └── types/
       ├── sdk/
       │   ├── package.json   (name: @forge/sdk, deps: none)
       │   ├── tsconfig.json
       │   └── src/
       │       ├── index.ts
       │       ├── client.ts
       │       └── types.ts
       ├── cli/
       │   ├── package.json   (name: @forge/cli, deps: @forge/sdk)
       │   ├── tsconfig.json
       │   └── src/
       │       └── index.ts
       ├── webui/
       │   ├── package.json   (name: @forge/webui, deps: @forge/sdk)
       │   ├── vite.config.ts
       │   ├── tsconfig.json
       │   └── src/
       │       └── main.ts
       └── mcp/
           ├── package.json   (name: @forge/mcp, deps: @forge/sdk)
           └── src/
               └── index.ts   (stub with comments)
   ```

3. Initialize Cargo workspace:
   ```
   forge/
   ├── Cargo.toml            (workspace members = ["crates/core"])
   └── crates/
       └── core/
           ├── Cargo.toml
           └── src/
               ├── lib.rs
               ├── db/
               │   └── mod.rs
               ├── services/
               │   └── mod.rs
               ├── ai/
               │   └── mod.rs
               └── routes/
                   └── mod.rs
   ```

4. Create docs structure:
   ```
   forge/docs/src/{adrs,data/models,architecture,mvp,api,sdk,lib,cli,webui,mcp}/
   ```

5. Create `data/.gitkeep` (database directory — `.gitignore` excludes `data/*.db`)

6. Create justfile with dev recipes (see `refs/spec/dev-environment.md`). **Important:** `just dev` must use process group management so Ctrl-C kills both processes. Use `trap` or `concurrently` package:
   ```justfile
   dev:
       npx concurrently -k "bun run --filter '@forge/core' dev" "bun run --filter '@forge/webui' dev"
   ```
   Or use `trap` to kill the backgrounded process on exit.

7. Create `.env.example` (see `refs/spec/dev-environment.md`)

**Acceptance Criteria:**
- [ ] `bun install` succeeds with no errors
- [ ] `bun run --filter '@forge/core' dev` starts (even if it does nothing yet)
- [ ] `cargo check --workspace` passes (Rust stubs compile)
- [ ] All package.json files have correct names, workspace protocol deps
- [ ] `.gitignore` covers: node_modules, data/*.db, .env, dist/, target/
- [ ] `data/.gitkeep` exists
- [ ] `just dev` runs without error (even if servers don't do anything yet)
- [ ] `just dev` Ctrl-C kills both processes cleanly (no orphan processes)
- [ ] `packages/mcp/src/index.ts` stub contains comments listing intended MCP tool names: `forge_search_sources`, `forge_derive_bullets`, `forge_review_pending`, `forge_get_chain`, `forge_gap_analysis`

**Failure Criteria:**
- Bun workspace resolution fails → check workspace protocol syntax
- Cargo workspace fails → check members path in root Cargo.toml

**Fallback Strategy:** N/A — this is scaffolding, issues are configuration bugs.

**Testing:**
- Smoke: `bun install && bun run --filter '*' build` passes
- Smoke: `cargo check --workspace` passes

**Documentation:**
- `docs/src/architecture/monorepo.md` — workspace layout, package names, dependency graph
- ADR files: copy ADR-001 through ADR-007 from spec into `docs/src/adrs/`
