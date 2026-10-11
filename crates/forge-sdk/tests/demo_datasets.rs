//! Generated demo datasets (`packages/demo-data`) open with the Rust SDK as they are: no
//! migration is pending, the schema satisfies its foreign keys, and the file is marked as
//! generated.
//!
//! Ignored by default because it needs generated files. `just demo-data verify` runs it with
//! `FORGE_DEMO_DATA_DIR` set to the generator's output (default `data/demo`); every
//! `user/<uuid>/data.sqlite` under it is checked.

use std::fs;
use std::path::{Path, PathBuf};

use forge_sdk::db::migrate::run_migrations;
use forge_sdk::Forge;
use rusqlite::Connection;

/// Every `user/*/data.sqlite` under `dir`, sorted.
fn datasets(dir: &Path) -> Vec<PathBuf> {
    let users = dir.join("user");
    let entries =
        fs::read_dir(&users).unwrap_or_else(|e| panic!("cannot list {}: {e}", users.display()));
    let mut found: Vec<PathBuf> = entries
        .filter_map(|entry| {
            let file = entry.ok()?.path().join("data.sqlite");
            file.is_file().then_some(file)
        })
        .collect();
    found.sort();
    found
}

/// A copy of a dataset in the temp dir, removed (with any `-wal`/`-shm`) when dropped.
/// The checks run on a copy because `Forge::open` switches the file to WAL mode.
struct Scratch(PathBuf);

impl Scratch {
    fn copy_of(src: &Path, index: usize) -> Self {
        let dst = std::env::temp_dir().join(format!(
            "forge-demo-dataset-{}-{index}.sqlite",
            std::process::id()
        ));
        fs::copy(src, &dst)
            .unwrap_or_else(|e| panic!("copy {} to {}: {e}", src.display(), dst.display()));
        Self(dst)
    }

    fn path(&self) -> &str {
        self.0.to_str().expect("temp path is UTF-8")
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        for suffix in ["", "-wal", "-shm"] {
            let _ = fs::remove_file(format!("{}{suffix}", self.0.display()));
        }
    }
}

fn check(src: &Path, index: usize) {
    let name = src.display();
    let copy = Scratch::copy_of(src, index);

    // The generator's schema is the head this SDK knows: nothing left to apply.
    {
        let conn = Connection::open(copy.path()).unwrap_or_else(|e| panic!("{name}: open: {e}"));
        conn.execute_batch("PRAGMA foreign_keys = ON")
            .unwrap_or_else(|e| panic!("{name}: foreign_keys: {e}"));
        let applied =
            run_migrations(&conn).unwrap_or_else(|e| panic!("{name}: run_migrations: {e}"));
        assert_eq!(applied, 0, "{name}: {applied} migration(s) were pending");
    }

    // And it opens the way the app opens it.
    let forge = Forge::open(copy.path()).unwrap_or_else(|e| panic!("{name}: Forge::open: {e}"));
    let conn = forge.conn();

    let mut stmt = conn
        .prepare("PRAGMA foreign_key_check")
        .unwrap_or_else(|e| panic!("{name}: foreign_key_check: {e}"));
    let violations: Vec<(String, Option<i64>, String)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .and_then(|rows| rows.collect())
        .unwrap_or_else(|e| panic!("{name}: foreign_key_check: {e}"));
    assert!(
        violations.is_empty(),
        "{name}: foreign key violations (table, rowid, parent): {violations:?}"
    );

    let kind: String = conn
        .query_row(
            "SELECT value FROM dataset_meta WHERE key = 'kind'",
            [],
            |r| r.get(0),
        )
        .unwrap_or_else(|e| panic!("{name}: dataset_meta.kind: {e}"));
    assert_eq!(kind, "generated", "{name}: dataset_meta.kind");
}

#[test]
#[ignore = "needs FORGE_DEMO_DATA_DIR; run via just demo-data verify"]
fn demo_datasets_open_with_no_pending_migrations() {
    let dir = std::env::var_os("FORGE_DEMO_DATA_DIR")
        .expect("set FORGE_DEMO_DATA_DIR to the demo-data output directory (e.g. data/demo)");
    let found = datasets(Path::new(&dir));
    assert!(
        !found.is_empty(),
        "no user/*/data.sqlite under {}",
        Path::new(&dir).display()
    );
    for (index, src) in found.iter().enumerate() {
        check(src, index);
        println!("ok: {}", src.display());
    }
}
