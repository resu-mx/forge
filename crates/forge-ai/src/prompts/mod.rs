//! Prompt template rendering for LLM derivation.

pub mod bullet_to_perspective;
pub mod jd_skill_extraction;
pub mod source_to_bullet;

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
