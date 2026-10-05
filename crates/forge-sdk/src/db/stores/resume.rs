//! Resume repository — data access layer for resumes, entries, sections, skills,
//! and certifications.
//!
//! Covers ~10 tables: `resumes`, `resume_entries`, `resume_sections`,
//! `resume_skills`, `resume_certifications`, and supporting junctions.

use rusqlite::{params, Connection, OptionalExtension};

use forge_core::{
    new_id, now_iso, AddResumeCertification, AddResumeEntry, CoverageSummary, CreateResume,
    ForgeError, Gap, GapAnalysis, GapBulletCandidate, Pagination, Resume, ResumeCertification,
    ResumeEntry, ResumeEntryWithContent, ResumeSectionEntity, ResumeSkill, ResumeStatus,
    ResumeWithEntries, ResumeWithEntriesSection, UpdateResume,
};

/// Data access for resume-related tables.
pub struct ResumeStore;

impl ResumeStore {
    // ── Resume CRUD ─────────────────────────────────────────────────

    /// Insert a new resume row. Returns the hydrated `Resume`.
    pub fn create(conn: &Connection, input: &CreateResume) -> Result<Resume, ForgeError> {
        let id = new_id();
        let now = now_iso();

        conn.execute(
            "INSERT INTO resumes (id, name, target_role, target_employer, archetype, status, summary_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'draft', ?6, ?7, ?7)",
            params![
                id,
                input.name,
                input.target_role,
                input.target_employer,
                input.archetype,
                input.summary_id,
                now,
            ],
        )?;

        Self::get(conn, &id)?
            .ok_or_else(|| ForgeError::Internal("Resume created but not found".into()))
    }

    /// Fetch a single resume by ID.
    pub fn get(conn: &Connection, id: &str) -> Result<Option<Resume>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT id, name, target_role, target_employer, archetype, status,
                    header, summary_id, generated_tagline, tagline_override,
                    markdown_override, markdown_override_updated_at,
                    latex_override, latex_override_updated_at,
                    summary_override, summary_override_updated_at,
                    show_clearance_in_header, created_at, updated_at
             FROM resumes WHERE id = ?1",
        )?;

        let result = stmt.query_row(params![id], Self::map_resume).optional()?;
        Ok(result)
    }

    /// Fetch a resume with all sections and entries hydrated, including
    /// per-entry `perspective_content` from the perspectives table.
    pub fn get_with_entries(
        conn: &Connection,
        id: &str,
    ) -> Result<Option<ResumeWithEntries>, ForgeError> {
        let resume = match Self::get(conn, id)? {
            Some(r) => r,
            None => return Ok(None),
        };

        // Fetch sections
        let sections = Self::list_sections(conn, id)?;

        // Fetch all entries for this resume with perspective content
        let mut entry_stmt = conn.prepare(
            "SELECT re.id, re.resume_id, re.section_id, re.perspective_id,
                    re.source_id, re.content, re.perspective_content_snapshot,
                    re.position, re.created_at, re.updated_at,
                    p.content AS perspective_content
             FROM resume_entries re
             LEFT JOIN perspectives p ON p.id = re.perspective_id
             WHERE re.resume_id = ?1
             ORDER BY re.position ASC",
        )?;

        let all_entries: Vec<(ResumeEntry, Option<String>)> = entry_stmt
            .query_map(params![id], |row| {
                let entry = ResumeEntry {
                    id: row.get(0)?,
                    resume_id: row.get(1)?,
                    section_id: row.get(2)?,
                    perspective_id: row.get(3)?,
                    source_id: row.get(4)?,
                    content: row.get(5)?,
                    perspective_content_snapshot: row.get(6)?,
                    position: row.get(7)?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                };
                let perspective_content: Option<String> = row.get(10)?;
                Ok((entry, perspective_content))
            })?
            .collect::<Result<_, _>>()?;

        // Group entries by section
        let result_sections = sections
            .into_iter()
            .map(|sec| {
                let entries: Vec<ResumeEntryWithContent> = all_entries
                    .iter()
                    .filter(|(e, _)| e.section_id == sec.id)
                    .map(|(e, pc)| ResumeEntryWithContent {
                        base: e.clone(),
                        perspective_content: pc.clone(),
                    })
                    .collect();

                ResumeWithEntriesSection {
                    id: sec.id,
                    title: sec.title,
                    entry_type: sec.entry_type,
                    position: sec.position,
                    entries,
                }
            })
            .collect();

        Ok(Some(ResumeWithEntries {
            base: resume,
            sections: result_sections,
        }))
    }

    /// List resumes ordered by `created_at DESC` with pagination.
    pub fn list(
        conn: &Connection,
        offset: i64,
        limit: i64,
    ) -> Result<(Vec<Resume>, Pagination), ForgeError> {
        let total: i64 = conn.query_row("SELECT COUNT(*) FROM resumes", [], |row| row.get(0))?;

        let mut stmt = conn.prepare(
            "SELECT id, name, target_role, target_employer, archetype, status,
                    header, summary_id, generated_tagline, tagline_override,
                    markdown_override, markdown_override_updated_at,
                    latex_override, latex_override_updated_at,
                    summary_override, summary_override_updated_at,
                    show_clearance_in_header, created_at, updated_at
             FROM resumes
             ORDER BY created_at DESC
             LIMIT ?1 OFFSET ?2",
        )?;

        let resumes: Vec<Resume> = stmt
            .query_map(params![limit, offset], Self::map_resume)?
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

    /// Partially update a resume. Only fields present in `UpdateResume`
    /// are patched. Timestamps for override fields are auto-managed.
    pub fn update(conn: &Connection, id: &str, input: &UpdateResume) -> Result<Resume, ForgeError> {
        // Verify exists
        Self::get(conn, id)?.ok_or_else(|| ForgeError::NotFound {
            entity_type: "resume".into(),
            id: id.into(),
        })?;

        let mut sets = Vec::new();
        let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        macro_rules! set_field {
            ($field:ident) => {
                if let Some(ref v) = input.$field {
                    sets.push(format!(
                        "{} = ?{}",
                        stringify!($field),
                        bind_values.len() + 1
                    ));
                    bind_values.push(Box::new(v.clone()));
                }
            };
        }

        macro_rules! set_option_field {
            ($field:ident) => {
                if let Some(ref v) = input.$field {
                    sets.push(format!(
                        "{} = ?{}",
                        stringify!($field),
                        bind_values.len() + 1
                    ));
                    bind_values.push(Box::new(v.clone()));
                }
            };
        }

        set_field!(name);
        set_field!(target_role);
        set_field!(target_employer);
        set_field!(archetype);

        if let Some(ref status) = input.status {
            sets.push(format!("status = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(status.to_string()));
        }

        if let Some(ref v) = input.show_clearance_in_header {
            sets.push(format!(
                "show_clearance_in_header = ?{}",
                bind_values.len() + 1
            ));
            bind_values.push(Box::new(*v));
        }

        // Option<Option<T>> fields: Some(None) clears, Some(Some(v)) sets
        set_option_field!(header);
        set_option_field!(summary_id);

        if let Some(ref v) = input.summary_override {
            sets.push(format!("summary_override = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
            let now = now_iso();
            if v.is_some() {
                sets.push(format!(
                    "summary_override_updated_at = ?{}",
                    bind_values.len() + 1
                ));
                bind_values.push(Box::new(now));
            }
        }

        if let Some(ref v) = input.markdown_override {
            sets.push(format!("markdown_override = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
            let now = now_iso();
            if v.is_some() {
                sets.push(format!(
                    "markdown_override_updated_at = ?{}",
                    bind_values.len() + 1
                ));
                bind_values.push(Box::new(now));
            }
        }

        if let Some(ref v) = input.latex_override {
            sets.push(format!("latex_override = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
            let now = now_iso();
            if v.is_some() {
                sets.push(format!(
                    "latex_override_updated_at = ?{}",
                    bind_values.len() + 1
                ));
                bind_values.push(Box::new(now));
            }
        }

        if !sets.is_empty() {
            let now = now_iso();
            sets.push(format!("updated_at = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(now));

            let sql = format!(
                "UPDATE resumes SET {} WHERE id = ?{}",
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
            .ok_or_else(|| ForgeError::Internal("Resume updated but not found".into()))
    }

    /// Delete a resume by ID. Cascades via foreign keys.
    pub fn delete(conn: &Connection, id: &str) -> Result<(), ForgeError> {
        let deleted = conn.execute("DELETE FROM resumes WHERE id = ?1", params![id])?;
        if deleted == 0 {
            return Err(ForgeError::NotFound {
                entity_type: "resume".into(),
                id: id.into(),
            });
        }
        Ok(())
    }

    // ── Resume Entries ──────────────────────────────────────────────

    /// Add an entry to a resume section. Auto-computes `position` as
    /// `MAX(position) + 1` within the section when omitted.
    pub fn add_entry(
        conn: &Connection,
        resume_id: &str,
        input: &AddResumeEntry,
    ) -> Result<ResumeEntry, ForgeError> {
        // Verify resume exists
        Self::get(conn, resume_id)?.ok_or_else(|| ForgeError::NotFound {
            entity_type: "resume".into(),
            id: resume_id.into(),
        })?;

        // Verify section exists and belongs to resume
        Self::get_section(conn, resume_id, &input.section_id)?;

        // Validate perspective if provided
        if let Some(ref pid) = input.perspective_id {
            let status: String = conn
                .query_row(
                    "SELECT status FROM perspectives WHERE id = ?1",
                    params![pid],
                    |row| row.get(0),
                )
                .optional()?
                .ok_or_else(|| ForgeError::NotFound {
                    entity_type: "perspective".into(),
                    id: pid.clone(),
                })?;

            if status != "approved" {
                return Err(ForgeError::Validation {
                    message: format!(
                        "Perspective {pid} has status '{status}', expected 'approved'"
                    ),
                    field: Some("perspective_id".into()),
                });
            }
        }

        let id = new_id();
        let now = now_iso();

        let position = match input.position {
            Some(p) => p,
            None => {
                let max: Option<i32> = conn
                    .query_row(
                        "SELECT MAX(position) FROM resume_entries WHERE section_id = ?1",
                        params![input.section_id],
                        |row| row.get(0),
                    )
                    .optional()?
                    .flatten();
                max.unwrap_or(-1) + 1
            }
        };

        // Snapshot perspective content if perspective_id provided
        let snapshot: Option<String> = if let Some(ref pid) = input.perspective_id {
            conn.query_row(
                "SELECT content FROM perspectives WHERE id = ?1",
                params![pid],
                |row| row.get(0),
            )
            .optional()?
        } else {
            None
        };

        conn.execute(
            "INSERT INTO resume_entries (id, resume_id, section_id, perspective_id, source_id, content, perspective_content_snapshot, position, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
            params![
                id,
                resume_id,
                input.section_id,
                input.perspective_id,
                input.source_id,
                input.content,
                snapshot,
                position,
                now,
            ],
        )?;

        Self::get_entry(conn, &id)?
            .ok_or_else(|| ForgeError::Internal("Entry created but not found".into()))
    }

    /// Update an existing resume entry.
    pub fn update_entry(
        conn: &Connection,
        resume_id: &str,
        entry_id: &str,
        content: Option<Option<String>>,
        section_id: Option<String>,
        position: Option<i32>,
    ) -> Result<ResumeEntry, ForgeError> {
        // Verify entry exists and belongs to resume
        let existing = Self::get_entry(conn, entry_id)?.ok_or_else(|| ForgeError::NotFound {
            entity_type: "resume_entry".into(),
            id: entry_id.into(),
        })?;
        if existing.resume_id != resume_id {
            return Err(ForgeError::NotFound {
                entity_type: "resume_entry".into(),
                id: entry_id.into(),
            });
        }

        let mut sets = Vec::new();
        let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(ref c) = content {
            sets.push(format!("content = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(c.clone()));
            // Clear snapshot when content set to None (reference mode)
            if c.is_none() {
                sets.push(format!(
                    "perspective_content_snapshot = ?{}",
                    bind_values.len() + 1
                ));
                bind_values.push(Box::new(None::<String>));
            }
        }

        if let Some(ref sid) = section_id {
            // Verify target section belongs to same resume
            Self::get_section(conn, resume_id, sid)?;
            sets.push(format!("section_id = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(sid.clone()));
        }

        if let Some(p) = position {
            sets.push(format!("position = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(p));
        }

        if !sets.is_empty() {
            let now = now_iso();
            sets.push(format!("updated_at = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(now));

            let sql = format!(
                "UPDATE resume_entries SET {} WHERE id = ?{}",
                sets.join(", "),
                bind_values.len() + 1
            );
            bind_values.push(Box::new(entry_id.to_string()));

            conn.execute(
                &sql,
                rusqlite::params_from_iter(bind_values.iter().map(|b| b.as_ref())),
            )?;
        }

        Self::get_entry(conn, entry_id)?
            .ok_or_else(|| ForgeError::Internal("Entry updated but not found".into()))
    }

    /// Remove an entry from a resume.
    pub fn remove_entry(
        conn: &Connection,
        resume_id: &str,
        entry_id: &str,
    ) -> Result<(), ForgeError> {
        let deleted = conn.execute(
            "DELETE FROM resume_entries WHERE id = ?1 AND resume_id = ?2",
            params![entry_id, resume_id],
        )?;
        if deleted == 0 {
            return Err(ForgeError::NotFound {
                entity_type: "resume_entry".into(),
                id: entry_id.into(),
            });
        }
        Ok(())
    }

    /// Atomically reorder entries across sections.
    pub fn reorder_entries(
        conn: &Connection,
        resume_id: &str,
        entries: &[(String, String, i32)],
    ) -> Result<(), ForgeError> {
        let mut stmt = conn.prepare(
            "UPDATE resume_entries SET section_id = ?1, position = ?2, updated_at = ?3
             WHERE id = ?4 AND resume_id = ?5",
        )?;
        let now = now_iso();
        for (id, section_id, position) in entries {
            let updated = stmt.execute(params![section_id, position, now, id, resume_id])?;
            if updated == 0 {
                return Err(ForgeError::NotFound {
                    entity_type: "resume_entry".into(),
                    id: id.clone(),
                });
            }
        }
        Ok(())
    }

    // ── Sections ────────────────────────────────────────────────────

    /// Create a new section within a resume.
    pub fn create_section(
        conn: &Connection,
        resume_id: &str,
        title: &str,
        entry_type: &str,
        position: Option<i32>,
    ) -> Result<ResumeSectionEntity, ForgeError> {
        // Verify resume exists
        Self::get(conn, resume_id)?.ok_or_else(|| ForgeError::NotFound {
            entity_type: "resume".into(),
            id: resume_id.into(),
        })?;

        let id = new_id();
        let now = now_iso();

        let position = match position {
            Some(p) => p,
            None => {
                let max: Option<i32> = conn
                    .query_row(
                        "SELECT MAX(position) FROM resume_sections WHERE resume_id = ?1",
                        params![resume_id],
                        |row| row.get(0),
                    )
                    .optional()?
                    .flatten();
                max.unwrap_or(-1) + 1
            }
        };

        conn.execute(
            "INSERT INTO resume_sections (id, resume_id, title, entry_type, position, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
            params![id, resume_id, title, entry_type, position, now],
        )?;

        Self::get_section_by_id(conn, &id)?
            .ok_or_else(|| ForgeError::Internal("Section created but not found".into()))
    }

    /// List all sections for a resume, ordered by `position ASC`.
    pub fn list_sections(
        conn: &Connection,
        resume_id: &str,
    ) -> Result<Vec<ResumeSectionEntity>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT id, resume_id, title, entry_type, position, created_at, updated_at
             FROM resume_sections
             WHERE resume_id = ?1
             ORDER BY position ASC",
        )?;

        let sections = stmt
            .query_map(params![resume_id], Self::map_section)?
            .collect::<Result<_, _>>()?;

        Ok(sections)
    }

    /// Update a section's `title` and/or `position`.
    pub fn update_section(
        conn: &Connection,
        resume_id: &str,
        section_id: &str,
        title: Option<&str>,
        position: Option<i32>,
    ) -> Result<ResumeSectionEntity, ForgeError> {
        // Verify section belongs to resume
        Self::get_section(conn, resume_id, section_id)?;

        let mut sets = Vec::new();
        let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(t) = title {
            sets.push(format!("title = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(t.to_string()));
        }

        if let Some(p) = position {
            sets.push(format!("position = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(p));
        }

        if !sets.is_empty() {
            let now = now_iso();
            sets.push(format!("updated_at = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(now));

            let sql = format!(
                "UPDATE resume_sections SET {} WHERE id = ?{}",
                sets.join(", "),
                bind_values.len() + 1
            );
            bind_values.push(Box::new(section_id.to_string()));

            conn.execute(
                &sql,
                rusqlite::params_from_iter(bind_values.iter().map(|b| b.as_ref())),
            )?;
        }

        Self::get_section_by_id(conn, section_id)?
            .ok_or_else(|| ForgeError::Internal("Section updated but not found".into()))
    }

    /// Delete a section. Cascades to entries and skills via FK.
    pub fn delete_section(
        conn: &Connection,
        resume_id: &str,
        section_id: &str,
    ) -> Result<(), ForgeError> {
        let deleted = conn.execute(
            "DELETE FROM resume_sections WHERE id = ?1 AND resume_id = ?2",
            params![section_id, resume_id],
        )?;
        if deleted == 0 {
            return Err(ForgeError::NotFound {
                entity_type: "resume_section".into(),
                id: section_id.into(),
            });
        }
        Ok(())
    }

    // ── Skills (per section) ────────────────────────────────────────

    /// Pin a skill to a skills-type section.
    pub fn add_skill(
        conn: &Connection,
        resume_id: &str,
        section_id: &str,
        skill_id: &str,
    ) -> Result<ResumeSkill, ForgeError> {
        // Verify section belongs to resume and is a skills section
        let section = Self::get_section(conn, resume_id, section_id)?;
        if section.entry_type != "skills" {
            return Err(ForgeError::Validation {
                message: format!(
                    "Section '{}' has entry_type '{}', expected 'skills'",
                    section_id, section.entry_type
                ),
                field: Some("section_id".into()),
            });
        }

        // Check for duplicate
        let exists: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM resume_skills WHERE section_id = ?1 AND skill_id = ?2",
            params![section_id, skill_id],
            |row| row.get(0),
        )?;
        if exists {
            return Err(ForgeError::Conflict {
                message: format!("Skill {skill_id} already pinned to section {section_id}"),
            });
        }

        let id = new_id();
        let now = now_iso();

        let max_pos: Option<i32> = conn
            .query_row(
                "SELECT MAX(position) FROM resume_skills WHERE section_id = ?1",
                params![section_id],
                |row| row.get(0),
            )
            .optional()?
            .flatten();
        let position = max_pos.unwrap_or(-1) + 1;

        conn.execute(
            "INSERT INTO resume_skills (id, section_id, skill_id, position, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, section_id, skill_id, position, now],
        )?;

        Ok(ResumeSkill {
            id,
            section_id: section_id.to_string(),
            skill_id: skill_id.to_string(),
            position,
            created_at: now,
        })
    }

    /// Remove a skill from a section.
    pub fn remove_skill(
        conn: &Connection,
        resume_id: &str,
        section_id: &str,
        skill_id: &str,
    ) -> Result<(), ForgeError> {
        // Verify section belongs to resume
        Self::get_section(conn, resume_id, section_id)?;

        let deleted = conn.execute(
            "DELETE FROM resume_skills WHERE section_id = ?1 AND skill_id = ?2",
            params![section_id, skill_id],
        )?;
        if deleted == 0 {
            return Err(ForgeError::NotFound {
                entity_type: "resume_skill".into(),
                id: format!("{section_id}/{skill_id}"),
            });
        }
        Ok(())
    }

    /// List all skills pinned to a section, ordered by `position ASC`.
    pub fn list_skills_for_section(
        conn: &Connection,
        resume_id: &str,
        section_id: &str,
    ) -> Result<Vec<ResumeSkill>, ForgeError> {
        // Verify section belongs to resume
        Self::get_section(conn, resume_id, section_id)?;

        let mut stmt = conn.prepare(
            "SELECT id, section_id, skill_id, position, created_at
             FROM resume_skills
             WHERE section_id = ?1
             ORDER BY position ASC",
        )?;

        let skills = stmt
            .query_map(params![section_id], |row| {
                Ok(ResumeSkill {
                    id: row.get(0)?,
                    section_id: row.get(1)?,
                    skill_id: row.get(2)?,
                    position: row.get(3)?,
                    created_at: row.get(4)?,
                })
            })?
            .collect::<Result<_, _>>()?;

        Ok(skills)
    }

    /// Atomically reorder skills within a section.
    pub fn reorder_skills(
        conn: &Connection,
        resume_id: &str,
        section_id: &str,
        skills: &[(String, i32)],
    ) -> Result<(), ForgeError> {
        // Verify section belongs to resume
        Self::get_section(conn, resume_id, section_id)?;

        let mut stmt = conn
            .prepare("UPDATE resume_skills SET position = ?1 WHERE id = ?2 AND section_id = ?3")?;
        for (id, position) in skills {
            let updated = stmt.execute(params![position, id, section_id])?;
            if updated == 0 {
                return Err(ForgeError::NotFound {
                    entity_type: "resume_skill".into(),
                    id: id.clone(),
                });
            }
        }
        Ok(())
    }

    // ── Certifications (per resume) ─────────────────────────────────

    /// Add a certification to a certifications-type section.
    pub fn add_certification(
        conn: &Connection,
        resume_id: &str,
        input: &AddResumeCertification,
    ) -> Result<ResumeCertification, ForgeError> {
        // Verify section belongs to resume and is certifications type
        let section = Self::get_section(conn, resume_id, &input.section_id)?;
        if section.entry_type != "certifications" {
            return Err(ForgeError::Validation {
                message: format!(
                    "Section '{}' has entry_type '{}', expected 'certifications'",
                    input.section_id, section.entry_type
                ),
                field: Some("section_id".into()),
            });
        }

        // Check for duplicate (resume_id, certification_id)
        let exists: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM resume_certifications WHERE resume_id = ?1 AND certification_id = ?2",
            params![resume_id, input.certification_id],
            |row| row.get(0),
        )?;
        if exists {
            return Err(ForgeError::Conflict {
                message: format!(
                    "Certification {} already on resume {}",
                    input.certification_id, resume_id
                ),
            });
        }

        let id = new_id();
        let now = now_iso();

        let position = match input.position {
            Some(p) => p,
            None => {
                let max: Option<i32> = conn
                    .query_row(
                        "SELECT MAX(position) FROM resume_certifications WHERE resume_id = ?1",
                        params![resume_id],
                        |row| row.get(0),
                    )
                    .optional()?
                    .flatten();
                max.unwrap_or(-1) + 1
            }
        };

        conn.execute(
            "INSERT INTO resume_certifications (id, resume_id, certification_id, section_id, position, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, resume_id, input.certification_id, input.section_id, position, now],
        )?;

        Ok(ResumeCertification {
            id,
            resume_id: resume_id.to_string(),
            certification_id: input.certification_id.clone(),
            section_id: input.section_id.clone(),
            position,
            created_at: now,
        })
    }

    /// Remove a certification from a resume.
    pub fn remove_certification(
        conn: &Connection,
        resume_id: &str,
        rc_id: &str,
    ) -> Result<(), ForgeError> {
        let deleted = conn.execute(
            "DELETE FROM resume_certifications WHERE id = ?1 AND resume_id = ?2",
            params![rc_id, resume_id],
        )?;
        if deleted == 0 {
            return Err(ForgeError::NotFound {
                entity_type: "resume_certification".into(),
                id: rc_id.into(),
            });
        }
        Ok(())
    }

    /// List all certifications for a resume, ordered by `position ASC`.
    pub fn list_certifications(
        conn: &Connection,
        resume_id: &str,
    ) -> Result<Vec<ResumeCertification>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT id, resume_id, certification_id, section_id, position, created_at
             FROM resume_certifications
             WHERE resume_id = ?1
             ORDER BY position ASC",
        )?;

        let certs = stmt
            .query_map(params![resume_id], |row| {
                Ok(ResumeCertification {
                    id: row.get(0)?,
                    resume_id: row.get(1)?,
                    certification_id: row.get(2)?,
                    section_id: row.get(3)?,
                    position: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })?
            .collect::<Result<_, _>>()?;

        Ok(certs)
    }

    // ── Gap Analysis helpers ───────────────────────────────────────

    /// Find approved bullets that have no perspective for a given
    /// archetype + domain combination.
    pub fn find_bullets_for_gap(
        conn: &Connection,
        archetype: &str,
        domain: &str,
    ) -> Result<Vec<GapBulletCandidate>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT b.id, b.content, COALESCE(s.title, 'Unknown Source') AS source_title
             FROM bullets b
             LEFT JOIN bullet_sources bs ON b.id = bs.bullet_id AND bs.is_primary = 1
             LEFT JOIN sources s ON bs.source_id = s.id
             WHERE b.status = 'approved'
             AND b.id NOT IN (
                 SELECT p.bullet_id FROM perspectives p
                 WHERE p.target_archetype = ?1
                 AND p.domain = ?2
                 AND p.status = 'approved'
             )",
        )?;

        let candidates = stmt
            .query_map(params![archetype, domain], |row| {
                Ok(GapBulletCandidate {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    source_title: row.get(2)?,
                })
            })?
            .collect::<Result<_, _>>()?;

        Ok(candidates)
    }

    /// Get the primary source title for a bullet.
    pub fn get_source_title_for_bullet(
        conn: &Connection,
        bullet_id: &str,
    ) -> Result<String, ForgeError> {
        let title: Option<String> = conn
            .query_row(
                "SELECT s.title FROM sources s
                 JOIN bullet_sources bs ON s.id = bs.source_id
                 WHERE bs.bullet_id = ?1 AND bs.is_primary = 1",
                params![bullet_id],
                |row| row.get(0),
            )
            .optional()?;

        Ok(title.unwrap_or_else(|| "Unknown Source".into()))
    }

    // ── Gap analysis ─────────────────────────────────────────────────

    /// Perspectives for an archetype's expected domains that this resume lacks.
    /// Port of TS `ResumeService.analyzeGaps`.
    pub fn analyze_gaps(conn: &Connection, resume_id: &str) -> Result<GapAnalysis, ForgeError> {
        /// A domain with fewer included perspectives than this is "thin".
        const THIN_COVERAGE_THRESHOLD: i64 = 2;

        let resume = Self::get(conn, resume_id)?.ok_or_else(|| ForgeError::NotFound {
            entity_type: "resume".into(),
            id: resume_id.into(),
        })?;

        // Domain coverage of the perspectives already in the resume.
        let mut included: Vec<(String, i64)> = Vec::new(); // insertion-ordered counts
        let mut perspectives_included: i64 = 0;
        let mut stmt = conn.prepare(
            "SELECT p.domain FROM resume_entries re
             LEFT JOIN perspectives p ON p.id = re.perspective_id
             WHERE re.resume_id = ?1 AND re.perspective_id IS NOT NULL
             ORDER BY re.position ASC",
        )?;
        let domains = stmt
            .query_map(params![resume_id], |row| row.get::<_, Option<String>>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        for domain in domains {
            perspectives_included += 1;
            if let Some(domain) = domain {
                match included.iter_mut().find(|(d, _)| *d == domain) {
                    Some((_, n)) => *n += 1,
                    None => included.push((domain, 1)),
                }
            }
        }

        // Domains this archetype is expected to cover.
        let expected: Vec<String> = conn
            .query_row(
                "SELECT id FROM archetypes WHERE name = ?1 COLLATE NOCASE",
                params![resume.archetype],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .map(|archetype_id| super::archetype::ArchetypeStore::list_domains(conn, &archetype_id))
            .transpose()?
            .map(|ds| ds.into_iter().map(|d| d.name).collect())
            .unwrap_or_default();

        let total_approved: i64 = conn.query_row(
            "SELECT COUNT(*) FROM perspectives WHERE target_archetype = ?1 AND status = 'approved'",
            params![resume.archetype],
            |row| row.get(0),
        )?;

        let mut gaps = Vec::new();
        for domain in &expected {
            let count = included
                .iter()
                .find(|(d, _)| d == domain)
                .map_or(0, |(_, n)| *n);
            if count == 0 {
                gaps.push(Gap::MissingDomain {
                    domain: domain.clone(),
                    description: format!("No approved perspectives with domain '{domain}' are included in this resume"),
                    available_bullets: Self::find_bullets_for_gap(conn, &resume.archetype, domain)?,
                    recommendation: format!("Derive perspectives with domain '{domain}' from these bullets"),
                });
            } else if count < THIN_COVERAGE_THRESHOLD {
                gaps.push(Gap::ThinCoverage {
                    domain: domain.clone(),
                    current_count: count,
                    description: format!(
                        "Only {count} perspective with domain '{domain}' — consider adding more"
                    ),
                    recommendation: format!(
                        "Review approved bullets for additional {domain} framing opportunities"
                    ),
                });
            }
        }

        // Approved bullets with no approved perspective for this archetype.
        let mut stmt = conn.prepare(
            "SELECT b.id, b.content FROM bullets b
             WHERE b.status = 'approved'
             AND NOT EXISTS (
                 SELECT 1 FROM perspectives p
                 WHERE p.bullet_id = b.id AND p.target_archetype = ?1 AND p.status = 'approved'
             )",
        )?;
        let unused = stmt
            .query_map(params![resume.archetype], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        for (bullet_id, bullet_content) in unused {
            gaps.push(Gap::UnusedBullet {
                source_title: Self::get_source_title_for_bullet(conn, &bullet_id)?,
                bullet_id,
                bullet_content,
                description: format!(
                    "This approved bullet has no perspective for archetype '{}'",
                    resume.archetype
                ),
                recommendation: format!(
                    "Derive a perspective targeting '{}' archetype",
                    resume.archetype
                ),
            });
        }

        Ok(GapAnalysis {
            resume_id: resume.id.clone(),
            archetype: resume.archetype.clone(),
            target_role: resume.target_role.clone(),
            target_employer: resume.target_employer.clone(),
            gaps,
            coverage_summary: CoverageSummary {
                perspectives_included,
                total_approved_perspectives_for_archetype: total_approved,
                domains_represented: included.iter().map(|(d, _)| d.clone()).collect(),
                domains_missing: expected
                    .iter()
                    .filter(|d| !included.iter().any(|(i, _)| i == *d))
                    .cloned()
                    .collect(),
            },
        })
    }

    // ── Header / Override storage ───────────────────────────────────

    /// Persist a structured header JSON blob on a resume.
    pub fn update_header(
        conn: &Connection,
        id: &str,
        header: &serde_json::Value,
    ) -> Result<Resume, ForgeError> {
        let now = now_iso();
        let header_str = serde_json::to_string(header)
            .map_err(|e| ForgeError::Internal(format!("Failed to serialize header: {e}")))?;

        let updated = conn.execute(
            "UPDATE resumes SET header = ?1, updated_at = ?2 WHERE id = ?3",
            params![header_str, now, id],
        )?;
        if updated == 0 {
            return Err(ForgeError::NotFound {
                entity_type: "resume".into(),
                id: id.into(),
            });
        }

        Self::get(conn, id)?
            .ok_or_else(|| ForgeError::Internal("Resume updated but not found".into()))
    }

    /// Set or clear (`None`) the tagline override and bump `updated_at`. Writes `content` as
    /// given: the blank-to-NULL rule lives in `services::tagline::set_override`.
    /// Never touches `generated_tagline`.
    pub fn update_tagline_override(
        conn: &Connection,
        id: &str,
        content: Option<&str>,
    ) -> Result<Resume, ForgeError> {
        let updated = conn.execute(
            "UPDATE resumes SET tagline_override = ?1, updated_at = ?2 WHERE id = ?3",
            params![content, now_iso(), id],
        )?;
        if updated == 0 {
            return Err(ForgeError::NotFound {
                entity_type: "resume".into(),
                id: id.into(),
            });
        }
        Self::get(conn, id)?
            .ok_or_else(|| ForgeError::Internal("Resume updated but not found".into()))
    }

    /// Set or clear the Markdown override for a resume.
    pub fn update_markdown_override(
        conn: &Connection,
        id: &str,
        content: Option<&str>,
    ) -> Result<Resume, ForgeError> {
        let now = now_iso();
        let ts = if content.is_some() {
            Some(now.as_str())
        } else {
            None
        };

        let updated = conn.execute(
            "UPDATE resumes SET markdown_override = ?1, markdown_override_updated_at = ?2, updated_at = ?3 WHERE id = ?4",
            params![content, ts, now, id],
        )?;
        if updated == 0 {
            return Err(ForgeError::NotFound {
                entity_type: "resume".into(),
                id: id.into(),
            });
        }

        Self::get(conn, id)?
            .ok_or_else(|| ForgeError::Internal("Resume updated but not found".into()))
    }

    /// Set or clear the LaTeX override for a resume.
    pub fn update_latex_override(
        conn: &Connection,
        id: &str,
        content: Option<&str>,
    ) -> Result<Resume, ForgeError> {
        let now = now_iso();
        let ts = if content.is_some() {
            Some(now.as_str())
        } else {
            None
        };

        let updated = conn.execute(
            "UPDATE resumes SET latex_override = ?1, latex_override_updated_at = ?2, updated_at = ?3 WHERE id = ?4",
            params![content, ts, now, id],
        )?;
        if updated == 0 {
            return Err(ForgeError::NotFound {
                entity_type: "resume".into(),
                id: id.into(),
            });
        }

        Self::get(conn, id)?
            .ok_or_else(|| ForgeError::Internal("Resume updated but not found".into()))
    }

    // ── Internal helpers ────────────────────────────────────────────

    fn get_entry(conn: &Connection, id: &str) -> Result<Option<ResumeEntry>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT id, resume_id, section_id, perspective_id, source_id,
                    content, perspective_content_snapshot, position, created_at, updated_at
             FROM resume_entries WHERE id = ?1",
        )?;

        let result = stmt.query_row(params![id], Self::map_entry).optional()?;
        Ok(result)
    }

    fn get_section_by_id(
        conn: &Connection,
        id: &str,
    ) -> Result<Option<ResumeSectionEntity>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT id, resume_id, title, entry_type, position, created_at, updated_at
             FROM resume_sections WHERE id = ?1",
        )?;
        let result = stmt.query_row(params![id], Self::map_section).optional()?;
        Ok(result)
    }

    /// Get a section, verifying it belongs to the given resume.
    fn get_section(
        conn: &Connection,
        resume_id: &str,
        section_id: &str,
    ) -> Result<ResumeSectionEntity, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT id, resume_id, title, entry_type, position, created_at, updated_at
             FROM resume_sections WHERE id = ?1 AND resume_id = ?2",
        )?;

        stmt.query_row(params![section_id, resume_id], Self::map_section)
            .optional()?
            .ok_or_else(|| ForgeError::NotFound {
                entity_type: "resume_section".into(),
                id: section_id.into(),
            })
    }

    // ── Row mapping ─────────────────────────────────────────────────

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

    fn map_section(row: &rusqlite::Row) -> rusqlite::Result<ResumeSectionEntity> {
        Ok(ResumeSectionEntity {
            id: row.get(0)?,
            resume_id: row.get(1)?,
            title: row.get(2)?,
            entry_type: row.get(3)?,
            position: row.get(4)?,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
        })
    }

    fn map_entry(row: &rusqlite::Row) -> rusqlite::Result<ResumeEntry> {
        Ok(ResumeEntry {
            id: row.get(0)?,
            resume_id: row.get(1)?,
            section_id: row.get(2)?,
            perspective_id: row.get(3)?,
            source_id: row.get(4)?,
            content: row.get(5)?,
            perspective_content_snapshot: row.get(6)?,
            position: row.get(7)?,
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::stores::bullet::BulletStore;
    use crate::db::stores::perspective::PerspectiveStore;
    use crate::db::stores::skill::SkillStore;
    use crate::db::stores::source::SourceStore;
    use crate::forge::Forge;
    use forge_core::{CreatePerspectiveInput, CreateSource, Framing, SkillCategory, SourceType};

    fn setup() -> Forge {
        Forge::open_memory().unwrap()
    }

    fn create_resume(conn: &Connection) -> Resume {
        ResumeStore::create(
            conn,
            &CreateResume {
                name: "SRE Resume".into(),
                target_role: "Site Reliability Engineer".into(),
                target_employer: "Acme Corp".into(),
                archetype: "sre".into(),
                summary_id: None,
            },
        )
        .unwrap()
    }

    fn create_source(conn: &Connection) -> String {
        let src = SourceStore::create(
            conn,
            &CreateSource {
                title: "Test Source".into(),
                description: "Test description".into(),
                source_type: Some(SourceType::General),
                ..Default::default()
            },
        )
        .unwrap();
        src.base.id
    }

    fn create_approved_perspective(conn: &Connection) -> (String, String) {
        let source_id = create_source(conn);
        let bullet = BulletStore::create(
            conn,
            "Built distributed systems",
            None,
            None,
            Some("backend"),
            &[(source_id.clone(), true)],
            &[],
        )
        .unwrap();

        // Transition bullet to approved
        BulletStore::transition_status(conn, &bullet.id, forge_core::BulletStatus::InReview, None)
            .unwrap();
        BulletStore::transition_status(conn, &bullet.id, forge_core::BulletStatus::Approved, None)
            .unwrap();

        let perspective = PerspectiveStore::create(
            conn,
            &CreatePerspectiveInput {
                bullet_id: bullet.id.clone(),
                content: "Architected distributed systems".into(),
                bullet_content_snapshot: "Built distributed systems".into(),
                target_archetype: "sre".into(),
                domain: "infra".into(),
                framing: Framing::Responsibility,
                status: None,
                prompt_log_id: None,
            },
        )
        .unwrap();

        // Approve perspective
        PerspectiveStore::transition_status(
            conn,
            &perspective.id,
            forge_core::PerspectiveStatus::InReview,
            None,
        )
        .unwrap();
        PerspectiveStore::transition_status(
            conn,
            &perspective.id,
            forge_core::PerspectiveStatus::Approved,
            None,
        )
        .unwrap();

        (perspective.id, bullet.id)
    }

    // ── Resume CRUD tests ───────────────────────────────────────────

    #[test]
    fn create_resume_basic() {
        let forge = setup();
        let resume = create_resume(forge.conn());
        assert_eq!(resume.name, "SRE Resume");
        assert_eq!(resume.target_role, "Site Reliability Engineer");
        assert_eq!(resume.archetype, "sre");
        assert_eq!(resume.status, ResumeStatus::Draft);
        assert_eq!(resume.show_clearance_in_header, 1);
    }

    #[test]
    fn get_resume_returns_none_for_missing() {
        let forge = setup();
        assert!(ResumeStore::get(forge.conn(), "nonexistent")
            .unwrap()
            .is_none());
    }

    #[test]
    fn list_empty() {
        let forge = setup();
        let (resumes, pagination) = ResumeStore::list(forge.conn(), 0, 50).unwrap();
        assert!(resumes.is_empty());
        assert_eq!(pagination.total, 0);
    }

    #[test]
    fn list_with_pagination() {
        let forge = setup();
        for i in 0..5 {
            ResumeStore::create(
                forge.conn(),
                &CreateResume {
                    name: format!("Resume {i}"),
                    target_role: "Dev".into(),
                    target_employer: "Co".into(),
                    archetype: "dev".into(),
                    summary_id: None,
                },
            )
            .unwrap();
        }

        let (page, pagination) = ResumeStore::list(forge.conn(), 0, 2).unwrap();
        assert_eq!(page.len(), 2);
        assert_eq!(pagination.total, 5);

        let (page2, _) = ResumeStore::list(forge.conn(), 2, 2).unwrap();
        assert_eq!(page2.len(), 2);
        // Pages should not overlap
        assert_ne!(page[0].id, page2[0].id);
    }

    #[test]
    fn update_resume_partial() {
        let forge = setup();
        let resume = create_resume(forge.conn());

        let updated = ResumeStore::update(
            forge.conn(),
            &resume.id,
            &UpdateResume {
                name: Some("Updated Resume".into()),
                status: Some(ResumeStatus::InReview),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(updated.name, "Updated Resume");
        assert_eq!(updated.status, ResumeStatus::InReview);
        // Unchanged fields stay the same
        assert_eq!(updated.archetype, "sre");
    }

    #[test]
    fn update_resume_overrides() {
        let forge = setup();
        let resume = create_resume(forge.conn());

        // Set markdown override
        let updated = ResumeStore::update(
            forge.conn(),
            &resume.id,
            &UpdateResume {
                markdown_override: Some(Some("# Custom MD".into())),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(updated.markdown_override, Some("# Custom MD".into()));
        assert!(updated.markdown_override_updated_at.is_some());

        // Clear markdown override
        let cleared = ResumeStore::update(
            forge.conn(),
            &resume.id,
            &UpdateResume {
                markdown_override: Some(None),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(cleared.markdown_override.is_none());
    }

    #[test]
    fn delete_resume() {
        let forge = setup();
        let resume = create_resume(forge.conn());
        ResumeStore::delete(forge.conn(), &resume.id).unwrap();
        assert!(ResumeStore::get(forge.conn(), &resume.id)
            .unwrap()
            .is_none());
    }

    #[test]
    fn delete_missing_returns_not_found() {
        let forge = setup();
        let result = ResumeStore::delete(forge.conn(), "nonexistent");
        assert!(matches!(result, Err(ForgeError::NotFound { .. })));
    }

    // ── Section tests ───────────────────────────────────────────────

    #[test]
    fn create_section_auto_position() {
        let forge = setup();
        let resume = create_resume(forge.conn());

        let s1 =
            ResumeStore::create_section(forge.conn(), &resume.id, "Experience", "experience", None)
                .unwrap();
        let s2 = ResumeStore::create_section(forge.conn(), &resume.id, "Skills", "skills", None)
            .unwrap();

        assert_eq!(s1.position, 0);
        assert_eq!(s2.position, 1);
    }

    #[test]
    fn create_section_explicit_position() {
        let forge = setup();
        let resume = create_resume(forge.conn());

        let s = ResumeStore::create_section(forge.conn(), &resume.id, "Awards", "awards", Some(5))
            .unwrap();
        assert_eq!(s.position, 5);
    }

    #[test]
    fn list_sections_ordered() {
        let forge = setup();
        let resume = create_resume(forge.conn());

        ResumeStore::create_section(forge.conn(), &resume.id, "Skills", "skills", Some(2)).unwrap();
        ResumeStore::create_section(
            forge.conn(),
            &resume.id,
            "Experience",
            "experience",
            Some(0),
        )
        .unwrap();
        ResumeStore::create_section(forge.conn(), &resume.id, "Education", "education", Some(1))
            .unwrap();

        let sections = ResumeStore::list_sections(forge.conn(), &resume.id).unwrap();
        assert_eq!(sections.len(), 3);
        assert_eq!(sections[0].title, "Experience");
        assert_eq!(sections[1].title, "Education");
        assert_eq!(sections[2].title, "Skills");
    }

    #[test]
    fn update_section_title() {
        let forge = setup();
        let resume = create_resume(forge.conn());
        let section =
            ResumeStore::create_section(forge.conn(), &resume.id, "Old Title", "experience", None)
                .unwrap();

        let updated = ResumeStore::update_section(
            forge.conn(),
            &resume.id,
            &section.id,
            Some("Work History"),
            None,
        )
        .unwrap();
        assert_eq!(updated.title, "Work History");
    }

    #[test]
    fn delete_section() {
        let forge = setup();
        let resume = create_resume(forge.conn());
        let section =
            ResumeStore::create_section(forge.conn(), &resume.id, "Experience", "experience", None)
                .unwrap();

        ResumeStore::delete_section(forge.conn(), &resume.id, &section.id).unwrap();
        let sections = ResumeStore::list_sections(forge.conn(), &resume.id).unwrap();
        assert!(sections.is_empty());
    }

    #[test]
    fn delete_section_wrong_resume() {
        let forge = setup();
        let r1 = create_resume(forge.conn());
        let r2 = ResumeStore::create(
            forge.conn(),
            &CreateResume {
                name: "Other".into(),
                target_role: "Dev".into(),
                target_employer: "Co".into(),
                archetype: "dev".into(),
                summary_id: None,
            },
        )
        .unwrap();

        let section =
            ResumeStore::create_section(forge.conn(), &r1.id, "Experience", "experience", None)
                .unwrap();

        let result = ResumeStore::delete_section(forge.conn(), &r2.id, &section.id);
        assert!(matches!(result, Err(ForgeError::NotFound { .. })));
    }

    // ── Entry tests ─────────────────────────────────────────────────

    #[test]
    fn add_entry_auto_position() {
        let forge = setup();
        let resume = create_resume(forge.conn());
        let section =
            ResumeStore::create_section(forge.conn(), &resume.id, "Experience", "experience", None)
                .unwrap();

        let e1 = ResumeStore::add_entry(
            forge.conn(),
            &resume.id,
            &AddResumeEntry {
                section_id: section.id.clone(),
                perspective_id: None,
                source_id: None,
                position: None,
                content: Some("Entry 1".into()),
            },
        )
        .unwrap();

        let e2 = ResumeStore::add_entry(
            forge.conn(),
            &resume.id,
            &AddResumeEntry {
                section_id: section.id.clone(),
                perspective_id: None,
                source_id: None,
                position: None,
                content: Some("Entry 2".into()),
            },
        )
        .unwrap();

        assert_eq!(e1.position, 0);
        assert_eq!(e2.position, 1);
    }

    #[test]
    fn add_entry_with_approved_perspective() {
        let forge = setup();
        let resume = create_resume(forge.conn());
        let section =
            ResumeStore::create_section(forge.conn(), &resume.id, "Experience", "experience", None)
                .unwrap();
        let (perspective_id, _) = create_approved_perspective(forge.conn());

        let entry = ResumeStore::add_entry(
            forge.conn(),
            &resume.id,
            &AddResumeEntry {
                section_id: section.id.clone(),
                perspective_id: Some(perspective_id.clone()),
                source_id: None,
                position: None,
                content: None,
            },
        )
        .unwrap();

        assert_eq!(entry.perspective_id, Some(perspective_id));
        // Snapshot should be populated
        assert!(entry.perspective_content_snapshot.is_some());
    }

    #[test]
    fn add_entry_rejects_unapproved_perspective() {
        let forge = setup();
        let resume = create_resume(forge.conn());
        let section =
            ResumeStore::create_section(forge.conn(), &resume.id, "Experience", "experience", None)
                .unwrap();

        // Create a draft perspective (not approved)
        let source_id = create_source(forge.conn());
        let bullet = BulletStore::create(
            forge.conn(),
            "Test bullet",
            None,
            None,
            None,
            &[(source_id, true)],
            &[],
        )
        .unwrap();
        let perspective = PerspectiveStore::create(
            forge.conn(),
            &CreatePerspectiveInput {
                bullet_id: bullet.id,
                content: "Draft perspective".into(),
                bullet_content_snapshot: "Test bullet".into(),
                target_archetype: "sre".into(),
                domain: "infra".into(),
                framing: Framing::Responsibility,
                status: None,
                prompt_log_id: None,
            },
        )
        .unwrap();

        let result = ResumeStore::add_entry(
            forge.conn(),
            &resume.id,
            &AddResumeEntry {
                section_id: section.id,
                perspective_id: Some(perspective.id),
                source_id: None,
                position: None,
                content: None,
            },
        );
        assert!(matches!(result, Err(ForgeError::Validation { .. })));
    }

    #[test]
    fn update_entry_content() {
        let forge = setup();
        let resume = create_resume(forge.conn());
        let section =
            ResumeStore::create_section(forge.conn(), &resume.id, "Experience", "experience", None)
                .unwrap();

        let entry = ResumeStore::add_entry(
            forge.conn(),
            &resume.id,
            &AddResumeEntry {
                section_id: section.id.clone(),
                perspective_id: None,
                source_id: None,
                position: None,
                content: Some("Original".into()),
            },
        )
        .unwrap();

        let updated = ResumeStore::update_entry(
            forge.conn(),
            &resume.id,
            &entry.id,
            Some(Some("Updated".into())),
            None,
            None,
        )
        .unwrap();

        assert_eq!(updated.content, Some("Updated".into()));
    }

    #[test]
    fn remove_entry() {
        let forge = setup();
        let resume = create_resume(forge.conn());
        let section =
            ResumeStore::create_section(forge.conn(), &resume.id, "Experience", "experience", None)
                .unwrap();

        let entry = ResumeStore::add_entry(
            forge.conn(),
            &resume.id,
            &AddResumeEntry {
                section_id: section.id.clone(),
                perspective_id: None,
                source_id: None,
                position: None,
                content: Some("To remove".into()),
            },
        )
        .unwrap();

        ResumeStore::remove_entry(forge.conn(), &resume.id, &entry.id).unwrap();
        assert!(ResumeStore::get_entry(forge.conn(), &entry.id)
            .unwrap()
            .is_none());
    }

    #[test]
    fn remove_entry_wrong_resume() {
        let forge = setup();
        let r1 = create_resume(forge.conn());
        let r2 = ResumeStore::create(
            forge.conn(),
            &CreateResume {
                name: "Other".into(),
                target_role: "Dev".into(),
                target_employer: "Co".into(),
                archetype: "dev".into(),
                summary_id: None,
            },
        )
        .unwrap();

        let section =
            ResumeStore::create_section(forge.conn(), &r1.id, "Experience", "experience", None)
                .unwrap();
        let entry = ResumeStore::add_entry(
            forge.conn(),
            &r1.id,
            &AddResumeEntry {
                section_id: section.id,
                perspective_id: None,
                source_id: None,
                position: None,
                content: Some("Test".into()),
            },
        )
        .unwrap();

        let result = ResumeStore::remove_entry(forge.conn(), &r2.id, &entry.id);
        assert!(matches!(result, Err(ForgeError::NotFound { .. })));
    }

    #[test]
    fn reorder_entries() {
        let forge = setup();
        let resume = create_resume(forge.conn());
        let section =
            ResumeStore::create_section(forge.conn(), &resume.id, "Experience", "experience", None)
                .unwrap();

        let e1 = ResumeStore::add_entry(
            forge.conn(),
            &resume.id,
            &AddResumeEntry {
                section_id: section.id.clone(),
                perspective_id: None,
                source_id: None,
                position: None,
                content: Some("First".into()),
            },
        )
        .unwrap();
        let e2 = ResumeStore::add_entry(
            forge.conn(),
            &resume.id,
            &AddResumeEntry {
                section_id: section.id.clone(),
                perspective_id: None,
                source_id: None,
                position: None,
                content: Some("Second".into()),
            },
        )
        .unwrap();

        // Swap positions
        ResumeStore::reorder_entries(
            forge.conn(),
            &resume.id,
            &[
                (e1.id.clone(), section.id.clone(), 1),
                (e2.id.clone(), section.id.clone(), 0),
            ],
        )
        .unwrap();

        let e1_after = ResumeStore::get_entry(forge.conn(), &e1.id)
            .unwrap()
            .unwrap();
        let e2_after = ResumeStore::get_entry(forge.conn(), &e2.id)
            .unwrap()
            .unwrap();
        assert_eq!(e1_after.position, 1);
        assert_eq!(e2_after.position, 0);
    }

    // ── Skills tests ────────────────────────────────────────────────

    #[test]
    fn add_skill_to_skills_section() {
        let forge = setup();
        let resume = create_resume(forge.conn());
        let section =
            ResumeStore::create_section(forge.conn(), &resume.id, "Skills", "skills", None)
                .unwrap();
        let skill =
            SkillStore::create(forge.conn(), "Rust", Some(SkillCategory::Language)).unwrap();

        let rs = ResumeStore::add_skill(forge.conn(), &resume.id, &section.id, &skill.id).unwrap();
        assert_eq!(rs.skill_id, skill.id);
        assert_eq!(rs.position, 0);
    }

    #[test]
    fn add_skill_rejects_non_skills_section() {
        let forge = setup();
        let resume = create_resume(forge.conn());
        let section =
            ResumeStore::create_section(forge.conn(), &resume.id, "Experience", "experience", None)
                .unwrap();
        let skill =
            SkillStore::create(forge.conn(), "Rust", Some(SkillCategory::Language)).unwrap();

        let result = ResumeStore::add_skill(forge.conn(), &resume.id, &section.id, &skill.id);
        assert!(matches!(result, Err(ForgeError::Validation { .. })));
    }

    #[test]
    fn add_skill_duplicate_conflict() {
        let forge = setup();
        let resume = create_resume(forge.conn());
        let section =
            ResumeStore::create_section(forge.conn(), &resume.id, "Skills", "skills", None)
                .unwrap();
        let skill = SkillStore::create(forge.conn(), "Go", Some(SkillCategory::Language)).unwrap();

        ResumeStore::add_skill(forge.conn(), &resume.id, &section.id, &skill.id).unwrap();
        let result = ResumeStore::add_skill(forge.conn(), &resume.id, &section.id, &skill.id);
        assert!(matches!(result, Err(ForgeError::Conflict { .. })));
    }

    #[test]
    fn remove_skill() {
        let forge = setup();
        let resume = create_resume(forge.conn());
        let section =
            ResumeStore::create_section(forge.conn(), &resume.id, "Skills", "skills", None)
                .unwrap();
        let skill =
            SkillStore::create(forge.conn(), "Python", Some(SkillCategory::Language)).unwrap();

        ResumeStore::add_skill(forge.conn(), &resume.id, &section.id, &skill.id).unwrap();
        ResumeStore::remove_skill(forge.conn(), &resume.id, &section.id, &skill.id).unwrap();

        let skills =
            ResumeStore::list_skills_for_section(forge.conn(), &resume.id, &section.id).unwrap();
        assert!(skills.is_empty());
    }

    #[test]
    fn reorder_skills() {
        let forge = setup();
        let resume = create_resume(forge.conn());
        let section =
            ResumeStore::create_section(forge.conn(), &resume.id, "Skills", "skills", None)
                .unwrap();

        let s1 = SkillStore::create(forge.conn(), "Rust", Some(SkillCategory::Language)).unwrap();
        let s2 = SkillStore::create(forge.conn(), "Go", Some(SkillCategory::Language)).unwrap();

        let rs1 = ResumeStore::add_skill(forge.conn(), &resume.id, &section.id, &s1.id).unwrap();
        let rs2 = ResumeStore::add_skill(forge.conn(), &resume.id, &section.id, &s2.id).unwrap();

        // Swap
        ResumeStore::reorder_skills(
            forge.conn(),
            &resume.id,
            &section.id,
            &[(rs1.id.clone(), 1), (rs2.id.clone(), 0)],
        )
        .unwrap();

        let skills =
            ResumeStore::list_skills_for_section(forge.conn(), &resume.id, &section.id).unwrap();
        assert_eq!(skills[0].skill_id, s2.id);
        assert_eq!(skills[1].skill_id, s1.id);
    }

    // ── Certification tests ─────────────────────────────────────────

    #[test]
    fn add_certification_to_certifications_section() {
        let forge = setup();
        let resume = create_resume(forge.conn());
        let section = ResumeStore::create_section(
            forge.conn(),
            &resume.id,
            "Certifications",
            "certifications",
            None,
        )
        .unwrap();

        // Create a certification directly in the DB for testing
        let cert_id = new_id();
        let now = now_iso();
        forge.conn().execute(
            "INSERT INTO certifications (id, short_name, long_name, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4)",
            params![cert_id, "AWS SAA", "AWS Solutions Architect Associate", now],
        ).unwrap();

        let rc = ResumeStore::add_certification(
            forge.conn(),
            &resume.id,
            &AddResumeCertification {
                certification_id: cert_id.clone(),
                section_id: section.id.clone(),
                position: None,
            },
        )
        .unwrap();

        assert_eq!(rc.certification_id, cert_id);
        assert_eq!(rc.position, 0);
    }

    #[test]
    fn add_certification_duplicate_conflict() {
        let forge = setup();
        let resume = create_resume(forge.conn());
        let section = ResumeStore::create_section(
            forge.conn(),
            &resume.id,
            "Certifications",
            "certifications",
            None,
        )
        .unwrap();

        let cert_id = new_id();
        let now = now_iso();
        forge.conn().execute(
            "INSERT INTO certifications (id, short_name, long_name, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4)",
            params![cert_id, "CKA", "Certified Kubernetes Administrator", now],
        ).unwrap();

        ResumeStore::add_certification(
            forge.conn(),
            &resume.id,
            &AddResumeCertification {
                certification_id: cert_id.clone(),
                section_id: section.id.clone(),
                position: None,
            },
        )
        .unwrap();

        let result = ResumeStore::add_certification(
            forge.conn(),
            &resume.id,
            &AddResumeCertification {
                certification_id: cert_id,
                section_id: section.id,
                position: None,
            },
        );
        assert!(matches!(result, Err(ForgeError::Conflict { .. })));
    }

    #[test]
    fn remove_certification() {
        let forge = setup();
        let resume = create_resume(forge.conn());
        let section = ResumeStore::create_section(
            forge.conn(),
            &resume.id,
            "Certifications",
            "certifications",
            None,
        )
        .unwrap();

        let cert_id = new_id();
        let now = now_iso();
        forge.conn().execute(
            "INSERT INTO certifications (id, short_name, long_name, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4)",
            params![cert_id, "OSCP", "Offensive Security Certified Professional", now],
        ).unwrap();

        let rc = ResumeStore::add_certification(
            forge.conn(),
            &resume.id,
            &AddResumeCertification {
                certification_id: cert_id,
                section_id: section.id,
                position: None,
            },
        )
        .unwrap();

        ResumeStore::remove_certification(forge.conn(), &resume.id, &rc.id).unwrap();
        let certs = ResumeStore::list_certifications(forge.conn(), &resume.id).unwrap();
        assert!(certs.is_empty());
    }

    // ── get_with_entries test ───────────────────────────────────────

    #[test]
    fn get_with_entries_hydrates_sections_and_entries() {
        let forge = setup();
        let resume = create_resume(forge.conn());

        let exp_section = ResumeStore::create_section(
            forge.conn(),
            &resume.id,
            "Experience",
            "experience",
            Some(0),
        )
        .unwrap();
        ResumeStore::create_section(forge.conn(), &resume.id, "Skills", "skills", Some(1)).unwrap();

        ResumeStore::add_entry(
            forge.conn(),
            &resume.id,
            &AddResumeEntry {
                section_id: exp_section.id.clone(),
                perspective_id: None,
                source_id: None,
                position: None,
                content: Some("Built APIs".into()),
            },
        )
        .unwrap();
        ResumeStore::add_entry(
            forge.conn(),
            &resume.id,
            &AddResumeEntry {
                section_id: exp_section.id.clone(),
                perspective_id: None,
                source_id: None,
                position: None,
                content: Some("Led team".into()),
            },
        )
        .unwrap();

        let result = ResumeStore::get_with_entries(forge.conn(), &resume.id)
            .unwrap()
            .unwrap();

        assert_eq!(result.base.id, resume.id);
        assert_eq!(result.sections.len(), 2);
        assert_eq!(result.sections[0].title, "Experience");
        assert_eq!(result.sections[0].entries.len(), 2);
        assert_eq!(result.sections[1].title, "Skills");
        assert_eq!(result.sections[1].entries.len(), 0);
    }

    #[test]
    fn get_with_entries_includes_perspective_content() {
        let forge = setup();
        let resume = create_resume(forge.conn());
        let section =
            ResumeStore::create_section(forge.conn(), &resume.id, "Experience", "experience", None)
                .unwrap();
        let (perspective_id, _) = create_approved_perspective(forge.conn());

        ResumeStore::add_entry(
            forge.conn(),
            &resume.id,
            &AddResumeEntry {
                section_id: section.id.clone(),
                perspective_id: Some(perspective_id),
                source_id: None,
                position: None,
                content: None,
            },
        )
        .unwrap();

        let result = ResumeStore::get_with_entries(forge.conn(), &resume.id)
            .unwrap()
            .unwrap();
        let entry = &result.sections[0].entries[0];
        assert!(entry.perspective_content.is_some());
        assert_eq!(
            entry.perspective_content.as_deref(),
            Some("Architected distributed systems")
        );
    }

    // ── Header / override tests ─────────────────────────────────────

    #[test]
    fn update_header_json() {
        let forge = setup();
        let resume = create_resume(forge.conn());
        let header = serde_json::json!({
            "name": "Adam",
            "email": "adam@example.com"
        });

        let updated = ResumeStore::update_header(forge.conn(), &resume.id, &header).unwrap();
        assert!(updated.header.is_some());
        let parsed: serde_json::Value =
            serde_json::from_str(updated.header.as_ref().unwrap()).unwrap();
        assert_eq!(parsed["name"], "Adam");
    }

    #[test]
    fn update_markdown_override_set_and_clear() {
        let forge = setup();
        let resume = create_resume(forge.conn());

        let set = ResumeStore::update_markdown_override(forge.conn(), &resume.id, Some("# Hello"))
            .unwrap();
        assert_eq!(set.markdown_override.as_deref(), Some("# Hello"));
        assert!(set.markdown_override_updated_at.is_some());

        let cleared =
            ResumeStore::update_markdown_override(forge.conn(), &resume.id, None).unwrap();
        assert!(cleared.markdown_override.is_none());
        assert!(cleared.markdown_override_updated_at.is_none());
    }

    #[test]
    fn update_tagline_override_writes_verbatim_and_bumps_updated_at() {
        let forge = setup();
        let resume = create_resume(forge.conn());
        forge
            .conn()
            .execute(
                "UPDATE resumes SET generated_tagline = 'gen', updated_at = '2000-01-01T00:00:00Z' WHERE id = ?1",
                [&resume.id],
            )
            .unwrap();

        let set = ResumeStore::update_tagline_override(forge.conn(), &resume.id, Some("  Mine  "))
            .unwrap();
        assert_eq!(set.tagline_override.as_deref(), Some("  Mine  "));
        assert_eq!(set.generated_tagline.as_deref(), Some("gen"));
        assert_ne!(set.updated_at, "2000-01-01T00:00:00Z");

        let cleared = ResumeStore::update_tagline_override(forge.conn(), &resume.id, None).unwrap();
        assert!(cleared.tagline_override.is_none());

        let missing = ResumeStore::update_tagline_override(forge.conn(), "nope", Some("x"));
        assert!(matches!(missing, Err(ForgeError::NotFound { .. })));
    }

    #[test]
    fn update_latex_override_set_and_clear() {
        let forge = setup();
        let resume = create_resume(forge.conn());

        let set =
            ResumeStore::update_latex_override(forge.conn(), &resume.id, Some("\\documentclass{}"))
                .unwrap();
        assert_eq!(set.latex_override.as_deref(), Some("\\documentclass{}"));
        assert!(set.latex_override_updated_at.is_some());

        let cleared = ResumeStore::update_latex_override(forge.conn(), &resume.id, None).unwrap();
        assert!(cleared.latex_override.is_none());
        assert!(cleared.latex_override_updated_at.is_none());
    }

    // ── Gap analysis helpers ────────────────────────────────────────

    #[test]
    fn find_bullets_for_gap_excludes_covered() {
        let forge = setup();
        let source_id = create_source(forge.conn());

        // Create two approved bullets
        let b1 = BulletStore::create(
            forge.conn(),
            "Bullet A",
            None,
            None,
            Some("infra"),
            &[(source_id.clone(), true)],
            &[],
        )
        .unwrap();
        BulletStore::transition_status(
            forge.conn(),
            &b1.id,
            forge_core::BulletStatus::InReview,
            None,
        )
        .unwrap();
        BulletStore::transition_status(
            forge.conn(),
            &b1.id,
            forge_core::BulletStatus::Approved,
            None,
        )
        .unwrap();

        let b2 = BulletStore::create(
            forge.conn(),
            "Bullet B",
            None,
            None,
            Some("backend"),
            &[(source_id.clone(), true)],
            &[],
        )
        .unwrap();
        BulletStore::transition_status(
            forge.conn(),
            &b2.id,
            forge_core::BulletStatus::InReview,
            None,
        )
        .unwrap();
        BulletStore::transition_status(
            forge.conn(),
            &b2.id,
            forge_core::BulletStatus::Approved,
            None,
        )
        .unwrap();

        // Create approved perspective for b1 only
        let p = PerspectiveStore::create(
            forge.conn(),
            &CreatePerspectiveInput {
                bullet_id: b1.id.clone(),
                content: "SRE perspective".into(),
                bullet_content_snapshot: "Bullet A".into(),
                target_archetype: "sre".into(),
                domain: "infra".into(),
                framing: Framing::Responsibility,
                status: None,
                prompt_log_id: None,
            },
        )
        .unwrap();
        PerspectiveStore::transition_status(
            forge.conn(),
            &p.id,
            forge_core::PerspectiveStatus::InReview,
            None,
        )
        .unwrap();
        PerspectiveStore::transition_status(
            forge.conn(),
            &p.id,
            forge_core::PerspectiveStatus::Approved,
            None,
        )
        .unwrap();

        // b1 is covered for sre/infra, b2 is not
        let gaps = ResumeStore::find_bullets_for_gap(forge.conn(), "sre", "infra").unwrap();
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].id, b2.id);
        assert_eq!(gaps[0].content, "Bullet B");
    }

    #[test]
    fn get_source_title_for_bullet_returns_primary() {
        let forge = setup();
        let source_id = create_source(forge.conn());
        let bullet = BulletStore::create(
            forge.conn(),
            "Test",
            None,
            None,
            None,
            &[(source_id, true)],
            &[],
        )
        .unwrap();

        let title = ResumeStore::get_source_title_for_bullet(forge.conn(), &bullet.id).unwrap();
        assert_eq!(title, "Test Source");
    }

    #[test]
    fn get_source_title_for_bullet_returns_unknown() {
        let forge = setup();
        let title = ResumeStore::get_source_title_for_bullet(forge.conn(), "nonexistent").unwrap();
        assert_eq!(title, "Unknown Source");
    }

    // ── Cascade tests ───────────────────────────────────────────────

    #[test]
    fn delete_resume_cascades_sections_and_entries() {
        let forge = setup();
        let resume = create_resume(forge.conn());
        let section =
            ResumeStore::create_section(forge.conn(), &resume.id, "Experience", "experience", None)
                .unwrap();
        ResumeStore::add_entry(
            forge.conn(),
            &resume.id,
            &AddResumeEntry {
                section_id: section.id.clone(),
                perspective_id: None,
                source_id: None,
                position: None,
                content: Some("Test".into()),
            },
        )
        .unwrap();

        ResumeStore::delete(forge.conn(), &resume.id).unwrap();

        // Sections and entries should be gone
        let sections = ResumeStore::list_sections(forge.conn(), &resume.id).unwrap();
        assert!(sections.is_empty());
    }

    #[test]
    fn delete_section_cascades_entries() {
        let forge = setup();
        let resume = create_resume(forge.conn());
        let section =
            ResumeStore::create_section(forge.conn(), &resume.id, "Experience", "experience", None)
                .unwrap();
        let entry = ResumeStore::add_entry(
            forge.conn(),
            &resume.id,
            &AddResumeEntry {
                section_id: section.id.clone(),
                perspective_id: None,
                source_id: None,
                position: None,
                content: Some("Test".into()),
            },
        )
        .unwrap();

        ResumeStore::delete_section(forge.conn(), &resume.id, &section.id).unwrap();
        assert!(ResumeStore::get_entry(forge.conn(), &entry.id)
            .unwrap()
            .is_none());
    }
}
