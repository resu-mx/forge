# BrowserStore Adapter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a vertical-slice `WaSqliteAdapter` (a.k.a. BrowserStore) that runs all 51 Forge SQL migrations against wa-sqlite from WASM and supports a minimal `SkillStore` end-to-end with parity tests.

**Architecture:** Lift the existing `MIGRATIONS` slice from `forge-sdk` into `forge-core` so both runners share one source. Extend the forge-nst6 wa-sqlite `Database` API with parameterized binding, typed-row extraction, and a transaction guard. Layer a migration runner, a thin `WaSqliteAdapter`, and a single `SkillStore` impl on top — purely additive, no refactor of rusqlite stores.

**Tech Stack:** Rust 2021, wasm-bindgen 0.2, wa-sqlite 1.0.0 (npm, Asyncify build, `IDBBatchAtomicVFS`), rustup-stable wasm32 toolchain, rusqlite (native side, unchanged).

**Spec:** `.claude/plans/forge-resume-builder/refs/specs/2026-04-28-browserstore-adapter-vertical-slice.md`

**Bead:** forge-lu5s (parent: forge-6z5l)

**Worktree:** `.claude/worktrees/forge-wasm/` (created in Task 1)

---

## Pre-flight reading (engineer)

Before starting, skim:

1. The spec (link above). Sections 4.1, 4.2, 4.3, 4.5 are load-bearing.
2. `crates/forge-sdk/src/db/migrate.rs` — current migration runner (rusqlite). The new slice must produce identical results when applied through rusqlite.
3. `crates/forge-wasm/src/database.rs` — forge-nst6 PoC Database. The new methods in this plan extend this file.
4. `crates/forge-wasm/src/wa_sqlite.rs` — extern bindings. New extern entries get added here.
5. `crates/forge-wasm/examples/browser-smoke/` — the manual harness used to validate browser behavior. Until forge-901c lands, this is the only way to exercise wa-sqlite-bound code.
6. `crates/forge-sdk/src/db/stores/skill.rs` — the rusqlite SkillStore we're mirroring.

## Toolchain notes

Homebrew rustc lacks the wasm32 sysroot. WASM builds use rustup-stable explicitly:

```bash
RUSTC=$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin/rustc \
  $HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin/cargo \
  build -p forge-wasm --target wasm32-unknown-unknown
```

Or use `wasm-pack`:

```bash
PATH=$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH \
  wasm-pack build --target bundler crates/forge-wasm
```

`just wasm-build` is broken until forge-6t8v is fixed; bypass with the explicit invocations above.

---

## Task 1: Worktree + bead claim + forge-es6o workaround

**Goal:** Get a fresh worktree on a `forge-lu5s` branch where `cargo check --workspace` passes.

**Files:**
- Modify: `crates/forge-sdk/src/db/stores/mod.rs:25,49` (comment out template module + re-export)

- [ ] **Step 1: Create the worktree from main**

Run from the main worktree (`/Users/adam/code/proj/forge`):

```bash
git fetch origin
git worktree add .claude/worktrees/forge-wasm -b forge-lu5s origin/main
cd .claude/worktrees/forge-wasm
```

Expected: new worktree at `.claude/worktrees/forge-wasm/`, on branch `forge-lu5s` tracking `origin/main`.

- [ ] **Step 2: Claim the bead**

```bash
bd update forge-lu5s --claim
bd show forge-lu5s
```

Expected: status flips to in_progress, owner aRustyDev.

- [ ] **Step 3: Apply the forge-es6o workaround**

Edit `crates/forge-sdk/src/db/stores/mod.rs`. Comment out the two `template` lines:

```rust
// pub mod template;  // forge-es6o: file untracked on main, blocks fresh checkouts. Re-enable when committed.
```

```rust
// pub use template::TemplateStore;  // forge-es6o
```

- [ ] **Step 4: Verify the workspace builds from the worktree**

```bash
cargo check --workspace
```

Expected: clean build. No errors. (If forge-2qns or forge-6t8v also block, stop and escalate — those are out of scope here. Most likely only es6o blocked, since 2qns is forge-server's lib.rs and 6t8v is the justfile mod.)

- [ ] **Step 5: Run baseline tests**

```bash
cargo test -p forge-core
cargo test -p forge-sdk
cargo test -p forge-wasm
```

Expected: all green. Record counts (forge-core 35, forge-sdk 290+, forge-wasm 4) so later regressions are obvious.

- [ ] **Step 6: Commit the workaround**

```bash
git add crates/forge-sdk/src/db/stores/mod.rs
git commit -m "fix(sdk): comment out untracked template module (forge-es6o)

Allows cargo check --workspace to succeed from fresh worktrees. The
template.rs file exists locally on the maintainer's tree but is not on
main HEAD — re-enable when the file is committed.

Closes forge-es6o."
```

---

## Task 2: Lift MIGRATIONS slice to forge-core

**Goal:** Single source of truth for the 51-entry SQL migrations slice, importable from both forge-sdk (rusqlite) and forge-wasm (wa-sqlite).

**Files:**
- Create: `crates/forge-core/src/migrations.rs`
- Modify: `crates/forge-core/src/lib.rs` (add `pub mod migrations;`)
- Modify: `crates/forge-sdk/src/db/migrate.rs` (delete local slice, import from forge-core)

- [ ] **Step 1: Write the failing test for the new forge-core module**

Create `crates/forge-core/src/migrations.rs` with **only** the test (no slice yet — the test should fail to compile because `MIGRATIONS` doesn't exist):

```rust
//! SQL migration manifest — single source of truth for the Forge schema.
//!
//! The slice contents are pure data (`include_str!` against
//! `packages/core/src/db/migrations/*.sql`), with no DB-runtime dependency.
//! Native consumers (forge-sdk) apply via rusqlite; WASM consumers
//! (forge-wasm) apply via wa-sqlite.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_slice_is_populated_and_named() {
        assert!(MIGRATIONS.len() >= 51, "expected at least 51 migrations, got {}", MIGRATIONS.len());

        // Every entry must have a non-empty name and non-empty SQL body.
        for (name, sql) in MIGRATIONS {
            assert!(!name.is_empty(), "migration with empty name");
            assert!(!sql.trim().is_empty(), "migration {} has empty SQL", name);
        }

        // First migration must be the canonical bootstrap.
        let (first_name, _) = MIGRATIONS.first().expect("MIGRATIONS must be non-empty");
        assert_eq!(*first_name, "001_initial");
    }

    #[test]
    fn migrations_are_filename_sorted() {
        let names: Vec<&str> = MIGRATIONS.iter().map(|(n, _)| *n).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted, "MIGRATIONS slice must be in filename-sorted order");
    }
}
```

- [ ] **Step 2: Add the module declaration to lib.rs and confirm test failure**

Edit `crates/forge-core/src/lib.rs`:

```rust
//! Core types, enums, and shapes for Forge.
//!
//! This crate defines the shared data model used across all Forge crates.
//! It contains no business logic and performs no I/O — only type definitions,
//! enums, and serialization derives.
//!
//! TS source: `packages/core/src/types/index.ts`

pub mod migrations;
pub mod types;
pub mod util;

pub use types::*;
pub use util::{new_id, now_iso};
```

Run the test:

```bash
cargo test -p forge-core migrations
```

Expected: FAIL with "cannot find value `MIGRATIONS` in this scope".

- [ ] **Step 3: Add the slice (copy from forge-sdk, fix include_str! paths)**

The slice in `crates/forge-sdk/src/db/migrate.rs` uses paths like `../../../../packages/core/src/db/migrations/001_initial.sql` (4 levels up from `crates/forge-sdk/src/db/migrate.rs` to repo root). From `crates/forge-core/src/migrations.rs` the path is **3 levels up**: `../../../packages/core/src/db/migrations/001_initial.sql`.

Append to `crates/forge-core/src/migrations.rs`:

```rust
/// Embedded SQL migrations in filename-sorted order.
/// Source: `packages/core/src/db/migrations/*.sql`.
pub const MIGRATIONS: &[(&str, &str)] = &[
    ("001_initial", include_str!("../../../packages/core/src/db/migrations/001_initial.sql")),
    ("002_schema_evolution", include_str!("../../../packages/core/src/db/migrations/002_schema_evolution.sql")),
    ("003_renderer_and_entities", include_str!("../../../packages/core/src/db/migrations/003_renderer_and_entities.sql")),
    ("004_resume_sections", include_str!("../../../packages/core/src/db/migrations/004_resume_sections.sql")),
    ("005_user_profile", include_str!("../../../packages/core/src/db/migrations/005_user_profile.sql")),
    ("006_summaries", include_str!("../../../packages/core/src/db/migrations/006_summaries.sql")),
    ("007_job_descriptions", include_str!("../../../packages/core/src/db/migrations/007_job_descriptions.sql")),
    ("008_resume_templates", include_str!("../../../packages/core/src/db/migrations/008_resume_templates.sql")),
    ("009_education_subtype_fields", include_str!("../../../packages/core/src/db/migrations/009_education_subtype_fields.sql")),
    ("010_education_org_fk", include_str!("../../../packages/core/src/db/migrations/010_education_org_fk.sql")),
    ("011_org_tags", include_str!("../../../packages/core/src/db/migrations/011_org_tags.sql")),
    ("012_org_kanban_statuses", include_str!("../../../packages/core/src/db/migrations/012_org_kanban_statuses.sql")),
    ("013_org_campuses", include_str!("../../../packages/core/src/db/migrations/013_org_campuses.sql")),
    ("014_campus_zipcode_hq", include_str!("../../../packages/core/src/db/migrations/014_campus_zipcode_hq.sql")),
    ("015_org_aliases", include_str!("../../../packages/core/src/db/migrations/015_org_aliases.sql")),
    ("016_source_skills", include_str!("../../../packages/core/src/db/migrations/016_source_skills.sql")),
    ("018_job_description_skills", include_str!("../../../packages/core/src/db/migrations/018_job_description_skills.sql")),
    ("019_clearance_structured_data", include_str!("../../../packages/core/src/db/migrations/019_clearance_structured_data.sql")),
    ("020_stale_education_text_cleanup", include_str!("../../../packages/core/src/db/migrations/020_stale_education_text_cleanup.sql")),
    ("021_drop_legacy_education_columns", include_str!("../../../packages/core/src/db/migrations/021_drop_legacy_education_columns.sql")),
    ("022_drop_legacy_org_location_columns", include_str!("../../../packages/core/src/db/migrations/022_drop_legacy_org_location_columns.sql")),
    ("023_contacts", include_str!("../../../packages/core/src/db/migrations/023_contacts.sql")),
    ("024_unified_kanban_statuses", include_str!("../../../packages/core/src/db/migrations/024_unified_kanban_statuses.sql")),
    ("025_embeddings", include_str!("../../../packages/core/src/db/migrations/025_embeddings.sql")),
    ("026_job_description_resumes", include_str!("../../../packages/core/src/db/migrations/026_job_description_resumes.sql")),
    ("027_salary_structured_fields", include_str!("../../../packages/core/src/db/migrations/027_salary_structured_fields.sql")),
    ("028_jd_pipeline_statuses", include_str!("../../../packages/core/src/db/migrations/028_jd_pipeline_statuses.sql")),
    ("029_prompt_logs_jd_entity_type", include_str!("../../../packages/core/src/db/migrations/029_prompt_logs_jd_entity_type.sql")),
    ("031_skills_expansion", include_str!("../../../packages/core/src/db/migrations/031_skills_expansion.sql")),
    ("032_industries_role_types", include_str!("../../../packages/core/src/db/migrations/032_industries_role_types.sql")),
    ("033_summary_structured_fields", include_str!("../../../packages/core/src/db/migrations/033_summary_structured_fields.sql")),
    ("034_resume_entry_source_id", include_str!("../../../packages/core/src/db/migrations/034_resume_entry_source_id.sql")),
    ("035_resume_tagline_engine", include_str!("../../../packages/core/src/db/migrations/035_resume_tagline_engine.sql")),
    ("036_null_auto_content_on_direct_source_entries", include_str!("../../../packages/core/src/db/migrations/036_null_auto_content_on_direct_source_entries.sql")),
    ("037_qualifications", include_str!("../../../packages/core/src/db/migrations/037_qualifications.sql")),
    ("038_resume_summary_override", include_str!("../../../packages/core/src/db/migrations/038_resume_summary_override.sql")),
    ("039_presentations", include_str!("../../../packages/core/src/db/migrations/039_presentations.sql")),
    ("040_resume_show_clearance_header", include_str!("../../../packages/core/src/db/migrations/040_resume_show_clearance_header.sql")),
    ("041_skill_categories_and_seeds", include_str!("../../../packages/core/src/db/migrations/041_skill_categories_and_seeds.sql")),
    ("042_project_open_source", include_str!("../../../packages/core/src/db/migrations/042_project_open_source.sql")),
    ("043_cert_schema_rework", include_str!("../../../packages/core/src/db/migrations/043_cert_schema_rework.sql")),
    ("044_skill_categories", include_str!("../../../packages/core/src/db/migrations/044_skill_categories.sql")),
    ("045_pending_derivations", include_str!("../../../packages/core/src/db/migrations/045_pending_derivations.sql")),
    ("046_profile_addresses_urls", include_str!("../../../packages/core/src/db/migrations/046_profile_addresses_urls.sql")),
    ("047_org_locations", include_str!("../../../packages/core/src/db/migrations/047_org_locations.sql")),
    ("048_notes_normalization", include_str!("../../../packages/core/src/db/migrations/048_notes_normalization.sql")),
    ("049_jd_parsed_fields", include_str!("../../../packages/core/src/db/migrations/049_jd_parsed_fields.sql")),
    ("050_answer_bank", include_str!("../../../packages/core/src/db/migrations/050_answer_bank.sql")),
    ("051_extension_infra", include_str!("../../../packages/core/src/db/migrations/051_extension_infra.sql")),
    ("052_skill_graph_schema", include_str!("../../../packages/core/src/db/migrations/052_skill_graph_schema.sql")),
    ("053_skill_graph_initial_population", include_str!("../../../packages/core/src/db/migrations/053_skill_graph_initial_population.sql")),
];
```

- [ ] **Step 4: Confirm forge-core tests pass**

```bash
cargo test -p forge-core migrations
```

Expected: PASS for both `migrations_slice_is_populated_and_named` and `migrations_are_filename_sorted`. Total forge-core count goes from 35 to 37.

- [ ] **Step 5: Refactor forge-sdk's runner to import from forge-core**

Replace the `MIGRATIONS` slice in `crates/forge-sdk/src/db/migrate.rs` with an import. Final state of the file's top:

```rust
//! Migration runner — applies the shared SQL slice from forge-core to a
//! rusqlite Connection, tracking state in `_migrations`.

use rusqlite::Connection;
use forge_core::ForgeError;
use forge_core::migrations::MIGRATIONS;

/// Run all pending migrations against the given connection.
///
/// Each migration runs in its own transaction. Migrations that contain
/// `PRAGMA foreign_keys = OFF` get that pragma set outside the transaction
/// (SQLite ignores it inside BEGIN/COMMIT).
pub fn run_migrations(conn: &Connection) -> Result<usize, ForgeError> {
    // ... unchanged body ...
}
```

The body of `run_migrations` and `get_applied` is **unchanged**. Only the local `const MIGRATIONS: &[(&str, &str)] = &[ ... ]` block is removed.

- [ ] **Step 6: Confirm forge-sdk tests still pass**

```bash
cargo test -p forge-sdk
```

Expected: same number of passing tests as before (290+). The `migrations_run_on_empty_db`, `migrations_are_idempotent`, `sources_table_exists_after_migration`, and the 052/053 skill_graph tests in `migrate.rs` exercise the relocated slice end-to-end.

- [ ] **Step 7: Commit**

```bash
git add crates/forge-core/src/migrations.rs crates/forge-core/src/lib.rs crates/forge-sdk/src/db/migrate.rs
git commit -m "refactor(core,sdk): lift MIGRATIONS slice to forge-core

Both the rusqlite runner (forge-sdk) and the upcoming wa-sqlite runner
(forge-wasm) need to apply identical migrations. Moving the slice up to
forge-core (which is wasm-compatible — default-features = []) makes it
the single source of truth. forge-sdk's runner becomes a thin import.

forge-wasm cannot depend on forge-sdk (rusqlite is a hard dep), so
forge-core is the only crate both sides can share.

Refs forge-lu5s."
```

---

## Task 3: Extend wa-sqlite extern bindings

**Goal:** Bind `prepare_v2`, `bind_*`, `step`, `column_*`, `reset`, `finalize` so the higher-level Statement wrapper has a foundation. Validation is browser-side via the existing harness — no native test possible for FFI bindings.

**Files:**
- Modify: `crates/forge-wasm/src/wa_sqlite.rs` (add extern entries to the `SqliteApi` block)

**Reference:** wa-sqlite's actual JS API lives at `node_modules/wa-sqlite/src/sqlite-api.js` after `bun install` in the browser-smoke harness. If any signature below differs, the engineer should reconcile against that file.

- [ ] **Step 1: Add extern bindings for prepare/step/reset/finalize**

Append to the `extern "C"` block in `crates/forge-wasm/src/wa_sqlite.rs` (after the existing `close` binding, before the closing `}`):

```rust
    /// `prepare_v2(db, sql) -> Promise<{stmt, sql}>` — `stmt` is the
    /// prepared-statement handle (opaque); `sql` is the unconsumed
    /// remainder of the input string. Returns `null`/`undefined` for
    /// `stmt` when the input was whitespace/comments only.
    #[wasm_bindgen(method, catch, js_name = "prepare_v2")]
    pub async fn prepare_v2(
        this: &SqliteApi,
        db: &JsValue,
        sql: &str,
    ) -> Result<JsValue, JsValue>;

    /// `step(stmt) -> Promise<resultCode>` — `SQLITE_ROW` (100) when a
    /// row is available, `SQLITE_DONE` (101) when iteration is complete.
    #[wasm_bindgen(method, catch)]
    pub async fn step(this: &SqliteApi, stmt: &JsValue) -> Result<JsValue, JsValue>;

    /// `reset(stmt) -> Promise<resultCode>` — rewinds the statement so
    /// `bind_*` + `step` can run again with new parameters.
    #[wasm_bindgen(method, catch)]
    pub async fn reset(this: &SqliteApi, stmt: &JsValue) -> Result<JsValue, JsValue>;

    /// `finalize(stmt) -> Promise<resultCode>` — destroys the prepared
    /// statement, releasing its memory. Idempotent in our wrapper.
    #[wasm_bindgen(method, catch)]
    pub async fn finalize(this: &SqliteApi, stmt: &JsValue) -> Result<JsValue, JsValue>;
```

- [ ] **Step 2: Add bind_* extern bindings**

Append after the `finalize` binding:

```rust
    /// `bind_text(stmt, idx, value) -> resultCode`. `idx` is 1-based.
    #[wasm_bindgen(method, catch)]
    pub fn bind_text(this: &SqliteApi, stmt: &JsValue, idx: i32, value: &str) -> Result<JsValue, JsValue>;

    /// `bind_int64(stmt, idx, value) -> resultCode`. wa-sqlite accepts
    /// JS Number; we widen i64 → f64 at the boundary, accepting precision
    /// loss for ids beyond 2^53 (forge UUIDs are strings, so this is moot
    /// in practice).
    #[wasm_bindgen(method, catch, js_name = "bind_int64")]
    pub fn bind_int64(this: &SqliteApi, stmt: &JsValue, idx: i32, value: f64) -> Result<JsValue, JsValue>;

    /// `bind_double(stmt, idx, value) -> resultCode`.
    #[wasm_bindgen(method, catch)]
    pub fn bind_double(this: &SqliteApi, stmt: &JsValue, idx: i32, value: f64) -> Result<JsValue, JsValue>;

    /// `bind_blob(stmt, idx, value) -> resultCode`. wa-sqlite expects a
    /// Uint8Array on the JS side; wasm-bindgen turns `&[u8]` into one.
    #[wasm_bindgen(method, catch)]
    pub fn bind_blob(this: &SqliteApi, stmt: &JsValue, idx: i32, value: &[u8]) -> Result<JsValue, JsValue>;

    /// `bind_null(stmt, idx) -> resultCode`.
    #[wasm_bindgen(method, catch)]
    pub fn bind_null(this: &SqliteApi, stmt: &JsValue, idx: i32) -> Result<JsValue, JsValue>;
```

- [ ] **Step 3: Add column_* extern bindings**

Append after the bind blocks:

```rust
    /// `column_count(stmt) -> i32`.
    #[wasm_bindgen(method, js_name = "column_count")]
    pub fn column_count(this: &SqliteApi, stmt: &JsValue) -> i32;

    /// `column_type(stmt, idx) -> i32`. Returns SQLite type codes:
    /// 1=INTEGER, 2=FLOAT, 3=TEXT, 4=BLOB, 5=NULL.
    #[wasm_bindgen(method, js_name = "column_type")]
    pub fn column_type(this: &SqliteApi, stmt: &JsValue, idx: i32) -> i32;

    /// `column_text(stmt, idx) -> String`. Returns "" for NULL — caller
    /// must check column_type first to disambiguate.
    #[wasm_bindgen(method, js_name = "column_text")]
    pub fn column_text(this: &SqliteApi, stmt: &JsValue, idx: i32) -> String;

    /// `column_int(stmt, idx) -> i32`. wa-sqlite has separate `column_int`
    /// and `column_int64`; for forge's needs (counts, small ints) `int`
    /// suffices. If you need full i64 range, switch to `column_int64`.
    #[wasm_bindgen(method, js_name = "column_int")]
    pub fn column_int(this: &SqliteApi, stmt: &JsValue, idx: i32) -> i32;

    /// `column_double(stmt, idx) -> f64`.
    #[wasm_bindgen(method, js_name = "column_double")]
    pub fn column_double(this: &SqliteApi, stmt: &JsValue, idx: i32) -> f64;

    /// `column_blob(stmt, idx) -> Uint8Array`. wasm-bindgen converts
    /// the JS Uint8Array into `Vec<u8>` on the Rust side.
    #[wasm_bindgen(method, js_name = "column_blob")]
    pub fn column_blob(this: &SqliteApi, stmt: &JsValue, idx: i32) -> Vec<u8>;
```

- [ ] **Step 4: Add SQLITE column-type constants**

Append at the end of `crates/forge-wasm/src/wa_sqlite.rs` (alongside the existing `SQLITE_OK` / `SQLITE_ROW` / `SQLITE_DONE`):

```rust
/// SQLite column type — INTEGER.
pub const SQLITE_INTEGER: i32 = 1;
/// SQLite column type — FLOAT.
pub const SQLITE_FLOAT: i32 = 2;
/// SQLite column type — TEXT.
pub const SQLITE_TEXT: i32 = 3;
/// SQLite column type — BLOB.
pub const SQLITE_BLOB: i32 = 4;
/// SQLite column type — NULL.
pub const SQLITE_NULL: i32 = 5;
```

- [ ] **Step 5: Build to verify the bindings compile**

```bash
RUSTC=$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin/rustc \
  $HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin/cargo \
  build -p forge-wasm --target wasm32-unknown-unknown
```

Expected: clean build. The bindings are declarative — at this point we have not yet exercised them.

- [ ] **Step 6: Native build check**

```bash
cargo check -p forge-wasm
```

Expected: clean. (forge-wasm has both `cdylib` and `rlib` crate types so it must compile natively too, even though the wasm-bindgen extern blocks won't link without a JS host.)

- [ ] **Step 7: Commit**

```bash
git add crates/forge-wasm/src/wa_sqlite.rs
git commit -m "feat(wasm): bind wa-sqlite prepare/bind/step/column/reset/finalize

Foundation for the BrowserStore parameterized-query API. The Statement
wrapper that consumes these (Task 4) makes them usable; the migration
runner (Task 6) and SkillStore (Task 8) are the first consumers.

Refs forge-lu5s."
```

---

## Task 4: Statement wrapper + Transaction guard + exec_batch rename

**Goal:** Build the typed Rust API consumers will actually call. Browser validation comes in Task 9 (the harness extension). Native validation here is limited to `cargo check`.

**Files:**
- Modify: `crates/forge-wasm/src/database.rs` (add Statement, Transaction; keep existing Database; rename `exec` → `exec_batch`; keep `query` for the harness)
- Modify: `crates/forge-wasm/src/lib.rs` (re-export Statement if needed by consumers)

- [ ] **Step 1: Add the Statement type and Database::prepare**

Edit `crates/forge-wasm/src/database.rs`. Add a new section after the existing JS-facing impl block (after `close`):

```rust
// ── Rust-only: Statement wrapper ─────────────────────────────────────────

/// Result of a `Statement::step()` call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepResult {
    /// A row is available; call `column_*` to read it.
    Row,
    /// Iteration complete; subsequent `step` calls return Done.
    Done,
}

/// Prepared-statement handle. Created via `Database::prepare`. Drop
/// finalizes the underlying wa-sqlite statement (best-effort).
pub struct Statement {
    api: SqliteApi,
    stmt: JsValue,
    finalized: bool,
}

impl Database {
    /// Prepare a SQL statement. Returns a single-statement handle — if
    /// `sql` contains multiple semicolon-separated statements, only the
    /// first is prepared and the remainder is silently discarded. Use
    /// `exec_batch` for migrations / DDL scripts.
    pub async fn prepare(&self, sql: &str) -> Result<Statement, ForgeError> {
        let result = self.api.prepare_v2(&self.db_handle, sql).await.map_jsvalue_err()?;
        // wa-sqlite returns `{stmt: handle, sql: remainder}`. Pluck `stmt`.
        let stmt = js_sys::Reflect::get(&result, &JsValue::from_str("stmt"))
            .map_jsvalue_err()?;
        if stmt.is_null() || stmt.is_undefined() {
            return Err(ForgeError::WasmDatabase(
                format!("prepare_v2 returned no statement for SQL: {sql}"),
            ));
        }
        Ok(Statement { api: self.api.clone(), stmt, finalized: false })
    }
}

impl Statement {
    /// Bind a TEXT parameter. `idx` is 1-based per SQLite convention.
    pub fn bind_text(&self, idx: i32, value: &str) -> Result<(), ForgeError> {
        self.api.bind_text(&self.stmt, idx, value).map_jsvalue_err()?;
        Ok(())
    }

    /// Bind an INTEGER parameter. Note: f64 widening — see wa_sqlite.rs notes.
    pub fn bind_int64(&self, idx: i32, value: i64) -> Result<(), ForgeError> {
        self.api.bind_int64(&self.stmt, idx, value as f64).map_jsvalue_err()?;
        Ok(())
    }

    /// Bind a FLOAT parameter.
    pub fn bind_double(&self, idx: i32, value: f64) -> Result<(), ForgeError> {
        self.api.bind_double(&self.stmt, idx, value).map_jsvalue_err()?;
        Ok(())
    }

    /// Bind a BLOB parameter.
    pub fn bind_blob(&self, idx: i32, value: &[u8]) -> Result<(), ForgeError> {
        self.api.bind_blob(&self.stmt, idx, value).map_jsvalue_err()?;
        Ok(())
    }

    /// Bind a NULL parameter.
    pub fn bind_null(&self, idx: i32) -> Result<(), ForgeError> {
        self.api.bind_null(&self.stmt, idx).map_jsvalue_err()?;
        Ok(())
    }

    /// Convenience: bind `Option<&str>` — None becomes NULL.
    pub fn bind_text_opt(&self, idx: i32, value: Option<&str>) -> Result<(), ForgeError> {
        match value {
            Some(s) => self.bind_text(idx, s),
            None => self.bind_null(idx),
        }
    }

    /// Advance the statement. Returns Row when a row is available, Done at end.
    pub async fn step(&self) -> Result<StepResult, ForgeError> {
        let code = self.api.step(&self.stmt).await.map_jsvalue_err()?;
        let code_i32 = code.as_f64().unwrap_or(-1.0) as i32;
        match code_i32 {
            crate::wa_sqlite::SQLITE_ROW => Ok(StepResult::Row),
            crate::wa_sqlite::SQLITE_DONE => Ok(StepResult::Done),
            other => Err(ForgeError::WasmDatabase(
                format!("step returned unexpected code: {other}"),
            )),
        }
    }

    /// Number of columns in the current row.
    pub fn column_count(&self) -> i32 {
        self.api.column_count(&self.stmt)
    }

    /// SQLite type code for a column in the current row.
    pub fn column_type(&self, idx: i32) -> i32 {
        self.api.column_type(&self.stmt, idx)
    }

    /// Read a TEXT column. Returns None for NULL.
    pub fn column_text(&self, idx: i32) -> Option<String> {
        if self.column_type(idx) == crate::wa_sqlite::SQLITE_NULL {
            return None;
        }
        Some(self.api.column_text(&self.stmt, idx))
    }

    /// Read an INTEGER column. Returns None for NULL.
    pub fn column_int(&self, idx: i32) -> Option<i32> {
        if self.column_type(idx) == crate::wa_sqlite::SQLITE_NULL {
            return None;
        }
        Some(self.api.column_int(&self.stmt, idx))
    }

    /// Read a FLOAT column. Returns None for NULL.
    pub fn column_double(&self, idx: i32) -> Option<f64> {
        if self.column_type(idx) == crate::wa_sqlite::SQLITE_NULL {
            return None;
        }
        Some(self.api.column_double(&self.stmt, idx))
    }

    /// Read a BLOB column. Returns empty vec for NULL — caller can check
    /// `column_type` if NULL/empty distinction matters.
    pub fn column_blob(&self, idx: i32) -> Vec<u8> {
        self.api.column_blob(&self.stmt, idx)
    }

    /// Reset the statement so it can be re-bound and re-stepped.
    pub async fn reset(&self) -> Result<(), ForgeError> {
        self.api.reset(&self.stmt).await.map_jsvalue_err()?;
        Ok(())
    }

    /// Finalize the underlying statement. Idempotent. Called by Drop.
    pub async fn finalize(mut self) -> Result<(), ForgeError> {
        if !self.finalized {
            self.api.finalize(&self.stmt).await.map_jsvalue_err()?;
            self.finalized = true;
        }
        Ok(())
    }
}

impl Drop for Statement {
    fn drop(&mut self) {
        if !self.finalized {
            // Best-effort sync finalize. wa-sqlite's `finalize` is async, but
            // Drop can't await. We fire-and-forget; if the JS side errors, we
            // log via web-sys::console rather than panic. This is a liveness
            // hazard if many statements leak without explicit `finalize()`,
            // but Drop is the safety net, not the primary path.
            //
            // The recommended pattern: `let s = db.prepare(...).await?; ...; s.finalize().await?;`
            //
            // wa-sqlite's underlying sqlite3_finalize is documented as safe
            // to call from anywhere; the async wrapper is a courtesy for
            // matching the Asyncify discipline elsewhere. Calling it
            // synchronously via Reflect would work, but adds complexity.
            // For now, log and move on.
            web_sys::console::warn_1(&JsValue::from_str(
                "forge-wasm: Statement dropped without explicit finalize() — \
                 prefer s.finalize().await for deterministic cleanup",
            ));
        }
    }
}
```

The `console::warn_1` call requires `web-sys` with the `console` feature — already enabled in `crates/forge-wasm/Cargo.toml`.

- [ ] **Step 2: Make SqliteApi cloneable for Statement to hold a copy**

`SqliteApi` is an opaque wasm-bindgen `extern type`. By default these are NOT `Clone`. Two options:

(a) Wrap in `Rc<SqliteApi>` and clone the Rc.
(b) Have Statement borrow `&SqliteApi` from Database.

Option (b) is cleaner but ties Statement's lifetime to Database, which complicates async usage. Option (a) is simpler.

Refactor `Database` to hold `Rc<SqliteApi>` instead of `SqliteApi`:

```rust
#[wasm_bindgen]
pub struct Database {
    api: std::rc::Rc<SqliteApi>,
    db_handle: JsValue,
    closed: bool,
}
```

Update `Database::open` (was already `Self { api, db_handle, closed: false }`):

```rust
        Ok(Self {
            api: std::rc::Rc::new(api),
            db_handle,
            closed: false,
        })
```

And update `Statement` to clone the Rc:

```rust
pub struct Statement {
    api: std::rc::Rc<SqliteApi>,
    stmt: JsValue,
    finalized: bool,
}
```

In `Database::prepare`, store the cloned Rc:

```rust
        Ok(Statement { api: std::rc::Rc::clone(&self.api), stmt, finalized: false })
```

Update all existing `self.api.foo(...)` call sites in Database's existing methods (`exec`, `query`, `close`) — they already work with auto-deref through Rc, no change needed.

- [ ] **Step 3: Rename `exec` → `exec_batch` and keep `query`**

In `crates/forge-wasm/src/database.rs`, the JS-facing `exec` becomes the multi-statement batch path. Migrations call this. Rename it:

```rust
    /// Execute SQL that doesn't capture rows (DDL, DML, DDL+DML scripts).
    /// Multi-statement: semicolons split; all run in order. No parameters.
    /// For parameterized queries, use `prepare` + `Statement::bind_*` + `step`.
    #[wasm_bindgen(js_name = "execBatch")]
    pub async fn exec_batch(&self, sql: String) -> Result<(), JsValue> {
        self.api
            .exec(&self.db_handle, &sql, &JsValue::UNDEFINED)
            .await
            .map_err(forge_error_jsvalue)?;
        Ok(())
    }
```

Keep the existing `query` method unchanged — the harness uses it for ad-hoc inspection.

Now also keep a JS-side `exec` alias so the harness doesn't break. Add a thin shim:

```rust
    /// Deprecated alias for `exec_batch`. Kept temporarily so the
    /// browser-smoke harness from forge-nst6 doesn't break.
    /// Remove after the harness is updated (Task 9).
    pub async fn exec(&self, sql: String) -> Result<(), JsValue> {
        self.exec_batch(sql).await
    }
```

- [ ] **Step 4: Add the Transaction guard**

Append to `crates/forge-wasm/src/database.rs`:

```rust
// ── Rust-only: Transaction guard ─────────────────────────────────────────

/// Async transaction guard. Begin via `Database::transaction()`. Caller
/// MUST explicitly call `commit()` or `rollback()` — failure to do so
/// before drop logs and rolls back is best-effort impossible (Drop is
/// sync; wa-sqlite needs async). Use the `with_transaction` helper to
/// avoid the discipline burden.
pub struct Transaction<'a> {
    db: &'a Database,
    finished: bool,
}

impl<'a> Transaction<'a> {
    pub async fn commit(mut self) -> Result<(), ForgeError> {
        self.db.exec_batch_internal("COMMIT").await?;
        self.finished = true;
        Ok(())
    }

    pub async fn rollback(mut self) -> Result<(), ForgeError> {
        self.db.exec_batch_internal("ROLLBACK").await?;
        self.finished = true;
        Ok(())
    }
}

impl<'a> Drop for Transaction<'a> {
    fn drop(&mut self) {
        if !self.finished {
            web_sys::console::error_1(&JsValue::from_str(
                "forge-wasm: Transaction dropped without commit() or rollback() — \
                 the underlying wa-sqlite session is still in BEGIN. Use \
                 Database::with_transaction to avoid this.",
            ));
        }
    }
}

impl Database {
    /// Begin a transaction. Caller MUST call `commit()` or `rollback()`.
    /// Prefer `with_transaction` which handles both paths.
    pub async fn transaction(&self) -> Result<Transaction<'_>, ForgeError> {
        self.exec_batch_internal("BEGIN").await?;
        Ok(Transaction { db: self, finished: false })
    }

    /// Run an async closure inside a transaction. Commits on Ok, rolls
    /// back on Err. The closure receives `&Database` (NOT the Transaction
    /// guard) — all SQL goes through Database methods directly, the
    /// guard is implicit.
    pub async fn with_transaction<F, Fut, T>(&self, f: F) -> Result<T, ForgeError>
    where
        F: FnOnce(&Database) -> Fut,
        Fut: std::future::Future<Output = Result<T, ForgeError>>,
    {
        self.exec_batch_internal("BEGIN").await?;
        match f(self).await {
            Ok(value) => {
                self.exec_batch_internal("COMMIT").await?;
                Ok(value)
            }
            Err(e) => {
                // Best-effort rollback; surface the original error even if rollback fails.
                let _ = self.exec_batch_internal("ROLLBACK").await;
                Err(e)
            }
        }
    }

    /// Internal exec that returns ForgeError instead of JsValue.
    /// Used by transaction primitives and the migration runner.
    pub(crate) async fn exec_batch_internal(&self, sql: &str) -> Result<(), ForgeError> {
        self.api
            .exec(&self.db_handle, sql, &JsValue::UNDEFINED)
            .await
            .map_jsvalue_err()?;
        Ok(())
    }
}
```

- [ ] **Step 5: Verify both builds pass**

```bash
cargo check -p forge-wasm
RUSTC=$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin/rustc \
  $HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin/cargo \
  check -p forge-wasm --target wasm32-unknown-unknown
cargo test -p forge-wasm
```

Expected: clean check (both targets), 4 native tests still pass.

- [ ] **Step 6: Commit**

```bash
git add crates/forge-wasm/src/database.rs
git commit -m "feat(wasm): Statement wrapper + Transaction guard + exec_batch

- Statement: prepare/bind_*/step/column_*/reset/finalize over the new
  wa-sqlite extern bindings. Result-typed.
- Transaction: explicit commit/rollback guard plus with_transaction
  helper that auto-rollback-on-err. No async-Drop, just a logged warning
  if dropped without resolution.
- Database now holds Rc<SqliteApi> so Statement can clone it.
- exec_batch is the new canonical multi-statement API. exec kept as an
  alias for the existing browser-smoke harness; removed in Task 9.

Validation: cargo check both targets passes. Browser validation comes
in Task 9 via the extended harness.

Refs forge-lu5s."
```

---

## Task 5: forge-wasm migration runner

**Goal:** Apply all 51 migrations to a wa-sqlite Database from WASM, mirroring `forge-sdk::db::migrate::run_migrations` semantics.

**Files:**
- Create: `crates/forge-wasm/src/migrate.rs`
- Modify: `crates/forge-wasm/src/lib.rs` (declare `pub mod migrate;`)

- [ ] **Step 1: Create the migration runner**

Create `crates/forge-wasm/src/migrate.rs`:

```rust
//! WASM migration runner — applies the shared `forge_core::migrations::MIGRATIONS`
//! slice through wa-sqlite. Mirrors `forge_sdk::db::migrate::run_migrations`
//! semantics:
//!
//! - Each migration runs in its own transaction.
//! - Migrations containing `PRAGMA foreign_keys = OFF` get the pragma
//!   set outside the transaction (SQLite ignores it inside BEGIN/COMMIT).
//! - Applied state persists in a `_migrations(name TEXT PRIMARY KEY)` table.

use forge_core::ForgeError;
use forge_core::migrations::MIGRATIONS;

use crate::database::{Database, StepResult};

/// Apply all pending migrations. Returns the count of newly-applied entries.
pub async fn run_migrations(db: &Database) -> Result<usize, ForgeError> {
    ensure_migrations_table(db).await?;
    let applied = get_applied(db).await?;
    let mut count = 0;

    for &(name, sql) in MIGRATIONS {
        if applied.iter().any(|n| n == name) {
            continue;
        }

        let needs_fk_off = sql.contains("PRAGMA foreign_keys = OFF");
        if needs_fk_off {
            db.exec_batch_internal("PRAGMA foreign_keys = OFF").await?;
        }

        db.exec_batch_internal("BEGIN").await?;
        let apply = async {
            db.exec_batch_internal(sql).await?;
            // Insert via prepared statement to bind the migration name safely.
            let stmt = db.prepare("INSERT OR IGNORE INTO _migrations (name) VALUES (?1)").await?;
            stmt.bind_text(1, name)?;
            match stmt.step().await? {
                StepResult::Done | StepResult::Row => {}
            }
            stmt.finalize().await?;
            Ok::<(), ForgeError>(())
        };

        match apply.await {
            Ok(()) => {
                db.exec_batch_internal("COMMIT").await?;
            }
            Err(e) => {
                let _ = db.exec_batch_internal("ROLLBACK").await;
                if needs_fk_off {
                    let _ = db.exec_batch_internal("PRAGMA foreign_keys = ON").await;
                }
                return Err(e);
            }
        }

        if needs_fk_off {
            db.exec_batch_internal("PRAGMA foreign_keys = ON").await?;
        }
        count += 1;
    }

    Ok(count)
}

/// Ensure the `_migrations` table exists. Idempotent.
async fn ensure_migrations_table(db: &Database) -> Result<(), ForgeError> {
    db.exec_batch_internal(
        "CREATE TABLE IF NOT EXISTS _migrations (\
            name TEXT PRIMARY KEY, \
            applied_at TEXT DEFAULT (datetime('now'))\
         )",
    ).await
}

/// Read applied migration names from `_migrations`.
async fn get_applied(db: &Database) -> Result<Vec<String>, ForgeError> {
    let stmt = db.prepare("SELECT name FROM _migrations").await?;
    let mut names = Vec::new();
    loop {
        match stmt.step().await? {
            StepResult::Row => {
                if let Some(name) = stmt.column_text(0) {
                    names.push(name);
                }
            }
            StepResult::Done => break,
        }
    }
    stmt.finalize().await?;
    Ok(names)
}
```

- [ ] **Step 2: Wire the module into lib.rs**

Edit `crates/forge-wasm/src/lib.rs`. Add the module declaration alongside the existing ones:

```rust
pub mod database;
pub mod error;
pub mod migrate;
pub mod wa_sqlite;
```

- [ ] **Step 3: Build to verify wiring**

```bash
cargo check -p forge-wasm
RUSTC=$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin/rustc \
  $HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin/cargo \
  check -p forge-wasm --target wasm32-unknown-unknown
```

Expected: clean. (Validation that the runner actually works on wa-sqlite happens in Task 9 via the harness.)

- [ ] **Step 4: Commit**

```bash
git add crates/forge-wasm/src/migrate.rs crates/forge-wasm/src/lib.rs
git commit -m "feat(wasm): migration runner over wa-sqlite

Mirrors forge-sdk's rusqlite runner: per-migration transaction, pragma
foreign_keys = OFF outside BEGIN, _migrations state table. Consumes the
shared forge_core::migrations::MIGRATIONS slice (lifted in Task 2).

Validation deferred to Task 9 (harness exercises 51 migrations against
wa-sqlite).

Refs forge-lu5s."
```

---

## Task 6: WaSqliteAdapter

**Goal:** Thin façade that opens a Database, runs migrations, exposes `&Database` to stores. No trait abstraction.

**Files:**
- Create: `crates/forge-wasm/src/adapter.rs`
- Modify: `crates/forge-wasm/src/lib.rs` (declare module + re-export `WaSqliteAdapter`)

- [ ] **Step 1: Create the adapter**

Create `crates/forge-wasm/src/adapter.rs`:

```rust
//! WaSqliteAdapter — a.k.a. BrowserStore. Owns a wa-sqlite-backed
//! Database, runs migrations on open, exposes `&Database` to stores.
//!
//! No trait abstraction. The rusqlite stores in forge-sdk stay
//! unchanged; this is purely additive for the wasm32 target.

use forge_core::ForgeError;

use crate::database::Database;
use crate::migrate::run_migrations;

/// The browser-side data layer entry point. Construct via `open()`.
pub struct WaSqliteAdapter {
    db: Database,
}

impl WaSqliteAdapter {
    /// Open (or create) the IDB-backed database, run all pending
    /// migrations, return the adapter ready for store use.
    pub async fn open(filename: &str) -> Result<Self, ForgeError> {
        let db = Database::open(filename).await?;
        let _applied = run_migrations(&db).await?;
        Ok(Self { db })
    }

    /// Borrow the underlying Database. Stores call this to issue queries.
    pub fn db(&self) -> &Database {
        &self.db
    }
}
```

- [ ] **Step 2: Wire it into lib.rs**

Edit `crates/forge-wasm/src/lib.rs`. Add module declaration and a `pub use`:

```rust
pub mod adapter;
pub mod database;
pub mod error;
pub mod migrate;
pub mod wa_sqlite;

pub use adapter::WaSqliteAdapter;
pub use database::Database;
```

- [ ] **Step 3: Build check**

```bash
cargo check -p forge-wasm
RUSTC=$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin/rustc \
  $HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin/cargo \
  check -p forge-wasm --target wasm32-unknown-unknown
```

Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add crates/forge-wasm/src/adapter.rs crates/forge-wasm/src/lib.rs
git commit -m "feat(wasm): WaSqliteAdapter — thin façade for BrowserStore

Opens a Database, runs the migration set, exposes &Database to stores.
Purely additive — no shared trait, no rusqlite-side touches.

Refs forge-lu5s."
```

---

## Task 7: SkillStore over wa-sqlite

**Goal:** First end-to-end consumer of the new API. Mirrors a subset of `forge-sdk::db::stores::skill::SkillStore` against `&Database`.

**Files:**
- Create: `crates/forge-wasm/src/stores/mod.rs`
- Create: `crates/forge-wasm/src/stores/skill.rs`
- Modify: `crates/forge-wasm/src/lib.rs` (declare `pub mod stores;`)

- [ ] **Step 1: Create the stores module**

Create `crates/forge-wasm/src/stores/mod.rs`:

```rust
//! Browser-side data-access stores. Mirror the interfaces of
//! `forge-sdk::db::stores::*` but implemented over `&Database` (wa-sqlite)
//! instead of `&Connection` (rusqlite). Purely additive — see the spec
//! at `.claude/plans/forge-resume-builder/refs/specs/2026-04-28-browserstore-adapter-vertical-slice.md`.

pub mod skill;

pub use skill::SkillStore;
```

- [ ] **Step 2: Implement SkillStore::create**

Create `crates/forge-wasm/src/stores/skill.rs`. Start with the smallest method:

```rust
//! Skill store over wa-sqlite. Subset port of forge-sdk's SkillStore.
//!
//! In scope: create, get, list (without domain_id filter), update, delete,
//! list_categories.
//!
//! Out of scope: get_with_domains, link_domain, unlink_domain — junction
//! queries land in a successor bead.

use forge_core::{ForgeError, Skill, SkillCategory, new_id};

use crate::database::{Database, StepResult};

/// Capitalize first character only, preserving the rest (e.g. "SAFe" stays "SAFe").
fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
    }
}

pub struct SkillStore;

impl SkillStore {
    /// Insert a new skill row. Returns Conflict if the (case-insensitive)
    /// name already exists. Mirrors forge-sdk's SkillStore::create.
    pub async fn create(
        db: &Database,
        name: &str,
        category: Option<SkillCategory>,
    ) -> Result<Skill, ForgeError> {
        let id = new_id();
        let cat = category.unwrap_or(SkillCategory::Other);
        let capitalized = capitalize_first(name.trim());

        let stmt = db.prepare(
            "INSERT INTO skills (id, name, category) VALUES (?1, ?2, ?3)",
        ).await?;
        stmt.bind_text(1, &id)?;
        stmt.bind_text(2, &capitalized)?;
        stmt.bind_text(3, cat.as_ref())?;

        match stmt.step().await {
            Ok(StepResult::Done) | Ok(StepResult::Row) => {
                stmt.finalize().await?;
            }
            Err(ForgeError::WasmDatabase(msg)) if msg.contains("UNIQUE") => {
                let _ = stmt.finalize().await;
                return Err(ForgeError::Conflict {
                    message: format!("Skill with name '{capitalized}' already exists"),
                });
            }
            Err(e) => {
                let _ = stmt.finalize().await;
                return Err(e);
            }
        }

        Self::get(db, &id).await?
            .ok_or_else(|| ForgeError::Internal("Skill created but not found".into()))
    }
```

- [ ] **Step 3: Implement SkillStore::get**

Continue in the same file:

```rust
    /// Fetch a single skill by ID.
    pub async fn get(db: &Database, id: &str) -> Result<Option<Skill>, ForgeError> {
        let stmt = db.prepare(
            "SELECT id, name, category FROM skills WHERE id = ?1",
        ).await?;
        stmt.bind_text(1, id)?;

        let result = match stmt.step().await? {
            StepResult::Row => Some(Self::map_skill_row(&stmt)?),
            StepResult::Done => None,
        };
        stmt.finalize().await?;
        Ok(result)
    }

    /// Read a row into a Skill. Caller must have just received StepResult::Row.
    fn map_skill_row(stmt: &crate::database::Statement) -> Result<Skill, ForgeError> {
        let id = stmt.column_text(0)
            .ok_or_else(|| ForgeError::Internal("skills.id is NULL".into()))?;
        let name = stmt.column_text(1)
            .ok_or_else(|| ForgeError::Internal("skills.name is NULL".into()))?;
        let category_str = stmt.column_text(2)
            .ok_or_else(|| ForgeError::Internal("skills.category is NULL".into()))?;
        let category = SkillCategory::try_from(category_str.as_str())
            .map_err(|_| ForgeError::Internal(format!("invalid skill category: {category_str}")))?;

        Ok(Skill { id, name, category })
    }
```

Note: this assumes `Skill` is a struct with `id: String`, `name: String`, `category: SkillCategory`, and `SkillCategory: TryFrom<&str>`. Verify against `crates/forge-core/src/types/skill.rs` — if the struct shape differs, adjust the mapping. The forge-sdk version uses `params!` + `row.get::<_, String>(0)` patterns, so the column types are TEXT.

- [ ] **Step 4: Implement SkillStore::list (no domain_id filter)**

```rust
    /// List skills, optionally filtered by category and/or text search.
    /// Sorted by name ASC. The domain_id filter is OUT OF SCOPE for this
    /// bead; that variant lives in a successor bead.
    pub async fn list(
        db: &Database,
        category: Option<SkillCategory>,
        search: Option<&str>,
    ) -> Result<Vec<Skill>, ForgeError> {
        let mut sql = String::from("SELECT id, name, category FROM skills WHERE 1=1");
        if category.is_some() {
            sql.push_str(" AND category = ?");
        }
        if search.is_some() {
            sql.push_str(" AND name LIKE ? COLLATE NOCASE");
        }
        sql.push_str(" ORDER BY name ASC");

        let stmt = db.prepare(&sql).await?;
        let mut idx = 1;
        if let Some(c) = category {
            stmt.bind_text(idx, c.as_ref())?;
            idx += 1;
        }
        if let Some(s) = search {
            let pattern = format!("%{s}%");
            stmt.bind_text(idx, &pattern)?;
        }

        let mut skills = Vec::new();
        loop {
            match stmt.step().await? {
                StepResult::Row => skills.push(Self::map_skill_row(&stmt)?),
                StepResult::Done => break,
            }
        }
        stmt.finalize().await?;
        Ok(skills)
    }
```

- [ ] **Step 5: Implement SkillStore::update and delete**

```rust
    /// Update a skill's name and/or category. Returns Conflict on duplicate name.
    pub async fn update(
        db: &Database,
        id: &str,
        new_name: Option<&str>,
        new_category: Option<SkillCategory>,
    ) -> Result<Skill, ForgeError> {
        if new_name.is_none() && new_category.is_none() {
            return Self::get(db, id).await?
                .ok_or_else(|| ForgeError::NotFound { entity: "skill".into(), id: id.into() });
        }

        let mut sets = Vec::new();
        if new_name.is_some() { sets.push("name = ?"); }
        if new_category.is_some() { sets.push("category = ?"); }
        let sql = format!("UPDATE skills SET {} WHERE id = ?", sets.join(", "));

        let stmt = db.prepare(&sql).await?;
        let mut idx = 1;
        let capitalized;
        if let Some(n) = new_name {
            capitalized = capitalize_first(n.trim());
            stmt.bind_text(idx, &capitalized)?;
            idx += 1;
        }
        if let Some(c) = new_category {
            stmt.bind_text(idx, c.as_ref())?;
            idx += 1;
        }
        stmt.bind_text(idx, id)?;

        match stmt.step().await {
            Ok(_) => { stmt.finalize().await?; }
            Err(ForgeError::WasmDatabase(msg)) if msg.contains("UNIQUE") => {
                let _ = stmt.finalize().await;
                return Err(ForgeError::Conflict {
                    message: format!("Skill name conflicts with an existing row"),
                });
            }
            Err(e) => {
                let _ = stmt.finalize().await;
                return Err(e);
            }
        }

        Self::get(db, id).await?
            .ok_or_else(|| ForgeError::NotFound { entity: "skill".into(), id: id.into() })
    }

    /// Delete a skill by ID. Returns the number of rows affected (0 or 1).
    pub async fn delete(db: &Database, id: &str) -> Result<usize, ForgeError> {
        let stmt = db.prepare("DELETE FROM skills WHERE id = ?1").await?;
        stmt.bind_text(1, id)?;
        stmt.step().await?;
        stmt.finalize().await?;
        // wa-sqlite's `changes` accessor isn't bound yet; for now we
        // round-trip a SELECT to confirm. Acceptable for the vertical-slice
        // because callers typically follow delete with a UI refresh.
        let still_exists = Self::get(db, id).await?.is_some();
        Ok(if still_exists { 0 } else { 1 })
    }
```

If `ForgeError::NotFound` doesn't exist in forge-core with that shape, check `crates/forge-core/src/types/mod.rs` (or wherever ForgeError lives) and adjust to the actual variant. The forge-sdk SkillStore returns `Result<Option<Skill>, ForgeError>` for `get`, so the `update` non-found path is rusqlite-tested; mirror that.

- [ ] **Step 6: Implement list_categories**

```rust
    /// List all skill_categories rows. Used by the webui category picker.
    pub async fn list_categories(db: &Database) -> Result<Vec<(String, String)>, ForgeError> {
        let stmt = db.prepare(
            "SELECT slug, label FROM skill_categories ORDER BY label ASC",
        ).await?;
        let mut out = Vec::new();
        loop {
            match stmt.step().await? {
                StepResult::Row => {
                    let slug = stmt.column_text(0)
                        .ok_or_else(|| ForgeError::Internal("skill_categories.slug is NULL".into()))?;
                    let label = stmt.column_text(1)
                        .ok_or_else(|| ForgeError::Internal("skill_categories.label is NULL".into()))?;
                    out.push((slug, label));
                }
                StepResult::Done => break,
            }
        }
        stmt.finalize().await?;
        Ok(out)
    }
}  // close `impl SkillStore`
```

Verify the actual columns of `skill_categories` against `packages/core/src/db/migrations/041_skill_categories_and_seeds.sql` and `044_skill_categories.sql` — adjust column names if `slug`/`label` are named differently. The 044 migration is the most recent rework.

- [ ] **Step 7: Wire stores into lib.rs**

Edit `crates/forge-wasm/src/lib.rs`:

```rust
pub mod adapter;
pub mod database;
pub mod error;
pub mod migrate;
pub mod stores;
pub mod wa_sqlite;

pub use adapter::WaSqliteAdapter;
pub use database::Database;
pub use stores::SkillStore;
```

- [ ] **Step 8: Build check both targets**

```bash
cargo check -p forge-wasm
RUSTC=$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin/rustc \
  $HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin/cargo \
  check -p forge-wasm --target wasm32-unknown-unknown
cargo test -p forge-wasm
```

Expected: clean check both targets. 4 native tests still pass.

- [ ] **Step 9: Commit**

```bash
git add crates/forge-wasm/src/stores/ crates/forge-wasm/src/lib.rs
git commit -m "feat(wasm): SkillStore subset over wa-sqlite

Mirrors forge-sdk's SkillStore: create, get, list, update, delete,
list_categories. Junction-table methods (get_with_domains, link_domain)
are out of scope and folded into a successor bead.

UNIQUE-constraint detection uses substring match on the wa-sqlite error
string, mirroring the rusqlite path. Browser validation in Task 9.

Refs forge-lu5s."
```

---

## Task 8: Extend the browser-smoke harness with parity exercises

**Goal:** End-to-end exercise of WaSqliteAdapter + SkillStore in the browser. This is the spec-acknowledged manual gate (replaced by forge-901c later).

**Files:**
- Modify: `crates/forge-wasm/examples/browser-smoke/index.html`
- Modify: `crates/forge-wasm/examples/browser-smoke/main.js` (or whatever the entry point is named — verify with `ls crates/forge-wasm/examples/browser-smoke/`)
- Modify: `crates/forge-wasm/examples/browser-smoke/vite.config.js` (only if new aliases are needed)

- [ ] **Step 1: Add wasm-bindgen exports for the new types**

The harness JS needs to call `WaSqliteAdapter.open` and `SkillStore.create/get/list/update/delete`. Add `#[wasm_bindgen]` JS-facing wrappers on the Rust side.

In `crates/forge-wasm/src/adapter.rs`, after the existing impl:

```rust
// JS-facing wrapper. Allows the harness to call this directly without
// the JS side reaching through ForgeRuntime.
#[wasm_bindgen]
pub struct WaSqliteAdapterJs(WaSqliteAdapter);

#[wasm_bindgen]
impl WaSqliteAdapterJs {
    /// Open + run migrations. Resolves to the adapter handle.
    #[wasm_bindgen(js_name = "open")]
    pub async fn open_js(filename: String) -> Result<WaSqliteAdapterJs, JsValue> {
        let inner = WaSqliteAdapter::open(&filename)
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(WaSqliteAdapterJs(inner))
    }

    /// Fire-and-forget: create a skill and return its id.
    #[wasm_bindgen(js_name = "createSkill")]
    pub async fn create_skill(&self, name: String, category: Option<String>) -> Result<String, JsValue> {
        use forge_core::SkillCategory;
        let cat = category.map(|c| SkillCategory::try_from(c.as_str()).unwrap_or(SkillCategory::Other));
        let skill = crate::stores::SkillStore::create(self.0.db(), &name, cat)
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(skill.id)
    }

    /// Returns a JSON string of all skills, name-sorted.
    #[wasm_bindgen(js_name = "listSkills")]
    pub async fn list_skills(&self) -> Result<String, JsValue> {
        let skills = crate::stores::SkillStore::list(self.0.db(), None, None)
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        serde_json::to_string(&skills).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Delete a skill by id. Returns rows affected (0 or 1).
    #[wasm_bindgen(js_name = "deleteSkill")]
    pub async fn delete_skill(&self, id: String) -> Result<u32, JsValue> {
        let rows = crate::stores::SkillStore::delete(self.0.db(), &id)
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(rows as u32)
    }

    /// JSON of all skill categories.
    #[wasm_bindgen(js_name = "listCategories")]
    pub async fn list_categories(&self) -> Result<String, JsValue> {
        let cats = crate::stores::SkillStore::list_categories(self.0.db())
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        serde_json::to_string(&cats).map_err(|e| JsValue::from_str(&e.to_string()))
    }
}
```

Add `use wasm_bindgen::prelude::*;` to the top of `adapter.rs` if not already present.

Re-export from lib.rs:

```rust
pub use adapter::{WaSqliteAdapter, WaSqliteAdapterJs};
```

- [ ] **Step 2: Rebuild the wasm artifact**

```bash
cd crates/forge-wasm
PATH=$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH \
  wasm-pack build --target bundler
```

Expected: pkg/ directory rebuilt with `forge_wasm_bg.wasm` and JS glue including the new `WaSqliteAdapterJs` class.

- [ ] **Step 3: Extend the harness HTML**

Look at the current harness entry point first:

```bash
ls crates/forge-wasm/examples/browser-smoke/
cat crates/forge-wasm/examples/browser-smoke/index.html
```

Add a section (above any existing PoC buttons) with these controls:

```html
<section id="browserstore-section">
  <h2>BrowserStore (forge-lu5s)</h2>
  <button id="btn-open">Open + run migrations</button>
  <button id="btn-create">Create "Rust" skill</button>
  <button id="btn-create-dup">Create "Rust" again (expect Conflict)</button>
  <button id="btn-list">List skills</button>
  <button id="btn-delete-first">Delete first listed skill</button>
  <button id="btn-list-categories">List categories</button>
  <pre id="browserstore-out"></pre>
</section>
```

- [ ] **Step 4: Add the JS wiring**

Append to the harness's main JS module (file name varies — check `ls crates/forge-wasm/examples/browser-smoke/`; likely `main.js` or `index.js`):

```js
import init, { WaSqliteAdapterJs } from '../../pkg/forge_wasm.js';

let adapter = null;
const out = document.getElementById('browserstore-out');
function log(msg) {
  const stamp = new Date().toISOString().split('T')[1].slice(0, 8);
  out.textContent += `[${stamp}] ${msg}\n`;
  out.scrollTop = out.scrollHeight;
}

document.getElementById('btn-open').addEventListener('click', async () => {
  await init();
  try {
    adapter = await WaSqliteAdapterJs.open('forge-lu5s.db');
    log('OK: adapter opened, migrations applied');
  } catch (e) {
    log(`FAIL open: ${e}`);
  }
});

document.getElementById('btn-create').addEventListener('click', async () => {
  if (!adapter) return log('FAIL: open first');
  try {
    const id = await adapter.createSkill('Rust', 'language');
    log(`OK: created skill id=${id}`);
  } catch (e) {
    log(`FAIL create: ${e}`);
  }
});

document.getElementById('btn-create-dup').addEventListener('click', async () => {
  if (!adapter) return log('FAIL: open first');
  try {
    const id = await adapter.createSkill('Rust', 'language');
    log(`UNEXPECTED OK: dup created id=${id} — UNIQUE not enforced?`);
  } catch (e) {
    log(`OK (expected): ${e}`);
  }
});

document.getElementById('btn-list').addEventListener('click', async () => {
  if (!adapter) return log('FAIL: open first');
  try {
    const json = await adapter.listSkills();
    const skills = JSON.parse(json);
    log(`OK: ${skills.length} skill(s)`);
    skills.slice(0, 5).forEach(s => log(`  ${s.id} — ${s.name} [${s.category}]`));
  } catch (e) {
    log(`FAIL list: ${e}`);
  }
});

document.getElementById('btn-delete-first').addEventListener('click', async () => {
  if (!adapter) return log('FAIL: open first');
  try {
    const json = await adapter.listSkills();
    const skills = JSON.parse(json);
    if (skills.length === 0) return log('FAIL: no skills to delete');
    const target = skills[0];
    const rows = await adapter.deleteSkill(target.id);
    log(`OK: deleted ${target.name} (rows=${rows})`);
  } catch (e) {
    log(`FAIL delete: ${e}`);
  }
});

document.getElementById('btn-list-categories').addEventListener('click', async () => {
  if (!adapter) return log('FAIL: open first');
  try {
    const json = await adapter.listCategories();
    const cats = JSON.parse(json);
    log(`OK: ${cats.length} categor(y/ies)`);
    cats.slice(0, 5).forEach(([slug, label]) => log(`  ${slug} — ${label}`));
  } catch (e) {
    log(`FAIL categories: ${e}`);
  }
});
```

- [ ] **Step 5: Remove the temporary `Database::exec` JS alias**

In `crates/forge-wasm/src/database.rs`, delete the `exec` shim added in Task 4 Step 3 ("Deprecated alias for `exec_batch`"). The harness now uses the high-level adapter API; the legacy `exec` button (if any) calls `execBatch`. Update the harness's existing PoC buttons to call `execBatch` if needed.

- [ ] **Step 6: Run the harness**

```bash
cd crates/forge-wasm/examples/browser-smoke
bun install   # if not already done in this worktree
bun run dev   # http://localhost:5180
```

Open http://localhost:5180. Click in order:

1. **Open + run migrations** — wait for "OK: adapter opened, migrations applied". (First run takes a few seconds; the IDB DB is empty, so all 51 migrations apply.)
2. **Create "Rust" skill** — expect "OK: created skill id=..."
3. **Create "Rust" again** — expect "OK (expected): ... UNIQUE constraint failed: skills.name" or similar. The error path should be a `Conflict` from Rust, surfaced as a string.
4. **List skills** — expect 1 skill (Rust) plus any seeded by migration 052/053 (skill_graph_initial_population mirrors legacy skills, but skills table starts empty in this fresh DB).
5. **Delete first listed skill** — expect "OK: deleted ..." with rows=1.
6. **List skills** — expect one fewer.
7. **List categories** — expect ≥10 categories (seeded by 044).

- [ ] **Step 7: Verify cross-reload persistence**

With "Rust" still present (re-create if you deleted it):

1. Reload the page (Cmd+R).
2. Click **Open + run migrations** — should be near-instant since `_migrations` already has all 51 entries.
3. Click **List skills** — Rust should still be there.

This confirms IDB persistence + idempotent migrations.

- [ ] **Step 8: Commit harness changes**

```bash
git add crates/forge-wasm/src/adapter.rs crates/forge-wasm/src/lib.rs \
        crates/forge-wasm/src/database.rs \
        crates/forge-wasm/examples/browser-smoke/
git commit -m "feat(wasm): browser-smoke harness exercises BrowserStore end-to-end

WaSqliteAdapterJs JS-facing wrapper exposes open/create/list/delete/
listCategories. Harness exercises the round-trip and verifies cross-
reload persistence. UNIQUE-constraint path verified via the dup-create
button.

This is the manual validation gate per the spec; automated wasm-bindgen-
test coverage lands with forge-901c.

Refs forge-lu5s."
```

---

## Task 9: Final workspace check + close bead

**Goal:** Confirm the whole workspace builds, all native tests pass, and close out.

- [ ] **Step 1: Run all native tests**

From the worktree root:

```bash
cargo test --workspace
```

Expected: forge-core 37+, forge-sdk 290+, forge-wasm 4, forge-server unchanged. No regressions. (forge-server may surface a warning about the unused `template` import — that's fine; the comment-out is intentional and tracked by forge-es6o being closed.)

- [ ] **Step 2: Workspace check**

```bash
cargo check --workspace
```

Expected: clean. **This is the gate that catches feature-unification fallout** — the lesson from forge-nst6.

- [ ] **Step 3: WASM build**

```bash
RUSTC=$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin/rustc \
  $HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin/cargo \
  build -p forge-wasm --target wasm32-unknown-unknown --release
```

Expected: clean release build. Bundle size is allowed to grow (was 71KB post-nst6); record the new size for the commit message and memory.

- [ ] **Step 4: Re-run the browser harness one final time**

Same steps as Task 8 Step 6. Confirm the workflow still works after the final commits.

- [ ] **Step 5: Update spec docs as required by the bead**

Per the bead's "Docs to update on completion" checklist:

- `docs/src/architecture/seams.md` — Seam 6 with actual BrowserStore implementation details. Add a brief entry pointing at the spec + this plan.
- `docs/src/architecture/models/deployment.md` — verify the storage mapping table claims match what shipped (IDB-backed via `IDBBatchAtomicVFS`, not OPFS — that's forge-n89p).

- [ ] **Step 6: Push and close**

```bash
git status                       # confirm clean
git push -u origin forge-lu5s
bd close forge-lu5s --suggest-next
bd close forge-es6o --reason="Resolved by mod-comment workaround in forge-lu5s branch; uncomment when template.rs is committed"
bd dolt push
```

- [ ] **Step 7: Save session memory**

Per CLAUDE.md, save a session-end memory under `/Users/adam/.claude/projects/-Users-adam-code-proj-forge/memory/`. File: `project_forge_lu5s_shipped_<YYYY_MM_DD>.md`. Include: what landed, key decisions encoded, mistakes/lessons (if any), critical-path state post-bead, the next bead (forge-afyg).

Update `MEMORY.md` index with a one-line link to the new file.

---

## Definition of Done

- [ ] forge-core has `migrations` module with the canonical slice (Task 2)
- [ ] forge-sdk's runner is a thin import (Task 2)
- [ ] forge-wasm has Statement, Transaction, exec_batch, migration runner, WaSqliteAdapter, SkillStore subset (Tasks 3–7)
- [ ] Browser-smoke harness exercises create/list/delete/list_categories end-to-end and survives reload (Task 8)
- [ ] `cargo check --workspace` passes from a fresh worktree (Task 9)
- [ ] All existing native tests pass (Task 9)
- [ ] forge-lu5s and forge-es6o closed; branch pushed (Task 9)
- [ ] Session memory written (Task 9)

## Risk register (recap from spec)

| Risk | Mitigation in this plan |
|---|---|
| Migrations contain SQLite features wa-sqlite doesn't support | Validation in Task 8 — first harness "open" runs all 51. Stop and isolate if any fail. |
| `exec_batch` doesn't handle multi-statement scripts | Task 8 Step 6 implicitly tests this — migration 052 has multiple statements. If broken, fall back to splitting on `;` outside string literals. |
| Async transaction guard semantics | `with_transaction` helper hides the discipline (Task 4 Step 4). |
| MIGRATIONS path-rebase breaks forge-sdk | Verified in Task 2 Step 6 (`cargo test -p forge-sdk`). |
| Subagents drift on column types / Skill struct shape | Each store method has a verification note pointing at the rusqlite reference and migration files. Engineer should grep before guessing. |

## Open questions (recap from spec)

1. SkillStore-relevant subset bounded at non-junction methods. If review surfaces a need for `get_with_domains` in MVP-2.0 sooner, scope can extend.
2. wasm-bindgen-test harness timing: this plan uses manual validation. forge-901c automates.
3. No per-store benchmarks. Performance is forge-afyg's concern.
