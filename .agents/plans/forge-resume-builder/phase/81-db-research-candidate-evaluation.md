# Phase 81: Database Research — Candidate Deep Dives

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans

**Depends on:** Phase 80 (evaluation rubric and candidate shortlist)
**Parallel with:** UI bug fixes, any non-DB work
**Duration:** Medium (6 tasks: T81.1 through T81.6, research + light prototyping)

## Goals

- Perform deep-dive evaluation of each shortlisted database candidate against the rubric from Phase 80
- Test graph query expressiveness with Forge's actual derivation chain schema
- Test vector search capability with Forge's 384-dimensional embeddings
- Verify edge/WASM deployment feasibility with proof-of-concept builds
- Score each candidate and produce a ranked comparison matrix

## Non-Goals

- Making the final decision (Phase 83)
- Full migration prototyping (Phase 83)
- Performance benchmarking at scale (out of scope — Forge's data volumes are small)
- Evaluating databases that failed hard constraints in Phase 80

## Context

Phase 80 produced an evaluation rubric, a candidate shortlist, and Forge-specific requirements. This phase applies those tools to each candidate. The two primary candidates are Turso (libSQL) and DuckDB, but the shortlist may include SurrealDB, PGlite, CozoDB, or others.

Each candidate gets the same evaluation: graph queries, vector search, edge deployment, and a quick prototype using Forge's actual schema.

---

## Tasks

### T81.1: Turso / libSQL Deep Dive

**Goal:** Evaluate Turso against all rubric dimensions.

**Steps:**
1. **Graph:** Research GraphQLite extension status — is it production-ready? What query language? Can it express Forge's derivation chain traversals? If no graph extension, how painful are recursive CTEs in libSQL?
2. **Vector:** Test native vector column support (`F32_BLOB` type). Create a test table with 384-dim vectors, insert sample data, run cosine similarity queries. Measure query syntax ergonomics.
3. **Edge:** Document Turso's embedded replica model (edge replicas sync from primary). Test with Cloudflare Workers — does the `@libsql/client` work in Workers runtime? Cold start time? Memory footprint?
4. **Client-side:** Test libSQL WASM build in browser. Does `@libsql/client` have a WASM target? Can it operate offline with periodic sync?
5. **Distribution:** Document Turso's multi-region replication, read replicas, embedded replicas. Conflict resolution model? Write latency?
6. **Migration path:** How close is libSQL to SQLite? Can Forge's 29 migrations run unmodified? What breaks?
7. **Ergonomics:** Evaluate TypeScript SDK, documentation, community, pricing (free tier limits)

**Deliverable:** `refs/research/db-research/81-turso-evaluation.md` with rubric scores

### T81.2: DuckDB Deep Dive

**Goal:** Evaluate DuckDB against all rubric dimensions.

**Steps:**
1. **Graph:** Research DuckPGQ extension — is it stable? Does it support property graph queries over existing tables? Can it express Forge's multi-hop derivation chain? What's the query syntax (SQL/PGQ)?
2. **Vector:** Research DuckDB vector support — `vss` extension? Array columns with distance functions? ANN index support? Compare with Turso's native vectors.
3. **Edge:** Test DuckDB WASM build. Does `@duckdb/duckdb-wasm` work in Cloudflare Workers? Memory constraints? Cold start? DuckDB is OLAP-oriented — is it suitable for Forge's OLTP-like workload (frequent small writes)?
4. **Client-side:** DuckDB-WASM is well-established in browsers. Test with Forge's schema. Offline capability?
5. **Distribution:** DuckDB is single-process. Research sync options — MotherDuck (cloud DuckDB), manual export/import, change data capture patterns.
6. **Migration path:** DuckDB SQL dialect differences from SQLite. Can Forge's migrations run? What needs rewriting?
7. **Ergonomics:** Evaluate `duckdb` npm package, documentation, community. Note: DuckDB excels at analytics — is Forge's workload a fit?

**Deliverable:** `refs/research/db-research/81-duckdb-evaluation.md` with rubric scores

### T81.3: Alternative Candidate Deep Dives

**Goal:** Evaluate remaining shortlisted candidates (from T80.3) at the same depth.

**Steps:**
1. For each remaining candidate from the Phase 80 shortlist, evaluate against all rubric dimensions
2. Spend proportional effort — if a candidate clearly fails early dimensions, note why and move on
3. Candidates likely to appear:
   - **SurrealDB**: Multi-model (document + graph + embedded). GRAPH traversals, vector search planned/shipped. Rust core, WASM target. Evaluate maturity and stability.
   - **PGlite**: Postgres in WASM (electric-sql/pglite). Inherits pgvector + Apache AGE (graph). Full Postgres compatibility. Evaluate WASM bundle size, cold start, sync story.
   - **CozoDB**: Embeddable Datalog DB with graph traversal and vector search. Rust core, WASM build. Evaluate query language learning curve and TypeScript integration.
4. Score each candidate using the rubric

**Deliverable:** `refs/research/db-research/81-alternatives-evaluation.md` with rubric scores per candidate

### T81.4: Graph Query Prototype

**Goal:** Test real Forge graph queries against the top 2-3 candidates.

**Steps:**
1. Take the graph queries defined in T80.4 (derivation chain traversal, bipartite skill matching, 1-hop neighborhood, orphan detection)
2. Write each query in each candidate's native syntax (Cypher for GraphQLite/SurrealDB, SQL/PGQ for DuckDB, Datalog for CozoDB, SQL+AGE for PGlite, recursive CTEs for libSQL fallback)
3. Compare: readability, expressiveness, number of lines, ease of parameterization
4. Note which queries are impossible or extremely awkward in each candidate

**Deliverable:** `refs/research/db-research/81-graph-query-comparison.md`

### T81.5: Vector Search Prototype

**Goal:** Test Forge's vector workload against the top 2-3 candidates.

**Steps:**
1. Create a test dataset: 200 embeddings (384-dim float32) representing Forge bullets/perspectives/JD requirements
2. For each candidate, implement:
   - Insert embeddings
   - Top-K cosine similarity search
   - Bulk alignment scoring (all bullets vs all JD requirements for a single JD)
   - Staleness check (content hash mismatch)
3. Compare: query syntax, result accuracy, index support (exact vs ANN), storage overhead

**Deliverable:** `refs/research/db-research/81-vector-comparison.md`

### T81.6: Produce Comparison Matrix

**Goal:** Consolidate all evaluations into a single ranked comparison.

**Steps:**
1. Collect rubric scores from T81.1-T81.3
2. Build a comparison table: candidates as rows, rubric dimensions as columns, weighted scores
3. Calculate total weighted score per candidate
4. Identify the top 2 candidates for Phase 82 deep dive
5. Note any surprising findings or disqualifications

**Deliverable:** `refs/research/db-research/81-comparison-matrix.md`
