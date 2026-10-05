//! SQL-backed implementation of the `SkillGraphTraversal` trait (forge-8xjh).
//!
//! Targets SQLite (rusqlite) — the same DDL also runs in wa-sqlite (browser)
//! and Cloudflare D1. Recursive CTEs are used for `n_hop_neighbors`; the JSON1
//! extension (built into SQLite) is used for alias and temporal-window queries.
//!
//! The store holds a borrowed connection so the trait can be `&self` and
//! object-safe. Construct one via `SqlSkillGraphStore::new(&conn)`.

use std::collections::{BTreeSet, HashSet};
use std::str::FromStr;

use rusqlite::{params, Connection, OptionalExtension, Row};

use forge_core::{
    now_iso,
    skill_graph::{
        CoOccurrenceStat, EdgeRow, EdgeType, NeighborhoodSubgraph, NodeSource, RelatedSkill,
        SkillGraphSnapshot, SkillGraphTraversal, SkillNode, SnapshotEdge, SnapshotHeader,
        SnapshotMetadata, SnapshotNode,
    },
    ForgeError,
};

const NODE_COLUMNS: &str = "\
    id, canonical_name, category, description, aliases, level_descriptors, \
    embedding, embedding_model_version, confidence, source, legacy_skill_id, \
    created_at, updated_at\
";

/// `NODE_COLUMNS` qualified by a table alias (e.g. `"n."`) for use inside
/// JOINs where unqualified `confidence` / `id` would be ambiguous.
fn node_columns(prefix: &str) -> String {
    format!(
        "{p}id, {p}canonical_name, {p}category, {p}description, {p}aliases, \
         {p}level_descriptors, {p}embedding, {p}embedding_model_version, \
         {p}confidence, {p}source, {p}legacy_skill_id, {p}created_at, {p}updated_at",
        p = prefix
    )
}

/// SQL-backed traversal over `skill_graph_nodes` / `skill_graph_edges`.
pub struct SqlSkillGraphStore<'c> {
    conn: &'c Connection,
}

impl<'c> SqlSkillGraphStore<'c> {
    pub fn new(conn: &'c Connection) -> Self {
        Self { conn }
    }

    // ── row mapping ─────────────────────────────────────────────────────

    fn map_node(row: &Row<'_>) -> rusqlite::Result<SkillNode> {
        let aliases_json: String = row.get("aliases")?;
        let aliases: Vec<String> = serde_json::from_str(&aliases_json).unwrap_or_default();

        let source_str: String = row.get("source")?;
        let source = NodeSource::from_str(&source_str).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(
                9, // arbitrary column index for the error
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("invalid NodeSource '{source_str}': {e}"),
                )),
            )
        })?;

        Ok(SkillNode {
            id: row.get("id")?,
            canonical_name: row.get("canonical_name")?,
            category: row.get("category")?,
            description: row.get("description")?,
            aliases,
            level_descriptors: row.get("level_descriptors")?,
            embedding: row.get("embedding")?,
            embedding_model_version: row.get("embedding_model_version")?,
            confidence: row.get("confidence")?,
            source,
            legacy_skill_id: row.get("legacy_skill_id")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    fn map_edge(row: &Row<'_>) -> rusqlite::Result<EdgeRow> {
        let edge_type_str: String = row.get("edge_type")?;
        let edge_type = EdgeType::from_str(&edge_type_str).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(
                2,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("invalid EdgeType '{edge_type_str}': {e}"),
                )),
            )
        })?;
        Ok(EdgeRow {
            source_id: row.get("source_id")?,
            target_id: row.get("target_id")?,
            edge_type,
            weight: row.get("weight")?,
            confidence: row.get("confidence")?,
            temporal_data: row.get("temporal_data")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    /// Fetch nodes by a set of ids.
    fn nodes_by_ids(&self, ids: &BTreeSet<String>) -> Result<Vec<SkillNode>, ForgeError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        // SQLite has no IN-list parameter binding, so use json_each over a JSON
        // array. This is the same trick used elsewhere in this module.
        let json_array = serde_json::to_string(&ids.iter().collect::<Vec<_>>())
            .unwrap_or_else(|_| "[]".to_string());
        let sql = format!(
            "SELECT {NODE_COLUMNS} FROM skill_graph_nodes \
             WHERE id IN (SELECT value FROM json_each(?1))"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt
            .query_map(params![json_array], Self::map_node)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}

// ── trait impl ──────────────────────────────────────────────────────────

impl<'c> SkillGraphTraversal for SqlSkillGraphStore<'c> {
    fn find_aliases(&self, skill_id: &str) -> Result<Vec<SkillNode>, ForgeError> {
        // alias-of is bidirectional in semantics: walk both source→target
        // (this skill aliases another) and target→source (another skill
        // aliases this one).
        let sql = format!(
            "SELECT {NODE_COLUMNS} FROM skill_graph_nodes WHERE id IN ( \
                SELECT target_id FROM skill_graph_edges \
                WHERE source_id = ?1 AND edge_type = 'alias-of' \
              UNION \
                SELECT source_id FROM skill_graph_edges \
                WHERE target_id = ?1 AND edge_type = 'alias-of' \
             )"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt
            .query_map(params![skill_id], Self::map_node)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    fn find_children(&self, skill_id: &str) -> Result<Vec<SkillNode>, ForgeError> {
        // Children = (skill_id) parent-of Y  OR  Y child-of (skill_id).
        let sql = format!(
            "SELECT {NODE_COLUMNS} FROM skill_graph_nodes WHERE id IN ( \
                SELECT target_id FROM skill_graph_edges \
                WHERE source_id = ?1 AND edge_type = 'parent-of' \
              UNION \
                SELECT source_id FROM skill_graph_edges \
                WHERE target_id = ?1 AND edge_type = 'child-of' \
             )"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt
            .query_map(params![skill_id], Self::map_node)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    fn find_parents(&self, skill_id: &str) -> Result<Vec<SkillNode>, ForgeError> {
        // Parents = Y parent-of (skill_id)  OR  (skill_id) child-of Y.
        let sql = format!(
            "SELECT {NODE_COLUMNS} FROM skill_graph_nodes WHERE id IN ( \
                SELECT source_id FROM skill_graph_edges \
                WHERE target_id = ?1 AND edge_type = 'parent-of' \
              UNION \
                SELECT target_id FROM skill_graph_edges \
                WHERE source_id = ?1 AND edge_type = 'child-of' \
             )"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt
            .query_map(params![skill_id], Self::map_node)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    fn find_related(
        &self,
        skill_id: &str,
        edge_types: &[EdgeType],
    ) -> Result<Vec<RelatedSkill>, ForgeError> {
        if edge_types.is_empty() {
            return Ok(Vec::new());
        }
        let edge_types_json =
            serde_json::to_string(&edge_types.iter().map(|e| e.as_ref()).collect::<Vec<_>>())
                .unwrap_or_else(|_| "[]".to_string());

        let cols = node_columns("n.");
        let sql = format!(
            "SELECT {cols}, e.edge_type AS et, e.weight AS w, e.confidence AS c \
             FROM skill_graph_edges e \
             JOIN skill_graph_nodes n ON n.id = e.target_id \
             WHERE e.source_id = ?1 \
               AND e.edge_type IN (SELECT value FROM json_each(?2))"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt
            .query_map(params![skill_id, edge_types_json], |row| {
                let skill = Self::map_node(row)?;
                let et: String = row.get("et")?;
                let edge_type = EdgeType::from_str(&et).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("invalid EdgeType '{et}': {e}"),
                        )),
                    )
                })?;
                Ok(RelatedSkill {
                    skill,
                    edge_type,
                    weight: row.get("w")?,
                    confidence: row.get("c")?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    fn n_hop_neighbors(
        &self,
        skill_id: &str,
        n: usize,
        edge_types: Option<&[EdgeType]>,
    ) -> Result<NeighborhoodSubgraph, ForgeError> {
        // The recursive CTE explores both directions of every edge. Filtering
        // by edge type uses the same json_each trick. n=0 returns just the
        // center node; the recursive arm doesn't fire because depth < 0 fails.
        let max_depth = n as i64;
        let edge_filter_json = edge_types
            .map(|ets| {
                serde_json::to_string(&ets.iter().map(|e| e.as_ref()).collect::<Vec<_>>())
                    .unwrap_or_else(|_| "[]".to_string())
            })
            .unwrap_or_else(|| "[]".to_string());
        let edge_filter_active = edge_types.map(|t| !t.is_empty()).unwrap_or(false);

        // SQLite parameter typing for booleans: bind as i64.
        let filter_active = if edge_filter_active { 1_i64 } else { 0_i64 };

        let sql = "\
            WITH RECURSIVE neighborhood(id, depth) AS ( \
                SELECT id, 0 FROM skill_graph_nodes WHERE id = ?1 \
              UNION \
                SELECT \
                    CASE WHEN e.source_id = nb.id THEN e.target_id ELSE e.source_id END, \
                    nb.depth + 1 \
                FROM neighborhood nb \
                JOIN skill_graph_edges e \
                  ON e.source_id = nb.id OR e.target_id = nb.id \
                WHERE nb.depth < ?2 \
                  AND ( \
                    ?3 = 0 \
                    OR e.edge_type IN (SELECT value FROM json_each(?4)) \
                  ) \
            ) \
            SELECT DISTINCT id FROM neighborhood\
        ";

        let mut stmt = self.conn.prepare(sql)?;
        let ids: BTreeSet<String> = stmt
            .query_map(
                params![skill_id, max_depth, filter_active, edge_filter_json],
                |row| row.get::<_, String>(0),
            )?
            .collect::<Result<_, _>>()?;

        // Materialize nodes.
        let nodes = self.nodes_by_ids(&ids)?;

        // Materialize edges among the neighborhood. Same json_each trick.
        let ids_json = serde_json::to_string(&ids.iter().collect::<Vec<_>>())
            .unwrap_or_else(|_| "[]".to_string());
        let edge_sql = "\
            SELECT source_id, target_id, edge_type, weight, confidence, \
                   temporal_data, created_at, updated_at \
            FROM skill_graph_edges \
            WHERE source_id IN (SELECT value FROM json_each(?1)) \
              AND target_id IN (SELECT value FROM json_each(?1)) \
              AND ( \
                ?2 = 0 \
                OR edge_type IN (SELECT value FROM json_each(?3)) \
              )\
        ";
        let mut edge_stmt = self.conn.prepare(edge_sql)?;
        let edges: Vec<EdgeRow> = edge_stmt
            .query_map(
                params![ids_json, filter_active, edge_filter_json],
                Self::map_edge,
            )?
            .collect::<Result<_, _>>()?;

        Ok(NeighborhoodSubgraph {
            center_id: skill_id.to_string(),
            nodes,
            edges,
        })
    }

    fn find_by_name(&self, name: &str) -> Result<Option<SkillNode>, ForgeError> {
        // Exact canonical match first.
        let sql = format!(
            "SELECT {NODE_COLUMNS} FROM skill_graph_nodes \
             WHERE canonical_name = ?1 \
             LIMIT 1"
        );
        if let Some(node) = self
            .conn
            .query_row(&sql, params![name], Self::map_node)
            .optional()?
        {
            return Ok(Some(node));
        }

        // Fallback: scan aliases JSON arrays for an exact entry match.
        let sql_alias = format!(
            "SELECT {NODE_COLUMNS} FROM skill_graph_nodes \
             WHERE EXISTS ( \
                SELECT 1 FROM json_each(skill_graph_nodes.aliases) WHERE value = ?1 \
             ) \
             LIMIT 1"
        );
        let result = self
            .conn
            .query_row(&sql_alias, params![name], Self::map_node)
            .optional()?;
        Ok(result)
    }

    fn co_occurrence_stats(
        &self,
        skill_id: &str,
        time_window: Option<&str>,
    ) -> Result<Vec<CoOccurrenceStat>, ForgeError> {
        match time_window {
            None => {
                // Headline weight from the edge column itself; no window data
                // parsed. jd_count is unknown in this mode.
                let cols = node_columns("n.");
                let sql = format!(
                    "SELECT {cols}, e.weight AS w \
                     FROM skill_graph_edges e \
                     JOIN skill_graph_nodes n ON n.id = e.target_id \
                     WHERE e.source_id = ?1 AND e.edge_type = 'co-occurs'"
                );
                let mut stmt = self.conn.prepare(&sql)?;
                let rows = stmt
                    .query_map(params![skill_id], |row| {
                        let skill = Self::map_node(row)?;
                        Ok(CoOccurrenceStat {
                            skill,
                            weight: row.get("w")?,
                            jd_count: None,
                            window: None,
                        })
                    })?
                    .collect::<Result<_, _>>()?;
                Ok(rows)
            }
            Some(window) => {
                // For each co-occurs edge, look inside temporal_data for the
                // matching window entry and pull weight + jd_count from it.
                // Edges with no entry for the window are filtered out.
                let cols = node_columns("n.");
                let sql = format!(
                    "SELECT {cols}, \
                            json_extract(je.value, '$.weight')   AS w, \
                            json_extract(je.value, '$.jd_count') AS jd \
                     FROM skill_graph_edges e \
                     JOIN skill_graph_nodes n ON n.id = e.target_id \
                     JOIN json_each(e.temporal_data) je \
                     WHERE e.source_id = ?1 \
                       AND e.edge_type = 'co-occurs' \
                       AND json_extract(je.value, '$.window') = ?2"
                );
                let mut stmt = self.conn.prepare(&sql)?;
                let rows = stmt
                    .query_map(params![skill_id, window], |row| {
                        let skill = Self::map_node(row)?;
                        Ok(CoOccurrenceStat {
                            skill,
                            weight: row.get("w")?,
                            jd_count: row.get("jd").ok(),
                            window: Some(window.to_string()),
                        })
                    })?
                    .collect::<Result<_, _>>()?;
                Ok(rows)
            }
        }
    }
}

// Suppress unused-imports for the case where downstream code only uses the
// trait through the public re-export.
#[allow(dead_code)]
fn _assert_traversal_object_safe() {
    fn assert_obj(_: &dyn SkillGraphTraversal) {}
    let _ = assert_obj;
    let _: HashSet<()> = HashSet::new();
}

// ────────────────────────────────────────────────────────────────────────
// Snapshot builder (forge-ubxb)
// ────────────────────────────────────────────────────────────────────────

/// Build a structural-only `SkillGraphSnapshot` from a SQLite connection.
///
/// "Structural-only" means nodes + edges + (optional) market_stats, but no
/// embedding payload and no HNSW index — those are populated by the WASM
/// runtime (forge-afyg) once it lands. Snapshots produced by this function
/// are valid as-is and round-trip through `SkillGraphSnapshot::encode/decode`;
/// downstream tooling can layer in the binary sections later.
pub fn build_structural_snapshot(
    conn: &Connection,
    snapshot_id: impl Into<String>,
) -> Result<SkillGraphSnapshot, ForgeError> {
    // Nodes — pull every row in canonical_name order so snapshots are
    // reproducible for a given DB state (helpful for caching / ETag).
    let mut node_stmt = conn.prepare(
        "SELECT id, canonical_name, category, aliases, source, confidence \
         FROM skill_graph_nodes \
         ORDER BY canonical_name, id",
    )?;
    let nodes: Vec<SnapshotNode> = node_stmt
        .query_map([], |row| {
            let aliases_json: String = row.get("aliases")?;
            let aliases: Vec<String> = serde_json::from_str(&aliases_json).unwrap_or_default();
            let source_str: String = row.get("source")?;
            let source = NodeSource::from_str(&source_str).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    4,
                    rusqlite::types::Type::Text,
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("invalid NodeSource '{source_str}': {e}"),
                    )),
                )
            })?;
            Ok(SnapshotNode {
                id: row.get("id")?,
                canonical_name: row.get("canonical_name")?,
                category: row.get("category")?,
                aliases,
                source,
                confidence: row.get("confidence")?,
            })
        })?
        .collect::<Result<_, _>>()?;

    // Edges — sorted by (source_id, target_id, edge_type) for reproducibility.
    let mut edge_stmt = conn.prepare(
        "SELECT source_id, target_id, edge_type, weight, confidence, temporal_data \
         FROM skill_graph_edges \
         ORDER BY source_id, target_id, edge_type",
    )?;
    let edges: Vec<SnapshotEdge> = edge_stmt
        .query_map([], |row| {
            let edge_type_str: String = row.get("edge_type")?;
            let edge_type = EdgeType::from_str(&edge_type_str).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    2,
                    rusqlite::types::Type::Text,
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("invalid EdgeType '{edge_type_str}': {e}"),
                    )),
                )
            })?;
            Ok(SnapshotEdge {
                source_id: row.get("source_id")?,
                target_id: row.get("target_id")?,
                edge_type,
                weight: row.get("weight")?,
                confidence: row.get("confidence")?,
                temporal_data: row.get("temporal_data")?,
            })
        })?
        .collect::<Result<_, _>>()?;

    let metadata = SnapshotMetadata {
        snapshot_id: snapshot_id.into(),
        created_at: now_iso(),
        skill_count: nodes.len() as u32,
        edge_count: edges.len() as u32,
        embedding_model: None,
        embedding_dim: None,
    };
    let header = SnapshotHeader {
        metadata,
        nodes,
        edges,
        market_stats: Vec::new(),
        embedding_byte_length: 0,
        hnsw_index_byte_length: 0,
    };

    Ok(SkillGraphSnapshot {
        header,
        embeddings: Vec::new(),
        hnsw_index: Vec::new(),
    })
}

// ────────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrate::run_migrations;

    /// Build a DB with the full migration chain applied (including 053
    /// initial population), plus three custom skills useful for the tests.
    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON").unwrap();

        // Insert legacy skills BEFORE migrations run, since 053 mirrors them.
        // Migration 044 has already created skill_categories at this point —
        // wait, no. Migrations run in order; we can't insert before they all
        // run. Insert AFTER and re-run only 053 to mirror them, OR insert
        // skills directly into both tables manually.
        //
        // Simpler: run all migrations on an empty DB, then build graph
        // structure manually for the test.
        run_migrations(&conn).unwrap();

        // Helper data:
        //   k8s          — Kubernetes (canonical), aliases ["K8s", "kube"]
        //   docker       — Docker
        //   helm         — Helm (co-occurs with kubernetes)
        //   container    — Container Orchestration (parent of kubernetes)
        let nodes = [
            ("k8s", "Kubernetes", "platform", "[\"K8s\",\"kube\"]"),
            ("docker", "Docker", "tool", "[]"),
            ("helm", "Helm", "tool", "[]"),
            ("container", "Container Orchestration", "concept", "[]"),
        ];
        for (id, name, cat, aliases_json) in nodes {
            let uuid = format!("{:0>36}", id);
            conn.execute(
                "INSERT INTO skill_graph_nodes (id, canonical_name, category, aliases, source) \
                 VALUES (?1, ?2, ?3, ?4, 'curated')",
                params![uuid, name, cat, aliases_json],
            )
            .unwrap();
        }

        // Edges:
        //   container parent-of k8s
        //   docker prerequisite k8s
        //   k8s related-to docker
        //   k8s co-occurs helm   (with temporal data)
        let edges = [
            ("container", "k8s", "parent-of", 1.0, None),
            ("docker", "k8s", "prerequisite", 1.0, None),
            ("k8s", "docker", "related-to", 0.4, None),
            (
                "k8s",
                "helm",
                "co-occurs",
                0.85,
                Some(
                    r#"[{"window":"2026-Q1","weight":0.85,"jd_count":140},{"window":"2025-Q4","weight":0.80,"jd_count":120}]"#,
                ),
            ),
        ];
        for (src, tgt, et, w, td) in edges {
            let src_uuid = format!("{:0>36}", src);
            let tgt_uuid = format!("{:0>36}", tgt);
            conn.execute(
                "INSERT INTO skill_graph_edges \
                   (source_id, target_id, edge_type, weight, confidence, temporal_data) \
                 VALUES (?1, ?2, ?3, ?4, 1.0, ?5)",
                params![src_uuid, tgt_uuid, et, w, td],
            )
            .unwrap();
        }

        // Add an explicit alias edge to verify find_aliases.
        // Insert one extra alias node so the alias-of edge has somewhere to point.
        conn.execute(
            "INSERT INTO skill_graph_nodes (id, canonical_name, category, aliases, source) \
             VALUES (?1, 'k8s (legacy)', 'other', '[]', 'curated')",
            params![format!("{:0>36}", "k8sLegacy")],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO skill_graph_edges (source_id, target_id, edge_type, weight, confidence) \
             VALUES (?1, ?2, 'alias-of', 1.0, 1.0)",
            params![format!("{:0>36}", "k8sLegacy"), format!("{:0>36}", "k8s")],
        )
        .unwrap();

        conn
    }

    fn id_of(handle: &str) -> String {
        format!("{:0>36}", handle)
    }

    #[test]
    fn find_aliases_walks_both_directions() {
        let conn = db();
        let store = SqlSkillGraphStore::new(&conn);

        let aliases = store.find_aliases(&id_of("k8s")).unwrap();
        assert_eq!(aliases.len(), 1);
        assert_eq!(aliases[0].canonical_name, "k8s (legacy)");

        // Same query from the other end should also resolve.
        let inverse = store.find_aliases(&id_of("k8sLegacy")).unwrap();
        assert_eq!(inverse.len(), 1);
        assert_eq!(inverse[0].canonical_name, "Kubernetes");
    }

    #[test]
    fn find_children_and_parents_use_parent_of_edge() {
        let conn = db();
        let store = SqlSkillGraphStore::new(&conn);

        let children = store.find_children(&id_of("container")).unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].canonical_name, "Kubernetes");

        let parents = store.find_parents(&id_of("k8s")).unwrap();
        assert_eq!(parents.len(), 1);
        assert_eq!(parents[0].canonical_name, "Container Orchestration");
    }

    #[test]
    fn find_related_filters_by_edge_type_and_returns_weights() {
        let conn = db();
        let store = SqlSkillGraphStore::new(&conn);

        let related = store
            .find_related(&id_of("k8s"), &[EdgeType::RelatedTo])
            .unwrap();
        assert_eq!(related.len(), 1);
        assert_eq!(related[0].skill.canonical_name, "Docker");
        assert_eq!(related[0].edge_type, EdgeType::RelatedTo);
        assert!((related[0].weight - 0.4).abs() < 1e-9);

        // Mixing edge types should pick up multiple.
        let mixed = store
            .find_related(&id_of("k8s"), &[EdgeType::RelatedTo, EdgeType::CoOccurs])
            .unwrap();
        assert_eq!(mixed.len(), 2);

        // Empty filter returns nothing.
        let empty = store.find_related(&id_of("k8s"), &[]).unwrap();
        assert!(empty.is_empty());
    }

    #[test]
    fn find_by_name_falls_back_to_alias_lookup() {
        let conn = db();
        let store = SqlSkillGraphStore::new(&conn);

        let by_canonical = store.find_by_name("Kubernetes").unwrap();
        assert!(by_canonical.is_some());
        assert_eq!(by_canonical.unwrap().id, id_of("k8s"));

        let by_alias = store.find_by_name("K8s").unwrap();
        assert!(by_alias.is_some());
        assert_eq!(by_alias.unwrap().canonical_name, "Kubernetes");

        let by_other_alias = store.find_by_name("kube").unwrap();
        assert!(by_other_alias.is_some());

        let missing = store.find_by_name("Cassandra").unwrap();
        assert!(missing.is_none());
    }

    #[test]
    fn co_occurrence_stats_no_window_returns_headline_weight() {
        let conn = db();
        let store = SqlSkillGraphStore::new(&conn);

        let stats = store.co_occurrence_stats(&id_of("k8s"), None).unwrap();
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].skill.canonical_name, "Helm");
        assert!((stats[0].weight - 0.85).abs() < 1e-9);
        assert!(stats[0].jd_count.is_none());
        assert!(stats[0].window.is_none());
    }

    #[test]
    fn co_occurrence_stats_with_window_pulls_from_temporal_data() {
        let conn = db();
        let store = SqlSkillGraphStore::new(&conn);

        let q1 = store
            .co_occurrence_stats(&id_of("k8s"), Some("2026-Q1"))
            .unwrap();
        assert_eq!(q1.len(), 1);
        assert_eq!(q1[0].window.as_deref(), Some("2026-Q1"));
        assert!((q1[0].weight - 0.85).abs() < 1e-9);
        assert_eq!(q1[0].jd_count, Some(140));

        let q4 = store
            .co_occurrence_stats(&id_of("k8s"), Some("2025-Q4"))
            .unwrap();
        assert_eq!(q4.len(), 1);
        assert!((q4[0].weight - 0.80).abs() < 1e-9);
        assert_eq!(q4[0].jd_count, Some(120));

        // A window with no data yields no rows.
        let unknown = store
            .co_occurrence_stats(&id_of("k8s"), Some("2024-Q1"))
            .unwrap();
        assert!(unknown.is_empty());
    }

    #[test]
    fn n_hop_neighbors_zero_returns_only_center() {
        let conn = db();
        let store = SqlSkillGraphStore::new(&conn);

        let sub = store.n_hop_neighbors(&id_of("k8s"), 0, None).unwrap();
        assert_eq!(sub.center_id, id_of("k8s"));
        assert_eq!(sub.nodes.len(), 1);
        assert_eq!(sub.nodes[0].id, id_of("k8s"));
    }

    #[test]
    fn n_hop_neighbors_one_returns_direct_neighbors_in_both_directions() {
        let conn = db();
        let store = SqlSkillGraphStore::new(&conn);

        let sub = store.n_hop_neighbors(&id_of("k8s"), 1, None).unwrap();
        let names: BTreeSet<String> = sub.nodes.iter().map(|n| n.canonical_name.clone()).collect();

        // 1-hop from k8s should include: k8s, container (incoming parent-of),
        // docker (outgoing related-to + incoming prerequisite), helm
        // (outgoing co-occurs), k8s (legacy) (incoming alias-of).
        assert!(names.contains("Kubernetes"));
        assert!(names.contains("Container Orchestration"));
        assert!(names.contains("Docker"));
        assert!(names.contains("Helm"));
        assert!(names.contains("k8s (legacy)"));
    }

    #[test]
    fn n_hop_neighbors_with_edge_type_filter() {
        let conn = db();
        let store = SqlSkillGraphStore::new(&conn);

        // Restrict to parent-of only — should reach Container Orchestration but
        // not Docker / Helm / Legacy.
        let sub = store
            .n_hop_neighbors(&id_of("k8s"), 2, Some(&[EdgeType::ParentOf]))
            .unwrap();
        let names: BTreeSet<String> = sub.nodes.iter().map(|n| n.canonical_name.clone()).collect();

        assert!(names.contains("Kubernetes"));
        assert!(names.contains("Container Orchestration"));
        assert!(!names.contains("Docker"));
        assert!(!names.contains("Helm"));
        assert!(!names.contains("k8s (legacy)"));
    }

    #[test]
    fn n_hop_neighbors_two_hops_reaches_transitive() {
        let conn = db();
        let store = SqlSkillGraphStore::new(&conn);

        // From container, 1 hop reaches k8s. 2 hops should also reach
        // docker, helm, legacy via k8s.
        let one = store.n_hop_neighbors(&id_of("container"), 1, None).unwrap();
        let one_names: BTreeSet<String> =
            one.nodes.iter().map(|n| n.canonical_name.clone()).collect();
        assert!(one_names.contains("Kubernetes"));
        assert!(!one_names.contains("Docker"));

        let two = store.n_hop_neighbors(&id_of("container"), 2, None).unwrap();
        let two_names: BTreeSet<String> =
            two.nodes.iter().map(|n| n.canonical_name.clone()).collect();
        assert!(two_names.contains("Docker"));
        assert!(two_names.contains("Helm"));
        assert!(two_names.contains("k8s (legacy)"));
    }

    // ── Snapshot builder ────────────────────────────────────────────────

    #[test]
    fn build_structural_snapshot_pulls_nodes_and_edges_in_deterministic_order() {
        let conn = db();

        let snap = build_structural_snapshot(&conn, "test-snap").unwrap();
        assert_eq!(snap.header.metadata.snapshot_id, "test-snap");

        // db() inserts 5 test nodes (k8s, docker, helm, container, k8sLegacy)
        // ON TOP of the 14 category-root nodes that migration 053 creates from
        // the seeded skill_categories table → 19 nodes total. Verify both:
        // the structural count includes everything in the DB, and our test
        // nodes are present.
        assert_eq!(
            snap.header.metadata.skill_count,
            snap.header.nodes.len() as u32
        );
        assert_eq!(snap.header.metadata.skill_count, 19);

        let test_node_names: BTreeSet<&str> = [
            "Container Orchestration",
            "Docker",
            "Helm",
            "Kubernetes",
            "k8s (legacy)",
        ]
        .into_iter()
        .collect();
        let snapshot_names: BTreeSet<&str> = snap
            .header
            .nodes
            .iter()
            .map(|n| n.canonical_name.as_str())
            .collect();
        assert!(
            test_node_names.is_subset(&snapshot_names),
            "snapshot must contain every test node"
        );

        // Nodes are sorted by canonical_name ASC, then id.
        let mut prev_name: Option<&str> = None;
        for n in &snap.header.nodes {
            if let Some(p) = prev_name {
                assert!(
                    p <= n.canonical_name.as_str(),
                    "nodes not sorted: {p} > {}",
                    n.canonical_name
                );
            }
            prev_name = Some(n.canonical_name.as_str());
        }

        // 5 test edges (container→k8s, docker→k8s, k8s→docker, k8s→helm,
        // k8sLegacy→k8s). 053 emits zero edges on a DB with no legacy skills.
        assert_eq!(snap.header.metadata.edge_count, 5);

        // Edges are sorted by (source_id, target_id, edge_type).
        let mut prev: Option<(String, String, String)> = None;
        for e in &snap.header.edges {
            let key = (
                e.source_id.clone(),
                e.target_id.clone(),
                e.edge_type.as_ref().to_string(),
            );
            if let Some(p) = prev {
                assert!(p < key, "edges not sorted: {p:?} should be before {key:?}");
            }
            prev = Some(key);
        }

        // Aliases parsed back into Vec<String>.
        let kubernetes = snap
            .header
            .nodes
            .iter()
            .find(|n| n.canonical_name == "Kubernetes")
            .unwrap();
        assert_eq!(kubernetes.aliases, vec!["K8s", "kube"]);

        // Embedding sections empty in MVP.
        assert_eq!(snap.embeddings.len(), 0);
        assert_eq!(snap.hnsw_index.len(), 0);
        assert_eq!(snap.header.embedding_byte_length, 0);
        assert_eq!(snap.header.hnsw_index_byte_length, 0);
    }

    #[test]
    fn build_structural_snapshot_round_trips_through_encode_decode() {
        let conn = db();
        let snap = build_structural_snapshot(&conn, "snap-rt").unwrap();
        let bytes = snap.encode().unwrap();
        let decoded = SkillGraphSnapshot::decode(&bytes).unwrap();
        assert_eq!(decoded, snap);
    }

    #[test]
    fn n_hop_neighbors_includes_internal_edges() {
        let conn = db();
        let store = SqlSkillGraphStore::new(&conn);

        let sub = store.n_hop_neighbors(&id_of("k8s"), 1, None).unwrap();
        // The materialized edges should include each edge whose endpoints are
        // both inside the neighborhood. With a 1-hop expansion from k8s, every
        // edge that touches k8s qualifies (the other endpoint is included).
        let edge_count = sub.edges.len();
        assert!(
            edge_count >= 4,
            "expected at least 4 edges, got {edge_count}"
        );
    }
}
