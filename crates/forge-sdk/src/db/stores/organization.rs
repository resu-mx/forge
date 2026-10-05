//! Organization repository — data access layer for organizations, tags,
//! aliases, and locations.
//!
//! Organizations have the widest cascade breadth of any entity: their
//! `cascade` includes `org_tags`, `org_locations`, `org_aliases`,
//! `contact_organizations`; and `setNull` includes `source_roles`,
//! `source_projects`, `source_education`, `contacts`, `job_descriptions`,
//! `credentials`, `certifications`.

use rusqlite::{params, Connection, OptionalExtension};

use forge_core::{
    new_id, now_iso, CreateOrganizationInput, ForgeError, OrgTag, Organization, OrganizationFilter,
    OrganizationStatus, Pagination, UpdateOrganizationInput,
};

/// `organizations.org_type` values (the column's CHECK; TS `VALID_ORG_TYPES`).
const VALID_ORG_TYPES: [&str; 8] = [
    "company",
    "nonprofit",
    "government",
    "military",
    "education",
    "volunteer",
    "freelance",
    "other",
];

/// Data access for the `organizations`, `org_tags`, `org_aliases`, and
/// `org_locations` tables.
pub struct OrganizationStore;

impl OrganizationStore {
    // ── Organization CRUD ───────────────────────────────────────────

    /// Insert a new organization row and seed its tags.
    ///
    /// If `tags` is omitted, defaults to `[org_type]` (e.g. `["company"]`).
    /// The `worked` field is a boolean stored as `0/1`.
    pub fn create(
        conn: &Connection,
        input: &CreateOrganizationInput,
    ) -> Result<Organization, ForgeError> {
        let id = new_id();
        let now = now_iso();
        let org_type = input.org_type.as_deref().unwrap_or("company");

        conn.execute(
            "INSERT INTO organizations (id, name, org_type, industry, size, worked, employment_type, website, linkedin_url, glassdoor_url, glassdoor_rating, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13)",
            params![
                id,
                input.name,
                org_type,
                input.industry,
                input.size,
                input.worked.unwrap_or(0),
                input.employment_type,
                input.website,
                input.linkedin_url,
                input.glassdoor_url,
                input.glassdoor_rating,
                input.status.map(|s| s.to_string()),
                now,
            ],
        )?;

        // Seed tags: default to [org_type] if none provided
        let tags = input.tags.as_deref().unwrap_or(&[]);
        if tags.is_empty() {
            // Use org_type as the default tag
            Self::insert_tag_ignore(conn, &id, org_type)?;
        } else {
            for tag in tags {
                Self::insert_tag_ignore(conn, &id, tag)?;
            }
        }

        Self::get(conn, &id)?
            .ok_or_else(|| ForgeError::Internal("Organization created but not found".into()))
    }

    /// Fetch a single organization by ID, including computed `tags` array
    /// from the `org_tags` junction table.
    pub fn get(conn: &Connection, id: &str) -> Result<Option<Organization>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT id, name, org_type, industry, size, worked, employment_type,
                    website, linkedin_url, glassdoor_url, glassdoor_rating, status,
                    created_at, updated_at
             FROM organizations WHERE id = ?1",
        )?;

        let row = stmt
            .query_row(params![id], Self::map_org_without_tags)
            .optional()?;
        match row {
            None => Ok(None),
            Some(mut org) => {
                org.tags = Self::get_tags(conn, &org.id)?;
                Ok(Some(org))
            }
        }
    }

    /// List organizations with optional filters. Supports:
    /// - `org_type` exact match
    /// - `tag` filter (walks `org_tags` junction)
    /// - `worked` boolean filter
    /// - `status` exact match
    /// - `search` substring match on `name` OR `org_aliases.alias`
    ///
    /// Results sorted by `name ASC` with pagination.
    pub fn list(
        conn: &Connection,
        filter: Option<&OrganizationFilter>,
        offset: Option<i64>,
        limit: Option<i64>,
    ) -> Result<(Vec<Organization>, Pagination), ForgeError> {
        let off = offset.unwrap_or(0);
        let lim = limit.unwrap_or(50);

        let mut conditions = Vec::new();
        let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(f) = filter {
            if let Some(ref ot) = f.org_type {
                conditions.push(format!("o.org_type = ?{}", bind_values.len() + 1));
                bind_values.push(Box::new(ot.clone()));
            }
            if let Some(w) = f.worked {
                conditions.push(format!("o.worked = ?{}", bind_values.len() + 1));
                bind_values.push(Box::new(w));
            }
            if let Some(ref st) = f.status {
                conditions.push(format!("o.status = ?{}", bind_values.len() + 1));
                bind_values.push(Box::new(st.clone()));
            }
            if let Some(ref tag) = f.tag {
                let param_idx = bind_values.len() + 1;
                conditions.push(format!(
                    "o.id IN (SELECT organization_id FROM org_tags WHERE tag = ?{param_idx})"
                ));
                bind_values.push(Box::new(tag.clone()));
            }
            if let Some(ref search) = f.search {
                let param_idx = bind_values.len() + 1;
                conditions.push(format!(
                    "(o.name LIKE ?{param_idx} OR o.id IN (SELECT organization_id FROM org_aliases WHERE alias LIKE ?{param_idx}))"
                ));
                bind_values.push(Box::new(format!("%{search}%")));
            }
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        // Count
        let count_sql = format!("SELECT COUNT(*) FROM organizations o {where_clause}");
        let total: i64 = conn.query_row(
            &count_sql,
            rusqlite::params_from_iter(bind_values.iter().map(|b| b.as_ref())),
            |row| row.get(0),
        )?;

        // Fetch page
        let query_sql = format!(
            "SELECT o.id, o.name, o.org_type, o.industry, o.size, o.worked, o.employment_type,
                    o.website, o.linkedin_url, o.glassdoor_url, o.glassdoor_rating, o.status,
                    o.created_at, o.updated_at
             FROM organizations o {where_clause}
             ORDER BY o.name ASC
             LIMIT ?{} OFFSET ?{}",
            bind_values.len() + 1,
            bind_values.len() + 2
        );
        bind_values.push(Box::new(lim));
        bind_values.push(Box::new(off));

        let mut stmt = conn.prepare(&query_sql)?;
        let orgs: Vec<Organization> = stmt
            .query_map(
                rusqlite::params_from_iter(bind_values.iter().map(|b| b.as_ref())),
                Self::map_org_without_tags,
            )?
            .collect::<Result<_, _>>()?;

        // Hydrate tags for each org
        let mut hydrated = Vec::with_capacity(orgs.len());
        for mut org in orgs {
            org.tags = Self::get_tags(conn, &org.id)?;
            hydrated.push(org);
        }

        Ok((
            hydrated,
            Pagination {
                total,
                offset: off,
                limit: lim,
            },
        ))
    }

    /// Partially update an organization (`PATCH /organizations/:id`).
    ///
    /// Absent fields are left alone; `Some(None)` sets a nullable column to NULL. If `tags` is
    /// provided, the tag list is replaced (delete-all + re-insert semantics).
    pub fn update(
        conn: &Connection,
        id: &str,
        input: &UpdateOrganizationInput,
    ) -> Result<Organization, ForgeError> {
        // Validate the body first, then check the id exists: the order TS uses.
        if input.name.as_deref().is_some_and(|n| n.trim().is_empty()) {
            return Err(ForgeError::Validation {
                message: "Name must not be empty".into(),
                field: Some("name".into()),
            });
        }
        if let Some(ot) = input.org_type.as_deref().filter(|ot| !ot.is_empty()) {
            if !VALID_ORG_TYPES.contains(&ot) {
                return Err(ForgeError::Validation {
                    message: format!("Invalid org_type: {ot}"),
                    field: Some("org_type".into()),
                });
            }
        }

        Self::get(conn, id)?.ok_or_else(|| ForgeError::NotFound {
            entity_type: "organization".into(),
            id: id.into(),
        })?;

        let mut sets: Vec<String> = Vec::new();
        let mut binds: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut set = |col: &str, value: Box<dyn rusqlite::types::ToSql>| {
            binds.push(value);
            sets.push(format!("{col} = ?{}", binds.len()));
        };

        if let Some(v) = &input.name {
            set("name", Box::new(v.clone()));
        }
        // An empty org_type is treated as absent, as TS does.
        if let Some(v) = input.org_type.as_ref().filter(|v| !v.is_empty()) {
            set("org_type", Box::new(v.clone()));
        }
        // Option<Option<_>>: the inner Option binds NULL when it is None.
        if let Some(v) = &input.industry {
            set("industry", Box::new(v.clone()));
        }
        if let Some(v) = &input.size {
            set("size", Box::new(v.clone()));
        }
        if let Some(v) = input.worked {
            set("worked", Box::new(v));
        }
        if let Some(v) = &input.employment_type {
            set("employment_type", Box::new(v.clone()));
        }
        if let Some(v) = &input.website {
            set("website", Box::new(v.clone()));
        }
        if let Some(v) = &input.linkedin_url {
            set("linkedin_url", Box::new(v.clone()));
        }
        if let Some(v) = &input.glassdoor_url {
            set("glassdoor_url", Box::new(v.clone()));
        }
        if let Some(v) = input.glassdoor_rating {
            set("glassdoor_rating", Box::new(v));
        }
        if let Some(v) = input.status {
            set("status", Box::new(v.map(|s| s.to_string())));
        }
        set("updated_at", Box::new(now_iso()));

        let sql = format!(
            "UPDATE organizations SET {} WHERE id = ?{}",
            sets.join(", "),
            binds.len() + 1
        );
        binds.push(Box::new(id.to_string()));
        conn.execute(
            &sql,
            rusqlite::params_from_iter(binds.iter().map(|b| b.as_ref())),
        )?;

        if let Some(ref tags) = input.tags {
            Self::replace_tags(conn, id, tags)?;
        }

        Self::get(conn, id)?
            .ok_or_else(|| ForgeError::Internal("Organization updated but not found".into()))
    }

    /// Delete an organization by ID.
    pub fn delete(conn: &Connection, id: &str) -> Result<(), ForgeError> {
        let deleted = conn.execute("DELETE FROM organizations WHERE id = ?1", params![id])?;
        if deleted == 0 {
            return Err(ForgeError::NotFound {
                entity_type: "organization".into(),
                id: id.into(),
            });
        }
        Ok(())
    }

    // ── Tags ────────────────────────────────────────────────────────

    /// Fetch all tags for an organization, sorted alphabetically.
    pub fn get_tags(conn: &Connection, org_id: &str) -> Result<Vec<OrgTag>, ForgeError> {
        let mut stmt =
            conn.prepare("SELECT tag FROM org_tags WHERE organization_id = ?1 ORDER BY tag ASC")?;
        let tags: Vec<OrgTag> = stmt
            .query_map(params![org_id], |row| {
                let tag_str: String = row.get(0)?;
                Ok(tag_str.parse::<OrgTag>().unwrap_or(OrgTag::Other))
            })?
            .collect::<Result<_, _>>()?;
        Ok(tags)
    }

    /// Replace the entire tag list for an organization (delete-all then
    /// insert). Invalid tags are silently dropped, matching the historical
    /// `INSERT OR IGNORE` semantics.
    pub fn replace_tags(
        conn: &Connection,
        org_id: &str,
        tags: &[String],
    ) -> Result<(), ForgeError> {
        conn.execute(
            "DELETE FROM org_tags WHERE organization_id = ?1",
            params![org_id],
        )?;
        for tag in tags {
            Self::insert_tag_ignore(conn, org_id, tag)?;
        }
        Ok(())
    }

    // ── Aliases ─────────────────────────────────────────────────────

    /// Find organization IDs that have an alias matching a LIKE pattern.
    /// Used by the search filter to extend name-based search to aliases.
    pub fn find_ids_by_alias_pattern(
        conn: &Connection,
        pattern: &str,
    ) -> Result<Vec<String>, ForgeError> {
        let mut stmt =
            conn.prepare("SELECT DISTINCT organization_id FROM org_aliases WHERE alias LIKE ?1")?;
        let ids: Vec<String> = stmt
            .query_map(params![pattern], |row| row.get(0))?
            .collect::<Result<_, _>>()?;
        Ok(ids)
    }

    // ── Internal helpers ────────────────────────────────────────────

    /// Insert a tag for an organization, silently ignoring invalid/duplicate tags.
    fn insert_tag_ignore(conn: &Connection, org_id: &str, tag: &str) -> Result<(), ForgeError> {
        // Use INSERT OR IGNORE to silently drop invalid tags
        let _ = conn.execute(
            "INSERT OR IGNORE INTO org_tags (organization_id, tag) VALUES (?1, ?2)",
            params![org_id, tag],
        );
        Ok(())
    }

    // ── Row mapping ─────────────────────────────────────────────────

    /// Map a row to an Organization with an empty tags vec (caller hydrates).
    fn map_org_without_tags(row: &rusqlite::Row) -> rusqlite::Result<Organization> {
        Ok(Organization {
            id: row.get(0)?,
            name: row.get(1)?,
            org_type: row.get(2)?,
            tags: Vec::new(), // hydrated by caller
            industry: row.get(3)?,
            size: row.get(4)?,
            worked: row.get(5)?,
            employment_type: row.get(6)?,
            website: row.get(7)?,
            linkedin_url: row.get(8)?,
            glassdoor_url: row.get(9)?,
            glassdoor_rating: row.get(10)?,
            status: row
                .get::<_, Option<String>>(11)?
                .and_then(|s| s.parse::<OrganizationStatus>().ok()),
            created_at: row.get(12)?,
            updated_at: row.get(13)?,
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
    fn create_and_get_organization() {
        let forge = setup();
        let input = CreateOrganizationInput {
            name: "Acme Corp".into(),
            org_type: Some("company".into()),
            tags: None,
            industry: Some("tech".into()),
            size: Some("100-500".into()),
            worked: Some(1),
            employment_type: Some("full-time".into()),
            website: Some("https://acme.com".into()),
            linkedin_url: None,
            glassdoor_url: None,
            glassdoor_rating: None,
            status: Some(OrganizationStatus::Interested),
        };
        let org = OrganizationStore::create(forge.conn(), &input).unwrap();
        assert_eq!(org.name, "Acme Corp");
        assert_eq!(org.org_type, "company");
        assert_eq!(org.worked, 1);
        assert!(org.tags.contains(&OrgTag::Company)); // default tag from org_type

        let fetched = OrganizationStore::get(forge.conn(), &org.id)
            .unwrap()
            .unwrap();
        assert_eq!(fetched.id, org.id);
        assert_eq!(fetched.name, "Acme Corp");
    }

    #[test]
    fn create_with_explicit_tags() {
        let forge = setup();
        let input = CreateOrganizationInput {
            name: "Startup Inc".into(),
            org_type: Some("company".into()),
            tags: Some(vec!["company".into(), "vendor".into()]),
            industry: None,
            size: None,
            worked: None,
            employment_type: None,
            website: None,
            linkedin_url: None,
            glassdoor_url: None,
            glassdoor_rating: None,
            status: None,
        };
        let org = OrganizationStore::create(forge.conn(), &input).unwrap();
        assert!(org.tags.contains(&OrgTag::Company));
        assert!(org.tags.contains(&OrgTag::Vendor));
    }

    #[test]
    fn get_returns_none_for_missing() {
        let forge = setup();
        let result = OrganizationStore::get(forge.conn(), "nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn list_empty() {
        let forge = setup();
        let (orgs, pagination) = OrganizationStore::list(forge.conn(), None, None, None).unwrap();
        assert!(orgs.is_empty());
        assert_eq!(pagination.total, 0);
    }

    #[test]
    fn list_with_org_type_filter() {
        let forge = setup();
        let mk = |name: &str, ot: &str| CreateOrganizationInput {
            name: name.into(),
            org_type: Some(ot.into()),
            tags: None,
            industry: None,
            size: None,
            worked: None,
            employment_type: None,
            website: None,
            linkedin_url: None,
            glassdoor_url: None,
            glassdoor_rating: None,
            status: None,
        };
        OrganizationStore::create(forge.conn(), &mk("CompanyA", "company")).unwrap();
        OrganizationStore::create(forge.conn(), &mk("University", "education")).unwrap();

        let filter = OrganizationFilter {
            org_type: Some("company".into()),
            ..Default::default()
        };
        let (orgs, _) = OrganizationStore::list(forge.conn(), Some(&filter), None, None).unwrap();
        assert_eq!(orgs.len(), 1);
        assert_eq!(orgs[0].name, "CompanyA");
    }

    #[test]
    fn list_with_search_filter() {
        let forge = setup();
        let mk = |name: &str| CreateOrganizationInput {
            name: name.into(),
            org_type: None,
            tags: None,
            industry: None,
            size: None,
            worked: None,
            employment_type: None,
            website: None,
            linkedin_url: None,
            glassdoor_url: None,
            glassdoor_rating: None,
            status: None,
        };
        OrganizationStore::create(forge.conn(), &mk("Google")).unwrap();
        OrganizationStore::create(forge.conn(), &mk("Amazon")).unwrap();

        let filter = OrganizationFilter {
            search: Some("goo".into()),
            ..Default::default()
        };
        let (orgs, _) = OrganizationStore::list(forge.conn(), Some(&filter), None, None).unwrap();
        assert_eq!(orgs.len(), 1);
        assert_eq!(orgs[0].name, "Google");
    }

    #[test]
    fn update_organization() {
        let forge = setup();
        let input = CreateOrganizationInput {
            name: "Old Name".into(),
            org_type: Some("company".into()),
            tags: None,
            industry: None,
            size: None,
            worked: None,
            employment_type: None,
            website: None,
            linkedin_url: None,
            glassdoor_url: None,
            glassdoor_rating: None,
            status: None,
        };
        let org = OrganizationStore::create(forge.conn(), &input).unwrap();

        let update_input = UpdateOrganizationInput {
            name: Some("New Name".into()),
            org_type: Some("nonprofit".into()),
            tags: Some(vec!["nonprofit".into()]),
            industry: Some(Some("education".into())),
            worked: Some(0),
            ..Default::default()
        };
        let updated = OrganizationStore::update(forge.conn(), &org.id, &update_input).unwrap();
        assert_eq!(updated.name, "New Name");
        assert_eq!(updated.org_type, "nonprofit");
        assert!(updated.tags.contains(&OrgTag::Nonprofit));
    }

    fn seed(forge: &Forge) -> Organization {
        OrganizationStore::create(
            forge.conn(),
            &CreateOrganizationInput {
                name: "Acme".into(),
                org_type: Some("company".into()),
                tags: None,
                industry: None,
                size: None,
                worked: None,
                employment_type: None,
                website: Some("https://acme.test".into()),
                linkedin_url: None,
                glassdoor_url: None,
                glassdoor_rating: None,
                status: Some(OrganizationStatus::Backlog),
            },
        )
        .unwrap()
    }

    #[test]
    fn update_with_only_status_leaves_other_fields() {
        let forge = setup();
        let org = seed(&forge);
        let input = UpdateOrganizationInput {
            status: Some(Some(OrganizationStatus::Researching)),
            ..Default::default()
        };
        let updated = OrganizationStore::update(forge.conn(), &org.id, &input).unwrap();
        assert_eq!(updated.status, Some(OrganizationStatus::Researching));
        assert_eq!(updated.name, "Acme");
        assert_eq!(updated.website.as_deref(), Some("https://acme.test"));
    }

    #[test]
    fn update_status_null_clears_it() {
        let forge = setup();
        let org = seed(&forge);
        let input = UpdateOrganizationInput {
            status: Some(None),
            ..Default::default()
        };
        assert_eq!(
            OrganizationStore::update(forge.conn(), &org.id, &input)
                .unwrap()
                .status,
            None
        );
    }

    #[test]
    fn update_null_clears_a_nullable_field() {
        let forge = setup();
        let org = seed(&forge);
        let input = UpdateOrganizationInput {
            website: Some(None),
            ..Default::default()
        };
        let updated = OrganizationStore::update(forge.conn(), &org.id, &input).unwrap();
        assert_eq!(updated.website, None);
        assert_eq!(updated.status, Some(OrganizationStatus::Backlog));
    }

    #[test]
    fn update_tags_replace_the_list() {
        let forge = setup();
        let org = seed(&forge);
        let set = |tags: Vec<String>| UpdateOrganizationInput {
            tags: Some(tags),
            ..Default::default()
        };
        let updated =
            OrganizationStore::update(forge.conn(), &org.id, &set(vec!["nonprofit".into()]))
                .unwrap();
        assert_eq!(updated.tags, vec![OrgTag::Nonprofit]);
        let updated =
            OrganizationStore::update(forge.conn(), &org.id, &set(vec!["university".into()]))
                .unwrap();
        assert_eq!(updated.tags, vec![OrgTag::University]);
        // Absent tags leave the list alone.
        let updated = OrganizationStore::update(
            forge.conn(),
            &org.id,
            &UpdateOrganizationInput {
                name: Some("Acme 2".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(updated.tags, vec![OrgTag::University]);
    }

    #[test]
    fn update_rejects_a_blank_name() {
        let forge = setup();
        let org = seed(&forge);
        let input = UpdateOrganizationInput {
            name: Some("  ".into()),
            ..Default::default()
        };
        assert!(matches!(
            OrganizationStore::update(forge.conn(), &org.id, &input),
            Err(ForgeError::Validation { .. })
        ));
    }

    #[test]
    fn update_rejects_an_unknown_org_type() {
        let forge = setup();
        let org = seed(&forge);
        let input = UpdateOrganizationInput {
            org_type: Some("guild".into()),
            ..Default::default()
        };
        assert!(matches!(
            OrganizationStore::update(forge.conn(), &org.id, &input),
            Err(ForgeError::Validation { .. })
        ));
    }

    #[test]
    fn update_unknown_id_is_not_found() {
        let forge = setup();
        let input = UpdateOrganizationInput {
            name: Some("X".into()),
            ..Default::default()
        };
        assert!(matches!(
            OrganizationStore::update(forge.conn(), "nope", &input),
            Err(ForgeError::NotFound { .. })
        ));
    }

    #[test]
    fn delete_organization() {
        let forge = setup();
        let input = CreateOrganizationInput {
            name: "To Delete".into(),
            org_type: None,
            tags: None,
            industry: None,
            size: None,
            worked: None,
            employment_type: None,
            website: None,
            linkedin_url: None,
            glassdoor_url: None,
            glassdoor_rating: None,
            status: None,
        };
        let org = OrganizationStore::create(forge.conn(), &input).unwrap();
        OrganizationStore::delete(forge.conn(), &org.id).unwrap();
        assert!(OrganizationStore::get(forge.conn(), &org.id)
            .unwrap()
            .is_none());
    }

    #[test]
    fn delete_missing_returns_not_found() {
        let forge = setup();
        let result = OrganizationStore::delete(forge.conn(), "nonexistent");
        assert!(matches!(result, Err(ForgeError::NotFound { .. })));
    }

    #[test]
    fn replace_tags() {
        let forge = setup();
        let input = CreateOrganizationInput {
            name: "TagTest".into(),
            org_type: Some("company".into()),
            tags: Some(vec!["company".into()]),
            industry: None,
            size: None,
            worked: None,
            employment_type: None,
            website: None,
            linkedin_url: None,
            glassdoor_url: None,
            glassdoor_rating: None,
            status: None,
        };
        let org = OrganizationStore::create(forge.conn(), &input).unwrap();
        assert!(org.tags.contains(&OrgTag::Company));

        OrganizationStore::replace_tags(
            forge.conn(),
            &org.id,
            &["vendor".into(), "platform".into()],
        )
        .unwrap();
        let tags = OrganizationStore::get_tags(forge.conn(), &org.id).unwrap();
        assert!(!tags.contains(&OrgTag::Company));
        assert!(tags.contains(&OrgTag::Vendor));
        assert!(tags.contains(&OrgTag::Platform));
    }
}
