# Critical Path Prompt: forge-62kb (Alignment Scoring Engine)

> Memory-aware kickoff prompt for a fresh session starting forge-62kb.
> Drafted at the close of the forge-afyg session (2026-04-28).
> Parallel to `critical-path-forge-jsxn.md`.

---

Continue the forge browser-first critical path. Next bead is forge-62kb
(alignment scoring engine — 8 match types, level adjustment,
provenance trace).

This bead is PARALLEL with forge-jsxn — they both consume the
`SkillGraphTraversal` trait now provided by forge-afyg, and they touch
different modules in `crates/forge-wasm/` (alignment vs extraction), so
they can run concurrently.

Context lives in memory and beads, not in conversation history:

- Read memory project_forge_afyg_shipped_2026_04_28 — the substrate
  this bead consumes. `SkillGraphTraversal` trait with all 7 methods
  is the spine of the match logic:
  - `find_aliases` → alias match type
  - `find_children` / `find_parents` → child / parent match types
  - `find_related` (filtered by edge type) → sibling, prerequisite
  - `search_by_embedding` (separate runtime method, NOT trait) →
    embedding-proximity match
  - `co_occurrence_stats` currently returns empty (forge-c4i5
    territory) — co-occurrence-proximity match must default to weight
    0 when stats are absent, NOT crash.

- Read memory project_skill_intelligence_architecture_2026_04_27 for
  the broader architecture: 8 match types with weights, level
  adjustment multiplier, gap / strength / coverage reports, provenance
  trace.

- Read memory project_skill_graph_implementation_2026_04_27 for the
  data layer this consumes via the trait.

- Read memory project_forge_lu5s_shipped_2026_04_28 for wa-sqlite
  patterns — forge-62kb DOES persist alignment_results in wa-sqlite,
  so the `Statement` / `Transaction` / `exec_batch` API and the
  `forge_core::migrations::MIGRATIONS` slice are directly relevant.

- Read memory feedback_zero_subprocess and feedback_bd_memory_ac_convention.

- Run `bd show forge-62kb` for bead details and acceptance criteria.

- Run `bd show forge-jsxn forge-5x2h` — sibling extractor (input
  source for production usage) and the downstream JS-API bead.

- Run `bd show forge-c4i5` if visible — co-occurrence stats are
  62kb's "co-occurrence proximity" match type AND c4i5's territory.
  62kb defaults to 0 weight when c4i5's stats are absent; this is
  the seam.

- Run `bd show forge-7e4f` — workspace-hygiene cleanup. Same workaround
  re-apply pattern as lu5s/afyg if still open.

Develop in `.claude/worktrees/forge-wasm/` (sequential after afyg or
jsxn) or a separate `.claude/worktrees/forge-62kb/` (parallel with
forge-jsxn). Use the `superpowers:using-git-worktrees` skill. Branch
from `main`.

Architectural constraints to respect (from memory + spec):

- Pure-Rust deps only.
- forge-server MUST NOT depend on forge-wasm.
- **8 match types**: direct, alias, cert, child, parent, sibling,
  embedding proximity, co-occurrence. Default weights from the
  architecture doc (1.0 / 1.0 / cert-weight / 0.9 / 0.5 / 0.4 /
  embedding-derived / 0.3) — make weights configurable via a struct
  field on `AlignmentEngine` so callers can experiment without
  recompiling. forge-c4i5 will tune later.
- **Level adjustment**: a multiplier applied AFTER the match type
  weight. JD asks "Senior X" + resume shows "Junior X" → multiplier
  < 1.0. Document the level scale (numeric? ordinal? per-skill via
  `level_descriptors`?) — this is the one piece that's underspecified
  in the architecture docs and worth pinning down in the brief.
- **Provenance trace**: every score entry carries
  `Vec<{ bullet_id: Option<String>, span: Option<Range>, match_type,
  source_skill_id }>` so forge-5x2h can render "this score came from
  these bullets". This is what makes the AC's "score explanations"
  testable. Pulled from forge-jsxn's `SkillCandidate.evidence` when
  jsxn's output is the input.
- **Persistence**: alignment_results table in wa-sqlite. **Verify
  whether a migration exists** before assuming you can write — likely
  needs a new migration in `forge-core::migrations::MIGRATIONS` slice
  (applied by both rusqlite and wa-sqlite runners). Conservative
  schema: `id` UUID PK, `resume_id`, `jd_id`, `computed_at`,
  `scores_json` (full AlignmentResult serialized), plus summary
  columns (overall_score REAL, gap_count INTEGER) for query speed.
- **Output shape**: `AlignmentResult { per_skill_scores, gap_report,
  strength_report, coverage_report, provenance_trace, overall_score
  }`. Single substantial return — no chatty per-method calls across
  WASM↔JS.
- **Embedding-proximity match**: the trait `SkillGraphTraversal`
  doesn't expose vector search — that lives on `SkillGraphRuntime`
  directly via `search_by_embedding`. `AlignmentEngine` could (a)
  take `&SkillGraphRuntime` concretely, or (b) take a separate
  `&dyn EmbeddingNearestNeighbor` parameter. Option (b) preserves
  testability (mock the NN search in unit tests) and is the cleaner
  shape for the long term.
- Run `cargo check --workspace` from MAIN worktree as the final gate.

Open questions worth surfacing in the brief (not for you to resolve
unilaterally):

- **alignment_results migration**: does it exist? If not, this bead
  adds it. Number of next migration after 053 — confirm with `ls
  packages/core/src/db/migrations/` (or via the `MIGRATIONS` slice).

- **Level model**: ordinal (Junior < Mid < Senior < Staff) with a
  fixed multiplier table? Per-skill via `level_descriptors`? MVP
  recommendation: ordinal with hardcoded multipliers; defer
  per-skill descriptors to a follow-up.

- **Gap / strength / coverage report shapes**: the architecture doc
  describes them conceptually but not as types. Pin them in the
  brief: a `GapReport` is `Vec<{required_skill, severity}>`,
  `StrengthReport` is `Vec<{skill, evidence_count, top_match_score}>`,
  `CoverageReport` is `{ matched_skills, total_required,
  coverage_pct }`.

- **Embedding NN seam**: confirm the `EmbeddingNearestNeighbor`
  trait shape (separate from `SkillGraphTraversal`). Single method:
  `search_by_embedding(&[f32], usize) -> Vec<(SkillNode, f32)>`,
  matching what `SkillGraphRuntime` already exposes.

- **Stored result lifecycle**: do alignment_results expire? Get
  superseded? For prototype: keep all, no expiry. forge-c4i5 or a
  later cleanup bead can add retention.

Brief me before claiming the bead so we agree on scope before code.
Specifically: the `AlignmentResult` struct shape; whether
alignment_results migration lands here or is deferred; the level
model; whether the embedding NN seam adds a new trait or stays
concrete on `SkillGraphRuntime`; which match types to implement
first (likely all 8 — they're the deliverable — but flag any that
need more design).
