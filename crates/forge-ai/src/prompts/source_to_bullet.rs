//! Source → Bullet derivation prompt (v1).
//!
//! Decomposes a source experience description into factual bullet points.
//! Port of `packages/core/src/ai/prompts.ts:renderSourceToBulletPrompt`.

use super::RenderedPrompt;

pub const TEMPLATE_VERSION: &str = "source-to-bullet-v1";

const SYSTEM: &str =
    "You are a resume content assistant. Given a source description of work performed,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_includes_description() {
        let prompt = render("Led a 4-person team building cloud forensics tools");
        assert!(prompt
            .user
            .contains("Led a 4-person team building cloud forensics tools"));
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
