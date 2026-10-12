//! Map SQLite constraint failures to the service-layer `ForgeError` codes.
//!
//! TS source: `packages/core/src/storage/error-mapper.ts` (FK/enum/required →
//! `VALIDATION_ERROR`, unique/restrict → `CONFLICT`). The ELM checks those before writing;
//! here SQLite is the check, and this is the one place that translates its extended result
//! codes (<https://www.sqlite.org/rescode.html>). It covers forge-server and forge-wasm alike,
//! since both reach SQLite through rusqlite.
//!
//! This is the one piece of logic in `forge-core`: `From<rusqlite::Error>` has to live next to
//! `ForgeError` (orphan rule), and that is what lets every store's `?` get the mapping.

use super::common::ForgeError;

const SQLITE_CONSTRAINT_CHECK: i32 = 275;
const SQLITE_CONSTRAINT_FOREIGNKEY: i32 = 787;
const SQLITE_CONSTRAINT_NOTNULL: i32 = 1299;
const SQLITE_CONSTRAINT_PRIMARYKEY: i32 = 1555;
const SQLITE_CONSTRAINT_UNIQUE: i32 = 2067;
/// What SQLite reports for a blocked `ON DELETE RESTRICT` (not 787, despite the message).
const SQLITE_CONSTRAINT_TRIGGER: i32 = 1811;

/// The extended result code of a SQLite failure, if `e` is one.
pub fn extended_code(e: &rusqlite::Error) -> Option<i32> {
    match e {
        rusqlite::Error::SqliteFailure(err, _) => Some(err.extended_code),
        _ => None,
    }
}

/// True for a FOREIGN KEY failure (a dangling reference, or a blocked `ON DELETE RESTRICT`).
pub fn is_foreign_key(e: &rusqlite::Error) -> bool {
    extended_code(e) == Some(SQLITE_CONSTRAINT_FOREIGNKEY)
}

/// True for a UNIQUE or PRIMARY KEY failure.
pub fn is_unique(e: &rusqlite::Error) -> bool {
    matches!(
        extended_code(e),
        Some(SQLITE_CONSTRAINT_UNIQUE | SQLITE_CONSTRAINT_PRIMARYKEY)
    )
}

/// `(table, columns)` from SQLite's `"<kind> constraint failed: t.c1, t.c2"` message.
/// Only names are taken, never the raw text, so nothing else leaks into the response.
fn table_and_columns(e: &rusqlite::Error) -> (Option<String>, Vec<String>) {
    let msg = match e {
        rusqlite::Error::SqliteFailure(_, Some(m)) => m.as_str(),
        _ => return (None, vec![]),
    };
    let Some((_, list)) = msg.split_once("constraint failed: ") else {
        return (None, vec![]);
    };
    let mut table = None;
    let mut cols = vec![];
    for part in list.split(',') {
        if let Some((t, c)) = part.trim().split_once('.') {
            if is_ident(t) && is_ident(c) {
                table.get_or_insert_with(|| t.to_string());
                cols.push(c.to_string());
            }
        }
    }
    (table, cols)
}

fn is_ident(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn subject(table: Option<String>) -> String {
    table.unwrap_or_else(|| "record".into())
}

impl From<rusqlite::Error> for ForgeError {
    /// Insert/update mapping. A FOREIGN KEY failure here is a missing reference; for a blocked
    /// delete use [`ForgeError::from_sqlite_delete`], since SQLite uses the same code for both.
    fn from(e: rusqlite::Error) -> Self {
        match extended_code(&e) {
            Some(SQLITE_CONSTRAINT_FOREIGNKEY) => ForgeError::Validation {
                message: "referenced record does not exist".into(),
                field: None,
            },
            Some(SQLITE_CONSTRAINT_UNIQUE | SQLITE_CONSTRAINT_PRIMARYKEY) => {
                let (table, cols) = table_and_columns(&e);
                let what = if cols.is_empty() {
                    "value".to_string()
                } else {
                    cols.join(", ")
                };
                ForgeError::Conflict {
                    message: format!("{}: {what} already exists", subject(table)),
                }
            }
            Some(SQLITE_CONSTRAINT_NOTNULL) => {
                let (table, cols) = table_and_columns(&e);
                let col = cols.into_iter().next();
                ForgeError::Validation {
                    message: format!(
                        "{}: {} is required",
                        subject(table),
                        col.as_deref().unwrap_or("a required field")
                    ),
                    field: col,
                }
            }
            // Only the FK message: a trigger's own RAISE() is 1811 too, and stays a 500.
            Some(SQLITE_CONSTRAINT_TRIGGER)
                if matches!(&e, rusqlite::Error::SqliteFailure(_, Some(m))
                    if m == "FOREIGN KEY constraint failed") =>
            {
                ForgeError::Conflict {
                    message: "Cannot delete: other records depend on it".into(),
                }
            }
            Some(SQLITE_CONSTRAINT_CHECK) => ForgeError::Validation {
                message: "a value is not allowed".into(),
                field: None,
            },
            _ => ForgeError::Database { source: e },
        }
    }
}

impl ForgeError {
    /// Delete mapping: a FOREIGN KEY failure is a blocked `ON DELETE RESTRICT`, so `CONFLICT`.
    /// Everything else goes through the generic `From`.
    pub fn from_sqlite_delete(e: rusqlite::Error, entity: &str) -> Self {
        if is_foreign_key(&e) {
            ForgeError::Conflict {
                message: format!("Cannot delete {entity}: other records depend on it"),
            }
        } else {
            e.into()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn db() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE parent (id TEXT PRIMARY KEY, name TEXT NOT NULL UNIQUE,
                                  kind TEXT CHECK (kind IN ('a','b')));
             CREATE TABLE child (id TEXT PRIMARY KEY,
                                 parent_id TEXT NOT NULL REFERENCES parent(id) ON DELETE RESTRICT);
             INSERT INTO parent (id, name) VALUES ('p1', 'one');
             INSERT INTO child (id, parent_id) VALUES ('c1', 'p1');",
        )
        .unwrap();
        c
    }

    fn err(sql: &str) -> rusqlite::Error {
        db().execute(sql, []).unwrap_err()
    }

    #[test]
    fn unique_is_conflict_naming_table_and_column() {
        match ForgeError::from(err("INSERT INTO parent (id, name) VALUES ('p2', 'one')")) {
            ForgeError::Conflict { message } => assert_eq!(message, "parent: name already exists"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn primary_key_is_conflict() {
        let e = ForgeError::from(err("INSERT INTO parent (id, name) VALUES ('p1', 'two')"));
        assert_eq!(e.code(), "CONFLICT");
    }

    #[test]
    fn dangling_fk_is_validation() {
        let e = ForgeError::from(err(
            "INSERT INTO child (id, parent_id) VALUES ('c2', 'nope')",
        ));
        assert!(matches!(e, ForgeError::Validation { .. }), "{e:?}");
        assert_eq!(e.code(), "VALIDATION_ERROR");
    }

    #[test]
    fn check_is_validation_without_leaking_sql() {
        let e = ForgeError::from(err(
            "INSERT INTO parent (id, name, kind) VALUES ('p2', 'x', 'z')",
        ));
        assert_eq!(e.code(), "VALIDATION_ERROR");
        assert!(!e.to_string().contains("CHECK"), "{e}");
    }

    #[test]
    fn not_null_is_validation_with_field() {
        match ForgeError::from(err("INSERT INTO parent (id) VALUES ('p2')")) {
            ForgeError::Validation { message, field } => {
                assert_eq!(field.as_deref(), Some("name"));
                assert_eq!(message, "parent: name is required");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn restricted_delete_is_conflict_only_via_delete_mapping() {
        let e = err("DELETE FROM parent WHERE id = 'p1'");
        assert_eq!(
            ForgeError::from_sqlite_delete(e, "parent").code(),
            "CONFLICT"
        );
    }

    #[test]
    fn on_delete_restrict_is_conflict_even_without_the_delete_mapping() {
        // SQLite reports a blocked ON DELETE RESTRICT as 1811 (CONSTRAINT_TRIGGER), not 787.
        let e = err("DELETE FROM parent WHERE id = 'p1'");
        assert_eq!(extended_code(&e), Some(1811));
        assert_eq!(ForgeError::from(e).code(), "CONFLICT");
    }

    #[test]
    fn no_action_delete_needs_the_delete_mapping() {
        // Without RESTRICT the same blocked delete is a plain 787, identical to a dangling insert.
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE p (id PRIMARY KEY); CREATE TABLE c (pid REFERENCES p(id));
             INSERT INTO p VALUES (1); INSERT INTO c VALUES (1);",
        )
        .unwrap();
        let e = c.execute("DELETE FROM p", []).unwrap_err();
        assert_eq!(ForgeError::from(e).code(), "VALIDATION_ERROR");
        let e = c.execute("DELETE FROM p", []).unwrap_err();
        assert_eq!(ForgeError::from_sqlite_delete(e, "p").code(), "CONFLICT");
    }

    #[test]
    fn non_constraint_errors_stay_database() {
        let e = ForgeError::from(err("SELEC nonsense"));
        assert!(matches!(e, ForgeError::Database { .. }), "{e:?}");
        assert_eq!(e.code(), "DATABASE_ERROR");
        let busy = rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY),
            None,
        );
        assert!(matches!(
            ForgeError::from(busy),
            ForgeError::Database { .. }
        ));
    }

    #[test]
    fn foreign_key_variant_serializes_as_validation_error() {
        let e = ForgeError::ForeignKey {
            message: "x".into(),
        };
        assert_eq!(
            serde_json::to_value(&e).unwrap()["code"],
            "VALIDATION_ERROR"
        );
    }
}
