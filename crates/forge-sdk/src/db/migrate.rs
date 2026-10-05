//! Migration runner — embeds SQL migration files from the TS codebase
//! and applies them in order, tracking state in `_migrations`.

use forge_core::ForgeError;
use rusqlite::Connection;

/// Embedded SQL migrations in filename-sorted order.
/// Source: `packages/core/src/db/migrations/*.sql`
const MIGRATIONS: &[(&str, &str)] = &[
    ("001_initial", include_str!("../../../../packages/core/src/db/migrations/001_initial.sql")),
    ("002_schema_evolution", include_str!("../../../../packages/core/src/db/migrations/002_schema_evolution.sql")),
    ("003_renderer_and_entities", include_str!("../../../../packages/core/src/db/migrations/003_renderer_and_entities.sql")),
    ("004_resume_sections", include_str!("../../../../packages/core/src/db/migrations/004_resume_sections.sql")),
    ("005_user_profile", include_str!("../../../../packages/core/src/db/migrations/005_user_profile.sql")),
    ("006_summaries", include_str!("../../../../packages/core/src/db/migrations/006_summaries.sql")),
    ("007_job_descriptions", include_str!("../../../../packages/core/src/db/migrations/007_job_descriptions.sql")),
    ("008_resume_templates", include_str!("../../../../packages/core/src/db/migrations/008_resume_templates.sql")),
    ("009_education_subtype_fields", include_str!("../../../../packages/core/src/db/migrations/009_education_subtype_fields.sql")),
    ("010_education_org_fk", include_str!("../../../../packages/core/src/db/migrations/010_education_org_fk.sql")),
    ("011_org_tags", include_str!("../../../../packages/core/src/db/migrations/011_org_tags.sql")),
    ("012_org_kanban_statuses", include_str!("../../../../packages/core/src/db/migrations/012_org_kanban_statuses.sql")),
    ("013_org_campuses", include_str!("../../../../packages/core/src/db/migrations/013_org_campuses.sql")),
    ("014_campus_zipcode_hq", include_str!("../../../../packages/core/src/db/migrations/014_campus_zipcode_hq.sql")),
    ("015_org_aliases", include_str!("../../../../packages/core/src/db/migrations/015_org_aliases.sql")),
    ("016_source_skills", include_str!("../../../../packages/core/src/db/migrations/016_source_skills.sql")),
    ("018_job_description_skills", include_str!("../../../../packages/core/src/db/migrations/018_job_description_skills.sql")),
    ("019_clearance_structured_data", include_str!("../../../../packages/core/src/db/migrations/019_clearance_structured_data.sql")),
    ("020_stale_education_text_cleanup", include_str!("../../../../packages/core/src/db/migrations/020_stale_education_text_cleanup.sql")),
    ("021_drop_legacy_education_columns", include_str!("../../../../packages/core/src/db/migrations/021_drop_legacy_education_columns.sql")),
    ("022_drop_legacy_org_location_columns", include_str!("../../../../packages/core/src/db/migrations/022_drop_legacy_org_location_columns.sql")),
    ("023_contacts", include_str!("../../../../packages/core/src/db/migrations/023_contacts.sql")),
    ("024_unified_kanban_statuses", include_str!("../../../../packages/core/src/db/migrations/024_unified_kanban_statuses.sql")),
    ("025_embeddings", include_str!("../../../../packages/core/src/db/migrations/025_embeddings.sql")),
    ("026_job_description_resumes", include_str!("../../../../packages/core/src/db/migrations/026_job_description_resumes.sql")),
    ("027_salary_structured_fields", include_str!("../../../../packages/core/src/db/migrations/027_salary_structured_fields.sql")),
    ("028_jd_pipeline_statuses", include_str!("../../../../packages/core/src/db/migrations/028_jd_pipeline_statuses.sql")),
    ("029_prompt_logs_jd_entity_type", include_str!("../../../../packages/core/src/db/migrations/029_prompt_logs_jd_entity_type.sql")),
    ("031_skills_expansion", include_str!("../../../../packages/core/src/db/migrations/031_skills_expansion.sql")),
    ("032_industries_role_types", include_str!("../../../../packages/core/src/db/migrations/032_industries_role_types.sql")),
    ("033_summary_structured_fields", include_str!("../../../../packages/core/src/db/migrations/033_summary_structured_fields.sql")),
    ("034_resume_entry_source_id", include_str!("../../../../packages/core/src/db/migrations/034_resume_entry_source_id.sql")),
    ("035_resume_tagline_engine", include_str!("../../../../packages/core/src/db/migrations/035_resume_tagline_engine.sql")),
    ("036_null_auto_content_on_direct_source_entries", include_str!("../../../../packages/core/src/db/migrations/036_null_auto_content_on_direct_source_entries.sql")),
    ("037_qualifications", include_str!("../../../../packages/core/src/db/migrations/037_qualifications.sql")),
    ("038_resume_summary_override", include_str!("../../../../packages/core/src/db/migrations/038_resume_summary_override.sql")),
    ("039_presentations", include_str!("../../../../packages/core/src/db/migrations/039_presentations.sql")),
    ("040_resume_show_clearance_header", include_str!("../../../../packages/core/src/db/migrations/040_resume_show_clearance_header.sql")),
    ("041_skill_categories_and_seeds", include_str!("../../../../packages/core/src/db/migrations/041_skill_categories_and_seeds.sql")),
    ("042_project_open_source", include_str!("../../../../packages/core/src/db/migrations/042_project_open_source.sql")),
    ("043_cert_schema_rework", include_str!("../../../../packages/core/src/db/migrations/043_cert_schema_rework.sql")),
    ("044_skill_categories", include_str!("../../../../packages/core/src/db/migrations/044_skill_categories.sql")),
    ("045_pending_derivations", include_str!("../../../../packages/core/src/db/migrations/045_pending_derivations.sql")),
    ("046_profile_addresses_urls", include_str!("../../../../packages/core/src/db/migrations/046_profile_addresses_urls.sql")),
    ("047_org_locations", include_str!("../../../../packages/core/src/db/migrations/047_org_locations.sql")),
    ("048_notes_normalization", include_str!("../../../../packages/core/src/db/migrations/048_notes_normalization.sql")),
    ("049_jd_parsed_fields", include_str!("../../../../packages/core/src/db/migrations/049_jd_parsed_fields.sql")),
    ("050_answer_bank", include_str!("../../../../packages/core/src/db/migrations/050_answer_bank.sql")),
    ("051_extension_infra", include_str!("../../../../packages/core/src/db/migrations/051_extension_infra.sql")),
    ("052_skill_graph_schema", include_str!("../../../../packages/core/src/db/migrations/052_skill_graph_schema.sql")),
    ("053_skill_graph_initial_population", include_str!("../../../../packages/core/src/db/migrations/053_skill_graph_initial_population.sql")),
    ("054_alignment_results", include_str!("../../../../packages/core/src/db/migrations/054_alignment_results.sql")),
];

/// Run all pending migrations against the given connection.
///
/// Each migration runs in its own transaction. Migrations that contain
/// `PRAGMA foreign_keys = OFF` get that pragma set outside the transaction
/// (SQLite ignores it inside BEGIN/COMMIT).
pub fn run_migrations(conn: &Connection) -> Result<usize, ForgeError> {
    let applied = get_applied(conn)?;
    let mut count = 0;

    for &(name, sql) in MIGRATIONS {
        if applied.contains(&name.to_string()) {
            continue;
        }

        let needs_fk_off = sql.contains("PRAGMA foreign_keys = OFF");
        if needs_fk_off {
            conn.execute_batch("PRAGMA foreign_keys = OFF")?;
        }

        conn.execute_batch("BEGIN")?;
        match conn.execute_batch(sql) {
            Ok(()) => {
                conn.execute(
                    "INSERT OR IGNORE INTO _migrations (name) VALUES (?1)",
                    rusqlite::params![name],
                )?;
                conn.execute_batch("COMMIT")?;
            }
            Err(e) => {
                let _ = conn.execute_batch("ROLLBACK");
                if needs_fk_off {
                    let _ = conn.execute_batch("PRAGMA foreign_keys = ON");
                }
                return Err(e.into());
            }
        }

        if needs_fk_off {
            conn.execute_batch("PRAGMA foreign_keys = ON")?;
        }
        count += 1;
    }

    Ok(count)
}

/// Get the set of migration names already applied.
fn get_applied(conn: &Connection) -> Result<Vec<String>, ForgeError> {
    // Check if _migrations table exists
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='_migrations')",
        [],
        |row| row.get(0),
    )?;

    if !exists {
        return Ok(Vec::new());
    }

    let mut stmt = conn.prepare("SELECT name FROM _migrations")?;
    let names = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_run_on_empty_db() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON").unwrap();
        let count = run_migrations(&conn).unwrap();
        assert_eq!(count, MIGRATIONS.len());

        // Verify _migrations table has all entries
        let applied = get_applied(&conn).unwrap();
        assert_eq!(applied.len(), MIGRATIONS.len());
    }

    #[test]
    fn migrations_are_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON").unwrap();

        let first = run_migrations(&conn).unwrap();
        assert_eq!(first, MIGRATIONS.len());

        let second = run_migrations(&conn).unwrap();
        assert_eq!(second, 0);
    }

    #[test]
    fn sources_table_exists_after_migration() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON").unwrap();
        run_migrations(&conn).unwrap();

        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='sources')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(exists);
    }

    // ------------------------------------------------------------------
    // 052_skill_graph_schema
    // ------------------------------------------------------------------

    fn fresh_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON").unwrap();
        run_migrations(&conn).unwrap();
        conn
    }

    fn insert_node(conn: &Connection, id: &str, name: &str) {
        conn.execute(
            "INSERT INTO skill_graph_nodes (id, canonical_name) VALUES (?1, ?2)",
            rusqlite::params![id, name],
        )
        .unwrap();
    }

    #[test]
    fn skill_graph_tables_exist_after_migration() {
        let conn = fresh_db();

        for table in ["skill_graph_nodes", "skill_graph_edges"] {
            let exists: bool = conn
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
                    rusqlite::params![table],
                    |row| row.get(0),
                )
                .unwrap();
            assert!(exists, "expected table {table} to exist");
        }
    }

    #[test]
    fn skill_graph_nodes_defaults_and_unique_name() {
        let conn = fresh_db();

        let node_id = "11111111-1111-4111-8111-111111111111";
        insert_node(&conn, node_id, "Kubernetes");

        let (category, aliases, source, confidence): (String, String, String, f64) = conn
            .query_row(
                "SELECT category, aliases, source, confidence FROM skill_graph_nodes WHERE id = ?1",
                rusqlite::params![node_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(category, "other");
        assert_eq!(aliases, "[]");
        assert_eq!(source, "extracted");
        assert!((confidence - 1.0).abs() < f64::EPSILON);

        // canonical_name UNIQUE
        let dup_id = "22222222-2222-4222-8222-222222222222";
        let err = conn.execute(
            "INSERT INTO skill_graph_nodes (id, canonical_name) VALUES (?1, ?2)",
            rusqlite::params![dup_id, "Kubernetes"],
        );
        assert!(err.is_err(), "duplicate canonical_name must be rejected");
    }

    #[test]
    fn skill_graph_nodes_category_fk_and_source_check() {
        let conn = fresh_db();

        // FK to skill_categories(slug) — bogus category must be rejected.
        let err = conn.execute(
            "INSERT INTO skill_graph_nodes (id, canonical_name, category) VALUES (?1, ?2, ?3)",
            rusqlite::params![
                "33333333-3333-4333-8333-333333333333",
                "Bad Category Skill",
                "not-a-real-category",
            ],
        );
        assert!(err.is_err(), "non-existent category slug must be rejected");

        // CHECK on source — invalid value must be rejected.
        let err = conn.execute(
            "INSERT INTO skill_graph_nodes (id, canonical_name, source) VALUES (?1, ?2, ?3)",
            rusqlite::params![
                "44444444-4444-4444-8444-444444444444",
                "Bad Source Skill",
                "imagined",
            ],
        );
        assert!(err.is_err(), "invalid source enum value must be rejected");
    }

    #[test]
    fn skill_graph_nodes_id_length_check() {
        let conn = fresh_db();

        let err = conn.execute(
            "INSERT INTO skill_graph_nodes (id, canonical_name) VALUES (?1, ?2)",
            rusqlite::params!["short-id", "Bad ID Skill"],
        );
        assert!(err.is_err(), "id length CHECK must reject short ids");
    }

    #[test]
    fn skill_graph_edges_accepts_all_edge_types() {
        let conn = fresh_db();

        let a = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
        let b = "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb";
        insert_node(&conn, a, "A");
        insert_node(&conn, b, "B");

        for edge_type in [
            "alias-of",
            "parent-of",
            "child-of",
            "prerequisite",
            "related-to",
            "co-occurs",
            "platform-for",
        ] {
            conn.execute(
                "INSERT INTO skill_graph_edges (source_id, target_id, edge_type) VALUES (?1, ?2, ?3)",
                rusqlite::params![a, b, edge_type],
            )
            .unwrap_or_else(|e| panic!("{edge_type} should be a valid edge type: {e}"));
        }

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM skill_graph_edges WHERE source_id = ?1 AND target_id = ?2",
                rusqlite::params![a, b],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 7);
    }

    #[test]
    fn skill_graph_edges_rejects_invalid_edge_type() {
        let conn = fresh_db();

        let a = "cccccccc-cccc-4ccc-8ccc-cccccccccccc";
        let b = "dddddddd-dddd-4ddd-8ddd-dddddddddddd";
        insert_node(&conn, a, "C");
        insert_node(&conn, b, "D");

        let err = conn.execute(
            "INSERT INTO skill_graph_edges (source_id, target_id, edge_type) VALUES (?1, ?2, ?3)",
            rusqlite::params![a, b, "not-an-edge-type"],
        );
        assert!(err.is_err(), "edge_type CHECK must reject unknown types");
    }

    #[test]
    fn skill_graph_edges_rejects_self_loops() {
        let conn = fresh_db();

        let a = "eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee";
        insert_node(&conn, a, "E");

        let err = conn.execute(
            "INSERT INTO skill_graph_edges (source_id, target_id, edge_type) VALUES (?1, ?1, ?2)",
            rusqlite::params![a, "alias-of"],
        );
        assert!(err.is_err(), "self-loop CHECK must reject source==target");
    }

    #[test]
    fn skill_graph_edges_composite_pk_dedups_by_type() {
        let conn = fresh_db();

        let a = "ffffffff-ffff-4fff-8fff-ffffffffffff";
        let b = "99999999-9999-4999-8999-999999999999";
        insert_node(&conn, a, "F");
        insert_node(&conn, b, "G");

        // Same (source, target) with different edge types is allowed.
        conn.execute(
            "INSERT INTO skill_graph_edges (source_id, target_id, edge_type) VALUES (?1, ?2, 'parent-of')",
            rusqlite::params![a, b],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO skill_graph_edges (source_id, target_id, edge_type) VALUES (?1, ?2, 'prerequisite')",
            rusqlite::params![a, b],
        )
        .unwrap();

        // Same (source, target, edge_type) is rejected.
        let err = conn.execute(
            "INSERT INTO skill_graph_edges (source_id, target_id, edge_type) VALUES (?1, ?2, 'parent-of')",
            rusqlite::params![a, b],
        );
        assert!(err.is_err(), "composite PK must reject duplicate edge");
    }

    #[test]
    fn skill_graph_edges_cascade_on_node_delete() {
        let conn = fresh_db();

        let a = "12121212-1212-4121-8121-121212121212";
        let b = "34343434-3434-4343-8343-343434343434";
        insert_node(&conn, a, "Cascade A");
        insert_node(&conn, b, "Cascade B");
        conn.execute(
            "INSERT INTO skill_graph_edges (source_id, target_id, edge_type) VALUES (?1, ?2, 'related-to')",
            rusqlite::params![a, b],
        )
        .unwrap();

        conn.execute(
            "DELETE FROM skill_graph_nodes WHERE id = ?1",
            rusqlite::params![a],
        )
        .unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM skill_graph_edges WHERE source_id = ?1 OR target_id = ?1",
                rusqlite::params![a],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            count, 0,
            "edges must cascade when a referenced node is deleted"
        );
    }

    // ------------------------------------------------------------------
    // 053_skill_graph_initial_population
    // ------------------------------------------------------------------

    /// Insert a legacy skill row and return its id.
    fn insert_legacy_skill(conn: &Connection, id: &str, name: &str, category: &str) {
        conn.execute(
            "INSERT INTO skills (id, name, category) VALUES (?1, ?2, ?3)",
            rusqlite::params![id, name, category],
        )
        .unwrap();
    }

    /// Set up a database that has migrations 1..=052 applied but NOT 053,
    /// then insert legacy skills + a representative bullet/junction so we can
    /// verify the population migration in isolation.
    fn db_through_052_with_legacy_skills() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON").unwrap();

        // Apply every migration up to and INCLUDING 052, but stop before 053.
        for &(name, sql) in MIGRATIONS {
            let needs_fk_off = sql.contains("PRAGMA foreign_keys = OFF");
            if needs_fk_off {
                conn.execute_batch("PRAGMA foreign_keys = OFF").unwrap();
            }
            conn.execute_batch("BEGIN").unwrap();
            conn.execute_batch(sql).unwrap();
            conn.execute(
                "INSERT OR IGNORE INTO _migrations (name) VALUES (?1)",
                rusqlite::params![name],
            )
            .unwrap();
            conn.execute_batch("COMMIT").unwrap();
            if needs_fk_off {
                conn.execute_batch("PRAGMA foreign_keys = ON").unwrap();
            }
            if name == "052_skill_graph_schema" {
                break;
            }
        }

        insert_legacy_skill(
            &conn,
            "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaa01",
            "Python",
            "language",
        );
        insert_legacy_skill(
            &conn,
            "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaa02",
            "Rust",
            "language",
        );
        insert_legacy_skill(
            &conn,
            "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaa03",
            "Docker",
            "tool",
        );

        conn
    }

    /// Apply only the 053 migration to a connection that's already at 052.
    fn apply_053(conn: &Connection) {
        let sql = MIGRATIONS
            .iter()
            .find(|(n, _)| *n == "053_skill_graph_initial_population")
            .map(|(_, s)| *s)
            .unwrap();
        conn.execute_batch("BEGIN").unwrap();
        conn.execute_batch(sql).unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO _migrations (name) VALUES ('053_skill_graph_initial_population')",
            [],
        )
        .unwrap();
        conn.execute_batch("COMMIT").unwrap();
    }

    #[test]
    fn population_mirrors_each_legacy_skill_as_a_node() {
        let conn = db_through_052_with_legacy_skills();
        apply_053(&conn);

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM skill_graph_nodes WHERE legacy_skill_id IS NOT NULL",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            count, 3,
            "every legacy skill should produce one mirror node"
        );

        // legacy_skill_id should equal id for migrated rows so junctions JOIN cleanly.
        let mismatched: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM skill_graph_nodes WHERE legacy_skill_id IS NOT NULL AND legacy_skill_id <> id",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            mismatched, 0,
            "migrated nodes must reuse the legacy skill UUID"
        );

        // Source and category propagate.
        let (name, category, source): (String, String, String) = conn
            .query_row(
                "SELECT canonical_name, category, source FROM skill_graph_nodes WHERE legacy_skill_id = ?1",
                rusqlite::params!["aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaa01"],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(name, "Python");
        assert_eq!(category, "language");
        assert_eq!(source, "seed");
    }

    #[test]
    fn population_creates_one_root_per_category() {
        let conn = db_through_052_with_legacy_skills();

        let category_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM skill_categories", [], |r| r.get(0))
            .unwrap();
        assert!(
            category_count > 0,
            "skill_categories should be seeded by 044"
        );

        apply_053(&conn);

        let root_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM skill_graph_nodes WHERE description LIKE 'category-root:%'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(root_count, category_count, "one root node per category");

        // Roots must have category = 'concept' and no legacy_skill_id.
        let bad_roots: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM skill_graph_nodes \
                 WHERE description LIKE 'category-root:%' \
                   AND (category <> 'concept' OR legacy_skill_id IS NOT NULL)",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            bad_roots, 0,
            "category roots must be concept-tagged with no legacy id"
        );
    }

    #[test]
    fn population_emits_parent_of_edges_from_category_roots_to_skills() {
        let conn = db_through_052_with_legacy_skills();
        apply_053(&conn);

        // Each migrated skill should have exactly one incoming parent-of edge
        // (from its category root).
        let orphan_skills: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM skill_graph_nodes n \
                 WHERE n.legacy_skill_id IS NOT NULL \
                   AND NOT EXISTS ( \
                     SELECT 1 FROM skill_graph_edges e \
                     WHERE e.target_id = n.id AND e.edge_type = 'parent-of' \
                   )",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            orphan_skills, 0,
            "every migrated skill must have a parent-of edge from its category root"
        );

        // The parent of "Python" should be the "Languages" root.
        let parent_name: String = conn
            .query_row(
                "SELECT root.canonical_name \
                 FROM skill_graph_edges e \
                 JOIN skill_graph_nodes node ON node.id = e.target_id \
                 JOIN skill_graph_nodes root ON root.id = e.source_id \
                 WHERE node.legacy_skill_id = ?1 AND e.edge_type = 'parent-of'",
                rusqlite::params!["aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaa01"],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(parent_name, "Languages");
    }

    #[test]
    fn population_is_idempotent() {
        let conn = db_through_052_with_legacy_skills();
        apply_053(&conn);

        let first_node_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM skill_graph_nodes", [], |r| r.get(0))
            .unwrap();
        let first_edge_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM skill_graph_edges", [], |r| r.get(0))
            .unwrap();

        // Re-run the population SQL (without re-recording in _migrations).
        let sql = MIGRATIONS
            .iter()
            .find(|(n, _)| *n == "053_skill_graph_initial_population")
            .map(|(_, s)| *s)
            .unwrap();
        conn.execute_batch(sql).unwrap();

        let second_node_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM skill_graph_nodes", [], |r| r.get(0))
            .unwrap();
        let second_edge_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM skill_graph_edges", [], |r| r.get(0))
            .unwrap();

        assert_eq!(
            first_node_count, second_node_count,
            "node insert must be idempotent"
        );
        assert_eq!(
            first_edge_count, second_edge_count,
            "edge insert must be idempotent"
        );
    }

    #[test]
    fn legacy_junction_tables_still_resolve_after_population() {
        // Build a DB through 052, insert a skill + a bullet that references it,
        // run 053, then verify the junction still resolves both forward (legacy)
        // and via the new graph node (which shares the same UUID).
        let conn = db_through_052_with_legacy_skills();

        // Insert a minimal source + bullet so we can reference Python in source_skills.
        conn.execute(
            "INSERT INTO sources (id, source_type, title, description) VALUES (?1, 'general', 'Test source', 'Test desc')",
            rusqlite::params!["bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbb01"],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO source_skills (source_id, skill_id) VALUES (?1, ?2)",
            rusqlite::params![
                "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbb01",
                "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaa01",
            ],
        )
        .unwrap();

        apply_053(&conn);

        // Forward: source_skills → skills still works.
        let legacy_name: String = conn
            .query_row(
                "SELECT s.name FROM source_skills ss JOIN skills s ON s.id = ss.skill_id \
                 WHERE ss.source_id = ?1",
                rusqlite::params!["bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbb01"],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(legacy_name, "Python");

        // New: source_skills.skill_id JOINs the new graph node directly because
        // the migrated node reuses the legacy UUID.
        let graph_name: String = conn
            .query_row(
                "SELECT n.canonical_name FROM source_skills ss \
                 JOIN skill_graph_nodes n ON n.id = ss.skill_id \
                 WHERE ss.source_id = ?1",
                rusqlite::params!["bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbb01"],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(graph_name, "Python");
    }

    #[test]
    fn population_runs_cleanly_on_fresh_full_migration_chain() {
        // The standard run_migrations path on a brand-new DB must apply 053
        // without error even though `skills` is empty (no rows to mirror).
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON").unwrap();
        run_migrations(&conn).unwrap();

        // Category roots are still created (skill_categories is seeded by 044).
        let root_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM skill_graph_nodes WHERE description LIKE 'category-root:%'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(
            root_count > 0,
            "category roots should be created even on fresh DB"
        );

        // No mirror nodes exist because there are no legacy skills.
        let mirror_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM skill_graph_nodes WHERE legacy_skill_id IS NOT NULL",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(mirror_count, 0);
    }

    #[test]
    fn skill_graph_traversal_indexes_present() {
        let conn = fresh_db();

        let mut names: Vec<String> = conn
            .prepare(
                "SELECT name FROM sqlite_master WHERE type='index' AND tbl_name='skill_graph_edges' AND name LIKE 'idx_%'",
            )
            .unwrap()
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        names.sort();

        assert_eq!(
            names,
            vec![
                "idx_skill_graph_edges_source".to_string(),
                "idx_skill_graph_edges_target".to_string(),
                "idx_skill_graph_edges_type".to_string(),
            ]
        );
    }

    // ------------------------------------------------------------------
    // 054_alignment_results
    // ------------------------------------------------------------------

    #[test]
    fn migration_054_is_registered_with_alignment_results_ddl() {
        let entry = MIGRATIONS
            .iter()
            .find(|(name, _)| name.starts_with("054_"))
            .expect("expected migration 054 to be registered");
        let (name, sql) = entry;
        assert_eq!(*name, "054_alignment_results");
        assert!(
            sql.contains("CREATE TABLE alignment_results"),
            "DDL must create alignment_results"
        );
        assert!(sql.contains("STRICT"), "table must be STRICT");
        assert!(
            sql.contains("idx_alignment_results_resume_jd"),
            "index must exist"
        );
    }

    #[test]
    fn migration_count_is_52() {
        assert_eq!(
            MIGRATIONS.len(),
            52,
            "expected 52 migrations after adding 054_alignment_results"
        );
    }
}
