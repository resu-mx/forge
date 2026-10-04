//! Serde conformance tests — verify Rust types produce the same JSON
//! shapes as the TS `packages/core/src/types/index.ts` wire format.

#[cfg(test)]
mod enum_roundtrip {
    use std::str::FromStr;

    use super::super::enums::*;

    macro_rules! roundtrip {
        ($ty:ty, $variant:expr, $expected:expr) => {{
            // serde JSON round-trip
            let json = serde_json::to_string(&$variant).unwrap();
            assert_eq!(json, format!("\"{}\"", $expected));
            let back: $ty = serde_json::from_str(&json).unwrap();
            assert_eq!(back, $variant);

            // strum Display + FromStr round-trip
            let display = $variant.to_string();
            assert_eq!(display, $expected);
            let parsed: $ty = <$ty>::from_str(&display).unwrap();
            assert_eq!(parsed, $variant);

            // strum AsRef<str>
            assert_eq!($variant.as_ref(), $expected);
        }};
    }

    #[test]
    fn source_status() {
        roundtrip!(SourceStatus, SourceStatus::Draft, "draft");
        roundtrip!(SourceStatus, SourceStatus::InReview, "in_review");
        roundtrip!(SourceStatus, SourceStatus::Approved, "approved");
        roundtrip!(SourceStatus, SourceStatus::Deriving, "deriving");
    }

    #[test]
    fn bullet_status() {
        roundtrip!(BulletStatus, BulletStatus::Draft, "draft");
        roundtrip!(BulletStatus, BulletStatus::Archived, "archived");
    }

    #[test]
    fn framing() {
        roundtrip!(Framing, Framing::Accomplishment, "accomplishment");
        roundtrip!(Framing, Framing::Responsibility, "responsibility");
        roundtrip!(Framing, Framing::Context, "context");
    }

    #[test]
    fn skill_category() {
        roundtrip!(SkillCategory, SkillCategory::AiMl, "ai_ml");
        roundtrip!(SkillCategory, SkillCategory::SoftSkill, "soft_skill");
        roundtrip!(SkillCategory, SkillCategory::DataSystems, "data_systems");
    }

    #[test]
    fn source_type() {
        roundtrip!(SourceType, SourceType::Role, "role");
        roundtrip!(SourceType, SourceType::Education, "education");
        roundtrip!(SourceType, SourceType::Presentation, "presentation");
    }

    #[test]
    fn resume_section() {
        roundtrip!(ResumeSection, ResumeSection::Experience, "experience");
        roundtrip!(ResumeSection, ResumeSection::WorkHistory, "work_history");
        roundtrip!(
            ResumeSection,
            ResumeSection::Certifications,
            "certifications"
        );
    }

    #[test]
    fn location_modality() {
        roundtrip!(LocationModality, LocationModality::InPerson, "in_person");
        roundtrip!(LocationModality, LocationModality::Remote, "remote");
    }

    #[test]
    fn job_description_status() {
        roundtrip!(
            JobDescriptionStatus,
            JobDescriptionStatus::Discovered,
            "discovered"
        );
        roundtrip!(
            JobDescriptionStatus,
            JobDescriptionStatus::Interviewing,
            "interviewing"
        );
        roundtrip!(
            JobDescriptionStatus,
            JobDescriptionStatus::Withdrawn,
            "withdrawn"
        );
    }

    #[test]
    fn clearance_level() {
        roundtrip!(ClearanceLevel, ClearanceLevel::TopSecret, "top_secret");
        roundtrip!(ClearanceLevel, ClearanceLevel::Q, "q");
    }

    #[test]
    fn education_type() {
        roundtrip!(EducationType, EducationType::SelfTaught, "self_taught");
        roundtrip!(EducationType, EducationType::Degree, "degree");
    }

    #[test]
    fn presentation_type() {
        roundtrip!(
            PresentationType,
            PresentationType::ConferenceTalk,
            "conference_talk"
        );
        roundtrip!(
            PresentationType,
            PresentationType::LightningTalk,
            "lightning_talk"
        );
    }

    #[test]
    fn org_tag() {
        roundtrip!(OrgTag, OrgTag::Company, "company");
        roundtrip!(OrgTag, OrgTag::Nonprofit, "nonprofit");
    }

    #[test]
    fn invalid_fromstr_fails() {
        assert!(SourceStatus::from_str("nonexistent").is_err());
        assert!(SkillCategory::from_str("").is_err());
    }
}

#[cfg(test)]
mod tagged_unions {
    use super::super::common::Gap;
    use super::super::ir::{ClearanceItem, IRSectionItem, SummaryItem};

    #[test]
    fn gap_tagged_by_type() {
        let gap = Gap::ThinCoverage {
            domain: "security".into(),
            current_count: 1,
            description: "Only 1 bullet".into(),
            recommendation: "Add more".into(),
        };
        let json = serde_json::to_value(&gap).unwrap();
        assert_eq!(json["type"], "thin_coverage");
        assert_eq!(json["domain"], "security");

        // round-trip
        let back: Gap = serde_json::from_value(json).unwrap();
        match back {
            Gap::ThinCoverage {
                domain,
                current_count,
                ..
            } => {
                assert_eq!(domain, "security");
                assert_eq!(current_count, 1);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn gap_missing_domain_tagged() {
        let gap = Gap::MissingDomain {
            domain: "cloud".into(),
            description: "No coverage".into(),
            available_bullets: vec![],
            recommendation: "Create bullets".into(),
        };
        let json = serde_json::to_value(&gap).unwrap();
        assert_eq!(json["type"], "missing_domain_coverage");
    }

    #[test]
    fn ir_section_item_tagged_by_kind() {
        let item = IRSectionItem::Summary(SummaryItem {
            content: "Experienced engineer".into(),
            entry_id: None,
        });
        let json = serde_json::to_value(&item).unwrap();
        assert_eq!(json["kind"], "summary");
        assert_eq!(json["content"], "Experienced engineer");

        // round-trip
        let back: IRSectionItem = serde_json::from_value(json).unwrap();
        match back {
            IRSectionItem::Summary(s) => assert_eq!(s.content, "Experienced engineer"),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn ir_clearance_item_tagged() {
        let item = IRSectionItem::Clearance(ClearanceItem {
            content: "Active TS/SCI".into(),
            entry_id: Some("e1".into()),
            source_id: None,
        });
        let json = serde_json::to_value(&item).unwrap();
        assert_eq!(json["kind"], "clearance");
        assert_eq!(json["source_id"], serde_json::Value::Null);
    }
}

#[cfg(test)]
mod error_serialization {
    use super::super::common::ForgeError;

    #[test]
    fn not_found_serializes_to_code_message() {
        let err = ForgeError::NotFound {
            entity_type: "source".into(),
            id: "abc-123".into(),
        };
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["code"], "NOT_FOUND");
        assert_eq!(json["message"], "source not found: abc-123");
    }

    #[test]
    fn validation_error_serializes() {
        let err = ForgeError::Validation {
            message: "title is required".into(),
            field: Some("title".into()),
        };
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["code"], "VALIDATION_ERROR");
        assert!(json["message"]
            .as_str()
            .unwrap()
            .contains("title is required"));
    }

    #[test]
    fn database_error_code() {
        let err = ForgeError::Internal("unexpected".into());
        assert_eq!(err.code(), "INTERNAL_ERROR");
    }

    #[test]
    fn wasm_database_error_constructs_and_displays() {
        let err = ForgeError::WasmDatabase("OPFS open failed: ENOENT".into());
        assert_eq!(err.code(), "DATABASE_ERROR");
        assert!(err.to_string().contains("OPFS open failed: ENOENT"));
    }

    #[test]
    fn wasm_database_error_serializes_with_database_error_code() {
        let err = ForgeError::WasmDatabase("connection lost".into());
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["code"], "DATABASE_ERROR");
        assert!(json["message"]
            .as_str()
            .unwrap()
            .contains("connection lost"));
    }
}

#[cfg(test)]
mod entity_serde {
    use super::super::entities::Source;
    use super::super::enums::{SourceStatus, SourceType, UpdatedBy};

    #[test]
    fn source_round_trip() {
        let source = Source {
            id: "s1".into(),
            title: "Backend Dev".into(),
            description: "Built APIs".into(),
            source_type: SourceType::Role,
            start_date: Some("2024-01-01".into()),
            end_date: None,
            status: SourceStatus::Approved,
            updated_by: UpdatedBy::Human,
            last_derived_at: None,
            created_at: "2024-01-01T00:00:00Z".into(),
            updated_at: "2024-01-01T00:00:00Z".into(),
        };
        let json = serde_json::to_value(&source).unwrap();
        assert_eq!(json["source_type"], "role");
        assert_eq!(json["status"], "approved");
        assert_eq!(json["end_date"], serde_json::Value::Null);

        let back: Source = serde_json::from_value(json).unwrap();
        assert_eq!(back.id, "s1");
        assert_eq!(back.source_type, SourceType::Role);
        assert!(back.end_date.is_none());
    }
}

#[cfg(test)]
mod extension_wire {
    use serde_json::json;

    use super::super::common::*;
    use super::super::entities::*;
    use super::super::inputs::*;

    #[test]
    fn config_defaults_match_ts() {
        assert_eq!(
            serde_json::to_value(ExtensionConfig::default()).unwrap(),
            json!({
                "baseUrl": "http://localhost:3000",
                "devMode": false,
                "enabledPlugins": ["linkedin"],
                "enableServerLogging": true
            })
        );
    }

    #[test]
    fn config_keys_match_wire_field_names() {
        let v = serde_json::to_value(ExtensionConfig::default()).unwrap();
        let mut keys: Vec<_> = v.as_object().unwrap().keys().cloned().collect();
        keys.sort();
        let mut expected: Vec<_> = EXTENSION_CONFIG_KEYS.iter().map(|k| k.to_string()).collect();
        expected.sort();
        assert_eq!(keys, expected);
    }

    #[test]
    fn config_round_trips_any_json_value() {
        let wire = json!({
            "baseUrl": "yes",
            "devMode": 1,
            "enabledPlugins": null,
            "enableServerLogging": {}
        });
        let cfg: ExtensionConfig = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(serde_json::to_value(&cfg).unwrap(), wire);

        let wire = json!({
            "baseUrl": ["a"],
            "devMode": "yes",
            "enabledPlugins": 3.5,
            "enableServerLogging": false
        });
        let cfg: ExtensionConfig = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(serde_json::to_value(&cfg).unwrap(), wire);
    }

    #[test]
    fn log_absent_fields_are_null_not_omitted() {
        let log = ExtensionLog {
            id: "00000000-0000-4000-8000-000000000000".into(),
            error_code: "API_UNREACHABLE".into(),
            message: "Failed to fetch".into(),
            layer: "sdk".into(),
            plugin: None,
            url: None,
            context: None,
            created_at: "2026-10-02T12:00:00Z".into(),
        };
        let v = serde_json::to_value(&log).unwrap();
        for key in ["plugin", "url", "context"] {
            assert!(v.as_object().unwrap().contains_key(key), "{key} missing");
            assert!(v[key].is_null());
        }
    }

    #[test]
    fn log_context_serialises_as_json_value_not_string() {
        let log = ExtensionLog {
            id: "i".into(),
            error_code: "E".into(),
            message: "m".into(),
            layer: "plugin".into(),
            plugin: Some("linkedin".into()),
            url: Some("https://x".into()),
            context: Some(json!({ "step": "extract_title" })),
            created_at: "t".into(),
        };
        let v = serde_json::to_value(&log).unwrap();
        assert_eq!(v["context"], json!({ "step": "extract_title" }));
    }

    #[test]
    fn create_log_tolerates_missing_and_unknown_fields() {
        let empty: CreateExtensionLog = serde_json::from_value(json!({})).unwrap();
        assert!(empty.error_code.is_none() && empty.context.is_none());

        let full: CreateExtensionLog = serde_json::from_value(json!({
            "error_code": "PLUGIN_THREW", "message": "parse error", "layer": "plugin",
            "plugin": "linkedin", "url": "https://linkedin.com/jobs/123",
            "context": { "step": "extract_title" }, "timestamp": "ignored"
        }))
        .unwrap();
        assert_eq!(full.context, Some(json!({ "step": "extract_title" })));
        assert_eq!(full.error_code.as_deref(), Some("PLUGIN_THREW"));
    }

    #[test]
    fn log_filter_default_is_all_none() {
        let f = ExtensionLogFilter::default();
        assert!(f.limit.is_none() && f.offset.is_none() && f.error_code.is_none() && f.layer.is_none());
    }

    #[test]
    fn types_reachable_from_crate_root() {
        let _ = crate::ExtensionConfig::default();
        let _ = crate::CreateExtensionLog::default();
        let _ = crate::ExtensionLogFilter::default();
        assert_eq!(crate::EXTENSION_CONFIG_KEYS.len(), 4);
    }
}
