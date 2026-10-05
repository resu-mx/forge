//! Skill extraction response validator.
//!
//! Port of `packages/core/src/ai/validator.ts:validateSkillExtraction`.

use serde::Serialize;
use serde_json::Value;

use super::{extra_fields, ValidatedResponse, ValidationError, ValidationResult, Warning};

const VALID_CATEGORIES: &[&str] = &[
    "language",
    "framework",
    "tool",
    "platform",
    "methodology",
    "domain",
    "soft_skill",
    "certification",
    "other",
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

    let skills_arr = skills_val
        .as_array()
        .ok_or_else(|| ValidationError::Schema {
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
        let name =
            skill
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

        let clamped = if !(0.0..=1.0).contains(&confidence) {
            warnings.push(Warning {
                field: format!("{prefix}.confidence"),
                message: format!(
                    "{prefix}.confidence {confidence} is outside [0, 1] range, clamping"
                ),
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
        let data = json!({
            "skills": [{ "name": "Test", "category": "unknown_cat", "confidence": 0.5 }]
        });
        let result = validate(&data).unwrap();
        assert!(!result.warnings.is_empty());
        assert!(result.warnings[0].message.contains("unknown_cat"));
    }

    #[test]
    fn clamps_confidence_with_warning() {
        let data = json!({
            "skills": [{ "name": "Test", "category": "tool", "confidence": 1.5 }]
        });
        let result = validate(&data).unwrap();
        assert_eq!(result.data.skills[0].confidence, 1.0);
        assert!(!result.warnings.is_empty());
    }

    #[test]
    fn clamps_negative_confidence() {
        let data = json!({
            "skills": [{ "name": "Test", "category": "tool", "confidence": -0.3 }]
        });
        let result = validate(&data).unwrap();
        assert_eq!(result.data.skills[0].confidence, 0.0);
    }

    #[test]
    fn trims_skill_names() {
        let data = json!({
            "skills": [{ "name": "  Rust  ", "category": "language", "confidence": 0.9 }]
        });
        let result = validate(&data).unwrap();
        assert_eq!(result.data.skills[0].name, "Rust");
    }

    #[test]
    fn warns_on_extra_root_fields() {
        let data = json!({ "skills": [], "metadata": {} });
        let result = validate(&data).unwrap();
        assert!(result
            .warnings
            .iter()
            .any(|w| w.message.contains("metadata")));
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
        let err =
            validate(&json!({ "skills": [{ "name": "", "category": "tool", "confidence": 0.5 }] }))
                .unwrap_err();
        assert!(err.to_string().contains("name"));
    }

    #[test]
    fn rejects_missing_confidence() {
        let err =
            validate(&json!({ "skills": [{ "name": "Test", "category": "tool" }] })).unwrap_err();
        assert!(err.to_string().contains("confidence"));
    }

    #[test]
    fn rejects_non_number_confidence() {
        let err = validate(
            &json!({ "skills": [{ "name": "Test", "category": "tool", "confidence": "high" }] }),
        )
        .unwrap_err();
        assert!(err.to_string().contains("confidence"));
    }
}
