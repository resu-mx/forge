# Alignment Scoring Engine — Design Spec (forge-62kb)

> Bead: forge-62kb
> Worktree: `.claude/worktrees/forge-62kb/` (parallel with forge-jsxn in `.claude/worktrees/forge-wasm/`)
> Branch: from `main`
> Status: Approved (2026-04-28)
> Parallel of: forge-jsxn (extraction)
> Consumes: `SkillGraphTraversal` trait (forge-afyg), `SkillGraphRuntime::search_by_embedding` (forge-afyg)
> Downstream: forge-5x2h (wasm-bindgen API + Svelte), forge-c4i5 (co-occurrence stats)

## Goal

Implement graph-aware alignment scoring in `forge-wasm`. Take a resume and a JD, return a single coarse-grained `AlignmentResult` covering all 8 match types, level adjustment, gap / strength / coverage reports, and provenance trace. Persist to a new `alignment_results` table in wa-sqlite for history.

## Non-goals (deferred to separate beads)

- Real co-occurrence weights — `forge-c4i5` territory; defaults to 0 here when `co_occurrence_stats` returns empty.
- Per-skill `level_descriptors` — MVP uses an ordinal level enum with hardcoded multipliers.
- Polished JS API + Svelte integration — `forge-5x2h`.
- Automated wasm-bindgen-test browser harness — `forge-901c`.
- Real HNSW library swap — `forge-afyg`'s linear-scan seam stays in place.
- Population of resume / JD skill inputs — assumes upstream has already produced them. Tests use synthetic fixtures.

## Architecture

### Module layout

```
crates/forge-wasm/src/alignment/
  mod.rs                 # public re-exports
  config.rs              # AlignmentConfig (weights, level multipliers, thresholds)
  level.rs               # SkillLevel enum + LevelAdjustment helper
  match_types.rs         # MatchType enum + per-type scoring functions
  engine.rs              # AlignmentEngine (the orchestrator)
  result.rs              # AlignmentResult + sub-reports + ProvenanceEntry
  embedding_nn.rs        # EmbeddingNearestNeighbor trait + impl-for-SkillGraphRuntime
  store.rs               # AlignmentResultStore (wa-sqlite persistence)
```

### Public API surface

```rust
// crates/forge-wasm/src/alignment/engine.rs
pub struct AlignmentEngine<'a> {
    graph: &'a dyn SkillGraphTraversal,
    embed_nn: &'a dyn EmbeddingNearestNeighbor,
    config: AlignmentConfig,
}

impl<'a> AlignmentEngine<'a> {
    pub fn new(
        graph: &'a dyn SkillGraphTraversal,
        embed_nn: &'a dyn EmbeddingNearestNeighbor,
    ) -> Self;

    pub fn with_config(mut self, config: AlignmentConfig) -> Self;

    pub fn align(
        &self,
        resume: &ResumeAlignmentInput,
        jd: &JdAlignmentInput,
    ) -> Result<AlignmentResult, ForgeError>;
}
```

`ResumeAlignmentInput` and `JdAlignmentInput` are simple DTOs (resume_id, jd_id, vec of `SkillRef { id, level: Option<SkillLevel>, evidence: Vec<EvidencePointer> }`). They are NOT consumed from forge-jsxn directly — jsxn's `SkillCandidate` will be adapted at the call site (in forge-5x2h or a glue layer). This keeps 62kb decoupled from jsxn's exact output type and lets each bead ship independently.

### Config struct

```rust
// alignment/config.rs
pub struct AlignmentConfig {
    pub weights: MatchWeights,
    pub level_multipliers: LevelMultipliers,
    pub gap_threshold: f64,        // default 0.4 — score below this = gap
    pub strong_threshold: f64,     // default 0.8
    pub embedding_similarity_min: f64, // default 0.7 — below this, embedding match disabled
}

pub struct MatchWeights {
    pub direct: f64,        // 1.0
    pub alias: f64,         // 1.0
    pub cert: f64,          // 0.95 (per architecture doc)
    pub child: f64,         // 0.9
    pub parent: f64,        // 0.5
    pub sibling: f64,       // 0.4
    pub embedding_max: f64, // 1.0 (multiplied by cosine similarity)
    pub co_occurrence: f64, // 0.3 (defaults to 0 effective until c4i5 lands)
}
```

`Default` impl matches the architecture doc. Callers (and forge-c4i5 later) can override without recompiling.

### Level model

```rust
// alignment/level.rs
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub enum SkillLevel { Junior, Mid, Senior, Staff, Principal }

pub struct LevelMultipliers {
    pub exceeds: f64,         // 1.0
    pub meets: f64,           // 1.0
    pub partial_one_rung: f64,  // 0.7
    pub partial_two_rungs: f64, // 0.5
    pub partial_three_plus: f64,// 0.3
    pub missing: f64,         // 0.8 — JD has no level info, assume moderate
}
```

Multiplier rule:
- `jd_level == None` → `missing`
- `resume_level == None` → `missing` (treat as no signal)
- `resume_level >= jd_level` (equal counts) → `exceeds`/`meets` (both 1.0; tracked separately for telemetry)
- `resume_level < jd_level` → `partial_one_rung` / `partial_two_rungs` / `partial_three_plus` based on `(jd_level as u8) - (resume_level as u8)`

Per-skill `level_descriptors` (custom ordinal scales per skill node) are deferred. The flat enum covers the MVP.

### Match types — execution order

For each JD skill, the engine runs all 8 match-type checks against the resume's skill set, takes the **maximum** score, then applies the level multiplier. Match types implemented in this order (cheapest → most expensive):

1. **Direct** — JD skill_id ∈ resume skill_ids. Trivial.
2. **Alias** — `graph.find_aliases(jd_skill)` ∩ resume → score = `weights.alias`.
3. **Cert** — resume has a cert that validates this skill (cert→skill edges). Stub: walk `graph.find_related(resume_cert, EdgeType::Validates)` for each cert in resume; score = `weights.cert`.
4. **Child** — JD asks broad, resume has specific. `graph.find_children(jd_skill)` ∩ resume → score = `weights.child`.
5. **Parent** — JD asks specific, resume has broad. `graph.find_parents(jd_skill)` ∩ resume → score = `weights.parent`.
6. **Sibling** — `graph.find_related(jd_skill, EdgeType::Sibling)` ∩ resume → score = `weights.sibling`.
7. **Embedding proximity** — `embed_nn.search_by_embedding(jd_skill_embedding, k=20)` returns `(SkillNode, similarity: f32)` with cosine similarity (higher = closer, per afyg's contract). Filter `similarity >= embedding_similarity_min`, find top hit ∈ resume → score = `weights.embedding_max * similarity`.
8. **Co-occurrence** — `graph.co_occurrence_stats(jd_skill)`. If empty, weight is 0. If populated (post-c4i5), find resume skills in the co-occurring list → score = `weights.co_occurrence * normalized_strength`.

Each contributing match yields a **provenance entry** (one per match type that fired, not just the winner — for debuggability).

### EmbeddingNearestNeighbor trait

```rust
// alignment/embedding_nn.rs
pub trait EmbeddingNearestNeighbor {
    fn search_by_embedding(
        &self,
        query: &[f32],
        top_k: usize,
    ) -> Result<Vec<(SkillNode, f32)>, ForgeError>;
}

// Concrete impl in same file:
#[cfg(not(target_arch = "wasm32"))]  // also on wasm32; cfg only narrows test code
impl EmbeddingNearestNeighbor for SkillGraphRuntime { ... }
```

Single-method trait. Mirrors the `SkillGraphRuntime::search_by_embedding` signature exactly. Tests use a `MockEmbeddingNN` that returns canned `(SkillNode, f32)` pairs.

### Output shape

```rust
// alignment/result.rs
pub struct AlignmentResult {
    pub resume_id: String,
    pub jd_id: String,
    pub computed_at_ms: u64,
    pub overall_score: f64,
    pub per_skill_scores: Vec<SkillScore>,
    pub gap_report: GapReport,
    pub strength_report: StrengthReport,
    pub coverage_report: CoverageReport,
    pub provenance: Vec<ProvenanceEntry>,
}

pub struct SkillScore {
    pub skill_id: String,
    pub score: f64,           // post-level-adjustment
    pub raw_score: f64,       // pre-level-adjustment
    pub level_multiplier: f64,
    pub top_match_type: MatchType,
}

pub struct GapReport {
    pub entries: Vec<GapEntry>, // {required_skill, severity, best_match: Option<MatchSummary>}
}

pub struct StrengthReport {
    pub entries: Vec<StrengthEntry>, // {skill, evidence_count, top_match_score}
}

pub struct CoverageReport {
    pub strong: u32,    // > strong_threshold
    pub moderate: u32,  // gap_threshold..=strong_threshold
    pub weak: u32,      // 0..gap_threshold AND > 0
    pub gap: u32,       // == 0
    pub total_required: u32,
    pub coverage_pct: f64, // (strong + moderate) / total_required
}

pub struct ProvenanceEntry {
    pub jd_skill_id: String,
    pub resume_skill_id: Option<String>, // None for embedding/co-occurrence misses
    pub match_type: MatchType,
    pub score: f64,
    pub bullet_id: Option<String>,
    pub span: Option<TextRange>,
}
```

`overall_score = weighted_mean(per_skill_scores)` — weights default to 1.0 per JD skill but `AlignmentConfig` could later add per-skill importance.

### Persistence

New migration `054_alignment_results.sql` lifted into `forge-core::migrations::MIGRATIONS` (lu5s pattern — both rusqlite and wa-sqlite runners pick it up):

```sql
CREATE TABLE alignment_results (
  id TEXT PRIMARY KEY,
  resume_id TEXT NOT NULL,
  jd_id TEXT NOT NULL,
  computed_at INTEGER NOT NULL,        -- unix milliseconds
  overall_score REAL NOT NULL,
  gap_count INTEGER NOT NULL,
  result_json TEXT NOT NULL            -- full AlignmentResult serialized
) STRICT;

CREATE INDEX idx_alignment_results_resume_jd
  ON alignment_results(resume_id, jd_id, computed_at DESC);
```

Lifecycle: keep all, no expiry. Retention is a follow-up bead.

`AlignmentResultStore` (in `crates/forge-wasm/src/alignment/store.rs`) provides:
- `insert(&Database, &AlignmentResult) -> Result<()>`
- `get_latest(&Database, resume_id, jd_id) -> Result<Option<AlignmentResult>>`
- `list_for_resume(&Database, resume_id, limit) -> Result<Vec<AlignmentResultSummary>>` (uses summary columns; doesn't decode `result_json`)

### wasm-bindgen export

```rust
// alignment/wasm_bindings.rs (target_arch = "wasm32" only)
#[wasm_bindgen]
pub struct AlignmentEngineJs { /* owns Database + SkillGraphRuntime handles */ }

#[wasm_bindgen]
impl AlignmentEngineJs {
    #[wasm_bindgen(js_name = alignResume)]
    pub fn align_resume(&self, resume_id: &str, jd_id: &str) -> Result<String, JsValue>;
    // returns AlignmentResult as JSON string (afyg's convention)
}
```

`forge-5x2h` will polish this surface. The minimum here: one method, JSON-string return.

## Testing strategy

- **Unit tests** (per match type): each match type has 2-3 tests using a tiny synthetic graph + canned resume/JD. Covers happy path + miss + boundary case (e.g., embedding distance just over threshold).
- **Integration test**: full `align()` call on a 50-node graph with a 5-skill resume and 8-skill JD. Asserts overall_score is reasonable, gap_report contains the expected missing skills, provenance trace covers every per-skill score.
- **Persistence test**: round-trip an `AlignmentResult` through `AlignmentResultStore` against an in-memory rusqlite database (host target, not wa-sqlite — wa-sqlite is exercised via the browser-smoke harness).
- **Performance regression test**: `perf_budgets_at_typical_size` — gated `#[cfg(not(debug_assertions))]` like afyg's. Budget: <100ms for 100-skill resume × 50-skill JD on the 10k-node fixture.
- **Co-occurrence absence test**: explicit test that with `co_occurrence_stats` returning empty, the engine completes without error and the co-occurrence weight contribution is 0.
- **MockEmbeddingNN**: in `tests/` module, returns canned results. No real HNSW search in unit tests.

Acceptance for this bead's "Scores explanations available" AC: any test that constructs an `AlignmentResult` must be able to inspect `provenance` and recover which match type produced each score.

## Risks & mitigations

| Risk | Mitigation |
|------|-----------|
| jsxn's `SkillCandidate` shape drifts before 62kb consumes it | 62kb does NOT consume `SkillCandidate` directly — uses simple `ResumeAlignmentInput` / `JdAlignmentInput` DTOs. Glue at call site (forge-5x2h or a thin adapter) absorbs jsxn's shape changes. |
| `co_occurrence_stats` returns Err instead of empty when c4i5 isn't ready | Engine treats both `Err(NotFound)` and `Ok(empty)` as "no co-occurrence data, weight = 0". Explicit test. |
| Migration 054 conflicts with parallel work | jsxn doesn't touch migrations. forge-7e4f's lib.rs work doesn't touch migrations. Low. |
| Bundle size grows | Pure additive Rust, no C deps. Re-measure post-bead via `wasm-pack build --release && wasm-opt -Oz`. Not gating, informational. |
| Performance budget violated | Same headroom as afyg (50× on graph queries). If embedding search dominates, can reduce `top_k` or pre-filter via direct/alias before embedding. Regression test catches drift. |
| `EmbeddingNearestNeighbor` impl on `SkillGraphRuntime` adds a circular module dep | The trait lives in `crates/forge-wasm/src/alignment/embedding_nn.rs`; the impl is in the same file. `SkillGraphRuntime` is in a sibling module. Same crate, no circularity. |

## Open questions

- **EvidencePointer shape**: kickoff says provenance carries `bullet_id: Option<String>` and `span: Option<Range>`. `Range<usize>` (byte offsets) vs `TextRange { start, end }` (custom). Going with `TextRange { start: u32, end: u32 }` because (a) `Range<usize>` doesn't implement `Copy`, (b) byte vs char offsets need to be explicit and documented at the type. Resolved in spec.
- **Cert→skill edge type name**: forge-afyg's `EdgeType` enum doesn't list `Validates` explicitly per the kickoff context — need to confirm what's actually in `forge-core::types::skill_graph::EdgeType` when implementing. If absent, cert match either: (a) defers to forge-jsxn's cert handling, or (b) adds the edge type. This is a code-discovery item for the implementation plan.
- **Overall score weighting**: spec uses unweighted mean across required JD skills. JD skill importance (must-have vs nice-to-have) isn't modeled yet. Follow-up bead can add `importance: f64` to `JdAlignmentInput::skills`.

## Deliverables checklist (mirrors AC)

- [ ] All 8 match types implemented
- [ ] Level matching with `SkillLevel` enum + multiplier table
- [ ] Provenance trace links jd_skill → resume_skill → bullet_id/span
- [ ] Migration 054 in `forge-core::migrations::MIGRATIONS`
- [ ] `alignment_results` round-trip via `AlignmentResultStore`
- [ ] `AlignmentEngineJs::alignResume(resume_id, jd_id)` exported via wasm-bindgen
- [ ] <100ms budget in `perf_budgets_at_typical_size` regression test
- [ ] `co_occurrence_stats` empty → weight 0, no panic
- [ ] `docs/src/architecture/retrieval/alignment-scoring.md` updated with implementation notes
- [ ] `docs/src/architecture/graphs/computed.md` updated with verified schema
- [ ] Session-end memory file at `~/.claude/projects/-Users-adam-code-proj-forge/memory/project_forge_62kb_shipped_2026_04_28.md` (per `feedback_bd_memory_ac_convention`)
- [ ] `cargo check --workspace` clean from MAIN worktree
- [ ] `cargo test -p forge-wasm` all passing
- [ ] `cargo check --target wasm32-unknown-unknown -p forge-wasm` clean

## Revision history

- 2026-04-28: Initial spec, approved after brainstorming session.
