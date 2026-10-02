# Critical Path Prompt: forge-jsxn (Skill Extraction Pipeline)

> Memory-aware kickoff prompt for a fresh session starting forge-jsxn.
> Drafted at the close of the forge-afyg session (2026-04-28).
> Parallel to `critical-path-forge-62kb.md`.

---

Continue the forge browser-first critical path. Next bead is forge-jsxn
(extraction pipeline — 4 extractors fused via Reciprocal Rank Fusion).

This bead is PARALLEL with forge-62kb — they both consume the
`SkillGraphTraversal` trait now provided by forge-afyg, and they touch
different modules in `crates/forge-wasm/` (extraction vs alignment), so
they can run concurrently in separate worktrees, or sequentially.

Context lives in memory and beads, not in conversation history:

- Read memory project_forge_afyg_shipped_2026_04_28 — the substrate
  this bead consumes. `SkillGraphRuntime` + `SkillGraphTraversal`
  trait + `search_by_embedding` (linear-scan prototype HNSW) +
  `search_skills` (substring autocomplete). The HNSW seam at
  `crates/forge-wasm/src/skill_graph/hnsw.rs` is brute-force linear
  scan; long-term library choice is under user research. Don't try to
  swap it as part of this bead — assume the seam holds.

- Read memory project_skill_graph_implementation_2026_04_27 for the
  data layer (schema migrations 052/053, traversal trait, snapshot
  format).

- Read memory project_skill_intelligence_architecture_2026_04_27 for
  the broader architecture: 4 extractors + RRF, search modalities
  matrix, browser-first deployment story.

- Read memory project_forge_lu5s_shipped_2026_04_28 for wa-sqlite
  patterns. forge-jsxn doesn't necessarily need to persist anything,
  but if it does, the `Statement` / `Transaction` / `exec_batch` API
  is already in place.

- Read memory feedback_zero_subprocess (no subprocess calls) and
  feedback_bd_memory_ac_convention (session-end memory is an AC).

- Run `bd show forge-jsxn` for bead details and acceptance criteria.

- Run `bd show forge-62kb forge-5x2h` — the sibling consumer and the
  downstream JS-API bead. Their needs constrain forge-jsxn's
  `SkillCandidate` output shape; surface any open questions in the
  brief.

- Run `bd show forge-7e4f` — workspace-hygiene cleanup. If still
  open, the lu5s/afyg workarounds (commits `fc13e74` + `8745049`)
  need re-applying on the fresh forge-jsxn branch.

Develop in `.claude/worktrees/forge-wasm/` (sequential after afyg)
or a separate `.claude/worktrees/forge-jsxn/` (parallel with
forge-62kb). Use the `superpowers:using-git-worktrees` skill to set
up cleanly. Branch from `main`, NOT from `forge-afyg` — that branch
is awaiting merge but jsxn doesn't depend on its file changes (only
on the SkillGraphTraversal trait, which lives in forge-core and
already shipped).

Architectural constraints to respect (from memory + spec):

- Pure-Rust deps only — no C-bindgen extras.
- forge-server MUST NOT depend on forge-wasm (passive guard in
  workspace.dependencies).
- All 4 extractors must be individually testable (per AC). Each
  takes a strategy-shape (text + entity_type + optional embedding)
  and emits a ranked list of `SkillCandidate`s.
- **Embedding model wire-up**: Option A from the bead (transformers.js
  on the JS side, vector handed to forge-wasm) is the pragmatic
  prototype path. Don't try to compile transformers.js into WASM
  (that's Option B, deferred). forge-jsxn's API should accept an
  optional pre-computed embedding so the JS-side caller can produce
  it once and pass it; if absent, extractor 1 (HNSW lookup) returns
  empty rather than blocking on a missing embedding.
- **RRF fusion**: rank-position-sum, score = Σ 1 / (k + rank_i). The
  paper's default k=60. Document the choice.
- **Entity-specific chunking**: JDs, bullets, certs, free text have
  different signal density and length. JDs are paragraph-heavy
  (chunk by sentence + sliding window); bullets are short (no
  chunking); certs are noun-heavy. Document each strategy in
  `pipelines/extraction.md` when shipping.
- **`SkillCandidate` shape**: pin this with forge-62kb's needs in
  mind. At minimum `{ skill_id, canonical_name, confidence,
  level_signal: Option<String>, evidence: Vec<{extractor, span,
  rank}> }`. The provenance vector is what makes alignment scoring's
  "score explanations" AC reachable.
- **HNSW miss flagging**: extractor 1 (embedding) returns no
  near-neighbor for a candidate sub-string with max cosine similarity
  < threshold (suggested 0.5) — flag those fragments as new-skill
  candidates for forge-0e9z curation. Define the data shape and where
  they get queued (a side channel in `SkillCandidate[]` or a separate
  return). MVP: a simple `flagged_unknown_terms: Vec<String>` field.
- Run `cargo check --workspace` from the MAIN worktree as the final
  gate — that's the pattern that catches feature-unification fallout
  (per the lu5s memory).

Open questions worth surfacing in the brief (not for you to resolve
unilaterally):

- **`SkillCandidate` shape**: confirm fields with forge-62kb's
  expected input. Bullet-id-level provenance only matters if the
  caller passes a bullet id — for free-text JD extraction there's no
  bullet, just spans. Consider making provenance an enum.

- **Pattern extractor regex set**: ESCO/O*NET seeding (forge-0e9z)
  hasn't shipped. For prototype, hand-coded patterns for the most
  common signals (years of experience, level adjectives like
  "senior" / "junior", licenses). Defer broader coverage to forge-0e9z.

- **Web Worker boundary**: The bead's perf AC ("< 500ms for typical
  JD") is achievable in main thread, but the extension's UI thread
  shouldn't block. Document whether forge-jsxn assumes Web Worker
  hosting (forge-5x2h's territory) or stays main-thread-friendly.

- **Stop words / noise filtering**: technical text has acronyms and
  proper nouns the embedding model treats as signal but aren't
  skills. Decide: filter at extractor boundary, or let RRF surface
  the noise and rely on a confidence threshold? MVP: confidence
  threshold, document.

Brief me before claiming the bead so we agree on scope before code.
Specifically: which extractors you plan to implement first (likely
all 4 — they're the deliverable — but flag any that warrant
splitting); the RRF k value; the `SkillCandidate` shape; whether the
pattern extractor's regex set ships in this bead or as a follow-up
absorbed into forge-0e9z.
