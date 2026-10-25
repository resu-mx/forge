# forge-sdk SPEC

> The data and domain layer of Forge: one store per entity, a few cross-entity services, the
> migration runner and the resume IR compiler. Both hosts (`forge-server`, `forge-wasm`) run
> this code. Read it with `crates/AGENTS.md`.

## Purpose and dependencies

forge-sdk owns everything between the HTTP contract and SQLite. It performs no HTTP and holds
no configuration; hosts own those. Types and errors come from `forge-core`.

- Forge deps: `forge-core` with `features = ["rusqlite"]`.
- Other deps: `rusqlite` (bundled), `serde`, `serde_json`, `uuid`, `thiserror`, `icu_collator`
  (tagline sorting). `tracing` is declared but nothing references it yet. Dev: `chrono`.
- Reverse deps: `forge-api`, `forge-typst`, `forge-server`, `forge-wasm`, `forge-mcp`, `forge-cli`.

## Layering

```
TS:   route -> service -> ELM -> StorageAdapter -> SQLite
Rust: forge-api route -> store -> rusqlite -> SQLite
```

- **The stores are the service layer.** Routes call `forge_sdk::db::stores::*` directly, and
  each store enforces its entity's validation and business rules (FK checks, enum checks,
  cascades), so both runtimes behave alike. There is no per-entity service module (deleted in
  #118; `.agents/plans/rust-mvp-roadmap.md:107`).
- **The TS services are the reference** for those rules. A rule a store does not enforce yet is
  marked `TODO(resu-mx/forge#240)` on the method that should carry it:
  `grep -rn 'TODO(resu-mx/forge#240)' crates/forge-sdk/src/db/stores`.
- **`forge-api` owns the HTTP contract**: status codes, JSON shapes, routing. forge-sdk returns
  `Result<T, ForgeError>`; `forge-core` maps `rusqlite::Error` into it (see its SPEC).
- **`services/` is for work that spans entities**: compiling, exporting, auditing, reviewing,
  integrity checks, taglines, Typst source.
- **`Forge` (`forge.rs`) is a connection owner, not a facade.** `open` / `open_memory` set the
  PRAGMAs, run the migrations and keep one `Connection`; `conn()` hands it out. It holds no
  services, and each store takes a `&Connection` per call.

## Modules

TS paths are relative to `packages/core/src/`. "none" means there is no TS counterpart.

| Module | Owns | TS source |
|---|---|---|
| `lib.rs` | module declarations, re-exports `Forge` | none |
| `forge.rs` | `Forge`: open, PRAGMAs (WAL, foreign keys, busy timeout), migrations, `conn()` | `db/connection.ts` |
| `db/mod.rs` | re-exports `stores::*` | none |
| `db/migrate.rs` | `MIGRATIONS`: the TS `.sql` files embedded with `include_str!`, applied in order, tracked in `_migrations`. `known_head()` is the newest entry, `schema_head(conn)` the newest applied one, and `is_newer_than_known()` tells whether a database was written by a newer build (the list must stay in name order, a test checks it) | `db/migrate.ts` |
| `db/backends/{mod,sqlite}.rs` | re-exports `rusqlite::Connection`; no backend trait yet | `storage/adapters/` |
| `db/stores/mod.rs` | store re-exports, and the "stores are the service layer" contract | none |
| `db/stores/{address,answer_bank,archetype,bullet,certification,contact,credential,derivation,domain,industry,note,organization,perspective,profile,resume,role_type,skill,source,summary,template}.rs` | `<Entity>Store`: SQL and validation for one entity and its junction tables | `services/<entity>-service.ts`, kebab-case (`answer_bank` is `services/answer-bank-service.ts`) |
| `db/stores/campus.rs` | `CampusStore`: `org_locations` and `org_aliases` | `services/org-location-service.ts`, `routes/campuses.ts` |
| `db/stores/jd.rs` | `JdStore`, `parse_status_filter` | `services/job-description-service.ts` |
| `db/stores/jd_resume.rs` | `JdResumeStore`: the `job_description_resumes` junction | inline SQL in `routes/job-descriptions.ts` and `routes/resumes.ts` |
| `db/stores/lookup.rs` | rules shared by `domain`, `industry` and `role_type` (no store type of its own) | `services/{domain,industry,role-type}-service.ts` |
| `db/stores/extension.rs` | `ExtensionConfigStore`, `ExtensionLogStore` | `services/extension-config-service.ts`, `services/extension-log-service.ts` |
| `db/stores/skill_graph.rs` | `SqlSkillGraphStore`: the SQL implementation of `SkillGraphTraversal` | none (migrations 052, 053) |
| `services/mod.rs` | re-exports of the services below | `services/index.ts` |
| `services/compiler_service.rs` | `CompilerService`: resume rows to the `ResumeDocument` IR | `services/resume-compiler.ts` |
| `services/export_service.rs` | `ExportService`: data export bundle, `dump_database` | `services/export-service.ts` |
| `services/audit_service.rs` | `AuditService`: derivation chain tracing, snapshot comparison | `services/audit-service.ts` |
| `services/integrity_service.rs` | `IntegrityService`: stale-snapshot (drift) detection | `services/integrity-service.ts` |
| `services/review_service.rs` | `ReviewService`: the review queue | `services/review-service.ts` |
| `services/tagline/{mod,generator,regenerate,golden}.rs` | tagline state rules, the pure TF-IDF generator, DB-backed regeneration, golden cases shared with TS | `services/tagline-service.ts`, the tagline routes in `routes/resumes.ts` |
| `services/typst_render.rs` | `render_typst`: IR to Typst source (the `sb2nov` layout). `forge-typst` compiles it to PDF | `templates/sb2nov.ts` |
| `tests/demo_datasets.rs` | integration test against the demo datasets | none |
A store's `//!` header names the exact TS function or route when it differs from the table.

## Invariants

- **Compiles for `wasm32-unknown-unknown`.** `forge-wasm` links it. `rusqlite` reaches the
  browser through `sqlite-wasm-rs` (ADR 0001). No tokio, no `std::process`, no native-only
  crates (`just wasm-deps-check`).
- **No subprocess.** PDFs are Typst, built in-process by `forge-typst` (ADR 0003, `docs/src/dev/adrs/rust-wasm/`).
  The database dump is in-process (`ExportService::dump_database`).
- **The schema is the TS migrations.** `packages/core/src/db/migrations/*.sql` is the source;
  `db/migrate.rs` embeds a hand-maintained list. **A new `.sql` migration needs a new entry in
  `MIGRATIONS`**, and no test compares the list with the directory.
- **No per-entity service layer.** Put a rule in the store. Add a `services/` module only for
  logic that spans entities; if lifecycle side effects ever need a real service layer (#96),
  design it then.

## Testing

- Inline `#[cfg(test)]` modules, on `Forge::open_memory()` where a database is needed.
- `tests/demo_datasets.rs` for the integration path.
- `services/tagline/golden.rs` checks the port against a fixture the TS test writes
  (`packages/core/src/services/__tests__/tagline-golden.test.ts`).
- Route behaviour is checked through `forge-api`: `just parity-tier0` runs the TS route tests
  against the Rust server.

## Not built

- A database backend trait. `db/backends/mod.rs` names Turso/libSQL and HelixDB as candidates,
  pending the DB strategy decision (`forge-58ht`).
- Store rules still on the TS side: tracked in #240.
- The TS-only data step `migrations/006_summaries_data.ts` (backfills summaries from legacy resume
  header blobs). Rust runs only the `.sql` files, so it does not run it.
