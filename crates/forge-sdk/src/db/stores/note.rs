//! Repository for user note persistence.
//!
//! Provides CRUD operations and polymorphic reference linking for the
//! `user_notes` and `note_references` tables.

use rusqlite::{params, Connection, OptionalExtension};

use forge_core::{
    new_id, now_iso, ForgeError, NoteReference, NoteReferenceEntityType, Pagination, UserNote,
};

/// Data-access repository for user notes and note references.
pub struct NoteStore;

impl NoteStore {
    // ── Core CRUD ───────────────────────────────────────────────────

    /// Insert a new user note row.
    pub fn create(
        conn: &Connection,
        title: Option<&str>,
        content: &str,
    ) -> Result<UserNote, ForgeError> {
        let id = new_id();
        let now = now_iso();

        conn.execute(
            "INSERT INTO user_notes (id, title, content, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?4)",
            params![id, title, content, now],
        )?;

        Self::get(conn, &id)?
            .ok_or_else(|| ForgeError::Internal("Note created but not found".into()))
    }

    /// Fetch a single note by primary key.
    pub fn get(conn: &Connection, id: &str) -> Result<Option<UserNote>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT id, title, content, created_at, updated_at
             FROM user_notes WHERE id = ?1",
        )?;

        let result = stmt.query_row(params![id], Self::map_note).optional()?;
        Ok(result)
    }

    /// List notes with optional text search and pagination.
    /// Searches across `title` and `content` fields (case-insensitive LIKE).
    pub fn list(
        conn: &Connection,
        search: Option<&str>,
        offset: i64,
        limit: i64,
    ) -> Result<(Vec<UserNote>, Pagination), ForgeError> {
        let mut conditions = Vec::new();
        let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(search_term) = search {
            let param_idx = bind_values.len() + 1;
            conditions.push(format!(
                "(content LIKE ?{param_idx} OR title LIKE ?{param_idx})"
            ));
            bind_values.push(Box::new(format!("%{search_term}%")));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        // Count
        let count_sql = format!("SELECT COUNT(*) FROM user_notes {where_clause}");
        let total: i64 = conn.query_row(
            &count_sql,
            rusqlite::params_from_iter(bind_values.iter().map(|b| b.as_ref())),
            |row| row.get(0),
        )?;

        // Fetch page
        let query_sql = format!(
            "SELECT id, title, content, created_at, updated_at
             FROM user_notes {where_clause}
             ORDER BY updated_at DESC
             LIMIT ?{} OFFSET ?{}",
            bind_values.len() + 1,
            bind_values.len() + 2
        );
        bind_values.push(Box::new(limit));
        bind_values.push(Box::new(offset));

        let mut stmt = conn.prepare(&query_sql)?;
        let rows: Vec<UserNote> = stmt
            .query_map(
                rusqlite::params_from_iter(bind_values.iter().map(|b| b.as_ref())),
                Self::map_note,
            )?
            .collect::<Result<_, _>>()?;

        Ok((
            rows,
            Pagination {
                total,
                offset,
                limit,
            },
        ))
    }

    /// Apply a partial update to an existing note (title and/or content).
    pub fn update(
        conn: &Connection,
        id: &str,
        title: Option<&str>,
        content: Option<&str>,
    ) -> Result<UserNote, ForgeError> {
        Self::get(conn, id)?.ok_or_else(|| ForgeError::NotFound {
            entity_type: "user_note".into(),
            id: id.into(),
        })?;

        let mut sets = Vec::new();
        let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(v) = title {
            sets.push(format!("title = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.to_string()));
        }
        if let Some(v) = content {
            sets.push(format!("content = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.to_string()));
        }

        if !sets.is_empty() {
            let now = now_iso();
            sets.push(format!("updated_at = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(now));

            let sql = format!(
                "UPDATE user_notes SET {} WHERE id = ?{}",
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
            .ok_or_else(|| ForgeError::Internal("Note updated but not found".into()))
    }

    /// Delete a note by primary key (cascades to `note_references`).
    pub fn delete(conn: &Connection, id: &str) -> Result<(), ForgeError> {
        let deleted = conn.execute("DELETE FROM user_notes WHERE id = ?1", params![id])?;
        if deleted == 0 {
            return Err(ForgeError::NotFound {
                entity_type: "user_note".into(),
                id: id.into(),
            });
        }
        Ok(())
    }

    // ── Reference junction ──────────────────────────────────────────

    /// Add a polymorphic reference linking a note to an entity.
    pub fn add_reference(
        conn: &Connection,
        note_id: &str,
        entity_type: NoteReferenceEntityType,
        entity_id: &str,
    ) -> Result<(), ForgeError> {
        conn.execute(
            "INSERT INTO note_references (note_id, entity_type, entity_id)
             VALUES (?1, ?2, ?3)",
            params![note_id, entity_type.as_ref(), entity_id],
        )?;
        Ok(())
    }

    /// Remove a polymorphic reference by composite key.
    /// Returns an error with `NOT_FOUND` if the reference does not exist.
    pub fn remove_reference(
        conn: &Connection,
        note_id: &str,
        entity_type: NoteReferenceEntityType,
        entity_id: &str,
    ) -> Result<(), ForgeError> {
        let deleted = conn.execute(
            "DELETE FROM note_references
             WHERE note_id = ?1 AND entity_type = ?2 AND entity_id = ?3",
            params![note_id, entity_type.as_ref(), entity_id],
        )?;
        if deleted == 0 {
            return Err(ForgeError::NotFound {
                entity_type: "note_reference".into(),
                id: format!("{note_id}/{}/{entity_id}", entity_type.as_ref()),
            });
        }
        Ok(())
    }

    /// List all references for a given note.
    pub fn list_references(
        conn: &Connection,
        note_id: &str,
    ) -> Result<Vec<NoteReference>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT note_id, entity_type, entity_id
             FROM note_references WHERE note_id = ?1",
        )?;

        let rows: Vec<NoteReference> = stmt
            .query_map(params![note_id], Self::map_reference)?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    /// Find all notes referencing a specific entity (reverse lookup).
    /// Returns notes ordered by `updated_at DESC`.
    pub fn find_by_entity(
        conn: &Connection,
        entity_type: NoteReferenceEntityType,
        entity_id: &str,
    ) -> Result<Vec<UserNote>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT n.id, n.title, n.content, n.created_at, n.updated_at
             FROM note_references nr
             JOIN user_notes n ON n.id = nr.note_id
             WHERE nr.entity_type = ?1 AND nr.entity_id = ?2
             ORDER BY n.updated_at DESC",
        )?;

        let rows: Vec<UserNote> = stmt
            .query_map(params![entity_type.as_ref(), entity_id], Self::map_note)?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    // ── Row mapping ──────────────────────────────────────────────────

    fn map_note(row: &rusqlite::Row) -> rusqlite::Result<UserNote> {
        Ok(UserNote {
            id: row.get(0)?,
            title: row.get(1)?,
            content: row.get(2)?,
            created_at: row.get(3)?,
            updated_at: row.get(4)?,
        })
    }

    fn map_reference(row: &rusqlite::Row) -> rusqlite::Result<NoteReference> {
        Ok(NoteReference {
            note_id: row.get(0)?,
            entity_type: row
                .get::<_, String>(1)?
                .parse()
                .unwrap_or(NoteReferenceEntityType::Source),
            entity_id: row.get(2)?,
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
    fn create_and_get() {
        let forge = setup();
        let note = NoteStore::create(
            forge.conn(),
            Some("My Note"),
            "This is the content of my note.",
        )
        .unwrap();
        assert_eq!(note.title, Some("My Note".into()));
        assert_eq!(note.content, "This is the content of my note.");

        let fetched = NoteStore::get(forge.conn(), &note.id).unwrap().unwrap();
        assert_eq!(fetched.id, note.id);
        assert_eq!(fetched.content, note.content);
    }

    #[test]
    fn create_without_title() {
        let forge = setup();
        let note = NoteStore::create(forge.conn(), None, "Untitled content").unwrap();
        assert_eq!(note.title, None);
        assert_eq!(note.content, "Untitled content");
    }

    #[test]
    fn get_returns_none_for_missing() {
        let forge = setup();
        let result = NoteStore::get(forge.conn(), "nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn list_empty() {
        let forge = setup();
        let (rows, pagination) = NoteStore::list(forge.conn(), None, 0, 50).unwrap();
        assert!(rows.is_empty());
        assert_eq!(pagination.total, 0);
    }

    #[test]
    fn list_with_search() {
        let forge = setup();
        NoteStore::create(
            forge.conn(),
            Some("Rust Notes"),
            "Learning Rust programming",
        )
        .unwrap();
        NoteStore::create(
            forge.conn(),
            Some("Python Notes"),
            "Learning Python scripting",
        )
        .unwrap();

        let (rows, _) = NoteStore::list(forge.conn(), Some("rust"), 0, 50).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].title, Some("Rust Notes".into()));
    }

    #[test]
    fn update_note() {
        let forge = setup();
        let created = NoteStore::create(forge.conn(), Some("Old Title"), "Old content").unwrap();
        let updated = NoteStore::update(
            forge.conn(),
            &created.id,
            Some("New Title"),
            Some("New content"),
        )
        .unwrap();
        assert_eq!(updated.title, Some("New Title".into()));
        assert_eq!(updated.content, "New content");
    }

    #[test]
    fn delete_note() {
        let forge = setup();
        let created = NoteStore::create(forge.conn(), Some("To Delete"), "content").unwrap();
        NoteStore::delete(forge.conn(), &created.id).unwrap();
        assert!(NoteStore::get(forge.conn(), &created.id).unwrap().is_none());
    }

    #[test]
    fn delete_missing_returns_not_found() {
        let forge = setup();
        let result = NoteStore::delete(forge.conn(), "nonexistent");
        assert!(matches!(result, Err(ForgeError::NotFound { .. })));
    }

    #[test]
    fn add_and_list_references() {
        let forge = setup();
        let note = NoteStore::create(forge.conn(), Some("Ref Note"), "content").unwrap();

        NoteStore::add_reference(
            forge.conn(),
            &note.id,
            NoteReferenceEntityType::Source,
            "source-123",
        )
        .unwrap();
        NoteStore::add_reference(
            forge.conn(),
            &note.id,
            NoteReferenceEntityType::Bullet,
            "bullet-456",
        )
        .unwrap();

        let refs = NoteStore::list_references(forge.conn(), &note.id).unwrap();
        assert_eq!(refs.len(), 2);
    }

    #[test]
    fn remove_reference() {
        let forge = setup();
        let note = NoteStore::create(forge.conn(), Some("Ref Note"), "content").unwrap();

        NoteStore::add_reference(
            forge.conn(),
            &note.id,
            NoteReferenceEntityType::Source,
            "source-123",
        )
        .unwrap();

        NoteStore::remove_reference(
            forge.conn(),
            &note.id,
            NoteReferenceEntityType::Source,
            "source-123",
        )
        .unwrap();

        let refs = NoteStore::list_references(forge.conn(), &note.id).unwrap();
        assert!(refs.is_empty());
    }

    #[test]
    fn remove_missing_reference_returns_not_found() {
        let forge = setup();
        let note = NoteStore::create(forge.conn(), Some("Note"), "content").unwrap();
        let result = NoteStore::remove_reference(
            forge.conn(),
            &note.id,
            NoteReferenceEntityType::Source,
            "nonexistent",
        );
        assert!(matches!(result, Err(ForgeError::NotFound { .. })));
    }

    #[test]
    fn find_by_entity() {
        let forge = setup();
        let note1 = NoteStore::create(forge.conn(), Some("Note 1"), "first").unwrap();
        let note2 = NoteStore::create(forge.conn(), Some("Note 2"), "second").unwrap();

        NoteStore::add_reference(
            forge.conn(),
            &note1.id,
            NoteReferenceEntityType::Source,
            "source-abc",
        )
        .unwrap();
        NoteStore::add_reference(
            forge.conn(),
            &note2.id,
            NoteReferenceEntityType::Source,
            "source-abc",
        )
        .unwrap();

        let notes =
            NoteStore::find_by_entity(forge.conn(), NoteReferenceEntityType::Source, "source-abc")
                .unwrap();
        assert_eq!(notes.len(), 2);
    }
}
