//! Answer bank repository — CRUD for the `answer_bank` table.
//!
//! Stores reusable answers for EEO/work-auth form fields, keyed by
//! `field_kind` (unique). Supports upsert semantics.

use rusqlite::{params, Connection};

use forge_core::{new_id, now_iso, AnswerBankEntry, ForgeError, UpsertAnswerInput};

/// Data access for the `answer_bank` table.
pub struct AnswerBankStore;

impl AnswerBankStore {
    /// List all answer bank entries.
    pub fn list(conn: &Connection) -> Result<Vec<AnswerBankEntry>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT id, field_kind, label, value, created_at, updated_at
             FROM answer_bank ORDER BY field_kind ASC",
        )?;
        let rows: Vec<AnswerBankEntry> = stmt
            .query_map([], Self::map_entry)?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    /// Upsert an answer by field_kind. If the field_kind already exists,
    /// update its label and value; otherwise insert a new row.
    pub fn upsert(
        conn: &Connection,
        input: &UpsertAnswerInput,
    ) -> Result<AnswerBankEntry, ForgeError> {
        let now = now_iso();

        // Check if exists by field_kind
        let existing: Option<String> = conn
            .query_row(
                "SELECT id FROM answer_bank WHERE field_kind = ?1",
                params![input.field_kind],
                |row| row.get(0),
            )
            .ok();

        match existing {
            Some(id) => {
                conn.execute(
                    "UPDATE answer_bank SET label = ?1, value = ?2, updated_at = ?3 WHERE id = ?4",
                    params![input.label, input.value, now, id],
                )?;
                Self::get_by_field_kind(conn, &input.field_kind)?
                    .ok_or_else(|| ForgeError::Internal("AnswerBank updated but not found".into()))
            }
            None => {
                let id = new_id();
                conn.execute(
                    "INSERT INTO answer_bank (id, field_kind, label, value, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
                    params![id, input.field_kind, input.label, input.value, now],
                )?;
                Self::get_by_field_kind(conn, &input.field_kind)?
                    .ok_or_else(|| ForgeError::Internal("AnswerBank created but not found".into()))
            }
        }
    }

    /// Delete an answer by field_kind.
    pub fn delete_by_field_kind(conn: &Connection, field_kind: &str) -> Result<(), ForgeError> {
        let deleted = conn.execute(
            "DELETE FROM answer_bank WHERE field_kind = ?1",
            params![field_kind],
        )?;
        if deleted == 0 {
            return Err(ForgeError::NotFound {
                entity_type: "answer_bank".into(),
                id: field_kind.into(),
            });
        }
        Ok(())
    }

    /// Fetch by field_kind.
    fn get_by_field_kind(
        conn: &Connection,
        field_kind: &str,
    ) -> Result<Option<AnswerBankEntry>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT id, field_kind, label, value, created_at, updated_at
             FROM answer_bank WHERE field_kind = ?1",
        )?;
        let result = stmt.query_row(params![field_kind], Self::map_entry).ok();
        Ok(result)
    }

    fn map_entry(row: &rusqlite::Row) -> rusqlite::Result<AnswerBankEntry> {
        Ok(AnswerBankEntry {
            id: row.get(0)?,
            field_kind: row.get(1)?,
            label: row.get(2)?,
            value: row.get(3)?,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
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
    fn upsert_creates_new_entry() {
        let forge = setup();
        let entry = AnswerBankStore::upsert(
            forge.conn(),
            &UpsertAnswerInput {
                field_kind: "work_authorization".into(),
                label: "Work Authorization".into(),
                value: "US Citizen".into(),
            },
        )
        .unwrap();

        assert_eq!(entry.field_kind, "work_authorization");
        assert_eq!(entry.label, "Work Authorization");
        assert_eq!(entry.value, "US Citizen");
    }

    #[test]
    fn upsert_updates_existing_entry() {
        let forge = setup();
        let first = AnswerBankStore::upsert(
            forge.conn(),
            &UpsertAnswerInput {
                field_kind: "veteran_status".into(),
                label: "Veteran Status".into(),
                value: "Yes".into(),
            },
        )
        .unwrap();

        let second = AnswerBankStore::upsert(
            forge.conn(),
            &UpsertAnswerInput {
                field_kind: "veteran_status".into(),
                label: "Veteran Status (updated)".into(),
                value: "No".into(),
            },
        )
        .unwrap();

        assert_eq!(first.id, second.id); // same row
        assert_eq!(second.label, "Veteran Status (updated)");
        assert_eq!(second.value, "No");
    }

    #[test]
    fn list_entries() {
        let forge = setup();
        AnswerBankStore::upsert(
            forge.conn(),
            &UpsertAnswerInput {
                field_kind: "eeo_gender".into(),
                label: "Gender".into(),
                value: "Male".into(),
            },
        )
        .unwrap();
        AnswerBankStore::upsert(
            forge.conn(),
            &UpsertAnswerInput {
                field_kind: "eeo_race".into(),
                label: "Race".into(),
                value: "White".into(),
            },
        )
        .unwrap();

        let all = AnswerBankStore::list(forge.conn()).unwrap();
        assert_eq!(all.len(), 2);
        // Ordered by field_kind ASC
        assert_eq!(all[0].field_kind, "eeo_gender");
        assert_eq!(all[1].field_kind, "eeo_race");
    }

    #[test]
    fn delete_by_field_kind() {
        let forge = setup();
        AnswerBankStore::upsert(
            forge.conn(),
            &UpsertAnswerInput {
                field_kind: "disability_status".into(),
                label: "Disability".into(),
                value: "No".into(),
            },
        )
        .unwrap();

        AnswerBankStore::delete_by_field_kind(forge.conn(), "disability_status").unwrap();
        let all = AnswerBankStore::list(forge.conn()).unwrap();
        assert!(all.is_empty());
    }

    #[test]
    fn delete_missing_returns_not_found() {
        let forge = setup();
        assert!(matches!(
            AnswerBankStore::delete_by_field_kind(forge.conn(), "nonexistent"),
            Err(ForgeError::NotFound { .. })
        ));
    }
}
