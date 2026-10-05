//! Review service — review queue for pending bullets and perspectives.
//!
//! Returns counts and items in `in_review` status with relevant context
//! (source title, bullet content, linked skills/technologies).

use rusqlite::{params, Connection};

use forge_core::{
    Bullet, BulletReviewItem, BulletStatus, ForgeError, Framing, Perspective,
    PerspectiveReviewItem, PerspectiveStatus, ReviewQueue, ReviewQueueSection,
};

/// Review queue operations.
pub struct ReviewService;

impl ReviewService {
    /// Get all pending review items (bullets and perspectives with
    /// `status = 'in_review'`).
    pub fn get_pending_review(conn: &Connection) -> Result<ReviewQueue, ForgeError> {
        let bullets = Self::get_pending_bullets(conn)?;
        let perspectives = Self::get_pending_perspectives(conn)?;

        Ok(ReviewQueue {
            bullets: ReviewQueueSection {
                count: bullets.len() as i64,
                items: bullets,
            },
            perspectives: ReviewQueueSection {
                count: perspectives.len() as i64,
                items: perspectives,
            },
        })
    }

    fn get_pending_bullets(conn: &Connection) -> Result<Vec<BulletReviewItem>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT b.id, b.content, b.source_content_snapshot, b.metrics, b.domain,
                    b.status, b.rejection_reason, b.prompt_log_id,
                    b.approved_at, b.approved_by, b.created_at,
                    COALESCE(s.title, 'Unknown Source') AS source_title
             FROM bullets b
             LEFT JOIN bullet_sources bs ON b.id = bs.bullet_id AND bs.is_primary = 1
             LEFT JOIN sources s ON bs.source_id = s.id
             WHERE b.status = 'in_review'
             ORDER BY b.created_at DESC",
        )?;

        let items: Vec<(Bullet, String)> = stmt
            .query_map([], |row| {
                let bullet = Bullet {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    source_content_snapshot: row.get::<_, String>(2).unwrap_or_default(),
                    technologies: Vec::new(), // populated below
                    metrics: row.get(3)?,
                    domain: row.get(4)?,
                    status: row
                        .get::<_, String>(5)?
                        .parse()
                        .unwrap_or(BulletStatus::Draft),
                    rejection_reason: row.get(6)?,
                    prompt_log_id: row.get(7)?,
                    approved_at: row.get(8)?,
                    approved_by: row.get(9)?,
                    created_at: row.get(10)?,
                };
                let source_title: String = row.get(11)?;
                Ok((bullet, source_title))
            })?
            .collect::<Result<_, _>>()?;

        // Hydrate technologies for each bullet
        let mut result = Vec::with_capacity(items.len());
        for (mut bullet, source_title) in items {
            let mut tech_stmt = conn.prepare(
                "SELECT s.name FROM bullet_skills bsk
                 JOIN skills s ON s.id = bsk.skill_id
                 WHERE bsk.bullet_id = ?1
                 ORDER BY LOWER(s.name) ASC",
            )?;
            let techs: Vec<String> = tech_stmt
                .query_map(params![bullet.id], |row| row.get(0))?
                .collect::<Result<_, _>>()?;
            bullet.technologies = techs;

            result.push(BulletReviewItem {
                base: bullet,
                source_title,
            });
        }

        Ok(result)
    }

    fn get_pending_perspectives(
        conn: &Connection,
    ) -> Result<Vec<PerspectiveReviewItem>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT p.id, p.bullet_id, p.content, p.bullet_content_snapshot,
                    p.target_archetype, p.domain, p.framing, p.status,
                    p.rejection_reason, p.prompt_log_id, p.approved_at,
                    p.approved_by, p.created_at,
                    b.content AS bullet_content,
                    COALESCE(s.title, 'Unknown Source') AS source_title
             FROM perspectives p
             JOIN bullets b ON b.id = p.bullet_id
             LEFT JOIN bullet_sources bs ON b.id = bs.bullet_id AND bs.is_primary = 1
             LEFT JOIN sources s ON bs.source_id = s.id
             WHERE p.status = 'in_review'
             ORDER BY p.created_at DESC",
        )?;

        let items = stmt
            .query_map([], |row| {
                let perspective = Perspective {
                    id: row.get(0)?,
                    bullet_id: row.get(1)?,
                    content: row.get(2)?,
                    bullet_content_snapshot: row.get(3)?,
                    target_archetype: row.get(4)?,
                    domain: row.get(5)?,
                    framing: row
                        .get::<_, String>(6)?
                        .parse()
                        .unwrap_or(Framing::Responsibility),
                    status: row
                        .get::<_, String>(7)?
                        .parse()
                        .unwrap_or(PerspectiveStatus::Draft),
                    rejection_reason: row.get(8)?,
                    prompt_log_id: row.get(9)?,
                    approved_at: row.get(10)?,
                    approved_by: row.get(11)?,
                    created_at: row.get(12)?,
                };
                let bullet_content: String = row.get(13)?;
                let source_title: String = row.get(14)?;

                Ok(PerspectiveReviewItem {
                    base: perspective,
                    bullet_content,
                    source_title,
                })
            })?
            .collect::<Result<_, _>>()?;

        Ok(items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::stores::bullet::BulletStore;
    use crate::db::stores::perspective::PerspectiveStore;
    use crate::db::stores::source::SourceStore;
    use crate::forge::Forge;
    use forge_core::{CreatePerspectiveInput, CreateSource, Framing, SourceType};

    fn setup() -> Forge {
        Forge::open_memory().unwrap()
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

    #[test]
    fn empty_review_queue() {
        let forge = setup();
        let queue = ReviewService::get_pending_review(forge.conn()).unwrap();
        assert_eq!(queue.bullets.count, 0);
        assert_eq!(queue.perspectives.count, 0);
    }

    #[test]
    fn bullets_in_review_appear_in_queue() {
        let forge = setup();
        let source_id = create_source(forge.conn());

        let bullet = BulletStore::create(
            forge.conn(),
            "Test bullet",
            None,
            None,
            Some("backend"),
            &[(source_id, true)],
            &["rust".into()],
        )
        .unwrap();

        // Move to in_review
        BulletStore::transition_status(forge.conn(), &bullet.id, BulletStatus::InReview, None)
            .unwrap();

        let queue = ReviewService::get_pending_review(forge.conn()).unwrap();
        assert_eq!(queue.bullets.count, 1);
        assert_eq!(queue.bullets.items[0].base.id, bullet.id);
        assert_eq!(queue.bullets.items[0].source_title, "Test Source");
        assert_eq!(queue.bullets.items[0].base.technologies, vec!["rust"]);
        assert_eq!(queue.perspectives.count, 0);
    }

    #[test]
    fn perspectives_in_review_appear_in_queue() {
        let forge = setup();
        let source_id = create_source(forge.conn());

        let bullet = BulletStore::create(
            forge.conn(),
            "Test bullet",
            None,
            None,
            Some("backend"),
            &[(source_id, true)],
            &[],
        )
        .unwrap();

        let perspective = PerspectiveStore::create(
            forge.conn(),
            &CreatePerspectiveInput {
                bullet_id: bullet.id.clone(),
                content: "SRE perspective".into(),
                bullet_content_snapshot: "Test bullet".into(),
                target_archetype: "sre".into(),
                domain: "infra".into(),
                framing: Framing::Responsibility,
                status: None,
                prompt_log_id: None,
            },
        )
        .unwrap();

        // Move to in_review
        PerspectiveStore::transition_status(
            forge.conn(),
            &perspective.id,
            PerspectiveStatus::InReview,
            None,
        )
        .unwrap();

        let queue = ReviewService::get_pending_review(forge.conn()).unwrap();
        assert_eq!(queue.perspectives.count, 1);
        assert_eq!(queue.perspectives.items[0].base.id, perspective.id);
        assert_eq!(queue.perspectives.items[0].bullet_content, "Test bullet");
        assert_eq!(queue.perspectives.items[0].source_title, "Test Source");
    }

    #[test]
    fn approved_items_not_in_queue() {
        let forge = setup();
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

        // Move to in_review then approved
        BulletStore::transition_status(forge.conn(), &bullet.id, BulletStatus::InReview, None)
            .unwrap();
        BulletStore::transition_status(forge.conn(), &bullet.id, BulletStatus::Approved, None)
            .unwrap();

        let queue = ReviewService::get_pending_review(forge.conn()).unwrap();
        assert_eq!(queue.bullets.count, 0);
    }
}
