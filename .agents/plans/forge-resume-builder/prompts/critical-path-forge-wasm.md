# Session Prompt: Critical Path — Skill Graph Schema → forge-wasm → CF Deploy

## Context Recovery

Run `bd prime` then review these in order:

1. **Architecture docs** — Read `docs/src/architecture/README.md` for the full graph landscape, then `docs/src/architecture/models/deployment.md` for the browser-first deployment model. These were designed 2026-04-27.
2. **Critical path beads** — Run `bd show forge-ucc2` (Skill Graph Data Model), then `bd show forge-6z5l` (R1.5: forge-wasm). These are the two epics on the critical path.
3. **Migration plan** — Read `docs/src/migrations/mvp-2.0-browser-first.md` for the 6-phase plan.

## What was decided

**Browser-first architecture:** Browser is the primary runtime. Server is optional SaaS enhancement. OSS users get the full app with browser-local storage (wa-sqlite + OPFS). SaaS adds sync, backup, premium data.

**Rust WASM data layer (forge-wasm):** All compute (extraction, alignment, HNSW, graph traversal, wa-sqlite access) runs in a Rust crate compiled to WASM. Svelte calls it via wasm-bindgen now. Dioxus calls it natively at R4. The expensive investment is done once.

**Progressive UI migration:** Svelte (now) → Svelte + forge-wasm (MVP 2.0) → Tauri desktop (R2) → Dioxus replaces Svelte (R4). The data layer doesn't change — only the UI consumer.

## Critical path

```
forge-ucc2 (Skill Graph Schema)    ← START HERE
  ├── forge-7rd7  Schema: nodes + typed edges tables
  ├── forge-e2js  Migration: existing skills → graph schema
  ├── forge-8xjh  Traversal API (trait definition)
  └── forge-ubxb  Snapshot format
         ↓
forge-6z5l (R1.5: forge-wasm)      ← THEN HERE
  ├── forge-f0gc  Crate scaffold + wa-sqlite binding    (first)
  ├── forge-lu5s  ELM BrowserStore adapter               (after scaffold)
  ├── forge-afyg  Skill graph runtime (petgraph + HNSW)  (parallel with BrowserStore)
  ├── forge-jsxn  Extraction pipeline (4 extractors)     (after graph runtime)
  ├── forge-62kb  Alignment scoring engine                (parallel with extraction)
  ├── forge-5x2h  wasm-bindgen API + Svelte integration  (after all above)
  └── forge-8rzs  CRDT operation log                     (P2, after BrowserStore)
         ↓
forge-4a01 (MVP 2.0: CF Deploy)   ← THEN SHIP
```

## How to start

1. Claim `forge-ucc2` and start with `forge-7rd7` (schema design). The schema is the foundation everything builds on.
2. Read `docs/src/architecture/graphs/skills.md` for the designed node/edge structure.
3. The schema must work in both server SQLite and browser wa-sqlite — use standard SQLite types only.
4. After schema is stable, work on traversal API (`forge-8xjh`) and snapshot format (`forge-ubxb`) in parallel.
5. Then move to forge-wasm (`forge-6z5l`), starting with crate scaffold (`forge-f0gc`).

## Key constraints

- **Same SQL schema everywhere:** wa-sqlite in browser, SQLite on server, D1 in SaaS. No schema divergence.
- **Every sub-task must update docs:** Each bead has a "Docs to update on completion" checklist. Don't close without updating docs.
- **forge-wasm API is coarse-grained:** Each wasm-bindgen export does substantial work and returns a complete result. No chatty per-field calls across the JS↔WASM boundary. Batch and debounce.
- **Embedding model question (forge-jsxn):** Decide whether to call transformers.js from Svelte and pass vectors to WASM, or compile ONNX runtime into forge-wasm. Option A is simpler for MVP.

## Key files

- Architecture docs: `docs/src/architecture/` (15 files)
- Migration plan: `docs/src/migrations/mvp-2.0-browser-first.md`
- Existing Rust crates: `crates/forge-core/`, `crates/forge-sdk/`, `crates/forge-server/`
- New crate target: `crates/forge-wasm/`
- Svelte app: `packages/webui/`
- Infra (private, gitignored): `infra/` (aRustyDev/forge-infra)
