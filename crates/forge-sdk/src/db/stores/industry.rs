//! Industry repository — CRUD for the `industries` lookup table.
//!
//! Industries are lightweight entities (e.g. "fintech", "healthcare",
//! "defense") referenced by organizations and summaries.

use rusqlite::{params, Connection, OptionalExtension};

use super::lookup::{self, LookupTable};
use forge_core::{new_id, now_iso, CreateIndustryInput, ForgeError, Industry, UpdateIndustryInput};

/// Data-access store for the `industries` table.
pub struct IndustryStore;

impl IndustryStore {
    // ── Create ───────────────────────────────────────────────────────

    /// Insert a new industry row.
    ///
    /// TS trims the name and rejects a blank one; the ELM answers 409 for a taken one.
    pub fn create(conn: &Connection, input: &CreateIndustryInput) -> Result<Industry, ForgeError> {
        let name = lookup::require_name(&input.name)?;
        lookup::ensure_name_free(conn, LookupTable::Industries, name, None)?;

        let id = new_id();
        let now = now_iso();

        conn.execute(
            "INSERT INTO industries (id, name, description, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![id, name, input.description, now],
        )?;

        Self::get(conn, &id)?
            .ok_or_else(|| ForgeError::Internal("Industry created but not found".into()))
    }

    // ── Read ─────────────────────────────────────────────────────────

    /// Fetch a single industry by ID.
    pub fn get(conn: &Connection, id: &str) -> Result<Option<Industry>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT id, name, description, created_at
             FROM industries WHERE id = ?1",
        )?;

        let result = stmt.query_row(params![id], Self::map_industry).optional()?;
        Ok(result)
    }

    /// List all industries, sorted by name.
    pub fn list(conn: &Connection) -> Result<Vec<Industry>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT id, name, description, created_at
             FROM industries
             ORDER BY name ASC",
        )?;

        let rows: Vec<Industry> = stmt
            .query_map([], Self::map_industry)?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    // ── Update ───────────────────────────────────────────────────────

    /// Apply a partial update and return the row.
    ///
    /// TS order (industry-service.ts `update`, then the ELM): a blank name is a 400, an unknown id
    /// a 404, then the trimmed name must be free of every other row (409). Absent fields are left
    /// alone and `description: null` clears it (resu-mx/forge#18).
    pub fn update(
        conn: &Connection,
        id: &str,
        input: &UpdateIndustryInput,
    ) -> Result<Industry, ForgeError> {
        let name = input
            .name
            .as_deref()
            .map(lookup::require_name)
            .transpose()?;
        Self::get(conn, id)?.ok_or_else(|| ForgeError::NotFound {
            entity_type: "industry".into(),
            id: id.into(),
        })?;
        if let Some(name) = name {
            lookup::ensure_name_free(conn, LookupTable::Industries, name, Some(id))?;
        }
        lookup::update_name_description(
            conn,
            LookupTable::Industries,
            id,
            name,
            input.description.as_ref().map(|d| d.as_deref()),
        )?;
        Self::get(conn, id)?
            .ok_or_else(|| ForgeError::Internal("Industry updated but not found".into()))
    }

    // ── Delete ───────────────────────────────────────────────────────

    /// Delete an industry by ID.
    pub fn delete(conn: &Connection, id: &str) -> Result<(), ForgeError> {
        let deleted = conn.execute("DELETE FROM industries WHERE id = ?1", params![id])?;
        if deleted == 0 {
            return Err(ForgeError::NotFound {
                entity_type: "industry".into(),
                id: id.into(),
            });
        }
        Ok(())
    }

    // ── Row mapping ──────────────────────────────────────────────────

    fn map_industry(row: &rusqlite::Row) -> rusqlite::Result<Industry> {
        Ok(Industry {
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
    fn create_and_get() {
        let forge = setup();
        let input = CreateIndustryInput {
            name: "Fintech".into(),
            description: Some("Financial technology services".into()),
        };
        let industry = IndustryStore::create(forge.conn(), &input).unwrap();
        assert_eq!(industry.name, "Fintech");
        assert_eq!(
            industry.description,
            Some("Financial technology services".into())
        );

        let fetched = IndustryStore::get(forge.conn(), &industry.id)
            .unwrap()
            .unwrap();
        assert_eq!(fetched.id, industry.id);
        assert_eq!(fetched.name, "Fintech");
    }

    #[test]
    fn list_industries() {
        let forge = setup();
        IndustryStore::create(
            forge.conn(),
            &CreateIndustryInput {
                name: "Healthcare".into(),
                description: None,
            },
        )
        .unwrap();
        IndustryStore::create(
            forge.conn(),
            &CreateIndustryInput {
                name: "Defense".into(),
                description: Some("Military and defense sector".into()),
            },
        )
        .unwrap();

        let rows = IndustryStore::list(forge.conn()).unwrap();
        assert_eq!(rows.len(), 2);
        // Sorted by name ASC
        assert_eq!(rows[0].name, "Defense");
        assert_eq!(rows[1].name, "Healthcare");
    }

    #[test]
    fn delete_industry() {
        let forge = setup();
        let industry = IndustryStore::create(
            forge.conn(),
            &CreateIndustryInput {
                name: "To Delete".into(),
                description: None,
            },
        )
        .unwrap();
        IndustryStore::delete(forge.conn(), &industry.id).unwrap();
        assert!(IndustryStore::get(forge.conn(), &industry.id)
            .unwrap()
            .is_none());
    }

    #[test]
    fn get_not_found() {
        let forge = setup();
        let result = IndustryStore::get(forge.conn(), "nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn delete_missing_returns_not_found() {
        let forge = setup();
        let result = IndustryStore::delete(forge.conn(), "nonexistent");
        assert!(matches!(result, Err(ForgeError::NotFound { .. })));
    }

    #[test]
    fn create_trims_the_name() {
        let forge = setup();
        let row = IndustryStore::create(
            forge.conn(),
            &CreateIndustryInput {
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
        let before = IndustryStore::list(forge.conn()).unwrap().len();
        for name in ["", "   "] {
            let err = IndustryStore::create(
                forge.conn(),
                &CreateIndustryInput {
                    name: name.into(),
                    description: None,
                },
            )
            .unwrap_err();
            assert!(matches!(&err, ForgeError::Validation { message, .. }
                if message == lookup::NAME_EMPTY));
        }
        assert_eq!(IndustryStore::list(forge.conn()).unwrap().len(), before);
    }

    #[test]
    fn create_duplicate_is_conflict_after_trimming() {
        let forge = setup();
        IndustryStore::create(
            forge.conn(),
            &CreateIndustryInput {
                name: "Dup Name".into(),
                description: None,
            },
        )
        .unwrap();
        let err = IndustryStore::create(
            forge.conn(),
            &CreateIndustryInput {
                name: " Dup Name ".into(),
                description: None,
            },
        )
        .unwrap_err();
        assert!(matches!(err, ForgeError::Conflict { .. }));
    }

    fn make(forge: &Forge, name: &str, description: Option<&str>) -> Industry {
        IndustryStore::create(
            forge.conn(),
            &CreateIndustryInput {
                name: name.into(),
                description: description.map(Into::into),
            },
        )
        .unwrap()
    }

    #[test]
    fn update_renames_with_a_trimmed_name_and_sets_the_description() {
        let forge = setup();
        let row = make(&forge, "Fin", None);
        let updated = IndustryStore::update(
            forge.conn(),
            &row.id,
            &UpdateIndustryInput {
                name: Some("  FinTech  ".into()),
                description: Some(Some("Money software".into())),
            },
        )
        .unwrap();
        assert_eq!(updated.name, "FinTech");
        assert_eq!(updated.description.as_deref(), Some("Money software"));
        // Stored, not only echoed.
        let reread = IndustryStore::get(forge.conn(), &row.id).unwrap().unwrap();
        assert_eq!(reread.name, "FinTech");
        assert_eq!(reread.created_at, row.created_at);
    }

    #[test]
    fn update_null_clears_the_description_and_absent_leaves_it() {
        let forge = setup();
        let row = make(&forge, "Health", Some("Care"));

        let untouched = IndustryStore::update(
            forge.conn(),
            &row.id,
            &UpdateIndustryInput {
                name: Some("Healthcare".into()),
                description: None,
            },
        )
        .unwrap();
        assert_eq!(untouched.description.as_deref(), Some("Care"));

        let cleared = IndustryStore::update(
            forge.conn(),
            &row.id,
            &UpdateIndustryInput {
                name: None,
                description: Some(None),
            },
        )
        .unwrap();
        assert_eq!(cleared.description, None);
        assert_eq!(cleared.name, "Healthcare");
    }

    #[test]
    fn update_with_nothing_present_changes_nothing() {
        let forge = setup();
        let row = make(&forge, "Defense", Some("d"));
        let same =
            IndustryStore::update(forge.conn(), &row.id, &UpdateIndustryInput::default()).unwrap();
        assert_eq!(
            (same.name.as_str(), same.description.as_deref()),
            ("Defense", Some("d"))
        );
    }

    #[test]
    fn update_blank_name_is_a_400_and_changes_nothing() {
        let forge = setup();
        let row = make(&forge, "Retail", Some("Shops"));
        let err = IndustryStore::update(
            forge.conn(),
            &row.id,
            &UpdateIndustryInput {
                name: Some("   ".into()),
                description: Some(Some("changed".into())),
            },
        )
        .unwrap_err();
        assert!(
            matches!(&err, ForgeError::Validation { message, field: Some(f) }
            if message == lookup::NAME_EMPTY && f == "name")
        );
        let after = IndustryStore::get(forge.conn(), &row.id).unwrap().unwrap();
        assert_eq!(after.description.as_deref(), Some("Shops"));
    }

    #[test]
    fn update_unknown_id_is_not_found() {
        let forge = setup();
        let err = IndustryStore::update(
            forge.conn(),
            "no-such-id",
            &UpdateIndustryInput {
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
        make(&forge, "Energy", None);
        let other = make(&forge, "Mining", None);

        let err = IndustryStore::update(
            forge.conn(),
            &other.id,
            &UpdateIndustryInput {
                name: Some(" Energy ".into()),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert!(matches!(err, ForgeError::Conflict { .. }));
        assert_eq!(
            IndustryStore::get(forge.conn(), &other.id)
                .unwrap()
                .unwrap()
                .name,
            "Mining"
        );

        let same = IndustryStore::update(
            forge.conn(),
            &other.id,
            &UpdateIndustryInput {
                name: Some("Mining".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(same.name, "Mining");
    }
}
