# TS → Rust rewrite: progress assessment

## Context
The user asked how far along the TS → Rust rewrite is. This was a read-only question, so
**no code changes are proposed**. This file records what I found so the answer can be checked.

Sources: `Cargo.toml` and `crates/**`, `packages/**`, the beads export at
`.beads/issues.jsonl` (the Dolt server returned "database beads_forge not found", so the
export is the only tracker data and may be stale), `.agents/plans/forge-resume-builder/refs/specs/2026-04-23-rust-implementation-phases.md`,
and `docs/src/migrations/mvp-2.0-browser-first.md`.

## Findings

| Phase | Tracker | What the code shows |
|---|---|---|
| R0 core types (`forge-core`) | closed | Done. 35 tests |
| R0 SDK (`forge-sdk`) | closed (9/9) | 22 data stores, the migration runner, the IR/LaTeX compiler and the export/audit/integrity/review services are implemented (283 tests). **11 of 16 domain services are still `todo!()`** (125 stubs). The `Forge` facade only holds the database connection |
| R0 AI (`forge-ai`) | closed | Done. Prompts, validators and the JD parser (62 tests). By design it makes no LLM calls |
| R1 Axum server | open (3/5) | Rust route files exist for 26 of 28 TS route modules (alignment and extension are missing). **They don't compile.** `main.rs` is `todo!()`. `lib.rs`, `state.rs`, `db.rs`, `response.rs`, `routes/templates.rs` and the SDK's `stores/template.rs` were never committed to any branch (forge-2qns, forge-es6o; restoring them is tracked as forge-7e4f). 0 tests |
| R1.5 `forge-wasm` (critical path) | open (4/7) | Done: wa-sqlite BrowserStore, skill graph (petgraph + HNSW), alignment engine (78 tests). Open: extraction pipeline (jsxn), CRDT log (8rzs), wasm-bindgen + Svelte integration (5x2h). **No package in `packages/` uses it yet** |
| R2 Tauri / R3 MCP / R4 Dioxus | open (0/5 each) | Not started. `forge-cli` and `forge-mcp` are 5-line `todo!()` binaries |

Size: about 28k lines of Rust against about 111k lines of TS/Svelte (core 49k, webui 43k).

Momentum: the last Rust commit was 2026-04-29 and the last commit of any kind was 2026-05-05.
There's been no activity for about 5 months.

Not verified: I didn't run `cargo check` or `cargo test` because plan mode is read-only.

## Verification (if wanted)
- `cargo check --workspace` should pass, because `forge-server` only compiles `main.rs`.
  The route modules are dead code.
- `cargo test --workspace` should run the about 458 tests across core, sdk, ai and wasm.

### Correction found after the assessment (2026-10-01)
The "wa-sqlite BrowserStore" row above overstated what is on `main`. forge-lu5s (typed
Statement/Transaction API, migration runner over wa-sqlite, `WaSqliteAdapter`, `SkillStore`,
and lifting `MIGRATIONS` into `forge-core`) lives only on `origin/forge-lu5s`. That branch is
15 commits ahead of `main` and 39 behind, and was **never merged**, even though the bead is
closed. `main` has only the forge-nst6 proof of concept (string-only `exec`/`query` over
`IDBBatchAtomicVFS`).
