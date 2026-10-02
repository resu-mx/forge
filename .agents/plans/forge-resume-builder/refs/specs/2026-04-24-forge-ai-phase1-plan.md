# forge-ai Phase 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement prompt templates, response validators, and JD parser in the forge-ai crate — the three pieces that unblock derivation routes in forge-server.

**Architecture:** Three modules (`prompts/`, `validators/`, `jd_parser.rs`) with no async, no I/O, no external dependencies beyond `serde_json` and `regex`. Pure functions: inputs in, structured outputs out. All types are crate-local (not in forge-core).

**Tech Stack:** Rust, serde/serde_json (JSON parsing), regex (JD parser), thiserror (errors), tracing (warning emission)

**Spec:** `.claude/plans/forge-resume-builder/refs/specs/2026-04-24-forge-ai-phase1-design.md`

---

## File Structure

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `Cargo.toml` (workspace) | Add `regex` to workspace deps |
| Modify | `crates/forge-ai/Cargo.toml` | Add `regex`, remove unused `reqwest`/`tokio` |
| Rewrite | `crates/forge-ai/src/lib.rs` | Public API re-exports |
| Create | `crates/forge-ai/src/prompts/mod.rs` | `RenderedPrompt` type, module exports |
| Create | `crates/forge-ai/src/prompts/source_to_bullet.rs` | v1 template + renderer |
| Create | `crates/forge-ai/src/prompts/bullet_to_perspective.rs` | v1 template + renderer |
| Create | `crates/forge-ai/src/prompts/jd_skill_extraction.rs` | v1 template + renderer |
| Create | `crates/forge-ai/src/validators/mod.rs` | `ValidationResult`, `Warning`, `ValidationError` |
| Create | `crates/forge-ai/src/validators/bullet.rs` | `BulletDerivationResponse` validator |
| Create | `crates/forge-ai/src/validators/perspective.rs` | `PerspectiveDerivationResponse` validator |
| Create | `crates/forge-ai/src/validators/skill_extraction.rs` | `SkillExtractionResponse` validator |
| Create | `crates/forge-ai/src/jd_parser.rs` | JD requirement parser (regex, sections, confidence) |

---

### Task 1: Workspace and Crate Setup

**Files:**
- Modify: `Cargo.toml` (workspace root)
- Modify: `crates/forge-ai/Cargo.toml`
- Rewrite: `crates/forge-ai/src/lib.rs`

- [ ] **Step 1: Add regex to workspace dependencies**

In `Cargo.toml` (workspace root), add to `[workspace.dependencies]`:

```toml
regex = "1"
```

- [ ] **Step 2: Update forge-ai Cargo.toml**

Replace `crates/forge-ai/Cargo.toml` with:

```toml
[package]
name = "forge-ai"
version = "0.1.0"
edition = "2021"
description = "LLM prompts, response validation, and text analysis for Forge"

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
regex = { workspace = true }

[dev-dependencies]
```

Note: `forge-core`, `reqwest`, `tokio` removed — not needed for phase 1. They can be re-added when phase 2 (embeddings) or AI Gateway lands.

- [ ] **Step 3: Create lib.rs with module declarations**

Replace `crates/forge-ai/src/lib.rs` with:

```rust
//! LLM prompts, response validation, and text analysis for Forge.
//!
//! This crate handles prompt construction, AI response validation, and
//! JD parsing. It does NOT call LLM APIs — Forge uses a split-handshake
//! model where the MCP client invokes the LLM.

pub mod jd_parser;
pub mod prompts;
pub mod validators;
```

- [ ] **Step 4: Verify it compiles (empty modules)**

Create empty module files so it compiles:
- `crates/forge-ai/src/prompts/mod.rs` — empty
- `crates/forge-ai/src/validators/mod.rs` — empty
- `crates/forge-ai/src/jd_parser.rs` — empty

Run: `cargo check -p forge-ai`
Expected: compiles with no errors

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/forge-ai/
git commit -m "chore(forge-ai): set up crate for phase 1 — prompts, validators, jd_parser"
```

---

### Task 2: Prompt Types and Source-to-Bullet Template

**Files:**
- Create: `crates/forge-ai/src/prompts/mod.rs`
- Create: `crates/forge-ai/src/prompts/source_to_bullet.rs`

- [ ] **Step 1: Write tests for source-to-bullet renderer**

Add to `crates/forge-ai/src/prompts/source_to_bullet.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_includes_description() {
        let prompt = render("Led a 4-person team building cloud forensics tools");
        assert!(prompt.user.contains("Led a 4-person team building cloud forensics tools"));
    }

    #[test]
    fn render_sets_template_version() {
        let prompt = render("test description");
        assert_eq!(prompt.template_version, "source-to-bullet-v1");
    }

    #[test]
    fn render_system_contains_instructions() {
        let prompt = render("test");
        assert!(prompt.system.contains("resume content assistant"));
        assert!(prompt.system.contains("factual bullet points"));
    }

    #[test]
    fn render_user_contains_json_schema() {
        let prompt = render("test");
        assert!(prompt.user.contains("\"bullets\""));
        assert!(prompt.user.contains("\"content\""));
        assert!(prompt.user.contains("\"technologies\""));
        assert!(prompt.user.contains("\"metrics\""));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p forge-ai`
Expected: FAIL — `render` function not defined

- [ ] **Step 3: Implement RenderedPrompt type and source-to-bullet renderer**

Write `crates/forge-ai/src/prompts/mod.rs`:

```rust
//! Prompt template rendering for LLM derivation.

pub mod source_to_bullet;
pub mod bullet_to_perspective;
pub mod jd_skill_extraction;

/// Rendered prompt ready for LLM invocation.
#[derive(Debug, Clone)]
pub struct RenderedPrompt {
    /// System message content.
    pub system: String,
    /// User message content.
    pub user: String,
    /// Template version identifier for attribution.
    pub template_version: &'static str,
}
```

Write `crates/forge-ai/src/prompts/source_to_bullet.rs`:

```rust
//! Source → Bullet derivation prompt (v1).
//!
//! Decomposes a source experience description into factual bullet points.
//! Port of `packages/core/src/ai/prompts.ts:renderSourceToBulletPrompt`.

use super::RenderedPrompt;

pub const TEMPLATE_VERSION: &str = "source-to-bullet-v1";

const SYSTEM: &str = "You are a resume content assistant. Given a source description of work performed,
decompose it into factual bullet points. Each bullet must:
- State only facts present in the source description
- Include specific technologies, tools, or methods mentioned
- Include quantitative metrics if present in the source
- NOT infer, embellish, or add context not explicitly stated";

/// Render the source-to-bullet derivation prompt.
pub fn render(description: &str) -> RenderedPrompt {
    let user = format!(
        r#"Source description:
---
{description}
---

Respond with a JSON object:
{{
  "bullets": [
    {{
      "content": "factual bullet text",
      "technologies": ["tech1", "tech2"],
      "metrics": "quantitative metric if present, null otherwise"
    }}
  ]
}}"#
    );

    RenderedPrompt {
        system: SYSTEM.to_string(),
        user,
        template_version: TEMPLATE_VERSION,
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p forge-ai`
Expected: all 4 tests pass

- [ ] **Step 5: Commit**

```bash
git add crates/forge-ai/src/prompts/
git commit -m "feat(forge-ai): source-to-bullet prompt template v1"
```

---

### Task 3: Bullet-to-Perspective Template

**Files:**
- Modify: `crates/forge-ai/src/prompts/bullet_to_perspective.rs`

- [ ] **Step 1: Write tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_includes_all_inputs() {
        let prompt = render(
            "Built CI/CD pipeline",
            &["GitLab".to_string(), "Terraform".to_string()],
            Some("reduced deploy time by 40%"),
            "platform-engineer",
            "infrastructure",
            "accomplishment",
        );
        assert!(prompt.user.contains("Built CI/CD pipeline"));
        assert!(prompt.user.contains("GitLab, Terraform"));
        assert!(prompt.user.contains("reduced deploy time by 40%"));
        assert!(prompt.user.contains("platform-engineer"));
        assert!(prompt.user.contains("infrastructure"));
        assert!(prompt.user.contains("accomplishment"));
    }

    #[test]
    fn render_empty_technologies_shows_none() {
        let prompt = render("Test bullet", &[], None, "arch", "domain", "context");
        assert!(prompt.user.contains("(none)"));
    }

    #[test]
    fn render_null_metrics_shows_none() {
        let prompt = render("Test", &["Python".into()], None, "a", "d", "f");
        // Metrics line should show (none)
        assert!(prompt.user.contains("Metrics: (none)"));
    }

    #[test]
    fn render_sets_template_version() {
        let prompt = render("x", &[], None, "a", "d", "f");
        assert_eq!(prompt.template_version, "bullet-to-perspective-v1");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p forge-ai`
Expected: FAIL — `render` not defined in `bullet_to_perspective`

- [ ] **Step 3: Implement**

Write `crates/forge-ai/src/prompts/bullet_to_perspective.rs`:

```rust
//! Bullet → Perspective derivation prompt (v1).
//!
//! Reframes a factual bullet point for a target role archetype.
//! Port of `packages/core/src/ai/prompts.ts:renderBulletToPerspectivePrompt`.

use super::RenderedPrompt;

pub const TEMPLATE_VERSION: &str = "bullet-to-perspective-v1";

const SYSTEM: &str = "You are a resume content assistant. Given a factual bullet point, reframe it
for a target role archetype. The reframing must:
- Only use facts present in the original bullet
- Emphasize aspects relevant to the target archetype
- NOT add claims, technologies, outcomes, or context not in the bullet
- Use active voice, concise phrasing";

/// Render the bullet-to-perspective derivation prompt.
pub fn render(
    content: &str,
    technologies: &[String],
    metrics: Option<&str>,
    archetype: &str,
    domain: &str,
    framing: &str,
) -> RenderedPrompt {
    let tech_list = if technologies.is_empty() {
        "(none)".to_string()
    } else {
        technologies.join(", ")
    };
    let metrics_text = metrics.unwrap_or("(none)");

    let user = format!(
        r#"Original bullet:
---
{content}
Technologies: {tech_list}
Metrics: {metrics_text}
---

Target archetype: {archetype}
Target domain: {domain}
Framing style: {framing} (accomplishment | responsibility | context)

Respond with a JSON object:
{{
  "content": "reframed bullet text",
  "reasoning": "brief explanation of what was emphasized and why"
}}"#
    );

    RenderedPrompt {
        system: SYSTEM.to_string(),
        user,
        template_version: TEMPLATE_VERSION,
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p forge-ai`
Expected: all tests pass

- [ ] **Step 5: Commit**

```bash
git add crates/forge-ai/src/prompts/bullet_to_perspective.rs
git commit -m "feat(forge-ai): bullet-to-perspective prompt template v1"
```

---

### Task 4: JD Skill Extraction Template

**Files:**
- Modify: `crates/forge-ai/src/prompts/jd_skill_extraction.rs`

- [ ] **Step 1: Write tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_includes_jd_text() {
        let prompt = render("We are looking for a Senior Rust developer...");
        assert!(prompt.user.contains("Senior Rust developer"));
    }

    #[test]
    fn render_sets_template_version() {
        let prompt = render("test");
        assert_eq!(prompt.template_version, "jd-skill-extraction-v1");
    }

    #[test]
    fn render_system_contains_extraction_instructions() {
        let prompt = render("test");
        assert!(prompt.system.contains("technical recruiter assistant"));
    }

    #[test]
    fn render_user_contains_category_list() {
        let prompt = render("test");
        assert!(prompt.user.contains("language"));
        assert!(prompt.user.contains("framework"));
        assert!(prompt.user.contains("methodology"));
    }

    #[test]
    fn render_user_contains_confidence_guidance() {
        let prompt = render("test");
        assert!(prompt.user.contains("0.8"));
        assert!(prompt.user.contains("0.5"));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p forge-ai`
Expected: FAIL

- [ ] **Step 3: Implement**

Write `crates/forge-ai/src/prompts/jd_skill_extraction.rs`:

```rust
//! JD Skill Extraction prompt (v1).
//!
//! Extracts technical skills from a job description.
//! Port of `packages/core/src/ai/prompts.ts:renderJDSkillExtractionPrompt`.

use super::RenderedPrompt;

pub const TEMPLATE_VERSION: &str = "jd-skill-extraction-v1";

const SYSTEM: &str = "You are a technical recruiter assistant. Given a job description, extract the
technical skills, tools, technologies, and competencies that are required or preferred
for the role. For each skill, provide:
- The skill name (normalized: proper casing, common abbreviation)
- A category (one of: language, framework, tool, platform, methodology, domain, soft_skill, certification, other)
- A confidence score (0.0 to 1.0) indicating how clearly the JD states this is required

Rules:
- Extract specific technologies, not vague terms (e.g., \"Python\" not \"programming\")
- Use the most common/recognized name for each skill (e.g., \"Kubernetes\" not \"K8s\", \"AWS\" not \"Amazon Web Services\")
- Include both required and preferred/nice-to-have skills
- Set confidence >= 0.8 for explicitly required skills
- Set confidence 0.5-0.7 for preferred/nice-to-have skills
- Set confidence 0.3-0.5 for implied skills (mentioned in context but not as a requirement)
- Do NOT extract generic job requirements (e.g., \"communication skills\", \"team player\") unless they are specifically technical competencies
- Do NOT extract years of experience as skills
- Do NOT extract degree requirements as skills";

/// Render the JD skill extraction prompt.
pub fn render(raw_text: &str) -> RenderedPrompt {
    let user = format!(
        r#"Job description:
---
{raw_text}
---

Respond with a JSON object:
{{
  "skills": [
    {{
      "name": "skill name",
      "category": "language | framework | tool | platform | methodology | domain | soft_skill | certification | other",
      "confidence": 0.9
    }}
  ]
}}"#
    );

    RenderedPrompt {
        system: SYSTEM.to_string(),
        user,
        template_version: TEMPLATE_VERSION,
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p forge-ai`
Expected: all tests pass

- [ ] **Step 5: Commit**

```bash
git add crates/forge-ai/src/prompts/jd_skill_extraction.rs
git commit -m "feat(forge-ai): jd-skill-extraction prompt template v1"
```

---

### Task 5: Validator Types and Bullet Validator

**Files:**
- Create: `crates/forge-ai/src/validators/mod.rs`
- Create: `crates/forge-ai/src/validators/bullet.rs`

- [ ] **Step 1: Write validator types in mod.rs**

Write `crates/forge-ai/src/validators/mod.rs`:

```rust
//! Response validation for AI-generated outputs.

pub mod bullet;
pub mod perspective;
pub mod skill_extraction;

use serde::Serialize;

/// Non-fatal issue found during validation.
#[derive(Debug, Clone, Serialize)]
pub struct Warning {
    pub field: String,
    pub message: String,
}

/// Successful validation — data plus any warnings.
#[derive(Debug)]
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

/// Helper: collect extra keys from a JSON object not in `known`.
pub(crate) fn extra_fields(obj: &serde_json::Map<String, serde_json::Value>, known: &[&str]) -> Vec<String> {
    obj.keys()
        .filter(|k| !known.contains(&k.as_str()))
        .cloned()
        .collect()
}
```

- [ ] **Step 2: Write bullet validator tests**

Write `crates/forge-ai/src/validators/bullet.rs`:

```rust
//! Bullet derivation response validator.
//!
//! Port of `packages/core/src/ai/validator.ts:validateBulletDerivation`.

use serde::Serialize;
use serde_json::Value;

use super::{extra_fields, ValidatedResponse, ValidationError, ValidationResult, Warning};

/// Validated bullet derivation response.
#[derive(Debug, Clone, Serialize)]
pub struct BulletDerivationResponse {
    pub bullets: Vec<DerivedBullet>,
}

/// A single derived bullet.
#[derive(Debug, Clone, Serialize)]
pub struct DerivedBullet {
    pub content: String,
    pub technologies: Vec<String>,
    pub metrics: Option<String>,
}

/// Validate a raw JSON value as a bullet derivation response.
pub fn validate(data: &Value) -> ValidationResult<BulletDerivationResponse> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn accepts_valid_response() {
        let data = json!({
            "bullets": [
                {
                    "content": "Led 4-engineer team migrating cloud forensics platform",
                    "technologies": ["ELK", "AWS OpenSearch"],
                    "metrics": "4 engineers, 6 months"
                },
                {
                    "content": "Built infrastructure automation with Terraform",
                    "technologies": ["Terraform", "GitLab CI/CD"],
                    "metrics": null
                }
            ]
        });
        let result = validate(&data).unwrap();
        assert_eq!(result.data.bullets.len(), 2);
        assert_eq!(result.data.bullets[0].content, "Led 4-engineer team migrating cloud forensics platform");
        assert_eq!(result.data.bullets[0].technologies, vec!["ELK", "AWS OpenSearch"]);
        assert_eq!(result.data.bullets[0].metrics.as_deref(), Some("4 engineers, 6 months"));
        assert!(result.data.bullets[1].metrics.is_none());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn accepts_empty_technologies() {
        let data = json!({ "bullets": [{ "content": "Something", "technologies": [], "metrics": null }] });
        let result = validate(&data).unwrap();
        assert!(result.data.bullets[0].technologies.is_empty());
    }

    #[test]
    fn warns_on_extra_root_fields() {
        let data = json!({ "bullets": [{ "content": "Test", "technologies": [], "metrics": null }], "extra_field": "surprise" });
        let result = validate(&data).unwrap();
        assert!(!result.warnings.is_empty());
        assert!(result.warnings[0].message.contains("extra_field"));
    }

    #[test]
    fn warns_on_extra_bullet_fields() {
        let data = json!({ "bullets": [{ "content": "Test", "technologies": [], "metrics": null, "confidence": 0.95 }] });
        let result = validate(&data).unwrap();
        assert!(!result.warnings.is_empty());
        assert!(result.warnings[0].message.contains("confidence"));
    }

    #[test]
    fn rejects_non_object() {
        assert!(validate(&json!("string")).is_err());
        assert!(validate(&json!(null)).is_err());
        assert!(validate(&json!(42)).is_err());
    }

    #[test]
    fn rejects_missing_bullets() {
        let err = validate(&json!({ "content": "no bullets" })).unwrap_err();
        assert!(err.to_string().contains("bullets"));
    }

    #[test]
    fn rejects_non_array_bullets() {
        let err = validate(&json!({ "bullets": "not an array" })).unwrap_err();
        assert!(err.to_string().contains("array"));
    }

    #[test]
    fn rejects_empty_bullets() {
        let err = validate(&json!({ "bullets": [] })).unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn rejects_missing_content() {
        let err = validate(&json!({ "bullets": [{ "technologies": [], "metrics": null }] })).unwrap_err();
        assert!(err.to_string().contains("content"));
    }

    #[test]
    fn rejects_empty_content() {
        let err = validate(&json!({ "bullets": [{ "content": "   ", "technologies": [], "metrics": null }] })).unwrap_err();
        assert!(err.to_string().contains("content"));
    }

    #[test]
    fn rejects_non_array_technologies() {
        let err = validate(&json!({ "bullets": [{ "content": "Test", "technologies": "Python", "metrics": null }] })).unwrap_err();
        assert!(err.to_string().contains("technologies"));
    }

    #[test]
    fn rejects_non_string_technology_item() {
        let err = validate(&json!({ "bullets": [{ "content": "Test", "technologies": [123], "metrics": null }] })).unwrap_err();
        assert!(err.to_string().contains("technologies"));
    }

    #[test]
    fn rejects_missing_metrics() {
        let err = validate(&json!({ "bullets": [{ "content": "Test", "technologies": [] }] })).unwrap_err();
        assert!(err.to_string().contains("metrics"));
    }

    #[test]
    fn rejects_non_string_non_null_metrics() {
        let err = validate(&json!({ "bullets": [{ "content": "Test", "technologies": [], "metrics": 42 }] })).unwrap_err();
        assert!(err.to_string().contains("metrics"));
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test -p forge-ai`
Expected: FAIL — `validate` is `todo!()`

- [ ] **Step 4: Implement the bullet validator**

Replace the `todo!()` in `validate`:

```rust
pub fn validate(data: &Value) -> ValidationResult<BulletDerivationResponse> {
    let mut warnings = Vec::new();

    let obj = data.as_object().ok_or_else(|| ValidationError::Schema {
        field: "".into(),
        message: "Response is not an object".into(),
    })?;

    // Extra fields at root
    let root_extra = extra_fields(obj, &["bullets"]);
    if !root_extra.is_empty() {
        warnings.push(Warning {
            field: "".into(),
            message: format!("Unexpected root fields: {}", root_extra.join(", ")),
        });
    }

    // .bullets must exist and be an array
    let bullets_val = obj.get("bullets").ok_or_else(|| ValidationError::Schema {
        field: "bullets".into(),
        message: "Missing required field \"bullets\"".into(),
    })?;

    let bullets_arr = bullets_val.as_array().ok_or_else(|| ValidationError::Schema {
        field: "bullets".into(),
        message: "\"bullets\" must be an array".into(),
    })?;

    if bullets_arr.is_empty() {
        return Err(ValidationError::Schema {
            field: "bullets".into(),
            message: "\"bullets\" array is empty — no bullets produced".into(),
        });
    }

    let mut bullets = Vec::with_capacity(bullets_arr.len());

    for (i, item) in bullets_arr.iter().enumerate() {
        let prefix = format!("bullets[{i}]");

        let bullet = item.as_object().ok_or_else(|| ValidationError::Schema {
            field: prefix.clone(),
            message: format!("{prefix} is not an object"),
        })?;

        // Extra fields on bullet items
        let item_extra = extra_fields(bullet, &["content", "technologies", "metrics"]);
        if !item_extra.is_empty() {
            warnings.push(Warning {
                field: prefix.clone(),
                message: format!("unexpected fields: {}", item_extra.join(", ")),
            });
        }

        // content — required, non-empty string
        let content = bullet
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ValidationError::Schema {
                field: format!("{prefix}.content"),
                message: format!("{prefix}.content must be a string"),
            })?;

        if content.trim().is_empty() {
            return Err(ValidationError::Schema {
                field: format!("{prefix}.content"),
                message: format!("{prefix}.content must be non-empty"),
            });
        }

        // technologies — required, array of strings
        let tech_val = bullet.get("technologies").ok_or_else(|| ValidationError::Schema {
            field: format!("{prefix}.technologies"),
            message: format!("{prefix}.technologies must be an array"),
        })?;

        let tech_arr = tech_val.as_array().ok_or_else(|| ValidationError::Schema {
            field: format!("{prefix}.technologies"),
            message: format!("{prefix}.technologies must be an array"),
        })?;

        let mut technologies = Vec::with_capacity(tech_arr.len());
        for (j, t) in tech_arr.iter().enumerate() {
            let s = t.as_str().ok_or_else(|| ValidationError::Schema {
                field: format!("{prefix}.technologies[{j}]"),
                message: format!("{prefix}.technologies[{j}] must be a string"),
            })?;
            technologies.push(s.to_string());
        }

        // metrics — required field, string or null
        if !bullet.contains_key("metrics") {
            return Err(ValidationError::Schema {
                field: format!("{prefix}.metrics"),
                message: format!("{prefix}.metrics is required (use null if none)"),
            });
        }

        let metrics_val = &bullet["metrics"];
        let metrics = if metrics_val.is_null() {
            None
        } else {
            Some(
                metrics_val
                    .as_str()
                    .ok_or_else(|| ValidationError::Schema {
                        field: format!("{prefix}.metrics"),
                        message: format!("{prefix}.metrics must be a string or null"),
                    })?
                    .to_string(),
            )
        };

        bullets.push(DerivedBullet {
            content: content.to_string(),
            technologies,
            metrics,
        });
    }

    for w in &warnings {
        tracing::warn!(field = %w.field, "{}", w.message);
    }

    Ok(ValidatedResponse {
        data: BulletDerivationResponse { bullets },
        warnings,
    })
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p forge-ai`
Expected: all bullet validator tests pass

- [ ] **Step 6: Commit**

```bash
git add crates/forge-ai/src/validators/
git commit -m "feat(forge-ai): bullet derivation response validator"
```

---

### Task 6: Perspective Validator

**Files:**
- Create: `crates/forge-ai/src/validators/perspective.rs`

- [ ] **Step 1: Write tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn accepts_valid_response() {
        let data = json!({
            "content": "Led cloud platform migration enabling ML-based log analysis",
            "reasoning": "Emphasized the ML/analytics enablement aspect"
        });
        let result = validate(&data).unwrap();
        assert_eq!(result.data.content, "Led cloud platform migration enabling ML-based log analysis");
        assert_eq!(result.data.reasoning, "Emphasized the ML/analytics enablement aspect");
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn warns_on_extra_fields() {
        let data = json!({ "content": "Test", "reasoning": "ok", "confidence": 0.9 });
        let result = validate(&data).unwrap();
        assert!(!result.warnings.is_empty());
        assert!(result.warnings[0].message.contains("confidence"));
    }

    #[test]
    fn rejects_non_object() {
        assert!(validate(&json!(null)).is_err());
        assert!(validate(&json!(42)).is_err());
    }

    #[test]
    fn rejects_missing_content() {
        let err = validate(&json!({ "reasoning": "ok" })).unwrap_err();
        assert!(err.to_string().contains("content"));
    }

    #[test]
    fn rejects_empty_content() {
        let err = validate(&json!({ "content": "", "reasoning": "ok" })).unwrap_err();
        assert!(err.to_string().contains("non-empty"));
    }

    #[test]
    fn rejects_non_string_content() {
        let err = validate(&json!({ "content": 123, "reasoning": "ok" })).unwrap_err();
        assert!(err.to_string().contains("content"));
    }

    #[test]
    fn rejects_missing_reasoning() {
        let err = validate(&json!({ "content": "test" })).unwrap_err();
        assert!(err.to_string().contains("reasoning"));
    }

    #[test]
    fn rejects_null_reasoning() {
        let err = validate(&json!({ "content": "test", "reasoning": null })).unwrap_err();
        assert!(err.to_string().contains("reasoning"));
    }
}
```

- [ ] **Step 2: Implement**

```rust
//! Perspective derivation response validator.
//!
//! Port of `packages/core/src/ai/validator.ts:validatePerspectiveDerivation`.

use serde::Serialize;
use serde_json::Value;

use super::{extra_fields, ValidatedResponse, ValidationError, ValidationResult, Warning};

/// Validated perspective derivation response.
#[derive(Debug, Clone, Serialize)]
pub struct PerspectiveDerivationResponse {
    pub content: String,
    pub reasoning: String,
}

/// Validate a raw JSON value as a perspective derivation response.
pub fn validate(data: &Value) -> ValidationResult<PerspectiveDerivationResponse> {
    let mut warnings = Vec::new();

    let obj = data.as_object().ok_or_else(|| ValidationError::Schema {
        field: "".into(),
        message: "Response is not an object".into(),
    })?;

    let extra = extra_fields(obj, &["content", "reasoning"]);
    if !extra.is_empty() {
        warnings.push(Warning {
            field: "".into(),
            message: format!("Unexpected fields: {}", extra.join(", ")),
        });
    }

    let content = obj
        .get("content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ValidationError::Schema {
            field: "content".into(),
            message: "Missing or invalid \"content\" field (must be a string)".into(),
        })?;

    if content.trim().is_empty() {
        return Err(ValidationError::Schema {
            field: "content".into(),
            message: "\"content\" must be non-empty".into(),
        });
    }

    let reasoning = obj
        .get("reasoning")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ValidationError::Schema {
            field: "reasoning".into(),
            message: "Missing or invalid \"reasoning\" field (must be a string)".into(),
        })?;

    for w in &warnings {
        tracing::warn!(field = %w.field, "{}", w.message);
    }

    Ok(ValidatedResponse {
        data: PerspectiveDerivationResponse {
            content: content.to_string(),
            reasoning: reasoning.to_string(),
        },
        warnings,
    })
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p forge-ai`
Expected: all perspective tests pass

- [ ] **Step 4: Commit**

```bash
git add crates/forge-ai/src/validators/perspective.rs
git commit -m "feat(forge-ai): perspective derivation response validator"
```

---

### Task 7: Skill Extraction Validator

**Files:**
- Create: `crates/forge-ai/src/validators/skill_extraction.rs`

- [ ] **Step 1: Write tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn accepts_valid_response() {
        let data = json!({
            "skills": [
                { "name": "Rust", "category": "language", "confidence": 0.9 },
                { "name": "Kubernetes", "category": "platform", "confidence": 0.7 }
            ]
        });
        let result = validate(&data).unwrap();
        assert_eq!(result.data.skills.len(), 2);
        assert_eq!(result.data.skills[0].name, "Rust");
        assert_eq!(result.data.skills[0].confidence, 0.9);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn accepts_empty_skills_with_warning() {
        let data = json!({ "skills": [] });
        let result = validate(&data).unwrap();
        assert!(result.data.skills.is_empty());
        assert!(!result.warnings.is_empty());
    }

    #[test]
    fn warns_on_unknown_category() {
        let data = json!({ "skills": [{ "name": "Test", "category": "unknown_cat", "confidence": 0.5 }] });
        let result = validate(&data).unwrap();
        assert!(!result.warnings.is_empty());
        assert!(result.warnings[0].message.contains("unknown_cat"));
    }

    #[test]
    fn clamps_confidence_with_warning() {
        let data = json!({ "skills": [{ "name": "Test", "category": "tool", "confidence": 1.5 }] });
        let result = validate(&data).unwrap();
        assert_eq!(result.data.skills[0].confidence, 1.0);
        assert!(!result.warnings.is_empty());
    }

    #[test]
    fn clamps_negative_confidence() {
        let data = json!({ "skills": [{ "name": "Test", "category": "tool", "confidence": -0.3 }] });
        let result = validate(&data).unwrap();
        assert_eq!(result.data.skills[0].confidence, 0.0);
    }

    #[test]
    fn trims_skill_names() {
        let data = json!({ "skills": [{ "name": "  Rust  ", "category": "language", "confidence": 0.9 }] });
        let result = validate(&data).unwrap();
        assert_eq!(result.data.skills[0].name, "Rust");
    }

    #[test]
    fn warns_on_extra_root_fields() {
        let data = json!({ "skills": [], "metadata": {} });
        let result = validate(&data).unwrap();
        assert!(result.warnings.iter().any(|w| w.message.contains("metadata")));
    }

    #[test]
    fn rejects_non_object() {
        assert!(validate(&json!(null)).is_err());
    }

    #[test]
    fn rejects_missing_skills() {
        assert!(validate(&json!({})).is_err());
    }

    #[test]
    fn rejects_non_array_skills() {
        assert!(validate(&json!({ "skills": "not array" })).is_err());
    }

    #[test]
    fn rejects_empty_name() {
        let err = validate(&json!({ "skills": [{ "name": "", "category": "tool", "confidence": 0.5 }] })).unwrap_err();
        assert!(err.to_string().contains("name"));
    }

    #[test]
    fn rejects_missing_confidence() {
        let err = validate(&json!({ "skills": [{ "name": "Test", "category": "tool" }] })).unwrap_err();
        assert!(err.to_string().contains("confidence"));
    }

    #[test]
    fn rejects_non_number_confidence() {
        let err = validate(&json!({ "skills": [{ "name": "Test", "category": "tool", "confidence": "high" }] })).unwrap_err();
        assert!(err.to_string().contains("confidence"));
    }
}
```

- [ ] **Step 2: Implement**

```rust
//! Skill extraction response validator.
//!
//! Port of `packages/core/src/ai/validator.ts:validateSkillExtraction`.

use serde::Serialize;
use serde_json::Value;

use super::{extra_fields, ValidatedResponse, ValidationError, ValidationResult, Warning};

const VALID_CATEGORIES: &[&str] = &[
    "language", "framework", "tool", "platform",
    "methodology", "domain", "soft_skill", "certification", "other",
];

/// Validated skill extraction response.
#[derive(Debug, Clone, Serialize)]
pub struct SkillExtractionResponse {
    pub skills: Vec<ExtractedSkill>,
}

/// A single extracted skill.
#[derive(Debug, Clone, Serialize)]
pub struct ExtractedSkill {
    pub name: String,
    pub category: String,
    pub confidence: f64,
}

/// Validate a raw JSON value as a skill extraction response.
pub fn validate(data: &Value) -> ValidationResult<SkillExtractionResponse> {
    let mut warnings = Vec::new();

    let obj = data.as_object().ok_or_else(|| ValidationError::Schema {
        field: "".into(),
        message: "Response is not an object".into(),
    })?;

    let root_extra = extra_fields(obj, &["skills"]);
    if !root_extra.is_empty() {
        warnings.push(Warning {
            field: "".into(),
            message: format!("Unexpected root fields: {}", root_extra.join(", ")),
        });
    }

    let skills_val = obj.get("skills").ok_or_else(|| ValidationError::Schema {
        field: "skills".into(),
        message: "Missing required field \"skills\"".into(),
    })?;

    let skills_arr = skills_val.as_array().ok_or_else(|| ValidationError::Schema {
        field: "skills".into(),
        message: "\"skills\" must be an array".into(),
    })?;

    if skills_arr.is_empty() {
        warnings.push(Warning {
            field: "skills".into(),
            message: "No skills extracted from job description".into(),
        });
    }

    let mut skills = Vec::with_capacity(skills_arr.len());

    for (i, item) in skills_arr.iter().enumerate() {
        let prefix = format!("skills[{i}]");

        let skill = item.as_object().ok_or_else(|| ValidationError::Schema {
            field: prefix.clone(),
            message: format!("{prefix} is not an object"),
        })?;

        let item_extra = extra_fields(skill, &["name", "category", "confidence"]);
        if !item_extra.is_empty() {
            warnings.push(Warning {
                field: prefix.clone(),
                message: format!("unexpected fields: {}", item_extra.join(", ")),
            });
        }

        // name — required, non-empty, trimmed
        let name = skill
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ValidationError::Schema {
                field: format!("{prefix}.name"),
                message: format!("{prefix}.name must be a string"),
            })?;

        let name = name.trim();
        if name.is_empty() {
            return Err(ValidationError::Schema {
                field: format!("{prefix}.name"),
                message: format!("{prefix}.name must be non-empty"),
            });
        }

        // category — required, string (unknown values warn)
        let category = skill
            .get("category")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ValidationError::Schema {
                field: format!("{prefix}.category"),
                message: format!("{prefix}.category must be a string"),
            })?;

        if !VALID_CATEGORIES.contains(&category) {
            warnings.push(Warning {
                field: format!("{prefix}.category"),
                message: format!("{prefix}.category \"{category}\" is not a recognized category"),
            });
        }

        // confidence — required, number, clamped to [0, 1]
        let confidence = skill
            .get("confidence")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ValidationError::Schema {
                field: format!("{prefix}.confidence"),
                message: format!("{prefix}.confidence must be a number"),
            })?;

        let clamped = if confidence < 0.0 || confidence > 1.0 {
            warnings.push(Warning {
                field: format!("{prefix}.confidence"),
                message: format!("{prefix}.confidence {confidence} is outside [0, 1] range, clamping"),
            });
            confidence.clamp(0.0, 1.0)
        } else {
            confidence
        };

        skills.push(ExtractedSkill {
            name: name.to_string(),
            category: category.to_string(),
            confidence: clamped,
        });
    }

    for w in &warnings {
        tracing::warn!(field = %w.field, "{}", w.message);
    }

    Ok(ValidatedResponse {
        data: SkillExtractionResponse { skills },
        warnings,
    })
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p forge-ai`
Expected: all skill extraction tests pass

- [ ] **Step 4: Commit**

```bash
git add crates/forge-ai/src/validators/skill_extraction.rs
git commit -m "feat(forge-ai): skill extraction response validator"
```

---

### Task 8: JD Requirement Parser

**Files:**
- Create: `crates/forge-ai/src/jd_parser.rs`

- [ ] **Step 1: Write tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_returns_empty() {
        let result = parse_requirements("");
        assert!(result.requirements.is_empty());
        assert_eq!(result.overall_confidence, 0.0);
    }

    #[test]
    fn whitespace_only_returns_empty() {
        let result = parse_requirements("   \n  \n  ");
        assert!(result.requirements.is_empty());
    }

    #[test]
    fn oversized_input_returns_empty() {
        let input = "a".repeat(100_001);
        let result = parse_requirements(&input);
        assert!(result.requirements.is_empty());
        assert_eq!(result.overall_confidence, 0.0);
    }

    #[test]
    fn parses_structured_requirements_section() {
        let jd = "## Requirements\n\
                   - 5+ years experience with Rust\n\
                   - Experience with distributed systems\n\
                   - Strong understanding of async programming\n\
                   \n\
                   ## Benefits\n\
                   - Health insurance\n";
        let result = parse_requirements(jd);
        assert_eq!(result.requirements.len(), 3);
        assert!(result.requirements[0].text.contains("Rust"));
        assert_eq!(result.requirements[0].confidence, 0.9);
        assert!(result.requirements[0].section.is_some());
    }

    #[test]
    fn responsibility_section_gets_lower_confidence() {
        let jd = "## Responsibilities\n\
                   - Design and implement microservices\n\
                   - Mentor junior engineers\n";
        let result = parse_requirements(jd);
        assert_eq!(result.requirements.len(), 2);
        assert_eq!(result.requirements[0].confidence, 0.7);
    }

    #[test]
    fn no_sections_applies_multiplier() {
        let jd = "- Experience with Python\n\
                   - Knowledge of AWS\n";
        let result = parse_requirements(jd);
        assert!(!result.requirements.is_empty());
        // 0.9 * 0.6 = 0.54 for bullet lists without sections
        for req in &result.requirements {
            assert!(req.confidence < 0.7);
        }
    }

    #[test]
    fn filters_short_requirements() {
        let jd = "## Requirements\n\
                   - Rust\n\
                   - Experience with distributed systems and microservices\n";
        let result = parse_requirements(jd);
        // "Rust" is only 4 chars, filtered out (< 10)
        assert_eq!(result.requirements.len(), 1);
    }

    #[test]
    fn deduplicates_requirements() {
        let jd = "## Requirements\n\
                   - Experience with Python programming\n\
                   - Experience with Python programming\n";
        let result = parse_requirements(jd);
        assert_eq!(result.requirements.len(), 1);
    }

    #[test]
    fn handles_multiple_sections() {
        let jd = "## Requirements\n\
                   - Strong Rust experience\n\
                   \n\
                   ## Qualifications\n\
                   - Computer science degree or equivalent\n";
        let result = parse_requirements(jd);
        assert_eq!(result.requirements.len(), 2);
    }

    #[test]
    fn overall_confidence_is_mean() {
        let jd = "## Requirements\n\
                   - Experience with Rust programming language\n\
                   - Experience with Python programming language\n";
        let result = parse_requirements(jd);
        assert_eq!(result.requirements.len(), 2);
        let expected = result.requirements.iter().map(|r| r.confidence).sum::<f64>()
            / result.requirements.len() as f64;
        assert!((result.overall_confidence - expected).abs() < 0.001);
    }

    #[test]
    fn handles_prose_without_bullets() {
        let jd = "## Requirements\n\
                   We are looking for someone with strong Rust experience and knowledge of distributed systems.\n";
        let result = parse_requirements(jd);
        assert!(!result.requirements.is_empty());
    }

    #[test]
    fn stops_at_benefits_section() {
        let jd = "## Requirements\n\
                   - Experience with Kubernetes orchestration\n\
                   \n\
                   ## Benefits\n\
                   - Unlimited PTO\n\
                   - Free lunch\n";
        let result = parse_requirements(jd);
        assert_eq!(result.requirements.len(), 1);
        assert!(result.requirements[0].text.contains("Kubernetes"));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p forge-ai`
Expected: FAIL

- [ ] **Step 3: Implement the JD parser**

Write `crates/forge-ai/src/jd_parser.rs` — full 1:1 port of the TS `jd-parser.ts`. This is a longer file (~200 lines). Key sections:

```rust
//! JD Requirement Parser — extract individual requirements from raw JD text.
//!
//! 1:1 port of `packages/core/src/lib/jd-parser.ts` for behavioral parity.
//! Logged for parser combinator rewrite: forge-odex.

use regex::Regex;
use std::collections::HashSet;
use std::sync::LazyLock;

// ── Types ────────────────────────────────────────────────────────────

/// A single parsed requirement.
#[derive(Debug, Clone)]
pub struct ParsedRequirement {
    pub text: String,
    pub confidence: f64,
    pub section: Option<String>,
}

/// Parsed requirements from a JD.
#[derive(Debug, Clone)]
pub struct ParsedRequirements {
    pub requirements: Vec<ParsedRequirement>,
    pub overall_confidence: f64,
}

// ── Regex Patterns ───────────────────────────────────────────────────

static REQUIREMENT_SECTIONS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"(?im)^#{1,3}\s*(requirements|required\s+qualifications|minimum\s+qualifications|must[\s-]haves?)").unwrap(),
        Regex::new(r"(?im)^#{1,3}\s*(qualifications|preferred\s+qualifications|desired\s+qualifications)").unwrap(),
        Regex::new(r"(?im)^#{1,3}\s*(what\s+you(?:'ll|\s+will)\s+(?:need|bring)|what\s+we(?:'re|\s+are)\s+looking\s+for)").unwrap(),
        Regex::new(r"(?im)^#{1,3}\s*(responsibilities|key\s+responsibilities|role\s+responsibilities)").unwrap(),
        Regex::new(r"(?im)^#{1,3}\s*(nice[\s-]to[\s-]haves?|preferred|bonus|plus)").unwrap(),
        Regex::new(r"(?im)^#{1,3}\s*(skills|technical\s+skills|required\s+skills)").unwrap(),
        Regex::new(r"(?im)^\*{0,2}(requirements|qualifications|responsibilities|skills|what\s+you.+need)\*{0,2}\s*:?\s*$").unwrap(),
        Regex::new(r"(?im)^(requirements|qualifications|responsibilities|skills|what\s+you.+need)\s*:?\s*$").unwrap(),
    ]
});

static RESPONSIBILITY_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)responsibilities").unwrap());

static NON_REQUIREMENT_SECTIONS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"(?im)^#{1,3}\s*(benefits|perks|compensation|salary|about\s+(?:us|the\s+company|the\s+team))").unwrap(),
        Regex::new(r"(?im)^#{1,3}\s*(how\s+to\s+apply|application\s+process|equal\s+opportunity)").unwrap(),
        Regex::new(r"(?im)^#{1,3}\s*(company\s+(?:overview|description)|our\s+(?:mission|values|culture))").unwrap(),
        Regex::new(r"(?im)^\*{0,2}(benefits|perks|about\s+(?:us|the))\*{0,2}\s*:?\s*$").unwrap(),
        Regex::new(r"(?im)^(benefits|perks|about\s+(?:us|the))\s*:?\s*$").unwrap(),
    ]
});

static BULLET_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(?:[-*+]|\d+[.)]\s|[a-z][.)]\s|>\s)").unwrap());

static SEMICOLON_SPLIT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r";\s*").unwrap());

static SENTENCE_SPLIT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\.\s+(?=[A-Z])").unwrap());

// ── Public API ───────────────────────────────────────────────────────

/// Parse requirements from raw JD text.
pub fn parse_requirements(raw_text: &str) -> ParsedRequirements {
    if raw_text.trim().is_empty() {
        return ParsedRequirements {
            requirements: vec![],
            overall_confidence: 0.0,
        };
    }

    if raw_text.len() > 100_000 {
        return ParsedRequirements {
            requirements: vec![],
            overall_confidence: 0.0,
        };
    }

    let lines: Vec<&str> = raw_text.lines().collect();
    let sections = detect_sections(&lines);

    let mut requirements = if !sections.is_empty() {
        let mut reqs = Vec::new();
        for section in &sections {
            let is_responsibility = RESPONSIBILITY_PATTERN.is_match(&section.name);
            let parsed = parse_section_content(&section.content, Some(&section.name), is_responsibility);
            reqs.extend(parsed);
        }
        reqs
    } else {
        let mut reqs = parse_section_content(raw_text, None, false);
        for req in &mut reqs {
            req.confidence *= 0.6;
        }
        reqs
    };

    // Filter short requirements
    requirements.retain(|r| r.text.len() >= 10);

    // Deduplicate
    let mut seen = HashSet::new();
    requirements.retain(|r| {
        let key = r.text.to_lowercase().trim().to_string();
        seen.insert(key)
    });

    let overall_confidence = if requirements.is_empty() {
        0.0
    } else {
        requirements.iter().map(|r| r.confidence).sum::<f64>() / requirements.len() as f64
    };

    ParsedRequirements {
        requirements,
        overall_confidence,
    }
}

// ── Internal Helpers ─────────────────────────────────────────────────

struct DetectedSection {
    name: String,
    content: String,
}

fn detect_sections(lines: &[&str]) -> Vec<DetectedSection> {
    let mut sections = Vec::new();
    let mut current: Option<(String, Vec<&str>)> = None;

    for line in lines {
        // Check end-of-content sections
        if NON_REQUIREMENT_SECTIONS.iter().any(|p| p.is_match(line)) {
            if let Some((name, content_lines)) = current.take() {
                sections.push(DetectedSection {
                    name,
                    content: content_lines.join("\n"),
                });
            }
            continue;
        }

        // Check requirement section headers
        let mut matched_section = false;
        for pat in REQUIREMENT_SECTIONS.iter() {
            if let Some(caps) = pat.captures(line) {
                // Close previous section
                if let Some((name, content_lines)) = current.take() {
                    sections.push(DetectedSection {
                        name,
                        content: content_lines.join("\n"),
                    });
                }
                let section_name = caps
                    .get(1)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_else(|| "Requirements".to_string());
                current = Some((section_name, Vec::new()));
                matched_section = true;
                break;
            }
        }

        // Accumulate lines into current section
        if !matched_section {
            if let Some((_, ref mut content_lines)) = current {
                content_lines.push(line);
            }
        }
    }

    // Close final section
    if let Some((name, content_lines)) = current {
        sections.push(DetectedSection {
            name,
            content: content_lines.join("\n"),
        });
    }

    sections
}

fn parse_section_content(
    content: &str,
    section_name: Option<&str>,
    is_responsibility_section: bool,
) -> Vec<ParsedRequirement> {
    let mut requirements = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    let base_confidence = if is_responsibility_section { 0.7 } else { 0.9 };

    let bullet_lines: Vec<&&str> = lines.iter().filter(|l| BULLET_PATTERN.is_match(l)).collect();
    let is_bullet_list = !bullet_lines.is_empty();

    if is_bullet_list {
        for line in &lines {
            let trimmed = BULLET_PATTERN.replace(line, "").trim().to_string();
            if trimmed.is_empty() {
                continue;
            }

            let parts: Vec<&str> = SEMICOLON_SPLIT.split(&trimmed).collect();
            if parts.len() >= 3 {
                for part in parts {
                    let sub = part.trim().to_string();
                    if sub.len() >= 10 {
                        requirements.push(ParsedRequirement {
                            text: sub,
                            confidence: base_confidence - 0.1,
                            section: section_name.map(|s| s.to_string()),
                        });
                    }
                }
            } else {
                requirements.push(ParsedRequirement {
                    text: trimmed,
                    confidence: base_confidence,
                    section: section_name.map(|s| s.to_string()),
                });
            }
        }
    } else {
        let non_empty: Vec<&str> = lines.iter().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();

        if non_empty.len() == 1 {
            let sentences: Vec<&str> = SENTENCE_SPLIT.split(non_empty[0]).collect();
            for sentence in sentences {
                let trimmed = sentence.trim().to_string();
                if trimmed.len() >= 10 {
                    requirements.push(ParsedRequirement {
                        text: trimmed,
                        confidence: 0.4,
                        section: section_name.map(|s| s.to_string()),
                    });
                }
            }
        } else {
            for line in non_empty {
                requirements.push(ParsedRequirement {
                    text: line.to_string(),
                    confidence: 0.6,
                    section: section_name.map(|s| s.to_string()),
                });
            }
        }
    }

    requirements
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p forge-ai`
Expected: all JD parser tests pass

- [ ] **Step 5: Commit**

```bash
git add crates/forge-ai/src/jd_parser.rs
git commit -m "feat(forge-ai): JD requirement parser — 1:1 port from TS"
```

---

### Task 9: Final Integration — Full Test Suite and Cleanup

**Files:**
- Modify: `crates/forge-ai/src/lib.rs` (verify re-exports)

- [ ] **Step 1: Run full test suite**

Run: `cargo test -p forge-ai -p forge-sdk -p forge-core -p forge-server`
Expected: all tests pass (forge-ai new tests + existing 203 tests)

- [ ] **Step 2: Run clippy**

Run: `cargo clippy -p forge-ai -- -D warnings`
Expected: no warnings

- [ ] **Step 3: Count test coverage**

Run: `cargo test -p forge-ai 2>&1 | grep "test result"`
Expected: report total test count (should be ~50+ tests)

- [ ] **Step 4: Commit final state**

```bash
git add -A crates/forge-ai/
git commit -m "feat(forge-ai): phase 1 complete — 3 prompt templates, 3 validators, JD parser"
```
