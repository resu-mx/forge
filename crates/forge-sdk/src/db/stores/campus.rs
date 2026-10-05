//! Campus (org location & alias) repository — CRUD for `org_locations`
//! and `org_aliases`, scoped to an organization.

use rusqlite::{params, Connection, OptionalExtension};

use forge_core::{
    new_id, now_iso, CreateOrgAlias, CreateOrgLocation, ForgeError, LocationModality, OrgAlias,
    OrgLocation, UpdateOrgLocation,
};

/// Data access for `org_locations` and `org_aliases` tables.
pub struct CampusStore;

impl CampusStore {
    // ── Org Locations ────────────────────────────────────────────────

    /// Create a new org location.
    pub fn create_location(
        conn: &Connection,
        input: &CreateOrgLocation,
    ) -> Result<OrgLocation, ForgeError> {
        let id = new_id();
        let now = now_iso();
        let modality = input.modality.unwrap_or(LocationModality::InPerson);
        let is_hq = input.is_headquarters.unwrap_or(false) as i32;

        conn.execute(
            "INSERT INTO org_locations (id, organization_id, name, modality, address_id, is_headquarters, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, input.organization_id, input.name, modality.as_ref(), input.address_id, is_hq, now],
        )?;

        Self::get_location(conn, &id)?
            .ok_or_else(|| ForgeError::Internal("OrgLocation created but not found".into()))
    }

    /// Fetch a single org location by ID.
    pub fn get_location(conn: &Connection, id: &str) -> Result<Option<OrgLocation>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT id, organization_id, name, modality, address_id, is_headquarters, created_at
             FROM org_locations WHERE id = ?1",
        )?;
        let result = stmt.query_row(params![id], Self::map_location).optional()?;
        Ok(result)
    }

    /// List all locations for an organization.
    pub fn list_by_org(conn: &Connection, org_id: &str) -> Result<Vec<OrgLocation>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT id, organization_id, name, modality, address_id, is_headquarters, created_at
             FROM org_locations WHERE organization_id = ?1
             ORDER BY is_headquarters DESC, name ASC",
        )?;
        let rows: Vec<OrgLocation> = stmt
            .query_map(params![org_id], Self::map_location)?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    /// Partially update an org location.
    pub fn update_location(
        conn: &Connection,
        id: &str,
        input: &UpdateOrgLocation,
    ) -> Result<OrgLocation, ForgeError> {
        Self::get_location(conn, id)?.ok_or_else(|| ForgeError::NotFound {
            entity_type: "org_location".into(),
            id: id.into(),
        })?;

        let mut sets = Vec::new();
        let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(ref v) = input.name {
            sets.push(format!("name = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = input.modality {
            sets.push(format!("modality = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.as_ref().to_string()));
        }
        if let Some(ref v) = input.address_id {
            sets.push(format!("address_id = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(v) = input.is_headquarters {
            sets.push(format!("is_headquarters = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v as i32));
        }

        if !sets.is_empty() {
            let sql = format!(
                "UPDATE org_locations SET {} WHERE id = ?{}",
                sets.join(", "),
                bind_values.len() + 1
            );
            bind_values.push(Box::new(id.to_string()));

            conn.execute(
                &sql,
                rusqlite::params_from_iter(bind_values.iter().map(|b| b.as_ref())),
            )?;
        }

        Self::get_location(conn, id)?
            .ok_or_else(|| ForgeError::Internal("OrgLocation updated but not found".into()))
    }

    /// Delete an org location by ID.
    pub fn delete_location(conn: &Connection, id: &str) -> Result<(), ForgeError> {
        let deleted = conn.execute("DELETE FROM org_locations WHERE id = ?1", params![id])?;
        if deleted == 0 {
            return Err(ForgeError::NotFound {
                entity_type: "org_location".into(),
                id: id.into(),
            });
        }
        Ok(())
    }

    // ── Org Aliases ──────────────────────────────────────────────────

    /// Create a new org alias.
    pub fn create_alias(
        conn: &Connection,
        org_id: &str,
        input: &CreateOrgAlias,
    ) -> Result<OrgAlias, ForgeError> {
        let trimmed = input.alias.trim();
        if trimmed.is_empty() {
            return Err(ForgeError::Validation {
                message: "Alias is required".into(),
                field: Some("alias".into()),
            });
        }
        let id = new_id();

        match conn.execute(
            "INSERT INTO org_aliases (id, organization_id, alias) VALUES (?1, ?2, ?3)",
            params![id, org_id, trimmed],
        ) {
            Ok(_) => {}
            Err(rusqlite::Error::SqliteFailure(err, _))
                if err.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                return Err(ForgeError::Validation {
                    message: "Alias already exists for this organization".into(),
                    field: Some("alias".into()),
                });
            }
            Err(e) => return Err(e.into()),
        }

        Ok(OrgAlias {
            id,
            organization_id: org_id.into(),
            alias: trimmed.into(),
        })
    }

    /// List all aliases for an organization.
    pub fn list_aliases(conn: &Connection, org_id: &str) -> Result<Vec<OrgAlias>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT id, organization_id, alias FROM org_aliases
             WHERE organization_id = ?1 ORDER BY alias",
        )?;
        let rows: Vec<OrgAlias> = stmt
            .query_map(params![org_id], |row| {
                Ok(OrgAlias {
                    id: row.get(0)?,
                    organization_id: row.get(1)?,
                    alias: row.get(2)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    /// Delete an alias by ID.
    pub fn delete_alias(conn: &Connection, id: &str) -> Result<(), ForgeError> {
        let deleted = conn.execute("DELETE FROM org_aliases WHERE id = ?1", params![id])?;
        if deleted == 0 {
            return Err(ForgeError::NotFound {
                entity_type: "org_alias".into(),
                id: id.into(),
            });
        }
        Ok(())
    }

    // ── Row mapping ──────────────────────────────────────────────────

    fn map_location(row: &rusqlite::Row) -> rusqlite::Result<OrgLocation> {
        Ok(OrgLocation {
            id: row.get(0)?,
            organization_id: row.get(1)?,
            name: row.get(2)?,
            modality: row
                .get::<_, String>(3)?
                .parse()
                .unwrap_or(LocationModality::InPerson),
            address_id: row.get(4)?,
            is_headquarters: row.get(5)?,
            created_at: row.get(6)?,
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

    fn create_org(conn: &Connection, name: &str) -> String {
        let id = new_id();
        let now = now_iso();
        conn.execute(
            "INSERT INTO organizations (id, name, org_type, worked, created_at, updated_at) VALUES (?1, ?2, 'company', 0, ?3, ?3)",
            params![id, name, now],
        ).unwrap();
        id
    }

    #[test]
    fn create_and_get_location() {
        let forge = setup();
        let org_id = create_org(forge.conn(), "Acme Corp");

        let loc = CampusStore::create_location(
            forge.conn(),
            &CreateOrgLocation {
                organization_id: org_id.clone(),
                name: "HQ".into(),
                modality: Some(LocationModality::InPerson),
                address_id: None,
                is_headquarters: Some(true),
            },
        )
        .unwrap();

        assert_eq!(loc.name, "HQ");
        assert_eq!(loc.organization_id, org_id);
        assert_eq!(loc.is_headquarters, 1);

        let fetched = CampusStore::get_location(forge.conn(), &loc.id)
            .unwrap()
            .unwrap();
        assert_eq!(fetched.id, loc.id);
    }

    #[test]
    fn get_location_returns_none_for_missing() {
        let forge = setup();
        let result = CampusStore::get_location(forge.conn(), "nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn list_locations_by_org() {
        let forge = setup();
        let org_id = create_org(forge.conn(), "Big Co");

        CampusStore::create_location(
            forge.conn(),
            &CreateOrgLocation {
                organization_id: org_id.clone(),
                name: "Office A".into(),
                modality: None,
                address_id: None,
                is_headquarters: None,
            },
        )
        .unwrap();
        CampusStore::create_location(
            forge.conn(),
            &CreateOrgLocation {
                organization_id: org_id.clone(),
                name: "Office B".into(),
                modality: None,
                address_id: None,
                is_headquarters: None,
            },
        )
        .unwrap();

        let locations = CampusStore::list_by_org(forge.conn(), &org_id).unwrap();
        assert_eq!(locations.len(), 2);
    }

    #[test]
    fn update_location() {
        let forge = setup();
        let org_id = create_org(forge.conn(), "Update Co");

        let loc = CampusStore::create_location(
            forge.conn(),
            &CreateOrgLocation {
                organization_id: org_id,
                name: "Old Name".into(),
                modality: None,
                address_id: None,
                is_headquarters: None,
            },
        )
        .unwrap();

        let updated = CampusStore::update_location(
            forge.conn(),
            &loc.id,
            &UpdateOrgLocation {
                name: Some("New Name".into()),
                modality: Some(LocationModality::Remote),
                is_headquarters: Some(true),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(updated.name, "New Name");
        assert_eq!(updated.modality, LocationModality::Remote);
        assert_eq!(updated.is_headquarters, 1);
    }

    #[test]
    fn update_missing_returns_not_found() {
        let forge = setup();
        let result = CampusStore::update_location(
            forge.conn(),
            "nonexistent",
            &UpdateOrgLocation::default(),
        );
        assert!(matches!(result, Err(ForgeError::NotFound { .. })));
    }

    #[test]
    fn delete_location() {
        let forge = setup();
        let org_id = create_org(forge.conn(), "Delete Co");

        let loc = CampusStore::create_location(
            forge.conn(),
            &CreateOrgLocation {
                organization_id: org_id,
                name: "Gone".into(),
                modality: None,
                address_id: None,
                is_headquarters: None,
            },
        )
        .unwrap();

        CampusStore::delete_location(forge.conn(), &loc.id).unwrap();
        assert!(CampusStore::get_location(forge.conn(), &loc.id)
            .unwrap()
            .is_none());
    }

    #[test]
    fn delete_location_missing_returns_not_found() {
        let forge = setup();
        let result = CampusStore::delete_location(forge.conn(), "nonexistent");
        assert!(matches!(result, Err(ForgeError::NotFound { .. })));
    }

    #[test]
    fn create_and_list_aliases() {
        let forge = setup();
        let org_id = create_org(forge.conn(), "Western Governors University");

        let alias = CampusStore::create_alias(
            forge.conn(),
            &org_id,
            &CreateOrgAlias {
                alias: "WGU".into(),
            },
        )
        .unwrap();
        assert_eq!(alias.alias, "WGU");
        assert_eq!(alias.organization_id, org_id);

        CampusStore::create_alias(
            forge.conn(),
            &org_id,
            &CreateOrgAlias {
                alias: "Western Gov".into(),
            },
        )
        .unwrap();

        let aliases = CampusStore::list_aliases(forge.conn(), &org_id).unwrap();
        assert_eq!(aliases.len(), 2);
    }

    #[test]
    fn create_alias_rejects_empty() {
        let forge = setup();
        let org_id = create_org(forge.conn(), "Empty Alias Co");
        let result = CampusStore::create_alias(
            forge.conn(),
            &org_id,
            &CreateOrgAlias {
                alias: "   ".into(),
            },
        );
        assert!(matches!(result, Err(ForgeError::Validation { .. })));
    }

    #[test]
    fn create_alias_rejects_duplicate() {
        let forge = setup();
        let org_id = create_org(forge.conn(), "Dup Alias Co");

        CampusStore::create_alias(
            forge.conn(),
            &org_id,
            &CreateOrgAlias {
                alias: "USAF".into(),
            },
        )
        .unwrap();

        let result = CampusStore::create_alias(
            forge.conn(),
            &org_id,
            &CreateOrgAlias {
                alias: "USAF".into(),
            },
        );
        assert!(matches!(result, Err(ForgeError::Validation { .. })));
    }

    #[test]
    fn delete_alias() {
        let forge = setup();
        let org_id = create_org(forge.conn(), "Del Alias Co");

        let alias = CampusStore::create_alias(
            forge.conn(),
            &org_id,
            &CreateOrgAlias {
                alias: "DAC".into(),
            },
        )
        .unwrap();

        CampusStore::delete_alias(forge.conn(), &alias.id).unwrap();
        let aliases = CampusStore::list_aliases(forge.conn(), &org_id).unwrap();
        assert!(aliases.is_empty());
    }

    #[test]
    fn delete_alias_missing_returns_not_found() {
        let forge = setup();
        let result = CampusStore::delete_alias(forge.conn(), "nonexistent");
        assert!(matches!(result, Err(ForgeError::NotFound { .. })));
    }
}
