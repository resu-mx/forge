# Graph DB Storage Abstraction — Design Exploration

**Status:** Phases 0-2 COMPLETE (2026-04-15). Phase 0 (adapter interface), Phase 1 (SQLite adapter + ELM rewiring), Phase 2 (HelixDB adapter + codegen + migration + Docker). Phases 3-5 (GraphQLite, DuckPGQ, benchmarks) pending.

**Specs:** `.claude/plans/forge-resume-builder/refs/specs/2026-04-08-storage-abstraction-spike-design.md`, `2026-04-14-helix-adapter-design.md`
**Plan:** `.claude/plans/forge-resume-builder/phase/2026-04-14-helix-adapter.md`
**Code:** `packages/core/src/storage/` (adapter interface, ELM, SQLite adapter, HelixDB adapter, migration)

## Decision Context

Forge currently uses SQLite with 13 junction tables for many-to-many relationships. Exploring graph DB options for better query ergonomics, vector integration, and future SaaS path.

## Evaluation Matrix

### HelixDB — Production / SaaS target
- Compile-time checked schemas (HQL) — security + data assurance for multi-tenant
- Native graph+vector (HNSW) — semantic↔graph lookups in single query
- Rust-native with TS SDK — aligns with WASM edge client vision
- Open source (MIT) — can fork for custom embedding/tokenization/algorithm control
- 5-20x faster than production-grade alternatives (benchmarked)
- Tradeoff: separate server process, early-stage company, proprietary query language

### GraphQLite — OSS / Self-hosted target
- Same SQLite file — zero infra, single-file deployment
- Cypher (industry standard) — familiar to graph community
- Can pair with sqlite-vec for vector search
- Tradeoff: schemaless EAV (no enforcement), no TS SDK, pre-1.0

### DuckPGQ — Analytics / R/O views
- ATTACH to SQLite — zero migration for read path
- SQL/PGQ (ISO SQL:2023) — standards-based
- Potential: CF CDN / R2 Glacier for read replicas
- Tradeoff: read-only, inherits relational structure (doesn't clean up junctions), no Rust SDK

## Architecture Direction

Abstract storage behind SDK adapter interface:

```
┌──────────────────────────────┐
│  Application Logic (MCP/API) │
├──────────────────────────────┤
│  SDK (Storage Adapter IF)    │
├──────┬──────────┬────────────┤
│SQLite│ HelixDB  │ DuckPGQ    │
│+GQL  │ (prod)   │ (analytics)│
│(oss) │          │            │
└──────┴──────────┴────────────┘
```

### Adapter Interface (rough shape)
- CRUD: create/read/update/delete entities by type
- Graph: traverse(from, edge_type, to_type, depth), shortest_path, subgraph
- Vector: embed(content), similar(query, entity_type, top_k, threshold)
- Analytics: aggregate, score, align (could be DuckPGQ-specific)

### Migration Strategy
1. ~~Define adapter interface in SDK~~ — **DONE** (Phase 0, 2026-04-10)
2. ~~Implement SQLite adapter (current behavior, wraps existing repos)~~ — **DONE** (Phase 1, 2026-04-12)
3. ~~Implement HelixDB adapter (graph+vector, typed schemas)~~ — **DONE** (Phase 2, 2026-04-15)
4. GraphQLite adapter (Cypher overlay on SQLite, for OSS builds) — pending
5. DuckPGQ adapter (analytics-only, read path) — pending

## Open Questions
- Can adapters compose? (SQLite for writes + DuckPGQ for reads + HelixDB for graph+vector)
- How to handle schema migrations across adapters?
- WASM edge client: which adapter compiles to WASM? (HelixDB Rust → WASM viable?)
- Vector algo selection: HNSW vs what alternatives for lighter datasets?
- PGQ query power: need deeper comparison of path quantifiers, aggregation, subqueries

## Lenses
1. **Single user → SaaS**: HelixDB compile-time schemas are the clearest win for multi-tenant
2. **TS → Rust**: HelixDB has both; GraphQLite is C (FFI from Rust is fine); DuckPGQ has no Rust SDK
3. **Extensibility**: HelixDB is OSS/forkable; GraphQLite is MIT; DuckPGQ is MIT
4. **Query power**: HQL graph+vector integration > Cypher (graph only) > PGQ (SQL extension)
