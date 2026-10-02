# Phase 2 — HelixDB Adapter + Two-Way Migration

## Context

Phase 1 of the storage abstraction is complete and merged to `main`.
All 26 services use the EntityLifecycleManager (ELM) for data access.
The raw repository layer is deleted. The codebase is ready for a second
adapter implementation.

## Memory to check before starting

- `project_storage_phase1_8_2026_04_12.md` — Phase 1.8 final state
- `project_storage_abstraction_2026_04_10.md` — Phase 0 architecture
- `project_graph_db_exploration_2026_04_08.md` — HelixDB evaluation context

## Key references

- Storage abstraction design spec:
  `.claude/plans/forge-resume-builder/refs/specs/2026-04-08-storage-abstraction-spike-design.md`
  (Section "Phase 2: HelixDB Adapter + Two-Way Migration")
- Existing HQL schema drafts:
  `.claude/plans/forge-prod-infra/.feats/dbs/helix/schemas/`
  (12 schema files covering nodes for all major entities)
- Existing HQL query drafts:
  `.claude/plans/forge-prod-infra/.feats/dbs/helix/queries.hql`
- Storage layer code:
  `packages/core/src/storage/` (adapter interface, entity map, lifecycle manager)
- Entity map with all 47 entities:
  `packages/core/src/storage/entity-map.data.ts`
- SQLite adapter (reference implementation):
  `packages/core/src/storage/adapters/sqlite-adapter.ts`

## Worktree setup

1. Create worktree at `.claude/worktrees/forge-storage`
2. Branch: `worktree-forge-storage-phase2`
3. Base: `main` (which has all of Phase 0 + Phase 1 merged)

## What Phase 2 delivers

### 2a: HQL Schema Design
- Map all 47 Forge entities to HelixDB node/edge types
- Nodes: core entities (sources, bullets, perspectives, skills, etc.)
- Edges: replace 13 junction tables + FK relationships (typed, with
  properties like `is_primary`, `position`, `relationship`)
- Validate against existing HQL schema drafts in `.feats/dbs/helix/`
- Document type-specific constraints moving from SQL CHECK to HQL schema

### 2b: HelixAdapter Implementation
- Implement `HelixAdapter` conforming to `StorageAdapter` interface
  (see `packages/core/src/storage/adapter.ts`)
- EntityStore: HQL node CRUD
- GraphStore: native HQL traversals
- VectorStore: native HNSW via HelixDB's `V::` type
- Connection management (helix-ts HTTP client)
- Error mapping (HQL errors → StorageError)

### 2c: Two-Way Migration
- `sqlite-to-helix`: read all entities via SqliteAdapter, write via HelixAdapter
- `helix-to-sqlite`: reverse direction
- Verification: round-trip integrity check (entity counts, FK consistency)
- Just targets: `just migrate-to-helix`, `just migrate-to-sqlite`

### 2d: Docker Compose for local HelixDB
- Add HelixDB service to existing `docker-compose.yml`
- Health check and startup ordering
- Dev workflow: `just helix-up`, `just helix-test`

## Approach

Start with brainstorming — HelixDB is early-stage and HQL syntax may
have evolved since the schema drafts were written. Research current
HelixDB docs/SDK before coding. The adapter must conform to the
existing `StorageAdapter` interface (don't change it). The entity map
(`entity-map.data.ts`) defines the contract — the HelixAdapter must
support the same 47 entities with the same field semantics.

## What NOT to do

- Do NOT modify the StorageAdapter interface (adapter.ts)
- Do NOT modify the EntityLifecycleManager (lifecycle-manager.ts)
- Do NOT modify any service files
- Do NOT touch the SQLite adapter
- Do NOT change the entity map
- Focus purely on: new adapter + schema + migration + docker
