//! Domain repository — CRUD for the `domains` lookup table.
//!
//! Domains are experience domains (e.g. `cloud_security`, `systems_engineering`)
//! used by perspectives and archetypes. Names are lowercase slugs.

use rusqlite::{params, Connection, OptionalExtension};

use forge_core::{
    new_id, now_iso, CreateDomainInput, Domain, DomainWithUsage, ForgeError, Pagination,
    UpdateDomainInput,
};

use super::lookup::{self, LookupTable};

/// Data-access store for the `domains` table.
pub struct DomainStore;

impl DomainStore {
    // ── Create ───────────────────────────────────────────────────────

    /// Insert a new domain row.
    ///
    /// TS order (domain-service.ts:22-34): empty, then format; then the ELM's
    /// uniqueness check (lifecycle-manager.ts:188-195), which is a 409.
    pub fn create(conn: &Connection, input: &CreateDomainInput) -> Result<Domain, ForgeError> {
        lookup::validate_domain_name(&input.name, lookup::DOMAIN_NAME_FORMAT_ON_CREATE)?;
        lookup::ensure_name_free(conn, LookupTable::Domains, &input.name, None)?;
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

    /// List domains with usage counts, sorted by name, one page at a time.
    ///
    /// Mirrors `DomainService.list` (packages/core/src/services/domain-service.ts:56-88).
    /// `perspective_count` matches perspectives on the domain's *name* (a text
    /// column, not an FK); `archetype_count` counts `archetype_domains` rows.
    pub fn list_with_usage(
        conn: &Connection,
        offset: i64,
        limit: i64,
    ) -> Result<(Vec<DomainWithUsage>, Pagination), ForgeError> {
        let total: i64 = conn.query_row("SELECT COUNT(*) FROM domains", [], |row| row.get(0))?;

        let mut stmt = conn.prepare(
            "SELECT d.id, d.name, d.description, d.created_at,
                    (SELECT COUNT(*) FROM perspectives p WHERE p.domain = d.name) AS perspective_count,
                    (SELECT COUNT(*) FROM archetype_domains ad WHERE ad.domain_id = d.id) AS archetype_count
             FROM domains d
             ORDER BY d.name ASC
             LIMIT ?1 OFFSET ?2",
        )?;
        let rows: Vec<DomainWithUsage> = stmt
            .query_map(params![limit, offset], |row| {
                Ok(DomainWithUsage {
                    base: Self::map_domain(row)?, // columns 0..=3
                    perspective_count: row.get(4)?,
                    archetype_count: row.get(5)?,
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

    /// Partially update a domain (`PATCH /domains/:id`).
    ///
    /// Same order as TS: the name rules (domain-service.ts:90-99), then 404, then
    /// uniqueness excluding this row (lifecycle-manager.ts:306-320, :362-370).
    /// A rename does not touch `perspectives.domain`, which keeps the old name as
    /// free text; TS doesn't rewrite it either (resu-mx/forge#17).
    pub fn update(
        conn: &Connection,
        id: &str,
        input: &UpdateDomainInput,
    ) -> Result<Domain, ForgeError> {
        let name = input.name.as_deref();
        if let Some(name) = name {
            lookup::validate_domain_name(name, lookup::DOMAIN_NAME_FORMAT_ON_UPDATE)?;
        }
        Self::get(conn, id)?.ok_or_else(|| ForgeError::NotFound {
            entity_type: "domain".into(),
            id: id.into(),
        })?;
        if let Some(name) = name {
            lookup::ensure_name_free(conn, LookupTable::Domains, name, Some(id))?;
        }
        lookup::update_name_description(
            conn,
            LookupTable::Domains,
            id,
            name,
            input.description.as_ref().map(|d| d.as_deref()),
        )?;
        Self::get(conn, id)?
            .ok_or_else(|| ForgeError::Internal("Domain updated but not found".into()))
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
        let domain = Self::get(conn, id)?.ok_or_else(|| ForgeError::NotFound {
            entity_type: "domain".into(),
            id: id.into(),
        })?;

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
        let bullet =
            BulletStore::create(forge.conn(), "Built APIs", None, None, None, &[], &[]).unwrap();
        PerspectiveStore::create_direct(
            forge.conn(),
            &bullet.id,
            "Designed APIs",
            None,
            Some(domain),
            None,
            false,
        )
        .unwrap();
    }

    fn make(forge: &Forge, name: &str) -> Domain {
        DomainStore::create(
            forge.conn(),
            &CreateDomainInput {
                name: name.into(),
                description: None,
            },
        )
        .unwrap()
    }

    #[test]
    fn list_with_usage_counts_archetype_links_and_perspectives() {
        let forge = setup();
        let (rows, page) = DomainStore::list_with_usage(forge.conn(), 0, 200).unwrap();
        assert_eq!(page.total as usize, rows.len());
        let security = rows.iter().find(|d| d.base.name == "security").unwrap();
        assert_eq!(security.archetype_count, 2); // migration 003: security-engineer, public-sector

        make(&forge, "fresh_dom");
        perspective_naming(&forge, "fresh_dom");
        let (rows, _) = DomainStore::list_with_usage(forge.conn(), 0, 200).unwrap();
        let fresh = rows.iter().find(|d| d.base.name == "fresh_dom").unwrap();
        assert_eq!((fresh.perspective_count, fresh.archetype_count), (1, 0));
    }

    #[test]
    fn list_with_usage_sorts_by_name_and_pages() {
        let forge = setup();
        let (all, _) = DomainStore::list_with_usage(forge.conn(), 0, 200).unwrap();
        let names: Vec<_> = all.iter().map(|d| d.base.name.clone()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);

        let (page, p) = DomainStore::list_with_usage(forge.conn(), 1, 2).unwrap();
        assert_eq!(page.len(), 2);
        assert_eq!(page[0].base.name, names[1]);
        assert_eq!((p.offset, p.limit, p.total), (1, 2, all.len() as i64));
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
        assert_eq!(
            DomainStore::count_archetypes(forge.conn(), SECURITY).unwrap(),
            before
        );
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
            name: "cloud_security".into(),
            description: Some("Securing cloud infrastructure and services".into()),
        };
        let domain = DomainStore::create(forge.conn(), &input).unwrap();
        assert_eq!(domain.name, "cloud_security");
        assert_eq!(
            domain.description,
            Some("Securing cloud infrastructure and services".into())
        );

        let fetched = DomainStore::get(forge.conn(), &domain.id).unwrap().unwrap();
        assert_eq!(fetched.id, domain.id);
        assert_eq!(fetched.name, "cloud_security");
    }

    #[test]
    fn list_domains() {
        let forge = setup();
        // Migrations seed domains, so count the baseline first
        let baseline = DomainStore::list(forge.conn()).unwrap().len();

        DomainStore::create(
            forge.conn(),
            &CreateDomainInput {
                name: "backend".into(),
                description: None,
            },
        )
        .unwrap();
        DomainStore::create(
            forge.conn(),
            &CreateDomainInput {
                name: "quantum_computing".into(),
                description: Some("Quantum computing and cryptography".into()),
            },
        )
        .unwrap();

        let rows = DomainStore::list(forge.conn()).unwrap();
        assert_eq!(rows.len(), baseline + 2);
        // Sorted by name ASC — verify our entries are present
        assert!(rows.iter().any(|d| d.name == "backend"));
        assert!(rows.iter().any(|d| d.name == "quantum_computing"));
    }

    #[test]
    fn delete_domain() {
        let forge = setup();
        let domain = DomainStore::create(
            forge.conn(),
            &CreateDomainInput {
                name: "to_delete".into(),
                description: None,
            },
        )
        .unwrap();
        DomainStore::delete(forge.conn(), &domain.id).unwrap();
        assert!(DomainStore::get(forge.conn(), &domain.id)
            .unwrap()
            .is_none());
    }

    fn new_domain(forge: &Forge, name: &str, description: Option<&str>) -> Domain {
        DomainStore::create(
            forge.conn(),
            &CreateDomainInput {
                name: name.into(),
                description: description.map(Into::into),
            },
        )
        .unwrap()
    }

    #[test]
    fn create_rejects_blank_and_malformed_names() {
        let forge = setup();
        let before = DomainStore::list(forge.conn()).unwrap().len();
        for (name, expected) in [
            ("", lookup::NAME_EMPTY),
            ("   ", lookup::NAME_EMPTY),
            ("Cloud Security", lookup::DOMAIN_NAME_FORMAT_ON_CREATE),
            ("2fa", lookup::DOMAIN_NAME_FORMAT_ON_CREATE),
            ("cloud-security", lookup::DOMAIN_NAME_FORMAT_ON_CREATE),
        ] {
            let err = DomainStore::create(
                forge.conn(),
                &CreateDomainInput {
                    name: name.into(),
                    description: None,
                },
            )
            .unwrap_err();
            assert!(
                matches!(&err, ForgeError::Validation { message, .. } if message == expected),
                "{name:?}"
            );
        }
        assert_eq!(DomainStore::list(forge.conn()).unwrap().len(), before);
    }

    #[test]
    fn create_duplicate_is_conflict() {
        let forge = setup();
        let before = DomainStore::list(forge.conn()).unwrap().len();
        let err = DomainStore::create(
            forge.conn(),
            &CreateDomainInput {
                name: "security".into(), // seeded by migration 003
                description: None,
            },
        )
        .unwrap_err();
        assert!(matches!(&err, ForgeError::Conflict { message }
            if message == "domains.name must be unique: \"security\" already exists"));
        assert_eq!(DomainStore::list(forge.conn()).unwrap().len(), before);
    }

    #[test]
    fn update_renames_and_clears_description() {
        let forge = setup();
        let d = new_domain(&forge, "patch_me", Some("Before"));
        let updated = DomainStore::update(
            forge.conn(),
            &d.id,
            &UpdateDomainInput {
                name: Some("patch_renamed".into()),
                description: Some(None),
            },
        )
        .unwrap();
        assert_eq!(updated.name, "patch_renamed");
        assert_eq!(updated.description, None);
        assert_eq!(updated.created_at, d.created_at);
    }

    #[test]
    fn update_leaves_absent_fields() {
        let forge = setup();
        let d = new_domain(&forge, "keep_name", None);
        let updated = DomainStore::update(
            forge.conn(),
            &d.id,
            &UpdateDomainInput {
                name: None,
                description: Some(Some("Now described".into())),
            },
        )
        .unwrap();
        assert_eq!(updated.name, "keep_name");
        assert_eq!(updated.description.as_deref(), Some("Now described"));
    }

    #[test]
    fn update_rejects_blank_and_malformed_names() {
        let forge = setup();
        let d = new_domain(&forge, "stays_put", None);
        for (name, expected) in [
            ("", lookup::NAME_EMPTY),
            ("   ", lookup::NAME_EMPTY),
            ("Cloud Security", lookup::DOMAIN_NAME_FORMAT_ON_UPDATE),
            ("2fa", lookup::DOMAIN_NAME_FORMAT_ON_UPDATE),
        ] {
            let err = DomainStore::update(
                forge.conn(),
                &d.id,
                &UpdateDomainInput {
                    name: Some(name.into()),
                    ..Default::default()
                },
            )
            .unwrap_err();
            assert!(
                matches!(&err, ForgeError::Validation { message, .. } if message == expected),
                "{name:?}"
            );
        }
        assert_eq!(
            DomainStore::get(forge.conn(), &d.id).unwrap().unwrap().name,
            "stays_put"
        );
    }

    #[test]
    fn update_to_taken_name_is_conflict_but_own_name_is_ok() {
        let forge = setup();
        let d = new_domain(&forge, "mine", None);
        let err = DomainStore::update(
            forge.conn(),
            &d.id,
            &UpdateDomainInput {
                name: Some("security".into()), // seeded by migration 003
                ..Default::default()
            },
        )
        .unwrap_err();
        assert!(matches!(err, ForgeError::Conflict { .. }));
        let ok = DomainStore::update(
            forge.conn(),
            &d.id,
            &UpdateDomainInput {
                name: Some("mine".into()),
                description: Some(Some("Edited".into())),
            },
        )
        .unwrap();
        assert_eq!(ok.description.as_deref(), Some("Edited"));
    }

    #[test]
    fn update_unknown_id_is_not_found() {
        let forge = setup();
        let err = DomainStore::update(
            forge.conn(),
            "nonexistent",
            &UpdateDomainInput {
                description: Some(Some("x".into())),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert!(matches!(err, ForgeError::NotFound { .. }));
    }

    #[test]
    fn update_validates_before_not_found() {
        let forge = setup();
        let err = DomainStore::update(
            forge.conn(),
            "nonexistent",
            &UpdateDomainInput {
                name: Some("   ".into()),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert!(matches!(err, ForgeError::Validation { .. }));
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
