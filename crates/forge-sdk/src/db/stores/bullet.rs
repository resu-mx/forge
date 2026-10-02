//! Bullet repository — CRUD + junction tables (bullet_sources, bullet_skills).
//!
//! Bullets are derived from sources. The `technologies` field is a projection
//! of linked skill names via the `bullet_skills` junction table.

use rusqlite::{params, Connection, OptionalExtension};

use forge_core::{
    Bullet, BulletFilter, BulletStatus, ForgeError, Pagination, PaginationParams, Skill,
    SkillCategory, Source, UpdateBulletInput, new_id, now_iso,
};

use super::source::SourceStore;

/// Valid status transitions for bullets.
fn valid_transitions(from: &BulletStatus) -> &'static [BulletStatus] {
    match from {
        BulletStatus::Draft => &[BulletStatus::InReview],
        BulletStatus::InReview => &[BulletStatus::Approved, BulletStatus::Rejected],
        BulletStatus::Rejected => &[BulletStatus::InReview],
        BulletStatus::Approved => &[BulletStatus::Archived],
        BulletStatus::Archived => &[BulletStatus::Draft],
    }
}

pub struct BulletStore;

impl BulletStore {
    // ── Create ───────────────────────────────────────────────────────

    /// Create a bullet, optionally linking to sources and technologies.
    pub fn create(
        conn: &Connection,
        content: &str,
        source_content_snapshot: Option<&str>,
        metrics: Option<&str>,
        domain: Option<&str>,
        source_ids: &[(String, bool)], // (source_id, is_primary)
        technologies: &[String],
    ) -> Result<Bullet, ForgeError> {
        let id = new_id();
        let now = now_iso();
        let snapshot = source_content_snapshot.unwrap_or(content);

        conn.execute(
            "INSERT INTO bullets (id, content, source_content_snapshot, metrics, domain, status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'draft', ?6)",
            params![id, content, snapshot, metrics, domain, now],
        )?;

        // Link sources
        for (source_id, is_primary) in source_ids {
            conn.execute(
                "INSERT INTO bullet_sources (bullet_id, source_id, is_primary)
                 VALUES (?1, ?2, ?3)",
                params![id, source_id, *is_primary as i32],
            )?;
        }

        // Link technologies (find-or-create skills, then junction)
        Self::replace_technologies(conn, &id, technologies)?;

        Self::get_hydrated(conn, &id)?
            .ok_or_else(|| ForgeError::Internal("Bullet created but not found".into()))
    }

    // ── Read ─────────────────────────────────────────────────────────

    /// Get a bullet by ID with hydrated technologies.
    pub fn get_hydrated(conn: &Connection, id: &str) -> Result<Option<Bullet>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT id, content, source_content_snapshot, metrics, domain, status,
                    rejection_reason, prompt_log_id, approved_at, approved_by, created_at
             FROM bullets WHERE id = ?1",
        )?;

        let bullet = stmt.query_row(params![id], |row| {
            Ok(Bullet {
                id: row.get(0)?,
                content: row.get(1)?,
                source_content_snapshot: row.get(2)?,
                technologies: Vec::new(), // hydrated below
                metrics: row.get(3)?,
                domain: row.get(4)?,
                status: row.get::<_, String>(5)?.parse().unwrap_or(BulletStatus::Draft),
                rejection_reason: row.get(6)?,
                prompt_log_id: row.get(7)?,
                approved_at: row.get(8)?,
                approved_by: row.get(9)?,
                created_at: row.get(10)?,
            })
        }).optional()?;

        match bullet {
            Some(mut b) => {
                b.technologies = Self::get_technologies(conn, &b.id)?;
                Ok(Some(b))
            }
            None => Ok(None),
        }
    }

    /// List bullets with optional filters and pagination.
    pub fn list(
        conn: &Connection,
        filter: &BulletFilter,
        pg: &PaginationParams,
    ) -> Result<(Vec<Bullet>, Pagination), ForgeError> {
        let offset = pg.offset.unwrap_or(0);
        let limit = pg.limit.unwrap_or(50);

        let mut conditions = Vec::new();
        let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(ref status) = filter.status {
            conditions.push(format!("status = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(status.clone()));
        }
        if let Some(ref domain) = filter.domain {
            conditions.push(format!("domain = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(domain.clone()));
        }
        if let Some(ref source_id) = filter.source_id {
            conditions.push(format!(
                "id IN (SELECT bullet_id FROM bullet_sources WHERE source_id = ?{})",
                bind_values.len() + 1
            ));
            bind_values.push(Box::new(source_id.clone()));
        }
        if let Some(ref tech) = filter.technology {
            conditions.push(format!(
                "id IN (SELECT bs.bullet_id FROM bullet_skills bs JOIN skills s ON bs.skill_id = s.id WHERE LOWER(s.name) = LOWER(?{}))",
                bind_values.len() + 1
            ));
            bind_values.push(Box::new(tech.clone()));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let count_sql = format!("SELECT COUNT(*) FROM bullets {where_clause}");
        let total: i64 = conn.query_row(
            &count_sql,
            rusqlite::params_from_iter(bind_values.iter().map(|b| b.as_ref())),
            |row| row.get(0),
        )?;

        let query_sql = format!(
            "SELECT id, content, source_content_snapshot, metrics, domain, status,
                    rejection_reason, prompt_log_id, approved_at, approved_by, created_at
             FROM bullets {where_clause}
             ORDER BY created_at DESC
             LIMIT ?{} OFFSET ?{}",
            bind_values.len() + 1,
            bind_values.len() + 2
        );
        bind_values.push(Box::new(limit));
        bind_values.push(Box::new(offset));

        let mut stmt = conn.prepare(&query_sql)?;
        let rows: Vec<Bullet> = stmt
            .query_map(
                rusqlite::params_from_iter(bind_values.iter().map(|b| b.as_ref())),
                |row| {
                    Ok(Bullet {
                        id: row.get(0)?,
                        content: row.get(1)?,
                        source_content_snapshot: row.get(2)?,
                        technologies: Vec::new(),
                        metrics: row.get(3)?,
                        domain: row.get(4)?,
                        status: row.get::<_, String>(5)?.parse().unwrap_or(BulletStatus::Draft),
                        rejection_reason: row.get(6)?,
                        prompt_log_id: row.get(7)?,
                        approved_at: row.get(8)?,
                        approved_by: row.get(9)?,
                        created_at: row.get(10)?,
                    })
                },
            )?
            .collect::<Result<_, _>>()?;

        let mut hydrated = Vec::with_capacity(rows.len());
        for mut b in rows {
            b.technologies = Self::get_technologies(conn, &b.id)?;
            hydrated.push(b);
        }

        Ok((hydrated, Pagination { total, offset, limit }))
    }

    // ── Update ───────────────────────────────────────────────────────

    /// Update bullet content/metrics/domain fields.
    pub fn update(conn: &Connection, id: &str, input: &UpdateBulletInput) -> Result<Bullet, ForgeError> {
        // Verify exists
        Self::get_hydrated(conn, id)?
            .ok_or_else(|| ForgeError::NotFound { entity_type: "bullet".into(), id: id.into() })?;

        let mut sets = Vec::new();
        let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(ref v) = input.content {
            sets.push(format!("content = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = input.metrics {
            sets.push(format!("metrics = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = input.domain {
            sets.push(format!("domain = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }

        if !sets.is_empty() {
            let sql = format!(
                "UPDATE bullets SET {} WHERE id = ?{}",
                sets.join(", "),
                bind_values.len() + 1
            );
            bind_values.push(Box::new(id.to_string()));
            conn.execute(&sql, rusqlite::params_from_iter(bind_values.iter().map(|b| b.as_ref())))?;
        }

        if let Some(ref techs) = input.technologies {
            Self::replace_technologies(conn, id, techs)?;
        }

        Self::get_hydrated(conn, id)?
            .ok_or_else(|| ForgeError::Internal("Bullet updated but not found".into()))
    }

    /// Transition bullet status with validation.
    pub fn transition_status(
        conn: &Connection,
        id: &str,
        new_status: BulletStatus,
        rejection_reason: Option<&str>,
    ) -> Result<Bullet, ForgeError> {
        let bullet = Self::get_hydrated(conn, id)?
            .ok_or_else(|| ForgeError::NotFound { entity_type: "bullet".into(), id: id.into() })?;

        if new_status == BulletStatus::Rejected
            && rejection_reason.map_or(true, |r| r.trim().is_empty())
        {
            return Err(ForgeError::Validation {
                message: "Rejection reason must not be empty".into(),
                field: Some("rejection_reason".into()),
            });
        }

        let allowed = valid_transitions(&bullet.status);
        if !allowed.contains(&new_status) {
            return Err(ForgeError::Validation {
                message: format!(
                    "Cannot transition from '{}' to '{}'",
                    bullet.status, new_status
                ),
                field: Some("status".into()),
            });
        }

        let now = now_iso();
        match new_status {
            BulletStatus::Approved => {
                conn.execute(
                    "UPDATE bullets SET status = ?1, approved_at = ?2, approved_by = 'human', rejection_reason = NULL WHERE id = ?3",
                    params![new_status.as_ref(), now, id],
                )?;
            }
            BulletStatus::Rejected => {
                conn.execute(
                    "UPDATE bullets SET status = ?1, rejection_reason = ?2 WHERE id = ?3",
                    params![new_status.as_ref(), rejection_reason, id],
                )?;
            }
            _ => {
                conn.execute(
                    "UPDATE bullets SET status = ?1, rejection_reason = NULL WHERE id = ?2",
                    params![new_status.as_ref(), id],
                )?;
            }
        }

        Self::get_hydrated(conn, id)?
            .ok_or_else(|| ForgeError::Internal("Bullet transitioned but not found".into()))
    }

    // ── Delete ───────────────────────────────────────────────────────

    /// Submit a draft for review. Unlike `transition_status` this accepts only
    /// drafts: a rejected bullet is sent back with `reopen`, not `submit`.
    pub fn submit(conn: &Connection, id: &str) -> Result<Bullet, ForgeError> {
        let bullet = Self::get_hydrated(conn, id)?
            .ok_or_else(|| ForgeError::NotFound { entity_type: "bullet".into(), id: id.into() })?;
        if bullet.status != BulletStatus::Draft {
            return Err(ForgeError::Validation {
                message: "Only draft bullets can be submitted for review".into(),
                field: Some("status".into()),
            });
        }
        Self::transition_status(conn, id, BulletStatus::InReview, None)
    }

    /// Delete a bullet (cascades to junction tables via FK). Refused while
    /// perspectives still derive from it.
    pub fn delete(conn: &Connection, id: &str) -> Result<(), ForgeError> {
        let perspectives: i64 = conn.query_row(
            "SELECT COUNT(*) FROM perspectives WHERE bullet_id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        if perspectives > 0 {
            return Err(ForgeError::Conflict {
                message: "Cannot delete bullet with existing perspectives".into(),
            });
        }
        let deleted = conn.execute("DELETE FROM bullets WHERE id = ?1", params![id])?;
        if deleted == 0 {
            return Err(ForgeError::NotFound { entity_type: "bullet".into(), id: id.into() });
        }
        Ok(())
    }

    // ── Skill links ──────────────────────────────────────────────────

    /// Skills linked to a bullet, by name.
    pub fn list_skills(conn: &Connection, bullet_id: &str) -> Result<Vec<Skill>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT s.id, s.name, s.category FROM skills s
             JOIN bullet_skills bs ON bs.skill_id = s.id
             WHERE bs.bullet_id = ?1
             ORDER BY s.name ASC",
        )?;
        let rows = stmt
            .query_map(params![bullet_id], Self::map_skill)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Link an existing skill. Linking twice is a no-op.
    pub fn link_skill(conn: &Connection, bullet_id: &str, skill_id: &str) -> Result<Skill, ForgeError> {
        match conn.execute(
            "INSERT OR IGNORE INTO bullet_skills (bullet_id, skill_id) VALUES (?1, ?2)",
            params![bullet_id, skill_id],
        ) {
            Ok(_) => {}
            Err(e) => {
                return Err(match ForgeError::from(e) {
                    ForgeError::ForeignKey { .. } => ForgeError::NotFound {
                        entity_type: "bullet or skill".into(),
                        id: format!("{bullet_id}/{skill_id}"),
                    },
                    other => other,
                })
            }
        }
        Self::skill_by_id(conn, skill_id)?.ok_or_else(|| ForgeError::NotFound {
            entity_type: "skill".into(),
            id: skill_id.into(),
        })
    }

    /// Link a skill by name, creating it if needed. The first letter is upper-cased
    /// (rest preserved: `foo` -> `Foo`, `SAFe` stays `SAFe`) and the lookup is
    /// case-insensitive, so `python` links an existing `Python`. An unknown or
    /// absent category falls back to `other`.
    pub fn link_skill_by_name(
        conn: &Connection,
        bullet_id: &str,
        name: &str,
        category: Option<&str>,
    ) -> Result<Skill, ForgeError> {
        let raw = name.trim();
        let mut chars = raw.chars();
        let name: String = match chars.next() {
            Some(first) => first.to_uppercase().chain(chars).collect(),
            None => {
                return Err(ForgeError::Validation {
                    message: "skill_id or name is required".into(),
                    field: None,
                })
            }
        };

        let existing = conn
            .query_row(
                "SELECT id, name, category FROM skills WHERE name = ?1 COLLATE NOCASE",
                params![name],
                Self::map_skill,
            )
            .optional()?;
        let skill = match existing {
            Some(skill) => skill,
            None => {
                let category = category
                    .and_then(|c| c.parse::<SkillCategory>().ok())
                    .unwrap_or(SkillCategory::Other);
                let id = new_id();
                conn.execute(
                    "INSERT INTO skills (id, name, category) VALUES (?1, ?2, ?3)",
                    params![id, name, category.as_ref()],
                )?;
                Self::skill_by_id(conn, &id)?
                    .ok_or_else(|| ForgeError::Internal("Skill created but not found".into()))?
            }
        };

        Self::link_skill(conn, bullet_id, &skill.id)
    }

    /// Remove a skill link. 404 if the link does not exist.
    pub fn unlink_skill(conn: &Connection, bullet_id: &str, skill_id: &str) -> Result<(), ForgeError> {
        let deleted = conn.execute(
            "DELETE FROM bullet_skills WHERE bullet_id = ?1 AND skill_id = ?2",
            params![bullet_id, skill_id],
        )?;
        if deleted == 0 {
            return Err(ForgeError::NotFound {
                entity_type: "skill link".into(),
                id: format!("{bullet_id}/{skill_id}"),
            });
        }
        Ok(())
    }

    fn skill_by_id(conn: &Connection, id: &str) -> Result<Option<Skill>, ForgeError> {
        Ok(conn
            .query_row(
                "SELECT id, name, category FROM skills WHERE id = ?1",
                params![id],
                Self::map_skill,
            )
            .optional()?)
    }

    fn map_skill(row: &rusqlite::Row) -> rusqlite::Result<Skill> {
        Ok(Skill {
            id: row.get(0)?,
            name: row.get(1)?,
            category: row.get::<_, String>(2)?.parse().unwrap_or(SkillCategory::Other),
        })
    }

    // ── Source links ─────────────────────────────────────────────────

    /// Sources a bullet was derived from, primary first, with the `is_primary` flag.
    pub fn list_sources(conn: &Connection, bullet_id: &str) -> Result<Vec<(Source, i32)>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT bs.source_id, bs.is_primary
             FROM bullet_sources bs
             JOIN sources s ON bs.source_id = s.id
             WHERE bs.bullet_id = ?1
             ORDER BY bs.is_primary DESC, s.title ASC",
        )?;
        let links = stmt
            .query_map(params![bullet_id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;

        let mut out = Vec::with_capacity(links.len());
        for (source_id, is_primary) in links {
            if let Some(source) = SourceStore::get(conn, &source_id)? {
                out.push((source, is_primary));
            }
        }
        Ok(out)
    }

    // ── Technology helpers ────────────────────────────────────────────

    /// Get technology names for a bullet (via bullet_skills → skills join).
    fn get_technologies(conn: &Connection, bullet_id: &str) -> Result<Vec<String>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT s.name FROM bullet_skills bs
             JOIN skills s ON bs.skill_id = s.id
             WHERE bs.bullet_id = ?1
             ORDER BY s.name",
        )?;
        let names: Vec<String> = stmt
            .query_map(params![bullet_id], |row| row.get(0))?
            .collect::<Result<_, _>>()?;
        Ok(names)
    }

    /// Replace all technologies for a bullet (clear + re-insert).
    fn replace_technologies(conn: &Connection, bullet_id: &str, technologies: &[String]) -> Result<(), ForgeError> {
        conn.execute("DELETE FROM bullet_skills WHERE bullet_id = ?1", params![bullet_id])?;

        for tech in technologies {
            let trimmed = tech.trim().to_lowercase();
            if trimmed.is_empty() {
                continue;
            }

            // Find or create skill
            let skill_id: String = match conn.query_row(
                "SELECT id FROM skills WHERE LOWER(name) = LOWER(?1)",
                params![trimmed],
                |row| row.get(0),
            ) {
                Ok(id) => id,
                Err(rusqlite::Error::QueryReturnedNoRows) => {
                    let new_skill_id = forge_core::new_id();
                    conn.execute(
                        "INSERT INTO skills (id, name, category) VALUES (?1, ?2, 'tool')",
                        params![new_skill_id, trimmed],
                    )?;
                    new_skill_id
                }
                Err(e) => return Err(e.into()),
            };

            conn.execute(
                "INSERT OR IGNORE INTO bullet_skills (bullet_id, skill_id) VALUES (?1, ?2)",
                params![bullet_id, skill_id],
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::stores::source::SourceStore;
    use crate::forge::Forge;
    use forge_core::{CreateSource, SourceType};

    fn setup() -> Forge {
        Forge::open_memory().unwrap()
    }

    fn create_source(conn: &Connection) -> String {
        let src = SourceStore::create(conn, &CreateSource {
            title: "Test Source".into(),
            description: "Test description".into(),
            source_type: Some(SourceType::General),
            ..Default::default()
        }).unwrap();
        src.base.id
    }

    #[test]
    fn create_bullet_basic() {
        let forge = setup();
        let source_id = create_source(forge.conn());
        let bullet = BulletStore::create(
            forge.conn(),
            "Built high-performance REST APIs",
            None,
            Some("reduced latency by 40%"),
            Some("backend"),
            &[(source_id, true)],
            &["rust".into(), "axum".into()],
        ).unwrap();

        assert_eq!(bullet.content, "Built high-performance REST APIs");
        assert_eq!(bullet.status, BulletStatus::Draft);
        assert_eq!(bullet.metrics, Some("reduced latency by 40%".into()));
        assert_eq!(bullet.domain, Some("backend".into()));
        assert_eq!(bullet.technologies, vec!["axum", "rust"]);
    }

    #[test]
    fn get_returns_none_for_missing() {
        let forge = setup();
        assert!(BulletStore::get_hydrated(forge.conn(), "nonexistent").unwrap().is_none());
    }

    #[test]
    fn list_with_source_filter() {
        let forge = setup();
        let s1 = create_source(forge.conn());
        let s2 = create_source(forge.conn());

        BulletStore::create(forge.conn(), "Bullet A", None, None, None, &[(s1.clone(), true)], &[]).unwrap();
        BulletStore::create(forge.conn(), "Bullet B", None, None, None, &[(s2, true)], &[]).unwrap();

        let (bullets, _) = BulletStore::list(
            forge.conn(),
            &BulletFilter { source_id: Some(s1), ..Default::default() },
            &PaginationParams::default(),
        ).unwrap();
        assert_eq!(bullets.len(), 1);
        assert_eq!(bullets[0].content, "Bullet A");
    }

    #[test]
    fn status_transitions() {
        let forge = setup();
        let source_id = create_source(forge.conn());
        let bullet = BulletStore::create(
            forge.conn(), "Test bullet", None, None, None, &[(source_id, true)], &[],
        ).unwrap();

        // draft → in_review
        let b = BulletStore::transition_status(forge.conn(), &bullet.id, BulletStatus::InReview, None).unwrap();
        assert_eq!(b.status, BulletStatus::InReview);

        // in_review → approved
        let b = BulletStore::transition_status(forge.conn(), &b.id, BulletStatus::Approved, None).unwrap();
        assert_eq!(b.status, BulletStatus::Approved);
        assert!(b.approved_at.is_some());

        // approved → draft (invalid)
        let result = BulletStore::transition_status(forge.conn(), &b.id, BulletStatus::Draft, None);
        assert!(result.is_err());
    }

    #[test]
    fn reject_with_reason() {
        let forge = setup();
        let source_id = create_source(forge.conn());
        let bullet = BulletStore::create(
            forge.conn(), "Test", None, None, None, &[(source_id, true)], &[],
        ).unwrap();

        BulletStore::transition_status(forge.conn(), &bullet.id, BulletStatus::InReview, None).unwrap();
        let b = BulletStore::transition_status(
            forge.conn(), &bullet.id, BulletStatus::Rejected, Some("Too vague"),
        ).unwrap();
        assert_eq!(b.status, BulletStatus::Rejected);
        assert_eq!(b.rejection_reason, Some("Too vague".into()));
    }

    #[test]
    fn update_technologies() {
        let forge = setup();
        let source_id = create_source(forge.conn());
        let bullet = BulletStore::create(
            forge.conn(), "Test", None, None, None, &[(source_id, true)], &["python".into()],
        ).unwrap();
        assert_eq!(bullet.technologies, vec!["python"]);

        let updated = BulletStore::update(forge.conn(), &bullet.id, &UpdateBulletInput {
            technologies: Some(vec!["rust".into(), "go".into()]),
            ..Default::default()
        }).unwrap();
        assert_eq!(updated.technologies, vec!["go", "rust"]);
    }

    #[test]
    fn delete_bullet() {
        let forge = setup();
        let source_id = create_source(forge.conn());
        let bullet = BulletStore::create(
            forge.conn(), "To delete", None, None, None, &[(source_id, true)], &[],
        ).unwrap();

        BulletStore::delete(forge.conn(), &bullet.id).unwrap();
        assert!(BulletStore::get_hydrated(forge.conn(), &bullet.id).unwrap().is_none());
    }
}
