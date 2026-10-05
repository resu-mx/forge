//! Derivation repository — lock management for the split-handshake
//! derivation protocol (`pending_derivations` table).
//!
//! The prepare/commit lifecycle:
//!   1. **prepare** — acquire exclusive lock, render prompt, capture snapshot
//!   2. (client executes LLM call externally)
//!   3. **commit** — validate response, write derived entities, release lock

use rusqlite::{params, Connection, OptionalExtension};

use forge_core::{new_id, now_iso, CreatePendingDerivationInput, ForgeError, PendingDerivation};

/// Data access for the `pending_derivations` table.
pub struct DerivationStore;

impl DerivationStore {
    /// Create a pending derivation (acquire lock).
    ///
    /// The UNIQUE index on (entity_type, entity_id) prevents concurrent
    /// prepares on the same entity.
    pub fn create(
        conn: &Connection,
        input: &CreatePendingDerivationInput,
    ) -> Result<PendingDerivation, ForgeError> {
        let id = new_id();
        let now = now_iso();

        match conn.execute(
            "INSERT INTO pending_derivations (id, entity_type, entity_id, client_id, prompt, snapshot, derivation_params, locked_at, expires_at, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                id,
                input.entity_type,
                input.entity_id,
                input.client_id,
                input.prompt,
                input.snapshot,
                input.derivation_params,
                now,
                input.expires_at,
                now,
            ],
        ) {
            Ok(_) => {}
            Err(rusqlite::Error::SqliteFailure(err, _))
                if err.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                return Err(ForgeError::Conflict {
                    message: format!(
                        "A derivation is already in progress for {} {}",
                        input.entity_type, input.entity_id
                    ),
                });
            }
            Err(e) => return Err(e.into()),
        }

        Self::get(conn, &id)?
            .ok_or_else(|| ForgeError::Internal("PendingDerivation created but not found".into()))
    }

    /// Fetch a pending derivation by ID.
    pub fn get(conn: &Connection, id: &str) -> Result<Option<PendingDerivation>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT id, entity_type, entity_id, client_id, prompt, snapshot,
                    derivation_params, locked_at, expires_at, created_at
             FROM pending_derivations WHERE id = ?1",
        )?;
        let result = stmt.query_row(params![id], Self::map_row).optional()?;
        Ok(result)
    }

    /// Check if an unexpired lock exists for the given entity.
    pub fn has_active_lock(
        conn: &Connection,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<bool, ForgeError> {
        let now = now_iso();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM pending_derivations
             WHERE entity_type = ?1 AND entity_id = ?2 AND expires_at > ?3",
            params![entity_type, entity_id, now],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// Delete a pending derivation (release lock).
    pub fn delete(conn: &Connection, id: &str) -> Result<(), ForgeError> {
        conn.execute("DELETE FROM pending_derivations WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Delete all expired locks. Returns the count of deleted rows.
    pub fn cleanup_expired(conn: &Connection) -> Result<usize, ForgeError> {
        let now = now_iso();
        let deleted = conn.execute(
            "DELETE FROM pending_derivations WHERE expires_at <= ?1",
            params![now],
        )?;
        Ok(deleted)
    }

    /// Check if a derivation has expired. If expired, deletes it and returns true.
    pub fn check_and_cleanup_if_expired(
        conn: &Connection,
        pending: &PendingDerivation,
    ) -> Result<bool, ForgeError> {
        let now = now_iso();
        if pending.expires_at <= now {
            Self::delete(conn, &pending.id)?;
            return Ok(true);
        }
        Ok(false)
    }

    /// Create a prompt_log entry for audit trail.
    pub fn create_prompt_log(
        conn: &Connection,
        entity_type: &str,
        entity_id: &str,
        prompt_template: &str,
        prompt_input: &str,
        raw_response: &str,
    ) -> Result<String, ForgeError> {
        let id = new_id();
        let now = now_iso();
        conn.execute(
            "INSERT INTO prompt_logs (id, entity_type, entity_id, prompt_template, prompt_input, raw_response, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, entity_type, entity_id, prompt_template, prompt_input, raw_response, now],
        )?;
        Ok(id)
    }

    fn map_row(row: &rusqlite::Row) -> rusqlite::Result<PendingDerivation> {
        Ok(PendingDerivation {
            id: row.get(0)?,
            entity_type: row.get(1)?,
            entity_id: row.get(2)?,
            client_id: row.get(3)?,
            prompt: row.get(4)?,
            snapshot: row.get(5)?,
            derivation_params: row.get(6)?,
            locked_at: row.get(7)?,
            expires_at: row.get(8)?,
            created_at: row.get(9)?,
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

    fn future_time() -> String {
        // 2 minutes from now (far enough to not expire during test)
        let now = chrono::Utc::now() + chrono::Duration::seconds(120);
        now.format("%Y-%m-%dT%H:%M:%SZ").to_string()
    }

    fn past_time() -> String {
        let past = chrono::Utc::now() - chrono::Duration::seconds(120);
        past.format("%Y-%m-%dT%H:%M:%SZ").to_string()
    }

    #[test]
    fn create_and_get_pending() {
        let forge = setup();
        let input = CreatePendingDerivationInput {
            entity_type: "source".into(),
            entity_id: "src-123".into(),
            client_id: "mcp-client-1".into(),
            prompt: "Decompose into bullets...".into(),
            snapshot: "Led a team of 4...".into(),
            derivation_params: None,
            expires_at: future_time(),
        };
        let pending = DerivationStore::create(forge.conn(), &input).unwrap();
        assert_eq!(pending.entity_type, "source");
        assert_eq!(pending.entity_id, "src-123");
        assert_eq!(pending.client_id, "mcp-client-1");

        let fetched = DerivationStore::get(forge.conn(), &pending.id)
            .unwrap()
            .unwrap();
        assert_eq!(fetched.id, pending.id);
    }

    #[test]
    fn create_duplicate_entity_returns_conflict() {
        let forge = setup();
        let input = CreatePendingDerivationInput {
            entity_type: "source".into(),
            entity_id: "src-123".into(),
            client_id: "client-a".into(),
            prompt: "prompt".into(),
            snapshot: "snapshot".into(),
            derivation_params: None,
            expires_at: future_time(),
        };
        DerivationStore::create(forge.conn(), &input).unwrap();

        let dup = CreatePendingDerivationInput {
            entity_type: "source".into(),
            entity_id: "src-123".into(),
            client_id: "client-b".into(),
            prompt: "prompt 2".into(),
            snapshot: "snapshot 2".into(),
            derivation_params: None,
            expires_at: future_time(),
        };
        let result = DerivationStore::create(forge.conn(), &dup);
        assert!(matches!(result, Err(ForgeError::Conflict { .. })));
    }

    #[test]
    fn has_active_lock_returns_true_for_unexpired() {
        let forge = setup();
        DerivationStore::create(
            forge.conn(),
            &CreatePendingDerivationInput {
                entity_type: "source".into(),
                entity_id: "src-456".into(),
                client_id: "c".into(),
                prompt: "p".into(),
                snapshot: "s".into(),
                derivation_params: None,
                expires_at: future_time(),
            },
        )
        .unwrap();

        assert!(DerivationStore::has_active_lock(forge.conn(), "source", "src-456").unwrap());
    }

    #[test]
    fn has_active_lock_returns_false_for_expired() {
        let forge = setup();
        DerivationStore::create(
            forge.conn(),
            &CreatePendingDerivationInput {
                entity_type: "source".into(),
                entity_id: "src-789".into(),
                client_id: "c".into(),
                prompt: "p".into(),
                snapshot: "s".into(),
                derivation_params: None,
                expires_at: past_time(),
            },
        )
        .unwrap();

        assert!(!DerivationStore::has_active_lock(forge.conn(), "source", "src-789").unwrap());
    }

    #[test]
    fn delete_releases_lock() {
        let forge = setup();
        let pending = DerivationStore::create(
            forge.conn(),
            &CreatePendingDerivationInput {
                entity_type: "bullet".into(),
                entity_id: "bul-123".into(),
                client_id: "c".into(),
                prompt: "p".into(),
                snapshot: "s".into(),
                derivation_params: Some(
                    r#"{"archetype":"swe","domain":"backend","framing":"accomplishment"}"#.into(),
                ),
                expires_at: future_time(),
            },
        )
        .unwrap();

        DerivationStore::delete(forge.conn(), &pending.id).unwrap();
        assert!(DerivationStore::get(forge.conn(), &pending.id)
            .unwrap()
            .is_none());
    }

    #[test]
    fn cleanup_expired_removes_old_locks() {
        let forge = setup();
        DerivationStore::create(
            forge.conn(),
            &CreatePendingDerivationInput {
                entity_type: "source".into(),
                entity_id: "old-1".into(),
                client_id: "c".into(),
                prompt: "p".into(),
                snapshot: "s".into(),
                derivation_params: None,
                expires_at: past_time(),
            },
        )
        .unwrap();
        DerivationStore::create(
            forge.conn(),
            &CreatePendingDerivationInput {
                entity_type: "source".into(),
                entity_id: "fresh-1".into(),
                client_id: "c".into(),
                prompt: "p".into(),
                snapshot: "s".into(),
                derivation_params: None,
                expires_at: future_time(),
            },
        )
        .unwrap();

        let deleted = DerivationStore::cleanup_expired(forge.conn()).unwrap();
        assert_eq!(deleted, 1);

        // Fresh one should still exist
        assert!(DerivationStore::has_active_lock(forge.conn(), "source", "fresh-1").unwrap());
    }

    #[test]
    fn check_and_cleanup_if_expired_deletes_expired() {
        let forge = setup();
        let pending = DerivationStore::create(
            forge.conn(),
            &CreatePendingDerivationInput {
                entity_type: "source".into(),
                entity_id: "exp-1".into(),
                client_id: "c".into(),
                prompt: "p".into(),
                snapshot: "s".into(),
                derivation_params: None,
                expires_at: past_time(),
            },
        )
        .unwrap();

        let expired =
            DerivationStore::check_and_cleanup_if_expired(forge.conn(), &pending).unwrap();
        assert!(expired);
        assert!(DerivationStore::get(forge.conn(), &pending.id)
            .unwrap()
            .is_none());
    }

    #[test]
    fn check_and_cleanup_if_expired_keeps_valid() {
        let forge = setup();
        let pending = DerivationStore::create(
            forge.conn(),
            &CreatePendingDerivationInput {
                entity_type: "source".into(),
                entity_id: "valid-1".into(),
                client_id: "c".into(),
                prompt: "p".into(),
                snapshot: "s".into(),
                derivation_params: None,
                expires_at: future_time(),
            },
        )
        .unwrap();

        let expired =
            DerivationStore::check_and_cleanup_if_expired(forge.conn(), &pending).unwrap();
        assert!(!expired);
        assert!(DerivationStore::get(forge.conn(), &pending.id)
            .unwrap()
            .is_some());
    }

    #[test]
    fn create_prompt_log() {
        let forge = setup();
        let log_id = DerivationStore::create_prompt_log(
            forge.conn(),
            "bullet",
            "bul-123",
            "source-to-bullet-v1",
            "prompt input text",
            r#"{"bullets":[{"content":"test","technologies":[],"metrics":null}]}"#,
        )
        .unwrap();
        assert!(!log_id.is_empty());
    }
}
