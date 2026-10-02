# Database Migration Strategy

## Approach

Numbered SQL files executed in order. A `_migrations` table tracks which migrations have been applied.

## File Location

`packages/core/src/db/migrations/NNN_description.sql`

## Naming Convention

`001_initial.sql`, `002_add_projects.sql`, `003_add_search_index.sql`, etc.

Three-digit zero-padded prefix. Underscore separator. Lowercase snake_case description.

## Migration Runner (`packages/core/src/db/migrate.ts`)

```
1. Open SQLite database (create if not exists)
2. Set PRAGMA foreign_keys = ON
3. Set PRAGMA journal_mode = WAL
4. Create _migrations table if not exists
5. Read all .sql files from migrations/ directory, sorted by name
6. For each file not in _migrations:
   a. Begin transaction
   b. Execute SQL file contents
   c. Insert filename into _migrations
   d. Commit transaction
   e. Log: "Applied migration: NNN_description.sql"
7. If any migration fails: rollback transaction, log error, exit with non-zero
```

## Rules

1. Migrations are **append-only** — never edit an applied migration
2. Each migration is **idempotent within its transaction** — uses `CREATE TABLE IF NOT EXISTS` where appropriate
3. Migrations run **automatically on core server startup**
4. Migrations run in a **single transaction each** — a failed migration rolls back cleanly
5. The `_migrations` table is not subject to STRICT mode (uses INTEGER PK autoincrement)

## Rollback Strategy

MVP does not support automatic rollback. If a migration fails:
1. Fix the migration SQL
2. Delete the partially-applied migration from `_migrations` (if the transaction didn't roll back cleanly)
3. Restart the server

Post-MVP: add `down` migration files for reversibility.
