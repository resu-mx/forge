/**
 * Migration runner.
 *
 * Reads numbered `.sql` files from a migrations directory and applies them
 * in filename order, tracking each applied migration in a `_migrations`
 * table so that subsequent runs are no-ops for already-applied files.
 *
 * Each migration file is executed inside its own transaction. If execution
 * fails the transaction is rolled back and the error is re-thrown, leaving
 * the database in the state it was in before that migration started.
 */

import { Database } from "bun:sqlite";
import { mkdirSync, readdirSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { DB_PATH_REQUIRED_MESSAGE, getDatabase } from "./connection";
import { migrateHeadersToSummaries } from './migrations/006_summaries_data'

/**
 * Check whether the `_migrations` table already exists.
 */
function migrationsTableExists(db: Database): boolean {
  const row = db
    .query(
      "SELECT name FROM sqlite_master WHERE type = 'table' AND name = '_migrations'",
    )
    .get() as { name: string } | null;
  return row !== null;
}

/**
 * Return the set of migration names that have already been recorded.
 *
 * Migration names are stored WITHOUT the `.sql` extension (e.g.
 * `001_initial`, not `001_initial.sql`), matching the convention used
 * inside the migration files themselves.
 */
function getAppliedMigrations(db: Database): Set<string> {
  if (!migrationsTableExists(db)) {
    return new Set();
  }
  const rows = db.query("SELECT name FROM _migrations").all() as {
    name: string;
  }[];
  return new Set(rows.map((r) => r.name));
}

/**
 * Derive the migration name from a filename by stripping the `.sql` suffix.
 */
function migrationName(filename: string): string {
  return filename.replace(/\.sql$/, "");
}

/**
 * Apply all unapplied `.sql` migrations found in `migrationsDir`.
 *
 * Migrations are sorted lexicographically by filename (the three-digit
 * zero-padded prefix ensures correct ordering). Each file is executed in
 * a single transaction so that a failure in one migration does not leave
 * the database in a partially-applied state.
 *
 * The `_migrations` table is expected to be created by the initial
 * migration file itself (001_initial.sql). This runner does NOT create it
 * ahead of time, because the migration file uses `CREATE TABLE` (without
 * `IF NOT EXISTS`) and would conflict.
 */
export function runMigrations(db: Database, migrationsDir: string): void {
  // Discover migration files and sort by name.
  const files = readdirSync(migrationsDir)
    .filter((f) => f.endsWith(".sql"))
    .sort();

  const applied = getAppliedMigrations(db);

  const pending = files.filter((f) => !applied.has(migrationName(f)));

  if (pending.length === 0) {
    console.log("All migrations up to date");
    return;
  }

  for (const file of pending) {
    const sql = readFileSync(join(migrationsDir, file), "utf-8");
    const name = migrationName(file);

    // Table-rebuild migrations need PRAGMA foreign_keys = OFF, which must be
    // set OUTSIDE any transaction (SQLite silently ignores it inside BEGIN).
    // Detect these migrations by their PRAGMA statement and handle accordingly.
    const needsFkOff = sql.includes("PRAGMA foreign_keys = OFF");
    if (needsFkOff) {
      db.exec("PRAGMA foreign_keys = OFF");
    }

    try {
      db.exec("BEGIN");
      db.exec(sql);

      // Record the migration. The SQL file may already insert its own
      // record (as 001_initial.sql does), so use INSERT OR IGNORE to
      // avoid a UNIQUE constraint failure on the name column.
      db.run("INSERT OR IGNORE INTO _migrations (name) VALUES (?)", [name]);

      db.exec("COMMIT");
      if (needsFkOff) {
        db.exec("PRAGMA foreign_keys = ON");
      }
      console.log(`Applied migration: ${file}`);

      // Run companion TypeScript data migrations
      if (name === '006_summaries') {
        try {
          const result = migrateHeadersToSummaries(db)
          console.log(`  Data migration: ${result.migrated} summaries created, ${result.skipped} resumes skipped`)
        } catch (err) {
          console.error(`  Data migration failed for ${name}:`, err)
          throw err
        }
      }
    } catch (err) {
      try {
        db.exec("ROLLBACK");
      } catch {
        // Rollback may fail if the transaction was already aborted by SQLite.
      }
      if (needsFkOff) {
        db.exec("PRAGMA foreign_keys = ON");
      }
      console.error(`Migration failed: ${file}`);
      throw err;
    }
  }
}

/** Where the migration files live: the same directory the server migrates from at boot. */
const MIGRATIONS_DIR = resolve(import.meta.dir, "migrations");

/**
 * The `migrate` script (`bun run --filter @forge/core migrate`, `just migrate`, the Docker test
 * image). Does what the server does at boot: opens `FORGE_DB_PATH` with the standard pragmas
 * (creating the file and its directory), applies every pending migration, and reports each one.
 *
 * Returns the process exit code: 0 on success (including "already up to date"), 1 when
 * `FORGE_DB_PATH` is unset or a migration fails. `runMigrations` is unchanged and still rolls
 * back the failing migration.
 *
 * `env` and `migrationsDir` are parameters so the command can be tested without a process.
 */
export function runMigrateCommand(
  env: Record<string, string | undefined> = process.env,
  migrationsDir: string = MIGRATIONS_DIR,
): 0 | 1 {
  const raw = env.FORGE_DB_PATH;
  if (!raw) {
    console.error(DB_PATH_REQUIRED_MESSAGE);
    return 1;
  }

  const dbPath = resolve(raw);
  mkdirSync(dirname(dbPath), { recursive: true });
  const db = getDatabase(dbPath);
  try {
    runMigrations(db, migrationsDir);
    return 0;
  } catch (err) {
    // `runMigrations` already printed which file failed; say why.
    console.error(err instanceof Error ? err.message : String(err));
    return 1;
  } finally {
    db.close();
  }
}

// Run only when this file is executed directly, never when it is imported (the server and the
// tests import `runMigrations` from here and must not migrate as a side effect).
if (import.meta.main) {
  process.exit(runMigrateCommand());
}
