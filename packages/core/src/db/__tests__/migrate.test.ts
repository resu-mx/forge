import { describe, test, expect, beforeEach, afterEach } from "bun:test";
import type { Database } from "bun:sqlite";
import { mkdtempSync, writeFileSync, readFileSync, readdirSync } from "node:fs";
import { join, resolve } from "node:path";
import { tmpdir } from "node:os";
import { getDatabase } from "../connection";
import { runMigrations } from "../migrate";

/** Path to the real migrations directory in the source tree. */
const REAL_MIGRATIONS_DIR = resolve(import.meta.dir, "../migrations");

/**
 * Every migration the runner should apply: the `.sql` files in the migrations
 * directory, sorted, without the extension (the name `_migrations` records).
 * Read from disk so a new migration never needs this file edited.
 */
const EXPECTED_MIGRATIONS = readdirSync(REAL_MIGRATIONS_DIR)
  .filter((f) => f.endsWith(".sql"))
  .sort()
  .map((f) => f.replace(/\.sql$/, ""));

/**
 * Expected tables after all migrations (001 + 002 + 003).
 *
 * 002_schema_evolution:
 *   - employers -> organizations (employers dropped)
 *   - projects dropped (subsumed by source_projects)
 *   - resume_perspectives -> resume_entries (resume_perspectives dropped)
 *   - New tables: organizations, source_roles, source_projects,
 *     source_education, source_clearances, bullet_sources,
 *     resume_entries, user_notes, note_references, v1_import_map
 *
 * 003_renderer_and_entities:
 *   - New tables: domains, archetypes, archetype_domains
 */
const EXPECTED_TABLES = [
  "organizations",
  "sources",
  "prompt_logs",
  "bullets",
  // bullet_technologies: dropped in migration 031 (absorbed into bullet_skills)
  "perspectives",
  "skills",
  "skill_domains", // added in migration 031
  "bullet_skills",
  "perspective_skills",
  "resumes",
  "resume_entries",
  "source_roles",
  "source_projects",
  "source_education",
  // source_clearances: dropped in migration 037 (replaced by credentials)
  "bullet_sources",
  "user_notes",
  "note_references",
  "v1_import_map",
  "domains",
  "archetypes",
  "archetype_domains",
  "summaries",
  "resume_templates",
  "org_tags",
  "org_locations",
  "org_aliases",
  "source_skills",
  "job_description_skills",
  // clearance_access_programs: dropped in migration 037
  "contacts",
  "contact_organizations",
  "contact_job_descriptions",
  "contact_resumes",
  "industries", // added in migration 032
  "role_types", // added in migration 032
  "summary_skills", // added in migration 033
  "credentials", // added in migration 037
  "certifications", // added in migration 037
  "certification_skills", // added in migration 037
  "source_presentations", // added in migration 039
  "resume_certifications", // added in migration 041
  "answer_bank", // added in migration 050
  "extension_config", // added in migration 051
  "extension_logs", // added in migration 051
  "_migrations",
];

/**
 * Tables created by 001_initial.sql alone (used in the "broken 002" test
 * where only 001 is applied).
 */
const TABLES_001_ONLY = [
  "employers",
  "projects",
  "sources",
  "prompt_logs",
  "bullets",
  "bullet_technologies",
  "perspectives",
  "skills",
  "bullet_skills",
  "perspective_skills",
  "resumes",
  "resume_perspectives",
  "_migrations",
];

// ── Migration runner ──────────────────────────────────────────────────

describe("runMigrations", () => {
  let db: Database;

  beforeEach(() => {
    db = getDatabase(":memory:");
  });

  afterEach(() => {
    db.close();
  });

  test("fresh database: all migrations applied and all tables exist", () => {
    runMigrations(db, REAL_MIGRATIONS_DIR);

    const rows = db
      .query(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
      )
      .all() as { name: string }[];
    const tableNames = rows.map((r) => r.name);

    for (const table of EXPECTED_TABLES) {
      expect(tableNames).toContain(table);
    }
  });

  test("after migration 008, resume_templates table exists with 3 built-in rows", () => {
    runMigrations(db, REAL_MIGRATIONS_DIR);

    const rows = db
      .query("SELECT * FROM resume_templates WHERE is_builtin = 1")
      .all() as Array<{ name: string; is_builtin: number }>;
    expect(rows).toHaveLength(3);
  });

  test("fresh database: all migrations are recorded in _migrations", () => {
    runMigrations(db, REAL_MIGRATIONS_DIR);

    const rows = db
      .query("SELECT name FROM _migrations ORDER BY name")
      .all() as { name: string }[];
    expect(EXPECTED_MIGRATIONS[0]).toBe("001_initial");
    expect(rows.map((r) => r.name)).toEqual(EXPECTED_MIGRATIONS);
  });

  test("already up-to-date: running again is a no-op with no errors", () => {
    runMigrations(db, REAL_MIGRATIONS_DIR);
    // Second run should succeed silently.
    runMigrations(db, REAL_MIGRATIONS_DIR);

    // All migrations still recorded.
    const rows = db.query("SELECT name FROM _migrations ORDER BY name").all() as {
      name: string;
    }[];
    expect(rows.map((r) => r.name)).toEqual(EXPECTED_MIGRATIONS);
  });

  test("failed migration: broken 002 file rolls back; 001 is intact", () => {
    // Create a temporary migrations directory containing the real 001 and
    // a deliberately broken 002.
    const tmpDir = mkdtempSync(join(tmpdir(), "forge-migrate-"));

    // Copy 001_initial.sql verbatim.
    const sql001 = readFileSync(
      join(REAL_MIGRATIONS_DIR, "001_initial.sql"),
      "utf-8",
    );
    writeFileSync(join(tmpDir, "001_initial.sql"), sql001);

    // Write a broken 002 that references a nonexistent table.
    writeFileSync(
      join(tmpDir, "002_broken.sql"),
      "ALTER TABLE nonexistent_table ADD COLUMN oops TEXT;",
    );

    // Run migrations -- should apply 001, then fail on 002.
    expect(() => {
      runMigrations(db, tmpDir);
    }).toThrow();

    // 001 should have been committed before 002 was attempted.
    const applied = db.query("SELECT name FROM _migrations").all() as {
      name: string;
    }[];
    expect(applied.map((r) => r.name)).toContain("001_initial");
    expect(applied.map((r) => r.name)).not.toContain("002_broken");

    // All tables from 001 should exist (not rolled back).
    const rows = db
      .query(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
      )
      .all() as { name: string }[];
    const tableNames = rows.map((r) => r.name);
    for (const table of TABLES_001_ONLY) {
      expect(tableNames).toContain(table);
    }

    // Database is still usable -- we can insert a row.
    // (In this temp dir, only 001 was applied, so employers table still exists)
    const id = crypto.randomUUID();
    db.run("INSERT INTO employers (id, name) VALUES (?, ?)", [id, "Acme"]);
    const emp = db
      .query("SELECT name FROM employers WHERE id = ?")
      .get(id) as { name: string };
    expect(emp.name).toBe("Acme");
  });
});

// ── Connection helper ─────────────────────────────────────────────────

describe("getDatabase", () => {
  let db: Database;

  beforeEach(() => {
    db = getDatabase(":memory:");
  });

  afterEach(() => {
    db.close();
  });

  test("PRAGMA foreign_keys = ON is active", () => {
    const row = db.query("PRAGMA foreign_keys").get() as {
      foreign_keys: number;
    };
    expect(row.foreign_keys).toBe(1);
  });

  test("PRAGMA journal_mode = WAL is active", () => {
    // For :memory: databases, WAL may not persist (memory is always
    // WAL-compatible), but the PRAGMA should still report 'wal' or
    // 'memory'. Accept either as valid.
    const row = db.query("PRAGMA journal_mode").get() as {
      journal_mode: string;
    };
    expect(["wal", "memory"]).toContain(row.journal_mode);
  });

  test("foreign key enforcement: FK-violating insert is rejected", () => {
    // Apply migrations so we have tables with FK constraints.
    runMigrations(db, REAL_MIGRATIONS_DIR);

    // Attempt to insert a bullet_sources row with a nonexistent bullet_id.
    // After 002_schema_evolution, bullets no longer have source_id column;
    // the bullet_sources junction table has FK constraints instead.
    expect(() => {
      db.run(
        "INSERT INTO bullet_sources (bullet_id, source_id, is_primary) VALUES (?, ?, 1)",
        [crypto.randomUUID(), crypto.randomUUID()],
      );
    }).toThrow();
  });
});
