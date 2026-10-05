//! Repository for contact persistence.
//!
//! Provides CRUD operations, junction table management, and reverse lookups
//! for the `contacts`, `contact_organizations`, `contact_job_descriptions`,
//! and `contact_resumes` tables.

use rusqlite::{params, Connection, OptionalExtension};

use forge_core::{
    new_id, now_iso, Contact, ContactFilter, ContactJDRelationship, ContactLink,
    ContactOrgRelationship, ContactResumeRelationship, ContactWithOrg, CreateContact, ForgeError,
    Pagination, UpdateContact,
};

/// A link to a missing contact or target breaks the junction's FOREIGN KEY, and
/// `INSERT OR IGNORE` does not cover foreign keys. TS answers 400 VALIDATION_ERROR
/// (packages/core/src/storage/error-mapper.ts:17). Fold into resu-mx/forge#98's
/// central mapping when that lands.
fn link_error(e: rusqlite::Error, junction: &str) -> ForgeError {
    if let rusqlite::Error::SqliteFailure(ref err, ref msg) = e {
        if err.code == rusqlite::ErrorCode::ConstraintViolation
            && msg.as_deref().unwrap_or("").contains("FOREIGN KEY")
        {
            return ForgeError::Validation {
                message: format!("{junction}: the contact or the linked record does not exist"),
                field: None,
            };
        }
    }
    e.into()
}

/// Data-access repository for contacts and their junction tables.
pub struct ContactStore;

impl ContactStore {
    // ── Core CRUD ───────────────────────────────────────────────────

    /// Insert a new contact row.
    pub fn create(conn: &Connection, input: &CreateContact) -> Result<Contact, ForgeError> {
        let id = new_id();
        let now = now_iso();

        conn.execute(
            "INSERT INTO contacts (id, name, title, email, phone, linkedin, team, dept, notes, organization_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)",
            params![
                id,
                input.name,
                input.title,
                input.email,
                input.phone,
                input.linkedin,
                input.team,
                input.dept,
                input.notes,
                input.organization_id,
                now,
            ],
        )?;

        Self::get(conn, &id)?
            .ok_or_else(|| ForgeError::Internal("Contact created but not found".into()))
    }

    /// Fetch a single contact by primary key.
    pub fn get(conn: &Connection, id: &str) -> Result<Option<Contact>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT id, name, title, email, phone, linkedin, team, dept, notes,
                    organization_id, created_at, updated_at
             FROM contacts WHERE id = ?1",
        )?;

        let result = stmt.query_row(params![id], Self::map_contact).optional()?;
        Ok(result)
    }

    /// Fetch a contact with its hydrated organization name.
    pub fn get_with_org(conn: &Connection, id: &str) -> Result<Option<ContactWithOrg>, ForgeError> {
        let contact = match Self::get(conn, id)? {
            Some(c) => c,
            None => return Ok(None),
        };
        let org_name = Self::lookup_org_name(conn, contact.organization_id.as_deref())?;
        Ok(Some(ContactWithOrg {
            base: contact,
            organization_name: org_name,
        }))
    }

    /// List contacts with optional filtering, search, and pagination.
    pub fn list(
        conn: &Connection,
        filter: &ContactFilter,
        offset: i64,
        limit: i64,
    ) -> Result<(Vec<ContactWithOrg>, Pagination), ForgeError> {
        let mut conditions = Vec::new();
        let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(ref org_id) = filter.organization_id {
            conditions.push(format!("organization_id = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(org_id.clone()));
        }
        if let Some(ref search) = filter.search {
            let param_idx = bind_values.len() + 1;
            conditions.push(format!(
                "(name LIKE ?{param_idx} COLLATE NOCASE OR title LIKE ?{param_idx} COLLATE NOCASE OR email LIKE ?{param_idx} COLLATE NOCASE)"
            ));
            bind_values.push(Box::new(format!("%{search}%")));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        // Count
        let count_sql = format!("SELECT COUNT(*) FROM contacts {where_clause}");
        let total: i64 = conn.query_row(
            &count_sql,
            rusqlite::params_from_iter(bind_values.iter().map(|b| b.as_ref())),
            |row| row.get(0),
        )?;

        // Fetch page
        let query_sql = format!(
            "SELECT id, name, title, email, phone, linkedin, team, dept, notes,
                    organization_id, created_at, updated_at
             FROM contacts {where_clause}
             ORDER BY name ASC
             LIMIT ?{} OFFSET ?{}",
            bind_values.len() + 1,
            bind_values.len() + 2
        );
        bind_values.push(Box::new(limit));
        bind_values.push(Box::new(offset));

        let mut stmt = conn.prepare(&query_sql)?;
        let contacts: Vec<Contact> = stmt
            .query_map(
                rusqlite::params_from_iter(bind_values.iter().map(|b| b.as_ref())),
                Self::map_contact,
            )?
            .collect::<Result<_, _>>()?;

        // Hydrate org names
        let mut hydrated = Vec::with_capacity(contacts.len());
        for contact in contacts {
            let org_name = Self::lookup_org_name(conn, contact.organization_id.as_deref())?;
            hydrated.push(ContactWithOrg {
                base: contact,
                organization_name: org_name,
            });
        }

        Ok((
            hydrated,
            Pagination {
                total,
                offset,
                limit,
            },
        ))
    }

    /// Apply a partial update to an existing contact.
    pub fn update(
        conn: &Connection,
        id: &str,
        input: &UpdateContact,
    ) -> Result<Contact, ForgeError> {
        Self::get(conn, id)?.ok_or_else(|| ForgeError::NotFound {
            entity_type: "contact".into(),
            id: id.into(),
        })?;

        let mut sets = Vec::new();
        let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(ref v) = input.name {
            sets.push(format!("name = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = input.title {
            sets.push(format!("title = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = input.email {
            sets.push(format!("email = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = input.phone {
            sets.push(format!("phone = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = input.linkedin {
            sets.push(format!("linkedin = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = input.team {
            sets.push(format!("team = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = input.dept {
            sets.push(format!("dept = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = input.notes {
            sets.push(format!("notes = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = input.organization_id {
            sets.push(format!("organization_id = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }

        if !sets.is_empty() {
            let now = now_iso();
            sets.push(format!("updated_at = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(now));

            let sql = format!(
                "UPDATE contacts SET {} WHERE id = ?{}",
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
            .ok_or_else(|| ForgeError::Internal("Contact updated but not found".into()))
    }

    /// Delete a contact by primary key.
    pub fn delete(conn: &Connection, id: &str) -> Result<(), ForgeError> {
        let deleted = conn.execute("DELETE FROM contacts WHERE id = ?1", params![id])?;
        if deleted == 0 {
            return Err(ForgeError::NotFound {
                entity_type: "contact".into(),
                id: id.into(),
            });
        }
        Ok(())
    }

    // ── Organization junction ───────────────────────────────────────

    /// Link a contact to an organization with a typed relationship (idempotent).
    pub fn link_organization(
        conn: &Connection,
        contact_id: &str,
        org_id: &str,
        relationship: ContactOrgRelationship,
    ) -> Result<(), ForgeError> {
        conn.execute(
            "INSERT OR IGNORE INTO contact_organizations (contact_id, organization_id, relationship)
             VALUES (?1, ?2, ?3)",
            params![contact_id, org_id, relationship.as_ref()],
        )
        .map_err(|e| link_error(e, "contact_organizations"))?;
        Ok(())
    }

    /// Remove a contact-organization link by composite key.
    pub fn unlink_organization(
        conn: &Connection,
        contact_id: &str,
        org_id: &str,
        relationship: ContactOrgRelationship,
    ) -> Result<(), ForgeError> {
        conn.execute(
            "DELETE FROM contact_organizations
             WHERE contact_id = ?1 AND organization_id = ?2 AND relationship = ?3",
            params![contact_id, org_id, relationship.as_ref()],
        )?;
        Ok(())
    }

    /// List organizations linked to a contact, with relationship type.
    /// Returns tuples of (org_id, org_name, relationship).
    pub fn list_organizations(
        conn: &Connection,
        contact_id: &str,
    ) -> Result<Vec<(String, String, ContactOrgRelationship)>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT co.organization_id, o.name, co.relationship
             FROM contact_organizations co
             JOIN organizations o ON o.id = co.organization_id
             WHERE co.contact_id = ?1
             ORDER BY o.name ASC",
        )?;

        let rows: Vec<(String, String, ContactOrgRelationship)> = stmt
            .query_map(params![contact_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?
                        .parse()
                        .unwrap_or(ContactOrgRelationship::Other),
                ))
            })?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    // ── Job description junction ────────────────────────────────────

    /// Link a contact to a job description with a typed relationship (idempotent).
    pub fn link_job_description(
        conn: &Connection,
        contact_id: &str,
        jd_id: &str,
        relationship: ContactJDRelationship,
    ) -> Result<(), ForgeError> {
        conn.execute(
            "INSERT OR IGNORE INTO contact_job_descriptions (contact_id, job_description_id, relationship)
             VALUES (?1, ?2, ?3)",
            params![contact_id, jd_id, relationship.as_ref()],
        )
        .map_err(|e| link_error(e, "contact_job_descriptions"))?;
        Ok(())
    }

    /// Remove a contact-job description link by composite key.
    pub fn unlink_job_description(
        conn: &Connection,
        contact_id: &str,
        jd_id: &str,
        relationship: ContactJDRelationship,
    ) -> Result<(), ForgeError> {
        conn.execute(
            "DELETE FROM contact_job_descriptions
             WHERE contact_id = ?1 AND job_description_id = ?2 AND relationship = ?3",
            params![contact_id, jd_id, relationship.as_ref()],
        )?;
        Ok(())
    }

    /// List job descriptions linked to a contact, with relationship type.
    /// Returns tuples of (jd_id, jd_title, org_name, relationship).
    pub fn list_job_descriptions(
        conn: &Connection,
        contact_id: &str,
    ) -> Result<Vec<(String, String, Option<String>, ContactJDRelationship)>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT cjd.job_description_id, jd.title, o.name, cjd.relationship
             FROM contact_job_descriptions cjd
             JOIN job_descriptions jd ON jd.id = cjd.job_description_id
             LEFT JOIN organizations o ON o.id = jd.organization_id
             WHERE cjd.contact_id = ?1
             ORDER BY jd.title ASC",
        )?;

        let rows: Vec<(String, String, Option<String>, ContactJDRelationship)> = stmt
            .query_map(params![contact_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?
                        .parse()
                        .unwrap_or(ContactJDRelationship::Other),
                ))
            })?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    // ── Resume junction ─────────────────────────────────────────────

    /// Link a contact to a resume with a typed relationship (idempotent).
    pub fn link_resume(
        conn: &Connection,
        contact_id: &str,
        resume_id: &str,
        relationship: ContactResumeRelationship,
    ) -> Result<(), ForgeError> {
        conn.execute(
            "INSERT OR IGNORE INTO contact_resumes (contact_id, resume_id, relationship)
             VALUES (?1, ?2, ?3)",
            params![contact_id, resume_id, relationship.as_ref()],
        )
        .map_err(|e| link_error(e, "contact_resumes"))?;
        Ok(())
    }

    /// Remove a contact-resume link by composite key.
    pub fn unlink_resume(
        conn: &Connection,
        contact_id: &str,
        resume_id: &str,
        relationship: ContactResumeRelationship,
    ) -> Result<(), ForgeError> {
        conn.execute(
            "DELETE FROM contact_resumes
             WHERE contact_id = ?1 AND resume_id = ?2 AND relationship = ?3",
            params![contact_id, resume_id, relationship.as_ref()],
        )?;
        Ok(())
    }

    /// List resumes linked to a contact, with relationship type.
    /// Returns tuples of (resume_id, resume_name, relationship).
    pub fn list_resumes(
        conn: &Connection,
        contact_id: &str,
    ) -> Result<Vec<(String, String, ContactResumeRelationship)>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT cr.resume_id, r.name, cr.relationship
             FROM contact_resumes cr
             JOIN resumes r ON r.id = cr.resume_id
             WHERE cr.contact_id = ?1
             ORDER BY r.name ASC",
        )?;

        let rows: Vec<(String, String, ContactResumeRelationship)> = stmt
            .query_map(params![contact_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?
                        .parse()
                        .unwrap_or(ContactResumeRelationship::Other),
                ))
            })?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    // ── Reverse lookups ─────────────────────────────────────────────

    /// List contacts linked to a given organization (reverse lookup).
    pub fn list_by_organization(
        conn: &Connection,
        org_id: &str,
    ) -> Result<Vec<ContactLink>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT c.id, c.name, c.title, c.email, co.relationship
             FROM contact_organizations co
             JOIN contacts c ON c.id = co.contact_id
             WHERE co.organization_id = ?1
             ORDER BY c.name ASC",
        )?;

        let rows: Vec<ContactLink> = stmt
            .query_map(params![org_id], Self::map_contact_link)?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    /// List contacts linked to a given job description (reverse lookup).
    pub fn list_by_job_description(
        conn: &Connection,
        jd_id: &str,
    ) -> Result<Vec<ContactLink>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT c.id, c.name, c.title, c.email, cjd.relationship
             FROM contact_job_descriptions cjd
             JOIN contacts c ON c.id = cjd.contact_id
             WHERE cjd.job_description_id = ?1
             ORDER BY c.name ASC",
        )?;

        let rows: Vec<ContactLink> = stmt
            .query_map(params![jd_id], Self::map_contact_link)?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    /// List contacts linked to a given resume (reverse lookup).
    pub fn list_by_resume(
        conn: &Connection,
        resume_id: &str,
    ) -> Result<Vec<ContactLink>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT c.id, c.name, c.title, c.email, cr.relationship
             FROM contact_resumes cr
             JOIN contacts c ON c.id = cr.contact_id
             WHERE cr.resume_id = ?1
             ORDER BY c.name ASC",
        )?;

        let rows: Vec<ContactLink> = stmt
            .query_map(params![resume_id], Self::map_contact_link)?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    // ── Row mapping ──────────────────────────────────────────────────

    fn map_contact(row: &rusqlite::Row) -> rusqlite::Result<Contact> {
        Ok(Contact {
            id: row.get(0)?,
            name: row.get(1)?,
            title: row.get(2)?,
            email: row.get(3)?,
            phone: row.get(4)?,
            linkedin: row.get(5)?,
            team: row.get(6)?,
            dept: row.get(7)?,
            notes: row.get(8)?,
            organization_id: row.get(9)?,
            created_at: row.get(10)?,
            updated_at: row.get(11)?,
        })
    }

    fn map_contact_link(row: &rusqlite::Row) -> rusqlite::Result<ContactLink> {
        Ok(ContactLink {
            contact_id: row.get(0)?,
            contact_name: row.get(1)?,
            contact_title: row.get(2)?,
            contact_email: row.get(3)?,
            relationship: row.get(4)?,
        })
    }

    fn lookup_org_name(
        conn: &Connection,
        org_id: Option<&str>,
    ) -> Result<Option<String>, ForgeError> {
        match org_id {
            Some(oid) => {
                let name: Option<String> = conn
                    .query_row(
                        "SELECT name FROM organizations WHERE id = ?1",
                        params![oid],
                        |row| row.get(0),
                    )
                    .optional()?;
                Ok(name)
            }
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::Forge;

    fn setup() -> Forge {
        Forge::open_memory().unwrap()
    }

    fn sample_input() -> CreateContact {
        CreateContact {
            name: "Jane Doe".into(),
            title: Some("Engineering Manager".into()),
            email: Some("jane@example.com".into()),
            phone: Some("+1-555-1234".into()),
            linkedin: Some("https://linkedin.com/in/janedoe".into()),
            team: Some("Platform".into()),
            dept: Some("Engineering".into()),
            notes: Some("Met at conference".into()),
            organization_id: None,
        }
    }

    #[test]
    fn create_and_get() {
        let forge = setup();
        let contact = ContactStore::create(forge.conn(), &sample_input()).unwrap();
        assert_eq!(contact.name, "Jane Doe");
        assert_eq!(contact.email, Some("jane@example.com".into()));

        let fetched = ContactStore::get(forge.conn(), &contact.id)
            .unwrap()
            .unwrap();
        assert_eq!(fetched.id, contact.id);
        assert_eq!(fetched.name, contact.name);
    }

    #[test]
    fn get_returns_none_for_missing() {
        let forge = setup();
        let result = ContactStore::get(forge.conn(), "nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn list_empty() {
        let forge = setup();
        let (rows, pagination) =
            ContactStore::list(forge.conn(), &ContactFilter::default(), 0, 50).unwrap();
        assert!(rows.is_empty());
        assert_eq!(pagination.total, 0);
    }

    #[test]
    fn list_with_search() {
        let forge = setup();
        ContactStore::create(forge.conn(), &sample_input()).unwrap();
        ContactStore::create(
            forge.conn(),
            &CreateContact {
                name: "John Smith".into(),
                email: Some("john@example.com".into()),
                ..sample_input()
            },
        )
        .unwrap();

        let (rows, _) = ContactStore::list(
            forge.conn(),
            &ContactFilter {
                search: Some("jane".into()),
                ..Default::default()
            },
            0,
            50,
        )
        .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].base.name, "Jane Doe");
    }

    #[test]
    fn update_contact() {
        let forge = setup();
        let created = ContactStore::create(forge.conn(), &sample_input()).unwrap();
        let updated = ContactStore::update(
            forge.conn(),
            &created.id,
            &UpdateContact {
                name: Some("Jane Smith".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(updated.name, "Jane Smith");
    }

    #[test]
    fn delete_contact() {
        let forge = setup();
        let created = ContactStore::create(forge.conn(), &sample_input()).unwrap();
        ContactStore::delete(forge.conn(), &created.id).unwrap();
        assert!(ContactStore::get(forge.conn(), &created.id)
            .unwrap()
            .is_none());
    }

    #[test]
    fn delete_missing_returns_not_found() {
        let forge = setup();
        let result = ContactStore::delete(forge.conn(), "nonexistent");
        assert!(matches!(result, Err(ForgeError::NotFound { .. })));
    }

    // ── Junctions ───────────────────────────────────────────────────

    fn seed_org(forge: &Forge, name: &str) -> String {
        let id = new_id();
        forge
            .conn()
            .execute(
                "INSERT INTO organizations (id, name) VALUES (?1, ?2)",
                params![id, name],
            )
            .unwrap();
        id
    }

    fn seed_jd(forge: &Forge, title: &str, org_id: Option<&str>) -> String {
        let id = new_id();
        forge
            .conn()
            .execute(
                "INSERT INTO job_descriptions (id, organization_id, title, raw_text) VALUES (?1, ?2, ?3, 'text')",
                params![id, org_id, title],
            )
            .unwrap();
        id
    }

    fn seed_resume(forge: &Forge, name: &str) -> String {
        let id = new_id();
        forge
            .conn()
            .execute(
                "INSERT INTO resumes (id, name, target_role, target_employer, archetype)
                 VALUES (?1, ?2, 'SRE', 'Acme', 'backend')",
                params![id, name],
            )
            .unwrap();
        id
    }

    #[test]
    fn link_organization_to_a_missing_org_is_a_validation_error() {
        let forge = setup();
        let c = ContactStore::create(forge.conn(), &sample_input()).unwrap();
        let err = ContactStore::link_organization(
            forge.conn(),
            &c.id,
            "no-such-org",
            ContactOrgRelationship::Recruiter,
        )
        .unwrap_err();
        assert!(matches!(err, ForgeError::Validation { .. }), "{err:?}");
    }

    #[test]
    fn link_organization_from_a_missing_contact_is_a_validation_error() {
        let forge = setup();
        let o = seed_org(&forge, "Acme");
        let err = ContactStore::link_organization(
            forge.conn(),
            "no-such-contact",
            &o,
            ContactOrgRelationship::Recruiter,
        )
        .unwrap_err();
        assert!(matches!(err, ForgeError::Validation { .. }), "{err:?}");
    }

    #[test]
    fn link_job_description_and_resume_to_missing_targets_are_validation_errors() {
        let forge = setup();
        let c = ContactStore::create(forge.conn(), &sample_input()).unwrap();
        let err = ContactStore::link_job_description(
            forge.conn(),
            &c.id,
            "no-such-jd",
            ContactJDRelationship::Recruiter,
        )
        .unwrap_err();
        assert!(matches!(err, ForgeError::Validation { .. }), "{err:?}");
        let err = ContactStore::link_resume(
            forge.conn(),
            &c.id,
            "no-such-resume",
            ContactResumeRelationship::Reference,
        )
        .unwrap_err();
        assert!(matches!(err, ForgeError::Validation { .. }), "{err:?}");
    }

    #[test]
    fn link_twice_is_a_noop() {
        let forge = setup();
        let c = ContactStore::create(forge.conn(), &sample_input()).unwrap();
        let o = seed_org(&forge, "Acme");
        let j = seed_jd(&forge, "SRE", Some(&o));
        let r = seed_resume(&forge, "Resume");
        for _ in 0..2 {
            ContactStore::link_organization(forge.conn(), &c.id, &o, ContactOrgRelationship::Hr)
                .unwrap();
            ContactStore::link_job_description(
                forge.conn(),
                &c.id,
                &j,
                ContactJDRelationship::Interviewer,
            )
            .unwrap();
            ContactStore::link_resume(
                forge.conn(),
                &c.id,
                &r,
                ContactResumeRelationship::Reference,
            )
            .unwrap();
        }
        assert_eq!(
            ContactStore::list_organizations(forge.conn(), &c.id)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            ContactStore::list_job_descriptions(forge.conn(), &c.id)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            ContactStore::list_resumes(forge.conn(), &c.id)
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn two_relationships_on_one_pair_are_both_listed() {
        let forge = setup();
        let c = ContactStore::create(forge.conn(), &sample_input()).unwrap();
        let o = seed_org(&forge, "Acme");
        let j = seed_jd(&forge, "SRE", None);
        let r = seed_resume(&forge, "Resume");
        ContactStore::link_organization(forge.conn(), &c.id, &o, ContactOrgRelationship::Recruiter)
            .unwrap();
        ContactStore::link_organization(forge.conn(), &c.id, &o, ContactOrgRelationship::Peer)
            .unwrap();
        ContactStore::link_job_description(
            forge.conn(),
            &c.id,
            &j,
            ContactJDRelationship::Recruiter,
        )
        .unwrap();
        ContactStore::link_job_description(
            forge.conn(),
            &c.id,
            &j,
            ContactJDRelationship::Referral,
        )
        .unwrap();
        ContactStore::link_resume(
            forge.conn(),
            &c.id,
            &r,
            ContactResumeRelationship::Reference,
        )
        .unwrap();
        ContactStore::link_resume(
            forge.conn(),
            &c.id,
            &r,
            ContactResumeRelationship::Recommender,
        )
        .unwrap();
        assert_eq!(
            ContactStore::list_organizations(forge.conn(), &c.id)
                .unwrap()
                .len(),
            2
        );
        let jds = ContactStore::list_job_descriptions(forge.conn(), &c.id).unwrap();
        assert_eq!(jds.len(), 2);
        assert_eq!(
            jds[0].2, None,
            "a JD without an organization has no organization name"
        );
        assert_eq!(
            ContactStore::list_resumes(forge.conn(), &c.id)
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn unlink_missing_row_is_ok() {
        let forge = setup();
        ContactStore::unlink_organization(forge.conn(), "x", "y", ContactOrgRelationship::Hr)
            .unwrap();
        ContactStore::unlink_job_description(forge.conn(), "x", "y", ContactJDRelationship::Other)
            .unwrap();
        ContactStore::unlink_resume(forge.conn(), "x", "y", ContactResumeRelationship::Other)
            .unwrap();
    }

    #[test]
    fn unlink_removes_only_the_named_relationship() {
        let forge = setup();
        let c = ContactStore::create(forge.conn(), &sample_input()).unwrap();
        let o = seed_org(&forge, "Acme");
        ContactStore::link_organization(forge.conn(), &c.id, &o, ContactOrgRelationship::Recruiter)
            .unwrap();
        ContactStore::link_organization(forge.conn(), &c.id, &o, ContactOrgRelationship::Peer)
            .unwrap();
        ContactStore::unlink_organization(
            forge.conn(),
            &c.id,
            &o,
            ContactOrgRelationship::Recruiter,
        )
        .unwrap();
        let rows = ContactStore::list_organizations(forge.conn(), &c.id).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].2, ContactOrgRelationship::Peer);
    }

    #[test]
    fn reverse_lookups_list_contact_links() {
        let forge = setup();
        let c = ContactStore::create(forge.conn(), &sample_input()).unwrap();
        let o = seed_org(&forge, "Acme");
        let j = seed_jd(&forge, "SRE", Some(&o));
        let r = seed_resume(&forge, "Resume");
        ContactStore::link_organization(forge.conn(), &c.id, &o, ContactOrgRelationship::Manager)
            .unwrap();
        ContactStore::link_job_description(
            forge.conn(),
            &c.id,
            &j,
            ContactJDRelationship::HiringManager,
        )
        .unwrap();
        ContactStore::link_resume(
            forge.conn(),
            &c.id,
            &r,
            ContactResumeRelationship::Recommender,
        )
        .unwrap();

        let by_org = ContactStore::list_by_organization(forge.conn(), &o).unwrap();
        assert_eq!(by_org.len(), 1);
        assert_eq!(by_org[0].contact_id, c.id);
        assert_eq!(by_org[0].contact_name, "Jane Doe");
        assert_eq!(by_org[0].relationship, "manager");
        assert_eq!(
            ContactStore::list_by_job_description(forge.conn(), &j)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            ContactStore::list_by_resume(forge.conn(), &r)
                .unwrap()
                .len(),
            1
        );
        assert!(ContactStore::list_by_resume(forge.conn(), "unknown")
            .unwrap()
            .is_empty());
    }
}
