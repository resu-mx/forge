/**
 * The `migrate` script (resu-mx/forge#104). It used to load the module and exit 0 without
 * touching any database, so `just migrate` and the Docker test image's "migrate" step did nothing
 * and the seeds then found no tables.
 *
 * Two layers: `runMigrateCommand` in process (its logic and exit codes), and the real script as a
 * subprocess, which is exactly what `bun run migrate` and the Docker entrypoint execute.
 */

import { describe, test, expect, beforeEach, afterEach, spyOn } from "bun:test";
import { Database } from "bun:sqlite";
import { existsSync, mkdtempSync, mkdirSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { tmpdir } from "node:os";
import { DB_PATH_REQUIRED_MESSAGE } from "../connection";
import { runMigrateCommand } from "../migrate";

const MIGRATIONS_DIR = resolve(import.meta.dir, "../migrations");
const MIGRATE_TS = resolve(import.meta.dir, "../migrate.ts");
const CORE_DIR = resolve(import.meta.dir, "../../..");

/** Every migration the command should apply: the `.sql` files, without the extension. */
const EXPECTED = readdirSync(MIGRATIONS_DIR)
  .filter((f) => f.endsWith(".sql"))
  .sort()
  .map((f) => f.replace(/\.sql$/, ""));

let dir: string;
beforeEach(() => {
  dir = mkdtempSync(join(tmpdir(), "forge-migrate-cmd-"));
});
afterEach(() => {
  rmSync(dir, { recursive: true, force: true });
});

/** Names recorded in `_migrations`, read through a fresh connection. */
function recorded(dbPath: string): string[] {
  const db = new Database(dbPath, { readonly: true });
  try {
    return (db.query("SELECT name FROM _migrations ORDER BY name").all() as { name: string }[]).map(
      (r) => r.name,
    );
  } finally {
    db.close();
  }
}

/** Run `fn` with console output captured; returns what was logged and what was errored. */
function capture<T>(fn: () => T): { result: T; out: string[]; err: string[] } {
  const out: string[] = [];
  const err: string[] = [];
  const log = spyOn(console, "log").mockImplementation((...a: unknown[]) => void out.push(a.join(" ")));
  const error = spyOn(console, "error").mockImplementation((...a: unknown[]) => void err.push(a.join(" ")));
  try {
    return { result: fn(), out, err };
  } finally {
    log.mockRestore();
    error.mockRestore();
  }
}

describe("runMigrateCommand", () => {
  test("creates the database file, applies every migration, and lists them", () => {
    const dbPath = join(dir, "nested", "dir", "forge.db"); // parent directories do not exist yet
    const { result, out } = capture(() => runMigrateCommand({ FORGE_DB_PATH: dbPath }));

    expect(result).toBe(0);
    expect(existsSync(dbPath)).toBe(true);
    expect(recorded(dbPath)).toEqual(EXPECTED);
    expect(out).toContain("Applied migration: 001_initial.sql");
    expect(out.filter((l) => l.startsWith("Applied migration: "))).toHaveLength(EXPECTED.length);
  });

  test("an up-to-date database prints 'All migrations up to date' and exits 0", () => {
    const dbPath = join(dir, "forge.db");
    capture(() => runMigrateCommand({ FORGE_DB_PATH: dbPath }));

    const { result, out } = capture(() => runMigrateCommand({ FORGE_DB_PATH: dbPath }));
    expect(result).toBe(0);
    expect(out).toEqual(["All migrations up to date"]);
    expect(recorded(dbPath)).toEqual(EXPECTED);
  });

  test("an unset FORGE_DB_PATH exits 1 with the same message as the server, and creates nothing", () => {
    const { result, err } = capture(() => runMigrateCommand({}));
    expect(result).toBe(1);
    expect(err).toEqual([DB_PATH_REQUIRED_MESSAGE]);
    expect(readdirSync(dir)).toEqual([]);
  });

  test("a failing migration exits 1, names the file, and leaves earlier migrations applied", () => {
    const migrations = join(dir, "migrations");
    mkdirSync(migrations);
    writeFileSync(
      join(migrations, "001_initial.sql"),
      "CREATE TABLE _migrations (id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE);",
    );
    writeFileSync(join(migrations, "002_broken.sql"), "CREATE TABLE this is not valid sql;");
    const dbPath = join(dir, "forge.db");

    const { result, err } = capture(() => runMigrateCommand({ FORGE_DB_PATH: dbPath }, migrations));
    expect(result).toBe(1);
    expect(err.join("\n")).toContain("Migration failed: 002_broken.sql");
    // The failing migration was rolled back; the one before it stays.
    expect(recorded(dbPath)).toEqual(["001_initial"]);
  });
});

/** Run a real `bun` process, as `bun run migrate` and the Docker test image do. */
function bun(args: string[], env: Record<string, string | undefined>) {
  const proc = Bun.spawnSync([process.execPath, ...args], {
    cwd: CORE_DIR,
    env: { ...env, PATH: process.env.PATH },
  });
  return {
    code: proc.exitCode,
    stdout: new TextDecoder().decode(proc.stdout),
    stderr: new TextDecoder().decode(proc.stderr),
  };
}

describe("the migrate script, run as a process", () => {
  test("migrates a new database and exits 0 (was: exited 0 without creating the file)", () => {
    const dbPath = join(dir, "forge.db");
    const run = bun([MIGRATE_TS], { FORGE_DB_PATH: dbPath });

    expect(run.code).toBe(0);
    expect(existsSync(dbPath)).toBe(true);
    expect(recorded(dbPath)).toEqual(EXPECTED);
    expect(run.stdout).toContain("Applied migration: 001_initial.sql");
  });

  test("runs again as a no-op", () => {
    const dbPath = join(dir, "forge.db");
    bun([MIGRATE_TS], { FORGE_DB_PATH: dbPath });
    const again = bun([MIGRATE_TS], { FORGE_DB_PATH: dbPath });

    expect(again.code).toBe(0);
    expect(again.stdout.trim()).toBe("All migrations up to date");
  });

  test("exits 1 with the server's message when FORGE_DB_PATH is unset", () => {
    const run = bun([MIGRATE_TS], {});
    expect(run.code).toBe(1);
    expect(run.stderr).toContain(DB_PATH_REQUIRED_MESSAGE);
  });

  test("importing the module runs nothing, so the server and the tests are unaffected", () => {
    const dbPath = join(dir, "forge.db");
    const run = bun(
      ["-e", `import "./src/db/migrate.ts"; console.log("imported");`],
      { FORGE_DB_PATH: dbPath },
    );

    expect(run.code).toBe(0);
    expect(run.stdout.trim()).toBe("imported");
    expect(existsSync(dbPath)).toBe(false);
  });
});
