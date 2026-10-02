# 0002. Browser runtime: a dedicated Worker, one owner tab, fetch as the transport (roadmap M4)

- **Status**: Accepted (Chrome verified; Firefox and Safari not yet verified)
- **Date**: 2026-10-02
- **Builds on**: [0001](./0001-rusqlite-and-axum-in-a-browser-worker.md)
- **Roadmap**: `.agents/plans/rust-mvp-roadmap.md`, milestone M4

## Context

ADR 0001 showed that `forge-sdk` and the `forge-api` router run in a browser Worker over
OPFS. M4 turns that into the runtime the web UI actually uses, which raises three questions
the spike left open: who may own the database, how the UI reaches the router, and how data
gets in and out of a browser.

## Decisions

### 1. A dedicated Worker owns the database, one tab at a time

**A SharedWorker cannot own it.** Verified in Chrome 156: inside a SharedWorker,
`FileSystemSyncAccessHandle` and `createSyncAccessHandle` are both `undefined`, and the
OPFS VFS never finishes initialising. One SharedWorker shared by every tab, the textbook
answer to multi-tab, is therefore not available.

Instead a **Web Lock** (`forge-database-owner`) elects one owner tab, which spawns the
dedicated Worker. A second tab shows "Forge is open in another tab", answers API calls
with `503 STORAGE_BUSY`, and **takes over automatically** when the owner closes (the
browser releases the lock when a tab closes or crashes). Verified in Chrome: the second
tab waited, and after the owner closed, a third tab found the lock held and queued behind it.

Rejected for now: the owner tab also serving other tabs over BroadcastChannel. It would give
real multi-tab use, but needs leader election, failover and request routing. Revisit if
single-tab use proves too limiting.

### 2. The transport is `fetch`

The web UI keeps its code. `@forge/runtime` provides a `fetch` that sends the request to the
Worker, which hands it to the router (`forge-wasm::dispatch`) and returns a real `Response`.
Two hooks use it:

- the SDK takes an optional `fetch` (`ForgeClientOptions.fetch`);
- `installApiFetch` wraps the global `fetch` for same-origin `/api/` URLs, which covers the
  UI's 17 raw `fetch('/api/…')` calls without rewriting them.

(The roadmap planned to convert those raw calls to SDK calls; the wrapper is less invasive
and also covers future callers.) Endpoints the Rust API does not serve yet answer **501**,
not 404, in the browser, so the UI can say "not available yet".

### 3. Data moves as a SQLite file

`exportDatabase` returns the database file; `importDatabase` replaces it. The OPFS pool will
not import over an existing file, so import validates the header, keeps the current bytes in
memory, deletes, imports, and **restores the old data if anything fails**. Verified: a
non-database is rejected up front; a truncated file that passes the header check fails
*inside* the import, after deletion, and the previous data came back. Migrations run on the
imported data. A TS `forge.db` must be checkpointed first (`PRAGMA wal_checkpoint(TRUNCATE)`)
because the browser VFS has no WAL.

## Measurements

Chrome 156, rustc 1.99.0, 2026-10-02.

| | |
|---|---|
| Runtime (`wasm-release`, names stripped) | **2.88 MB raw, 0.94 MB gzipped** (budget 5 MB). Without `--remove-name-section` it was 6.0 MB |
| Core loop in the browser | profile, organization, typed `role` source, derivation prepare and commit, approval, perspective, resume from a 6-section template, IR: all pass with no server |
| Persistence | data survived a reload; export is a valid SQLite file |
| Production build | a 12 KB Worker chunk plus the wasm as a separate asset |

## Consequences

- **Persistence is not granted by default.** Chrome did not grant `navigator.storage.persist()`
  here, so the UI shows a notice pointing at Settings → Storage and the export button.
- **Build needs a wasm-capable clang** (ADR 0001) and `wasm-bindgen-cli` pinned to the version in
  `Cargo.lock` (0.2.129). `just wasm-bundle` builds the runtime; the UI build in `api` mode needs neither.
- **Not served yet** (501 in the browser): the legacy `/organizations/:id/campuses` routes the UI
  calls directly, alignment, extension and the other Tier-1/2 endpoints from the M3 parity run;
  PDF is M5.
- **Firefox and Safari are unverified.** `createSyncAccessHandle` is supported in Firefox dedicated
  Workers, but the runtime has only been run in Chrome.
- The wa-sqlite proof of concept and its `peerDependency` are removed.
