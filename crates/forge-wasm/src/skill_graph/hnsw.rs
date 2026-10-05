//! Nearest-neighbor index seam for the skill graph runtime.
//!
//! # Prototype implementation — NOT a long-term choice
//!
//! This module currently ships a **brute-force linear-scan** backing impl.
//! For every search query it computes cosine similarity against every stored
//! vector and partial-sorts the results. That's O(N·d) per query.
//!
//! Why this is acceptable for the SaaS prototype:
//! - At N≈10k and d=384, one query is ~7.7M f32 ops — well under the 50ms
//!   acceptance criterion in WASM.
//! - It avoids pulling in `hnsw_rs` / `instant-distance` / `usearch`, all of
//!   which currently require workspace-level rustflags surgery to build for
//!   `wasm32-unknown-unknown` (their transitive `getrandom` deps don't pick a
//!   wasm-compatible backend by default).
//! - It defers the long-term HNSW library choice — under active research at
//!   the time of writing — without blocking the rest of forge-afyg.
//!
//! When the long-term library is chosen, swap the bodies of [`HnswIndex::build`]
//! and [`HnswIndex::search`] only. The seam's public surface stays identical;
//! callers in `super::SkillGraphRuntime` and downstream consumers don't move.
//!
//! Limitations of the prototype to remember:
//! - No incremental insertion (rebuild the whole index to add vectors).
//! - O(N) per query — fine at 10k, painful at 1M+.
//! - No persistence — the snapshot's `hnsw_index` payload slot stays empty;
//!   the index is rebuilt at every load. (Build cost at 10k×384 is small
//!   enough to keep the AC's <500ms total load budget.)

use forge_core::ForgeError;

/// Internal node identifier — index into the snapshot's `nodes` array. The
/// runtime maps this to the public string id at the call site.
pub(crate) type NodeIndex = usize;

/// Nearest-neighbor index over a fixed set of equal-length vectors.
///
/// Consult the module-level rustdoc for the prototype caveat: the backing
/// implementation is brute-force linear scan and is intended to be swapped
/// once the long-term library evaluation completes.
#[derive(Debug)]
pub(crate) struct HnswIndex {
    /// Length of every stored vector. Queries with mismatched dim are rejected.
    dim: usize,
    /// Stored vectors paired with their internal id, in insertion order.
    /// Vectors are normalized (L2) at build time so [`Self::search`] reduces
    /// to a dot product.
    entries: Vec<(NodeIndex, Vec<f32>)>,
}

impl HnswIndex {
    /// Build an index from a slice of `(node_index, vector)` pairs.
    ///
    /// Every vector must have length `dim`. Zero-norm vectors are stored
    /// as-is and will produce a similarity score of 0 for every query.
    pub(crate) fn build(vectors: &[(NodeIndex, &[f32])], dim: usize) -> Result<Self, ForgeError> {
        let mut entries = Vec::with_capacity(vectors.len());
        for (id, v) in vectors {
            if v.len() != dim {
                return Err(ForgeError::Internal(format!(
                    "HnswIndex::build: vector for node {} has length {}, expected dim {}",
                    id,
                    v.len(),
                    dim
                )));
            }
            entries.push((*id, l2_normalize(v)));
        }
        Ok(Self { dim, entries })
    }

    /// Return up to `k` nearest neighbors of `query`, by cosine similarity,
    /// in descending order of score. Higher score = more similar.
    ///
    /// Score is in `[-1.0, 1.0]` for non-zero vectors. A zero-norm query
    /// produces a score of 0 against every entry.
    pub(crate) fn search(&self, query: &[f32], k: usize) -> Vec<(NodeIndex, f32)> {
        if query.len() != self.dim || k == 0 || self.entries.is_empty() {
            return Vec::new();
        }
        let q = l2_normalize(query);

        let mut scored: Vec<(NodeIndex, f32)> = self
            .entries
            .iter()
            .map(|(id, v)| (*id, dot(&q, v)))
            .collect();

        let take = k.min(scored.len());
        scored.select_nth_unstable_by(take.saturating_sub(1), |a, b| {
            b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
        });
        scored.truncate(take);
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored
    }
}

fn l2_normalize(v: &[f32]) -> Vec<f32> {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm == 0.0 {
        return v.to_vec();
    }
    v.iter().map(|x| x / norm).collect()
}

fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_scan_finds_nearest_neighbor_by_cosine_similarity() {
        // Three vectors in 2-D. Query is close to vector 0 ((1,0)).
        // Expected ranking by cosine similarity: 0 (best) → 2 → 1 (worst).
        let v0 = vec![1.0_f32, 0.0];
        let v1 = vec![0.0_f32, 1.0];
        let v2 = vec![0.7_f32, 0.7];
        let owned = [(0_usize, v0), (1_usize, v1), (2_usize, v2)];
        let refs: Vec<(NodeIndex, &[f32])> =
            owned.iter().map(|(i, v)| (*i, v.as_slice())).collect();

        let idx = HnswIndex::build(&refs, 2).expect("build");

        let results = idx.search(&[1.0_f32, 0.05], 3);

        assert_eq!(results.len(), 3, "k=3 with 3 entries returns all three");
        assert_eq!(results[0].0, 0, "nearest must be the (1,0) vector");
        assert_eq!(
            results[1].0, 2,
            "second-nearest must be the (0.7,0.7) vector"
        );
        assert_eq!(results[2].0, 1, "farthest must be the (0,1) vector");

        // Scores are monotonically non-increasing.
        assert!(results[0].1 >= results[1].1);
        assert!(results[1].1 >= results[2].1);

        // Score of (1,0) against query close to (1,0) is near 1.0.
        assert!(
            results[0].1 > 0.99,
            "expected near-perfect cosine sim, got {}",
            results[0].1
        );
    }

    #[test]
    fn build_rejects_mismatched_dim() {
        let owned = vec![(0_usize, vec![1.0_f32, 0.0, 0.0])];
        let refs: Vec<(NodeIndex, &[f32])> =
            owned.iter().map(|(i, v)| (*i, v.as_slice())).collect();

        let err = HnswIndex::build(&refs, 2).expect_err("dim 2 vs vector len 3 should fail");
        let msg = format!("{err}");
        assert!(
            msg.contains("dim 2") || msg.contains("expected dim 2"),
            "error should mention expected dim, got: {msg}"
        );
    }

    #[test]
    fn search_returns_empty_on_dim_mismatch_or_empty_index() {
        // Empty index → empty results regardless of k.
        let empty = HnswIndex::build(&[], 4).unwrap();
        assert!(empty.search(&[0.0; 4], 5).is_empty());

        // Mismatched query dim → empty (treated as no-match rather than panic).
        let owned = vec![(0_usize, vec![1.0_f32; 4])];
        let refs: Vec<(NodeIndex, &[f32])> =
            owned.iter().map(|(i, v)| (*i, v.as_slice())).collect();
        let idx = HnswIndex::build(&refs, 4).unwrap();
        assert!(idx.search(&[1.0_f32; 3], 1).is_empty());

        // k = 0 → empty.
        assert!(idx.search(&[1.0_f32; 4], 0).is_empty());
    }

    #[test]
    fn search_caps_at_k_when_index_is_larger() {
        let owned: Vec<(usize, Vec<f32>)> =
            (0..10).map(|i| (i, vec![i as f32 + 1.0, 0.0])).collect();
        let refs: Vec<(NodeIndex, &[f32])> =
            owned.iter().map(|(i, v)| (*i, v.as_slice())).collect();
        let idx = HnswIndex::build(&refs, 2).unwrap();

        let results = idx.search(&[1.0_f32, 0.0], 3);
        assert_eq!(results.len(), 3);
    }
}
