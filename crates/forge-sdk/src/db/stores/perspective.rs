//! Perspective repository — CRUD + chain tracing + status transitions.
//!
//! Perspectives reframe bullets for specific archetypes/domains.
//! Each perspective links back to a bullet (and transitively to a source).

use rusqlite::{params, Connection, OptionalExtension};

use forge_core::{
    Bullet, BulletStatus, CreatePerspectiveInput, ForgeError, Framing, Pagination,
    PaginationParams, Perspective, PerspectiveFilter, PerspectiveStatus,
    PerspectiveWithChain, Source, SourceStatus, SourceType, UpdatePerspectiveInput,
    UpdatedBy, new_id, now_iso,
};

/// Valid status transitions for perspectives.
fn valid_transitions(from: &PerspectiveStatus) -> &'static [PerspectiveStatus] {
    match from {
        PerspectiveStatus::Draft => &[PerspectiveStatus::InReview],
        PerspectiveStatus::InReview => &[PerspectiveStatus::Approved, PerspectiveStatus::Rejected],
        PerspectiveStatus::Rejected => &[PerspectiveStatus::InReview],
        PerspectiveStatus::Approved => &[PerspectiveStatus::Archived],
        PerspectiveStatus::Archived => &[PerspectiveStatus::Draft],
    }
}

pub struct PerspectiveStore;

impl PerspectiveStore {
    // ── Create ───────────────────────────────────────────────────────

    pub fn create(conn: &Connection, input: &CreatePerspectiveInput) -> Result<Perspective, ForgeError> {
        let id = new_id();
        let now = now_iso();
        let status = input.status.unwrap_or(PerspectiveStatus::Draft);

        conn.execute(
            "INSERT INTO perspectives (id, bullet_id, content, bullet_content_snapshot,
             target_archetype, domain, framing, status, prompt_log_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                id,
                input.bullet_id,
                input.content,
                input.bullet_content_snapshot,
                input.target_archetype,
                input.domain,
                input.framing.as_ref(),
                status.as_ref(),
                input.prompt_log_id,
                now,
            ],
        )?;

        Self::get(conn, &id)?
            .ok_or_else(|| ForgeError::Internal("Perspective created but not found".into()))
    }

    /// Create a perspective by hand (not via a derivation): the snapshot is taken from
    /// the bullet, framing defaults to `accomplishment`, and `auto_approve` (the TS
    /// default) approves it immediately as `approved_by = 'direct'`.
    pub fn create_direct(
        conn: &Connection,
        bullet_id: &str,
        content: &str,
        target_archetype: Option<&str>,
        domain: Option<&str>,
        framing: Option<Framing>,
        auto_approve: bool,
    ) -> Result<Perspective, ForgeError> {
        if content.trim().is_empty() {
            return Err(ForgeError::Validation {
                message: "Content must not be empty".into(),
                field: Some("content".into()),
            });
        }
        let snapshot: String = conn
            .query_row("SELECT content FROM bullets WHERE id = ?1", params![bullet_id], |row| row.get(0))
            .optional()?
            .ok_or_else(|| ForgeError::NotFound { entity_type: "bullet".into(), id: bullet_id.into() })?;

        let id = new_id();
        let now = now_iso();
        let (status, approved_at, approved_by) = if auto_approve {
            (PerspectiveStatus::Approved, Some(now.clone()), Some("direct"))
        } else {
            (PerspectiveStatus::Draft, None, None)
        };

        conn.execute(
            "INSERT INTO perspectives (id, bullet_id, content, bullet_content_snapshot,
             target_archetype, domain, framing, status, approved_at, approved_by, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                id,
                bullet_id,
                content.trim(),
                snapshot,
                target_archetype,
                domain,
                framing.unwrap_or(Framing::Accomplishment).as_ref(),
                status.as_ref(),
                approved_at,
                approved_by,
                now,
            ],
        )?;

        Self::get(conn, &id)?
            .ok_or_else(|| ForgeError::Internal("Perspective created but not found".into()))
    }

    // ── Read ─────────────────────────────────────────────────────────

    pub fn get(conn: &Connection, id: &str) -> Result<Option<Perspective>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT id, bullet_id, content, bullet_content_snapshot, target_archetype,
                    domain, framing, status, rejection_reason, prompt_log_id,
                    approved_at, approved_by, created_at
             FROM perspectives WHERE id = ?1",
        )?;
        let result = stmt.query_row(params![id], Self::map_perspective).optional()?;
        Ok(result)
    }

    /// Get perspective with its full derivation chain (bullet + source).
    pub fn get_with_chain(conn: &Connection, id: &str) -> Result<Option<PerspectiveWithChain>, ForgeError> {
        let perspective = match Self::get(conn, id)? {
            Some(p) => p,
            None => return Ok(None),
        };

        let bullet = conn.query_row(
            "SELECT id, content, source_content_snapshot, metrics, domain, status,
                    rejection_reason, prompt_log_id, approved_at, approved_by, created_at
             FROM bullets WHERE id = ?1",
            params![perspective.bullet_id],
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
        )?;

        // Get primary source via bullet_sources junction
        let source = conn.query_row(
            "SELECT s.id, s.title, s.description, s.source_type, s.start_date, s.end_date,
                    s.status, s.updated_by, s.last_derived_at, s.created_at, s.updated_at
             FROM sources s
             JOIN bullet_sources bs ON s.id = bs.source_id
             WHERE bs.bullet_id = ?1
             ORDER BY bs.is_primary DESC
             LIMIT 1",
            params![bullet.id],
            |row| {
                Ok(Source {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    description: row.get(2)?,
                    source_type: row.get::<_, String>(3)?.parse().unwrap_or(SourceType::General),
                    start_date: row.get(4)?,
                    end_date: row.get(5)?,
                    status: row.get::<_, String>(6)?.parse().unwrap_or(SourceStatus::Draft),
                    updated_by: row.get::<_, String>(7)?.parse().unwrap_or(UpdatedBy::Human),
                    last_derived_at: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            },
        )?;

        Ok(Some(PerspectiveWithChain {
            base: perspective,
            bullet,
            source,
        }))
    }

    /// List perspectives with filters and pagination.
    pub fn list(
        conn: &Connection,
        filter: &PerspectiveFilter,
        pg: &PaginationParams,
    ) -> Result<(Vec<Perspective>, Pagination), ForgeError> {
        let offset = pg.offset.unwrap_or(0);
        let limit = pg.limit.unwrap_or(50);

        let mut conditions = Vec::new();
        let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(ref v) = filter.bullet_id {
            conditions.push(format!("bullet_id = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = filter.target_archetype {
            conditions.push(format!("target_archetype = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = filter.domain {
            conditions.push(format!("domain = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = filter.framing {
            conditions.push(format!("framing = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.to_string()));
        }
        if let Some(ref v) = filter.status {
            conditions.push(format!("status = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.to_string()));
        }
        if let Some(ref v) = filter.source_id {
            conditions.push(format!(
                "bullet_id IN (SELECT bullet_id FROM bullet_sources WHERE source_id = ?{})",
                bind_values.len() + 1
            ));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = filter.search {
            let idx = bind_values.len() + 1;
            conditions.push(format!("content LIKE ?{idx}"));
            bind_values.push(Box::new(format!("%{v}%")));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let total: i64 = conn.query_row(
            &format!("SELECT COUNT(*) FROM perspectives {where_clause}"),
            rusqlite::params_from_iter(bind_values.iter().map(|b| b.as_ref())),
            |row| row.get(0),
        )?;

        let query_sql = format!(
            "SELECT id, bullet_id, content, bullet_content_snapshot, target_archetype,
                    domain, framing, status, rejection_reason, prompt_log_id,
                    approved_at, approved_by, created_at
             FROM perspectives {where_clause}
             ORDER BY created_at DESC
             LIMIT ?{} OFFSET ?{}",
            bind_values.len() + 1,
            bind_values.len() + 2
        );
        bind_values.push(Box::new(limit));
        bind_values.push(Box::new(offset));

        let mut stmt = conn.prepare(&query_sql)?;
        let perspectives: Vec<Perspective> = stmt
            .query_map(
                rusqlite::params_from_iter(bind_values.iter().map(|b| b.as_ref())),
                Self::map_perspective,
            )?
            .collect::<Result<_, _>>()?;

        Ok((perspectives, Pagination { total, offset, limit }))
    }

    // ── Update ───────────────────────────────────────────────────────

    pub fn update(conn: &Connection, id: &str, input: &UpdatePerspectiveInput) -> Result<Perspective, ForgeError> {
        Self::get(conn, id)?
            .ok_or_else(|| ForgeError::NotFound { entity_type: "perspective".into(), id: id.into() })?;

        let mut sets = Vec::new();
        let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(ref v) = input.content {
            sets.push(format!("content = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = input.target_archetype {
            sets.push(format!("target_archetype = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = input.domain {
            sets.push(format!("domain = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = input.framing {
            sets.push(format!("framing = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.to_string()));
        }

        if !sets.is_empty() {
            let sql = format!(
                "UPDATE perspectives SET {} WHERE id = ?{}",
                sets.join(", "),
                bind_values.len() + 1
            );
            bind_values.push(Box::new(id.to_string()));
            conn.execute(&sql, rusqlite::params_from_iter(bind_values.iter().map(|b| b.as_ref())))?;
        }

        Self::get(conn, id)?
            .ok_or_else(|| ForgeError::Internal("Perspective updated but not found".into()))
    }

    /// Transition perspective status with validation.
    pub fn transition_status(
        conn: &Connection,
        id: &str,
        new_status: PerspectiveStatus,
        rejection_reason: Option<&str>,
    ) -> Result<Perspective, ForgeError> {
        let p = Self::get(conn, id)?
            .ok_or_else(|| ForgeError::NotFound { entity_type: "perspective".into(), id: id.into() })?;

        if new_status == PerspectiveStatus::Rejected
            && rejection_reason.map_or(true, |r| r.trim().is_empty())
        {
            return Err(ForgeError::Validation {
                message: "Rejection reason must not be empty".into(),
                field: Some("rejection_reason".into()),
            });
        }

        let allowed = valid_transitions(&p.status);
        if !allowed.contains(&new_status) {
            return Err(ForgeError::Validation {
                message: format!("Cannot transition from '{}' to '{}'", p.status, new_status),
                field: Some("status".into()),
            });
        }

        let now = now_iso();
        match new_status {
            PerspectiveStatus::Approved => {
                conn.execute(
                    "UPDATE perspectives SET status = ?1, approved_at = ?2, approved_by = 'human', rejection_reason = NULL WHERE id = ?3",
                    params![new_status.as_ref(), now, id],
                )?;
            }
            PerspectiveStatus::Rejected => {
                conn.execute(
                    "UPDATE perspectives SET status = ?1, rejection_reason = ?2 WHERE id = ?3",
                    params![new_status.as_ref(), rejection_reason, id],
                )?;
            }
            _ => {
                conn.execute(
                    "UPDATE perspectives SET status = ?1, rejection_reason = NULL WHERE id = ?2",
                    params![new_status.as_ref(), id],
                )?;
            }
        }

        Self::get(conn, id)?
            .ok_or_else(|| ForgeError::Internal("Perspective transitioned but not found".into()))
    }

    // ── Delete ───────────────────────────────────────────────────────

    /// Delete a perspective. Refused while a resume entry uses it.
    pub fn delete(conn: &Connection, id: &str) -> Result<(), ForgeError> {
        let in_resume: i64 = conn.query_row(
            "SELECT COUNT(*) FROM resume_entries WHERE perspective_id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        if in_resume > 0 {
            return Err(ForgeError::Conflict {
                message: "Cannot delete perspective that is in a resume".into(),
            });
        }
        let deleted = conn.execute("DELETE FROM perspectives WHERE id = ?1", params![id])?;
        if deleted == 0 {
            return Err(ForgeError::NotFound { entity_type: "perspective".into(), id: id.into() });
        }
        Ok(())
    }

    // ── Row mapping ──────────────────────────────────────────────────

    fn map_perspective(row: &rusqlite::Row) -> rusqlite::Result<Perspective> {
        Ok(Perspective {
            id: row.get(0)?,
            bullet_id: row.get(1)?,
            content: row.get(2)?,
            bullet_content_snapshot: row.get(3)?,
            target_archetype: row.get(4)?,
            domain: row.get(5)?,
            framing: row.get::<_, String>(6)?.parse().unwrap_or(Framing::Accomplishment),
            status: row.get::<_, String>(7)?.parse().unwrap_or(PerspectiveStatus::Draft),
            rejection_reason: row.get(8)?,
            prompt_log_id: row.get(9)?,
            approved_at: row.get(10)?,
            approved_by: row.get(11)?,
            created_at: row.get(12)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::stores::bullet::BulletStore;
    use crate::db::stores::source::SourceStore;
    use crate::forge::Forge;
    use forge_core::{CreateSource, SourceType};

    fn setup() -> (Forge, String, String) {
        let forge = Forge::open_memory().unwrap();
        let source = SourceStore::create(forge.conn(), &CreateSource {
            title: "Test Source".into(),
            description: "Test".into(),
            source_type: Some(SourceType::General),
            ..Default::default()
        }).unwrap();
        let bullet = BulletStore::create(
            forge.conn(), "Built APIs", None, None, Some("backend"),
            &[(source.base.id.clone(), true)], &[],
        ).unwrap();
        (forge, source.base.id, bullet.id)
    }

    #[test]
    fn create_perspective() {
        let (forge, _, bullet_id) = setup();
        let p = PerspectiveStore::create(forge.conn(), &CreatePerspectiveInput {
            bullet_id: bullet_id.clone(),
            content: "Designed and built scalable REST APIs".into(),
            bullet_content_snapshot: "Built APIs".into(),
            target_archetype: "backend_engineer".into(),
            domain: "backend".into(),
            framing: Framing::Accomplishment,
            status: None,
            prompt_log_id: None,
        }).unwrap();

        assert_eq!(p.content, "Designed and built scalable REST APIs");
        assert_eq!(p.target_archetype, Some("backend_engineer".into()));
        assert_eq!(p.framing, Framing::Accomplishment);
        assert_eq!(p.status, PerspectiveStatus::Draft);
    }

    #[test]
    fn get_with_chain() {
        let (forge, _, bullet_id) = setup();
        let p = PerspectiveStore::create(forge.conn(), &CreatePerspectiveInput {
            bullet_id: bullet_id.clone(),
            content: "Reframed bullet".into(),
            bullet_content_snapshot: "Built APIs".into(),
            target_archetype: "sre".into(),
            domain: "infra".into(),
            framing: Framing::Responsibility,
            status: None,
            prompt_log_id: None,
        }).unwrap();

        let chain = PerspectiveStore::get_with_chain(forge.conn(), &p.id).unwrap().unwrap();
        assert_eq!(chain.base.id, p.id);
        assert_eq!(chain.bullet.id, bullet_id);
        assert_eq!(chain.source.title, "Test Source");
    }

    #[test]
    fn list_with_archetype_filter() {
        let (forge, _, bullet_id) = setup();
        PerspectiveStore::create(forge.conn(), &CreatePerspectiveInput {
            bullet_id: bullet_id.clone(),
            content: "P1".into(),
            bullet_content_snapshot: "snap".into(),
            target_archetype: "backend".into(),
            domain: "d".into(),
            framing: Framing::Accomplishment,
            status: None,
            prompt_log_id: None,
        }).unwrap();
        PerspectiveStore::create(forge.conn(), &CreatePerspectiveInput {
            bullet_id: bullet_id.clone(),
            content: "P2".into(),
            bullet_content_snapshot: "snap".into(),
            target_archetype: "frontend".into(),
            domain: "d".into(),
            framing: Framing::Context,
            status: None,
            prompt_log_id: None,
        }).unwrap();

        let (perspectives, _) = PerspectiveStore::list(
            forge.conn(),
            &PerspectiveFilter { target_archetype: Some("backend".into()), ..Default::default() },
            &PaginationParams::default(),
        ).unwrap();
        assert_eq!(perspectives.len(), 1);
        assert_eq!(perspectives[0].content, "P1");
    }

    #[test]
    fn status_transitions() {
        let (forge, _, bullet_id) = setup();
        let p = PerspectiveStore::create(forge.conn(), &CreatePerspectiveInput {
            bullet_id,
            content: "Test".into(),
            bullet_content_snapshot: "snap".into(),
            target_archetype: "a".into(),
            domain: "d".into(),
            framing: Framing::Accomplishment,
            status: None,
            prompt_log_id: None,
        }).unwrap();

        let p = PerspectiveStore::transition_status(forge.conn(), &p.id, PerspectiveStatus::InReview, None).unwrap();
        assert_eq!(p.status, PerspectiveStatus::InReview);

        let p = PerspectiveStore::transition_status(forge.conn(), &p.id, PerspectiveStatus::Approved, None).unwrap();
        assert_eq!(p.status, PerspectiveStatus::Approved);
        assert!(p.approved_at.is_some());
    }

    #[test]
    fn delete_perspective() {
        let (forge, _, bullet_id) = setup();
        let p = PerspectiveStore::create(forge.conn(), &CreatePerspectiveInput {
            bullet_id,
            content: "To delete".into(),
            bullet_content_snapshot: "snap".into(),
            target_archetype: "a".into(),
            domain: "d".into(),
            framing: Framing::Accomplishment,
            status: None,
            prompt_log_id: None,
        }).unwrap();

        PerspectiveStore::delete(forge.conn(), &p.id).unwrap();
        assert!(PerspectiveStore::get(forge.conn(), &p.id).unwrap().is_none());
    }
}
