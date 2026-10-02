# Storage Abstraction & Graph DB Spike — Design Document

**Status:** Draft / Research
**Created:** 2026-04-08
**Scope:** Multi-phase design spike for abstracting Forge's storage layer and evaluating graph+vector DB options

## Motivation

Forge has 47 tables, 13 junction tables, and multi-hop query patterns (source→skills→JDs, bullet→perspective→resume_entry) that are increasingly complex in relational SQL. The current SQLite-only architecture works for single-user but does not support:

- SaaS multi-tenant with schema-level safety guarantees
- Native graph traversals without N-way JOINs
- Integrated vector+graph queries (semantic search informed by relationship structure)
- Flexible deployment (embedded for OSS, server-mode for prod, edge for CDN)

## Guiding Principles

1. **Pluggable backends** — OSS users get SQLite simplicity; production gets HelixDB power
2. **Two-way migration** — datasets must round-trip between backends for benchmarking/experimentation
3. **No application logic changes** — adapter interface absorbs all storage differences
4. **Incremental adoption** — each phase is independently useful and testable
5. **Data sovereignty clarity** — every entity has a defined home (user-scoped, global, private)

## Target Backends

| Backend | Role | Query Language | Deployment |
|---|---|---|---|
| SQLite (current) | Baseline / OSS self-hosted | SQL | Embedded file |
| GraphQLite | Graph overlay for OSS / testing | Cypher via `SELECT cypher(...)` | SQLite extension |
| HelixDB | Production graph+vector | HQL (compiled, type-checked) | Server (Docker) |
| DuckPGQ | Read-only analytics | SQL/PGQ (ISO SQL:2023) | DuckDB process, ATTACHes SQLite |

---

## Phase 0: Adapter Interface Design

**Goal:** Define the storage abstraction boundary in the SDK.

### Scope
- Define `StorageAdapter` interface with clear separation of concerns
- Split into composable sub-interfaces:
  - `EntityStore` — CRUD by entity type (create, read, update, delete, list, search)
  - `GraphStore` — traversals, path queries, subgraph extraction
  - `VectorStore` — embed, similar, nearest neighbors, staleness checks
  - `AnalyticsStore` — aggregation, scoring, alignment (read-only)
- Define `AdapterCapabilities` enum so callers can feature-detect (not all backends support all operations)
- Define serialization contract for entities (JSON-based, schema-versioned)
- Define migration/export format (portable between backends)

### Key Decisions
- Where does the interface live? (`packages/sdk/src/storage/`)
- How do adapters compose? (e.g., SQLite writes + DuckPGQ reads)
- Transaction semantics across composed adapters
- Error handling: adapter-specific errors vs generic storage errors

### Deliverables
- `StorageAdapter` TypeScript interface + sub-interfaces
- `AdapterCapabilities` type
- Entity serialization schema (JSON Schema or Zod)
- Portable export/import format spec (for two-way migration)
- Decision record: composition model (CQRS-lite vs unified vs routing)

### Dependencies
- None (foundational)

---

## Phase 1: SQLite Adapter (Baseline)

**Goal:** Wrap existing Forge repos/services behind the adapter interface with zero behavior change.

### Scope
- Implement `SqliteAdapter` conforming to `StorageAdapter`
- `EntityStore` — wraps existing repository classes
- `GraphStore` — implemented as multi-JOIN SQL queries (current behavior, explicit)
- `VectorStore` — wraps existing `EmbeddingService` (brute-force cosine)
- `AnalyticsStore` — wraps existing alignment/scoring queries
- All existing tests must pass unchanged
- Adapter is the new entry point; direct repo access deprecated

### Deliverables
- `SqliteAdapter` implementation
- Adapter integration tests (round-trip CRUD for all 47 tables)
- Migration: rewire `packages/core/` routes to use adapter instead of raw repos

### Dependencies
- Phase 0 (interface)

---

## Phase 2: HelixDB Adapter + Two-Way Migration

**Goal:** Implement HelixDB backend with bidirectional data migration for experimentation.

### Scope

#### 2a: HQL Schema Design
- Map all 47 Forge tables to HelixDB node/edge types
- Nodes: sources, bullets, perspectives, skills, organizations, resumes, JDs, contacts, credentials, certifications, summaries, user_profile, user_notes, prompt_logs, embeddings, pending_derivations, etc.
- Edges: replaces all 13 junction tables + FK relationships (typed, with properties where junction tables carry metadata like `is_primary`, `position`, `relationship`)
- Leverage HQL compile-time schema validation
- Document type-specific constraints that move from SQL CHECK to HQL schema

#### 2b: Adapter Implementation
- Implement `HelixAdapter` conforming to `StorageAdapter`
- `EntityStore` — HQL node CRUD
- `GraphStore` — native HQL traversals (the whole point)
- `VectorStore` — native HNSW via HelixDB's `V::` type and `Embed()`
- `AnalyticsStore` — HQL aggregation queries
- Connection management (helix-ts HTTP client)
- Error mapping (HQL errors → generic storage errors)

#### 2c: Two-Way Migration
- `sqlite-to-helix`: read all entities via SQLiteAdapter, write via HelixAdapter
- `helix-to-sqlite`: read all entities via HelixAdapter, write via SQLiteAdapter
- Verification: round-trip integrity check (entity counts, FK consistency, content hashes)
- Scripted and repeatable (`just migrate-to-helix`, `just migrate-to-sqlite`)

### Deliverables
- HQL schema files (`.hx`)
- `HelixAdapter` implementation
- Migration scripts (both directions)
- Round-trip integrity test suite
- Docker Compose for local HelixDB dev instance

### Dependencies
- Phase 0 (interface), Phase 1 (SQLite adapter for migration source)

---

## Phase 3: GraphQLite Adapter

**Goal:** Cypher-based graph overlay on existing SQLite file for OSS/self-hosted users.

### Scope
- Implement `GraphQLiteAdapter` conforming to `StorageAdapter`
- `EntityStore` — delegates to SQLite (relational tables are the source of truth)
- `GraphStore` — Cypher queries via `SELECT cypher(...)` over GraphQLite extension
- `VectorStore` — delegates to sqlite-vec extension (if available) or falls back to brute-force
- `AnalyticsStore` — delegates to SQLite
- Extension loading: `db.loadExtension("graphqlite")` with macOS `setCustomSQLite()` handling
- Graph sync: mechanism to mirror relational FK/junction data as GraphQLite edges (on-write hooks or periodic sync)

### Key Decisions
- Sync strategy: eager (write-through to both relational + graph) vs lazy (periodic rebuild)
- How to handle GraphQLite's schemaless EAV in a typed codebase (validation layer?)
- sqlite-vec integration: load both extensions on same connection?

### Deliverables
- `GraphQLiteAdapter` implementation
- Extension loading + platform handling (macOS, Linux, CI)
- Graph sync mechanism
- Cypher query library for Forge's common traversal patterns

### Dependencies
- Phase 0 (interface), Phase 1 (SQLite adapter for delegation)

---

## Phase 4: DuckPGQ Adapter

**Goal:** Read-only analytics and graph query layer over existing SQLite data.

### Scope
- Implement `DuckPgqAdapter` conforming to `StorageAdapter` (read-only subset)
- `EntityStore` — read-only (ATTACH SQLite, query through DuckDB)
- `GraphStore` — `CREATE PROPERTY GRAPH` over attached tables, SQL/PGQ pattern matching
- `VectorStore` — not implemented (or delegate to DuckDB `vss` extension if viable)
- `AnalyticsStore` — DuckDB columnar engine for aggregation, scoring, coverage analysis
- Validate: does `CREATE PROPERTY GRAPH` work over ATTACH'd SQLite tables? (critical unknown)

### Key Decisions
- Property graph definition: which tables are vertices, which are edges?
- Deployment: embedded in API process or separate analytics service?
- CDN/R2 angle: can a DuckDB file be served read-only from Cloudflare R2/Glacier?

### Deliverables
- `DuckPgqAdapter` implementation (read-only)
- Property graph DDL for Forge schema
- Validation: ATTACH'd SQLite interop test
- Deployment exploration doc (embedded vs service vs CDN)

### Dependencies
- Phase 0 (interface)

---

## Phase 5: Benchmarking & Experimentation Framework

**Goal:** Repeatable benchmark suite for comparing backends across Forge's real query patterns.

### Scope

#### 5a: Mock Data Generator
- Generate synthetic Forge datasets at multiple scales:
  - **Small**: ~100 sources, ~500 bullets, ~1K perspectives, ~50 skills, ~10 JDs, ~5 resumes (current Forge)
  - **Medium**: ~1K sources, ~5K bullets, ~10K perspectives, ~500 skills, ~100 JDs, ~50 resumes (active user)
  - **Large**: ~10K sources, ~50K bullets, ~100K perspectives, ~5K skills, ~1K JDs, ~500 resumes (SaaS multi-tenant)
  - **XL**: ~100K sources, ~500K bullets, ~1M perspectives, ~50K skills, ~10K JDs, ~5K resumes (stress test)
- Realistic distribution: skills follow power law, bullets per source follow normal, junction density ~3-5 edges per entity
- Deterministic seeding for reproducibility

#### 5b: Query Benchmark Suite
- Define canonical queries representing Forge's core operations:
  1. **CRUD**: single entity create/read/update/delete (latency, throughput)
  2. **List + filter**: paginated list with status/type filters
  3. **2-hop traversal**: "skills for this source" (source→source_skills→skills)
  4. **3-hop traversal**: "JDs matching this source's skills" (source→skills→jd_skills→JDs)
  5. **N-hop traversal**: `forge_trace_chain` (resume_entry→perspective→bullet→source, with skills at each level)
  6. **Aggregation**: "skill coverage score for resume vs JD" (count matching/total)
  7. **Vector similarity**: "top 10 bullets similar to this JD requirement"
  8. **Graph+vector**: "top 10 bullets similar to this JD requirement that are also linked to skills the JD requires"
  9. **Write-heavy**: batch create 100 bullets with skills + sources (transaction throughput)
  10. **Concurrent reads**: 10/50/100 simultaneous read queries (SaaS simulation)

#### 5c: Benchmark Harness
- Runs each query N times per backend per scale
- Records: p50/p95/p99 latency, throughput, memory usage, index build time
- Output: JSON results + comparison tables + charts
- `just bench [backend] [scale] [query]` CLI

### Deliverables
- Mock data generator (deterministic, multi-scale)
- Query benchmark suite (10 canonical queries)
- Benchmark harness with reporting
- Baseline results for SQLite adapter

### Dependencies
- Phase 0-4 (all adapters for comparison), but can start mock data generator independently

---

## Phase 6: Data Model Analysis & Trade-off Testing

**Goal:** Determine where each class of data belongs and what constraints govern it.

### Scope

#### 6a: Data Classification
Classify every Forge entity by:

| Dimension | Values |
|---|---|
| **Ownership** | User-scoped (my bullets, my resumes) vs Global (skill taxonomy, org directory) vs Private (salary, clearance) |
| **Mutability** | Immutable (prompt_logs, embeddings) vs Mutable (sources, resumes) vs Append-only (user_notes) |
| **Access pattern** | Write-heavy (derivation flow) vs Read-heavy (resume rendering) vs Mixed (CRUD pages) |
| **Sensitivity** | PII (user_profile, contacts) vs Confidential (credentials, salary) vs Public (skills, orgs) |
| **Staleness tolerance** | Real-time (resume editing) vs Eventually consistent (analytics, alignment scores) vs Cached (skill taxonomy) |

#### 6b: Backend Affinity Mapping
Map each data class to its natural backend:
- Real-time mutable user data → HelixDB (or SQLite for OSS)
- Global taxonomy → shared read replica (DuckPGQ/CDN)
- Analytics/scoring → DuckPGQ
- Vector embeddings → HelixDB native (or sqlite-vec for OSS)
- Audit/logs → append-only store (SQLite or HelixDB)

#### 6c: Constraint Testing
- Test FK cascade behavior across backends
- Test CHECK constraint equivalents in HQL vs Cypher vs SQL/PGQ
- Test unique constraint enforcement
- Test transaction isolation (concurrent writes to same entity)
- Test data integrity after two-way migration at each scale

### Deliverables
- Data classification matrix (all 47 tables)
- Backend affinity map
- Constraint compatibility matrix (SQL vs HQL vs Cypher vs PGQ)
- Test results for integrity/isolation

### Dependencies
- Phase 2-4 (adapters), Phase 5 (mock data)

---

## Phase 7: Deployment Pattern Exploration

**Goal:** Map storage architecture to deployment targets.

### Scope
- **OSS/Self-hosted**: Single SQLite file + optional GraphQLite extension. Zero infra.
- **Dev/Staging**: SQLite + HelixDB (Docker Compose). Two-way sync for experimentation.
- **SaaS/Production**: HelixDB primary (graph+vector) + DuckPGQ analytics sidecar. Multi-tenant isolation.
- **Edge/CDN**: DuckDB read replica on CF R2. Precomputed analytics. Served via CF Workers.
- **WASM Client**: Thin client speaking HQL over HTTP to HelixDB server. Local cache in IndexedDB or Origin Private File System.

### Key Questions
- Multi-tenant in HelixDB: database-per-tenant vs namespace isolation?
- DuckDB on R2: what's the read latency for a 50MB analytics DB?
- WASM: can helix-ts client bundle to <100KB?
- Schema migrations: how to coordinate across HelixDB + SQLite + DuckDB?

### Deliverables
- Deployment architecture diagrams (per target)
- Cost estimates (HelixDB Cloud vs self-hosted vs CF Workers)
- Migration coordination strategy
- Decision record: recommended production topology

### Dependencies
- Phase 5 (benchmarks), Phase 6 (data model analysis)

---

## Phase 8: Graph Algorithm Benchmarking

**Goal:** Evaluate graph algorithms against Forge's data model at multiple scales.

### Scope
- **Algorithms to test:**
  - Shortest path (source→JD via skills)
  - PageRank (identify most-connected skills, highest-value sources)
  - Community detection / Louvain (skill clusters, experience groupings)
  - Connected components (orphan detection — entities with no relationships)
  - BFS/DFS traversal (trace_chain, dependency walks)
  - Betweenness centrality (which skills are bridges between domains?)
  - Subgraph extraction (pull all entities relevant to a specific resume)

- **Per algorithm:**
  - Correctness verification against known-good SQLite JOIN results
  - Latency at Small/Medium/Large/XL scales
  - Memory usage
  - Compare: HelixDB native vs GraphQLite Cypher vs DuckPGQ SQL/PGQ vs raw SQLite JOINs

### Deliverables
- Algorithm benchmark results (per backend, per scale)
- Recommendation: which algorithms to use in production, which backend serves them best
- Forge-specific findings (e.g., "PageRank over skills reveals X insight about resume optimization")

### Dependencies
- Phase 2-4 (adapters), Phase 5 (mock data + harness)

---

## Phase 9: Vector Algorithm Benchmarking

**Goal:** Evaluate vector search algorithms against Forge's embedding workload.

### Scope
- **Algorithms to test:**
  - Brute-force cosine similarity (current Forge implementation)
  - HNSW (HelixDB native, sqlite-vec)
  - IVF-Flat (if available in any backend)
  - Product Quantization (PQ) — if available

- **Dimensions to measure:**
  - Index build time
  - Query latency (single query, batch)
  - Recall@10, Recall@50 (vs brute-force ground truth)
  - Memory usage (index size)
  - Staleness handling (incremental index update vs full rebuild)

- **Scales:**
  - Small: ~500 embeddings (current Forge)
  - Medium: ~10K embeddings
  - Large: ~100K embeddings
  - XL: ~1M embeddings

- **Forge-specific queries:**
  - "Top 10 bullets similar to this JD requirement" (single query)
  - "Alignment matrix: all JD requirements vs all resume perspectives" (batch)
  - "Stale embedding detection" (content hash mismatch scan)

### Deliverables
- Vector benchmark results (per algorithm, per scale)
- Recall vs latency tradeoff curves
- Recommendation: which algorithm at which scale
- Index size projections for SaaS (per-tenant embedding counts)

### Dependencies
- Phase 2 (HelixDB vector), Phase 5 (mock data + harness)

---

## Phase 10: Graph + Vector Combined Benchmarking

**Goal:** Evaluate hybrid graph+vector queries that are the core differentiator for HelixDB.

### Scope
- **Combined query patterns:**
  1. "Find bullets semantically similar to JD requirement X that are also linked to skills the JD requires" (vector filter → graph filter)
  2. "Find sources whose descriptions are similar to JD text AND whose skills overlap with JD skills" (vector + graph intersection)
  3. "Rank resumes for a JD by combined graph proximity (skill overlap) + vector similarity (content match)" (hybrid scoring)
  4. "Suggest skills to add based on semantically similar bullets from other sources" (vector → graph expansion)
  5. "Detect resume gaps: JD requirements with no similar bullets AND no matching skills" (vector + graph negation)

- **Per query:**
  - Compare: HelixDB native (single-query graph+vector) vs SQLite (two-phase: vector first, then JOIN) vs composed adapter (vector from one backend, graph from another)
  - Latency, accuracy, code complexity
  - Does single-system graph+vector actually outperform two-phase?

### Deliverables
- Combined benchmark results
- Architecture recommendation: single-system (HelixDB) vs composed (SQLite+vec + GraphQLite) vs hybrid
- Decision record: is HelixDB's graph+vector integration worth the migration cost?

### Dependencies
- Phase 8 (graph benchmarks), Phase 9 (vector benchmarks)

---

## Phase Dependencies (DAG)

```
Phase 0 (Adapter Interface)
├── Phase 1 (SQLite Adapter)
│   ├── Phase 2 (HelixDB Adapter + Migration)
│   ├── Phase 3 (GraphQLite Adapter)
│   └── Phase 4 (DuckPGQ Adapter)
├── Phase 5 (Benchmark Framework) ← can start mock data independently
│   ├── Phase 8 (Graph Algo Benchmarks) ← needs 2,3,4
│   ├── Phase 9 (Vector Algo Benchmarks) ← needs 2
│   └── Phase 10 (Combined Benchmarks) ← needs 8,9
├── Phase 6 (Data Model Analysis) ← needs 2,3,4,5
└── Phase 7 (Deployment Patterns) ← needs 5,6
```

## Open Questions

1. Can HelixDB's LMDB storage handle concurrent writes from multiple API server instances (SaaS)?
2. Does DuckPGQ's `CREATE PROPERTY GRAPH` work over ATTACH'd SQLite tables?
3. Can GraphQLite + sqlite-vec coexist on the same SQLite connection?
4. What's the HQL equivalent of SQL's ON DELETE CASCADE / RESTRICT?
5. How does HelixDB handle schema migrations for deployed production data?
6. Is there a portable graph serialization format (GraphML, JSON-LD, RDF) that all three backends can import/export?
7. What's the minimum viable adapter interface that supports all four backends without lowest-common-denominator constraints?
8. WASM: can helix-ts or helix-rs compile to a <100KB WASM bundle for edge deployment?
