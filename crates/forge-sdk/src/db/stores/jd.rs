//! Repository for job description persistence.
//!
//! Provides CRUD operations and query methods for the `job_descriptions` table.

use rusqlite::{params, Connection, OptionalExtension};

use forge_core::{
    new_id, now_iso, CreateJobDescription, ForgeError, JobDescription, JobDescriptionFilter,
    JobDescriptionStatus, JobDescriptionWithOrg, Pagination, SkillRow, UpdateJobDescription,
};

use super::skill::SkillStore;

/// Data-access repository for job descriptions.
pub struct JdStore;

impl JdStore {
    // ── Create ───────────────────────────────────────────────────────

    /// Insert a new job description row.
    pub fn create(
        conn: &Connection,
        input: &CreateJobDescription,
    ) -> Result<JobDescription, ForgeError> {
        require_non_blank(&input.title, "title", TITLE_EMPTY)?;
        require_non_blank(&input.raw_text, "raw_text", RAW_TEXT_EMPTY)?;
        let id = new_id();
        let now = now_iso();
        let status = input.status.unwrap_or(JobDescriptionStatus::Discovered);

        conn.execute(
            "INSERT INTO job_descriptions (id, organization_id, title, url, raw_text, status,
                salary_range, salary_min, salary_max, location, parsed_sections,
                work_posture, parsed_locations, salary_period, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?15)",
            params![
                id,
                input.organization_id,
                input.title,
                input.url,
                input.raw_text,
                status.as_ref(),
                input.salary_range,
                input.salary_min,
                input.salary_max,
                input.location,
                input.parsed_sections,
                input.work_posture,
                input.parsed_locations,
                input.salary_period,
                now,
            ],
        )?;

        Self::get(conn, &id)?
            .ok_or_else(|| ForgeError::Internal("Job description created but not found".into()))
    }

    // ── Read ─────────────────────────────────────────────────────────

    /// Fetch a single job description by primary key.
    pub fn get(conn: &Connection, id: &str) -> Result<Option<JobDescription>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT id, organization_id, title, url, raw_text, status,
                    salary_range, salary_min, salary_max, location, parsed_sections,
                    work_posture, parsed_locations, salary_period, created_at, updated_at
             FROM job_descriptions WHERE id = ?1",
        )?;

        let result = stmt.query_row(params![id], Self::map_jd).optional()?;
        Ok(result)
    }

    /// Fetch a job description with its hydrated organization name.
    pub fn get_with_org(
        conn: &Connection,
        id: &str,
    ) -> Result<Option<JobDescriptionWithOrg>, ForgeError> {
        let jd = match Self::get(conn, id)? {
            Some(jd) => jd,
            None => return Ok(None),
        };
        let org_name = Self::lookup_org_name(conn, jd.organization_id.as_deref())?;
        Ok(Some(JobDescriptionWithOrg {
            base: jd,
            organization_name: org_name,
        }))
    }

    /// List job descriptions with optional filtering and pagination.
    pub fn list(
        conn: &Connection,
        filter: &JobDescriptionFilter,
        offset: i64,
        limit: i64,
    ) -> Result<(Vec<JobDescription>, Pagination), ForgeError> {
        let mut conditions = Vec::new();
        let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(ref status) = filter.status {
            conditions.push(format!("status = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(status.to_string()));
        }
        if let Some(ref org_id) = filter.organization_id {
            conditions.push(format!("organization_id = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(org_id.clone()));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        // Count
        let count_sql = format!("SELECT COUNT(*) FROM job_descriptions {where_clause}");
        let total: i64 = conn.query_row(
            &count_sql,
            rusqlite::params_from_iter(bind_values.iter().map(|b| b.as_ref())),
            |row| row.get(0),
        )?;

        // Fetch page
        let query_sql = format!(
            "SELECT id, organization_id, title, url, raw_text, status,
                    salary_range, salary_min, salary_max, location, parsed_sections,
                    work_posture, parsed_locations, salary_period, created_at, updated_at
             FROM job_descriptions {where_clause}
             ORDER BY updated_at DESC
             LIMIT ?{} OFFSET ?{}",
            bind_values.len() + 1,
            bind_values.len() + 2
        );
        bind_values.push(Box::new(limit));
        bind_values.push(Box::new(offset));

        let mut stmt = conn.prepare(&query_sql)?;
        let rows: Vec<JobDescription> = stmt
            .query_map(
                rusqlite::params_from_iter(bind_values.iter().map(|b| b.as_ref())),
                Self::map_jd,
            )?
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

    /// List job descriptions with hydrated organization names.
    pub fn list_with_org(
        conn: &Connection,
        filter: &JobDescriptionFilter,
        offset: i64,
        limit: i64,
    ) -> Result<(Vec<JobDescriptionWithOrg>, Pagination), ForgeError> {
        let (rows, pagination) = Self::list(conn, filter, offset, limit)?;
        let mut hydrated = Vec::with_capacity(rows.len());
        for jd in rows {
            let org_name = Self::lookup_org_name(conn, jd.organization_id.as_deref())?;
            hydrated.push(JobDescriptionWithOrg {
                base: jd,
                organization_name: org_name,
            });
        }
        Ok((hydrated, pagination))
    }

    // ── Update ───────────────────────────────────────────────────────

    /// Apply a partial update to an existing job description.
    pub fn update(
        conn: &Connection,
        id: &str,
        input: &UpdateJobDescription,
    ) -> Result<JobDescription, ForgeError> {
        if let Some(ref v) = input.title {
            require_non_blank(v, "title", TITLE_EMPTY)?;
        }
        if let Some(ref v) = input.raw_text {
            require_non_blank(v, "raw_text", RAW_TEXT_EMPTY)?;
        }
        // Verify existence
        Self::get(conn, id)?.ok_or_else(|| ForgeError::NotFound {
            entity_type: "job_description".into(),
            id: id.into(),
        })?;

        let mut sets = Vec::new();
        let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(ref v) = input.title {
            sets.push(format!("title = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = input.organization_id {
            sets.push(format!("organization_id = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = input.url {
            sets.push(format!("url = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = input.raw_text {
            sets.push(format!("raw_text = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = input.status {
            sets.push(format!("status = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.to_string()));
        }
        if let Some(ref v) = input.salary_range {
            sets.push(format!("salary_range = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = input.salary_min {
            sets.push(format!("salary_min = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(*v));
        }
        if let Some(ref v) = input.salary_max {
            sets.push(format!("salary_max = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(*v));
        }
        if let Some(ref v) = input.location {
            sets.push(format!("location = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = input.parsed_sections {
            sets.push(format!("parsed_sections = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = input.work_posture {
            sets.push(format!("work_posture = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = input.parsed_locations {
            sets.push(format!("parsed_locations = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = input.salary_period {
            sets.push(format!("salary_period = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }

        if !sets.is_empty() {
            let now = now_iso();
            sets.push(format!("updated_at = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(now));

            let sql = format!(
                "UPDATE job_descriptions SET {} WHERE id = ?{}",
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
            .ok_or_else(|| ForgeError::Internal("Job description updated but not found".into()))
    }

    // ── Delete ───────────────────────────────────────────────────────

    /// Delete a job description by primary key.
    pub fn delete(conn: &Connection, id: &str) -> Result<(), ForgeError> {
        let deleted = conn.execute("DELETE FROM job_descriptions WHERE id = ?1", params![id])?;
        if deleted == 0 {
            return Err(ForgeError::NotFound {
                entity_type: "job_description".into(),
                id: id.into(),
            });
        }
        Ok(())
    }

    // ── Lookup ───────────────────────────────────────────────────────

    /// Look up a job description by its URL field (exact match).
    pub fn find_by_url(conn: &Connection, url: &str) -> Result<Option<JobDescription>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT id, organization_id, title, url, raw_text, status,
                    salary_range, salary_min, salary_max, location, parsed_sections,
                    work_posture, parsed_locations, salary_period, created_at, updated_at
             FROM job_descriptions WHERE url = ?1 LIMIT 1",
        )?;

        let result = stmt.query_row(params![url], Self::map_jd).optional()?;
        Ok(result)
    }

    // ── Skill links ──────────────────────────────────────────────────

    /// Skills linked to a job description, ordered by name (`BINARY`, as TS). Doesn't check
    /// that the JD exists: an unknown id has no links, so `[]`.
    pub fn list_skills(conn: &Connection, jd_id: &str) -> Result<Vec<SkillRow>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT s.id, s.name, s.category, s.created_at
             FROM skills s
             JOIN job_description_skills jds ON jds.skill_id = s.id
             WHERE jds.job_description_id = ?1
             ORDER BY s.name ASC",
        )?;
        let rows = stmt
            .query_map(params![jd_id], SkillStore::map_skill_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Unlink a skill. Silent when there is no such link, including for an unknown JD or skill,
    /// as TS (`job-descriptions.ts:136-142`). Never deletes the `skills` row.
    pub fn remove_skill(conn: &Connection, jd_id: &str, skill_id: &str) -> Result<(), ForgeError> {
        conn.execute(
            "DELETE FROM job_description_skills WHERE job_description_id = ?1 AND skill_id = ?2",
            params![jd_id, skill_id],
        )?;
        Ok(())
    }

    /// Link an existing skill. Idempotent: linking twice is a no-op. The JD and the skill are
    /// checked first, because a raw FK failure would surface as `Database` (500), not TS's 404.
    pub fn add_skill(
        conn: &Connection,
        jd_id: &str,
        skill_id: &str,
    ) -> Result<SkillRow, ForgeError> {
        Self::require_jd(conn, jd_id)?;
        let skill = SkillStore::get_row(conn, skill_id)?.ok_or_else(|| ForgeError::NotFound {
            entity_type: "Skill".into(),
            id: skill_id.into(),
        })?;
        conn.execute(
            "INSERT OR IGNORE INTO job_description_skills (job_description_id, skill_id)
             VALUES (?1, ?2)",
            params![jd_id, skill_id],
        )?;
        Ok(skill)
    }

    /// Link a skill by name, creating it if needed ([`SkillStore::get_or_create_for_link`], the
    /// TS name and category rules). The JD check, the resolve and the link run in one
    /// transaction, so an unknown JD gives `NotFound` and creates no skill (TS creates one and
    /// answers 500).
    pub fn add_skill_by_name(
        conn: &Connection,
        jd_id: &str,
        name: &str,
        category: Option<&str>,
    ) -> Result<SkillRow, ForgeError> {
        let tx = conn.unchecked_transaction()?;
        Self::require_jd(&tx, jd_id)?;
        let skill = SkillStore::get_or_create_for_link(&tx, name, category)?;
        tx.execute(
            "INSERT OR IGNORE INTO job_description_skills (job_description_id, skill_id)
             VALUES (?1, ?2)",
            params![jd_id, skill.base.id],
        )?;
        tx.commit()?;
        Ok(skill)
    }

    // Entity type isn't "Route ...": the wasm dispatcher rewrites a "Route not found" 404 to 501.
    fn require_jd(conn: &Connection, jd_id: &str) -> Result<(), ForgeError> {
        match Self::get(conn, jd_id)? {
            Some(_) => Ok(()),
            None => Err(ForgeError::NotFound {
                entity_type: "Job description".into(),
                id: jd_id.into(),
            }),
        }
    }

    // ── Helpers ──────────────────────────────────────────────────────

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

    fn map_jd(row: &rusqlite::Row) -> rusqlite::Result<JobDescription> {
        Ok(JobDescription {
            id: row.get(0)?,
            organization_id: row.get(1)?,
            title: row.get(2)?,
            url: row.get(3)?,
            raw_text: row.get(4)?,
            status: row
                .get::<_, String>(5)?
                .parse()
                .unwrap_or(JobDescriptionStatus::Discovered),
            salary_range: row.get(6)?,
            salary_min: row.get(7)?,
            salary_max: row.get(8)?,
            location: row.get(9)?,
            parsed_sections: row.get(10)?,
            work_posture: row.get(11)?,
            parsed_locations: row.get(12)?,
            salary_period: row.get(13)?,
            created_at: row.get(14)?,
            updated_at: row.get(15)?,
        })
    }
}

/// Reject blank title / raw_text, mirroring the TS `JobDescriptionService`.
fn require_non_blank(value: &str, field: &str, message: &str) -> Result<(), ForgeError> {
    if value.trim().is_empty() {
        return Err(ForgeError::Validation {
            message: message.into(),
            field: Some(field.into()),
        });
    }
    Ok(())
}

const TITLE_EMPTY: &str = "Title must not be empty";
const RAW_TEXT_EMPTY: &str = "Job description text (raw_text) must not be empty";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::Forge;
    use forge_core::SkillCategory;

    fn setup() -> Forge {
        Forge::open_memory().unwrap()
    }

    fn sample_input() -> CreateJobDescription {
        CreateJobDescription {
            title: "Senior Rust Engineer".into(),
            organization_id: None,
            url: Some("https://example.com/jobs/123".into()),
            raw_text: "We are looking for a senior Rust engineer...".into(),
            status: None,
            salary_range: Some("150k-200k".into()),
            salary_min: Some(150_000.0),
            salary_max: Some(200_000.0),
            location: Some("Remote".into()),
            parsed_sections: None,
            work_posture: Some("remote".into()),
            parsed_locations: None,
            salary_period: Some("annual".into()),
        }
    }

    /// Link through raw SQL so these tests don't depend on `add_skill`.
    fn link(forge: &Forge, jd_id: &str, skill_id: &str) {
        forge
            .conn()
            .execute(
                "INSERT INTO job_description_skills (job_description_id, skill_id) VALUES (?1, ?2)",
                params![jd_id, skill_id],
            )
            .unwrap();
    }

    fn assert_validation_field<T: std::fmt::Debug>(r: Result<T, ForgeError>, expected: &str) {
        match r {
            Err(ForgeError::Validation { field, .. }) => {
                assert_eq!(field.as_deref(), Some(expected))
            }
            other => panic!("expected validation error on {expected}, got {other:?}"),
        }
    }

    #[test]
    fn create_rejects_blank_title_and_raw_text() {
        let forge = setup();
        let mut input = sample_input();
        input.title = "   ".into();
        assert_validation_field(JdStore::create(forge.conn(), &input), "title");

        let mut input = sample_input();
        input.raw_text = " \n".into();
        assert_validation_field(JdStore::create(forge.conn(), &input), "raw_text");
    }

    #[test]
    fn update_rejects_blank_title_and_raw_text() {
        let forge = setup();
        let jd = JdStore::create(forge.conn(), &sample_input()).unwrap();
        let blank_title = UpdateJobDescription {
            title: Some("".into()),
            ..Default::default()
        };
        assert_validation_field(JdStore::update(forge.conn(), &jd.id, &blank_title), "title");
        let blank_text = UpdateJobDescription {
            raw_text: Some("  ".into()),
            ..Default::default()
        };
        assert_validation_field(
            JdStore::update(forge.conn(), &jd.id, &blank_text),
            "raw_text",
        );
    }

    #[test]
    fn list_skills_orders_by_name_and_carries_created_at() {
        let forge = setup();
        let jd = JdStore::create(forge.conn(), &sample_input()).unwrap();
        let tf = SkillStore::create(forge.conn(), "Terraform", Some(SkillCategory::Tool)).unwrap();
        let k8s =
            SkillStore::create(forge.conn(), "Kubernetes", Some(SkillCategory::Platform)).unwrap();
        link(&forge, &jd.id, &tf.id);
        link(&forge, &jd.id, &k8s.id);

        let rows = JdStore::list_skills(forge.conn(), &jd.id).unwrap();
        let names: Vec<_> = rows.iter().map(|r| r.base.name.as_str()).collect();
        assert_eq!(names, ["Kubernetes", "Terraform"]);
        assert_eq!(rows[0].base.category, SkillCategory::Platform);
        assert!(!rows[0].created_at.is_empty());
    }

    #[test]
    fn list_skills_is_empty_for_an_unlinked_or_unknown_jd() {
        let forge = setup();
        let jd = JdStore::create(forge.conn(), &sample_input()).unwrap();
        assert!(JdStore::list_skills(forge.conn(), &jd.id)
            .unwrap()
            .is_empty());
        assert!(JdStore::list_skills(forge.conn(), "nope")
            .unwrap()
            .is_empty());
    }

    #[test]
    fn remove_skill_unlinks_only_that_pair() {
        let forge = setup();
        let a = JdStore::create(forge.conn(), &sample_input()).unwrap();
        let b = JdStore::create(
            forge.conn(),
            &CreateJobDescription {
                title: "Other".into(),
                ..sample_input()
            },
        )
        .unwrap();
        let tf = SkillStore::create(forge.conn(), "Terraform", None).unwrap();
        link(&forge, &a.id, &tf.id);
        link(&forge, &b.id, &tf.id);

        JdStore::remove_skill(forge.conn(), &a.id, &tf.id).unwrap();
        assert!(JdStore::list_skills(forge.conn(), &a.id)
            .unwrap()
            .is_empty());
        assert_eq!(JdStore::list_skills(forge.conn(), &b.id).unwrap().len(), 1);
        assert!(SkillStore::get(forge.conn(), &tf.id).unwrap().is_some());
    }

    #[test]
    fn remove_skill_without_a_link_is_ok() {
        let forge = setup();
        let jd = JdStore::create(forge.conn(), &sample_input()).unwrap();
        JdStore::remove_skill(forge.conn(), &jd.id, "nope").unwrap();
        JdStore::remove_skill(forge.conn(), "nope", "nope").unwrap();
    }

    #[test]
    fn add_skill_is_idempotent_and_not_found_for_unknown_ids() {
        let forge = setup();
        let jd = JdStore::create(forge.conn(), &sample_input()).unwrap();
        let py = SkillStore::create(forge.conn(), "Python", Some(SkillCategory::Language)).unwrap();
        JdStore::add_skill(forge.conn(), &jd.id, &py.id).unwrap();
        JdStore::add_skill(forge.conn(), &jd.id, &py.id).unwrap();
        assert_eq!(JdStore::list_skills(forge.conn(), &jd.id).unwrap().len(), 1);
        assert!(matches!(
            JdStore::add_skill(forge.conn(), "nope", &py.id),
            Err(ForgeError::NotFound { .. })
        ));
        assert!(matches!(
            JdStore::add_skill(forge.conn(), &jd.id, "nope"),
            Err(ForgeError::NotFound { .. })
        ));
    }

    #[test]
    fn add_skill_by_name_capitalises_reuses_and_applies_the_category_rule() {
        let forge = setup();
        let jd = JdStore::create(forge.conn(), &sample_input()).unwrap();
        let py = SkillStore::create(forge.conn(), "Python", Some(SkillCategory::Language)).unwrap();

        let reused =
            JdStore::add_skill_by_name(forge.conn(), &jd.id, "  python ", Some("tool")).unwrap();
        assert_eq!(reused.base.id, py.id);
        assert_eq!(reused.base.category, SkillCategory::Language);

        let tf =
            JdStore::add_skill_by_name(forge.conn(), &jd.id, "terraform", Some("tool")).unwrap();
        assert_eq!(
            (tf.base.name.as_str(), tf.base.category),
            ("Terraform", SkillCategory::Tool)
        );
        let safe = JdStore::add_skill_by_name(forge.conn(), &jd.id, "sAFe", Some("ai_ml")).unwrap();
        assert_eq!(
            (safe.base.name.as_str(), safe.base.category),
            ("SAFe", SkillCategory::Other)
        );

        let names: Vec<_> = JdStore::list_skills(forge.conn(), &jd.id)
            .unwrap()
            .into_iter()
            .map(|r| r.base.name)
            .collect();
        assert_eq!(names, ["Python", "SAFe", "Terraform"]);
    }

    #[test]
    fn add_skill_by_name_for_an_unknown_jd_creates_nothing() {
        let forge = setup();
        let err = JdStore::add_skill_by_name(forge.conn(), "nope", "Zebra Mesh", None).unwrap_err();
        assert!(matches!(err, ForgeError::NotFound { .. }));
        assert!(SkillStore::find_by_name(forge.conn(), "Zebra Mesh")
            .unwrap()
            .is_none());
    }

    #[test]
    fn add_skill_by_name_reuses_a_skill_with_a_non_ascii_first_letter() {
        // Relies on get_or_create_for_link capitalising before the lookup: SQLite's LOWER folds
        // ASCII only, so a raw "élan" lookup would miss "Élan" and `create` would 409.
        let forge = setup();
        let jd = JdStore::create(forge.conn(), &sample_input()).unwrap();
        let elan = SkillStore::create(forge.conn(), "Élan", None).unwrap();
        let row = JdStore::add_skill_by_name(forge.conn(), &jd.id, "élan", Some("tool")).unwrap();
        assert_eq!(row.base.id, elan.id);
    }

    #[test]
    fn create_and_get() {
        let forge = setup();
        let jd = JdStore::create(forge.conn(), &sample_input()).unwrap();
        assert_eq!(jd.title, "Senior Rust Engineer");
        assert_eq!(jd.status, JobDescriptionStatus::Discovered);
        assert_eq!(jd.url, Some("https://example.com/jobs/123".into()));

        let fetched = JdStore::get(forge.conn(), &jd.id).unwrap().unwrap();
        assert_eq!(fetched.id, jd.id);
        assert_eq!(fetched.title, jd.title);
    }

    #[test]
    fn get_returns_none_for_missing() {
        let forge = setup();
        let result = JdStore::get(forge.conn(), "nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn list_empty() {
        let forge = setup();
        let (rows, pagination) =
            JdStore::list(forge.conn(), &JobDescriptionFilter::default(), 0, 50).unwrap();
        assert!(rows.is_empty());
        assert_eq!(pagination.total, 0);
    }

    #[test]
    fn list_with_status_filter() {
        let forge = setup();
        JdStore::create(forge.conn(), &sample_input()).unwrap();
        JdStore::create(
            forge.conn(),
            &CreateJobDescription {
                title: "Python Dev".into(),
                raw_text: "Python job".into(),
                status: Some(JobDescriptionStatus::Applied),
                ..sample_input()
            },
        )
        .unwrap();

        let (rows, _) = JdStore::list(
            forge.conn(),
            &JobDescriptionFilter {
                status: Some(JobDescriptionStatus::Applied),
                ..Default::default()
            },
            0,
            50,
        )
        .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].title, "Python Dev");
    }

    #[test]
    fn update_title() {
        let forge = setup();
        let created = JdStore::create(forge.conn(), &sample_input()).unwrap();
        let updated = JdStore::update(
            forge.conn(),
            &created.id,
            &UpdateJobDescription {
                title: Some("Staff Rust Engineer".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(updated.title, "Staff Rust Engineer");
    }

    #[test]
    fn delete_jd() {
        let forge = setup();
        let created = JdStore::create(forge.conn(), &sample_input()).unwrap();
        JdStore::delete(forge.conn(), &created.id).unwrap();
        assert!(JdStore::get(forge.conn(), &created.id).unwrap().is_none());
    }

    #[test]
    fn delete_missing_returns_not_found() {
        let forge = setup();
        let result = JdStore::delete(forge.conn(), "nonexistent");
        assert!(matches!(result, Err(ForgeError::NotFound { .. })));
    }

    #[test]
    fn find_by_url() {
        let forge = setup();
        let created = JdStore::create(forge.conn(), &sample_input()).unwrap();
        let found = JdStore::find_by_url(forge.conn(), "https://example.com/jobs/123").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, created.id);

        let missing = JdStore::find_by_url(forge.conn(), "https://nope.com").unwrap();
        assert!(missing.is_none());
    }
}
