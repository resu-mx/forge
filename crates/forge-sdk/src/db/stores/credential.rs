//! Credential repository — CRUD for the `credentials` table.
//!
//! Credentials are non-certification qualifications: clearances,
//! driver's licenses, bar admissions, medical licenses.

use rusqlite::{params, Connection, OptionalExtension};

use forge_core::{
    new_id, now_iso, CreateCredential, Credential, CredentialStatus, CredentialType, ForgeError,
    UpdateCredential,
};

/// Data access for the `credentials` table.
pub struct CredentialStore;

impl CredentialStore {
    // ── Create ───────────────────────────────────────────────────────

    pub fn create(conn: &Connection, input: &CreateCredential) -> Result<Credential, ForgeError> {
        let id = new_id();
        let now = now_iso();
        let status = input.status.unwrap_or(CredentialStatus::Active);
        let details = input.details.as_deref().unwrap_or("{}");

        conn.execute(
            "INSERT INTO credentials (id, credential_type, label, status, organization_id, details, issued_date, expiry_date, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
            params![
                id,
                input.credential_type.as_ref(),
                input.label,
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

    /// List all credentials.
    pub fn list(conn: &Connection) -> Result<Vec<Credential>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT id, credential_type, label, status, organization_id, details,
                    issued_date, expiry_date, created_at, updated_at
             FROM credentials ORDER BY created_at DESC",
        )?;
        let rows: Vec<Credential> = stmt
            .query_map([], Self::map_credential)?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    /// List credentials filtered by type.
    pub fn find_by_type(conn: &Connection, cred_type: &str) -> Result<Vec<Credential>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT id, credential_type, label, status, organization_id, details,
                    issued_date, expiry_date, created_at, updated_at
             FROM credentials WHERE credential_type = ?1
             ORDER BY created_at DESC",
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
        Self::get(conn, id)?.ok_or_else(|| ForgeError::NotFound {
            entity_type: "credential".into(),
            id: id.into(),
        })?;

        let mut sets = Vec::new();
        let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(ref v) = input.label {
            sets.push(format!("label = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = input.status {
            sets.push(format!("status = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.as_ref().to_string()));
        }
        if let Some(ref v) = input.organization_id {
            sets.push(format!("organization_id = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = input.details {
            sets.push(format!("details = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
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
            details: row.get(5)?,
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

    fn setup() -> Forge {
        Forge::open_memory().unwrap()
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
                details: Some(r#"{"level":"top_secret","polygraph":"full_scope"}"#.into()),
                issued_date: Some("2020-01-15".into()),
                expiry_date: Some("2025-01-15".into()),
            },
        )
        .unwrap();

        assert_eq!(cred.label, "TS/SCI");
        assert_eq!(cred.credential_type, CredentialType::Clearance);
        assert_eq!(cred.status, CredentialStatus::Active);

        let fetched = CredentialStore::get(forge.conn(), &cred.id)
            .unwrap()
            .unwrap();
        assert_eq!(fetched.id, cred.id);
        assert_eq!(fetched.label, "TS/SCI");
    }

    #[test]
    fn get_returns_none_for_missing() {
        let forge = setup();
        assert!(CredentialStore::get(forge.conn(), "nonexistent")
            .unwrap()
            .is_none());
    }

    #[test]
    fn list_credentials() {
        let forge = setup();
        CredentialStore::create(
            forge.conn(),
            &CreateCredential {
                credential_type: CredentialType::Clearance,
                label: "Secret".into(),
                status: None,
                organization_id: None,
                details: None,
                issued_date: None,
                expiry_date: None,
            },
        )
        .unwrap();
        CredentialStore::create(
            forge.conn(),
            &CreateCredential {
                credential_type: CredentialType::DriversLicense,
                label: "TX CDL".into(),
                status: None,
                organization_id: None,
                details: None,
                issued_date: None,
                expiry_date: None,
            },
        )
        .unwrap();

        let all = CredentialStore::list(forge.conn()).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn find_by_type_filters() {
        let forge = setup();
        CredentialStore::create(
            forge.conn(),
            &CreateCredential {
                credential_type: CredentialType::Clearance,
                label: "Secret".into(),
                status: None,
                organization_id: None,
                details: None,
                issued_date: None,
                expiry_date: None,
            },
        )
        .unwrap();
        CredentialStore::create(
            forge.conn(),
            &CreateCredential {
                credential_type: CredentialType::DriversLicense,
                label: "TX CDL".into(),
                status: None,
                organization_id: None,
                details: None,
                issued_date: None,
                expiry_date: None,
            },
        )
        .unwrap();

        let clearances = CredentialStore::find_by_type(forge.conn(), "clearance").unwrap();
        assert_eq!(clearances.len(), 1);
        assert_eq!(clearances[0].label, "Secret");
    }

    #[test]
    fn update_credential() {
        let forge = setup();
        let cred = CredentialStore::create(
            forge.conn(),
            &CreateCredential {
                credential_type: CredentialType::Clearance,
                label: "Secret".into(),
                status: None,
                organization_id: None,
                details: None,
                issued_date: None,
                expiry_date: None,
            },
        )
        .unwrap();

        let updated = CredentialStore::update(
            forge.conn(),
            &cred.id,
            &UpdateCredential {
                label: Some("TS/SCI".into()),
                status: Some(CredentialStatus::Expired),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(updated.label, "TS/SCI");
        assert_eq!(updated.status, CredentialStatus::Expired);
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
        let cred = CredentialStore::create(
            forge.conn(),
            &CreateCredential {
                credential_type: CredentialType::BarAdmission,
                label: "TX Bar".into(),
                status: None,
                organization_id: None,
                details: None,
                issued_date: None,
                expiry_date: None,
            },
        )
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
            &CreateCredential {
                credential_type: CredentialType::MedicalLicense,
                label: "MD License".into(),
                status: None,
                organization_id: None,
                details: None,
                issued_date: None,
                expiry_date: None,
            },
        )
        .unwrap();
        assert_eq!(cred.status, CredentialStatus::Active);
    }

    #[test]
    fn default_details_is_empty_json() {
        let forge = setup();
        let cred = CredentialStore::create(
            forge.conn(),
            &CreateCredential {
                credential_type: CredentialType::Clearance,
                label: "Test".into(),
                status: None,
                organization_id: None,
                details: None,
                issued_date: None,
                expiry_date: None,
            },
        )
        .unwrap();
        assert_eq!(cred.details, "{}");
    }
}
