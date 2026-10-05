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

    let content =
        obj.get("content")
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
        assert_eq!(
            result.data.content,
            "Led cloud platform migration enabling ML-based log analysis"
        );
        assert_eq!(
            result.data.reasoning,
            "Emphasized the ML/analytics enablement aspect"
        );
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
