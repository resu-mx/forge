# Phase 1: Shared Types & Database Schema

**Goal:** Define all TypeScript types and create the initial SQLite schema. This phase produces the contracts that every other phase builds against.

**Non-Goals:** No business logic, no HTTP routes, no AI integration. Types and schema only.

**Depends on:** Phase 0 (monorepo exists)
**Blocks:** Phase 2 (repos), Phase 3 (services), Phase 4 (routes), Phase 5 (SDK types)

---

## Task 1.1: Shared Type Definitions

**Goal:** Create the TypeScript type definitions shared across core and SDK.

**File:** `packages/core/src/types/index.ts`

**Reference:** `refs/contracts/entity-types.md`

**Steps:**
1. Define all entity interfaces (Source, Bullet, Perspective, Resume, etc.)
2. Define all input types (CreateSource, UpdateSource, DerivePerspectiveInput, etc.)
3. Define rich response types (SourceWithBullets, PerspectiveWithChain, etc.)
4. Define status enums as const objects + type unions (not TS enums — they don't exist at runtime in the way we need)
5. Define the Result<T>, PaginatedResult<T>, Pagination, and ForgeError types (shared with SDK)
6. Define ReviewQueue type for the review endpoint
7. Export everything from a barrel file

**Type design decisions:**
- Use `string` for UUIDs (not a branded type for MVP — add later if desired)
- Use string literal unions for enums: `type SourceStatus = 'draft' | 'approved' | 'deriving'`
- Nullable fields use `T | null`, not `T | undefined`
- Dates are ISO 8601 strings, not Date objects (JSON serialization)

**Acceptance Criteria:**
- [ ] All entity types match the spec's data model section
- [ ] All input types cover every POST/PATCH endpoint's request body
- [ ] Result<T>, PaginatedResult<T>, Pagination, and ForgeError match `refs/contracts/result-type.md`
- [ ] ReviewQueue type defined for review endpoint
- [ ] TypeScript compiles with `strict: true`
- [ ] Types are exported and importable from `@forge/core/types` (or re-exported)

**Failure Criteria:**
- Type definitions diverge from spec → fix types, not spec (spec is source of truth)

**Testing:**
- Type-check: `tsc --noEmit` passes
- Doc test: each type has a JSDoc comment with a one-line description

**Documentation:** `docs/src/data/models/entity-types.md` — mirror of the type file with explanations

---

## Task 1.2: SQLite Schema (Migration 001)

**Goal:** Create the initial database schema as a migration file.

**File:** `packages/core/src/db/migrations/001_initial.sql`

**Reference:** `refs/schemas/001-initial.sql` (complete SQL)

**Steps:**
1. Copy the reference schema into the migration file
2. Verify all CHECK constraints match the type definitions from Task 1.1
3. Verify all indexes match `refs/strategies/indexing.md`
4. Verify FK cascade rules match `refs/schemas/erd.md`

**NOTE:** The reference schema (`refs/schemas/001-initial.sql`) does NOT contain PRAGMAs. PRAGMAs are set by the connection helper (Task 2.1), not inside migration files. SQLite ignores `PRAGMA foreign_keys` inside transactions. The migration filename is `001_initial.sql` (underscore, not hyphen).

**Acceptance Criteria:**
- [ ] Schema creates all tables: employers, projects, sources, bullets, bullet_technologies, perspectives, skills, bullet_skills, perspective_skills, resumes, resume_perspectives, prompt_logs, _migrations
- [ ] All entity tables use STRICT mode (`_migrations` is intentionally non-STRICT for AUTOINCREMENT compatibility)
- [ ] All UUID PKs have CHECK constraint for length 36
- [ ] All enum fields have CHECK constraints matching type definitions
- [ ] All FK relationships have correct ON DELETE behavior
- [ ] All indexes from the indexing strategy are present
- [ ] `_migrations` table is created and `001_initial` is inserted

**Failure Criteria:**
- SQLite rejects the schema → fix SQL syntax
- CHECK constraints don't match type enums → align schema with types

**Testing:**
- Unit: Load schema into in-memory SQLite (`:memory:`), verify all tables exist
- Unit: Attempt to insert a row violating each CHECK constraint — verify rejection
- Unit: Attempt to delete a source with bullets — verify RESTRICT blocks it
- Unit: Attempt to insert a bullet with nonexistent source_id — verify FK violation

**Documentation:** `docs/src/data/models/schema.md` — table descriptions, index rationale, FK rules

---

## Task 1.3: Migration Runner

**Goal:** Build the migration runner that applies SQL files in order.

**File:** `packages/core/src/db/migrate.ts`

**Reference:** `refs/strategies/db-migration.md`

**Steps:**
1. Implement `runMigrations(db: Database, migrationsDir: string)`
2. Create `_migrations` table if not exists
3. Read `.sql` files from directory, sorted by name
4. For each file not in `_migrations`: begin transaction, execute SQL, record in `_migrations`, commit
5. On failure: rollback transaction, log error with migration name, throw

**Acceptance Criteria:**
- [ ] Fresh database: all migrations applied in order
- [ ] Existing database: only new migrations applied
- [ ] Already up-to-date: no-op, logs "All migrations applied"
- [ ] Failed migration: rolls back cleanly, database is not corrupted
- [ ] PRAGMA foreign_keys = ON is set before migrations run
- [ ] PRAGMA journal_mode = WAL is set

**Failure Criteria:**
- Migration partially applies (some tables created, others not) → ensure transaction wraps entire migration file

**Fallback Strategy:** If a migration file contains multiple statements and SQLite can't wrap them in a single transaction, split into multiple `exec()` calls within the same transaction.

**Testing:**
- Unit: Apply 001 to fresh `:memory:` db, verify all tables
- Unit: Apply 001 twice, verify idempotent (second run is no-op)
- Unit: Create a broken 002 migration, verify 001 is applied but 002 rolls back
- Integration: `just migrate` runs from CLI and applies to file-based db

**Documentation:** `docs/src/lib/migrations.md` — how to add a new migration, naming convention, rollback procedures

---

---

## Task 1.4: Archetype/Domain Constants

**Goal:** Create application constants for archetype→expected-domains mappings used by gap analysis.

**File:** `packages/core/src/constants/archetypes.ts`

**Reference:** `refs/taxonomy/archetypes.md`, `refs/taxonomy/domains.md`, `refs/examples/gap-analysis/algorithm.md`

**Steps:**
1. Define `ARCHETYPES` constant with all 6 archetype values
2. Define `DOMAINS` constant with all 6 domain values
3. Define `ARCHETYPE_EXPECTED_DOMAINS` map:
   ```typescript
   export const ARCHETYPE_EXPECTED_DOMAINS: Record<string, string[]> = {
     'agentic-ai': ['ai_ml', 'software_engineering', 'leadership'],
     'infrastructure': ['systems_engineering', 'devops', 'software_engineering'],
     'security-engineer': ['security', 'systems_engineering', 'devops'],
     'solutions-architect': ['systems_engineering', 'software_engineering', 'leadership'],
     'public-sector': ['security', 'systems_engineering', 'leadership'],
     'hft': ['systems_engineering', 'software_engineering'],
   }
   ```
4. Define `THIN_COVERAGE_THRESHOLD = 2`
5. Define `RESUME_SECTIONS` constant with valid section values

**Acceptance Criteria:**
- [ ] All 6 archetypes from `refs/taxonomy/archetypes.md` are present
- [ ] All 6 domains from `refs/taxonomy/domains.md` are present
- [ ] Expected domains map covers every archetype
- [ ] Constants are importable from `@forge/core`
- [ ] TypeScript compiles with `strict: true`

**Testing:**
- Type-check: `tsc --noEmit` passes
- Unit: Every archetype key exists in the expected domains map

---

## Parallelization

Tasks 1.1, 1.2, and 1.4 can be developed simultaneously — they reference the same spec but produce different artifacts (TS types vs SQL vs constants). Task 1.3 depends on Task 1.2 (needs the migration file to test against).

```
Task 1.1 (types) ──────────────┐
Task 1.4 (constants) ──────────┤
                                ├──► Phase 2
Task 1.2 (schema) ──► Task 1.3 (runner) ──┘
```
