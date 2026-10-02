# Critical Path Prompt: forge-afyg (Skill Graph Runtime)

> Memory-aware kickoff prompt for a fresh session starting forge-afyg.
> Drafted at the close of the forge-lu5s session (2026-04-28).
> Parallel to `critical-path-forge-wasm.md`.

---

Continue the forge browser-first critical path. Next bead is forge-afyg
(skill graph runtime — petgraph + HNSW loaded from the SkillGraphSnapshot,
exposing the same `SkillGraphTraversal` trait the rusqlite SQL store
already implements).

Context lives in memory and beads, not in conversation history:

- Read memory project_forge_lu5s_shipped_2026_04_28 for what just
  shipped: WaSqliteAdapter with typed Statement/Transaction API, 51-
  migration runner consuming forge_core::migrations::MIGRATIONS,
  SkillStore subset (create/get/list/update/delete/list_categories)
  as the proof-of-life consumer, JS-facing WaSqliteAdapterJs wrapper.
  Also note the 3 runtime bugs caught at manual harness validation
  (log identifier collision in main.js, prepare_v2 pointer-not-string,
  _migrations ownership conflict with 001_initial.sql) — those lessons
  inform any wa-sqlite-adjacent work; don't repeat them here.

- Read memory project_skill_graph_implementation_2026_04_27 for the
  data layer this bead consumes: schema (migrations 052/053), the
  `SkillGraphTraversal` trait in `forge-core::types::skill_graph`
  (object-safe, 7 methods), and `SkillGraphSnapshot` binary format
  (MAGIC "FSGS" + format version + length-prefixed JSON header + raw
  embedding bytes + opaque HNSW blob). The HNSW payload slot is empty
  by default — forge-afyg fills it.

- Read memory project_skill_intelligence_architecture_2026_04_27 for
  the broader browser-first architecture, the market-stats slot
  (forge-c4i5's territory, NOT forge-afyg), and the progressive
  Svelte → Tauri → Dioxus consumption story.

- Read memory feedback_bd_memory_ac_convention for the new bd
  convention. Every open bead now carries a "save a session memory"
  criterion, and forge-afyg has it embedded in its acceptance
  criteria. The session-end memory file under
  /Users/adam/.claude/projects/-Users-adam-code-proj-forge/memory/
  is part of definition-of-done, not optional housekeeping.

- Read memory project_forge_nst6_shipped_2026_04_28 only if you need
  the wa-sqlite binding mechanics; the lu5s memory subsumes most of it.

- Run `bd show forge-afyg` for bead details and acceptance criteria.

- Run `bd show forge-jsxn forge-62kb forge-5x2h` — the three
  downstream consumers. Their API needs constrain forge-afyg's
  surface; surface any traversal/HNSW signature questions they imply.

- Run `bd show forge-7e4f` — workspace-hygiene cleanup bead filed at
  lu5s close. If it's still open by the time you start, the
  forge-es6o (template module) and forge-2qns (forge-server [lib])
  workarounds from forge-lu5s commits `fc13e74` and `8745049` may
  need re-applying on the forge-afyg branch. Same pattern, same
  reversibility.

Develop in .claude/worktrees/forge-wasm/. The lu5s branch's worktree
may still be on disk from the prior session — use the
superpowers:using-git-worktrees skill to cleanly remove it (if present)
and create fresh from main on a new forge-afyg branch.

Architectural constraints to respect (from memory + spec):

- forge-afyg implements `SkillGraphTraversal` as a PARALLEL impl
  alongside `SqlSkillGraphStore`. The trait already exists in
  forge-core; both impls live side by side. Do NOT refactor the SQL
  impl during this task.

- forge-wasm builds for both wasm32-unknown-unknown (cdylib) and host
  (rlib). All deps must be pure Rust — no C-bindgen extras. Candidate
  HNSW libs: hnsw-rs (pure Rust, MIT, claims wasm-friendly) and
  instant-distance (pure Rust, MIT). Verify wasm32 compat with the
  rustup-stable toolchain BEFORE committing to one.

- Bundle size matters. forge-lu5s release wasm is 840 KB pre-wasm-opt;
  HNSW + petgraph + traversal logic should not bloat this dramatically.
  Measure before merging.

- Load from snapshot, NOT from wa-sqlite at query time. forge-afyg
  reads the binary snapshot once (caller hands it a `&[u8]`; no
  network in this crate), holds the graph + HNSW in memory, and serves
  reads from there. The wa-sqlite database is for user-private skill
  *mutations* via SkillStore — the in-memory graph is for the global
  curated graph + queries. These are separate concerns.

- The `co_occurrence_stats` trait method reads the JSON header's
  market_stats slot. That slot is forge-c4i5's territory; for
  forge-afyg, return empty/default stats and document the seam in
  doc-comments.

- No subprocess calls (feedback_zero_subprocess rule). Compiled-in only.

- forge-server still MUST NOT depend on forge-wasm (passive guard in
  workspace.dependencies). Don't add forge-wasm to anything outside
  the wasm-bound consumer path.

- Run `cargo check --workspace` from the MAIN worktree (not the
  forge-afyg one) as the final gate before declaring done — that's
  the pattern that catches feature-unification fallout.

Open questions worth surfacing in the brief (not for you to resolve
unilaterally):

- HNSW library choice — needs an evaluation against wasm32, bundle
  size, and snapshot interchange (does the lib's serialization match
  the snapshot's opaque-blob expectation?).

- Snapshot loading entry point: a free function `fn from_snapshot(bytes:
  &[u8]) -> Result<SkillGraphRuntime, ForgeError>` is probably right.
  Confirm that's the shape downstream consumers expect.

- Eager vs lazy index construction. The skill_graph snapshot memory
  says "pre-built HNSW index" — so the blob is loaded as-is, no
  construction cost on the browser side. Confirm against the chosen
  lib's API: it must support deserialization from a known byte format,
  not just construction-from-vectors.

- n_hop_neighbors translation: the SQL impl uses a recursive CTE.
  petgraph's BFS/DFS visitors are a natural fit, but edge_type
  filtering and weight aggregation may need custom logic. Sketch the
  approach in the brief.

Brief me before claiming the bead so we agree on scope before code.
Specifically: which subset of the 7 `SkillGraphTraversal` methods you
plan to implement first (probably all of them — the trait is small —
but flag any that compose HNSW + graph traversal in a way that
warrants splitting); the HNSW library you're proposing with rationale;
and whether the snapshot format's HNSW slot needs ANY schema additions
for your chosen lib's serialized form.
