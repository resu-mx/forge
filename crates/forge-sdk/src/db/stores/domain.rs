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

    // ── Usage ────────────────────────────────────────────────────────

    /// Perspectives whose free-text `domain` equals this domain's name.
    /// (`perspectives.domain` is not a foreign key, so nothing else enforces this.)
    pub fn count_perspectives(conn: &Connection, name: &str) -> Result<i64, ForgeError> {
        Ok(conn.query_row(
            "SELECT COUNT(*) FROM perspectives WHERE domain = ?1",
            params![name],
            |row| row.get(0),
        )?)
    }

    /// `archetype_domains` rows linking an archetype to this domain.
    pub fn count_archetypes(conn: &Connection, domain_id: &str) -> Result<i64, ForgeError> {
        Ok(conn.query_row(
            "SELECT COUNT(*) FROM archetype_domains WHERE domain_id = ?1",
            params![domain_id],
            |row| row.get(0),
        )?)
    }

    // ── Delete ───────────────────────────────────────────────────────

    /// Delete a domain by ID, unless it is in use.
    ///
    /// Mirrors `DomainService.delete` (packages/core/src/services/domain-service.ts:113-157):
    /// perspectives first (they name the domain in a text column), then
    /// `archetype_domains` (an `ON DELETE CASCADE` child that would otherwise be
    /// torn down silently). `skill_domains` is not checked, as in TS.
    pub fn delete(conn: &Connection, id: &str) -> Result<(), ForgeError> {
        let domain = Self::get(conn, id)?
            .ok_or_else(|| ForgeError::NotFound { entity_type: "domain".into(), id: id.into() })?;

        let perspectives = Self::count_perspectives(conn, &domain.name)?;
        if perspectives > 0 {
            return Err(ForgeError::Conflict {
                message: format!(
                    "Cannot delete domain '{}': referenced by {perspectives} perspective(s)",
                    domain.name
                ),
            });
        }

        let archetypes = Self::count_archetypes(conn, &domain.id)?;
        if archetypes > 0 {
            return Err(ForgeError::Conflict {
                message: format!(
                    "Cannot delete domain '{}': associated with {archetypes} archetype(s)",
                    domain.name
                ),
            });
        }

        conn.execute("DELETE FROM domains WHERE id = ?1", params![id])?;
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
    use crate::db::stores::bullet::BulletStore;
    use crate::db::stores::perspective::PerspectiveStore;
    use crate::forge::Forge;

    /// Seeded by migration 003 and linked to `security-engineer` and `public-sector`.
    const SECURITY: &str = "d0000001-0000-4000-8000-000000000003";

    fn setup() -> Forge {
        Forge::open_memory().unwrap()
    }

    fn perspective_naming(forge: &Forge, domain: &str) {
        let bullet = BulletStore::create(forge.conn(), "Built APIs", None, None, None, &[], &[]).unwrap();
        PerspectiveStore::create_direct(
            forge.conn(), &bullet.id, "Designed APIs", None, Some(domain), None, false,
        )
        .unwrap();
    }

    fn make(forge: &Forge, name: &str) -> Domain {
        DomainStore::create(forge.conn(), &CreateDomainInput { name: name.into(), description: None }).unwrap()
    }

    #[test]
    fn delete_refuses_domain_named_by_a_perspective() {
        let forge = setup();
        let d = make(&forge, "block_delete");
        perspective_naming(&forge, "block_delete");

        match DomainStore::delete(forge.conn(), &d.id) {
            Err(ForgeError::Conflict { message }) => assert_eq!(
                message,
                "Cannot delete domain 'block_delete': referenced by 1 perspective(s)"
            ),
            other => panic!("expected Conflict, got {other:?}"),
        }
        assert!(DomainStore::get(forge.conn(), &d.id).unwrap().is_some());
    }

    #[test]
    fn delete_refuses_seeded_domain_linked_to_archetypes() {
        let forge = setup();
        let before = DomainStore::count_archetypes(forge.conn(), SECURITY).unwrap();
        assert_eq!(before, 2);

        let err = DomainStore::delete(forge.conn(), SECURITY).unwrap_err();
        assert!(matches!(&err, ForgeError::Conflict { message }
            if message == "Cannot delete domain 'security': associated with 2 archetype(s)"));
        assert_eq!(DomainStore::count_archetypes(forge.conn(), SECURITY).unwrap(), before);
        assert!(DomainStore::get(forge.conn(), SECURITY).unwrap().is_some());
    }

    #[test]
    fn delete_reports_perspectives_before_archetypes() {
        let forge = setup();
        // `security` is linked to two archetypes; also name it from a perspective.
        perspective_naming(&forge, "security");

        let err = DomainStore::delete(forge.conn(), SECURITY).unwrap_err();
        assert!(matches!(&err, ForgeError::Conflict { message }
            if message == "Cannot delete domain 'security': referenced by 1 perspective(s)"));
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
