# BrowserStore Adapter — Vertical-Slice Design (forge-lu5s)

**Status:** approved (scope) · **Date:** 2026-04-28 · **Bead:** forge-lu5s · **Predecessor:** forge-nst6

## 1. Context

forge-nst6 shipped a working wa-sqlite browser-persistence binding: `crates/forge-wasm/` exposes a `Database` struct (`exec`, `query`, `close`) backed by `IDBBatchAtomicVFS`. The PoC proved end-to-end round-trip + cross-reload IDB persistence.

forge-lu5s is the next critical-path bead: a `WaSqliteAdapter` (a.k.a. `BrowserStore`) that takes that PoC binding and surfaces the first repository operations against it from a Rust-WASM consumer. It is the foundation that every subsequent skill-graph / extraction / alignment piece (forge-afyg, forge-jsxn, forge-62kb) calls into.

## 2. Goal

Prove BrowserStore can back a real Forge repository operation end-to-end in the browser, by:

1. Expanding the wa-sqlite `Database` API from PoC-minimal (string-only `exec`/`query`) to the smallest typed surface a real store needs.
2. Porting the existing migration runner so all 51 SQL migrations apply against wa-sqlite from WASM.
3. Implementing **one** store (`SkillStore`) end-to-end as the proof-of-life consumer, with parity tests against the rusqlite implementation.

This is a **vertical slice**, not a horizontal trait abstraction. Subsequent stores fold in via successor beads following the template established here.

## 3. Non-Goals (explicit)

- **No shared trait extraction across rusqlite stores.** The bead's risk callout is binding: "Do NOT refactor the rusqlite stores around a new shared trait abstraction during this task." BrowserStore is purely additive.
- **No port of the other 22 stores.** Each gets its own successor bead.
- **No sqlite-vec extension loading.** Vector ops belong to forge-afyg's HNSW work; loading wa-sqlite extensions is a non-trivial separate problem. Defer.
- **No CRDT / sync.** forge-8rzs.
- **No OPFS migration.** forge-n89p (separate trigger).
- **No wasm-bindgen UI surface.** forge-5x2h.

## 4. Architecture

### 4.1 Crate layout & boundaries

```
forge-core (existing)
└── migrations/                ← NEW: lift MIGRATIONS slice up from forge-sdk
    └── pub const MIGRATIONS: &[(&str, &str)]   (pure data, include_str! paths)

forge-sdk (existing, native, rusqlite)
└── db/migrate.rs               ← REFACTOR: import MIGRATIONS from forge-core,
                                   keep the rusqlite::Connection runner here.

forge-wasm (existing, wasm32)
├── database.rs                 ← EXPAND: parameterized binding + typed-row API
├── migrate.rs                  ← NEW: wa-sqlite migration runner
├── adapter.rs                  ← NEW: WaSqliteAdapter (the BrowserStore)
└── stores/
    └── skill.rs                ← NEW: SkillStore impl over Database API
```

**Why lift `MIGRATIONS` to forge-core.** forge-sdk hard-depends on rusqlite (`forge-core = { features = ["rusqlite"] }` plus a top-level rusqlite dep). forge-wasm is `default-features = false` and cannot import forge-sdk without breaking wasm32 builds. The existing `MIGRATIONS: &[(&str, &str)]` in `forge-sdk/src/db/migrate.rs` is pure `include_str!` data with no rusqlite involvement, so it lifts cleanly. forge-sdk's runner becomes a thin wrapper that imports the slice and applies it through rusqlite. forge-wasm gets its own runner that applies the same slice through wa-sqlite. Single source of truth, zero duplication.

**Risk-callout compliance.** No store under `crates/forge-sdk/src/db/stores/` is touched. forge-server and other native consumers are unaffected. The only shared mutation is the migrations slice relocation, which is mechanical.

### 4.2 Database API expansion (in `crates/forge-wasm/src/database.rs`)

The forge-nst6 PoC API is insufficient because every real store does parameterized SQL like `INSERT INTO skills (id, name, category) VALUES (?1, ?2, ?3)`. String concatenation is a non-starter (injection, type round-tripping). Minimal expansion:

| New surface | Purpose |
|---|---|
| `Database::prepare(sql) -> Statement` | wa-sqlite `prepare_v2` wrapper; returns a Statement handle. |
| `Statement::bind_text(idx, &str)` etc. | `bind_text`, `bind_int64`, `bind_double`, `bind_blob`, `bind_null`. |
| `Statement::step() -> StepResult` | `Row \| Done`. |
| `Statement::column_text(idx) -> Option<String>` etc. | `column_text`, `column_int64`, `column_double`, `column_blob`. NULL → `None`. |
| `Statement::reset()` / `Drop` | Reset for re-bind; finalize on drop. |
| `Database::transaction() -> Transaction<'_>` | `BEGIN`/`COMMIT`-on-`Drop`-Ok / `ROLLBACK`-on-`Drop`-Err RAII guard. |
| `Database::exec_batch(sql)` (rename of current `exec`) | Multi-statement script, no parameters. Used by migrations. |

**JS surface stays the same.** All new methods are Rust-only (`impl Database` without `#[wasm_bindgen]`). The `Database` JS handle keeps `exec`/`query`/`close` as the harness already uses them. forge-5x2h can decide later which to expose to JS.

**Async everywhere.** wa-sqlite's `bind_*` / `column_*` / `step` are async on the api object. Statement methods are `async fn`. Transaction guard returns `async fn commit()` / `async fn rollback()` rather than relying on `Drop` (since async-drop is unstable). The "RAII-ish" pattern: explicit `commit()` consumes self; failure to commit-or-rollback before drop logs and best-effort rolls back via a stored `Closure`.

### 4.3 Migration runner (in `crates/forge-wasm/src/migrate.rs`)

Mirrors `forge-sdk/src/db/migrate.rs` semantics:

```rust
pub async fn run_migrations(db: &Database) -> Result<usize, ForgeError> { ... }
```

- Imports `forge_core::migrations::MIGRATIONS`.
- Reads applied state from `_migrations` (`SELECT name FROM _migrations`, after table-existence check).
- For each unapplied migration:
  - If SQL contains `PRAGMA foreign_keys = OFF`, exec that pragma outside any transaction (matches existing rusqlite runner — SQLite ignores pragma inside `BEGIN`).
  - Open a transaction guard.
  - `db.exec_batch(sql)` — wa-sqlite's `exec` accepts multi-statement scripts, so all statements in a migration apply in one call.
  - On success, INSERT into `_migrations`, commit.
  - On error, rollback (best-effort), restore `PRAGMA foreign_keys = ON` if needed, propagate.
- Returns count of newly-applied migrations.

**Test approach.** A wasm-bindgen-test that opens a fresh `Database`, runs migrations, asserts `_migrations` row count == 51, asserts a sentinel table (`sources`) exists. Plus an idempotency test: second run applies 0. (Browser test infrastructure is forge-901c — until that lands, validation is via the manual browser-smoke harness extended with a "run migrations" button.)

### 4.4 WaSqliteAdapter (in `crates/forge-wasm/src/adapter.rs`)

Thin holder/façade. Owns the `Database`, exposes `&Database` to stores. No trait. Direct struct.

```rust
pub struct WaSqliteAdapter {
    db: Database,
}

impl WaSqliteAdapter {
    pub async fn open(filename: &str) -> Result<Self, ForgeError> {
        let db = Database::open(filename).await?;
        crate::migrate::run_migrations(&db).await?;
        Ok(Self { db })
    }

    pub fn db(&self) -> &Database { &self.db }
}
```

That's it for this bead. No trait shape. Successor beads grow it as they need.

### 4.5 SkillStore (in `crates/forge-wasm/src/stores/skill.rs`)

Re-implements a minimal subset of `forge-sdk::db::stores::skill::SkillStore` against `&Database`:

| Method | Purpose | Status |
|---|---|---|
| `create(db, name, category) -> Skill` | INSERT one row, return populated `Skill`. | **In scope** |
| `get(db, id) -> Option<Skill>` | SELECT by id. | **In scope** |
| `list(db, category, search, domain_id) -> Vec<Skill>` | filtered SELECT, ORDER BY name. | **In scope** (without domain_id filter — that's a junction-table query and requires the domains plumbing) |
| `update(db, id, ...)` | UPDATE single row. | **In scope** |
| `delete(db, id)` | DELETE. | **In scope** |
| `get_with_domains` / `link_domain` / `unlink_domain` | junction queries | **Out of scope** (next bead — covers domain_id filter at the same time) |
| `list_categories` | reads `skill_categories` | **In scope** (one-table SELECT, used by webui) |

Matches the rusqlite `SkillStore` shape exactly: free-functions on a unit struct, taking `&Database` instead of `&Connection`. Same signatures, same `ForgeError` boundaries. Same case-insensitive name matching, same UNIQUE-violation → `ForgeError::Conflict` mapping.

### 4.6 Parity tests

A new module `crates/forge-wasm/tests/skill_store_parity.rs` (gated `#[cfg(target_arch = "wasm32")]` for wasm-bindgen-test, plus a native-only counterpart against rusqlite for comparison).

Strategy: for each in-scope SkillStore method, run the rusqlite version against an in-memory connection and the wa-sqlite version against a fresh IDB-backed database, assert identical observable outputs (return values, error variants for conflict cases). Fixtures via shared test data in `forge-core::tests` or a new `forge-test-fixtures` if cross-crate sharing is needed.

**Scope concession:** until forge-901c lands the wasm-bindgen-test harness, the wasm side runs through the manual browser-smoke harness with assertions printed to console. Acceptance is "the manual run matches expectations as logged in the bead's commit message." Automation is forge-901c's deliverable.

## 5. Workspace hygiene fold-in (forge-es6o)

`crates/forge-sdk/src/db/stores/mod.rs` declares `pub mod template;` but the file is untracked locally and missing from main HEAD. Fresh worktrees (including this branch) cannot compile forge-sdk → cannot run `cargo check --workspace` → cannot validate that the migrations-slice relocation didn't regress the rusqlite runner.

**Resolution in this branch:** comment out the `pub mod template;` line and the `pub use template::TemplateStore;` re-export. One-line change, fully reversible. The maintainer's local `template.rs` is unaffected; when they're ready to commit it, they uncomment. forge-es6o gets closed by this fix; forge-2qns and forge-6t8v stay deferred (unrelated to BrowserStore work).

## 6. Build & validation gates

1. `cargo check --workspace` from main worktree (the forge-nst6 lesson — feature-unification fallout only shows up here).
2. `cargo test -p forge-core` (covers the lifted migrations slice — should be a no-op since it's just data; add a smoke test that verifies `MIGRATIONS.len() >= 51` to lock the count).
3. `cargo test -p forge-sdk` (must pass unchanged — the existing 290+ tests already cover the rusqlite runner).
4. Native `forge-wasm` tests: `cargo test -p forge-wasm` (existing 4 tests + new SkillStore native-side parity reference).
5. WASM build: `RUSTC=$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin/rustc $HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin/cargo build -p forge-wasm --target wasm32-unknown-unknown`.
6. Manual browser-smoke harness (extended): open IDB DB, run migrations, exercise SkillStore::{create, get, list, update, delete}, verify cross-reload persistence, log results.

## 7. Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Migrations contain SQLite features wa-sqlite doesn't support (e.g., specific PRAGMAs, FTS5 if used). | Medium | High — runner fails on first encounter, blocks everything downstream. | Read every migration ahead of porting to spot suspect statements. wa-sqlite ships standard SQLite + sqlite-vec optional; vanilla SQL should pass. If any migration uses FTS or json1-only-on-native quirks, isolate and document. |
| `db.exec_batch(sql)` doesn't handle multi-statement scripts the way rusqlite's `execute_batch` does. | Low | High | Verify by running a known multi-statement migration (e.g., 052) early. Fall back to splitting on `;` outside string-literals if needed. |
| Async transaction guard semantics get subtle (no async drop). | Medium | Medium | Document the explicit `commit()`/`rollback()` discipline in adapter.rs; provide a `with_transaction(async \|tx\|)` helper that handles both paths so callers don't need to reason about it. |
| The `MIGRATIONS` relocation breaks forge-sdk include_str! paths. | Low | Medium | The new path from forge-core is `../../packages/core/src/db/migrations/*.sql`; verify with `cargo check -p forge-sdk` immediately after the move. forge-sdk's `migrate.rs` then just `use forge_core::migrations::MIGRATIONS` — no path strings on its side. |
| forge-es6o fix-by-comment-out conflicts with the maintainer's local template.rs work when they next commit. | Low | Low | Single-line revert when they're ready. Note in the bead comments. |
| sqlite-vec absence breaks any in-scope migration. | Low | High | None of migrations 001–053 require sqlite-vec — embeddings (025) declares a BLOB column but doesn't depend on the extension. Verify by grep before porting. |
| Async `bind_*` / `column_*` round-trips per cell tank performance for large queries. | Medium | Low (for this bead's scope; relevant for forge-afyg) | Defer optimization. SkillStore queries return small row counts. Measure before optimizing. |
| Browser-smoke manual validation is slow and easy to skip. | High | Medium | Keep the harness open during dev, log SkillStore output on every page load. forge-901c automates. |

## 8. Open Questions

1. **Where does the SkillStore-relevant subset stop?** This spec puts the `domain_id` filter and junction queries in a successor bead. If reviewing this design surfaces a need for domains in MVP-2.0 sooner rather than later, scope can stretch.
2. **Should the lifted `MIGRATIONS` slice live at `forge_core::migrations::MIGRATIONS` or in a new `forge-migrations` crate?** Default to the former (a module in forge-core) — one constant doesn't justify a crate. Open to revisiting if other shared-data items emerge.
3. **wasm-bindgen-test harness timing.** This spec assumes manual validation until forge-901c. If forge-901c lands first, the parity tests upgrade to automated. If forge-lu5s ships first, we accept the manual gate.
4. **Do we want a per-store benchmark gate?** Out of scope. SkillStore correctness is what this bead proves; performance is a forge-afyg concern (HNSW is the bottleneck, not row-by-row CRUD).

## 9. Revision History

| Date | Author | Change |
|---|---|---|
| 2026-04-28 | aRustyDev (Claude) | Initial draft. Scope confirmed (Option A — vertical slice). |
