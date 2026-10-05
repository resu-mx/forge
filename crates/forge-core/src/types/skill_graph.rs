//! Skill graph types and traversal contract (forge-8xjh).
//!
//! Backbone of the skill intelligence system. Defines the row shapes for
//! `skill_graph_nodes` / `skill_graph_edges` (created in migration 052), the
//! typed edge vocabulary (`EdgeType`), and the `SkillGraphTraversal` trait
//! that both the SQL backend (forge-sdk) and the in-memory petgraph backend
//! (forge-wasm, future) implement.
//!
//! Architecture: this module contains no I/O — only types and the trait.
//! Implementations live in their respective backend crates.

use serde::{Deserialize, Serialize};

use crate::ForgeError;

// ────────────────────────────────────────────────────────────────────────
// Edge vocabulary
// ────────────────────────────────────────────────────────────────────────

/// Typed edge vocabulary. Stored on `skill_graph_edges.edge_type` and gated
/// by a SQL `CHECK` constraint. The string form (kebab-case) is the
/// authoritative SQL representation.
///
/// Numeric semantics for alignment scoring live in the alignment engine
/// (Epic forge-etam), not here. See `docs/src/architecture/graphs/skills.md`.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    strum::Display,
    strum::EnumString,
    strum::AsRefStr,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum EdgeType {
    AliasOf,
    ParentOf,
    ChildOf,
    Prerequisite,
    RelatedTo,
    CoOccurs,
    PlatformFor,
}

impl EdgeType {
    /// Every edge type, in declaration order. Useful for "all edge types"
    /// queries and round-trip tests.
    pub const ALL: [EdgeType; 7] = [
        EdgeType::AliasOf,
        EdgeType::ParentOf,
        EdgeType::ChildOf,
        EdgeType::Prerequisite,
        EdgeType::RelatedTo,
        EdgeType::CoOccurs,
        EdgeType::PlatformFor,
    ];
}

/// Provenance for a skill graph node. Stored on `skill_graph_nodes.source`.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    strum::Display,
    strum::EnumString,
    strum::AsRefStr,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum NodeSource {
    Seed,
    Extracted,
    Curated,
    UserCreated,
}

// ────────────────────────────────────────────────────────────────────────
// Row shapes
// ────────────────────────────────────────────────────────────────────────

/// A row from `skill_graph_nodes`, with `aliases` parsed out of its JSON
/// column for caller convenience.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillNode {
    pub id: String,
    pub canonical_name: String,
    /// FK to `skill_categories.slug`.
    pub category: String,
    pub description: Option<String>,
    /// Variant strings (parsed from the JSON `aliases` column).
    pub aliases: Vec<String>,
    /// Raw JSON object (caller decides how to parse).
    pub level_descriptors: Option<String>,
    /// Packed Float32Array. `None` until an extraction pass produces one.
    pub embedding: Option<Vec<u8>>,
    pub embedding_model_version: Option<String>,
    pub confidence: f64,
    pub source: NodeSource,
    pub legacy_skill_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// A row from `skill_graph_edges`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeRow {
    pub source_id: String,
    pub target_id: String,
    pub edge_type: EdgeType,
    pub weight: f64,
    pub confidence: f64,
    /// Raw JSON array of windowed measurements (only set for `co-occurs`).
    pub temporal_data: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// A skill returned by `find_related`, paired with the edge that produced it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedSkill {
    pub skill: SkillNode,
    pub edge_type: EdgeType,
    pub weight: f64,
    pub confidence: f64,
}

/// The N-hop neighborhood around a skill: all nodes within range plus the
/// edges connecting them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeighborhoodSubgraph {
    pub center_id: String,
    pub nodes: Vec<SkillNode>,
    pub edges: Vec<EdgeRow>,
}

/// One entry in a co-occurrence stats response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoOccurrenceStat {
    pub skill: SkillNode,
    pub weight: f64,
    /// Number of JDs supporting this co-occurrence in the window. `None` when
    /// the row is taken from the edge's headline `weight` column rather than
    /// from a windowed entry.
    pub jd_count: Option<i64>,
    /// The window label that produced this row, e.g. `"2026-Q1"`. `None` when
    /// the caller passed `time_window = None` and the row uses the edge's
    /// headline weight.
    pub window: Option<String>,
}

// ────────────────────────────────────────────────────────────────────────
// Traversal contract
// ────────────────────────────────────────────────────────────────────────

/// Read-only typed traversal over a skill graph. Implementations can be
/// SQL-backed (forge-sdk `SqlSkillGraphStore`) or in-memory petgraph-backed
/// (forge-wasm, future).
///
/// Direction conventions:
/// - `find_aliases` is **bidirectional**: an alias-of edge is treated the same
///   regardless of which side `skill_id` sits on.
/// - `find_children` follows `parent-of` outbound from `skill_id` AND
///   `child-of` inbound to it. Only one direction needs to be materialized
///   per concept; the trait abstracts over the storage choice.
/// - `find_parents` is the inverse of `find_children`.
/// - `find_related` returns OUTGOING edges only — it's the precise primitive
///   for callers who care about edge direction.
/// - `n_hop_neighbors` is undirected: both incoming and outgoing edges are
///   followed.
pub trait SkillGraphTraversal {
    fn find_aliases(&self, skill_id: &str) -> Result<Vec<SkillNode>, ForgeError>;

    fn find_children(&self, skill_id: &str) -> Result<Vec<SkillNode>, ForgeError>;

    fn find_parents(&self, skill_id: &str) -> Result<Vec<SkillNode>, ForgeError>;

    fn find_related(
        &self,
        skill_id: &str,
        edge_types: &[EdgeType],
    ) -> Result<Vec<RelatedSkill>, ForgeError>;

    fn n_hop_neighbors(
        &self,
        skill_id: &str,
        n: usize,
        edge_types: Option<&[EdgeType]>,
    ) -> Result<NeighborhoodSubgraph, ForgeError>;

    /// Exact match on `canonical_name`, falling back to exact match on any
    /// entry of the JSON `aliases` column.
    fn find_by_name(&self, name: &str) -> Result<Option<SkillNode>, ForgeError>;

    /// Co-occurrence statistics with `skill_id` as the source.
    ///
    /// `time_window = None` returns one row per co-occurs edge using the
    /// edge's headline `weight` column (most-recent window in the temporal
    /// series, by convention).
    ///
    /// `time_window = Some("2026-Q1")` parses each edge's `temporal_data`
    /// JSON array and returns only edges that have an entry for that window,
    /// using the windowed weight + jd_count.
    fn co_occurrence_stats(
        &self,
        skill_id: &str,
        time_window: Option<&str>,
    ) -> Result<Vec<CoOccurrenceStat>, ForgeError>;
}

// ────────────────────────────────────────────────────────────────────────
// Snapshot format (forge-ubxb)
// ────────────────────────────────────────────────────────────────────────
//
// Distribution format for the global skill graph. Built server-side, uploaded
// to a CDN, fetched by the browser, cached in IndexedDB, and loaded into the
// forge-wasm runtime. The format is designed to keep the embedding payload
// (which dominates total size) out of the JSON header so it can be transferred
// as raw bytes — a JSON-encoded `Vec<u8>` would explode the wire size 4×.
//
// Container layout:
//
//   ┌────────────────────────────────────────────────────────────────┐
//   │ MAGIC          (4 bytes)  ASCII "FSGS" — Forge Skill Graph Snap │
//   │ FORMAT_VERSION (4 bytes)  big-endian u32                        │
//   │ HEADER_LEN     (4 bytes)  big-endian u32 — length of JSON header│
//   │ HEADER         (HEADER_LEN bytes)  UTF-8 JSON, see SnapshotHeader│
//   │ EMBEDDINGS     (header.embedding_byte_length bytes)             │
//   │ HNSW_INDEX     (header.hnsw_index_byte_length bytes)            │
//   └────────────────────────────────────────────────────────────────┘
//
// Embeddings are stored as a contiguous packed array of native-endian f32 bytes.
// At index `i` of `header.nodes`, embedding `i` is `embeddings[i*dim*4 .. (i+1)*dim*4]`.
// If a node has no embedding yet (e.g. extracted but not embedded), `dim` is 0
// for the snapshot OR a sentinel record marks that index as missing — the MVP
// implementation requires either ALL or NONE of the nodes have embeddings.
//
// HNSW_INDEX is opaque to this crate; the runtime that consumes it
// (forge-afyg, usearch-rs / hnswlib-rs) chooses the on-disk format. Empty for
// snapshots built before the HNSW builder lands.

/// Current on-the-wire format version. Bump on any breaking change to the
/// header schema or container layout. Readers MUST refuse versions they don't
/// know.
pub const SNAPSHOT_FORMAT_VERSION: u32 = 1;

/// Magic bytes at the start of every snapshot file.
pub const SNAPSHOT_MAGIC: [u8; 4] = *b"FSGS";

/// Top-level metadata. Lives at the root of the JSON header so consumers can
/// validate compatibility before deserializing the rest of the snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SnapshotMetadata {
    /// Free-form snapshot identifier (e.g. timestamped build id).
    pub snapshot_id: String,
    /// ISO-8601 UTC build time.
    pub created_at: String,
    /// Number of nodes in this snapshot.
    pub skill_count: u32,
    /// Number of edges in this snapshot.
    pub edge_count: u32,
    /// Embedding model identifier (e.g. `"all-MiniLM-L6-v2@v1"`).
    /// `None` when no embeddings are included.
    pub embedding_model: Option<String>,
    /// Embedding dimension (e.g. 384 for MiniLM-L6).
    /// `None` when no embeddings are included.
    pub embedding_dim: Option<u32>,
}

/// Compact projection of a node for over-the-wire transport. Excludes columns
/// that are either heavy (embedding lives in the binary payload, not here) or
/// server-private (legacy_skill_id, level_descriptors, timestamps).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SnapshotNode {
    pub id: String,
    pub canonical_name: String,
    pub category: String,
    pub aliases: Vec<String>,
    pub source: NodeSource,
    pub confidence: f64,
}

/// Compact projection of an edge.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SnapshotEdge {
    pub source_id: String,
    pub target_id: String,
    pub edge_type: EdgeType,
    pub weight: f64,
    pub confidence: f64,
    /// JSON array of windowed measurements (only set for `co-occurs`). Kept
    /// as a raw String to avoid coupling to a temporal-data schema that may
    /// evolve independently.
    pub temporal_data: Option<String>,
}

/// Aggregate market statistics for a single skill in a single window.
/// Populated by the curation pipeline (forge-c4i5); empty in MVP snapshots.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MarketStat {
    pub skill_id: String,
    pub window: String,
    pub jd_count: i64,
    pub demand_score: f64,
}

/// JSON-serialized header that lives between MAGIC/HEADER_LEN and the binary
/// payload sections.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SnapshotHeader {
    pub metadata: SnapshotMetadata,
    pub nodes: Vec<SnapshotNode>,
    pub edges: Vec<SnapshotEdge>,
    pub market_stats: Vec<MarketStat>,
    /// Length in bytes of the embeddings payload that follows the header.
    /// Always equals `nodes.len() * embedding_dim * 4` when embeddings are
    /// present; equals 0 when they're not.
    pub embedding_byte_length: u64,
    /// Length in bytes of the HNSW index payload that follows the embeddings.
    pub hnsw_index_byte_length: u64,
}

/// In-memory view of a complete snapshot. The serialized form is produced by
/// `Self::encode` and read back by `Self::decode`.
#[derive(Debug, Clone, PartialEq)]
pub struct SkillGraphSnapshot {
    pub header: SnapshotHeader,
    /// Concatenated native-endian f32 bytes, one block per node in `nodes`.
    /// Length == `header.embedding_byte_length`.
    pub embeddings: Vec<u8>,
    /// Opaque HNSW index produced by the runtime that consumes this snapshot.
    /// Length == `header.hnsw_index_byte_length`.
    pub hnsw_index: Vec<u8>,
}

impl SkillGraphSnapshot {
    /// Build a snapshot WITHOUT embeddings or HNSW index — useful for tests
    /// and as the MVP form (forge-afyg will populate the binary sections).
    pub fn structural_only(
        metadata_id: impl Into<String>,
        created_at: impl Into<String>,
        nodes: Vec<SnapshotNode>,
        edges: Vec<SnapshotEdge>,
        market_stats: Vec<MarketStat>,
    ) -> Self {
        let metadata = SnapshotMetadata {
            snapshot_id: metadata_id.into(),
            created_at: created_at.into(),
            skill_count: nodes.len() as u32,
            edge_count: edges.len() as u32,
            embedding_model: None,
            embedding_dim: None,
        };
        let header = SnapshotHeader {
            metadata,
            nodes,
            edges,
            market_stats,
            embedding_byte_length: 0,
            hnsw_index_byte_length: 0,
        };
        Self {
            header,
            embeddings: Vec::new(),
            hnsw_index: Vec::new(),
        }
    }

    /// Encode the snapshot to bytes per the documented container layout.
    pub fn encode(&self) -> Result<Vec<u8>, ForgeError> {
        // Sanity-check that header lengths match payload lengths so consumers
        // can trust the metadata.
        if self.header.embedding_byte_length as usize != self.embeddings.len() {
            return Err(ForgeError::Internal(format!(
                "snapshot header.embedding_byte_length ({}) != embeddings.len() ({})",
                self.header.embedding_byte_length,
                self.embeddings.len()
            )));
        }
        if self.header.hnsw_index_byte_length as usize != self.hnsw_index.len() {
            return Err(ForgeError::Internal(format!(
                "snapshot header.hnsw_index_byte_length ({}) != hnsw_index.len() ({})",
                self.header.hnsw_index_byte_length,
                self.hnsw_index.len()
            )));
        }

        let header_json = serde_json::to_vec(&self.header).map_err(|e| {
            ForgeError::Internal(format!("snapshot header JSON encode failed: {e}"))
        })?;
        let header_len = u32::try_from(header_json.len())
            .map_err(|_| ForgeError::Internal("snapshot header exceeds 4 GiB".to_string()))?;

        let mut out = Vec::with_capacity(
            12 + header_json.len() + self.embeddings.len() + self.hnsw_index.len(),
        );
        out.extend_from_slice(&SNAPSHOT_MAGIC);
        out.extend_from_slice(&SNAPSHOT_FORMAT_VERSION.to_be_bytes());
        out.extend_from_slice(&header_len.to_be_bytes());
        out.extend_from_slice(&header_json);
        out.extend_from_slice(&self.embeddings);
        out.extend_from_slice(&self.hnsw_index);
        Ok(out)
    }

    /// Decode a snapshot from bytes. Returns an error if the magic, version,
    /// or any length field is invalid.
    pub fn decode(bytes: &[u8]) -> Result<Self, ForgeError> {
        if bytes.len() < 12 {
            return Err(ForgeError::Internal(
                "snapshot too short for fixed header".to_string(),
            ));
        }
        let mut cursor = 0_usize;

        let magic = &bytes[cursor..cursor + 4];
        cursor += 4;
        if magic != SNAPSHOT_MAGIC {
            return Err(ForgeError::Internal(format!(
                "bad snapshot magic: {magic:?}"
            )));
        }

        let version = u32::from_be_bytes(bytes[cursor..cursor + 4].try_into().unwrap());
        cursor += 4;
        if version != SNAPSHOT_FORMAT_VERSION {
            return Err(ForgeError::Internal(format!(
                "unsupported snapshot format version {version} (this build understands {SNAPSHOT_FORMAT_VERSION})"
            )));
        }

        let header_len = u32::from_be_bytes(bytes[cursor..cursor + 4].try_into().unwrap()) as usize;
        cursor += 4;
        if cursor + header_len > bytes.len() {
            return Err(ForgeError::Internal(
                "snapshot header length exceeds payload".to_string(),
            ));
        }
        let header: SnapshotHeader = serde_json::from_slice(&bytes[cursor..cursor + header_len])
            .map_err(|e| {
                ForgeError::Internal(format!("snapshot header JSON decode failed: {e}"))
            })?;
        cursor += header_len;

        let emb_len = header.embedding_byte_length as usize;
        if cursor + emb_len > bytes.len() {
            return Err(ForgeError::Internal(
                "snapshot embedding length exceeds payload".to_string(),
            ));
        }
        let embeddings = bytes[cursor..cursor + emb_len].to_vec();
        cursor += emb_len;

        let hnsw_len = header.hnsw_index_byte_length as usize;
        if cursor + hnsw_len > bytes.len() {
            return Err(ForgeError::Internal(
                "snapshot hnsw index length exceeds payload".to_string(),
            ));
        }
        let hnsw_index = bytes[cursor..cursor + hnsw_len].to_vec();

        Ok(Self {
            header,
            embeddings,
            hnsw_index,
        })
    }
}

// ────────────────────────────────────────────────────────────────────────
// Snapshot tests
// ────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod snapshot_tests {
    use super::*;

    fn sample_node(id: &str, name: &str) -> SnapshotNode {
        SnapshotNode {
            id: id.to_string(),
            canonical_name: name.to_string(),
            category: "tool".to_string(),
            aliases: vec!["foo".to_string(), "bar".to_string()],
            source: NodeSource::Curated,
            confidence: 0.97,
        }
    }

    fn sample_edge(src: &str, tgt: &str) -> SnapshotEdge {
        SnapshotEdge {
            source_id: src.to_string(),
            target_id: tgt.to_string(),
            edge_type: EdgeType::ParentOf,
            weight: 1.0,
            confidence: 1.0,
            temporal_data: None,
        }
    }

    #[test]
    fn structural_only_snapshot_roundtrips() {
        let snap = SkillGraphSnapshot::structural_only(
            "snap-001",
            "2026-04-27T00:00:00Z",
            vec![sample_node("a", "Alpha"), sample_node("b", "Beta")],
            vec![sample_edge("a", "b")],
            Vec::new(),
        );

        let bytes = snap.encode().unwrap();
        let decoded = SkillGraphSnapshot::decode(&bytes).unwrap();
        assert_eq!(decoded, snap);
    }

    #[test]
    fn snapshot_with_simulated_embeddings_roundtrips() {
        let nodes = vec![sample_node("a", "Alpha"), sample_node("b", "Beta")];
        let dim = 4_u32;
        // Two nodes × 4 dims × 4 bytes = 32 bytes. Use a recognizable pattern.
        let embeddings: Vec<u8> = (0..nodes.len() * dim as usize)
            .flat_map(|i| (i as f32).to_ne_bytes())
            .collect();

        let mut header = SnapshotHeader {
            metadata: SnapshotMetadata {
                snapshot_id: "snap-002".to_string(),
                created_at: "2026-04-27T00:00:00Z".to_string(),
                skill_count: nodes.len() as u32,
                edge_count: 0,
                embedding_model: Some("test-model@v1".to_string()),
                embedding_dim: Some(dim),
            },
            nodes,
            edges: Vec::new(),
            market_stats: Vec::new(),
            embedding_byte_length: embeddings.len() as u64,
            hnsw_index_byte_length: 0,
        };
        // tiny HNSW blob to also exercise that section
        let hnsw_index = vec![0xAA, 0xBB, 0xCC, 0xDD];
        header.hnsw_index_byte_length = hnsw_index.len() as u64;

        let snap = SkillGraphSnapshot {
            header,
            embeddings,
            hnsw_index,
        };

        let bytes = snap.encode().unwrap();
        let decoded = SkillGraphSnapshot::decode(&bytes).unwrap();
        assert_eq!(decoded, snap);
    }

    #[test]
    fn decode_rejects_bad_magic() {
        let mut bad = vec![b'X', b'X', b'X', b'X'];
        bad.extend_from_slice(&SNAPSHOT_FORMAT_VERSION.to_be_bytes());
        bad.extend_from_slice(&0_u32.to_be_bytes());
        let err = SkillGraphSnapshot::decode(&bad).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("magic"), "expected magic error, got: {msg}");
    }

    #[test]
    fn decode_rejects_unknown_version() {
        let mut bad = SNAPSHOT_MAGIC.to_vec();
        bad.extend_from_slice(&999_u32.to_be_bytes());
        bad.extend_from_slice(&0_u32.to_be_bytes());
        let err = SkillGraphSnapshot::decode(&bad).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("version"),
            "expected version error, got: {msg}"
        );
    }

    #[test]
    fn encode_rejects_inconsistent_header_lengths() {
        let mut snap = SkillGraphSnapshot::structural_only(
            "snap-bad",
            "2026-04-27T00:00:00Z",
            vec![sample_node("a", "Alpha")],
            Vec::new(),
            Vec::new(),
        );
        snap.header.embedding_byte_length = 100; // lie about embedding size
        let err = snap.encode().unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("embedding_byte_length"));
    }

    /// Verify that a populated snapshot at 10k nodes with 384-dim float32
    /// embeddings stays under the 25 MB budget called out in the bead's
    /// acceptance criteria. The HNSW slot is empty here (forge-afyg fills it).
    #[test]
    fn snapshot_at_10k_nodes_fits_under_25mb() {
        const N: usize = 10_000;
        const DIM: usize = 384;
        const MAX_BYTES: usize = 25 * 1024 * 1024;

        let nodes: Vec<SnapshotNode> = (0..N)
            .map(|i| SnapshotNode {
                // Real UUIDs are 36 chars — match width so the size estimate
                // is realistic.
                id: format!("{:036}", i),
                canonical_name: format!("Skill {i}"),
                category: "tool".to_string(),
                aliases: vec![format!("alias{i}-a"), format!("alias{i}-b")],
                source: NodeSource::Seed,
                confidence: 1.0,
            })
            .collect();

        // Average ~3 outgoing edges per node — realistic for a hierarchical
        // graph with co-occurrence enrichment.
        let edges: Vec<SnapshotEdge> = (0..3 * N)
            .map(|i| SnapshotEdge {
                source_id: format!("{:036}", i % N),
                target_id: format!("{:036}", (i * 7 + 1) % N),
                edge_type: EdgeType::CoOccurs,
                weight: 0.5,
                confidence: 0.9,
                temporal_data: None,
            })
            .collect();

        let embeddings: Vec<u8> = vec![0_u8; N * DIM * 4];
        assert_eq!(embeddings.len(), N * DIM * 4); // 15 MiB

        let header = SnapshotHeader {
            metadata: SnapshotMetadata {
                snapshot_id: "snap-10k".to_string(),
                created_at: "2026-04-27T00:00:00Z".to_string(),
                skill_count: N as u32,
                edge_count: edges.len() as u32,
                embedding_model: Some("all-MiniLM-L6-v2@v1".to_string()),
                embedding_dim: Some(DIM as u32),
            },
            nodes,
            edges,
            market_stats: Vec::new(),
            embedding_byte_length: embeddings.len() as u64,
            hnsw_index_byte_length: 0,
        };
        let snap = SkillGraphSnapshot {
            header,
            embeddings,
            hnsw_index: Vec::new(),
        };

        let bytes = snap.encode().unwrap();
        assert!(
            bytes.len() < MAX_BYTES,
            "snapshot at 10k nodes is {} bytes ({:.2} MiB), exceeds 25 MiB budget",
            bytes.len(),
            bytes.len() as f64 / (1024.0 * 1024.0)
        );

        // Sanity: also round-trip decodes.
        let decoded = SkillGraphSnapshot::decode(&bytes).unwrap();
        assert_eq!(decoded.header.metadata.skill_count, N as u32);
        assert_eq!(decoded.embeddings.len(), N * DIM * 4);
    }
}

// ────────────────────────────────────────────────────────────────────────
// Tests — round-trips for the enum vocabularies.
// ────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn edge_type_kebab_case_roundtrip() {
        for et in EdgeType::ALL {
            let s = et.as_ref();
            let parsed = EdgeType::from_str(s).unwrap();
            assert_eq!(parsed, et, "round-trip failed for {s}");
        }
    }

    #[test]
    fn edge_type_string_form_matches_sql_check() {
        // These strings match the CHECK constraint in migration 052. Touching
        // either side requires updating the other.
        assert_eq!(EdgeType::AliasOf.as_ref(), "alias-of");
        assert_eq!(EdgeType::ParentOf.as_ref(), "parent-of");
        assert_eq!(EdgeType::ChildOf.as_ref(), "child-of");
        assert_eq!(EdgeType::Prerequisite.as_ref(), "prerequisite");
        assert_eq!(EdgeType::RelatedTo.as_ref(), "related-to");
        assert_eq!(EdgeType::CoOccurs.as_ref(), "co-occurs");
        assert_eq!(EdgeType::PlatformFor.as_ref(), "platform-for");
    }

    #[test]
    fn node_source_string_form_matches_sql_check() {
        // Matches the CHECK constraint on skill_graph_nodes.source.
        assert_eq!(NodeSource::Seed.as_ref(), "seed");
        assert_eq!(NodeSource::Extracted.as_ref(), "extracted");
        assert_eq!(NodeSource::Curated.as_ref(), "curated");
        assert_eq!(NodeSource::UserCreated.as_ref(), "user-created");
    }
}
