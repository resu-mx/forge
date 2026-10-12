# forge-core SPEC

> Canonical type definitions for the Forge resume builder.
> TS source: `packages/core/src/types/index.ts`

## Purpose

forge-core is the shared vocabulary crate. It defines all entity structs,
enums, input/output types, IR shapes, and error types used across the Rust
workspace. It contains **no business logic and performs no I/O**.

## Module Structure

```
src/
├── lib.rs          # re-exports types::*
└── types/
    ├── mod.rs      # submodule declarations + glob re-exports
    ├── enums.rs    # status unions, category enums, type discriminators
    ├── entities.rs # database row shapes (entity structs)
    ├── inputs.rs   # create/update input structs
    ├── common.rs   # result types, pagination, filters, constants
    └── ir.rs       # resume intermediate representation
```

## Conventions

### Serde

All types derive `Serialize, Deserialize`. Enums use `#[serde(rename_all = "snake_case")]`
to match the TS/JSON wire format. Tagged unions use `#[serde(tag = "kind")]` or
`#[serde(tag = "type")]` to match TS discriminated unions.

### Nullability

TS `field?: T | null` maps to Rust `Option<T>`. For update inputs where
"absent" (don't change) differs from "null" (clear the value), use
`Option<Option<T>>` — outer None = absent, Some(None) = set null.

### IDs

All entity IDs are `String` (UUIDs stored as TEXT in SQLite). Generated
via `uuid::Uuid::new_v4().to_string()` at the repository layer.

### Timestamps

Stored as `String` in ISO 8601 format, matching SQLite TEXT columns. No
chrono parsing at the type level — parsing happens in services if needed.

### Boolean-like integers

SQLite STRICT mode uses INTEGER for booleans. Entity structs use `i32`
for fields like `is_headquarters`, `worked`, `is_in_progress` to match
the storage layer. IR types use `bool` for computed fields like
`is_cloned`, `is_override`.

### Error type

`ForgeError` is a structured error with `code`, `message`, and optional
`details` (serde_json::Value). All fallible operations across the
workspace return `Result<T, ForgeError>`.

With the `rusqlite` feature, `From<rusqlite::Error> for ForgeError` (`types/sqlite_error.rs`)
maps SQLite constraint failures to the TS codes (TS: `storage/error-mapper.ts`), so every
store's `?` gets it and both hosts (forge-server, forge-wasm) behave alike:

| SQLite | Extended code | `ForgeError` | Code / status |
|---|---|---|---|
| FOREIGN KEY (insert/update) | 787 | `Validation` | `VALIDATION_ERROR` 400 |
| `ON DELETE RESTRICT` blocked | **1811** (`CONSTRAINT_TRIGGER`, message `FOREIGN KEY constraint failed`) | `Conflict` | `CONFLICT` 409 |
| FOREIGN KEY on delete without RESTRICT | 787 | `Conflict` via `ForgeError::from_sqlite_delete` | `CONFLICT` 409 |
| UNIQUE / PRIMARY KEY | 2067 / 1555 | `Conflict` | `CONFLICT` 409 |
| CHECK, NOT NULL | 275, 1299 | `Validation` | `VALIDATION_ERROR` 400 |
| anything else | n/a | `Database` | `DATABASE_ERROR` 500 |

`ForgeError::ForeignKey` serializes as `VALIDATION_ERROR` (TS never sends `FK_VIOLATION`).
Stores keep a targeted mapping only where TS answers differently (skill name → 409 with its own
message; org alias duplicate → 400, as `routes/campuses.ts` does).

## Rust Divergences from TS

| TS Pattern | Rust Equivalent |
|---|---|
| `interface Foo extends Bar` | `struct Foo { #[serde(flatten)] base: Bar, ... }` |
| `type T = 'a' \| 'b'` | `enum T { A, B }` with serde rename |
| `T \| null \| undefined` | `Option<T>` |
| `field?: T` (update input) | `Option<Option<T>>` for null-clearable |
| `Uint8Array` / `Float32Array` | `Vec<u8>` / `Vec<f32>` |
| `unknown` | `serde_json::Value` |
| `Result<T>` | `ForgeResult<T>` (avoids std conflict) |

## Dependencies

- `serde` + `serde_json` — serialization
- `chrono` — date/time types (used sparingly; timestamps are String)

## What Does NOT Belong Here

- Database access (→ forge-sdk)
- Business logic (→ forge-sdk)
- LLM interactions (→ forge-ai)
- HTTP routing (→ forge-server)
- Configuration types (→ each binary crate owns its own config)
