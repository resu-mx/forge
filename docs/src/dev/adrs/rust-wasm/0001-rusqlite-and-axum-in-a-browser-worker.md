# 0001. Run rusqlite and an axum Router in a browser Worker (roadmap M1 outcome)

- **Status**: Accepted (Chrome verified; Firefox and Safari not yet verified, see Open items)
- **Date**: 2026-10-01
- **Roadmap**: `.agents/plans/rust-mvp-roadmap.md`, milestone M1
- **Spike**: `spikes/m1-wasm/` (throwaway; delete once M4 lands)

## Context

The roadmap bets that the existing `forge-sdk` (rusqlite) and an axum `Router` can run unchanged
inside a browser Worker, so the browser-first app reuses the native code instead of re-implementing
22 stores over wa-sqlite. M1 was the decision gate for that bet. The alternative, Path B, is merging
`origin/forge-lu5s` and abstracting storage behind an async trait.

## Decision

**Take Path A.** Browser storage is rusqlite on `sqlite-wasm-rs` with the `sqlite-wasm-vfs` OPFS
`sahpool` VFS, inside a dedicated Worker. The API is an axum `Router` built with
`default-features = false` and driven through `tower::ServiceExt::oneshot`.

This supersedes the April decisions to use wa-sqlite from npm (forge-nst6 A) and to run on the main
thread without a Worker (forge-nst6 B), and the `query(sql)` API in forge-5x2h.

## Evidence

All measured on 2026-10-01, macOS arm64, Chrome 156, rustc stable 1.99.0.

| Gate | Result |
|---|---|
| 1. rusqlite 0.32 → 0.40.2 | Workspace compiled with zero errors. The same 457 native tests pass |
| 2. `forge-sdk` on wasm32 | Builds and runs. `uuid` (`js` feature) and `chrono` needed no changes |
| 3. OPFS sahpool in a Worker | Unmodified `Forge::open("forge.db")` applied all **52 migrations in 156 ms**. After a page reload the row was still there and open took **9 ms** with no migrations re-run. The first, empty-database run is the control showing the check can tell fresh from persisted |
| 4. axum `Router`, no tokio | `GET /api/health`, `GET /api/sources` and `POST /api/sources` (a real `SourceStore::create` with the JSON contract) all answered through `oneshot` in 1-2 ms |
| 5. Bundle size | **1.49 MB raw, 574 KB gzipped** (release, `opt-level = "z"`, LTO). It includes SQLite 3.53.0, the stores, axum and all migrations. Budget was 5 MB |

## Consequences and constraints found

- **Build needs a clang with the WebAssembly backend.** `sqlite-wasm-rs` compiles SQLite from C. Apple clang has no wasm backend, so
  the build fails there; Homebrew LLVM works (`CC_wasm32_unknown_unknown`, `AR_wasm32_unknown_unknown`). CI's Linux clang is
  expected to work but has not been tested.
- **Version coupling is fragile.** rusqlite 0.40 pulls `sqlite-wasm-rs` 0.5.x. `sqlite-wasm-vfs` **0.2.x** matches it. `sqlite-wasm-vfs` 0.3.x targets
  `sqlite-wasm-rs` 0.6 and has an incompatible `OsCallback` trait. Pin both and upgrade them together.
- **No WAL.** The sahpool VFS ignores `PRAGMA journal_mode = WAL`; the database runs in `delete` mode. `Forge::init` sets WAL
  and this is silently accepted. This is fine for a single-writer browser database.
- **Worker only.** OPFS `createSyncAccessHandle()` is unavailable on the main thread, so every database call crosses `postMessage`.
- **Persistence is not yet protected.** `navigator.storage.persisted()` was `false`, so the browser may evict the data under storage pressure. M4 must call
  `navigator.storage.persist()` and ship export/import.
- **Single connection per directory per Worker.** The VFS has one owner per OPFS directory, so there can be only one tab or Worker writing.
  Multi-tab use needs a design (a `SharedWorker`, or a Web Locks guard) before M4.
- **Path B stays as the fallback**, and `origin/forge-lu5s` should not be deleted until M4 is done.

## Open items (not verified)

- **Firefox and Safari.** Only Chrome was driven. OPFS sync access handles exist in Firefox Workers, but sahpool was not run there.
  Please run `just --justfile spikes/m1-wasm/justfile build serve` and open `http://localhost:8137/` in Zen/Firefox.
- **CI build.** Whether `ubuntu-latest` clang can build `sqlite-wasm-rs` for wasm32 (needed to add a spike or `forge-api` wasm job).
- **Real route surface.** The spike routes are tiny. The full router and `with_conn` (no `spawn_blocking` on wasm) are M2 work.
