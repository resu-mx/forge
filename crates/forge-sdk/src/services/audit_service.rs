//! Audit service — chain tracing and integrity checking.
//!
//! Provides full derivation chain traversal (perspective -> bullet -> source)
//! and snapshot integrity comparison.

use rusqlite::Connection;

use forge_core::{ChainTrace, ForgeError, IntegrityReport, SnapshotDiff};

use crate::db::stores::perspective::PerspectiveStore;

/// Audit operations for the derivation chain.
pub struct AuditService;

impl AuditService {
    /// Trace the full derivation chain for a perspective.
    ///
    /// Resolves: perspective -> bullet -> primary source.
    pub fn trace_chain(conn: &Connection, perspective_id: &str) -> Result<ChainTrace, ForgeError> {
        let chain = PerspectiveStore::get_with_chain(conn, perspective_id)?.ok_or_else(|| {
            ForgeError::NotFound {
                entity_type: "perspective".into(),
                id: perspective_id.into(),
            }
        })?;

        Ok(ChainTrace {
            perspective: chain.base,
            bullet: chain.bullet,
            source: chain.source,
        })
    }

    /// Check integrity of a perspective's content snapshots.
    ///
    /// Compares:
    /// - `perspective.bullet_content_snapshot` against `bullet.content`
    /// - `bullet.source_content_snapshot` against `source.description`
    pub fn check_integrity(
        conn: &Connection,
        perspective_id: &str,
    ) -> Result<IntegrityReport, ForgeError> {
        let chain = Self::trace_chain(conn, perspective_id)?;

        let bullet_matches = chain.perspective.bullet_content_snapshot == chain.bullet.content;
        let source_matches = chain.bullet.source_content_snapshot == chain.source.description;

        Ok(IntegrityReport {
            perspective_id: perspective_id.to_string(),
            bullet_snapshot_matches: bullet_matches,
            source_snapshot_matches: source_matches,
            bullet_diff: if bullet_matches {
                None
            } else {
                Some(SnapshotDiff {
                    snapshot: chain.perspective.bullet_content_snapshot,
                    current: chain.bullet.content,
                })
            },
            source_diff: if source_matches {
                None
            } else {
                Some(SnapshotDiff {
                    snapshot: chain.bullet.source_content_snapshot,
                    current: chain.source.description,
                })
            },
        })
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

    fn setup() -> (Forge, String, String, String) {
        let forge = Forge::open_memory().unwrap();
        let source = SourceStore::create(
            forge.conn(),
            &CreateSource {
                title: "Test Source".into(),
                description: "Original source description".into(),
                source_type: Some(SourceType::General),
                ..Default::default()
            },
        )
        .unwrap();
        let bullet = BulletStore::create(
            forge.conn(),
            "Original bullet content",
            Some("Original source description"),
            None,
            Some("backend"),
            &[(source.base.id.clone(), true)],
            &[],
        )
        .unwrap();
        let perspective = PerspectiveStore::create(
            forge.conn(),
            &CreatePerspectiveInput {
                bullet_id: bullet.id.clone(),
                content: "Reframed for SRE".into(),
                bullet_content_snapshot: "Original bullet content".into(),
                target_archetype: "sre".into(),
                domain: "infra".into(),
                framing: Framing::Responsibility,
                status: None,
                prompt_log_id: None,
            },
        )
        .unwrap();

        (forge, source.base.id, bullet.id, perspective.id)
    }

    #[test]
    fn trace_chain_returns_full_chain() {
        let (forge, source_id, bullet_id, perspective_id) = setup();
        let chain = AuditService::trace_chain(forge.conn(), &perspective_id).unwrap();

        assert_eq!(chain.perspective.id, perspective_id);
        assert_eq!(chain.bullet.id, bullet_id);
        assert_eq!(chain.source.id, source_id);
    }

    #[test]
    fn trace_chain_not_found() {
        let forge = Forge::open_memory().unwrap();
        let result = AuditService::trace_chain(forge.conn(), "nonexistent");
        assert!(matches!(result, Err(ForgeError::NotFound { .. })));
    }

    #[test]
    fn check_integrity_all_matching() {
        let (forge, _, _, perspective_id) = setup();
        let report = AuditService::check_integrity(forge.conn(), &perspective_id).unwrap();

        assert!(report.bullet_snapshot_matches);
        assert!(report.source_snapshot_matches);
        assert!(report.bullet_diff.is_none());
        assert!(report.source_diff.is_none());
    }

    #[test]
    fn check_integrity_detects_bullet_drift() {
        let (forge, _, bullet_id, perspective_id) = setup();

        // Mutate the bullet content directly
        forge
            .conn()
            .execute(
                "UPDATE bullets SET content = 'Changed bullet content' WHERE id = ?1",
                rusqlite::params![bullet_id],
            )
            .unwrap();

        let report = AuditService::check_integrity(forge.conn(), &perspective_id).unwrap();

        assert!(!report.bullet_snapshot_matches);
        assert!(report.source_snapshot_matches);
        let diff = report.bullet_diff.unwrap();
        assert_eq!(diff.snapshot, "Original bullet content");
        assert_eq!(diff.current, "Changed bullet content");
    }

    #[test]
    fn check_integrity_detects_source_drift() {
        let (forge, source_id, _, perspective_id) = setup();

        // Mutate the source description directly
        forge
            .conn()
            .execute(
                "UPDATE sources SET description = 'Changed source description' WHERE id = ?1",
                rusqlite::params![source_id],
            )
            .unwrap();

        let report = AuditService::check_integrity(forge.conn(), &perspective_id).unwrap();

        assert!(report.bullet_snapshot_matches);
        assert!(!report.source_snapshot_matches);
        let diff = report.source_diff.unwrap();
        assert_eq!(diff.snapshot, "Original source description");
        assert_eq!(diff.current, "Changed source description");
    }
}
