//! What a database file says about its schema, and whether this bundle may open it.
//!
//! A snapshot written by a newer bundle can carry migrations this one has never heard of.
//! Opening it anyway would run the old code against a schema it does not understand, so
//! `importDatabase` refuses it, and the sync engine uses the same heads to decide what it
//! may pull. The check reads the incoming bytes in memory, so a refused file never touches
//! storage and the live database is left exactly as it was.
//!
//! Host-independent (no wasm-bindgen, no OPFS), so it is tested natively. `runtime` is the
//! only caller.

use std::borrow::Cow;
use std::io::Cursor;

use forge_sdk::db::migrate::{is_newer_than_known, known_head, schema_head};
use rusqlite::{Connection, MAIN_DB};
use serde_json::json;

/// First 16 bytes of every SQLite database file.
pub const SQLITE_HEADER: &[u8] = b"SQLite format 3\0";

/// The two file-format version bytes in the header: `1, 1` is a rollback-journal database,
/// `2, 2` is WAL.
const JOURNAL_BYTES: std::ops::Range<usize> = 18..20;

/// Why a file may not replace the live database. Each variant has a stable `code` the page
/// can switch on; the message is for people.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportRefusal {
    /// Does not start with the SQLite header.
    NotSqlite,
    /// Starts like SQLite but cannot be read (truncated, corrupt, encrypted).
    Unreadable(String),
    /// Written by a newer bundle: its newest migration is past this bundle's.
    SchemaTooNew { schema_head: String },
}

impl ImportRefusal {
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotSqlite => "not_sqlite",
            Self::Unreadable(_) => "invalid_database",
            Self::SchemaTooNew { .. } => "schema_too_new",
        }
    }

    pub fn message(&self) -> String {
        match self {
            Self::NotSqlite => "import rejected, current data kept: not a SQLite database file".into(),
            Self::Unreadable(why) => {
                format!("import rejected, current data kept: the file could not be read as a database ({why})")
            }
            Self::SchemaTooNew { schema_head } => format!(
                "import rejected, current data kept: this database was written by a newer version of Forge (schema {schema_head}, this version knows up to {}). Reload the page to get the latest version, then import again.",
                known_head()
            ),
        }
    }

    /// The JSON the runtime throws to JS: `{ code, message, known_head }`, plus
    /// `schema_head` when the file's head is known.
    pub fn to_json(&self) -> String {
        let mut v = json!({
            "code": self.code(),
            "message": self.message(),
            "known_head": known_head(),
        });
        if let Self::SchemaTooNew { schema_head } = self {
            v["schema_head"] = json!(schema_head);
        }
        v.to_string()
    }
}

/// The newest applied migration recorded in the SQLite file `bytes`, or `None` when it has
/// none. Reads a copy in memory; nothing is written anywhere.
pub fn head_of_bytes(bytes: &[u8]) -> Result<Option<String>, ImportRefusal> {
    if !bytes.starts_with(SQLITE_HEADER) {
        return Err(ImportRefusal::NotSqlite);
    }
    // A WAL-flagged header cannot be opened without its -wal file. The OPFS pool rewrites
    // those bytes to `1, 1` on import, so read the file the way it will be stored.
    let image: Cow<[u8]> = if bytes.len() >= JOURNAL_BYTES.end && bytes[JOURNAL_BYTES] != [1, 1] {
        let mut owned = bytes.to_vec();
        owned[JOURNAL_BYTES].copy_from_slice(&[1, 1]);
        Cow::Owned(owned)
    } else {
        Cow::Borrowed(bytes)
    };

    let unreadable = |e: &dyn std::fmt::Display| ImportRefusal::Unreadable(e.to_string());
    let mut conn = Connection::open_in_memory().map_err(|e| unreadable(&e))?;
    conn.deserialize_read_exact(MAIN_DB, Cursor::new(image.as_ref()), image.len(), true)
        .map_err(|e| unreadable(&e))?;
    schema_head(&conn).map_err(|e| unreadable(&e))
}

/// The schema fields `start`, `exportDatabase` and `importDatabase` report side by side:
/// the database's `schema_head` (null when unknown), this bundle's `known_head` and its
/// `bundle_version`. The sync engine sends the first and third with every upload and
/// compares the first two before it pulls.
pub fn schema_report(schema_head: Option<&str>) -> serde_json::Map<String, serde_json::Value> {
    let mut m = serde_json::Map::new();
    m.insert("schema_head".into(), json!(schema_head));
    m.insert("known_head".into(), json!(known_head()));
    m.insert("bundle_version".into(), json!(crate::BUNDLE_VERSION));
    m
}

/// Decide whether `bytes` may replace the live database. Returns the file's schema head.
pub fn check_import(bytes: &[u8]) -> Result<Option<String>, ImportRefusal> {
    let head = head_of_bytes(bytes)?;
    match head {
        Some(h) if is_newer_than_known(&h) => Err(ImportRefusal::SchemaTooNew { schema_head: h }),
        other => Ok(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_sdk::Forge;

    /// A real Forge database image, with `extra_migration` recorded as applied if given.
    fn database_image(extra_migration: Option<&str>) -> Vec<u8> {
        let forge = Forge::open_memory().unwrap();
        if let Some(name) = extra_migration {
            forge
                .conn()
                .execute("INSERT INTO _migrations (name) VALUES (?1)", [name])
                .unwrap();
        }
        forge.conn().serialize(MAIN_DB).unwrap().to_vec()
    }

    #[test]
    fn head_of_a_current_database_is_the_known_head() {
        let image = database_image(None);
        assert_eq!(
            head_of_bytes(&image).unwrap().as_deref(),
            Some(known_head())
        );
        assert_eq!(check_import(&image).unwrap().as_deref(), Some(known_head()));
    }

    #[test]
    fn a_database_from_a_newer_bundle_is_refused() {
        let image = database_image(Some("999_from_the_future"));
        let refusal = check_import(&image).unwrap_err();
        assert_eq!(
            refusal,
            ImportRefusal::SchemaTooNew {
                schema_head: "999_from_the_future".into()
            }
        );
        assert_eq!(refusal.code(), "schema_too_new");
        let message = refusal.message();
        assert!(message.contains("current data kept"), "{message}");
        assert!(message.contains("999_from_the_future"), "{message}");
        assert!(message.contains(known_head()), "{message}");
    }

    #[test]
    fn the_refusal_json_names_both_heads() {
        let refusal = check_import(&database_image(Some("999_from_the_future"))).unwrap_err();
        let v: serde_json::Value = serde_json::from_str(&refusal.to_json()).unwrap();
        assert_eq!(v["code"], "schema_too_new");
        assert_eq!(v["schema_head"], "999_from_the_future");
        assert_eq!(v["known_head"], known_head());
        assert!(v["message"]
            .as_str()
            .unwrap()
            .contains("newer version of Forge"));
    }

    #[test]
    fn an_older_database_is_accepted_and_migrated_later() {
        // Everything but the newest migration applied: the open that follows brings it up.
        let forge = Forge::open_memory().unwrap();
        forge
            .conn()
            .execute("DELETE FROM _migrations WHERE name = ?1", [known_head()])
            .unwrap();
        let image = forge.conn().serialize(MAIN_DB).unwrap().to_vec();
        let head = check_import(&image).unwrap().unwrap();
        assert!(head.as_str() < known_head());
    }

    #[test]
    fn a_sqlite_file_without_migrations_has_no_head_and_is_accepted() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE unrelated (x)").unwrap();
        let image = conn.serialize(MAIN_DB).unwrap().to_vec();
        assert_eq!(head_of_bytes(&image).unwrap(), None);
        assert_eq!(check_import(&image).unwrap(), None);
    }

    #[test]
    fn something_that_is_not_sqlite_is_refused_with_its_own_code() {
        for bytes in [&b""[..], &b"not a database"[..], &b"SQLite format 3"[..]] {
            let refusal = check_import(bytes).unwrap_err();
            assert_eq!(refusal, ImportRefusal::NotSqlite);
            assert_eq!(refusal.code(), "not_sqlite");
            assert!(refusal.message().contains("not a SQLite database file"));
        }
    }

    #[test]
    fn a_truncated_database_is_refused_as_unreadable() {
        let image = database_image(None);
        // Keep the header and the first page, lose the rest.
        let refusal = check_import(&image[..512]).unwrap_err();
        assert_eq!(refusal.code(), "invalid_database", "{refusal:?}");
        assert!(refusal.message().contains("current data kept"));
    }

    #[test]
    fn a_wal_flagged_header_is_read_as_the_pool_will_store_it() {
        let mut image = database_image(None);
        image[18] = 2;
        image[19] = 2;
        assert_eq!(
            head_of_bytes(&image).unwrap().as_deref(),
            Some(known_head())
        );
        // The caller's bytes are not modified.
        assert_eq!(&image[18..20], &[2, 2]);
    }

    #[test]
    fn the_schema_report_carries_the_three_fields() {
        let report = schema_report(Some("055_dataset_meta"));
        assert_eq!(report["schema_head"], "055_dataset_meta");
        assert_eq!(report["known_head"], known_head());
        assert_eq!(report["bundle_version"], crate::BUNDLE_VERSION);
        assert_eq!(report.len(), 3);
        // An unreadable database reports null, not a missing key.
        assert!(schema_report(None)["schema_head"].is_null());
    }

    #[test]
    fn reading_the_head_does_not_change_the_bytes() {
        let image = database_image(None);
        let copy = image.clone();
        head_of_bytes(&image).unwrap();
        assert_eq!(image, copy);
    }
}
