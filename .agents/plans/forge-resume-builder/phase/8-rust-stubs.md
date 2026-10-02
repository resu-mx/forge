# Phase 8: Rust Stubs

**Goal:** Mirror all TypeScript types and interfaces as Rust stubs in `crates/core/`.

**Non-Goals:** No working implementation. Stubs only with `todo!()` bodies.

**Depends on:** Phase 3+ (TS implementation substantially complete — stubs should capture implementation learnings)
**Blocks:** Nothing — completely off the critical path
**Can parallelize with:** Phase 9 (start once TS services are implemented, run concurrently with integration testing)

**NOTE:** Starting stubs too early defeats their purpose. The value is capturing design decisions and edge cases discovered during TS implementation. Write stubs after Phase 3 (services) is complete so the doc comments reflect real implementation knowledge.

---

## Task 8.1: Rust Type Definitions

**File:** `crates/core/src/types.rs`

**Steps:**
1. Define Rust structs mirroring every TypeScript entity type
2. Use `serde` for JSON serialization (derive `Serialize`, `Deserialize`)
3. Use Rust enums for status types (with string representation)
4. Add doc comments referencing the TS source file

**Example:**
```rust
use serde::{Deserialize, Serialize};

/// Source — human-authored ground truth description of work performed.
/// TS: packages/core/src/types/index.ts → Source
#[derive(Debug, Serialize, Deserialize)]
pub struct Source {
    pub id: String,
    pub title: String,
    pub description: String,
    pub employer_id: Option<String>,
    pub project_id: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub status: SourceStatus,
    pub updated_by: UpdatedBy,
    pub last_derived_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceStatus {
    Draft,
    Approved,
    Deriving,
}
```

**Acceptance Criteria:**
- [ ] All entity types have Rust struct equivalents
- [ ] All enums have Rust enum equivalents with serde rename
- [ ] Doc comments reference the TS source file
- [ ] `cargo check` passes

---

## Task 8.2: Repository Stubs

**Files:**
- `crates/core/src/db/source_repository.rs`
- `crates/core/src/db/bullet_repository.rs`
- `crates/core/src/db/perspective_repository.rs`
- `crates/core/src/db/resume_repository.rs`
- `crates/core/src/db/mod.rs`

**Reference:** `refs/examples/rs-stubs/source-repository.rs`

**Steps:**
1. Create each repository with function signatures matching TS counterparts
2. Bodies are `todo!()`
3. Doc comments describe the TS implementation behavior, edge cases, and design decisions discovered during TS development
4. Note any TS-specific patterns that need different Rust approaches

**Acceptance Criteria:**
- [ ] All repository methods have Rust function signatures
- [ ] Doc comments describe behavior and edge cases
- [ ] `cargo check` passes (todo! compiles but panics at runtime)

---

## Task 8.3: Service Stubs

**Files:**
- `crates/core/src/services/source_service.rs`
- `crates/core/src/services/derivation_service.rs`
- `crates/core/src/services/resume_service.rs`
- `crates/core/src/services/audit_service.rs`
- `crates/core/src/services/review_service.rs`
- `crates/core/src/services/mod.rs`

**Acceptance Criteria:**
- [ ] All service methods have Rust function signatures
- [ ] Doc comments describe business logic rules
- [ ] Transaction boundaries noted in comments
- [ ] `cargo check` passes

---

## Task 8.4: AI Module Stubs

**Files:**
- `crates/core/src/ai/mod.rs`
- `crates/core/src/ai/claude.rs`
- `crates/core/src/ai/prompts.rs`
- `crates/core/src/ai/validator.rs`

**Doc comments should note:**
- In Rust, this will likely use the Anthropic API directly (not CLI)
- Async runtime: tokio
- JSON parsing: serde_json
- Process spawning: tokio::process (if still wrapping CLI)

**Acceptance Criteria:**
- [ ] Function signatures mirror TS AI module
- [ ] Doc comments describe the prompt templates and validation pipeline
- [ ] Notes on Rust-specific implementation choices
- [ ] `cargo check` passes

---

## Task 8.5: Route Stubs

**Files:**
- `crates/core/src/routes/mod.rs`
- Route handler stubs for each resource

**Doc comments should note:**
- In Rust, this will likely use axum or actix-web
- The Tauri integration may replace HTTP routes with Tauri commands

**Acceptance Criteria:**
- [ ] Route function signatures defined
- [ ] Doc comments reference the API spec
- [ ] `cargo check` passes

---

**Rust API pattern:** Use the struct-based repository pattern (as shown in `refs/examples/rs-stubs/source-repository.rs`), NOT the free-function pattern shown in the spec. The struct pattern allows dependency injection (database connection in the struct) which mirrors the TS service instantiation pattern.

## Parallelization

Tasks 8.2-8.5 depend on 8.1 (type definitions). After 8.1, they can run in parallel:

```
Task 8.1 (types) ──┬──► Task 8.2 (repos)
                    ├──► Task 8.3 (services)
                    ├──► Task 8.4 (AI)
                    └──► Task 8.5 (routes)
```

However, 8.1 should be done first as other stubs depend on the type definitions.

## Testing

- `cargo check --workspace` passes — that's the only test for stubs
- No unit tests (nothing to test — all bodies are `todo!()`)

## Documentation

- Each stub file IS the documentation — doc comments describe the contract
- `docs/src/architecture/rust-migration.md` — notes on what to consider during the rewrite
