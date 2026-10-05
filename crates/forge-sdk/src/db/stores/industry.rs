//! Industry repository — CRUD for the `industries` lookup table.
//!
//! Industries are lightweight entities (e.g. "fintech", "healthcare",
//! "defense") referenced by organizations and summaries.

use rusqlite::{params, Connection, OptionalExtension};

use forge_core::{new_id, now_iso, CreateIndustryInput, ForgeError, Industry};

/// Data-access store for the `industries` table.
pub struct IndustryStore;

impl IndustryStore {
    // ── Create ───────────────────────────────────────────────────────

    /// Insert a new industry row.
    pub fn create(conn: &Connection, input: &CreateIndustryInput) -> Result<Industry, ForgeError> {
        let id = new_id();
        let now = now_iso();

        conn.execute(
            "INSERT INTO industries (id, name, description, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![id, input.name, input.description, now],
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
}
