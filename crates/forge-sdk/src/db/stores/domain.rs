//! Domain repository — CRUD for the `domains` lookup table.
//!
//! Domains are experience domains (e.g. "Cloud Security", "Systems Programming")
//! used by perspectives and archetypes.

use rusqlite::{params, Connection, OptionalExtension};

use forge_core::{new_id, now_iso, CreateDomainInput, Domain, ForgeError};

/// Data-access store for the `domains` table.
pub struct DomainStore;

impl DomainStore {
    // ── Create ───────────────────────────────────────────────────────

    /// Insert a new domain row.
    pub fn create(conn: &Connection, input: &CreateDomainInput) -> Result<Domain, ForgeError> {
        let id = new_id();
        let now = now_iso();

        conn.execute(
            "INSERT INTO domains (id, name, description, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![id, input.name, input.description, now],
        )?;

        Self::get(conn, &id)?
            .ok_or_else(|| ForgeError::Internal("Domain created but not found".into()))
    }

    // ── Read ─────────────────────────────────────────────────────────

    /// Fetch a single domain by ID.
    pub fn get(conn: &Connection, id: &str) -> Result<Option<Domain>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT id, name, description, created_at
             FROM domains WHERE id = ?1",
        )?;

        let result = stmt.query_row(params![id], Self::map_domain).optional()?;
        Ok(result)
    }

    /// List all domains, sorted by name.
    pub fn list(conn: &Connection) -> Result<Vec<Domain>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT id, name, description, created_at
             FROM domains
             ORDER BY name ASC",
        )?;

        let rows: Vec<Domain> = stmt
            .query_map([], Self::map_domain)?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    // ── Delete ───────────────────────────────────────────────────────

    /// Delete a domain by ID.
    pub fn delete(conn: &Connection, id: &str) -> Result<(), ForgeError> {
        let deleted = conn.execute("DELETE FROM domains WHERE id = ?1", params![id])?;
        if deleted == 0 {
            return Err(ForgeError::NotFound {
                entity_type: "domain".into(),
                id: id.into(),
            });
        }
        Ok(())
    }

    // ── Row mapping ──────────────────────────────────────────────────

    fn map_domain(row: &rusqlite::Row) -> rusqlite::Result<Domain> {
        Ok(Domain {
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
        let input = CreateDomainInput {
            name: "Cloud Security".into(),
            description: Some("Securing cloud infrastructure and services".into()),
        };
        let domain = DomainStore::create(forge.conn(), &input).unwrap();
        assert_eq!(domain.name, "Cloud Security");
        assert_eq!(
            domain.description,
            Some("Securing cloud infrastructure and services".into())
        );

        let fetched = DomainStore::get(forge.conn(), &domain.id).unwrap().unwrap();
        assert_eq!(fetched.id, domain.id);
        assert_eq!(fetched.name, "Cloud Security");
    }

    #[test]
    fn list_domains() {
        let forge = setup();
        // Migrations seed domains, so count the baseline first
        let baseline = DomainStore::list(forge.conn()).unwrap().len();

        DomainStore::create(
            forge.conn(),
            &CreateDomainInput {
                name: "Backend".into(),
                description: None,
            },
        )
        .unwrap();
        DomainStore::create(
            forge.conn(),
            &CreateDomainInput {
                name: "Quantum Computing".into(),
                description: Some("Quantum computing and cryptography".into()),
            },
        )
        .unwrap();

        let rows = DomainStore::list(forge.conn()).unwrap();
        assert_eq!(rows.len(), baseline + 2);
        // Sorted by name ASC — verify our entries are present
        assert!(rows.iter().any(|d| d.name == "Backend"));
        assert!(rows.iter().any(|d| d.name == "Quantum Computing"));
    }

    #[test]
    fn delete_domain() {
        let forge = setup();
        let domain = DomainStore::create(
            forge.conn(),
            &CreateDomainInput {
                name: "To Delete".into(),
                description: None,
            },
        )
        .unwrap();
        DomainStore::delete(forge.conn(), &domain.id).unwrap();
        assert!(DomainStore::get(forge.conn(), &domain.id)
            .unwrap()
            .is_none());
    }

    #[test]
    fn get_not_found() {
        let forge = setup();
        let result = DomainStore::get(forge.conn(), "nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn delete_missing_returns_not_found() {
        let forge = setup();
        let result = DomainStore::delete(forge.conn(), "nonexistent");
        assert!(matches!(result, Err(ForgeError::NotFound { .. })));
    }
}
