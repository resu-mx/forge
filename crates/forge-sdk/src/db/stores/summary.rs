//! Repository for summary persistence.
//!
//! Provides CRUD operations, skill junction management, linked resume queries,
//! and template toggling for the `summaries` and `summary_skills` tables.

use rusqlite::{params, Connection, OptionalExtension};

use forge_core::{
    new_id, now_iso, CreateSummary, ForgeError, Pagination, Resume, ResumeStatus, Skill,
    SkillCategory, SortDirection, Summary, SummaryFilter, SummarySort, SummarySortBy,
    UpdateSummary,
};

/// Data-access repository for summaries and the `summary_skills` junction.
pub struct SummaryStore;

impl SummaryStore {
    // ── Core CRUD ───────────────────────────────────────────────────

    /// Insert a new summary row.
    pub fn create(conn: &Connection, input: &CreateSummary) -> Result<Summary, ForgeError> {
        let id = new_id();
        let now = now_iso();

        conn.execute(
            "INSERT INTO summaries (id, title, role, description, is_template, industry_id, role_type_id, notes, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
            params![
                id,
                input.title,
                input.role,
                input.description,
                input.is_template.unwrap_or(0),
                input.industry_id,
                input.role_type_id,
                input.notes,
                now,
            ],
        )?;

        Self::get(conn, &id)?
            .ok_or_else(|| ForgeError::Internal("Summary created but not found".into()))
    }

    /// Fetch a single summary by primary key.
    pub fn get(conn: &Connection, id: &str) -> Result<Option<Summary>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT id, title, role, description, is_template, industry_id, role_type_id,
                    notes, created_at, updated_at
             FROM summaries WHERE id = ?1",
        )?;

        let row = stmt
            .query_row(params![id], Self::map_summary_without_count)
            .optional()?;
        match row {
            None => Ok(None),
            Some(mut summary) => {
                summary.linked_resume_count = Self::count_linked_resumes(conn, &summary.id)?;
                Ok(Some(summary))
            }
        }
    }

    /// List summaries with optional filtering, sorting, and pagination.
    pub fn list(
        conn: &Connection,
        filter: Option<&SummaryFilter>,
        sort: Option<&SummarySort>,
        offset: i64,
        limit: i64,
    ) -> Result<(Vec<Summary>, Pagination), ForgeError> {
        let sort_by = sort
            .and_then(|s| s.sort_by)
            .unwrap_or(SummarySortBy::UpdatedAt);
        let direction = sort.and_then(|s| s.direction).unwrap_or_else(|| {
            if sort_by == SummarySortBy::Title {
                SortDirection::Asc
            } else {
                SortDirection::Desc
            }
        });

        let mut conditions = Vec::new();
        let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(f) = filter {
            if let Some(is_tpl) = f.is_template {
                conditions.push(format!("s.is_template = ?{}", bind_values.len() + 1));
                bind_values.push(Box::new(is_tpl));
            }
            if let Some(ref iid) = f.industry_id {
                conditions.push(format!("s.industry_id = ?{}", bind_values.len() + 1));
                bind_values.push(Box::new(iid.clone()));
            }
            if let Some(ref rtid) = f.role_type_id {
                conditions.push(format!("s.role_type_id = ?{}", bind_values.len() + 1));
                bind_values.push(Box::new(rtid.clone()));
            }
            if let Some(ref sid) = f.skill_id {
                let param_idx = bind_values.len() + 1;
                conditions.push(format!(
                    "s.id IN (SELECT summary_id FROM summary_skills WHERE skill_id = ?{param_idx})"
                ));
                bind_values.push(Box::new(sid.clone()));
            }
            if let Some(ref search) = f.search {
                let param_idx = bind_values.len() + 1;
                conditions.push(format!(
                    "(s.title LIKE ?{param_idx} OR s.description LIKE ?{param_idx})"
                ));
                bind_values.push(Box::new(format!("%{search}%")));
            }
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let sort_col = match sort_by {
            SummarySortBy::Title => "s.title",
            SummarySortBy::CreatedAt => "s.created_at",
            SummarySortBy::UpdatedAt => "s.updated_at",
        };
        let dir = match direction {
            SortDirection::Asc => "ASC",
            SortDirection::Desc => "DESC",
        };

        // Count
        let count_sql = format!("SELECT COUNT(*) FROM summaries s {where_clause}");
        let total: i64 = conn.query_row(
            &count_sql,
            rusqlite::params_from_iter(bind_values.iter().map(|b| b.as_ref())),
            |row| row.get(0),
        )?;

        // Fetch page — templates float to top (is_template DESC first)
        let query_sql = format!(
            "SELECT s.id, s.title, s.role, s.description, s.is_template, s.industry_id,
                    s.role_type_id, s.notes, s.created_at, s.updated_at
             FROM summaries s {where_clause}
             ORDER BY s.is_template DESC, {sort_col} {dir}
             LIMIT ?{} OFFSET ?{}",
            bind_values.len() + 1,
            bind_values.len() + 2
        );
        bind_values.push(Box::new(limit));
        bind_values.push(Box::new(offset));

        let mut stmt = conn.prepare(&query_sql)?;
        let rows: Vec<Summary> = stmt
            .query_map(
                rusqlite::params_from_iter(bind_values.iter().map(|b| b.as_ref())),
                Self::map_summary_without_count,
            )?
            .collect::<Result<_, _>>()?;

        // Hydrate linked_resume_count
        let mut hydrated = Vec::with_capacity(rows.len());
        for mut summary in rows {
            summary.linked_resume_count = Self::count_linked_resumes(conn, &summary.id)?;
            hydrated.push(summary);
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

    /// Apply a partial update to an existing summary.
    pub fn update(
        conn: &Connection,
        id: &str,
        input: &UpdateSummary,
    ) -> Result<Summary, ForgeError> {
        Self::get(conn, id)?.ok_or_else(|| ForgeError::NotFound {
            entity_type: "summary".into(),
            id: id.into(),
        })?;

        let mut sets = Vec::new();
        let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(ref title) = input.title {
            sets.push(format!("title = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(title.clone()));
        }
        if let Some(ref role) = input.role {
            sets.push(format!("role = ?{}", bind_values.len() + 1));
            // Option<Option<String>>: Some(None) => set null, Some(Some(v)) => set v
            bind_values.push(Box::new(role.clone()));
        }
        if let Some(ref desc) = input.description {
            sets.push(format!("description = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(desc.clone()));
        }
        if let Some(is_tpl) = input.is_template {
            sets.push(format!("is_template = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(is_tpl));
        }
        if let Some(ref iid) = input.industry_id {
            sets.push(format!("industry_id = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(iid.clone()));
        }
        if let Some(ref rtid) = input.role_type_id {
            sets.push(format!("role_type_id = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(rtid.clone()));
        }
        if let Some(ref notes) = input.notes {
            sets.push(format!("notes = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(notes.clone()));
        }

        if !sets.is_empty() {
            let now = now_iso();
            sets.push(format!("updated_at = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(now));

            let sql = format!(
                "UPDATE summaries SET {} WHERE id = ?{}",
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
            .ok_or_else(|| ForgeError::Internal("Summary updated but not found".into()))
    }

    /// Delete a summary by primary key.
    pub fn delete(conn: &Connection, id: &str) -> Result<(), ForgeError> {
        let deleted = conn.execute("DELETE FROM summaries WHERE id = ?1", params![id])?;
        if deleted == 0 {
            return Err(ForgeError::NotFound {
                entity_type: "summary".into(),
                id: id.into(),
            });
        }
        Ok(())
    }

    // ── Template helpers ────────────────────────────────────────────

    /// Toggle the `is_template` flag on a summary (0 -> 1 or 1 -> 0).
    pub fn toggle_template(conn: &Connection, id: &str) -> Result<Summary, ForgeError> {
        let summary = Self::get(conn, id)?.ok_or_else(|| ForgeError::NotFound {
            entity_type: "summary".into(),
            id: id.into(),
        })?;

        let new_val = if summary.is_template == 1 { 0 } else { 1 };
        let now = now_iso();
        conn.execute(
            "UPDATE summaries SET is_template = ?1, updated_at = ?2 WHERE id = ?3",
            params![new_val, now, id],
        )?;

        Self::get(conn, id)?
            .ok_or_else(|| ForgeError::Internal("Summary toggled but not found".into()))
    }

    // ── Skill junction ──────────────────────────────────────────────

    /// Link a skill keyword to a summary (idempotent).
    pub fn add_skill(
        conn: &Connection,
        summary_id: &str,
        skill_id: &str,
    ) -> Result<(), ForgeError> {
        conn.execute(
            "INSERT OR IGNORE INTO summary_skills (summary_id, skill_id) VALUES (?1, ?2)",
            params![summary_id, skill_id],
        )
        .map_err(|e| {
            if let rusqlite::Error::SqliteFailure(ref err, ref msg) = e {
                if err.code == rusqlite::ErrorCode::ConstraintViolation {
                    if msg.as_deref().unwrap_or("").contains("FOREIGN KEY") {
                        return ForgeError::NotFound {
                            entity_type: "skill or summary".into(),
                            id: format!("summary={}, skill={}", summary_id, skill_id),
                        };
                    }
                }
            }
            ForgeError::Database { source: e }
        })?;
        Ok(())
    }

    /// Remove a skill keyword from a summary.
    pub fn remove_skill(
        conn: &Connection,
        summary_id: &str,
        skill_id: &str,
    ) -> Result<(), ForgeError> {
        conn.execute(
            "DELETE FROM summary_skills WHERE summary_id = ?1 AND skill_id = ?2",
            params![summary_id, skill_id],
        )?;
        Ok(())
    }

    /// Get all skills linked to a summary.
    pub fn get_skills(conn: &Connection, summary_id: &str) -> Result<Vec<Skill>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT sk.id, sk.name, sk.category
             FROM skills sk
             INNER JOIN summary_skills ss ON ss.skill_id = sk.id
             WHERE ss.summary_id = ?1
             ORDER BY sk.name ASC",
        )?;
        let skills: Vec<Skill> = stmt
            .query_map(params![summary_id], |row| {
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

    // ── Linked resumes ──────────────────────────────────────────────

    /// List resumes that reference this summary via `summary_id`, with pagination.
    pub fn list_linked_resumes(
        conn: &Connection,
        summary_id: &str,
        offset: i64,
        limit: i64,
    ) -> Result<(Vec<Resume>, Pagination), ForgeError> {
        let total: i64 = conn.query_row(
            "SELECT COUNT(*) FROM resumes WHERE summary_id = ?1",
            params![summary_id],
            |row| row.get(0),
        )?;

        let mut stmt = conn.prepare(
            "SELECT id, name, target_role, target_employer, archetype, status, header,
                    summary_id, generated_tagline, tagline_override, markdown_override,
                    markdown_override_updated_at, latex_override, latex_override_updated_at,
                    summary_override, summary_override_updated_at, show_clearance_in_header,
                    created_at, updated_at
             FROM resumes WHERE summary_id = ?1
             ORDER BY updated_at DESC
             LIMIT ?2 OFFSET ?3",
        )?;
        let resumes: Vec<Resume> = stmt
            .query_map(params![summary_id, limit, offset], Self::map_resume)?
            .collect::<Result<_, _>>()?;

        Ok((
            resumes,
            Pagination {
                total,
                offset,
                limit,
            },
        ))
    }

    /// Count resumes linked to a summary (used for `linked_resume_count`).
    pub fn count_linked_resumes(conn: &Connection, summary_id: &str) -> Result<i64, ForgeError> {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM resumes WHERE summary_id = ?1",
            params![summary_id],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    // ── Clone ───────────────────────────────────────────────────────

    /// Duplicate a summary row and its `summary_skills` links, returning the new row.
    pub fn clone_summary(conn: &Connection, id: &str) -> Result<Summary, ForgeError> {
        let source = Self::get(conn, id)?.ok_or_else(|| ForgeError::NotFound {
            entity_type: "summary".into(),
            id: id.into(),
        })?;

        let new_id = new_id();
        let now = now_iso();

        conn.execute(
            "INSERT INTO summaries (id, title, role, description, is_template, industry_id, role_type_id, notes, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6, ?7, ?8, ?8)",
            params![
                new_id,
                format!("Copy of {}", source.title),
                source.role,
                source.description,
                source.industry_id,
                source.role_type_id,
                source.notes,
                now,
            ],
        )?;

        // Copy skill links
        let skills = Self::get_skills(conn, id)?;
        for skill in &skills {
            let _ = conn.execute(
                "INSERT OR IGNORE INTO summary_skills (summary_id, skill_id) VALUES (?1, ?2)",
                params![new_id, skill.id],
            );
        }

        Self::get(conn, &new_id)?
            .ok_or_else(|| ForgeError::Internal("Cloned summary not found".into()))
    }

    // ── Internal helpers ────────────────────────────────────────────

    /// Map a row to a Summary with linked_resume_count defaulted to 0 (caller hydrates).
    fn map_summary_without_count(row: &rusqlite::Row) -> rusqlite::Result<Summary> {
        Ok(Summary {
            id: row.get(0)?,
            title: row.get(1)?,
            role: row.get(2)?,
            description: row.get(3)?,
            is_template: row.get(4)?,
            industry_id: row.get(5)?,
            role_type_id: row.get(6)?,
            linked_resume_count: 0, // hydrated by caller
            notes: row.get(7)?,
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
        })
    }

    fn map_resume(row: &rusqlite::Row) -> rusqlite::Result<Resume> {
        Ok(Resume {
            id: row.get(0)?,
            name: row.get(1)?,
            target_role: row.get(2)?,
            target_employer: row.get(3)?,
            archetype: row.get(4)?,
            status: row
                .get::<_, String>(5)?
                .parse()
                .unwrap_or(ResumeStatus::Draft),
            header: row.get(6)?,
            summary_id: row.get(7)?,
            generated_tagline: row.get(8)?,
            tagline_override: row.get(9)?,
            markdown_override: row.get(10)?,
            markdown_override_updated_at: row.get(11)?,
            latex_override: row.get(12)?,
            latex_override_updated_at: row.get(13)?,
            summary_override: row.get(14)?,
            summary_override_updated_at: row.get(15)?,
            show_clearance_in_header: row.get(16)?,
            created_at: row.get(17)?,
            updated_at: row.get(18)?,
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
    fn create_and_get_summary() {
        let forge = setup();
        let input = CreateSummary {
            title: "Senior SRE Summary".into(),
            role: Some("Senior SRE".into()),
            description: Some("Experienced in cloud infrastructure".into()),
            is_template: Some(0),
            industry_id: None,
            role_type_id: None,
            notes: Some("For cloud roles".into()),
        };
        let summary = SummaryStore::create(forge.conn(), &input).unwrap();
        assert_eq!(summary.title, "Senior SRE Summary");
        assert_eq!(summary.role, Some("Senior SRE".into()));
        assert_eq!(summary.is_template, 0);
        assert_eq!(summary.linked_resume_count, 0);

        let fetched = SummaryStore::get(forge.conn(), &summary.id)
            .unwrap()
            .unwrap();
        assert_eq!(fetched.id, summary.id);
        assert_eq!(fetched.title, "Senior SRE Summary");
    }

    #[test]
    fn create_template() {
        let forge = setup();
        let input = CreateSummary {
            title: "Generic Template".into(),
            role: None,
            description: None,
            is_template: Some(1),
            industry_id: None,
            role_type_id: None,
            notes: None,
        };
        let summary = SummaryStore::create(forge.conn(), &input).unwrap();
        assert_eq!(summary.is_template, 1);
    }

    #[test]
    fn get_returns_none_for_missing() {
        let forge = setup();
        let result = SummaryStore::get(forge.conn(), "nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn list_empty() {
        let forge = setup();
        let (summaries, pagination) = SummaryStore::list(forge.conn(), None, None, 0, 50).unwrap();
        assert!(summaries.is_empty());
        assert_eq!(pagination.total, 0);
    }

    #[test]
    fn list_with_template_filter() {
        let forge = setup();
        SummaryStore::create(
            forge.conn(),
            &CreateSummary {
                title: "Template".into(),
                role: None,
                description: None,
                is_template: Some(1),
                industry_id: None,
                role_type_id: None,
                notes: None,
            },
        )
        .unwrap();
        SummaryStore::create(
            forge.conn(),
            &CreateSummary {
                title: "Regular".into(),
                role: None,
                description: None,
                is_template: Some(0),
                industry_id: None,
                role_type_id: None,
                notes: None,
            },
        )
        .unwrap();

        let filter = SummaryFilter {
            is_template: Some(1),
            ..Default::default()
        };
        let (summaries, _) = SummaryStore::list(forge.conn(), Some(&filter), None, 0, 50).unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].title, "Template");
    }

    #[test]
    fn list_with_search() {
        let forge = setup();
        SummaryStore::create(
            forge.conn(),
            &CreateSummary {
                title: "Cloud Engineer".into(),
                role: None,
                description: Some("AWS and GCP expert".into()),
                is_template: None,
                industry_id: None,
                role_type_id: None,
                notes: None,
            },
        )
        .unwrap();
        SummaryStore::create(
            forge.conn(),
            &CreateSummary {
                title: "Security Analyst".into(),
                role: None,
                description: None,
                is_template: None,
                industry_id: None,
                role_type_id: None,
                notes: None,
            },
        )
        .unwrap();

        let filter = SummaryFilter {
            search: Some("cloud".into()),
            ..Default::default()
        };
        let (summaries, _) = SummaryStore::list(forge.conn(), Some(&filter), None, 0, 50).unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].title, "Cloud Engineer");
    }

    #[test]
    fn update_summary() {
        let forge = setup();
        let summary = SummaryStore::create(
            forge.conn(),
            &CreateSummary {
                title: "Old Title".into(),
                role: Some("Old Role".into()),
                description: None,
                is_template: None,
                industry_id: None,
                role_type_id: None,
                notes: None,
            },
        )
        .unwrap();

        let updated = SummaryStore::update(
            forge.conn(),
            &summary.id,
            &UpdateSummary {
                title: Some("New Title".into()),
                role: Some(Some("New Role".into())),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(updated.title, "New Title");
        assert_eq!(updated.role, Some("New Role".into()));
    }

    #[test]
    fn delete_summary() {
        let forge = setup();
        let summary = SummaryStore::create(
            forge.conn(),
            &CreateSummary {
                title: "To Delete".into(),
                role: None,
                description: None,
                is_template: None,
                industry_id: None,
                role_type_id: None,
                notes: None,
            },
        )
        .unwrap();

        SummaryStore::delete(forge.conn(), &summary.id).unwrap();
        assert!(SummaryStore::get(forge.conn(), &summary.id)
            .unwrap()
            .is_none());
    }

    #[test]
    fn delete_missing_returns_not_found() {
        let forge = setup();
        let result = SummaryStore::delete(forge.conn(), "nonexistent");
        assert!(matches!(result, Err(ForgeError::NotFound { .. })));
    }

    #[test]
    fn toggle_template() {
        let forge = setup();
        let summary = SummaryStore::create(
            forge.conn(),
            &CreateSummary {
                title: "Toggle Test".into(),
                role: None,
                description: None,
                is_template: Some(0),
                industry_id: None,
                role_type_id: None,
                notes: None,
            },
        )
        .unwrap();
        assert_eq!(summary.is_template, 0);

        let toggled = SummaryStore::toggle_template(forge.conn(), &summary.id).unwrap();
        assert_eq!(toggled.is_template, 1);

        let toggled_back = SummaryStore::toggle_template(forge.conn(), &summary.id).unwrap();
        assert_eq!(toggled_back.is_template, 0);
    }

    #[test]
    fn clone_summary() {
        let forge = setup();
        let original = SummaryStore::create(
            forge.conn(),
            &CreateSummary {
                title: "Original".into(),
                role: Some("Dev".into()),
                description: Some("Desc".into()),
                is_template: Some(1),
                industry_id: None,
                role_type_id: None,
                notes: Some("Notes".into()),
            },
        )
        .unwrap();

        let cloned = SummaryStore::clone_summary(forge.conn(), &original.id).unwrap();
        assert_eq!(cloned.title, "Copy of Original");
        assert_eq!(cloned.role, Some("Dev".into()));
        assert_eq!(cloned.description, Some("Desc".into()));
        assert_eq!(cloned.is_template, 0); // clone is not a template
        assert_ne!(cloned.id, original.id);
    }

    #[test]
    fn list_templates_float_to_top() {
        let forge = setup();
        // Create regular first, then template
        SummaryStore::create(
            forge.conn(),
            &CreateSummary {
                title: "Regular Summary".into(),
                role: None,
                description: None,
                is_template: Some(0),
                industry_id: None,
                role_type_id: None,
                notes: None,
            },
        )
        .unwrap();
        SummaryStore::create(
            forge.conn(),
            &CreateSummary {
                title: "Template Summary".into(),
                role: None,
                description: None,
                is_template: Some(1),
                industry_id: None,
                role_type_id: None,
                notes: None,
            },
        )
        .unwrap();

        let (summaries, _) = SummaryStore::list(forge.conn(), None, None, 0, 50).unwrap();
        assert_eq!(summaries.len(), 2);
        assert_eq!(summaries[0].is_template, 1); // template first
        assert_eq!(summaries[1].is_template, 0);
    }
}
