# forge-core SPEC

> The shared vocabulary of the Forge workspace: entity structs, enums, input and filter types,
> the resume IR, the skill-graph contract and `ForgeError`. Read it with `crates/AGENTS.md`.

## Purpose and dependencies

forge-core defines the types every other crate speaks. It contains **no business logic and
performs no I/O**. The one piece of logic in it is `From<rusqlite::Error> for ForgeError`, which
has to sit beside the type (orphan rule).

- Forge deps: none.
- Deps: `serde`, `serde_json`, `chrono`, `thiserror`, `uuid`, `strum`, and `rusqlite` (optional).
  `chrono` is used only by `util::now_iso`.

## Features

`default = []`. Features are opt-in, so no consumer inherits one by accident. The reason is in
the comment in `Cargo.toml`: cargo's feature unification can switch a default-on feature back on
for a `default-features = false` consumer.

| Feature | Adds |
|---|---|
| `rusqlite` | `types/sqlite_error.rs` (`From<rusqlite::Error> for ForgeError`, `ForgeError::from_sqlite_delete`, `is_foreign_key`, `is_unique`) and `ForgeError::Database` |

The workspace declares forge-core with `default-features = false`. Every consumer enables
`rusqlite` today, `forge-wasm` included: `rusqlite` reaches the browser through `sqlite-wasm-rs`
(ADR 0001). `ForgeError::WasmDatabase(String)` is not gated; `forge-wasm` builds it from the
JS-side message.

## Modules

TS paths are relative to `packages/core/src/`. Each `types/*.rs` header names the section of
`types/index.ts` it mirrors.

| Module | Owns | TS source |
|---|---|---|
| `lib.rs` | module declarations; re-exports `types::*`, `new_id`, `now_iso` | `types/index.ts` |
| `serde_util.rs` | `double_option` (absent vs `null` on PATCH inputs), `int_or_bool` (a 0/1 column the wire sends as a boolean) | none |
| `util.rs` | `new_id()` (UUID v4 string), `now_iso()` (UTC, second precision) | none |
| `types/mod.rs` | submodule declarations and glob re-exports | `types/index.ts` |
| `types/enums.rs` | status unions, category enums, type discriminators | `types/index.ts` (status unions) |
| `types/entities.rs` | database row shapes, and the `*WithOrg`-style joined shapes | `types/index.ts` (core entities) |
| `types/inputs.rs` | create and update input structs | `types/index.ts` (input types) |
| `types/common.rs` | `ForgeError`, results, pagination, filters, shared constants | `types/index.ts` (result types, filters) |
| `types/ir.rs` | the resume intermediate representation | `types/index.ts` (Resume IR) |
| `types/skill_graph.rs` | `skill_graph_*` row shapes, `EdgeType`, `NodeSource`, the `SkillGraphTraversal` trait | none (migration 052; `docs/src/architecture/graphs/skills.md`) |
| `types/sqlite_error.rs` | SQLite constraint failures to `ForgeError` (feature `rusqlite`) | `storage/error-mapper.ts` |
| `types/tests.rs` | serde conformance tests against the TS wire format | `types/index.ts` |

`SkillGraphTraversal` is implemented by `SqlSkillGraphStore` (forge-sdk) and by
`SkillGraphRuntime` (forge-wasm).

## Conventions

- **Serde.** Types derive `Serialize` and `Deserialize`. Enums use `rename_all = "snake_case"`
  plus the strum derives (`Display`, `EnumString`, `AsRefStr`) for the SQL text form; the
  skill-graph enums are kebab-case. Tagged unions use `tag = "kind"` or `tag = "type"` to match
  the TS discriminated unions. `#[serde(flatten)]` models `extends`.
- **Nullability.** `T | null | undefined` is `Option<T>`. A PATCH input where "absent" (leave
  alone) differs from `null` (clear) is `Option<Option<T>>` with `#[serde(default,
  deserialize_with = "double_option")]`.
- **IDs and timestamps.** IDs are `String` UUIDs from `forge_core::new_id()`. Timestamps are
  `String`, ISO 8601, from `now_iso()`; nothing parses them at the type level.
- **Booleans.** SQLite has no boolean. Entity fields mirror their column: some are `i32`
  (`worked`), some are `bool` (`is_headquarters`). Read the struct. Computed IR fields are
  `bool`. `int_or_bool` lets an input accept either on the wire.
- **`ForgeError`** is an enum, not a struct. `code()` gives the wire code and `client_message()`
  the text a client sees; `Display` keeps the variant label for logs. `Serialize` is
  hand-written and emits `{ code, message }`. `forge-api` (`crates/forge-api/src/error.rs`)
  maps the code to an HTTP status.

### SQLite constraint mapping

With the `rusqlite` feature, every store's `?` gets this mapping, so both hosts behave alike
(TS: `storage/error-mapper.ts`):

| SQLite | Extended code | `ForgeError` | Code / status |
|---|---|---|---|
| FOREIGN KEY (insert/update) | 787 | `Validation` | `VALIDATION_ERROR` 400 |
| `ON DELETE RESTRICT` blocked | **1811** (`CONSTRAINT_TRIGGER`, message `FOREIGN KEY constraint failed`) | `Conflict` | `CONFLICT` 409 |
| FOREIGN KEY on delete without RESTRICT | 787 | `Conflict` via `ForgeError::from_sqlite_delete` | `CONFLICT` 409 |
| UNIQUE / PRIMARY KEY | 2067 / 1555 | `Conflict` | `CONFLICT` 409 |
| CHECK, NOT NULL | 275, 1299 | `Validation` | `VALIDATION_ERROR` 400 |
| anything else (including a trigger's own `RAISE`) | n/a | `Database` | `DATABASE_ERROR` 500 |

`ForgeError::ForeignKey` serializes as `VALIDATION_ERROR`: TS never sends `FK_VIOLATION`. A
store overrides the mapping only where TS answers differently (a duplicate skill name, in
`db/stores/skill.rs`; a duplicate org alias, in `db/stores/campus.rs`).

## Divergences from TS

| TS | Rust |
|---|---|
| `interface Foo extends Bar` | `struct Foo { #[serde(flatten)] base: Bar, ... }` |
| `type T = 'a' \| 'b'` | `enum T { A, B }` with serde and strum renames |
| `T \| null \| undefined` | `Option<T>` |
| `field?: T` (nullable update input) | `Option<Option<T>>` |
| `Uint8Array` / `Float32Array` | `Vec<u8>` / `Vec<f32>` |
| `unknown` | `serde_json::Value` |

## What does not belong here

- Database access and business rules: the `forge-sdk` stores.
- LLM prompts and response checks: `forge-ai`.
- HTTP routing and status codes: `forge-api`.
- Hosting and configuration: `forge-server`, `forge-wasm`, and each binary crate.
