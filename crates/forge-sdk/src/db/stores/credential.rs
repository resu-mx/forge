//! Credential repository — CRUD for the `credentials` table.
//!
//! Credentials are non-certification qualifications: clearances,
//! driver's licenses, bar admissions, medical licenses.
//!
//! Validation mirrors `packages/core/src/services/credential-service.ts`:
//! a non-empty label, a `details` object carrying the fields its
//! `credential_type` requires, an existing organization, and partial
//! `details` updates merged over the stored object.

use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;

use forge_core::{
    new_id, now_iso, CreateCredential, Credential, CredentialStatus, CredentialType, ForgeError,
    UpdateCredential,
};

const VALID_TYPES: &str = "clearance, drivers_license, bar_admission, medical_license";

fn validation(message: impl Into<String>, field: &str) -> ForgeError {
    ForgeError::Validation {
        message: message.into(),
        field: Some(field.into()),
    }
}

/// JavaScript truthiness, for TS's `if (!input.details)`.
fn is_falsy(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::Bool(b) => !b,
        Value::Number(n) => n.as_f64() == Some(0.0),
        Value::String(s) => s.is_empty(),
        Value::Array(_) | Value::Object(_) => false,
    }
}

/// The fields each `credential_type` requires in `details`, and whether a
/// whitespace-only value counts as missing (credential-service.ts `validateDetails`;
/// only `clearance_type` skips the trim check there).
fn required_detail_fields(t: CredentialType) -> &'static [(&'static str, bool)] {
    match t {
        CredentialType::Clearance => &[("level", true), ("clearance_type", false)],
        CredentialType::DriversLicense => &[("class", true), ("state", true)],
        CredentialType::BarAdmission => &[("jurisdiction", true)],
        CredentialType::MedicalLicense => &[("license_type", true), ("state", true)],
    }
}

/// Check the type-specific details payload; the error names the first missing field.
/// A payload that is not an object (a JSON string, say) has none of the fields.
fn validate_details(t: CredentialType, details: &Value) -> Result<(), ForgeError> {
    for &(field, trimmed) in required_detail_fields(t) {
        let ok = match details.get(field) {
            Some(Value::String(s)) => !(if trimmed { s.trim() } else { s.as_str() }).is_empty(),
            _ => false,
        };
        if !ok {
            return Err(validation(
                format!("{} details.{field} is required", t.as_ref()),
                "details",
            ));
        }
    }
    Ok(())
}

/// TS `{ ...existing, ...patch }`: the patch's keys win. A patch that is not an
/// object adds nothing (spreading `null`, a number or a boolean is a no-op).
fn merge_details(existing: &Value, patch: &Value) -> Value {
    let mut merged = match existing {
        Value::Object(map) => map.clone(),
        _ => serde_json::Map::new(),
    };
    if let Value::Object(map) = patch {
        for (k, v) in map {
            merged.insert(k.clone(), v.clone());
        }
    }
    Value::Object(merged)
}

/// The trimmed label, or the TS validation error.
fn non_empty_label<'a>(label: &'a str, message: &str) -> Result<&'a str, ForgeError> {
    let trimmed = label.trim();
    if trimmed.is_empty() {
        return Err(validation(message, "label"));
    }
    Ok(trimmed)
}

/// 404 for an organization id that does not exist, as TS checks it before writing.
fn ensure_organization(conn: &Connection, org_id: &str) -> Result<(), ForgeError> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM organizations WHERE id = ?1)",
        params![org_id],
        |row| row.get(0),
    )?;
    if !exists {
        return Err(ForgeError::NotFound {
            entity_type: "Organization".into(),
            id: org_id.into(),
        });
    }
    Ok(())
}

fn to_json_text(value: &Value) -> Result<String, ForgeError> {
    serde_json::to_string(value).map_err(|e| ForgeError::Internal(e.to_string()))
}

/// Data access for the `credentials` table.
pub struct CredentialStore;

impl CredentialStore {
    // ── Create ───────────────────────────────────────────────────────

    pub fn create(conn: &Connection, input: &CreateCredential) -> Result<Credential, ForgeError> {
        let label = non_empty_label(&input.label, "label is required")?;
        let details = match &input.details {
            Some(d) if !is_falsy(d) => d,
            _ => return Err(validation("details is required", "details")),
        };
        validate_details(input.credential_type, details)?;
        if let Some(org_id) = input.organization_id.as_deref().filter(|s| !s.is_empty()) {
            ensure_organization(conn, org_id)?;
        }

        let id = new_id();
        let now = now_iso();
        let status = input.status.unwrap_or(CredentialStatus::Active);
        let details = to_json_text(details)?;

        conn.execute(
            "INSERT INTO credentials (id, credential_type, label, status, organization_id, details, issued_date, expiry_date, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
            params![
                id,
                input.credential_type.as_ref(),
                label,
                status.as_ref(),
                input.organization_id,
                details,
                input.issued_date,
                input.expiry_date,
                now,
            ],
        )?;

        Self::get(conn, &id)?
            .ok_or_else(|| ForgeError::Internal("Credential created but not found".into()))
    }

    // ── Read ─────────────────────────────────────────────────────────

    pub fn get(conn: &Connection, id: &str) -> Result<Option<Credential>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT id, credential_type, label, status, organization_id, details,
                    issued_date, expiry_date, created_at, updated_at
             FROM credentials WHERE id = ?1",
        )?;
        let result = stmt
            .query_row(params![id], Self::map_credential)
            .optional()?;
        Ok(result)
    }

    /// List all credentials, ordered by type then label (as TS lists them).
    pub fn list(conn: &Connection) -> Result<Vec<Credential>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT id, credential_type, label, status, organization_id, details,
                    issued_date, expiry_date, created_at, updated_at
             FROM credentials ORDER BY credential_type ASC, label ASC",
        )?;
        let rows: Vec<Credential> = stmt
            .query_map([], Self::map_credential)?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    /// List credentials of one type, ordered by label. An unknown type is a
    /// validation error, as in TS.
    pub fn find_by_type(conn: &Connection, cred_type: &str) -> Result<Vec<Credential>, ForgeError> {
        if cred_type.parse::<CredentialType>().is_err() {
            return Err(validation(
                format!("Invalid credential_type '{cred_type}'. Valid: {VALID_TYPES}"),
                "credential_type",
            ));
        }
        let mut stmt = conn.prepare(
            "SELECT id, credential_type, label, status, organization_id, details,
                    issued_date, expiry_date, created_at, updated_at
             FROM credentials WHERE credential_type = ?1
             ORDER BY label ASC",
        )?;
        let rows: Vec<Credential> = stmt
            .query_map(params![cred_type], Self::map_credential)?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    // ── Update ───────────────────────────────────────────────────────

    pub fn update(
        conn: &Connection,
        id: &str,
        input: &UpdateCredential,
    ) -> Result<Credential, ForgeError> {
        // Validate first, as TS does; then 404.
        let label = input
            .label
            .as_deref()
            .map(|v| non_empty_label(v, "label must not be empty"))
            .transpose()?;
        if let Some(Some(org_id)) = &input.organization_id {
            if !org_id.is_empty() {
                ensure_organization(conn, org_id)?;
            }
        }

        let existing = Self::get(conn, id)?.ok_or_else(|| ForgeError::NotFound {
            entity_type: "credential".into(),
            id: id.into(),
        })?;

        let details = match &input.details {
            Some(patch) => {
                let merged = merge_details(&existing.details, patch);
                validate_details(existing.credential_type, &merged)?;
                Some(to_json_text(&merged)?)
            }
            None => None,
        };

        let mut sets = Vec::new();
        let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(v) = label {
            sets.push(format!("label = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.to_string()));
        }
        if let Some(ref v) = input.status {
            sets.push(format!("status = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.as_ref().to_string()));
        }
        if let Some(ref v) = input.organization_id {
            sets.push(format!("organization_id = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(v) = details {
            sets.push(format!("details = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v));
        }
        if let Some(ref v) = input.issued_date {
            sets.push(format!("issued_date = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = input.expiry_date {
            sets.push(format!("expiry_date = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }

        if !sets.is_empty() {
            let now = now_iso();
            sets.push(format!("updated_at = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(now));

            let sql = format!(
                "UPDATE credentials SET {} WHERE id = ?{}",
                sets.join(", "),
                bind_values.len() + 1
            );
            bind_values.push(Box::new(id.to_string()));

            conn.execute(
                &sql,
                rusqlite::params_from_iter(bind_values.iter().map(|b| b.as_ref())),
            )?;
        }

        Self::get(conn, id)?
            .ok_or_else(|| ForgeError::Internal("Credential updated but not found".into()))
    }

    // ── Delete ───────────────────────────────────────────────────────

    pub fn delete(conn: &Connection, id: &str) -> Result<(), ForgeError> {
        let deleted = conn.execute("DELETE FROM credentials WHERE id = ?1", params![id])?;
        if deleted == 0 {
            return Err(ForgeError::NotFound {
                entity_type: "credential".into(),
                id: id.into(),
            });
        }
        Ok(())
    }

    // ── Row mapping ──────────────────────────────────────────────────

    fn map_credential(row: &rusqlite::Row) -> rusqlite::Result<Credential> {
        Ok(Credential {
            id: row.get(0)?,
            credential_type: row
                .get::<_, String>(1)?
                .parse()
                .unwrap_or(CredentialType::Clearance),
            label: row.get(2)?,
            status: row
                .get::<_, String>(3)?
                .parse()
                .unwrap_or(CredentialStatus::Active),
            organization_id: row.get(4)?,
            details: {
                // TS's ELM parses a `json` column and falls back to the raw text.
                let raw: String = row.get(5)?;
                serde_json::from_str(&raw).unwrap_or(Value::String(raw))
            },
            issued_date: row.get(6)?,
            expiry_date: row.get(7)?,
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::Forge;
    use serde_json::json;

    fn setup() -> Forge {
        Forge::open_memory().unwrap()
    }

    /// The smallest valid details object for each type.
    fn details_for(t: CredentialType) -> Value {
        match t {
            CredentialType::Clearance => json!({"level": "secret", "clearance_type": "personnel"}),
            CredentialType::DriversLicense => {
                json!({"class": "C", "state": "OR", "endorsements": []})
            }
            CredentialType::BarAdmission => json!({"jurisdiction": "Oregon"}),
            CredentialType::MedicalLicense => json!({"license_type": "MD", "state": "OR"}),
        }
    }

    fn input(t: CredentialType, label: &str) -> CreateCredential {
        CreateCredential {
            credential_type: t,
            label: label.into(),
            status: None,
            organization_id: None,
            details: Some(details_for(t)),
            issued_date: None,
            expiry_date: None,
        }
    }

    fn validation_message(err: ForgeError) -> String {
        match err {
            ForgeError::Validation { message, .. } => message,
            other => panic!("expected a validation error, got {other:?}"),
        }
    }

    #[test]
    fn create_and_get_credential() {
        let forge = setup();
        let cred = CredentialStore::create(
            forge.conn(),
            &CreateCredential {
                credential_type: CredentialType::Clearance,
                label: "TS/SCI".into(),
                status: Some(CredentialStatus::Active),
                organization_id: None,
                details: Some(json!({
                    "level": "top_secret",
                    "polygraph": "full_scope",
                    "clearance_type": "personnel",
                    "access_programs": ["sci"],
                })),
                issued_date: Some("2020-01-15".into()),
                expiry_date: Some("2025-01-15".into()),
            },
        )
        .unwrap();

        assert_eq!(cred.label, "TS/SCI");
        assert_eq!(cred.credential_type, CredentialType::Clearance);
        assert_eq!(cred.status, CredentialStatus::Active);
        assert_eq!(cred.details["level"], "top_secret");
        assert_eq!(cred.details["access_programs"], json!(["sci"]));

        let fetched = CredentialStore::get(forge.conn(), &cred.id)
            .unwrap()
            .unwrap();
        assert_eq!(fetched.id, cred.id);
        assert_eq!(fetched.details, cred.details);
    }

    #[test]
    fn details_are_stored_as_json_text() {
        let forge = setup();
        let cred =
            CredentialStore::create(forge.conn(), &input(CredentialType::DriversLicense, "DL"))
                .unwrap();
        let raw: String = forge
            .conn()
            .query_row(
                "SELECT details FROM credentials WHERE id = ?1",
                params![cred.id],
                |r| r.get(0),
            )
            .unwrap();
        let parsed: Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(parsed, details_for(CredentialType::DriversLicense));
    }

    #[test]
    fn unparseable_stored_details_come_back_as_text() {
        let forge = setup();
        let cred =
            CredentialStore::create(forge.conn(), &input(CredentialType::BarAdmission, "Bar"))
                .unwrap();
        forge
            .conn()
            .execute(
                "UPDATE credentials SET details = 'not json' WHERE id = ?1",
                params![cred.id],
            )
            .unwrap();
        let fetched = CredentialStore::get(forge.conn(), &cred.id)
            .unwrap()
            .unwrap();
        assert_eq!(fetched.details, json!("not json"));
    }

    #[test]
    fn create_trims_the_label() {
        let forge = setup();
        let cred = CredentialStore::create(
            forge.conn(),
            &input(CredentialType::Clearance, "  Secret  "),
        )
        .unwrap();
        assert_eq!(cred.label, "Secret");
    }

    #[test]
    fn create_rejects_an_empty_label() {
        let forge = setup();
        let err = CredentialStore::create(forge.conn(), &input(CredentialType::Clearance, "  "))
            .unwrap_err();
        assert_eq!(validation_message(err), "label is required");
    }

    #[test]
    fn create_requires_details() {
        let forge = setup();
        for missing in [None, Some(Value::Null), Some(json!(""))] {
            let mut i = input(CredentialType::Clearance, "Secret");
            i.details = missing;
            let err = CredentialStore::create(forge.conn(), &i).unwrap_err();
            assert_eq!(validation_message(err), "details is required");
        }
    }

    #[test]
    fn create_names_the_first_missing_detail_field() {
        let forge = setup();
        let cases = [
            (
                CredentialType::Clearance,
                json!({"clearance_type": "personnel"}),
                "clearance details.level is required",
            ),
            (
                CredentialType::Clearance,
                json!({"level": "secret"}),
                "clearance details.clearance_type is required",
            ),
            (
                CredentialType::Clearance,
                json!({"level": "  ", "clearance_type": "personnel"}),
                "clearance details.level is required",
            ),
            (
                CredentialType::DriversLicense,
                json!({"state": "OR"}),
                "drivers_license details.class is required",
            ),
            (
                CredentialType::DriversLicense,
                json!({"class": "C"}),
                "drivers_license details.state is required",
            ),
            (
                CredentialType::BarAdmission,
                json!({}),
                "bar_admission details.jurisdiction is required",
            ),
            (
                CredentialType::MedicalLicense,
                json!({"state": "OR"}),
                "medical_license details.license_type is required",
            ),
            (
                CredentialType::MedicalLicense,
                json!({"license_type": "MD"}),
                "medical_license details.state is required",
            ),
        ];
        for (t, details, expected) in cases {
            let mut i = input(t, "X");
            i.details = Some(details);
            let err = CredentialStore::create(forge.conn(), &i).unwrap_err();
            assert_eq!(validation_message(err), expected);
        }
    }

    #[test]
    fn create_rejects_details_sent_as_a_json_string() {
        // TS validates fields on the value it receives, so a stringified object has none.
        let forge = setup();
        let mut i = input(CredentialType::DriversLicense, "DL");
        i.details = Some(json!(r#"{"class":"C","state":"OR"}"#));
        let err = CredentialStore::create(forge.conn(), &i).unwrap_err();
        assert_eq!(
            validation_message(err),
            "drivers_license details.class is required"
        );
    }

    #[test]
    fn create_with_unknown_organization_is_not_found() {
        let forge = setup();
        let mut i = input(CredentialType::Clearance, "Secret");
        i.organization_id = Some(new_id());
        let err = CredentialStore::create(forge.conn(), &i).unwrap_err();
        assert!(matches!(err, ForgeError::NotFound { .. }));
    }

    #[test]
    fn get_returns_none_for_missing() {
        let forge = setup();
        assert!(CredentialStore::get(forge.conn(), "nonexistent")
            .unwrap()
            .is_none());
    }

    #[test]
    fn list_orders_by_type_then_label() {
        let forge = setup();
        for (t, label) in [
            (CredentialType::DriversLicense, "TX CDL"),
            (CredentialType::Clearance, "TS"),
            (CredentialType::Clearance, "Secret"),
        ] {
            CredentialStore::create(forge.conn(), &input(t, label)).unwrap();
        }

        let labels: Vec<String> = CredentialStore::list(forge.conn())
            .unwrap()
            .into_iter()
            .map(|c| c.label)
            .collect();
        assert_eq!(labels, ["Secret", "TS", "TX CDL"]);
    }

    #[test]
    fn find_by_type_filters() {
        let forge = setup();
        CredentialStore::create(forge.conn(), &input(CredentialType::Clearance, "Secret")).unwrap();
        CredentialStore::create(
            forge.conn(),
            &input(CredentialType::DriversLicense, "TX CDL"),
        )
        .unwrap();

        let clearances = CredentialStore::find_by_type(forge.conn(), "clearance").unwrap();
        assert_eq!(clearances.len(), 1);
        assert_eq!(clearances[0].label, "Secret");
    }

    #[test]
    fn find_by_unknown_type_is_a_validation_error() {
        let forge = setup();
        let err = CredentialStore::find_by_type(forge.conn(), "bogus").unwrap_err();
        assert!(validation_message(err).starts_with("Invalid credential_type 'bogus'"));
    }

    #[test]
    fn update_credential() {
        let forge = setup();
        let cred =
            CredentialStore::create(forge.conn(), &input(CredentialType::Clearance, "Secret"))
                .unwrap();

        let updated = CredentialStore::update(
            forge.conn(),
            &cred.id,
            &UpdateCredential {
                label: Some(" TS/SCI ".into()),
                status: Some(CredentialStatus::Expired),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(updated.label, "TS/SCI");
        assert_eq!(updated.status, CredentialStatus::Expired);
        assert_eq!(updated.details, details_for(CredentialType::Clearance));
    }

    #[test]
    fn update_merges_partial_details() {
        let forge = setup();
        let cred =
            CredentialStore::create(forge.conn(), &input(CredentialType::Clearance, "Secret"))
                .unwrap();

        let updated = CredentialStore::update(
            forge.conn(),
            &cred.id,
            &UpdateCredential {
                details: Some(json!({"polygraph": "full_scope", "level": "top_secret"})),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(
            updated.details,
            json!({"level": "top_secret", "clearance_type": "personnel", "polygraph": "full_scope"})
        );
    }

    #[test]
    fn update_validates_the_merged_details() {
        let forge = setup();
        let cred =
            CredentialStore::create(forge.conn(), &input(CredentialType::DriversLicense, "DL"))
                .unwrap();
        let err = CredentialStore::update(
            forge.conn(),
            &cred.id,
            &UpdateCredential {
                details: Some(json!({"state": ""})),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert_eq!(
            validation_message(err),
            "drivers_license details.state is required"
        );
    }

    #[test]
    fn update_rejects_an_empty_label() {
        let forge = setup();
        let cred =
            CredentialStore::create(forge.conn(), &input(CredentialType::Clearance, "Secret"))
                .unwrap();
        let err = CredentialStore::update(
            forge.conn(),
            &cred.id,
            &UpdateCredential {
                label: Some("   ".into()),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert_eq!(validation_message(err), "label must not be empty");
    }

    #[test]
    fn update_with_unknown_organization_is_not_found() {
        let forge = setup();
        let cred =
            CredentialStore::create(forge.conn(), &input(CredentialType::Clearance, "Secret"))
                .unwrap();
        let err = CredentialStore::update(
            forge.conn(),
            &cred.id,
            &UpdateCredential {
                organization_id: Some(Some(new_id())),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert!(matches!(err, ForgeError::NotFound { .. }));
    }

    #[test]
    fn update_missing_returns_not_found() {
        let forge = setup();
        let result =
            CredentialStore::update(forge.conn(), "nonexistent", &UpdateCredential::default());
        assert!(matches!(result, Err(ForgeError::NotFound { .. })));
    }

    #[test]
    fn delete_credential() {
        let forge = setup();
        let cred =
            CredentialStore::create(forge.conn(), &input(CredentialType::BarAdmission, "TX Bar"))
                .unwrap();

        CredentialStore::delete(forge.conn(), &cred.id).unwrap();
        assert!(CredentialStore::get(forge.conn(), &cred.id)
            .unwrap()
            .is_none());
    }

    #[test]
    fn delete_missing_returns_not_found() {
        let forge = setup();
        assert!(matches!(
            CredentialStore::delete(forge.conn(), "nonexistent"),
            Err(ForgeError::NotFound { .. })
        ));
    }

    #[test]
    fn default_status_is_active() {
        let forge = setup();
        let cred = CredentialStore::create(
            forge.conn(),
            &input(CredentialType::MedicalLicense, "MD License"),
        )
        .unwrap();
        assert_eq!(cred.status, CredentialStatus::Active);
    }
}
