# Design: Rust Implementation Phases & Migration Strategy

**Date**: 2026-04-23
**Status**: Accepted
**Context**: Stubs (forge-dw4) and crate selection (forge-zng6) complete.

## Hard Constraint

**Zero subprocess calls.** After the TS→Rust migration, all functionality
must be compiled in. No shelling out to external binaries (tectonic, sqlite3,
pdflatex, etc.). This means:
- PDF generation via compiled-in tectonic engine (or typst)
- DB export via rusqlite backup API
- LLM via compiled-in reqwest HTTP client

## R0 Implementation Order

### Phase R0.1: Foundation (forge-core + forge-sdk infra)

1. **Error types** — `ForgeError` enum with thiserror derives, `From<rusqlite::Error>`
2. **DB connection** — `Connection` wrapper, migration runner (reuse TS SQL files)
3. **Repository trait** — common pattern: `new(&Connection)`, typed CRUD
4. **Forge facade** — `Forge::new(db_path)` constructing all services

### Phase R0.2: IR Compiler (pure computation)

Best first implementation target:
- No DB dependency (takes assembled data, returns IR tree)
- Pure functions, easily testable with golden-file tests
- Most complex logic but cleanly isolated
- Already has comprehensive TS test suite to port

### Phase R0.3: Core CRUD (Source → Bullet → Perspective)

In dependency order:
1. **Source** — simplest entity, establishes repo CRUD pattern
2. **Bullet** — depends on source, introduces junction tables
3. **Perspective** — depends on bullet, completes derivation chain

### Phase R0.4: Resume + Support

1. **Skill, Organization, Summary** — independent CRUD domains
2. **Resume** — depends on all above, most complex repo
3. **Contact, Note, Profile, JD** — remaining CRUD domains
4. **Audit, Review, Integrity, Export** — cross-cutting services

## Crate Seam Design

### forge-core → forge-sdk

forge-core exports types only. No trait definitions that force
implementation patterns. forge-sdk imports types and defines its own
internal traits if needed.

### forge-sdk → forge-server

The `Forge` struct is the single public API surface:

```rust
// forge-sdk/src/lib.rs
pub struct Forge {
    conn: Connection,
    pub sources: SourceService,
    pub bullets: BulletService,
    // ...
}

impl Forge {
    pub fn open(path: &str) -> Result<Self, ForgeError> { ... }
    pub fn open_memory() -> Result<Self, ForgeError> { ... } // for tests
}
```

axum routes receive `Arc<Forge>` via State extractor:

```rust
// forge-server/src/routes/sources.rs
async fn list_sources(
    State(forge): State<Arc<Forge>>,
    Query(params): Query<SourceFilter>,
) -> impl IntoResponse {
    // forge-sdk is sync; wrap in spawn_blocking if needed
    let result = forge.sources.list(&params, &PaginationParams::default());
    Json(result)
}
```

### forge-sdk → forge-ai

forge-ai is independent of forge-sdk. The server orchestrates:

```rust
// forge-server calls forge-ai for LLM operations
let prompt = forge_ai::build_derivation_prompt(&source, &config);
let response = forge_ai::complete(prompt).await?;
let bullets = forge_ai::parse_bullets(&response)?;
forge.bullets.create_batch(&bullets)?;
```

No circular dependency: server imports both sdk and ai.

## Database Migration Strategy

### Shared SQL files with TS

The Rust and TS codebases share the same SQLite schema. Migrations are
plain SQL files (already exist in `packages/core/src/db/migrations/`).

Rust migration runner:
- Read SQL files from an embedded directory (`include_str!` or `rust-embed`)
- Track applied migrations in a `_migrations` table
- Run in order at `Forge::open()` startup

This avoids schema divergence. Both TS and Rust can operate on the
same `forge.db` file during the transition period.

### Long-term

Once TS is fully replaced, migrations move to `crates/forge-sdk/migrations/`.
For now, symlink or copy at build time.

## Testing Strategy

### Unit tests

Each repository and service gets `#[cfg(test)]` module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup() -> (Connection, Forge) {
        let forge = Forge::open_memory().unwrap();
        // run migrations automatically
        forge
    }

    #[test]
    fn create_source() {
        let forge = setup();
        let source = forge.sources.create(&CreateSource {
            title: "Test".into(),
            description: "Desc".into(),
            ..Default::default()
        }).unwrap();
        assert_eq!(source.title, "Test");
    }
}
```

### Integration tests

`tests/` directory in forge-sdk, testing full chains:
source → bullet → perspective → resume entry → IR compile

### IR Compiler tests

Golden-file testing:
1. Seed known data via repos
2. Compile to IR
3. Compare JSON output against snapshot
4. Render to LaTeX, compare against snapshot

## Error Type Design

```rust
// forge-core/src/error.rs
#[derive(Debug, thiserror::Error)]
pub enum ForgeError {
    #[error("not found: {entity_type} {id}")]
    NotFound { entity_type: String, id: String },

    #[error("validation: {message}")]
    Validation { message: String, field: Option<String> },

    #[error("conflict: {message}")]
    Conflict { message: String },

    #[error("FK violation: {message}")]
    ForeignKey { message: String },

    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("{0}")]
    Internal(String),
}
```

This replaces the TS `ForgeError { code, message, details }` struct
with a proper Rust enum. Serialization to JSON uses the same shape
for API responses.
