# forge-sdk SPEC

> Business logic layer for the Forge resume builder.
> TS source: `packages/core/src/services/`, `packages/core/src/storage/`

## Purpose

forge-sdk contains the data access repositories, domain services, and the
resume IR compiler. It depends on forge-core for types and on rusqlite for
SQLite access. All domain logic lives here — binary crates (server, mcp,
cli) are thin wrappers.

## Module Structure

```
src/
├── lib.rs
├── db/
│   ├── mod.rs              # re-exports all repos
│   ├── source_repo.rs      # SourceRepository
│   ├── bullet_repo.rs      # BulletRepository
│   ├── perspective_repo.rs # PerspectiveRepository
│   ├── resume_repo.rs      # ResumeRepository
│   ├── skill_repo.rs       # SkillRepository
│   ├── organization_repo.rs
│   ├── jd_repo.rs          # JobDescriptionRepository
│   ├── contact_repo.rs
│   ├── summary_repo.rs
│   ├── note_repo.rs
│   └── profile_repo.rs
└── services/
    ├── mod.rs               # re-exports all services
    ├── source_service.rs
    ├── bullet_service.rs
    ├── perspective_service.rs
    ├── resume_service.rs
    ├── skill_service.rs
    ├── organization_service.rs
    ├── jd_service.rs
    ├── contact_service.rs
    ├── summary_service.rs
    ├── tagline/             # Tagline state rules (TS: services/tagline-service.ts, routes/resumes.ts:222-307)
    │   └── mod.rs           # js_trim, has_override, tagline_state, get_state
    ├── note_service.rs
    ├── profile_service.rs
    ├── compiler_service.rs  # Resume IR compiler
    ├── audit_service.rs     # Chain tracing, integrity
    ├── review_service.rs    # Review queue
    ├── integrity_service.rs # Drift detection
    └── export_service.rs    # JSON/LaTeX/PDF export
```

## Architecture

### TS Pattern → Rust Translation

The TS codebase uses an EntityLifecycleManager (ELM) as a generic
storage abstraction with constraint enforcement. In Rust, we skip the
ELM layer and use direct rusqlite calls in repositories:

```
TS:  Route → Service → ELM → StorageAdapter → SQLite
Rust: Route → Service → Repository → rusqlite → SQLite
```

Constraint enforcement (FK checks, enum validation, cascade deletes)
moves into the repository methods themselves, using SQL constraints
and explicit checks.

### Service Facade

The SDK exposes a single `Forge` struct as the public API:

```rust
pub struct Forge {
    pub sources: SourceService,
    pub bullets: BulletService,
    pub perspectives: PerspectiveService,
    pub resumes: ResumeService,
    // ... all domain services
}

impl Forge {
    pub fn new(db_path: &str) -> Result<Self, ForgeError> { ... }
}
```

This mirrors the TS `createServices(db)` factory. Binary crates
construct one `Forge` instance and pass references to route handlers.

### Repository Pattern

Repositories own a `rusqlite::Connection` reference and provide
typed CRUD + query methods:

```rust
pub struct SourceRepository<'conn> {
    conn: &'conn Connection,
}

impl<'conn> SourceRepository<'conn> {
    pub fn create(&self, input: &CreateSource) -> Result<Source, ForgeError> { ... }
    pub fn get(&self, id: &str) -> Result<Option<Source>, ForgeError> { ... }
    pub fn list(&self, filter: &SourceFilter, pg: &PaginationParams) -> Result<(Vec<Source>, Pagination), ForgeError> { ... }
}
```

### Error Handling

- Repositories map `rusqlite::Error` → `ForgeError`
- Services return `Result<T, ForgeError>` for all operations
- Binary crates use `anyhow` for top-level error handling

### Database

- **Crate**: rusqlite (sync, mature, lightweight)
- **Migrations**: Shared SQL files with the TS codebase (same schema)
- **Connection**: Single `rusqlite::Connection`, not pooled (single-user)
- **Transactions**: `conn.execute_batch()` or explicit `Transaction`

### Configuration

- **Crate**: figment (layered: env vars → config files → CLI flags)
- Each binary crate defines its own config struct

## Implementation Order (R0)

1. **IR Compiler** — pure computation, no DB needed, easily testable
2. **Source + Bullet** — simple CRUD, establishes repo pattern
3. **Perspective** — depends on bullet, tests derivation chain
4. **Resume** — most complex, depends on all above
5. **Support services** — audit, review, integrity, export

## Testing Strategy

- Unit tests: per-service, using rusqlite in-memory DB (`:memory:`)
- Integration tests: full Forge instance against temp DB file
- Fixtures: port key TS test fixtures, or seed via repository methods
- IR compiler tests: golden-file testing (known input → expected output)

## Hard Constraint: Zero Subprocess Calls

After migration, all functionality must be compiled in. No shelling out
to external binaries. Implications for forge-sdk:
- **PDF generation**: tectonic compiled-in as library, not subprocess
- **DB export**: `rusqlite::backup::Backup` API, not `sqlite3` CLI
- **All I/O**: via Rust crates, never `std::process::Command`

## Dependencies

- `forge-core` — types
- `rusqlite` — SQLite access (with `bundled` feature)
- `tectonic` — LaTeX → PDF (compiled-in engine)
- `serde` + `serde_json` — serialization
- `uuid` — ID generation
- `thiserror` — error derive macros
