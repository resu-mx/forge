//! Integrity service — content drift detection for snapshot-based derivation chain.
//!
//! Scans all bullets and perspectives for stale content snapshots.

use rusqlite::Connection;
use serde::Serialize;

use forge_core::ForgeError;

/// Entity type discriminator for drift reports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DriftedEntityType {
    Bullet,
    Perspective,
}

/// A single entity with a stale snapshot.
#[derive(Debug, Clone, Serialize)]
pub struct DriftedEntity {
    pub entity_type: DriftedEntityType,
    pub entity_id: String,
    pub snapshot_field: String,
    pub snapshot_value: String,
    pub current_value: String,
}

/// Content drift detection across the entire derivation chain.
pub struct IntegrityService;

impl IntegrityService {
    /// Find all entities with stale snapshots across the entire database.
    ///
    /// Checks:
    /// - **Bullets**: `source_content_snapshot` != primary source's `description`
    /// - **Perspectives**: `bullet_content_snapshot` != bullet's `content`
    pub fn get_drifted_entities(conn: &Connection) -> Result<Vec<DriftedEntity>, ForgeError> {
        let mut result = Vec::new();

        // Drifted bullets
        let mut bullet_stmt = conn.prepare(
            "SELECT b.id, b.source_content_snapshot, s.description
             FROM bullets b
             JOIN bullet_sources bs ON b.id = bs.bullet_id AND bs.is_primary = 1
             JOIN sources s ON bs.source_id = s.id
             WHERE b.source_content_snapshot != s.description",
        )?;

        let drifted_bullets: Vec<DriftedEntity> = bullet_stmt
            .query_map([], |row| {
                Ok(DriftedEntity {
                    entity_type: DriftedEntityType::Bullet,
                    entity_id: row.get(0)?,
                    snapshot_field: "source_content_snapshot".into(),
                    snapshot_value: row.get(1)?,
                    current_value: row.get(2)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        result.extend(drifted_bullets);

        // Drifted perspectives
        let mut perspective_stmt = conn.prepare(
            "SELECT p.id, p.bullet_content_snapshot, b.content
             FROM perspectives p
             JOIN bullets b ON p.bullet_id = b.id
             WHERE p.bullet_content_snapshot != b.content",
        )?;

        let drifted_perspectives: Vec<DriftedEntity> = perspective_stmt
            .query_map([], |row| {
                Ok(DriftedEntity {
                    entity_type: DriftedEntityType::Perspective,
                    entity_id: row.get(0)?,
                    snapshot_field: "bullet_content_snapshot".into(),
                    snapshot_value: row.get(1)?,
                    current_value: row.get(2)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        result.extend(drifted_perspectives);

        Ok(result)
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
    use rusqlite::params;

    fn setup() -> Forge {
        Forge::open_memory().unwrap()
    }

    fn create_chain(conn: &Connection) -> (String, String, String) {
        let source = SourceStore::create(
            conn,
            &CreateSource {
                title: "Source".into(),
                description: "Original description".into(),
                source_type: Some(SourceType::General),
                ..Default::default()
            },
        )
        .unwrap();

        let bullet = BulletStore::create(
            conn,
            "Original bullet",
            Some("Original description"),
            None,
            None,
            &[(source.base.id.clone(), true)],
            &[],
        )
        .unwrap();

        let perspective = PerspectiveStore::create(
            conn,
            &CreatePerspectiveInput {
                bullet_id: bullet.id.clone(),
                content: "Perspective content".into(),
                bullet_content_snapshot: "Original bullet".into(),
                target_archetype: "sre".into(),
                domain: "infra".into(),
                framing: Framing::Responsibility,
                status: None,
                prompt_log_id: None,
            },
        )
        .unwrap();

        (source.base.id, bullet.id, perspective.id)
    }

    #[test]
    fn no_drift_when_all_in_sync() {
        let forge = setup();
        create_chain(forge.conn());

        let drifted = IntegrityService::get_drifted_entities(forge.conn()).unwrap();
        assert!(drifted.is_empty());
    }

    #[test]
    fn detects_bullet_drift() {
        let forge = setup();
        let (source_id, bullet_id, _) = create_chain(forge.conn());

        // Mutate source description
        forge
            .conn()
            .execute(
                "UPDATE sources SET description = 'Changed description' WHERE id = ?1",
                params![source_id],
            )
            .unwrap();

        let drifted = IntegrityService::get_drifted_entities(forge.conn()).unwrap();
        assert_eq!(drifted.len(), 1);
        assert_eq!(drifted[0].entity_type, DriftedEntityType::Bullet);
        assert_eq!(drifted[0].entity_id, bullet_id);
        assert_eq!(drifted[0].snapshot_value, "Original description");
        assert_eq!(drifted[0].current_value, "Changed description");
    }

    #[test]
    fn detects_perspective_drift() {
        let forge = setup();
        let (_, bullet_id, perspective_id) = create_chain(forge.conn());

        // Mutate bullet content
        forge
            .conn()
            .execute(
                "UPDATE bullets SET content = 'Changed bullet' WHERE id = ?1",
                params![bullet_id],
            )
            .unwrap();

        let drifted = IntegrityService::get_drifted_entities(forge.conn()).unwrap();
        assert_eq!(drifted.len(), 1);
        assert_eq!(drifted[0].entity_type, DriftedEntityType::Perspective);
        assert_eq!(drifted[0].entity_id, perspective_id);
    }

    #[test]
    fn detects_both_bullet_and_perspective_drift() {
        let forge = setup();
        let (source_id, bullet_id, _) = create_chain(forge.conn());

        // Mutate both source and bullet
        forge
            .conn()
            .execute(
                "UPDATE sources SET description = 'Changed source' WHERE id = ?1",
                params![source_id],
            )
            .unwrap();
        forge
            .conn()
            .execute(
                "UPDATE bullets SET content = 'Changed bullet' WHERE id = ?1",
                params![bullet_id],
            )
            .unwrap();

        let drifted = IntegrityService::get_drifted_entities(forge.conn()).unwrap();
        assert_eq!(drifted.len(), 2);
        assert_eq!(drifted[0].entity_type, DriftedEntityType::Bullet);
        assert_eq!(drifted[1].entity_type, DriftedEntityType::Perspective);
    }
}
