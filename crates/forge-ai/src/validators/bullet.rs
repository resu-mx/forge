//! Bullet derivation response validator.

use serde::Serialize;
use serde_json::Value;

use super::{extra_fields, ValidatedResponse, ValidationError, ValidationResult, Warning};

/// Validated bullet derivation response.
#[derive(Debug, Clone, Serialize)]
pub struct BulletDerivationResponse {
    pub bullets: Vec<DerivedBullet>,
}

/// A single derived bullet from AI output.
#[derive(Debug, Clone, Serialize)]
pub struct DerivedBullet {
    pub content: String,
    pub technologies: Vec<String>,
    pub metrics: Option<String>,
}

const KNOWN_ROOT_FIELDS: &[&str] = &["bullets"];
const KNOWN_BULLET_FIELDS: &[&str] = &["content", "technologies", "metrics"];

/// Validate a raw JSON value as a bullet derivation response.
pub fn validate(data: &Value) -> ValidationResult<BulletDerivationResponse> {
    let mut warnings = Vec::new();

    // Root must be an object
    let obj = data.as_object().ok_or_else(|| ValidationError::Schema {
        field: "root".into(),
        message: "Response is not an object".into(),
    })?;

    // Warn on extra root fields
    for key in extra_fields(obj, KNOWN_ROOT_FIELDS) {
        warnings.push(Warning {
            field: format!("root.{key}"),
            message: format!("unexpected field \"{key}\""),
        });
    }

    // "bullets" must exist
    let bullets_val = obj.get("bullets").ok_or_else(|| ValidationError::Schema {
        field: "bullets".into(),
        message: "missing required field \"bullets\"".into(),
    })?;

    // "bullets" must be an array
    let bullets_arr = bullets_val
        .as_array()
        .ok_or_else(|| ValidationError::Schema {
            field: "bullets".into(),
            message: "\"bullets\" must be an array".into(),
        })?;

    // "bullets" must be non-empty
    if bullets_arr.is_empty() {
        return Err(ValidationError::Schema {
            field: "bullets".into(),
            message: "\"bullets\" must not be empty".into(),
        });
    }

    let mut bullets = Vec::with_capacity(bullets_arr.len());

    for (i, item) in bullets_arr.iter().enumerate() {
        let bullet_obj = item.as_object().ok_or_else(|| ValidationError::Schema {
            field: format!("bullets[{i}]"),
            message: "bullet must be an object".into(),
        })?;

        // Warn on extra bullet fields
        for key in extra_fields(bullet_obj, KNOWN_BULLET_FIELDS) {
            warnings.push(Warning {
                field: format!("bullets[{i}].{key}"),
                message: format!("unexpected field \"{key}\""),
            });
        }

        // content: required non-empty string
        let content = match bullet_obj.get("content") {
            Some(Value::String(s)) if !s.trim().is_empty() => s.clone(),
            Some(Value::String(_)) => {
                return Err(ValidationError::Schema {
                    field: format!("bullets[{i}].content"),
                    message: "\"content\" must not be empty or whitespace".into(),
                });
            }
            _ => {
                return Err(ValidationError::Schema {
                    field: format!("bullets[{i}].content"),
                    message: "missing or invalid \"content\" (must be a non-empty string)".into(),
                });
            }
        };

        // technologies: required array of strings
        let technologies = match bullet_obj.get("technologies") {
            Some(Value::Array(arr)) => {
                let mut techs = Vec::with_capacity(arr.len());
                for (j, t) in arr.iter().enumerate() {
                    match t.as_str() {
                        Some(s) => techs.push(s.to_string()),
                        None => {
                            return Err(ValidationError::Schema {
                                field: format!("bullets[{i}].technologies[{j}]"),
                                message: "technology item must be a string".into(),
                            });
                        }
                    }
                }
                techs
            }
            _ => {
                return Err(ValidationError::Schema {
                    field: format!("bullets[{i}].technologies"),
                    message: "missing or invalid \"technologies\" (must be an array)".into(),
                });
            }
        };

        // metrics: required, must be string or null
        let metrics = match bullet_obj.get("metrics") {
            Some(Value::String(s)) => Some(s.clone()),
            Some(Value::Null) => None,
            Some(_) => {
                return Err(ValidationError::Schema {
                    field: format!("bullets[{i}].metrics"),
                    message: "\"metrics\" must be a string or null".into(),
                });
            }
            None => {
                return Err(ValidationError::Schema {
                    field: format!("bullets[{i}].metrics"),
                    message: "missing required field \"metrics\"".into(),
                });
            }
        };

        bullets.push(DerivedBullet {
            content,
            technologies,
            metrics,
        });
    }

    // Emit tracing warnings
    for w in &warnings {
        tracing::warn!(field = %w.field, "{}", w.message);
    }

    Ok(ValidatedResponse {
        data: BulletDerivationResponse { bullets },
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
            "bullets": [
                {
                    "content": "Led migration of 12 microservices to Kubernetes",
                    "technologies": ["Kubernetes", "Docker", "Helm"],
                    "metrics": "reduced deployment time by 40%"
                },
                {
                    "content": "Implemented CI/CD pipeline for team of 8",
                    "technologies": ["GitHub Actions", "Terraform"],
                    "metrics": null
                }
            ]
        });
        let result = validate(&data).expect("should be valid");
        assert_eq!(result.data.bullets.len(), 2);
        assert_eq!(
            result.data.bullets[0].content,
            "Led migration of 12 microservices to Kubernetes"
        );
        assert_eq!(result.data.bullets[0].technologies.len(), 3);
        assert_eq!(
            result.data.bullets[0].metrics.as_deref(),
            Some("reduced deployment time by 40%")
        );
        assert!(result.data.bullets[1].metrics.is_none());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn accepts_empty_technologies() {
        let data = json!({
            "bullets": [
                {
                    "content": "Managed cross-functional team",
                    "technologies": [],
                    "metrics": null
                }
            ]
        });
        let result = validate(&data).expect("should be valid");
        assert!(result.data.bullets[0].technologies.is_empty());
    }

    #[test]
    fn warns_on_extra_root_fields() {
        let data = json!({
            "bullets": [
                {
                    "content": "Did something",
                    "technologies": [],
                    "metrics": null
                }
            ],
            "extra_field": "surprise"
        });
        let result = validate(&data).expect("should be valid");
        assert_eq!(result.warnings.len(), 1);
        assert_eq!(result.warnings[0].field, "root.extra_field");
    }

    #[test]
    fn warns_on_extra_bullet_fields() {
        let data = json!({
            "bullets": [
                {
                    "content": "Did something",
                    "technologies": [],
                    "metrics": null,
                    "confidence": 0.95
                }
            ]
        });
        let result = validate(&data).expect("should be valid");
        assert_eq!(result.warnings.len(), 1);
        assert_eq!(result.warnings[0].field, "bullets[0].confidence");
    }

    #[test]
    fn rejects_non_object_string() {
        let data = json!("not an object");
        let err = validate(&data).unwrap_err();
        match err {
            ValidationError::Schema { field, message } => {
                assert_eq!(field, "root");
                assert!(message.contains("not an object"));
            }
            _ => panic!("expected Schema error"),
        }
    }

    #[test]
    fn rejects_non_object_null() {
        let data = json!(null);
        let err = validate(&data).unwrap_err();
        match err {
            ValidationError::Schema { field, .. } => assert_eq!(field, "root"),
            _ => panic!("expected Schema error"),
        }
    }

    #[test]
    fn rejects_non_object_number() {
        let data = json!(42);
        let err = validate(&data).unwrap_err();
        match err {
            ValidationError::Schema { field, .. } => assert_eq!(field, "root"),
            _ => panic!("expected Schema error"),
        }
    }

    #[test]
    fn rejects_missing_bullets() {
        let data = json!({});
        let err = validate(&data).unwrap_err();
        match err {
            ValidationError::Schema { field, message } => {
                assert_eq!(field, "bullets");
                assert!(message.contains("missing"));
            }
            _ => panic!("expected Schema error"),
        }
    }

    #[test]
    fn rejects_non_array_bullets() {
        let data = json!({ "bullets": "not an array" });
        let err = validate(&data).unwrap_err();
        match err {
            ValidationError::Schema { field, message } => {
                assert_eq!(field, "bullets");
                assert!(message.contains("array"));
            }
            _ => panic!("expected Schema error"),
        }
    }

    #[test]
    fn rejects_empty_bullets() {
        let data = json!({ "bullets": [] });
        let err = validate(&data).unwrap_err();
        match err {
            ValidationError::Schema { field, message } => {
                assert_eq!(field, "bullets");
                assert!(message.contains("empty"));
            }
            _ => panic!("expected Schema error"),
        }
    }

    #[test]
    fn rejects_missing_content() {
        let data = json!({
            "bullets": [
                {
                    "technologies": [],
                    "metrics": null
                }
            ]
        });
        let err = validate(&data).unwrap_err();
        match err {
            ValidationError::Schema { field, message } => {
                assert_eq!(field, "bullets[0].content");
                assert!(message.contains("content"));
            }
            _ => panic!("expected Schema error"),
        }
    }

    #[test]
    fn rejects_empty_content() {
        let data = json!({
            "bullets": [
                {
                    "content": "   ",
                    "technologies": [],
                    "metrics": null
                }
            ]
        });
        let err = validate(&data).unwrap_err();
        match err {
            ValidationError::Schema { field, message } => {
                assert_eq!(field, "bullets[0].content");
                assert!(message.contains("empty") || message.contains("whitespace"));
            }
            _ => panic!("expected Schema error"),
        }
    }

    #[test]
    fn rejects_non_array_technologies() {
        let data = json!({
            "bullets": [
                {
                    "content": "Did something",
                    "technologies": "Rust",
                    "metrics": null
                }
            ]
        });
        let err = validate(&data).unwrap_err();
        match err {
            ValidationError::Schema { field, message } => {
                assert_eq!(field, "bullets[0].technologies");
                assert!(message.contains("array"));
            }
            _ => panic!("expected Schema error"),
        }
    }

    #[test]
    fn rejects_non_string_technology_item() {
        let data = json!({
            "bullets": [
                {
                    "content": "Did something",
                    "technologies": ["Rust", 42],
                    "metrics": null
                }
            ]
        });
        let err = validate(&data).unwrap_err();
        match err {
            ValidationError::Schema { field, message } => {
                assert_eq!(field, "bullets[0].technologies[1]");
                assert!(message.contains("string"));
            }
            _ => panic!("expected Schema error"),
        }
    }

    #[test]
    fn rejects_missing_metrics() {
        let data = json!({
            "bullets": [
                {
                    "content": "Did something",
                    "technologies": []
                }
            ]
        });
        let err = validate(&data).unwrap_err();
        match err {
            ValidationError::Schema { field, message } => {
                assert_eq!(field, "bullets[0].metrics");
                assert!(message.contains("missing"));
            }
            _ => panic!("expected Schema error"),
        }
    }

    #[test]
    fn rejects_non_string_non_null_metrics() {
        let data = json!({
            "bullets": [
                {
                    "content": "Did something",
                    "technologies": [],
                    "metrics": 123
                }
            ]
        });
        let err = validate(&data).unwrap_err();
        match err {
            ValidationError::Schema { field, message } => {
                assert_eq!(field, "bullets[0].metrics");
                assert!(message.contains("string or null"));
            }
            _ => panic!("expected Schema error"),
        }
    }
}
