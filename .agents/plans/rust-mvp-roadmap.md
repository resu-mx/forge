# Roadmap: minimal Rust Forge (browser-first, core loop)

**Date**: 2026-10-01
**Status**: Proposed. M0 (merged) and M1 (spike, see below) are done; the rest is a plan.
**Parent**: `forge-nfpz` (Rust Rewrite). Baseline: [`this-repo-is-in-moonlit-pinwheel.md`](./this-repo-is-in-moonlit-pinwheel.md) (progress assessment).

## Decisions

| Decision | Choice |
|---|---|
| Shape | **Browser-first WASM.** The Rust data layer runs in the browser and the Svelte UI stays |
| Scope | **Core loop only.** Sources → bullets → perspectives → resume → rendered output |
| Derivation | **Claude in Chrome as the bridge.** Claude Code drives the open Forge tab via `javascript_tool`, which replaces the HTTP-only MCP |
| Rendering | **Typst compiled to WASM** for PDF (zero subprocesses) |

## Definition of done ("minimal functional")
1. A user opens the static Forge app in Chrome. All data lives in browser SQLite (OPFS), served by Rust/WASM. No server runs.
2. The core loop works end to end:
   - Create a profile, orgs and sources.
   - Claude derives bullets and perspectives through Claude in Chrome.
   - The user approves them in the UI.
   - The user builds a resume (from a template or by hand) and gets a Typst-rendered PDF in the browser.
3. An existing `data/forge.db` can be imported, and the database can be exported, so data survives browser storage eviction.
4. Endpoints outside Tier 0 return a 501 error in the usual envelope, and the UI keeps working.

## Architecture (one router, two hosts)
```
Svelte webui ──► TS SDK (ForgeClient, custom `fetch`) ──► WASM transport ──postMessage──►
  Dedicated Worker: forge-wasm ──► forge-api (axum Router, no tokio) ──► forge-sdk stores
                                                                     ──► rusqlite → sqlite-wasm-rs (OPFS sahpool VFS)
Native: forge-server (tokio/hyper) ──► same forge-api Router ──► forge-sdk ──► rusqlite (file)
Claude Code ──claude-in-chrome javascript_tool──► window.forge (same ForgeClient)
```
Key levers (from upstream release notes and reading the code; **not yet confirmed by a build**, which is what M1 is for):
- **rusqlite ≥ 0.38 supports `wasm32-unknown-unknown` via sqlite-wasm-rs.** The repo pins 0.32. If it works,
  the existing `forge-sdk` (22 stores, compiler, 283 tests) runs in the browser unchanged. No duplicate
  wa-sqlite store layer and no async refactor.
- **axum runs on wasm32** with `default-features = false` (official `simple-router-wasm` example). The existing
  `crates/forge-server/src/routes/*` (about 128 endpoints, written to the TS JSON contract) become the
  in-browser API too, so the web UI and SDK change only their transport.
- **The TS SDK sends every request through `fetch`** (`packages/sdk/src/client.ts:187,352`). One injectable
  `fetch` option reroutes it. The web UI makes 18 raw `fetch` calls in 4 files that need converting to SDK calls.

**This reverses earlier decisions, so flag it in the doc.** forge-nst6 decision A (wa-sqlite from npm) and
decision B (main thread, no Worker) are superseded: OPFS sahpool requires a dedicated Worker. forge-5x2h's
`query(sql)` API is replaced by dispatch through the REST contract. The forge-lu5s approach (re-implementing
each store over wa-sqlite) becomes the **fallback**, and its unmerged branch `origin/forge-lu5s` is kept for that.

## M0: Unblock the workspace
- Fix `just`:
  - Root `justfile:11` imports the missing `packages/justfile` (forge-6t8v). Every recipe fails because of it.
  - Add `.env.example`.
  - Make the wasm recipes portable; they hard-code an `aarch64-apple-darwin` rustup path at `justfile:107,116`.
- Add `rust-toolchain.toml` (stable + `wasm32-unknown-unknown`).
- Add Rust CI on PRs: `fmt`, `clippy`, `cargo test --workspace`, and `cargo check --target wasm32-unknown-unknown -p forge-wasm`. There is no CI today except extension publishing.
- Re-scope forge-7e4f. The uncommitted server files (`lib.rs`, `state.rs`, `db.rs`, `response.rs`,
  `routes/templates.rs`, `stores/template.rs`, `tests/`) aren't on any branch or in this checkout.
  Ask once whether the maintainer's old tree (`/Users/adam/code/proj/forge`) survives. If not, rewrite them in M2.
- **Done when:** `just --list` works, and CI is green on a no-op PR.

## M1: Spike, Rust SQLite and router in a browser Worker (decision gate)

**Status: done 2026-10-01. Path A confirmed in Chrome.** See `docs/src/dev/adrs/rust-wasm/0001-rusqlite-and-axum-in-a-browser-worker.md`.
Not yet verified: Firefox/Safari, and a Linux CI build of the wasm SQLite. Two constraints found: the build needs a clang with the
wasm backend, and `sqlite-wasm-vfs` must be 0.2.x to match rusqlite 0.40's `sqlite-wasm-rs` 0.5.
Build a throwaway harness in `crates/forge-wasm/examples/` (it extends `browser-smoke`). It must show:
1. rusqlite upgraded 0.32 → ≥ 0.38 (latest is 0.40.2), with all forge-sdk tests still green natively.
2. `forge-sdk` compiles to wasm32. Watch for getrandom's wasm backend feature (uuid v4) and chrono's `wasmbind` (on by default).
3. The sqlite-wasm-rs OPFS sahpool VFS is installed in a dedicated Worker. All 52 migrations apply, and a store
   round-trip survives a reload. Check in Chrome and Firefox.
4. axum `Router` (no tokio) gets a `oneshot` dispatch inside the Worker that returns `/api/health`.
5. Bundle size is measured: the data layer must be < 5 MB (forge-5x2h's criterion).

**Gates:**
- If (2) or (3) fails, take **Path B**: merge `origin/forge-lu5s` (wa-sqlite typed API, migrations moved into forge-core), then do forge-txdt option A (an async SQL-executor trait over rusqlite and wa-sqlite, with stores made async). M3 and M4 roughly double in size.
- If (4) fails, write a `match`-based dispatcher in forge-wasm that calls the same handler functions.

Record the outcome as an ADR.

## M2: `forge-api` crate (transport-agnostic router) and a native server that runs

**Status: done 2026-10-02.** `forge-api` holds the router and compiles for wasm32 without tokio; `forge-server` is a thin native host.
Verified: 11 new tests (6 router, 5 server over a real socket), the real binary serving all UI load-time GETs with 200, and a
derivation prepare → commit producing an `in_review` bullet with the TS payload shapes. Not verified: the Svelte UI itself and the
TS SDK (no `bun` on the dev machine), and a wasm32 CI job for `forge-api` (needs a wasm-capable clang on the runner).
Deviations: the server binds 127.0.0.1 by default (`FORGE_HOST` to change) because the API has no auth; the TS server listens on all interfaces.
- New crate `crates/forge-api`:
  - Move `forge-server/src/routes/*` and `error.rs` into it.
  - Write the lost glue:
    - `SharedState` as `Arc<Mutex<rusqlite::Connection>>`.
    - `with_conn`, an `FnOnce` closure runner: `spawn_blocking` natively, a direct call on wasm (behind `cfg`).
    - The response types: `ApiData<T> { data }`, `ApiList<T> { data, pagination }`, `Created<T>` (201 + `{data}`), and
      `NoContent` (204). Shapes are inferred from how the existing route files call them.
  - axum `default-features = false`.
- `forge-server` becomes a thin tokio binary:
  - `FORGE_PORT`/`FORGE_DB_PATH`, migrations via `Forge::open`, and `DerivationStore::cleanup_expired` at startup.
  - CORS matching `packages/core/src/routes/server.ts:65-85`, request ID, a JSON 500 handler, and a JSON envelope for axum extractor rejections.
- Rewrite `TemplateStore` and `routes/templates.rs`, and make `POST /resumes` honor `template_id`. The forge-core types already exist; the rules are in TS `template-service.ts`.
- **Done when:** `cargo run -p forge-server` serves `/api/health`, and the web UI pointed at it via `FORGE_API_URL` loads.

## M3: Tier-0 API parity (native first) plus the parity harness

**Status: done 2026-10-02 for the Tier-0 files.** `just parity` runs the TS route tests against the Rust server (`FORGE_TEST_SERVER_BIN`);
`just parity-tier0` is the CI gate. Baseline 241/415 passing; now 300/415, with every Tier-0 file (contracts, sources, bullets, perspectives,
resumes, derivations, profile) plus export, CORS, server and review passing (154 tests). The 115 remaining failures are Tier-1/2 files
(job descriptions, extension, campuses, credentials, certifications, summaries, domains, archetypes, supporting).
Not done: the stubbed `forge-sdk` services were not implemented; the validation rules were put in the stores instead, where the routes call them.
`POST /pdf` and `?format=pdf` answer 501 until M5. Tagline, resume job-description and contact links are Tier 2 and not ported.
Tier-0 endpoints (about 40), taken from what the web UI, MCP server and CLI call:
- health, `GET`/`PATCH` profile, `GET` archetypes and domains, `GET`/`POST` orgs
- sources CRUD, derivations prepare/commit
- bullets and perspectives: list/get/patch/create/approve/reject
- resumes CRUD, sections CRUD, entries CRUD and reorder
- `GET /resumes/:id/ir`, `GET /export/resume/:id?format=markdown|latex|json`
- `GET /review/pending` and `GET /integrity/drift`, both called by the dashboard

Work items:
- **Fix the wire-format mismatches** found by reading the Rust route files against the TS routes (not compiler-verified):
  - Entry reorder must be PATCH, with field `id`, returning 200 `{data:null}`.
  - Sources must serialize `role`/`project`/`education`/`presentation` instead of `extension`, accept nested inputs, and have `update` write extension rows.
  - Add a double-option deserializer so PATCH can clear nullable fields (72 fields in `forge-core/src/types/inputs.rs`).
  - `ON DELETE RESTRICT` must map to 409, not 500.
  - Drift `entity_type` must be lowercase.
  - Rejection reason must be required and non-empty.
- **Port the Tier-0 validation rules** from the TS services into the stubbed Rust services (source, bullet, perspective,
  resume, organization, profile). Implement those stubs and leave the other `todo!()` stubs in place.
  The stubs' doc comments already list the rules.
- **Parity harness:** make `packages/core/src/routes/__tests__/helpers.ts` `apiRequest` honor `FORGE_TEST_BASE_URL`.
  Run the Tier-0 route test files (`contracts`, `sources`, `bullets`, `perspectives`, `resumes`, `derivations`, `profile`)
  against a spawned `forge-server` with a temp DB per file. Seed by writing directly to the shared file, and add
  `busy_timeout` on the TS side.
- **Done when:** all Tier-0 TS route tests pass against Rust, and the web UI completes the loop against native Rust (PDF excepted).

## M4: Browser runtime (forge-wasm hosts forge-api)

**Status: done 2026-10-02 in Chrome, apart from the two items below.** See ADR 0002. `forge-wasm` now hosts `forge-api` in a
dedicated Worker over OPFS; `@forge/runtime` provides the Worker client, Web Lock owner election and a `fetch`; the web UI runs
the core loop with no server when built with `VITE_FORGE_MODE=wasm`; export/import and a Storage settings page are in.
Deviations: the UI's raw `fetch('/api/...')` calls are routed by a global fetch wrapper instead of being rewritten to SDK calls; a SharedWorker
cannot own the database (verified), so a second tab waits and takes over rather than sharing. Runtime: 2.88 MB raw / 0.94 MB gzipped.
**Not done:** the headless `wasm-pack test` harness (the browser behaviour was verified by driving Chrome, and the dispatch layer has native
tests); Firefox/Safari. Endpoints not yet in the Rust API answer 501 in the browser (for example the legacy campuses routes the UI calls).
- forge-wasm:
  - Worker entry that owns the OPFS Connection.
  - `dispatch(method, url, headers, body) -> {status, headers, body}` into the forge-api Router.
  - A 501 fallback for anything not in Tier 0.
  - Call `navigator.storage.persist()`.
  - Retire the wa-sqlite proof of concept (`database.rs`, `wa_sqlite.rs`). Move `alignment/store.rs` onto the shared rusqlite path.
- TS:
  - `ForgeClient` gets an optional `fetch` option (`packages/sdk/src/client.ts`). Add `createWasmFetch()`, which posts to the Worker and returns a `Response`.
  - Add a web UI mode flag `VITE_FORGE_MODE=api|wasm` (default `api` until M7), set in `packages/webui/src/lib/sdk.ts`.
  - Convert the 18 raw `fetch` calls (`SourcesView.svelte`, `KanbanBoard.svelte`, `routes/data/organizations/+page.svelte`, `RoleChoropleth.svelte`) into SDK calls.
- **Database import/export** (Rust, rusqlite backup/serialize API): upload an existing `forge.db` into OPFS, and download the current DB. Add `/api/export/dump` in wasm mode.
- Tests: headless `wasm-pack test` (forge-901c) for dispatch and migrations, plus a CI job.
- **Done when:** the web UI in wasm mode runs the M3 loop (minus PDF) with the server stopped, and data survives a reload.

## M5: Typst rendering (works in both hosts)
- Add `render_typst(&ResumeDocument) -> String` next to `render_markdown` and `render_latex`
  (`crates/forge-sdk/src/services/compiler_service.rs`). Port the sb2nov layout (`packages/core/src/templates/sb2nov.ts`) to a Typst template.
- New crate `forge-typst`: `typst` + `typst-pdf` with embedded fonts. It ships as a separate, lazily loaded wasm module so it doesn't count against the data layer's 5 MB budget.
- Wire `POST /resumes/:id/pdf` and `export ?format=pdf` to it. Natively the same crate replaces tectonic, which removes the subprocess. Return 422 with a `details` field on compile errors.
- Known gap: `latex_override` can't compile in the browser. Show a UI notice and fall back to generated Typst. A `typst_override` column is follow-up work.
- **Done when:** the PDF preview renders in wasm mode, and a golden-file test passes on the generated Typst.
- **Status:** done in PR for `feat/m5-typst-pdf`; see ADR 0003 (`docs/src/dev/adrs/rust-wasm/0003-typst-pdf-in-both-hosts.md`). Checked in Chrome only. Rather than a golden file, the generated source is covered by string-level assertions plus compile-and-extract-text tests.

## M6: Agent bridge (Claude in Chrome)
- Expose `window.forge`, the same `ForgeClient` on the wasm transport, so Claude Code calls
  `await forge.derivations.prepare({...})` and `await forge.derivations.commit(id, {...})` through `javascript_tool`.
  The split-handshake protocol is unchanged.
- After each mutating request, the transport dispatches a `forge:changed` DOM event. Tier-0 list pages listen for it and refetch, so Claude's writes appear live.
- Add a project skill, `.claude/skills/forge-in-chrome/SKILL.md`. It covers:
  - finding the Forge tab
  - the prepare → generate → commit flow, mirroring the MCP tools in `packages/mcp/src/tools/derive.ts`
  - that approvals stay with the human in the UI
  - keeping JSON results small
- Replace the web UI's "use MCP tools" toasts (`SourcesView.svelte:555`, `DerivePerspectivesDialog.svelte:60`) with instructions for Claude in Chrome.
- **Caveat to state in the doc:** this needs Chrome plus the extension with site permission for the app origin. Browser
  storage is per browser, so Forge's data lives in Chrome, not Zen. The M4 export/import moves it between browsers.
- **Done when:** in a live session, Claude derives bullets from a source and then a perspective, and both appear in the UI awaiting approval.
- **Status:** done in the PR for `feat/m6-agent-bridge`. `window.forge` is always present in wasm mode; the runtime fires `forge:changed` after every successful write; the sources and bullets/perspectives lists refetch on it; skill at `.claude/skills/forge-in-chrome/SKILL.md`. Checked by driving a Chrome tab through `javascript_tool` (source, then bullets, then an approved bullet, then a perspective, all `in_review`, bullets visible without a reload). Not done: other list pages (organizations, skills, resumes, and so on) do not listen for `forge:changed` yet; the model-writing step was me following the prompt by hand, not an unattended run.

## M7: Minimal-Rust acceptance and switching the default to wasm
- Add a Playwright core-loop spec in `packages/webui/e2e` for wasm mode: profile → org → source → (seeded) bullet →
  approve → perspective → approve → resume from template → PDF. Run it in CI.
- Dogfood: import the real `data/forge.db`, build one real resume end to end, and export the PDF.
- Switch `VITE_FORGE_MODE` to default to `wasm`. Keep `api` mode working, and **don't delete the TS server.**
- Update `docs/src/migrations/mvp-2.0-browser-first.md` and `docs/src/architecture/models/runtime.md` with what actually shipped.

## Ordering
- **Main path:** M0 → M1 (gate) → M2 → M3 → M4 → M6 → M7.
- **Parallel:** M5 can start after M2 (`render_typst` is a pure function) and run alongside M3 and M4.

## Explicitly after the minimal version
Each has an existing bead:
- CF Pages deploy (forge-4a01)
- CRDT/sync (forge-8rzs)
- extraction pipeline (forge-jsxn)
- alignment/embeddings (forge-a36u)
- Tier-1/2 endpoints
- Rust MCP (R3), Tauri (R2), Dioxus (R4)
- multi-tenant auth (forge-j9by)

## Bead mapping
| Milestone | Existing beads it absorbs or re-scopes |
|---|---|
| M0 | forge-6t8v, forge-7e4f |
| M1 | forge-n89p, forge-73hi (superseded); forge-lu5s and forge-txdt (fallback) |
| M2 | forge-2qns, forge-es6o |
| M4 | forge-5x2h, forge-ae8y, forge-901c |
| M5 | forge-12nc |
