# Phase 82: Database Research — Deployment & Architecture Validation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans

**Depends on:** Phase 81 (comparison matrix with top 2 candidates identified)
**Parallel with:** UI bug fixes, any non-DB work
**Duration:** Medium (5 tasks: T82.1 through T82.5, prototyping + architecture)

## Goals

- Validate the top 2 database candidates in Forge's target deployment environment (Cloudflare stack)
- Prototype the data access layer swap to assess migration effort
- Design the sync/replication architecture for edge + origin topology
- Stress-test client-side operation (if applicable) for offline-first scenarios
- Produce architecture diagrams for each candidate's deployment model

## Non-Goals

- Final decision (Phase 83)
- Production migration (future phase post-decision)
- UI changes (the WebUI is DB-agnostic through the SDK)
- Changing the MCP server interface (tools remain the same regardless of DB backend)

## Context

Phase 81 identified the top 2 candidates from the shortlist. This phase takes those candidates and validates them in Forge's real deployment context: Cloudflare Workers + Durable Objects + R2 + Containers. The goal is to answer: "Can this database actually run where Forge needs it to run, and what does the architecture look like?"

---

## Tasks

### T82.1: Cloudflare Workers Compatibility Test

**Goal:** Verify each top candidate runs in Cloudflare Workers.

**Steps:**
1. Create a minimal Cloudflare Worker project (`wrangler init`)
2. For each candidate:
   - Install the candidate's WASM/edge client library
   - Create a simple API that opens the DB, runs a query, returns results
   - Deploy to Cloudflare Workers (or test with `wrangler dev`)
   - Measure: bundle size, cold start time, memory usage, CPU time per request
   - Test with Forge's actual schema (create tables, insert sample data, run representative queries)
3. Document compatibility issues, workarounds, and hard blockers
4. Test with Durable Objects SQLite storage as a comparison baseline

**Deliverable:** `refs/research/db-research/82-cloudflare-workers-test.md`

### T82.2: Edge-Origin Sync Architecture

**Goal:** Design the data sync topology for each candidate.

**Steps:**
1. For each candidate, design the read/write architecture:
   - Where does the primary database live? (Cloudflare Container? External hosted service? Turso cloud?)
   - How do edge replicas sync? (Turso embedded replicas? DuckDB export/import? PGlite sync?)
   - What's the write path? (Proxy to origin? Local write + sync? CRDT?)
   - What's the read latency at edge? (Local replica? Remote query? Cached?)
2. Document consistency guarantees (eventual? strong? causal?)
3. Document conflict resolution for concurrent writes from multiple edge locations
4. Assess operational complexity: what needs to be managed, monitored, backed up?
5. Draw architecture diagrams (ASCII or Mermaid) showing the data flow

**Deliverable:** `refs/research/db-research/82-sync-architecture.md`

### T82.3: Data Access Layer Migration Prototype

**Goal:** Assess how much of Forge's repository layer needs to change for each candidate.

**Steps:**
1. Pick one representative repository (e.g., `bullet-repository.ts` — medium complexity, joins, filters)
2. For each candidate, fork the repository file and adapt it to the candidate's query syntax and driver API
3. Measure:
   - Lines changed vs total lines
   - Query syntax differences (parameterization, type coercion, RETURNING clauses)
   - Driver API differences (sync vs async, connection pooling, transaction model)
   - Type safety (does the candidate's TypeScript SDK provide typed results?)
4. Estimate total migration effort: how many of Forge's ~15 repositories need changes? How deep?
5. Assess whether an adapter/abstraction layer could minimize changes

**Deliverable:** `refs/research/db-research/82-dal-migration-prototype.md`

### T82.4: Client-Side / Offline Operation Test

**Goal:** Validate in-browser database operation for offline-first scenarios.

**Steps:**
1. For each candidate with WASM support:
   - Create a minimal Vite/Svelte app that loads the DB in-browser
   - Insert Forge sample data (sources, bullets, perspectives)
   - Run representative queries (list, filter, derivation chain traversal)
   - Measure: WASM bundle size, initialization time, query performance, memory usage
2. Test offline operation:
   - Load app, go offline, perform CRUD operations
   - Come back online — does sync work? How?
3. If a candidate doesn't support client-side: document the fallback (API-only mode)
4. Assess whether client-side operation is even needed for Forge's use case, or if edge-served API is sufficient

**Deliverable:** `refs/research/db-research/82-client-side-test.md`

### T82.5: Architecture Decision Records

**Goal:** Produce a structured ADR for each candidate summarizing deployment viability.

**Steps:**
1. For each top candidate, write an Architecture Decision Record:
   - **Context:** Forge's requirements (from Phase 80)
   - **Decision drivers:** Graph, vector, edge, sync, migration effort
   - **Options considered:** This candidate vs staying on SQLite
   - **Deployment architecture:** From T82.2 diagrams
   - **Migration effort:** From T82.3 assessment
   - **Risks:** What could go wrong? Vendor lock-in? Immaturity? Missing features?
   - **Trade-offs:** What do we gain vs what do we lose vs SQLite?
2. Include a "stay on SQLite" ADR as the null hypothesis — what's the cost of NOT migrating?

**Deliverable:** `refs/research/db-research/82-adrs/` (one file per candidate + one for SQLite baseline)
