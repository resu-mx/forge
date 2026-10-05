//! Source repository — CRUD + extension table management.
//!
//! Sources are polymorphic: the `sources` table stores common fields,
//! `source_type` discriminates among four extension tables
//! (`source_roles`, `source_projects`, `source_education`,
//! `source_presentations`). Type `general` has no extension row.

use rusqlite::{params, Connection, OptionalExtension};

use forge_core::{
    CreateSource, ForgeError, Pagination, PaginationParams, Source, SourceEducation,
    SourceExtension, SourceFilter, SourcePresentation, SourceProject, SourceRole, SourceStatus,
    SkillRow, SourceType, SourceWithExtension, UpdateSource, UpdatedBy, new_id, now_iso,
};

use super::skill::SkillStore;

pub struct SourceStore;

impl SourceStore {
    // ── Create ───────────────────────────────────────────────────────

    /// Insert a source base row + extension row. Returns the full hydrated source.
    pub fn create(
        conn: &Connection,
        input: &CreateSource,
    ) -> Result<SourceWithExtension, ForgeError> {
        if input.title.trim().is_empty() {
            return Err(ForgeError::Validation {
                message: "Title must not be empty".into(),
                field: Some("title".into()),
            });
        }
        if input.description.trim().is_empty() {
            return Err(ForgeError::Validation {
                message: "Description must not be empty".into(),
                field: Some("description".into()),
            });
        }

        let id = new_id();
        let now = now_iso();
        let source_type = input.source_type.unwrap_or(SourceType::General);

        // Base row and extension row succeed or fail together.
        let tx = conn.unchecked_transaction()?;

        tx.execute(
            "INSERT INTO sources (id, title, description, source_type, start_date, end_date, status, updated_by, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'draft', 'human', ?7, ?7)",
            params![
                id,
                input.title,
                input.description,
                source_type.as_ref(),
                input.start_date,
                input.end_date,
                now,
            ],
        )?;

        Self::insert_extension(&tx, &id, source_type, input)?;
        tx.commit()?;

        Self::get_hydrated(conn, &id)?
            .ok_or_else(|| ForgeError::Internal("Source created but not found".into()))
    }

    // ── Read ─────────────────────────────────────────────────────────

    /// Get a source by ID without extension data.
    pub fn get(conn: &Connection, id: &str) -> Result<Option<Source>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT id, title, description, source_type, start_date, end_date,
                    status, updated_by, last_derived_at, created_at, updated_at
             FROM sources WHERE id = ?1",
        )?;

        let result = stmt.query_row(params![id], Self::map_source).optional()?;
        Ok(result)
    }

    /// Get a source by ID with its extension data.
    pub fn get_hydrated(
        conn: &Connection,
        id: &str,
    ) -> Result<Option<SourceWithExtension>, ForgeError> {
        let source = match Self::get(conn, id)? {
            Some(s) => s,
            None => return Ok(None),
        };
        let extension = Self::get_extension(conn, &source.id, &source.source_type)?;
        Ok(Some(SourceWithExtension {
            base: source,
            extension,
        }))
    }

    /// List sources with optional filters and pagination.
    pub fn list(
        conn: &Connection,
        filter: &SourceFilter,
        pg: &PaginationParams,
    ) -> Result<(Vec<SourceWithExtension>, Pagination), ForgeError> {
        let offset = pg.offset.unwrap_or(0);
        let limit = pg.limit.unwrap_or(50);

        let mut conditions = Vec::new();
        let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(ref st) = filter.source_type {
            conditions.push(format!("source_type = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(st.to_string()));
        }
        if let Some(ref status) = filter.status {
            conditions.push(format!("status = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(status.to_string()));
        }
        if let Some(ref search) = filter.search {
            let param_idx = bind_values.len() + 1;
            conditions.push(format!(
                "(title LIKE ?{param_idx} OR description LIKE ?{param_idx})"
            ));
            bind_values.push(Box::new(format!("%{search}%")));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        // Count
        let count_sql = format!("SELECT COUNT(*) FROM sources {where_clause}");
        let total: i64 = conn.query_row(
            &count_sql,
            rusqlite::params_from_iter(bind_values.iter().map(|b| b.as_ref())),
            |row| row.get(0),
        )?;

        // Fetch page
        let query_sql = format!(
            "SELECT id, title, description, source_type, start_date, end_date,
                    status, updated_by, last_derived_at, created_at, updated_at
             FROM sources {where_clause}
             ORDER BY created_at DESC
             LIMIT ?{} OFFSET ?{}",
            bind_values.len() + 1,
            bind_values.len() + 2
        );
        bind_values.push(Box::new(limit));
        bind_values.push(Box::new(offset));

        let mut stmt = conn.prepare(&query_sql)?;
        let sources: Vec<Source> = stmt
            .query_map(
                rusqlite::params_from_iter(bind_values.iter().map(|b| b.as_ref())),
                Self::map_source,
            )?
            .collect::<Result<_, _>>()?;

        // Hydrate extensions
        let mut hydrated = Vec::with_capacity(sources.len());
        for source in sources {
            let extension = Self::get_extension(conn, &source.id, &source.source_type)?;
            hydrated.push(SourceWithExtension {
                base: source,
                extension,
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

    // ── Update ───────────────────────────────────────────────────────

    /// Update a source's base row and extension row.
    pub fn update(
        conn: &Connection,
        id: &str,
        input: &UpdateSource,
    ) -> Result<SourceWithExtension, ForgeError> {
        if matches!(&input.title, Some(v) if v.trim().is_empty()) {
            return Err(ForgeError::Validation {
                message: "Title must not be empty".into(),
                field: Some("title".into()),
            });
        }
        if matches!(&input.description, Some(v) if v.trim().is_empty()) {
            return Err(ForgeError::Validation {
                message: "Description must not be empty".into(),
                field: Some("description".into()),
            });
        }

        let source = Self::get(conn, id)?.ok_or_else(|| ForgeError::NotFound {
            entity_type: "source".into(),
            id: id.into(),
        })?;

        let mut sets = Vec::new();
        let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(ref v) = input.title {
            sets.push(format!("title = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = input.description {
            sets.push(format!("description = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref sd) = input.start_date {
            sets.push(format!("start_date = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(sd.clone()));
        }
        if let Some(ref ed) = input.end_date {
            sets.push(format!("end_date = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(ed.clone()));
        }

        if !sets.is_empty() {
            let now = now_iso();
            sets.push(format!("updated_at = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(now));

            let sql = format!(
                "UPDATE sources SET {} WHERE id = ?{}",
                sets.join(", "),
                bind_values.len() + 1
            );
            bind_values.push(Box::new(id.to_string()));

            conn.execute(
                &sql,
                rusqlite::params_from_iter(bind_values.iter().map(|b| b.as_ref())),
            )?;
        }

        Self::update_extension(conn, id, source.source_type, input)?;

        Self::get_hydrated(conn, id)?
            .ok_or_else(|| ForgeError::Internal("Source updated but not found".into()))
    }

    /// Patch the type-specific extension row. Only fields present in `input` are
    /// written (an explicit `null` clears the column). `source_type` is immutable,
    /// so there is no insert path here. Port of TS `buildExtensionPatch`.
    fn update_extension(
        conn: &Connection,
        id: &str,
        source_type: SourceType,
        input: &UpdateSource,
    ) -> Result<(), ForgeError> {
        let mut patch = ColumnPatch::default();
        let table = match source_type {
            SourceType::Role => {
                patch.set_opt("organization_id", &input.organization_id);
                patch.set_flag("is_current", input.is_current);
                patch.set_opt("work_arrangement", &input.work_arrangement);
                patch.set_opt("base_salary", &input.base_salary);
                patch.set_opt("total_comp_notes", &input.total_comp_notes);
                patch.set_opt("start_date", &input.start_date);
                patch.set_opt("end_date", &input.end_date);
                "source_roles"
            }
            SourceType::Project => {
                patch.set_opt("organization_id", &input.organization_id);
                patch.set_flag("is_personal", input.is_personal);
                patch.set_flag("open_source", input.open_source);
                patch.set_opt("url", &input.url);
                patch.set_opt("start_date", &input.start_date);
                patch.set_opt("end_date", &input.end_date);
                "source_projects"
            }
            SourceType::Education => {
                patch.set(
                    "education_type",
                    input.education_type.map(|e| e.to_string()),
                );
                patch.set_opt("organization_id", &input.education_organization_id);
                patch.set_opt("campus_id", &input.campus_id);
                patch.set_opt("field", &input.field);
                patch.set_flag("is_in_progress", input.is_in_progress);
                patch.set_opt("credential_id", &input.credential_id);
                patch.set_opt("expiration_date", &input.expiration_date);
                patch.set_opt("url", &input.url);
                patch.set_opt("start_date", &input.start_date);
                patch.set_opt("end_date", &input.end_date);
                patch.set_opt(
                    "degree_level",
                    &input.degree_level.map(|d| d.map(|v| v.to_string())),
                );
                patch.set_opt("degree_type", &input.degree_type);
                patch.set_opt(
                    "certificate_subtype",
                    &input.certificate_subtype.map(|c| c.map(|v| v.to_string())),
                );
                patch.set_opt("gpa", &input.gpa);
                patch.set_opt("location", &input.location);
                patch.set_opt("edu_description", &input.edu_description);
                "source_education"
            }
            SourceType::Presentation => {
                patch.set("venue", input.venue.clone());
                patch.set(
                    "presentation_type",
                    input.presentation_type.map(|p| p.to_string()),
                );
                patch.set_opt("url", &input.url);
                patch.set("coauthors", input.coauthors.clone());
                "source_presentations"
            }
            SourceType::General => return Ok(()),
        };

        if patch.is_empty() {
            return Ok(());
        }
        let sql = format!(
            "UPDATE {table} SET {} WHERE source_id = ?{}",
            patch.sets.join(", "),
            patch.binds.len() + 1
        );
        let mut binds = patch.binds;
        binds.push(Box::new(id.to_string()));
        conn.execute(
            &sql,
            rusqlite::params_from_iter(binds.iter().map(|b| b.as_ref())),
        )?;
        Ok(())
    }

    // ── Delete ───────────────────────────────────────────────────────

    /// Delete a source and its extension row (cascade via FK).
    pub fn delete(conn: &Connection, id: &str) -> Result<(), ForgeError> {
        let deleted = conn.execute("DELETE FROM sources WHERE id = ?1", params![id])?;
        if deleted == 0 {
            return Err(ForgeError::NotFound {
                entity_type: "source".into(),
                id: id.into(),
            });
        }
        Ok(())
    }

    // ── Skill links (source_skills) ──────────────────────────────────

    /// List the skills linked to a source as full rows, ordered by name.
    /// An unknown source yields an empty list (no existence check, like TS).
    pub fn list_skills(conn: &Connection, source_id: &str) -> Result<Vec<SkillRow>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT s.id, s.name, s.category, s.created_at FROM skills s \
             JOIN source_skills ss ON ss.skill_id = s.id \
             WHERE ss.source_id = ?1 ORDER BY s.name ASC",
        )?;
        let rows = stmt
            .query_map(params![source_id], SkillStore::map_skill_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Link an existing skill to a source. Idempotent. Returns `NotFound`
    /// (entity "Source" or "Skill") when either side is missing; nothing is written.
    pub fn add_skill(conn: &Connection, source_id: &str, skill_id: &str) -> Result<SkillRow, ForgeError> {
        Self::require_source(conn, source_id)?;
        let skill = SkillStore::get_row(conn, skill_id)?
            .ok_or_else(|| ForgeError::NotFound { entity_type: "Skill".into(), id: skill_id.into() })?;
        conn.execute(
            "INSERT OR IGNORE INTO source_skills (source_id, skill_id) VALUES (?1, ?2)",
            params![source_id, skill_id],
        )?;
        Ok(skill)
    }

    /// Find-or-create a skill by name (TS rules, see
    /// [`SkillStore::get_or_create_for_link`]) and link it. The source is
    /// checked first so an unknown source never leaves an orphan skill, and the
    /// create + link run in one transaction. Idempotent.
    pub fn add_skill_by_name(
        conn: &Connection,
        source_id: &str,
        name: &str,
        category: Option<&str>,
    ) -> Result<SkillRow, ForgeError> {
        Self::require_source(conn, source_id)?;
        let tx = conn.unchecked_transaction()?;
        let skill = SkillStore::get_or_create_for_link(&tx, name, category)?;
        tx.execute(
            "INSERT OR IGNORE INTO source_skills (source_id, skill_id) VALUES (?1, ?2)",
            params![source_id, skill.base.id],
        )?;
        tx.commit()?;
        Ok(skill)
    }

    /// Remove one source-skill link. `NotFound` ("Skill link") when the pair
    /// is not linked. Never deletes the skill or other sources' links.
    pub fn remove_skill(conn: &Connection, source_id: &str, skill_id: &str) -> Result<(), ForgeError> {
        let n = conn.execute(
            "DELETE FROM source_skills WHERE source_id = ?1 AND skill_id = ?2",
            params![source_id, skill_id],
        )?;
        if n == 0 {
            return Err(ForgeError::NotFound {
                entity_type: "Skill link".into(),
                id: format!("{source_id}/{skill_id}"),
            });
        }
        Ok(())
    }

    fn require_source(conn: &Connection, source_id: &str) -> Result<(), ForgeError> {
        match Self::get(conn, source_id)? {
            Some(_) => Ok(()),
            None => Err(ForgeError::NotFound { entity_type: "Source".into(), id: source_id.into() }),
        }
    }

    // ── Extension helpers ────────────────────────────────────────────

    fn get_extension(
        conn: &Connection,
        source_id: &str,
        source_type: &SourceType,
    ) -> Result<Option<SourceExtension>, ForgeError> {
        match source_type {
            SourceType::Role => {
                let mut stmt = conn.prepare(
                    "SELECT source_id, organization_id, start_date, end_date, is_current,
                            work_arrangement, base_salary, total_comp_notes
                     FROM source_roles WHERE source_id = ?1",
                )?;
                let role = stmt
                    .query_row(params![source_id], |row| {
                        Ok(SourceRole {
                            source_id: row.get(0)?,
                            organization_id: row.get(1)?,
                            start_date: row.get(2)?,
                            end_date: row.get(3)?,
                            is_current: row.get(4)?,
                            work_arrangement: row.get(5)?,
                            base_salary: row.get(6)?,
                            total_comp_notes: row.get(7)?,
                        })
                    })
                    .optional()?;
                Ok(role.map(SourceExtension::Role))
            }
            SourceType::Project => {
                let mut stmt = conn.prepare(
                    "SELECT source_id, organization_id, is_personal, open_source, url, start_date, end_date
                     FROM source_projects WHERE source_id = ?1"
                )?;
                let proj = stmt
                    .query_row(params![source_id], |row| {
                        Ok(SourceProject {
                            source_id: row.get(0)?,
                            organization_id: row.get(1)?,
                            is_personal: row.get(2)?,
                            open_source: row.get(3)?,
                            url: row.get(4)?,
                            start_date: row.get(5)?,
                            end_date: row.get(6)?,
                        })
                    })
                    .optional()?;
                Ok(proj.map(SourceExtension::Project))
            }
            SourceType::Education => {
                let mut stmt = conn.prepare(
                    "SELECT source_id, education_type, organization_id, campus_id, edu_description,
                            location, start_date, end_date, url, degree_level, degree_type,
                            field, gpa, is_in_progress, certificate_subtype, credential_id, expiration_date
                     FROM source_education WHERE source_id = ?1"
                )?;
                let edu = stmt
                    .query_row(params![source_id], |row| {
                        Ok(SourceEducation {
                            source_id: row.get(0)?,
                            education_type: row
                                .get::<_, String>(1)?
                                .parse()
                                .unwrap_or(forge_core::EducationType::Certificate),
                            organization_id: row.get(2)?,
                            campus_id: row.get(3)?,
                            edu_description: row.get(4)?,
                            location: row.get(5)?,
                            start_date: row.get(6)?,
                            end_date: row.get(7)?,
                            url: row.get(8)?,
                            degree_level: row
                                .get::<_, Option<String>>(9)?
                                .and_then(|s| s.parse().ok()),
                            degree_type: row.get(10)?,
                            field: row.get(11)?,
                            gpa: row.get(12)?,
                            is_in_progress: row.get(13)?,
                            certificate_subtype: row
                                .get::<_, Option<String>>(14)?
                                .and_then(|s| s.parse().ok()),
                            credential_id: row.get(15)?,
                            expiration_date: row.get(16)?,
                        })
                    })
                    .optional()?;
                Ok(edu.map(SourceExtension::Education))
            }
            SourceType::Presentation => {
                let mut stmt = conn.prepare(
                    "SELECT source_id, venue, presentation_type, url, coauthors
                     FROM source_presentations WHERE source_id = ?1",
                )?;
                let pres = stmt
                    .query_row(params![source_id], |row| {
                        Ok(SourcePresentation {
                            source_id: row.get(0)?,
                            venue: row.get(1)?,
                            presentation_type: row
                                .get::<_, String>(2)?
                                .parse()
                                .unwrap_or(forge_core::PresentationType::ConferenceTalk),
                            url: row.get(3)?,
                            coauthors: row.get(4)?,
                        })
                    })
                    .optional()?;
                Ok(pres.map(SourceExtension::Presentation))
            }
            SourceType::General => Ok(None),
        }
    }

    fn insert_extension(
        conn: &Connection,
        source_id: &str,
        source_type: SourceType,
        input: &CreateSource,
    ) -> Result<(), ForgeError> {
        match source_type {
            SourceType::Role => {
                conn.execute(
                    "INSERT INTO source_roles (source_id, organization_id, start_date, end_date, is_current, work_arrangement, base_salary, total_comp_notes)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        source_id,
                        input.organization_id,
                        input.start_date,
                        input.end_date,
                        input.is_current.unwrap_or(0),
                        input.work_arrangement,
                        input.base_salary,
                        input.total_comp_notes,
                    ],
                )?;
            }
            SourceType::Project => {
                conn.execute(
                    "INSERT INTO source_projects (source_id, organization_id, is_personal, open_source, url, start_date, end_date)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        source_id,
                        input.organization_id,
                        input.is_personal.unwrap_or(0),
                        input.open_source.unwrap_or(0),
                        input.url,
                        input.start_date,
                        input.end_date,
                    ],
                )?;
            }
            SourceType::Education => {
                conn.execute(
                    "INSERT INTO source_education (source_id, education_type, organization_id, campus_id, field, start_date, end_date, is_in_progress, credential_id, expiration_date, url, degree_level, degree_type, certificate_subtype, gpa, location, edu_description)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
                    params![
                        source_id,
                        input.education_type.map(|e| e.to_string()).unwrap_or_else(|| "certificate".into()),
                        input.education_organization_id,
                        input.campus_id,
                        input.field,
                        input.start_date,
                        input.end_date,
                        input.is_in_progress.unwrap_or(0),
                        input.credential_id,
                        input.expiration_date,
                        input.url,
                        input.degree_level.map(|d| d.to_string()),
                        input.degree_type,
                        input.certificate_subtype.map(|c| c.to_string()),
                        input.gpa,
                        input.location,
                        input.edu_description,
                    ],
                )?;
            }
            SourceType::Presentation => {
                conn.execute(
                    "INSERT INTO source_presentations (source_id, venue, presentation_type, url, coauthors)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        source_id,
                        input.venue,
                        input.presentation_type.map(|p| p.to_string()).unwrap_or_else(|| "conference_talk".into()),
                        input.url,
                        input.coauthors,
                    ],
                )?;
            }
            SourceType::General => {}
        }
        Ok(())
    }

    // ── Row mapping ──────────────────────────────────────────────────

    fn map_source(row: &rusqlite::Row) -> rusqlite::Result<Source> {
        Ok(Source {
            id: row.get(0)?,
            title: row.get(1)?,
            description: row.get(2)?,
            source_type: row
                .get::<_, String>(3)?
                .parse()
                .unwrap_or(SourceType::General),
            start_date: row.get(4)?,
            end_date: row.get(5)?,
            status: row
                .get::<_, String>(6)?
                .parse()
                .unwrap_or(SourceStatus::Draft),
            updated_by: row.get::<_, String>(7)?.parse().unwrap_or(UpdatedBy::Human),
            last_derived_at: row.get(8)?,
            created_at: row.get(9)?,
            updated_at: row.get(10)?,
        })
    }
}

/// Accumulates `col = ?N` assignments for a partial UPDATE.
#[derive(Default)]
struct ColumnPatch {
    sets: Vec<String>,
    binds: Vec<Box<dyn rusqlite::types::ToSql>>,
}

impl ColumnPatch {
    fn push(&mut self, col: &str, value: Box<dyn rusqlite::types::ToSql>) {
        self.sets.push(format!("{col} = ?{}", self.binds.len() + 1));
        self.binds.push(value);
    }

    /// Set when the field is present (`Some`). A present `None` writes NULL.
    fn set<T: rusqlite::types::ToSql + 'static>(&mut self, col: &str, value: Option<T>) {
        if let Some(v) = value {
            self.push(col, Box::new(v));
        }
    }

    /// For double-option fields: absent leaves the column alone, `null` clears it.
    fn set_opt<T: rusqlite::types::ToSql + Clone + 'static>(
        &mut self,
        col: &str,
        value: &Option<Option<T>>,
    ) {
        if let Some(inner) = value {
            self.push(col, Box::new(inner.clone()));
        }
    }

    /// 0/1 integer flag fields.
    fn set_flag(&mut self, col: &str, value: Option<i32>) {
        self.set(col, value);
    }

    fn is_empty(&self) -> bool {
        self.sets.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::Forge;

    fn setup() -> Forge {
        Forge::open_memory().unwrap()
    }

    fn new_source(forge: &Forge) -> String {
        SourceStore::create(forge.conn(), &CreateSource {
            title: "T".into(),
            description: "D".into(),
            ..Default::default()
        }).unwrap().base.id
    }

    fn skill_count(forge: &Forge) -> i64 {
        forge.conn().query_row("SELECT COUNT(*) FROM skills", [], |r| r.get(0)).unwrap()
    }

    fn link_count(forge: &Forge) -> i64 {
        forge.conn().query_row("SELECT COUNT(*) FROM source_skills", [], |r| r.get(0)).unwrap()
    }

    #[test]
    fn source_skills_list_is_ordered_by_name_with_created_at() {
        let forge = setup();
        let c = forge.conn();
        let sid = new_source(&forge);
        let zig = SkillStore::create(c, "Zig", None).unwrap();
        let ada = SkillStore::create(c, "Ada", None).unwrap();
        SourceStore::add_skill(c, &sid, &zig.id).unwrap();
        SourceStore::add_skill(c, &sid, &ada.id).unwrap();
        let rows = SourceStore::list_skills(c, &sid).unwrap();
        let names: Vec<_> = rows.iter().map(|r| r.base.name.as_str()).collect();
        assert_eq!(names, ["Ada", "Zig"]);
        assert!(rows.iter().all(|r| !r.created_at.is_empty()));
    }

    #[test]
    fn source_skills_list_unknown_source_is_empty() {
        let forge = setup();
        assert!(SourceStore::list_skills(forge.conn(), "missing").unwrap().is_empty());
    }

    #[test]
    fn source_skills_add_is_idempotent() {
        let forge = setup();
        let c = forge.conn();
        let sid = new_source(&forge);
        let skill = SkillStore::create(c, "Zig", None).unwrap();
        SourceStore::add_skill(c, &sid, &skill.id).unwrap();
        let again = SourceStore::add_skill(c, &sid, &skill.id).unwrap();
        assert_eq!(again.base.id, skill.id);
        assert_eq!(link_count(&forge), 1);
    }

    #[test]
    fn source_skills_add_unknown_source_or_skill_is_not_found() {
        let forge = setup();
        let c = forge.conn();
        let sid = new_source(&forge);
        let skill = SkillStore::create(c, "Zig", None).unwrap();
        assert!(matches!(SourceStore::add_skill(c, "missing", &skill.id), Err(ForgeError::NotFound { .. })));
        assert!(matches!(SourceStore::add_skill(c, &sid, "missing"), Err(ForgeError::NotFound { .. })));
        assert_eq!(link_count(&forge), 0);
    }

    #[test]
    fn source_skills_add_by_name_unknown_source_creates_nothing() {
        let forge = setup();
        let before = skill_count(&forge);
        let r = SourceStore::add_skill_by_name(forge.conn(), "missing", "Orphan check", None);
        assert!(matches!(r, Err(ForgeError::NotFound { .. })));
        assert_eq!(skill_count(&forge), before);
    }

    #[test]
    fn source_skills_add_by_name_reuses_case_insensitive_match() {
        let forge = setup();
        let c = forge.conn();
        let sid = new_source(&forge);
        let py = SkillStore::get_or_create(c, "Python", None).unwrap();
        let before = skill_count(&forge);
        let row = SourceStore::add_skill_by_name(c, &sid, "python", None).unwrap();
        assert_eq!(row.base.id, py.id);
        assert_eq!(skill_count(&forge), before);
        assert_eq!(SourceStore::list_skills(c, &sid).unwrap().len(), 1);
    }

    #[test]
    fn source_skills_remove_unlinks_only() {
        let forge = setup();
        let c = forge.conn();
        let a = new_source(&forge);
        let b = new_source(&forge);
        let skill = SkillStore::create(c, "Zig", None).unwrap();
        SourceStore::add_skill(c, &a, &skill.id).unwrap();
        SourceStore::add_skill(c, &b, &skill.id).unwrap();
        SourceStore::remove_skill(c, &a, &skill.id).unwrap();
        assert!(SourceStore::list_skills(c, &a).unwrap().is_empty());
        assert_eq!(SourceStore::list_skills(c, &b).unwrap().len(), 1);
        assert!(SkillStore::get_row(c, &skill.id).unwrap().is_some());
    }

    #[test]
    fn source_skills_remove_missing_link_is_not_found() {
        let forge = setup();
        let sid = new_source(&forge);
        let r = SourceStore::remove_skill(forge.conn(), &sid, "missing");
        assert!(matches!(r, Err(ForgeError::NotFound { .. })));
    }

    /// Reads on the creating connection see uncommitted rows, so this must check
    /// from a second connection that the create really committed.
    #[test]
    fn create_is_visible_to_another_connection() {
        let path = std::env::temp_dir().join(format!("forge-src-commit-{}.db", new_id()));
        let path_str = path.to_str().unwrap();
        let id = {
            let forge = Forge::open(path_str).unwrap();
            let created = SourceStore::create(
                forge.conn(),
                &CreateSource {
                    title: "T".into(),
                    description: "D".into(),
                    source_type: Some(SourceType::Education),
                    ..Default::default()
                },
            )
            .unwrap();
            created.base.id
        };
        let other = Forge::open(path_str).unwrap();
        let found = SourceStore::get_hydrated(other.conn(), &id).unwrap();
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path_str}{suffix}"));
        }
        assert!(found.is_some(), "source was not committed");
        assert!(
            found.unwrap().extension.is_some(),
            "extension row was not committed"
        );
    }

    #[test]
    fn empty_title_or_description_is_rejected() {
        let forge = setup();
        for (title, description) in [(" ", "d"), ("t", "  ")] {
            let r = SourceStore::create(
                forge.conn(),
                &CreateSource {
                    title: title.into(),
                    description: description.into(),
                    ..Default::default()
                },
            );
            assert!(
                matches!(r, Err(ForgeError::Validation { .. })),
                "{title:?}/{description:?}"
            );
        }
    }

    #[test]
    fn update_patches_extension_and_null_clears() {
        let forge = setup();
        let created = SourceStore::create(
            forge.conn(),
            &CreateSource {
                title: "T".into(),
                description: "D".into(),
                source_type: Some(SourceType::Education),
                field: Some("Physics".into()),
                location: Some("Pasadena".into()),
                ..Default::default()
            },
        )
        .unwrap();

        let patch: UpdateSource =
            serde_json::from_str(r#"{"location": null, "gpa": "3.9"}"#).unwrap();
        let updated = SourceStore::update(forge.conn(), &created.base.id, &patch).unwrap();
        let Some(SourceExtension::Education(edu)) = updated.extension else {
            panic!("no education ext")
        };
        assert_eq!(edu.location, None, "null clears");
        assert_eq!(edu.gpa.as_deref(), Some("3.9"));
        assert_eq!(
            edu.field.as_deref(),
            Some("Physics"),
            "absent fields are untouched"
        );
    }

    #[test]
    fn create_general_source() {
        let forge = setup();
        let input = CreateSource {
            title: "Backend Engineer at Acme".into(),
            description: "Built REST APIs and microservices".into(),
            source_type: None,
            ..Default::default()
        };
        let source = SourceStore::create(forge.conn(), &input).unwrap();
        assert_eq!(source.base.title, "Backend Engineer at Acme");
        assert_eq!(source.base.source_type, SourceType::General);
        assert_eq!(source.base.status, SourceStatus::Draft);
        assert!(source.extension.is_none());
    }

    #[test]
    fn create_role_source_with_extension() {
        let forge = setup();
        let input = CreateSource {
            title: "Senior Dev".into(),
            description: "Led team".into(),
            source_type: Some(SourceType::Role),
            is_current: Some(1),
            work_arrangement: Some("remote".into()),
            ..Default::default()
        };
        let source = SourceStore::create(forge.conn(), &input).unwrap();
        assert_eq!(source.base.source_type, SourceType::Role);
        match source.extension {
            Some(SourceExtension::Role(role)) => {
                assert_eq!(role.is_current, 1);
                assert_eq!(role.work_arrangement, Some("remote".into()));
            }
            other => panic!("Expected Role extension, got {:?}", other),
        }
    }

    #[test]
    fn get_returns_none_for_missing() {
        let forge = setup();
        let result = SourceStore::get(forge.conn(), "nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn list_empty() {
        let forge = setup();
        let (sources, pagination) = SourceStore::list(
            forge.conn(),
            &SourceFilter::default(),
            &PaginationParams::default(),
        )
        .unwrap();
        assert!(sources.is_empty());
        assert_eq!(pagination.total, 0);
    }

    #[test]
    fn list_with_type_filter() {
        let forge = setup();
        SourceStore::create(
            forge.conn(),
            &CreateSource {
                title: "Role".into(),
                description: "d".into(),
                source_type: Some(SourceType::Role),
                ..Default::default()
            },
        )
        .unwrap();
        SourceStore::create(
            forge.conn(),
            &CreateSource {
                title: "General".into(),
                description: "d".into(),
                source_type: None,
                ..Default::default()
            },
        )
        .unwrap();

        let (sources, _) = SourceStore::list(
            forge.conn(),
            &SourceFilter {
                source_type: Some(SourceType::Role),
                ..Default::default()
            },
            &PaginationParams::default(),
        )
        .unwrap();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].base.title, "Role");
    }

    #[test]
    fn update_source_title() {
        let forge = setup();
        let created = SourceStore::create(
            forge.conn(),
            &CreateSource {
                title: "Old Title".into(),
                description: "desc".into(),
                ..Default::default()
            },
        )
        .unwrap();

        let updated = SourceStore::update(
            forge.conn(),
            &created.base.id,
            &UpdateSource {
                title: Some("New Title".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(updated.base.title, "New Title");
    }

    #[test]
    fn delete_source() {
        let forge = setup();
        let created = SourceStore::create(
            forge.conn(),
            &CreateSource {
                title: "To Delete".into(),
                description: "desc".into(),
                ..Default::default()
            },
        )
        .unwrap();

        SourceStore::delete(forge.conn(), &created.base.id).unwrap();
        assert!(SourceStore::get(forge.conn(), &created.base.id)
            .unwrap()
            .is_none());
    }

    #[test]
    fn delete_missing_returns_not_found() {
        let forge = setup();
        let result = SourceStore::delete(forge.conn(), "nonexistent");
        assert!(matches!(result, Err(ForgeError::NotFound { .. })));
    }

    #[test]
    fn search_filter() {
        let forge = setup();
        SourceStore::create(
            forge.conn(),
            &CreateSource {
                title: "Rust Developer".into(),
                description: "Systems programming".into(),
                ..Default::default()
            },
        )
        .unwrap();
        SourceStore::create(
            forge.conn(),
            &CreateSource {
                title: "Python Dev".into(),
                description: "Data science".into(),
                ..Default::default()
            },
        )
        .unwrap();

        let (sources, _) = SourceStore::list(
            forge.conn(),
            &SourceFilter {
                search: Some("rust".into()),
                ..Default::default()
            },
            &PaginationParams::default(),
        )
        .unwrap();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].base.title, "Rust Developer");
    }
}
