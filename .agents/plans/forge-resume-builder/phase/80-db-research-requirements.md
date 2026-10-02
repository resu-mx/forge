# Phase 80: Database Research — Requirements & Evaluation Framework

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans

**Depends on:** None (research phase, no code dependencies)
**Parallel with:** UI bug fixes, any non-DB work
**Duration:** Small (4 tasks: T80.1 through T80.4, research-only)

## Goals

- Audit Forge's current SQLite usage to establish a baseline: query patterns, schema complexity, data volumes, performance characteristics
- Define weighted evaluation criteria for database candidates covering graph queries, vector search, edge/WASM deployment, distribution, and developer ergonomics
- Produce a scoring rubric that subsequent phases use to evaluate each candidate consistently
- Document hard constraints (must-haves) vs soft preferences (nice-to-haves)

## Non-Goals

- Evaluating specific database candidates (Phase 81)
- Making a final database decision (Phase 83)
- Writing migration code or prototypes (Phase 83)
- Benchmarking runtime performance (Phase 81)

## Context

Forge currently uses SQLite via `better-sqlite3` with 29 migrations, ~15 entity tables, and a derivation chain (Source → Bullet → Perspective → Resume Entry) that has graph-like query patterns. Phase 69 added 384-dimensional vector embeddings for cosine similarity alignment. The planned deployment target is Cloudflare (R2 + Workers + Durable Objects + Containers + WASM), which constrains database choices to those that can run on the edge or sync from edge to origin.

The research must answer: does Forge's workload justify moving off SQLite, and if so, which database best fits the graph + vector + edge trifecta?

---

## Tasks

### T80.1: Audit Current SQLite Usage

**Goal:** Catalog every query pattern Forge uses today so candidates can be evaluated against real workload, not hypotheticals.

**Steps:**
1. Grep all repository files in `packages/core/src/db/repositories/` for SQL query patterns
2. Categorize queries: simple CRUD, joins (how many tables?), recursive CTEs, full-text search, aggregation, window functions
3. Identify the derivation chain traversal queries (Source → Bullet → Perspective → Entry) — these are the proto-graph queries
4. Document the embedding queries from `embedding-repository.ts` (cosine similarity, staleness checks)
5. Note any queries that are currently awkward or slow in SQLite
6. Record current DB file size and row counts per table

**Deliverable:** `refs/research/db-research/80-sqlite-audit.md`

### T80.2: Define Evaluation Dimensions

**Goal:** Create a structured rubric so all candidates are scored consistently.

**Steps:**
1. Define evaluation dimensions with weights:
   - **Graph queries** (weight: high) — Can the DB express multi-hop traversals (Source→Bullet→Perspective→Entry) natively? Cypher/SPARQL/GQL support?
   - **Vector search** (weight: high) — Native vector column type? ANN index? Cosine/dot-product similarity? Dimensionality limits?
   - **Edge deployment** (weight: high) — Runs in Cloudflare Workers/WASM? Size constraints? Cold start impact?
   - **Client-side operation** (weight: medium) — Runs in-browser? IndexedDB sync? Offline-first capability?
   - **Distribution/sync** (weight: medium) — Multi-region replication? Conflict resolution? Read replicas?
   - **Migration path from SQLite** (weight: medium) — Schema compatibility? Data migration tooling? ORM support?
   - **Developer ergonomics** (weight: low) — TypeScript SDK quality? Documentation? Community size? Maintenance velocity?
   - **Cost** (weight: low) — Free tier? Per-query pricing? Storage costs?
2. For each dimension, define scoring levels (0-3: unsupported, partial, good, excellent)
3. Define hard constraints (must-pass or candidate is eliminated)

**Deliverable:** `refs/research/db-research/80-evaluation-rubric.md`

### T80.3: Identify Candidate Databases

**Goal:** Compile the initial candidate list beyond Turso and DuckDB.

**Steps:**
1. Start with the two named candidates:
   - **Turso** (libSQL fork of SQLite — edge-native, vector columns, potential GraphQLite extension)
   - **DuckDB** (analytical OLAP — DuckPGQ graph extension, in-process, WASM build)
2. Research and add other candidates that hit ≥2 of the three pillars (graph, vector, edge):
   - SurrealDB (multi-model: document + graph + vector, WASM target)
   - EdgeDB (graph-relational, TypeScript-first, but server-dependent)
   - PGlite (Postgres in WASM — pgvector + Apache AGE graph)
   - CozoDB (Datalog + graph + vector, embeddable, WASM)
   - LanceDB (vector-native, embeddable, but no graph)
   - Milvus Lite / Qdrant (vector-only, likely eliminated by graph requirement)
3. For each candidate, note the claimed capabilities for quick pre-screening
4. Eliminate candidates that fail hard constraints from T80.2

**Deliverable:** `refs/research/db-research/80-candidate-shortlist.md`

### T80.4: Document Forge-Specific Requirements

**Goal:** Translate Forge's architecture into concrete DB requirements.

**Steps:**
1. Document the derivation chain as a graph schema (nodes: Source, Bullet, Perspective, Entry, Resume, JD, Skill, Organization; edges: derives_from, tagged_with, linked_to, belongs_to)
2. List the graph queries Forge would benefit from:
   - "Trace a resume entry back to its original source" (multi-hop path)
   - "Find all bullets that share a skill with this JD requirement" (bipartite matching)
   - "Show the 1-hop neighborhood of a source" (local graph widget, Phase 56)
   - "Detect orphan bullets not used in any resume" (reachability)
3. List the vector queries:
   - "Find top-K similar bullets to this JD requirement" (ANN search)
   - "Score resume alignment against JD" (bulk cosine similarity)
   - "Detect stale embeddings" (content hash comparison)
4. Document Cloudflare deployment constraints:
   - Workers: 128MB memory limit, 10ms CPU (free) / 30s CPU (paid)
   - Durable Objects: single-threaded, SQLite storage built-in
   - Containers: full Linux, but cold start cost
   - R2: object storage, not queryable
5. Document offline/client-side requirements (if any)

**Deliverable:** `refs/research/db-research/80-forge-requirements.md`
