//! RoleType repository — lookup table CRUD for role types
//! (e.g. IC, lead, architect, manager).

use rusqlite::{params, Connection, OptionalExtension};

use super::lookup::{self, LookupTable};
use forge_core::{new_id, now_iso, CreateRoleTypeInput, ForgeError, RoleType};

/// Data access for the `role_types` table.
pub struct RoleTypeStore;

impl RoleTypeStore {
    // ── Create ───────────────────────────────────────────────────────

    /// Insert a new role type row.
    ///
    /// TS trims the name and rejects a blank one; the ELM answers 409 for a taken one.
    pub fn create(conn: &Connection, input: &CreateRoleTypeInput) -> Result<RoleType, ForgeError> {
        let name = lookup::require_name(&input.name)?;
        lookup::ensure_name_free(conn, LookupTable::RoleTypes, name, None)?;

        let id = new_id();
        let now = now_iso();

        conn.execute(
            "INSERT INTO role_types (id, name, description, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![id, name, input.description, now],
        )?;

        Self::get(conn, &id)?
            .ok_or_else(|| ForgeError::Internal("RoleType created but not found".into()))
    }

    // ── Read ─────────────────────────────────────────────────────────

    /// Fetch a single role type by ID.
    pub fn get(conn: &Connection, id: &str) -> Result<Option<RoleType>, ForgeError> {
        let mut stmt =
            conn.prepare("SELECT id, name, description, created_at FROM role_types WHERE id = ?1")?;
        let result = stmt
            .query_row(params![id], Self::map_role_type)
            .optional()?;
        Ok(result)
    }

    /// List all role types, sorted by name.
    pub fn list(conn: &Connection) -> Result<Vec<RoleType>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT id, name, description, created_at FROM role_types ORDER BY name ASC",
        )?;
        let rows: Vec<RoleType> = stmt
            .query_map([], Self::map_role_type)?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    // ── Delete ───────────────────────────────────────────────────────

    /// Delete a role type by ID.
    pub fn delete(conn: &Connection, id: &str) -> Result<(), ForgeError> {
        let deleted = conn.execute("DELETE FROM role_types WHERE id = ?1", params![id])?;
        if deleted == 0 {
            return Err(ForgeError::NotFound {
                entity_type: "role_type".into(),
                id: id.into(),
            });
        }
        Ok(())
    }

    // ── Row mapping ──────────────────────────────────────────────────

    fn map_role_type(row: &rusqlite::Row) -> rusqlite::Result<RoleType> {
        Ok(RoleType {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            created_at: row.get(3)?,
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
    fn create_and_get_role_type() {
        let forge = setup();
        let input = CreateRoleTypeInput {
            name: "IC".into(),
            description: Some("Individual contributor".into()),
        };
        let rt = RoleTypeStore::create(forge.conn(), &input).unwrap();
        assert_eq!(rt.name, "IC");
        assert_eq!(rt.description, Some("Individual contributor".into()));

        let fetched = RoleTypeStore::get(forge.conn(), &rt.id).unwrap().unwrap();
        assert_eq!(fetched.id, rt.id);
        assert_eq!(fetched.name, "IC");
    }

    #[test]
    fn get_returns_none_for_missing() {
        let forge = setup();
        let result = RoleTypeStore::get(forge.conn(), "nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn list_role_types() {
        let forge = setup();
        RoleTypeStore::create(
            forge.conn(),
            &CreateRoleTypeInput {
                name: "Manager".into(),
                description: None,
            },
        )
        .unwrap();
        RoleTypeStore::create(
            forge.conn(),
            &CreateRoleTypeInput {
                name: "Architect".into(),
                description: Some("System designer".into()),
            },
        )
        .unwrap();

        let list = RoleTypeStore::list(forge.conn()).unwrap();
        assert_eq!(list.len(), 2);
        // Sorted by name ASC
        assert_eq!(list[0].name, "Architect");
        assert_eq!(list[1].name, "Manager");
    }

    #[test]
    fn delete_role_type() {
        let forge = setup();
        let rt = RoleTypeStore::create(
            forge.conn(),
            &CreateRoleTypeInput {
                name: "Lead".into(),
                description: None,
            },
        )
        .unwrap();

        RoleTypeStore::delete(forge.conn(), &rt.id).unwrap();
        assert!(RoleTypeStore::get(forge.conn(), &rt.id).unwrap().is_none());
    }

    #[test]
    fn delete_missing_returns_not_found() {
        let forge = setup();
        let result = RoleTypeStore::delete(forge.conn(), "nonexistent");
        assert!(matches!(result, Err(ForgeError::NotFound { .. })));
    }

    #[test]
    fn create_trims_the_name() {
        let forge = setup();
        let row = RoleTypeStore::create(
            forge.conn(),
            &CreateRoleTypeInput {
                name: "  Trim Me  ".into(),
                description: None,
            },
        )
        .unwrap();
        assert_eq!(row.name, "Trim Me");
    }

    #[test]
    fn create_rejects_a_blank_name() {
        let forge = setup();
        let before = RoleTypeStore::list(forge.conn()).unwrap().len();
        for name in ["", "   "] {
            let err = RoleTypeStore::create(
                forge.conn(),
                &CreateRoleTypeInput {
                    name: name.into(),
                    description: None,
                },
            )
            .unwrap_err();
            assert!(matches!(&err, ForgeError::Validation { message, .. }
                if message == lookup::NAME_EMPTY));
        }
        assert_eq!(RoleTypeStore::list(forge.conn()).unwrap().len(), before);
    }

    #[test]
    fn create_duplicate_is_conflict_after_trimming() {
        let forge = setup();
        RoleTypeStore::create(
            forge.conn(),
            &CreateRoleTypeInput {
                name: "Dup Name".into(),
                description: None,
            },
        )
        .unwrap();
        let err = RoleTypeStore::create(
            forge.conn(),
            &CreateRoleTypeInput {
                name: " Dup Name ".into(),
                description: None,
            },
        )
        .unwrap_err();
        assert!(matches!(err, ForgeError::Conflict { .. }));
    }
}
