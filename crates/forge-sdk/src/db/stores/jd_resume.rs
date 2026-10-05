//! Repository for the `job_description_resumes` junction (shared migration 026).
//!
//! TS writes this SQL inline in its routes: `packages/core/src/routes/job-descriptions.ts:146-239`
//! (JD side) and `packages/core/src/routes/resumes.ts:343-364` (resume side). Shapes, column
//! sources and ordering match those queries field for field.

use rusqlite::{params, Connection, OptionalExtension};

use forge_core::{now_iso, ForgeError, JDLink, JobDescriptionStatus, ResumeLink, ResumeStatus};

/// Data access for JD <-> resume links.
pub struct JdResumeStore;

/// `ResumeLink` columns, in `map_resume_link` order.
const RESUME_LINK_SELECT: &str =
    "SELECT r.id, r.name, r.target_role, r.target_employer, r.archetype, r.status,
            jdr.created_at, r.created_at
     FROM job_description_resumes jdr
     JOIN resumes r ON r.id = jdr.resume_id";

/// `JDLink` columns, in `map_jd_link` order.
const JD_LINK_SELECT: &str =
    "SELECT jd.id, jd.title, o.name, jd.status, jd.location, jd.salary_range,
            jdr.created_at, jd.created_at
     FROM job_description_resumes jdr
     JOIN job_descriptions jd ON jd.id = jdr.job_description_id
     LEFT JOIN organizations o ON o.id = jd.organization_id";

impl JdResumeStore {
    // ── Read ─────────────────────────────────────────────────────────

    /// Resumes linked to a JD, most recently linked first. An unknown JD lists as `[]`;
    /// the route checks existence.
    pub fn list_by_jd(conn: &Connection, jd_id: &str) -> Result<Vec<ResumeLink>, ForgeError> {
        let mut stmt = conn.prepare(&format!(
            "{RESUME_LINK_SELECT}
             WHERE jdr.job_description_id = ?1
             ORDER BY jdr.created_at DESC, jdr.rowid DESC"
        ))?;
        let rows = stmt
            .query_map(params![jd_id], Self::map_resume_link)?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    /// One link in `ResumeLink` shape: the `data` of the link response.
    pub fn get_link(
        conn: &Connection,
        jd_id: &str,
        resume_id: &str,
    ) -> Result<Option<ResumeLink>, ForgeError> {
        let mut stmt = conn.prepare(&format!(
            "{RESUME_LINK_SELECT} WHERE jdr.job_description_id = ?1 AND jdr.resume_id = ?2"
        ))?;
        Ok(stmt
            .query_row(params![jd_id, resume_id], Self::map_resume_link)
            .optional()?)
    }

    /// JDs linked to a resume, most recently linked first. `organization_name` is
    /// `None` when the JD has no organization (LEFT JOIN).
    pub fn list_by_resume(conn: &Connection, resume_id: &str) -> Result<Vec<JDLink>, ForgeError> {
        let mut stmt = conn.prepare(&format!(
            "{JD_LINK_SELECT}
             WHERE jdr.resume_id = ?1
             ORDER BY jdr.created_at DESC, jdr.rowid DESC"
        ))?;
        let rows = stmt
            .query_map(params![resume_id], Self::map_jd_link)?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    // ── Write ────────────────────────────────────────────────────────

    /// Link a resume to a JD. Idempotent: `true` when a row was inserted, `false` when
    /// the link already existed (its `created_at` is kept). The caller checks that both
    /// rows exist; a missing one is a foreign-key error, which OR IGNORE does not swallow.
    pub fn link(conn: &Connection, jd_id: &str, resume_id: &str) -> Result<bool, ForgeError> {
        let changed = conn.execute(
            "INSERT OR IGNORE INTO job_description_resumes (job_description_id, resume_id, created_at)
             VALUES (?1, ?2, ?3)",
            params![jd_id, resume_id, now_iso()],
        )?;
        Ok(changed > 0)
    }

    /// Remove one link. Idempotent: returns whether a row existed.
    pub fn unlink(conn: &Connection, jd_id: &str, resume_id: &str) -> Result<bool, ForgeError> {
        let removed = conn.execute(
            "DELETE FROM job_description_resumes WHERE job_description_id = ?1 AND resume_id = ?2",
            params![jd_id, resume_id],
        )?;
        Ok(removed > 0)
    }

    // ── Helpers ──────────────────────────────────────────────────────

    fn map_resume_link(row: &rusqlite::Row) -> rusqlite::Result<ResumeLink> {
        Ok(ResumeLink {
            resume_id: row.get(0)?,
            resume_name: row.get(1)?,
            target_role: row.get(2)?,
            target_employer: row.get(3)?,
            archetype: row.get(4)?,
            status: row
                .get::<_, String>(5)?
                .parse()
                .unwrap_or(ResumeStatus::Draft),
            created_at: row.get(6)?,
            resume_created_at: row.get(7)?,
        })
    }

    fn map_jd_link(row: &rusqlite::Row) -> rusqlite::Result<JDLink> {
        Ok(JDLink {
            job_description_id: row.get(0)?,
            title: row.get(1)?,
            organization_name: row.get(2)?,
            status: row
                .get::<_, String>(3)?
                .parse()
                .unwrap_or(JobDescriptionStatus::Discovered),
            location: row.get(4)?,
            salary_range: row.get(5)?,
            created_at: row.get(6)?,
            jd_created_at: row.get(7)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{JdStore, OrganizationStore, ResumeStore};
    use crate::forge::Forge;
    use forge_core::{CreateJobDescription, CreateOrganizationInput, CreateResume};
    use serde_json::json;

    fn resume(f: &Forge, name: &str) -> String {
        ResumeStore::create(
            f.conn(),
            &CreateResume {
                name: name.into(),
                target_role: "Security Engineer".into(),
                target_employer: "Acme".into(),
                archetype: "devsecops".into(),
                summary_id: None,
            },
        )
        .unwrap()
        .id
    }

    fn jd(f: &Forge, title: &str, org_id: Option<&str>) -> String {
        let input: CreateJobDescription = serde_json::from_value(json!({
            "title": title, "raw_text": "", "organization_id": org_id,
            "location": "Remote", "salary_range": "$150k-$200k",
        }))
        .unwrap();
        JdStore::create(f.conn(), &input).unwrap().id
    }

    fn org(f: &Forge, name: &str) -> String {
        let input: CreateOrganizationInput =
            serde_json::from_value(json!({ "name": name })).unwrap();
        OrganizationStore::create(f.conn(), &input).unwrap().id
    }

    fn link_at(f: &Forge, jd_id: &str, resume_id: &str, at: &str) {
        f.conn()
            .execute(
                "INSERT INTO job_description_resumes (job_description_id, resume_id, created_at)
                 VALUES (?1, ?2, ?3)",
                params![jd_id, resume_id, at],
            )
            .unwrap();
    }

    #[test]
    fn link_is_idempotent_and_keeps_first_created_at() {
        let f = Forge::open_memory().unwrap();
        let (j, r) = (jd(&f, "SRE", None), resume(&f, "A"));
        assert!(JdResumeStore::link(f.conn(), &j, &r).unwrap());
        let first = JdResumeStore::get_link(f.conn(), &j, &r).unwrap().unwrap();
        assert!(!JdResumeStore::link(f.conn(), &j, &r).unwrap());
        let again = JdResumeStore::get_link(f.conn(), &j, &r).unwrap().unwrap();
        assert_eq!(again.created_at, first.created_at);
        assert_eq!(JdResumeStore::list_by_jd(f.conn(), &j).unwrap().len(), 1);
    }

    #[test]
    fn list_by_jd_returns_resume_link_fields_newest_first() {
        let f = Forge::open_memory().unwrap();
        let j = jd(&f, "SRE", None);
        let (r1, r2) = (resume(&f, "First"), resume(&f, "Second"));
        link_at(&f, &j, &r1, "2026-01-01T00:00:00Z");
        link_at(&f, &j, &r2, "2026-01-02T00:00:00Z");

        let rows = JdResumeStore::list_by_jd(f.conn(), &j).unwrap();
        let names: Vec<_> = rows.iter().map(|l| l.resume_name.as_str()).collect();
        assert_eq!(names, ["Second", "First"]);
        assert_eq!(rows[0].resume_id, r2);
        assert_eq!(rows[0].created_at, "2026-01-02T00:00:00Z"); // link time, not the resume's
        assert_ne!(rows[0].resume_created_at, rows[0].created_at);
        assert_eq!(rows[0].status, ResumeStatus::Draft);
        assert_eq!(
            (
                rows[0].target_role.as_str(),
                rows[0].target_employer.as_str(),
                rows[0].archetype.as_str()
            ),
            ("Security Engineer", "Acme", "devsecops")
        );
        assert!(JdResumeStore::list_by_jd(f.conn(), "unknown")
            .unwrap()
            .is_empty());
    }

    #[test]
    fn list_by_resume_left_joins_organization() {
        let f = Forge::open_memory().unwrap();
        let o = org(&f, "Anthropic");
        let (with_org, no_org) = (
            jd(&f, "Security Engineer", Some(&o)),
            jd(&f, "No Org JD", None),
        );
        let r = resume(&f, "A");
        link_at(&f, &with_org, &r, "2026-01-02T00:00:00Z");
        link_at(&f, &no_org, &r, "2026-01-01T00:00:00Z");

        let rows = JdResumeStore::list_by_resume(f.conn(), &r).unwrap();
        assert_eq!(rows[0].job_description_id, with_org);
        assert_eq!(rows[0].organization_name.as_deref(), Some("Anthropic"));
        assert_eq!(rows[0].status, JobDescriptionStatus::Discovered);
        assert_eq!(rows[0].location.as_deref(), Some("Remote"));
        assert_eq!(rows[0].salary_range.as_deref(), Some("$150k-$200k"));
        assert_eq!(rows[1].organization_name, None);
        // created_at is the link time; jd_created_at is the JD's own.
        assert_eq!(rows[0].created_at, "2026-01-02T00:00:00Z");
        let jd_row = JdStore::get(f.conn(), &with_org).unwrap().unwrap();
        assert_eq!(rows[0].jd_created_at, jd_row.created_at);
        assert_ne!(rows[0].jd_created_at, rows[0].created_at);
    }

    #[test]
    fn same_second_links_list_newest_insert_first() {
        let f = Forge::open_memory().unwrap();
        let at = "2026-01-01T00:00:00Z";
        let j = jd(&f, "SRE", None);
        let (r1, r2) = (resume(&f, "One"), resume(&f, "Two"));
        link_at(&f, &j, &r1, at);
        link_at(&f, &j, &r2, at);
        let ids: Vec<_> = JdResumeStore::list_by_jd(f.conn(), &j)
            .unwrap()
            .into_iter()
            .map(|l| l.resume_id)
            .collect();
        assert_eq!(ids, [r2.clone(), r1.clone()]);

        let (j1, j2) = (jd(&f, "A", None), jd(&f, "B", None));
        let r = resume(&f, "Three");
        link_at(&f, &j1, &r, at);
        link_at(&f, &j2, &r, at);
        let ids: Vec<_> = JdResumeStore::list_by_resume(f.conn(), &r)
            .unwrap()
            .into_iter()
            .map(|l| l.job_description_id)
            .collect();
        assert_eq!(ids, [j2, j1]);
    }

    #[test]
    fn unlink_reports_removal_and_leaves_everything_else() {
        let f = Forge::open_memory().unwrap();
        let j = jd(&f, "SRE", None);
        let (r1, r2) = (resume(&f, "One"), resume(&f, "Two"));
        JdResumeStore::link(f.conn(), &j, &r1).unwrap();
        JdResumeStore::link(f.conn(), &j, &r2).unwrap();

        assert!(JdResumeStore::unlink(f.conn(), &j, &r1).unwrap());
        assert!(!JdResumeStore::unlink(f.conn(), &j, &r1).unwrap());
        assert!(!JdResumeStore::unlink(f.conn(), "nope", "nope").unwrap());

        assert!(JdStore::get(f.conn(), &j).unwrap().is_some());
        assert!(ResumeStore::get(f.conn(), &r1).unwrap().is_some());
        let left = JdResumeStore::list_by_jd(f.conn(), &j).unwrap();
        assert_eq!(left.len(), 1);
        assert_eq!(left[0].resume_id, r2);
    }

    #[test]
    fn links_cascade_on_jd_and_resume_delete() {
        let f = Forge::open_memory().unwrap();
        let (j1, j2) = (jd(&f, "A", None), jd(&f, "B", None));
        let (r1, r2) = (resume(&f, "One"), resume(&f, "Two"));
        JdResumeStore::link(f.conn(), &j1, &r1).unwrap();
        JdResumeStore::link(f.conn(), &j1, &r2).unwrap();
        JdResumeStore::link(f.conn(), &j2, &r1).unwrap();

        JdStore::delete(f.conn(), &j1).unwrap();
        assert!(JdResumeStore::list_by_jd(f.conn(), &j1).unwrap().is_empty());
        assert!(ResumeStore::get(f.conn(), &r1).unwrap().is_some());
        assert_eq!(
            JdResumeStore::list_by_resume(f.conn(), &r1).unwrap().len(),
            1
        );

        ResumeStore::delete(f.conn(), &r1).unwrap();
        assert!(JdResumeStore::list_by_jd(f.conn(), &j2).unwrap().is_empty());
        assert!(JdStore::get(f.conn(), &j2).unwrap().is_some());
    }

    #[test]
    fn link_with_missing_parent_is_a_database_error() {
        let f = Forge::open_memory().unwrap();
        assert!(JdResumeStore::link(f.conn(), "no-jd", "no-resume").is_err());
    }
}
