/**
 * Database connection helper.
 *
 * Creates and configures a bun:sqlite Database instance with the required
 * PRAGMAs (WAL journal mode, foreign keys ON). Does NOT run migrations --
 * callers should invoke `runMigrations()` separately after obtaining a
 * connection.
 */

import { Database } from "bun:sqlite";

/**
 * Open (or create) a SQLite database at `dbPath` and apply connection-level
 * PRAGMAs before returning it.
 *
 * Accepts `:memory:` for in-memory databases (useful in tests).
 */
export function getDatabase(dbPath: string): Database {
  const db = new Database(dbPath);

  // WAL must be set outside of any transaction; it persists across connections.
  db.exec("PRAGMA journal_mode = WAL");

  // Foreign-key enforcement must be enabled per-connection and before any
  // DML that could reference FK constraints.
  db.exec("PRAGMA foreign_keys = ON");

  // Wait for a competing writer instead of failing immediately with SQLITE_BUSY.
  // The Rust server (forge-server) can open the same file and sets the same 5s.
  db.exec("PRAGMA busy_timeout = 5000");

  return db;
}
