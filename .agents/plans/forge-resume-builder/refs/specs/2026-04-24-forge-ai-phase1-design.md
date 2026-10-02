# forge-ai Phase 1: Prompt Templates, Response Validators, JD Parser

**Date**: 2026-04-24
**Status**: Accepted
**Bead**: forge-w8co (R0.ai epic)

## Context

forge-ai is the AI/LLM interaction layer for Forge. It handles prompt construction,
response validation, and text analysis. It does NOT call LLM APIs directly — Forge
uses a split-handshake model where the server prepares prompts and the MCP client
invokes the LLM. Direct API calls are deferred to the AI Gateway initiative (forge-9s98).

Phase 1 covers the three pieces that unblock derivation routes in forge-server:
1. Prompt templates (3 renderers)
2. Response validators (3 validators)
3. JD requirement parser

Embeddings + alignment algorithms are deferred to phase 2.

## Module Structure

```
crates/forge-ai/src/
├── lib.rs                          # Public API surface
├── prompts/
│   ├── mod.rs                      # PromptTemplate enum, render dispatch
│   ├── source_to_bullet.rs         # v1 template + renderer
│   ├── bullet_to_perspective.rs    # v1 template + renderer
│   └── jd_skill_extraction.rs      # v1 template + renderer
├── validators/
│   ├── mod.rs                      # ValidationResult<T>, Warning type
│   ├── bullet.rs                   # BulletDerivationResponse validator
│   ├── perspective.rs              # PerspectiveDerivationResponse validator
│   └── skill_extraction.rs         # SkillExtractionResponse validator
└── jd_parser.rs                    # Requirement parsing (regex, sections, confidence)
```

## Prompt Templates

### Design

Three renderers, each producing a `RenderedPrompt` with system + user messages.
Templates are compiled-in `const &str` values with a version tag. Renderers are
pure functions: `(inputs) → RenderedPrompt`.

**Seam for future variablization (forge-u5ru):** The render functions take content
inputs, not template strings. The template text is a constant today, but the
function boundary is `(inputs) → RenderedPrompt`. When prompt management (forge-7dct)
lands, the constant gets replaced by a registry lookup at the call site.

### Types

```rust
/// Rendered prompt ready for LLM invocation.
pub struct RenderedPrompt {
    /// System message content.
    pub system: String,
    /// User message content.
    pub user: String,
    /// Template version identifier for attribution.
    pub template_version: &'static str,
}
```

### Renderers

#### 1. Source → Bullet (`source-to-bullet-v1`)

```rust
pub fn render_source_to_bullet(description: &str) -> RenderedPrompt;
```

Decomposes a source experience description into factual bullet points. Output
schema instructs the LLM to produce:
```json
{
  "bullets": [{
    "content": "factual bullet text",
    "technologies": ["tech1", "tech2"],
    "metrics": "quantitative metric or null"
  }]
}
```

Rules embedded in prompt:
- State only facts present in source
- Include specific technologies/tools/methods
- Include quantitative metrics if present
- Do NOT infer, embellish, or add context

#### 2. Bullet → Perspective (`bullet-to-perspective-v1`)

```rust
pub fn render_bullet_to_perspective(
    content: &str,
    technologies: &[String],
    metrics: Option<&str>,
    archetype: &str,
    domain: &str,
    framing: &str,
) -> RenderedPrompt;
```

Reframes a factual bullet for a target role archetype. Output schema:
```json
{
  "content": "reframed bullet text",
  "reasoning": "brief explanation of emphasis choices"
}
```

Rules embedded in prompt:
- Only use facts present in original bullet
- Emphasize aspects relevant to target archetype
- Do NOT add claims, technologies, outcomes not in bullet
- Use active voice, concise phrasing

#### 3. JD Skill Extraction (`jd-skill-extraction-v1`)

```rust
pub fn render_jd_skill_extraction(raw_text: &str) -> RenderedPrompt;
```

Extracts technical skills from a job description. Output schema:
```json
{
  "skills": [{
    "name": "skill name",
    "category": "language|framework|tool|platform|methodology|domain|soft_skill|certification|other",
    "confidence": 0.9
  }]
}
```

Confidence scale:
- ≥ 0.8: explicitly required
- 0.5–0.7: preferred/nice-to-have
- 0.3–0.5: implied but not required

## Response Validators

### Design

Strict JSON schema validators that parse LLM responses into typed structs.
Return `Result<ValidatedResponse<T>, ValidationError>` where success carries
both the parsed data and any non-fatal warnings.

Warnings are in-band (returned to caller) so the derivation service can store
them in `prompt_logs` for attribution. Validators also emit `tracing::warn!()`
for each warning so they're observable via OTEL when it lands.

### Types

```rust
/// Non-fatal issue found during validation.
#[derive(Debug, Clone, Serialize)]
pub struct Warning {
    pub field: String,
    pub message: String,
}

/// Successful validation — data plus any warnings.
pub struct ValidatedResponse<T> {
    pub data: T,
    pub warnings: Vec<Warning>,
}

/// Validation outcome.
pub type ValidationResult<T> = Result<ValidatedResponse<T>, ValidationError>;

/// Validation failure.
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("invalid JSON: {0}")]
    InvalidJson(String),
    #[error("{field}: {message}")]
    Schema { field: String, message: String },
}
```

### Response Types

```rust
/// Validated output from source → bullet derivation.
pub struct BulletDerivationResponse {
    pub bullets: Vec<DerivedBullet>,
}

pub struct DerivedBullet {
    pub content: String,
    pub technologies: Vec<String>,
    pub metrics: Option<String>,
}

/// Validated output from bullet → perspective derivation.
pub struct PerspectiveDerivationResponse {
    pub content: String,
    pub reasoning: String,
}

/// Validated output from JD skill extraction.
pub struct SkillExtractionResponse {
    pub skills: Vec<ExtractedSkill>,
}

pub struct ExtractedSkill {
    pub name: String,
    pub category: String,
    pub confidence: f64, // clamped to [0.0, 1.0]
}
```

### Validation Rules

#### BulletDerivationResponse
- `bullets` array must be present and non-empty
- Each bullet: `content` required (non-empty string), `technologies` required
  (array, may be empty), `metrics` required (string or null)
- Extra fields produce warnings, not errors

#### PerspectiveDerivationResponse
- `content` required (non-empty string)
- `reasoning` required (may be empty string)
- Extra fields produce warnings

#### SkillExtractionResponse
- `skills` array must be present (empty array is valid, with warning)
- Each skill: `name` required (non-empty, trimmed), `category` required
  (unknown values produce warning, not error), `confidence` required
  (clamped to [0, 1] with warning if out of range)
- Deduplicates by normalized name

## JD Requirement Parser

### Design

Regex-based section detection and heuristic confidence scoring. Direct 1:1 port
of `packages/core/src/lib/jd-parser.ts` for behavioral parity during strangler
fig migration. Logged for parser combinator rewrite after TS decommission (forge-odex).

### Public API

```rust
pub struct ParsedRequirements {
    pub requirements: Vec<ParsedRequirement>,
    pub overall_confidence: f64,
}

pub struct ParsedRequirement {
    pub text: String,
    pub confidence: f64,
    pub section: Option<String>,
}

/// Parse requirements from raw JD text.
/// Returns empty vec (not error) if no requirements detected.
pub fn parse_requirements(raw_text: &str) -> ParsedRequirements;
```

### Algorithm (matching TS behavior)

1. Guard: reject if > 100,000 chars (return empty)
2. Detect section headers via regex (Requirements, Qualifications, Responsibilities,
   Skills, Must-haves, etc.)
3. Extract content between section boundaries
4. Split on bullet markers (`-`, `*`, `+`), numbered lists (`1.`, `a)`), or line breaks
5. Score each requirement by structure confidence:
   - 0.9: structured list under requirement sections
   - 0.7: structured list under responsibility sections
   - 0.4–0.6: semi-structured or prose
   - ×0.6 multiplier if no sections detected
6. Filter out < 10 chars, deduplicate by normalized text
7. `overall_confidence` = mean of all requirement confidences

### Section Keywords

- **Requirement sections** (high confidence): Requirements, Qualifications,
  Must-haves, Skills, What you'll need, What we're looking for
- **Responsibility sections** (medium confidence): Responsibilities,
  Key Responsibilities, What you'll do
- **End-of-content markers**: Benefits, About Us, How to Apply, Perks,
  Equal Opportunity

## Dependencies

### Crate dependencies (no changes needed)

forge-ai already declares:
- `forge-core` (for shared types if needed)
- `serde`, `serde_json` (JSON parsing for validators)
- `thiserror` (error types)
- `tracing` (warning emission)
- `reqwest`, `tokio` — not used in phase 1, can remain declared

Additional (must be added to workspace Cargo.toml + forge-ai Cargo.toml):
- `regex` — needed for JD parser section detection

### No async required

All operations are pure CPU: string rendering, JSON parsing, regex matching.
No tokio runtime needed. Functions are sync, making testing straightforward.

## Testing

- **Prompt renderers**: snapshot tests — render with known inputs, assert output
  matches expected string. Test that template_version is set correctly.
- **Validators**: valid input, missing required fields, wrong types, extra fields
  (produce warnings), empty arrays, out-of-range confidence (clamped), deduplication.
- **JD parser**: structured JD with clear sections, unstructured prose, edge cases
  (>100K chars, no sections detected, mixed bullet/numbered formats, non-English
  section headers).
- Test fixtures extracted from TS test suite where available.

## Future Work (Tracked)

- **forge-u5ru**: Prompt template variablization — decouple templates from binary
- **forge-odex**: JD parser combinator rewrite (nom/winnow)
- **forge-bjih**: Idiomatic Rust validator rewrite (serde custom deserializers)
- **Phase 2**: Embedding computation (all-MiniLM-L6-v2 via ort) + alignment algorithms

## Related Beads

- forge-w8co: R0.ai epic (parent)
- forge-9s98: AI Gateway (future direct API client)
- forge-7dct: Prompt Management & LLMOps (future versioning/promotion)
- forge-dkuz: LLMOps platform evaluation
