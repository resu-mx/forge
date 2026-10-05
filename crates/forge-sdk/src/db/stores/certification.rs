//! Certification repository — CRUD for certifications and the
//! `certification_skills` junction table.

use rusqlite::{params, Connection, OptionalExtension};

use forge_core::{
    new_id, now_iso, Certification, CertificationWithSkills, CreateCertification, ForgeError,
    Skill, SkillCategory, UpdateCertification,
};

/// The trimmed value, or a `Validation` error carrying the TS message
/// (certification-service.ts:35-41, :104-116).
fn non_empty<'a>(value: &'a str, field: &str, message: &str) -> Result<&'a str, ForgeError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ForgeError::Validation {
            message: message.into(),
            field: Some(field.into()),
        });
    }
    Ok(trimmed)
}

/// Data access for the `certifications` and `certification_skills` tables.
pub struct CertificationStore;

impl CertificationStore {
    // ── Create ───────────────────────────────────────────────────────

    /// Insert a new certification row.
    pub fn create(
        conn: &Connection,
        input: &CreateCertification,
    ) -> Result<Certification, ForgeError> {
        let short_name = non_empty(&input.short_name, "short_name", "short_name is required")?;
        let long_name = non_empty(&input.long_name, "long_name", "long_name is required")?;
        let id = new_id();
        let now = now_iso();

        conn.execute(
            "INSERT INTO certifications (id, short_name, long_name, cert_id, issuer_id, date_earned, expiry_date, credential_id, credential_url, credly_url, in_progress, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12)",
            params![
                id,
                short_name,
                long_name,
                input.cert_id,
                input.issuer_id,
                input.date_earned,
                input.expiry_date,
                input.credential_id,
                input.credential_url,
                input.credly_url,
                input.in_progress.unwrap_or(false) as i32,
                now,
            ],
        )?;

        Self::get(conn, &id)?
            .ok_or_else(|| ForgeError::Internal("Certification created but not found".into()))
    }

    // ── Read ─────────────────────────────────────────────────────────

    /// Fetch a single certification by ID.
    pub fn get(conn: &Connection, id: &str) -> Result<Option<Certification>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT id, short_name, long_name, cert_id, issuer_id, date_earned,
                    expiry_date, credential_id, credential_url, credly_url,
                    in_progress, created_at, updated_at
             FROM certifications WHERE id = ?1",
        )?;
        let result = stmt
            .query_row(params![id], Self::map_certification)
            .optional()?;
        Ok(result)
    }

    /// Every certification, sorted by `short_name`. No pagination: the TS route
    /// returns all rows (certification-service.ts:86-102).
    pub fn list_all(conn: &Connection) -> Result<Vec<Certification>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT id, short_name, long_name, cert_id, issuer_id, date_earned,
                    expiry_date, credential_id, credential_url, credly_url,
                    in_progress, created_at, updated_at
             FROM certifications
             ORDER BY short_name ASC",
        )?;
        let rows: Vec<Certification> = stmt
            .query_map([], Self::map_certification)?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    /// A certification with its linked skills, or `None` for an unknown id.
    pub fn get_with_skills(
        conn: &Connection,
        id: &str,
    ) -> Result<Option<CertificationWithSkills>, ForgeError> {
        let Some(base) = Self::get(conn, id)? else {
            return Ok(None);
        };
        let skills = Self::get_skills(conn, id)?;
        Ok(Some(CertificationWithSkills { base, skills }))
    }

    /// Every certification with its linked skills, sorted by `short_name`.
    /// One `get_skills` query per row; certification lists are small.
    pub fn list_with_skills(conn: &Connection) -> Result<Vec<CertificationWithSkills>, ForgeError> {
        Self::list_all(conn)?
            .into_iter()
            .map(|base| {
                let skills = Self::get_skills(conn, &base.id)?;
                Ok(CertificationWithSkills { base, skills })
            })
            .collect()
    }

    // ── Update ───────────────────────────────────────────────────────

    /// Partially update a certification.
    pub fn update(
        conn: &Connection,
        id: &str,
        input: &UpdateCertification,
    ) -> Result<Certification, ForgeError> {
        // Validate first, as TS does; then 404.
        let short_name = input
            .short_name
            .as_deref()
            .map(|v| non_empty(v, "short_name", "short_name must not be empty"))
            .transpose()?;
        let long_name = input
            .long_name
            .as_deref()
            .map(|v| non_empty(v, "long_name", "long_name must not be empty"))
            .transpose()?;

        Self::get(conn, id)?.ok_or_else(|| ForgeError::NotFound {
            entity_type: "certification".into(),
            id: id.into(),
        })?;

        let mut sets = Vec::new();
        let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(v) = short_name {
            sets.push(format!("short_name = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.to_string()));
        }
        if let Some(v) = long_name {
            sets.push(format!("long_name = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.to_string()));
        }
        if let Some(ref v) = input.cert_id {
            sets.push(format!("cert_id = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = input.issuer_id {
            sets.push(format!("issuer_id = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = input.date_earned {
            sets.push(format!("date_earned = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = input.expiry_date {
            sets.push(format!("expiry_date = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = input.credential_id {
            sets.push(format!("credential_id = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = input.credential_url {
            sets.push(format!("credential_url = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = input.credly_url {
            sets.push(format!("credly_url = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(v) = input.in_progress {
            sets.push(format!("in_progress = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v as i32));
        }

        if !sets.is_empty() {
            let now = now_iso();
            sets.push(format!("updated_at = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(now));

            let sql = format!(
                "UPDATE certifications SET {} WHERE id = ?{}",
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
            .ok_or_else(|| ForgeError::Internal("Certification updated but not found".into()))
    }

    // ── Delete ───────────────────────────────────────────────────────

    /// Delete a certification by ID.
    pub fn delete(conn: &Connection, id: &str) -> Result<(), ForgeError> {
        let deleted = conn.execute("DELETE FROM certifications WHERE id = ?1", params![id])?;
        if deleted == 0 {
            return Err(ForgeError::NotFound {
                entity_type: "certification".into(),
                id: id.into(),
            });
        }
        Ok(())
    }

    // ── Skills junction ─────────────────────────────────────────────

    /// Link a skill to a certification.
    pub fn add_skill(conn: &Connection, cert_id: &str, skill_id: &str) -> Result<(), ForgeError> {
        conn.execute(
            "INSERT OR IGNORE INTO certification_skills (certification_id, skill_id) VALUES (?1, ?2)",
            params![cert_id, skill_id],
        )?;
        Ok(())
    }

    /// Unlink a skill from a certification.
    pub fn remove_skill(
        conn: &Connection,
        cert_id: &str,
        skill_id: &str,
    ) -> Result<(), ForgeError> {
        conn.execute(
            "DELETE FROM certification_skills WHERE certification_id = ?1 AND skill_id = ?2",
            params![cert_id, skill_id],
        )?;
        Ok(())
    }

    /// Get all skills linked to a certification via the junction table.
    pub fn get_skills(conn: &Connection, cert_id: &str) -> Result<Vec<Skill>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT s.id, s.name, s.category
             FROM skills s
             INNER JOIN certification_skills cs ON cs.skill_id = s.id
             WHERE cs.certification_id = ?1
             ORDER BY s.name ASC",
        )?;
        let skills: Vec<Skill> = stmt
            .query_map(params![cert_id], |row| {
                Ok(Skill {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    category: row
                        .get::<_, String>(2)?
                        .parse()
                        .unwrap_or(SkillCategory::Other),
                })
            })?
            .collect::<Result<_, _>>()?;
        Ok(skills)
    }

    // ── Row mapping ──────────────────────────────────────────────────

    fn map_certification(row: &rusqlite::Row) -> rusqlite::Result<Certification> {
        Ok(Certification {
            id: row.get(0)?,
            short_name: row.get(1)?,
            long_name: row.get(2)?,
            cert_id: row.get(3)?,
            issuer_id: row.get(4)?,
            date_earned: row.get(5)?,
            expiry_date: row.get(6)?,
            credential_id: row.get(7)?,
            credential_url: row.get(8)?,
            credly_url: row.get(9)?,
            in_progress: row.get(10)?,
            created_at: row.get(11)?,
            updated_at: row.get(12)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::Forge;
    use forge_core::SkillCategory;

    fn setup() -> Forge {
        Forge::open_memory().unwrap()
    }

    fn create_skill(conn: &Connection, name: &str) -> Skill {
        let id = new_id();
        conn.execute(
            "INSERT INTO skills (id, name, category) VALUES (?1, ?2, ?3)",
            params![id, name, "tool"],
        )
        .unwrap();
        Skill {
            id,
            name: name.into(),
            category: SkillCategory::Tool,
        }
    }

    fn blank_input(short: &str, long: &str) -> CreateCertification {
        CreateCertification {
            short_name: short.into(),
            long_name: long.into(),
            cert_id: None,
            issuer_id: None,
            date_earned: None,
            expiry_date: None,
            credential_id: None,
            credential_url: None,
            credly_url: None,
            in_progress: None,
        }
    }

    #[test]
    fn create_rejects_blank_names() {
        let forge = setup();
        for (short, long, needle) in [
            ("", "X", "short_name"),
            ("   ", "X", "short_name"),
            ("X", "", "long_name"),
        ] {
            let err =
                CertificationStore::create(forge.conn(), &blank_input(short, long)).unwrap_err();
            assert!(
                matches!(&err, ForgeError::Validation { message, .. } if message.contains(needle)),
                "{err:?}"
            );
        }
        assert!(CertificationStore::list_all(forge.conn())
            .unwrap()
            .is_empty());
    }

    #[test]
    fn create_trims_names() {
        let forge = setup();
        let c =
            CertificationStore::create(forge.conn(), &blank_input("  CKA  ", " Long ")).unwrap();
        assert_eq!(c.short_name, "CKA");
        assert_eq!(c.long_name, "Long");
    }

    #[test]
    fn update_validates_and_trims_names() {
        let forge = setup();
        let c = CertificationStore::create(forge.conn(), &blank_input("CKA", "Long")).unwrap();

        let err = CertificationStore::update(
            forge.conn(),
            &c.id,
            &UpdateCertification {
                short_name: Some("   ".into()),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert!(
            matches!(&err, ForgeError::Validation { message, .. } if message.contains("short_name"))
        );

        let err = CertificationStore::update(
            forge.conn(),
            &c.id,
            &UpdateCertification {
                long_name: Some("".into()),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert!(
            matches!(&err, ForgeError::Validation { message, .. } if message.contains("long_name"))
        );

        let u = CertificationStore::update(
            forge.conn(),
            &c.id,
            &UpdateCertification {
                short_name: Some("  CKAD ".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(u.short_name, "CKAD");
    }

    #[test]
    fn update_validation_runs_before_not_found() {
        let forge = setup();
        let err = CertificationStore::update(
            forge.conn(),
            "missing",
            &UpdateCertification {
                short_name: Some("".into()),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert!(matches!(err, ForgeError::Validation { .. }));

        let err = CertificationStore::update(
            forge.conn(),
            "missing",
            &UpdateCertification {
                short_name: Some("ok".into()),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert!(matches!(err, ForgeError::NotFound { .. }));
    }

    #[test]
    fn create_and_get_certification() {
        let forge = setup();
        let input = CreateCertification {
            short_name: "CKA".into(),
            long_name: "Certified Kubernetes Administrator".into(),
            cert_id: Some("CKA-2024".into()),
            issuer_id: None,
            date_earned: Some("2024-01-15".into()),
            expiry_date: Some("2027-01-15".into()),
            credential_id: Some("LF-abc123".into()),
            credential_url: Some("https://training.linuxfoundation.org/cka".into()),
            credly_url: Some("https://credly.com/badges/abc".into()),
            in_progress: Some(false),
        };
        let cert = CertificationStore::create(forge.conn(), &input).unwrap();
        assert_eq!(cert.short_name, "CKA");
        assert_eq!(cert.long_name, "Certified Kubernetes Administrator");
        assert_eq!(cert.cert_id, Some("CKA-2024".into()));
        assert!(!cert.in_progress);

        let fetched = CertificationStore::get(forge.conn(), &cert.id)
            .unwrap()
            .unwrap();
        assert_eq!(fetched.id, cert.id);
        assert_eq!(fetched.short_name, "CKA");
    }

    #[test]
    fn get_returns_none_for_missing() {
        let forge = setup();
        let result = CertificationStore::get(forge.conn(), "nonexistent").unwrap();
        assert!(result.is_none());
    }

    fn minimal(short: &str) -> CreateCertification {
        CreateCertification {
            short_name: short.into(),
            long_name: format!("{short} long"),
            cert_id: None,
            issuer_id: None,
            date_earned: None,
            expiry_date: None,
            credential_id: None,
            credential_url: None,
            credly_url: None,
            in_progress: None,
        }
    }

    #[test]
    fn list_with_skills_sorts_by_short_name_and_hydrates() {
        let forge = setup();
        let mut ids = std::collections::HashMap::new();
        for name in ["PMP", "CISSP", "AWS"] {
            let c = CertificationStore::create(forge.conn(), &minimal(name)).unwrap();
            ids.insert(name, c.id);
        }
        let sk = create_skill(forge.conn(), "Cloud");
        CertificationStore::add_skill(forge.conn(), &ids["AWS"], &sk.id).unwrap();

        let all = CertificationStore::list_with_skills(forge.conn()).unwrap();
        let names: Vec<_> = all.iter().map(|c| c.base.short_name.as_str()).collect();
        assert_eq!(names, ["AWS", "CISSP", "PMP"]);
        assert_eq!(all[0].skills.len(), 1);
        assert_eq!(all[0].skills[0].name, "Cloud");
        assert!(all[1].skills.is_empty());
        assert!(all[2].skills.is_empty());
    }

    #[test]
    fn get_with_skills_hydrates_and_none_for_missing() {
        let forge = setup();
        assert!(
            CertificationStore::get_with_skills(forge.conn(), "nonexistent")
                .unwrap()
                .is_none()
        );
        let cert = CertificationStore::create(forge.conn(), &minimal("CKA")).unwrap();
        let s = create_skill(forge.conn(), "Kubernetes");
        CertificationStore::add_skill(forge.conn(), &cert.id, &s.id).unwrap();
        let got = CertificationStore::get_with_skills(forge.conn(), &cert.id)
            .unwrap()
            .unwrap();
        assert_eq!(got.base.id, cert.id);
        assert_eq!(got.skills.len(), 1);
    }

    #[test]
    fn list_all_is_not_capped_at_50() {
        let forge = setup();
        for i in 0..51 {
            CertificationStore::create(forge.conn(), &minimal(&format!("C{i:02}"))).unwrap();
        }
        assert_eq!(
            CertificationStore::list_all(forge.conn()).unwrap().len(),
            51
        );
    }

    #[test]
    fn update_certification() {
        let forge = setup();
        let cert = CertificationStore::create(
            forge.conn(),
            &CreateCertification {
                short_name: "CKA".into(),
                long_name: "Old Name".into(),
                cert_id: None,
                issuer_id: None,
                date_earned: None,
                expiry_date: None,
                credential_id: None,
                credential_url: None,
                credly_url: None,
                in_progress: None,
            },
        )
        .unwrap();

        let updated = CertificationStore::update(
            forge.conn(),
            &cert.id,
            &UpdateCertification {
                long_name: Some("Certified Kubernetes Administrator".into()),
                in_progress: Some(true),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(updated.long_name, "Certified Kubernetes Administrator");
        assert!(updated.in_progress);
        assert_eq!(updated.short_name, "CKA"); // unchanged
    }

    #[test]
    fn update_missing_returns_not_found() {
        let forge = setup();
        let result = CertificationStore::update(
            forge.conn(),
            "nonexistent",
            &UpdateCertification::default(),
        );
        assert!(matches!(result, Err(ForgeError::NotFound { .. })));
    }

    #[test]
    fn delete_certification() {
        let forge = setup();
        let cert = CertificationStore::create(
            forge.conn(),
            &CreateCertification {
                short_name: "CKA".into(),
                long_name: "CKA".into(),
                cert_id: None,
                issuer_id: None,
                date_earned: None,
                expiry_date: None,
                credential_id: None,
                credential_url: None,
                credly_url: None,
                in_progress: None,
            },
        )
        .unwrap();

        CertificationStore::delete(forge.conn(), &cert.id).unwrap();
        assert!(CertificationStore::get(forge.conn(), &cert.id)
            .unwrap()
            .is_none());
    }

    #[test]
    fn delete_missing_returns_not_found() {
        let forge = setup();
        let result = CertificationStore::delete(forge.conn(), "nonexistent");
        assert!(matches!(result, Err(ForgeError::NotFound { .. })));
    }

    #[test]
    fn add_skill_and_get_skills() {
        let forge = setup();
        let cert = CertificationStore::create(
            forge.conn(),
            &CreateCertification {
                short_name: "CKA".into(),
                long_name: "CKA".into(),
                cert_id: None,
                issuer_id: None,
                date_earned: None,
                expiry_date: None,
                credential_id: None,
                credential_url: None,
                credly_url: None,
                in_progress: None,
            },
        )
        .unwrap();

        let s1 = create_skill(forge.conn(), "Kubernetes");
        let s2 = create_skill(forge.conn(), "Docker");

        CertificationStore::add_skill(forge.conn(), &cert.id, &s1.id).unwrap();
        CertificationStore::add_skill(forge.conn(), &cert.id, &s2.id).unwrap();

        let skills = CertificationStore::get_skills(forge.conn(), &cert.id).unwrap();
        assert_eq!(skills.len(), 2);
        let names: Vec<&str> = skills.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"Docker"));
        assert!(names.contains(&"Kubernetes"));
    }

    #[test]
    fn add_skill_is_idempotent() {
        let forge = setup();
        let cert = CertificationStore::create(
            forge.conn(),
            &CreateCertification {
                short_name: "CKA".into(),
                long_name: "CKA".into(),
                cert_id: None,
                issuer_id: None,
                date_earned: None,
                expiry_date: None,
                credential_id: None,
                credential_url: None,
                credly_url: None,
                in_progress: None,
            },
        )
        .unwrap();
        let s1 = create_skill(forge.conn(), "Kubernetes");

        CertificationStore::add_skill(forge.conn(), &cert.id, &s1.id).unwrap();
        CertificationStore::add_skill(forge.conn(), &cert.id, &s1.id).unwrap(); // no error

        let skills = CertificationStore::get_skills(forge.conn(), &cert.id).unwrap();
        assert_eq!(skills.len(), 1);
    }

    #[test]
    fn remove_skill() {
        let forge = setup();
        let cert = CertificationStore::create(
            forge.conn(),
            &CreateCertification {
                short_name: "CKA".into(),
                long_name: "CKA".into(),
                cert_id: None,
                issuer_id: None,
                date_earned: None,
                expiry_date: None,
                credential_id: None,
                credential_url: None,
                credly_url: None,
                in_progress: None,
            },
        )
        .unwrap();
        let s1 = create_skill(forge.conn(), "Kubernetes");
        let s2 = create_skill(forge.conn(), "Docker");

        CertificationStore::add_skill(forge.conn(), &cert.id, &s1.id).unwrap();
        CertificationStore::add_skill(forge.conn(), &cert.id, &s2.id).unwrap();

        CertificationStore::remove_skill(forge.conn(), &cert.id, &s1.id).unwrap();

        let skills = CertificationStore::get_skills(forge.conn(), &cert.id).unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "Docker");
    }
}
