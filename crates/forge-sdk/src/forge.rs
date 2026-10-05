//! Forge facade — single entry point for all SDK operations.
//!
//! Mirrors the TS `createServices(db)` factory. Binary crates construct
//! one `Forge` instance and pass references to route handlers.

use forge_core::ForgeError;
use rusqlite::Connection;

use crate::db::migrate::run_migrations;

/// The main SDK entry point. Owns the database connection and provides
/// access to all domain services.
pub struct Forge {
    conn: Connection,
}

impl Forge {
    /// Open (or create) a Forge database at the given path.
    /// Runs all pending migrations automatically.
    pub fn open(path: &str) -> Result<Self, ForgeError> {
        let conn = Connection::open(path)?;
        Self::init(conn)
    }

    /// Open an in-memory database for testing.
    /// Runs all migrations so the schema is fully set up.
    pub fn open_memory() -> Result<Self, ForgeError> {
        let conn = Connection::open_in_memory()?;
        Self::init(conn)
    }

    /// Shared initialization: set PRAGMAs and run migrations.
    fn init(conn: Connection) -> Result<Self, ForgeError> {
        // Set pragmas before migrations (SQLite ignores foreign_keys inside transactions)
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;
             PRAGMA busy_timeout = 5000;",
        )?;
        run_migrations(&conn)?;
        Ok(Self { conn })
    }

    /// Get a reference to the underlying database connection.
    /// Used by repositories and services.
    pub fn conn(&self) -> &Connection {
        &self.conn
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_memory_succeeds() {
        let forge = Forge::open_memory().unwrap();
        // Verify foreign keys are on
        let fk: i32 = forge
            .conn()
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();
        assert_eq!(fk, 1);
    }

    #[test]
    fn open_memory_has_sources_table() {
        let forge = Forge::open_memory().unwrap();
        let exists: bool = forge
            .conn()
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='sources')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(exists);
    }
}
