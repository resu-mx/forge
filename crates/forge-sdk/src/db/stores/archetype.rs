//! Archetype repository — CRUD for archetypes and the
//! `archetype_domains` junction table.

use rusqlite::{params, Connection, OptionalExtension};

use forge_core::{
    new_id, now_iso, Archetype, ArchetypeWithCounts, ArchetypeWithDomains, CreateArchetypeInput,
    Domain, ForgeError, Pagination, UpdateArchetypeInput,
};

/// Data access for the `archetypes` and `archetype_domains` tables.
pub struct ArchetypeStore;

impl ArchetypeStore {
    // ── Create ───────────────────────────────────────────────────────

    /// Insert a new archetype row.
    pub fn create(
        conn: &Connection,
        input: &CreateArchetypeInput,
    ) -> Result<Archetype, ForgeError> {
        let id = new_id();
        let now = now_iso();

        conn.execute(
            "INSERT INTO archetypes (id, name, description, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![id, input.name, input.description, now],
        )?;

        Self::get(conn, &id)?
            .ok_or_else(|| ForgeError::Internal("Archetype created but not found".into()))
    }

    // ── Read ─────────────────────────────────────────────────────────

    /// Fetch a single archetype by ID.
    pub fn get(conn: &Connection, id: &str) -> Result<Option<Archetype>, ForgeError> {
        let mut stmt =
            conn.prepare("SELECT id, name, description, created_at FROM archetypes WHERE id = ?1")?;
        let result = stmt
            .query_row(params![id], Self::map_archetype)
            .optional()?;
        Ok(result)
    }

    /// Fetch an archetype with its linked domains.
    pub fn get_with_domains(
        conn: &Connection,
        id: &str,
    ) -> Result<Option<ArchetypeWithDomains>, ForgeError> {
        let base = match Self::get(conn, id)? {
            Some(a) => a,
            None => return Ok(None),
        };
        let domains = Self::list_domains(conn, id)?;
        Ok(Some(ArchetypeWithDomains { base, domains }))
    }

    /// List archetypes with pagination.
    pub fn list(
        conn: &Connection,
        offset: i64,
        limit: i64,
    ) -> Result<(Vec<Archetype>, Pagination), ForgeError> {
        let total: i64 = conn.query_row("SELECT COUNT(*) FROM archetypes", [], |row| row.get(0))?;

        let mut stmt = conn.prepare(
            "SELECT id, name, description, created_at
             FROM archetypes
             ORDER BY created_at DESC
             LIMIT ?1 OFFSET ?2",
        )?;
        let rows: Vec<Archetype> = stmt
            .query_map(params![limit, offset], Self::map_archetype)?
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

    /// List archetypes with aggregated counts.
    pub fn list_with_counts(
        conn: &Connection,
        offset: i64,
        limit: i64,
    ) -> Result<(Vec<ArchetypeWithCounts>, Pagination), ForgeError> {
        let total: i64 = conn.query_row("SELECT COUNT(*) FROM archetypes", [], |row| row.get(0))?;

        let mut stmt = conn.prepare(
            "SELECT a.id, a.name, a.description, a.created_at,
                    (SELECT COUNT(*) FROM resumes r WHERE r.archetype_id = a.id) AS resume_count,
                    (SELECT COUNT(*) FROM perspectives p WHERE p.archetype_id = a.id) AS perspective_count,
                    (SELECT COUNT(*) FROM archetype_domains ad WHERE ad.archetype_id = a.id) AS domain_count
             FROM archetypes a
             ORDER BY a.created_at DESC
             LIMIT ?1 OFFSET ?2",
        )?;
        let rows: Vec<ArchetypeWithCounts> = stmt
            .query_map(params![limit, offset], |row| {
                Ok(ArchetypeWithCounts {
                    base: Archetype {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        description: row.get(2)?,
                        created_at: row.get(3)?,
                    },
                    resume_count: row.get(4)?,
                    perspective_count: row.get(5)?,
                    domain_count: row.get(6)?,
                })
            })?
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

    /// Partially update an archetype.
    pub fn update(
        conn: &Connection,
        id: &str,
        input: &UpdateArchetypeInput,
    ) -> Result<Archetype, ForgeError> {
        Self::get(conn, id)?.ok_or_else(|| ForgeError::NotFound {
            entity_type: "archetype".into(),
            id: id.into(),
        })?;

        let mut sets = Vec::new();
        let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(ref v) = input.name {
            sets.push(format!("name = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = input.description {
            sets.push(format!("description = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }

        if !sets.is_empty() {
            let sql = format!(
                "UPDATE archetypes SET {} WHERE id = ?{}",
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
            .ok_or_else(|| ForgeError::Internal("Archetype updated but not found".into()))
    }

    // ── Delete ───────────────────────────────────────────────────────

    /// Delete an archetype by ID. Cascading deletes archetype_domains rows.
    pub fn delete(conn: &Connection, id: &str) -> Result<(), ForgeError> {
        let deleted = conn.execute("DELETE FROM archetypes WHERE id = ?1", params![id])?;
        if deleted == 0 {
            return Err(ForgeError::NotFound {
                entity_type: "archetype".into(),
                id: id.into(),
            });
        }
        Ok(())
    }

    // ── Domain junction ──────────────────────────────────────────────

    /// Link a domain to an archetype.
    pub fn add_domain(
        conn: &Connection,
        archetype_id: &str,
        domain_id: &str,
    ) -> Result<(), ForgeError> {
        let now = now_iso();
        conn.execute(
            "INSERT OR IGNORE INTO archetype_domains (archetype_id, domain_id, created_at) VALUES (?1, ?2, ?3)",
            params![archetype_id, domain_id, now],
        )?;
        Ok(())
    }

    /// Unlink a domain from an archetype.
    pub fn remove_domain(
        conn: &Connection,
        archetype_id: &str,
        domain_id: &str,
    ) -> Result<(), ForgeError> {
        conn.execute(
            "DELETE FROM archetype_domains WHERE archetype_id = ?1 AND domain_id = ?2",
            params![archetype_id, domain_id],
        )?;
        Ok(())
    }

    /// Get all domains linked to an archetype via the junction table.
    pub fn list_domains(conn: &Connection, archetype_id: &str) -> Result<Vec<Domain>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT d.id, d.name, d.description, d.created_at
             FROM domains d
             INNER JOIN archetype_domains ad ON ad.domain_id = d.id
             WHERE ad.archetype_id = ?1
             ORDER BY d.name ASC",
        )?;
        let domains: Vec<Domain> = stmt
            .query_map(params![archetype_id], |row| {
                Ok(Domain {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        Ok(domains)
    }

    // ── Row mapping ──────────────────────────────────────────────────

    fn map_archetype(row: &rusqlite::Row) -> rusqlite::Result<Archetype> {
        Ok(Archetype {
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

    fn create_domain(conn: &Connection, name: &str) -> Domain {
        let id = new_id();
        let now = now_iso();
        conn.execute(
            "INSERT INTO domains (id, name, description, created_at) VALUES (?1, ?2, NULL, ?3)",
            params![id, name, now],
        )
        .unwrap();
        Domain {
            id,
            name: name.into(),
            description: None,
            created_at: now,
        }
    }

    #[test]
    fn create_and_get_archetype() {
        let forge = setup();
        let input = CreateArchetypeInput {
            name: "Security Engineer".into(),
            description: Some("Offensive and defensive security roles".into()),
        };
        let arch = ArchetypeStore::create(forge.conn(), &input).unwrap();
        assert_eq!(arch.name, "Security Engineer");
        assert_eq!(
            arch.description,
            Some("Offensive and defensive security roles".into())
        );

        let fetched = ArchetypeStore::get(forge.conn(), &arch.id)
            .unwrap()
            .unwrap();
        assert_eq!(fetched.id, arch.id);
        assert_eq!(fetched.name, "Security Engineer");
    }

    #[test]
    fn get_returns_none_for_missing() {
        let forge = setup();
        let result = ArchetypeStore::get(forge.conn(), "nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn list_archetypes() {
        let forge = setup();
        // Count seed data first
        let (_, before) = ArchetypeStore::list(forge.conn(), 0, 100).unwrap();
        let seeded = before.total;

        ArchetypeStore::create(
            forge.conn(),
            &CreateArchetypeInput {
                name: "SWE".into(),
                description: None,
            },
        )
        .unwrap();
        ArchetypeStore::create(
            forge.conn(),
            &CreateArchetypeInput {
                name: "SRE".into(),
                description: None,
            },
        )
        .unwrap();

        let (archs, pagination) = ArchetypeStore::list(forge.conn(), 0, 100).unwrap();
        assert_eq!(archs.len() as i64, seeded + 2);
        assert_eq!(pagination.total, seeded + 2);
    }

    #[test]
    fn update_archetype() {
        let forge = setup();
        let arch = ArchetypeStore::create(
            forge.conn(),
            &CreateArchetypeInput {
                name: "Old Name".into(),
                description: None,
            },
        )
        .unwrap();

        let updated = ArchetypeStore::update(
            forge.conn(),
            &arch.id,
            &UpdateArchetypeInput {
                name: Some("New Name".into()),
                description: Some(Some("Now has a description".into())),
            },
        )
        .unwrap();
        assert_eq!(updated.name, "New Name");
        assert_eq!(updated.description, Some("Now has a description".into()));
    }

    #[test]
    fn update_missing_returns_not_found() {
        let forge = setup();
        let result = ArchetypeStore::update(
            forge.conn(),
            "nonexistent",
            &UpdateArchetypeInput::default(),
        );
        assert!(matches!(result, Err(ForgeError::NotFound { .. })));
    }

    #[test]
    fn delete_archetype() {
        let forge = setup();
        let arch = ArchetypeStore::create(
            forge.conn(),
            &CreateArchetypeInput {
                name: "To Delete".into(),
                description: None,
            },
        )
        .unwrap();

        ArchetypeStore::delete(forge.conn(), &arch.id).unwrap();
        assert!(ArchetypeStore::get(forge.conn(), &arch.id)
            .unwrap()
            .is_none());
    }

    #[test]
    fn delete_missing_returns_not_found() {
        let forge = setup();
        let result = ArchetypeStore::delete(forge.conn(), "nonexistent");
        assert!(matches!(result, Err(ForgeError::NotFound { .. })));
    }

    #[test]
    fn add_domain_and_list_domains() {
        let forge = setup();
        let arch = ArchetypeStore::create(
            forge.conn(),
            &CreateArchetypeInput {
                name: "SecEng".into(),
                description: None,
            },
        )
        .unwrap();

        let d1 = create_domain(forge.conn(), "Security");
        let d2 = create_domain(forge.conn(), "Cloud");

        ArchetypeStore::add_domain(forge.conn(), &arch.id, &d1.id).unwrap();
        ArchetypeStore::add_domain(forge.conn(), &arch.id, &d2.id).unwrap();

        let domains = ArchetypeStore::list_domains(forge.conn(), &arch.id).unwrap();
        assert_eq!(domains.len(), 2);
        let names: Vec<&str> = domains.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"Cloud"));
        assert!(names.contains(&"Security"));
    }

    #[test]
    fn add_domain_is_idempotent() {
        let forge = setup();
        let arch = ArchetypeStore::create(
            forge.conn(),
            &CreateArchetypeInput {
                name: "SWE".into(),
                description: None,
            },
        )
        .unwrap();
        let d1 = create_domain(forge.conn(), "Backend");

        ArchetypeStore::add_domain(forge.conn(), &arch.id, &d1.id).unwrap();
        ArchetypeStore::add_domain(forge.conn(), &arch.id, &d1.id).unwrap(); // no error

        let domains = ArchetypeStore::list_domains(forge.conn(), &arch.id).unwrap();
        assert_eq!(domains.len(), 1);
    }

    #[test]
    fn remove_domain() {
        let forge = setup();
        let arch = ArchetypeStore::create(
            forge.conn(),
            &CreateArchetypeInput {
                name: "SRE".into(),
                description: None,
            },
        )
        .unwrap();
        let d1 = create_domain(forge.conn(), "Infra");
        let d2 = create_domain(forge.conn(), "Observability");

        ArchetypeStore::add_domain(forge.conn(), &arch.id, &d1.id).unwrap();
        ArchetypeStore::add_domain(forge.conn(), &arch.id, &d2.id).unwrap();

        ArchetypeStore::remove_domain(forge.conn(), &arch.id, &d1.id).unwrap();

        let domains = ArchetypeStore::list_domains(forge.conn(), &arch.id).unwrap();
        assert_eq!(domains.len(), 1);
        assert_eq!(domains[0].name, "Observability");
    }

    #[test]
    fn get_with_domains() {
        let forge = setup();
        let arch = ArchetypeStore::create(
            forge.conn(),
            &CreateArchetypeInput {
                name: "Full-Stack".into(),
                description: Some("End-to-end development".into()),
            },
        )
        .unwrap();
        let d1 = create_domain(forge.conn(), "Frontend");

        ArchetypeStore::add_domain(forge.conn(), &arch.id, &d1.id).unwrap();

        let awd = ArchetypeStore::get_with_domains(forge.conn(), &arch.id)
            .unwrap()
            .unwrap();
        assert_eq!(awd.base.name, "Full-Stack");
        assert_eq!(awd.domains.len(), 1);
        assert_eq!(awd.domains[0].name, "Frontend");
    }

    #[test]
    fn delete_cascades_domain_links() {
        let forge = setup();
        let arch = ArchetypeStore::create(
            forge.conn(),
            &CreateArchetypeInput {
                name: "Temp".into(),
                description: None,
            },
        )
        .unwrap();
        let d = create_domain(forge.conn(), "Whatever");
        ArchetypeStore::add_domain(forge.conn(), &arch.id, &d.id).unwrap();

        ArchetypeStore::delete(forge.conn(), &arch.id).unwrap();

        // Junction rows should be gone too
        let count: i64 = forge
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM archetype_domains WHERE archetype_id = ?1",
                params![arch.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }
}
