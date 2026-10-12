//! RoleType repository — lookup table CRUD for role types
//! (e.g. IC, lead, architect, manager).

use rusqlite::{params, Connection, OptionalExtension};

use super::lookup::{self, LookupTable};
use forge_core::{
    new_id, now_iso, CreateRoleTypeInput, ForgeError, Pagination, RoleType, UpdateRoleTypeInput,
};

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

    /// List role types with pagination, sorted by name (TS `RoleTypeService.list`).
    pub fn list(
        conn: &Connection,
        offset: i64,
        limit: i64,
    ) -> Result<(Vec<RoleType>, Pagination), ForgeError> {
        let total: i64 = conn.query_row("SELECT COUNT(*) FROM role_types", [], |row| row.get(0))?;

        let mut stmt = conn.prepare(
            "SELECT id, name, description, created_at FROM role_types
             ORDER BY name ASC
             LIMIT ?1 OFFSET ?2",
        )?;
        let rows: Vec<RoleType> = stmt
            .query_map(params![limit, offset], Self::map_role_type)?
            .collect::<Result<_, _>>()?;
        Ok((
            rows,
            Pagination {
                total,
                offset,
                limit,
            },
        ))
    }

    // ── Update ───────────────────────────────────────────────────────

    /// Apply a partial update and return the row.
    ///
    /// TS order (role-type-service.ts `update`, then the ELM): a blank name is a 400, an unknown id
    /// a 404, then the trimmed name must be free of every other row (409). Absent fields are left
    /// alone and `description: null` clears it (resu-mx/forge#19).
    pub fn update(
        conn: &Connection,
        id: &str,
        input: &UpdateRoleTypeInput,
    ) -> Result<RoleType, ForgeError> {
        let name = input
            .name
            .as_deref()
            .map(lookup::require_name)
            .transpose()?;
        Self::get(conn, id)?.ok_or_else(|| ForgeError::NotFound {
            entity_type: "role_type".into(),
            id: id.into(),
        })?;
        if let Some(name) = name {
            lookup::ensure_name_free(conn, LookupTable::RoleTypes, name, Some(id))?;
        }
        lookup::update_name_description(
            conn,
            LookupTable::RoleTypes,
            id,
            name,
            input.description.as_ref().map(|d| d.as_deref()),
        )?;
        Self::get(conn, id)?
            .ok_or_else(|| ForgeError::Internal("RoleType updated but not found".into()))
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

        let (list, page) = RoleTypeStore::list(forge.conn(), 0, 50).unwrap();
        assert_eq!(list.len(), 2);
        // Sorted by name ASC
        assert_eq!(list[0].name, "Architect");
        assert_eq!(list[1].name, "Manager");
        assert_eq!((page.total, page.offset, page.limit), (2, 0, 50));
    }

    #[test]
    fn list_pages_by_name_and_reports_the_full_total() {
        let forge = setup();
        for i in (0..5).rev() {
            RoleTypeStore::create(
                forge.conn(),
                &CreateRoleTypeInput {
                    name: format!("Role {i}"),
                    description: None,
                },
            )
            .unwrap();
        }

        let (rows, page) = RoleTypeStore::list(forge.conn(), 1, 2).unwrap();
        let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["Role 1", "Role 2"]);
        assert_eq!((page.total, page.offset, page.limit), (5, 1, 2));

        let (rows, page) = RoleTypeStore::list(forge.conn(), 10, 2).unwrap();
        assert!(rows.is_empty());
        assert_eq!((page.total, page.offset, page.limit), (5, 10, 2));
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
        let before = RoleTypeStore::list(forge.conn(), 0, 200).unwrap().1.total;
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
        assert_eq!(
            RoleTypeStore::list(forge.conn(), 0, 200).unwrap().1.total,
            before
        );
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

    fn make(forge: &Forge, name: &str, description: Option<&str>) -> RoleType {
        RoleTypeStore::create(
            forge.conn(),
            &CreateRoleTypeInput {
                name: name.into(),
                description: description.map(Into::into),
            },
        )
        .unwrap()
    }

    #[test]
    fn update_renames_with_a_trimmed_name_and_sets_the_description() {
        let forge = setup();
        let row = make(&forge, "TL", None);
        let updated = RoleTypeStore::update(
            forge.conn(),
            &row.id,
            &UpdateRoleTypeInput {
                name: Some("  Tech Lead  ".into()),
                description: Some(Some("Leads a team".into())),
            },
        )
        .unwrap();
        assert_eq!(updated.name, "Tech Lead");
        assert_eq!(updated.description.as_deref(), Some("Leads a team"));
        // Stored, not only echoed.
        let reread = RoleTypeStore::get(forge.conn(), &row.id).unwrap().unwrap();
        assert_eq!(reread.name, "Tech Lead");
        assert_eq!(reread.created_at, row.created_at);
    }

    #[test]
    fn update_null_clears_the_description_and_absent_leaves_it() {
        let forge = setup();
        let row = make(&forge, "IC", Some("Individual contributor"));

        let untouched = RoleTypeStore::update(
            forge.conn(),
            &row.id,
            &UpdateRoleTypeInput {
                name: Some("Individual Contributor".into()),
                description: None,
            },
        )
        .unwrap();
        assert_eq!(
            untouched.description.as_deref(),
            Some("Individual contributor")
        );

        let cleared = RoleTypeStore::update(
            forge.conn(),
            &row.id,
            &UpdateRoleTypeInput {
                name: None,
                description: Some(None),
            },
        )
        .unwrap();
        assert_eq!(cleared.description, None);
        assert_eq!(cleared.name, "Individual Contributor");
    }

    #[test]
    fn update_with_nothing_present_changes_nothing() {
        let forge = setup();
        let row = make(&forge, "Architect", Some("d"));
        let same =
            RoleTypeStore::update(forge.conn(), &row.id, &UpdateRoleTypeInput::default()).unwrap();
        assert_eq!(
            (same.name.as_str(), same.description.as_deref()),
            ("Architect", Some("d"))
        );
    }

    #[test]
    fn update_blank_name_is_a_400_and_changes_nothing() {
        let forge = setup();
        let row = make(&forge, "Manager", Some("People"));
        let err = RoleTypeStore::update(
            forge.conn(),
            &row.id,
            &UpdateRoleTypeInput {
                name: Some("   ".into()),
                description: Some(Some("changed".into())),
            },
        )
        .unwrap_err();
        assert!(
            matches!(&err, ForgeError::Validation { message, field: Some(f) }
            if message == lookup::NAME_EMPTY && f == "name")
        );
        let after = RoleTypeStore::get(forge.conn(), &row.id).unwrap().unwrap();
        assert_eq!(after.description.as_deref(), Some("People"));
    }

    #[test]
    fn update_unknown_id_is_not_found() {
        let forge = setup();
        let err = RoleTypeStore::update(
            forge.conn(),
            "no-such-id",
            &UpdateRoleTypeInput {
                description: Some(Some("x".into())),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert!(matches!(err, ForgeError::NotFound { .. }));
    }

    #[test]
    fn update_to_a_taken_name_is_a_conflict_but_its_own_name_is_fine() {
        let forge = setup();
        make(&forge, "Director", None);
        let other = make(&forge, "Staff", None);

        let err = RoleTypeStore::update(
            forge.conn(),
            &other.id,
            &UpdateRoleTypeInput {
                name: Some(" Director ".into()),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert!(matches!(err, ForgeError::Conflict { .. }));
        assert_eq!(
            RoleTypeStore::get(forge.conn(), &other.id)
                .unwrap()
                .unwrap()
                .name,
            "Staff"
        );

        let same = RoleTypeStore::update(
            forge.conn(),
            &other.id,
            &UpdateRoleTypeInput {
                name: Some("Staff".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(same.name, "Staff");
    }
}
