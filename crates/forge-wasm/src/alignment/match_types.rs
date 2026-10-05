//! Per-match-type scoring. Each match type is a function that takes the JD
//! skill context + the resume's skill set + traversal trait and returns
//! `Option<MatchHit>` — `None` if the match type doesn't fire.
//!
//! The engine (engine.rs) calls every match-type function for every JD
//! skill, collects all hits, takes the maximum-scored hit per JD skill for
//! the per-skill score, and emits ALL hits as provenance entries.

use std::collections::HashSet;

use forge_core::types::skill_graph::SkillGraphTraversal;
use forge_core::ForgeError;
use serde::{Deserialize, Serialize};

use super::config::MatchWeights;
use super::embedding_nn::EmbeddingNearestNeighbor;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum MatchType {
    Direct,
    Alias,
    Cert,
    Child,
    Parent,
    Sibling,
    Embedding,
    CoOccurrence,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchHit {
    pub match_type: MatchType,
    pub resume_skill_id: String,
    pub raw_score: f64,
}

/// Direct match: JD skill exactly equals a resume skill.
pub fn direct_match(
    jd_skill_id: &str,
    resume_skill_ids: &HashSet<String>,
    weights: &MatchWeights,
) -> Option<MatchHit> {
    if resume_skill_ids.contains(jd_skill_id) {
        Some(MatchHit {
            match_type: MatchType::Direct,
            resume_skill_id: jd_skill_id.to_string(),
            raw_score: weights.direct,
        })
    } else {
        None
    }
}

/// Alias match: JD skill has graph-edge aliases; one of the aliases is in
/// the resume.
pub fn alias_match(
    jd_skill_id: &str,
    resume_skill_ids: &HashSet<String>,
    graph: &dyn SkillGraphTraversal,
    weights: &MatchWeights,
) -> Result<Option<MatchHit>, ForgeError> {
    let aliases = graph.find_aliases(jd_skill_id)?;
    for a in aliases {
        if resume_skill_ids.contains(&a.id) {
            return Ok(Some(MatchHit {
                match_type: MatchType::Alias,
                resume_skill_id: a.id,
                raw_score: weights.alias,
            }));
        }
    }
    Ok(None)
}

/// Child match: JD asks broad, resume has specific.
pub fn child_match(
    jd_skill_id: &str,
    resume_skill_ids: &HashSet<String>,
    graph: &dyn SkillGraphTraversal,
    weights: &MatchWeights,
) -> Result<Option<MatchHit>, ForgeError> {
    let children = graph.find_children(jd_skill_id)?;
    for c in children {
        if resume_skill_ids.contains(&c.id) {
            return Ok(Some(MatchHit {
                match_type: MatchType::Child,
                resume_skill_id: c.id,
                raw_score: weights.child,
            }));
        }
    }
    Ok(None)
}

/// Parent match: JD asks specific, resume has broad.
pub fn parent_match(
    jd_skill_id: &str,
    resume_skill_ids: &HashSet<String>,
    graph: &dyn SkillGraphTraversal,
    weights: &MatchWeights,
) -> Result<Option<MatchHit>, ForgeError> {
    let parents = graph.find_parents(jd_skill_id)?;
    for p in parents {
        if resume_skill_ids.contains(&p.id) {
            return Ok(Some(MatchHit {
                match_type: MatchType::Parent,
                resume_skill_id: p.id,
                raw_score: weights.parent,
            }));
        }
    }
    Ok(None)
}

/// Sibling match: resume has a related-to alternative.
pub fn sibling_match(
    jd_skill_id: &str,
    resume_skill_ids: &HashSet<String>,
    graph: &dyn SkillGraphTraversal,
    weights: &MatchWeights,
) -> Result<Option<MatchHit>, ForgeError> {
    use forge_core::types::skill_graph::EdgeType;
    let siblings = graph.find_related(jd_skill_id, &[EdgeType::RelatedTo])?;
    for s in siblings {
        if resume_skill_ids.contains(&s.skill.id) {
            return Ok(Some(MatchHit {
                match_type: MatchType::Sibling,
                resume_skill_id: s.skill.id,
                raw_score: weights.sibling,
            }));
        }
    }
    Ok(None)
}

/// Cert match: resume has a certification that validates this JD skill.
/// `validated_skill_ids` is pre-resolved upstream from cert→skill junction
/// tables (no `EdgeType::Validates` exists in the graph today).
pub fn cert_match(
    jd_skill_id: &str,
    validated_skill_ids: &HashSet<String>,
    weights: &MatchWeights,
) -> Option<MatchHit> {
    if validated_skill_ids.contains(jd_skill_id) {
        Some(MatchHit {
            match_type: MatchType::Cert,
            // The cert validates the requirement directly — no separate
            // resume-side skill node. The resume_skill_id is the JD skill
            // itself; provenance carries the actual cert via a separate
            // path (engine.rs reads the cert from ResumeAlignmentInput).
            resume_skill_id: jd_skill_id.to_string(),
            raw_score: weights.cert,
        })
    } else {
        None
    }
}

/// Embedding proximity match. `query_embedding` is the JD skill's embedding
/// (caller supplies it). Returns the highest-scoring hit ∈ resume above
/// `min_similarity`. Cosine similarity range is roughly [-1.0, 1.0]; values
/// closer to 1.0 mean more similar.
pub fn embedding_match(
    query_embedding: &[f32],
    resume_skill_ids: &HashSet<String>,
    nn: &dyn EmbeddingNearestNeighbor,
    weights: &MatchWeights,
    min_similarity: f64,
    top_k: usize,
) -> Result<Option<MatchHit>, ForgeError> {
    let hits = nn.search_by_embedding(query_embedding, top_k)?;
    for (node, sim) in hits {
        if (sim as f64) < min_similarity {
            continue;
        }
        if resume_skill_ids.contains(&node.id) {
            return Ok(Some(MatchHit {
                match_type: MatchType::Embedding,
                resume_skill_id: node.id,
                raw_score: weights.embedding_max * (sim as f64),
            }));
        }
    }
    Ok(None)
}

/// Co-occurrence match. Returns Ok(None) when stats are empty (forge-c4i5
/// hasn't shipped) OR when the JD skill is unknown to the graph (returns
/// NotFound from the trait). Never panics.
pub fn co_occurrence_match(
    jd_skill_id: &str,
    resume_skill_ids: &HashSet<String>,
    graph: &dyn SkillGraphTraversal,
    weights: &MatchWeights,
) -> Result<Option<MatchHit>, ForgeError> {
    let stats = match graph.co_occurrence_stats(jd_skill_id, None) {
        Ok(s) => s,
        Err(ForgeError::NotFound { .. }) => return Ok(None),
        Err(e) => return Err(e),
    };
    for entry in stats {
        if resume_skill_ids.contains(&entry.skill.id) {
            return Ok(Some(MatchHit {
                match_type: MatchType::CoOccurrence,
                resume_skill_id: entry.skill.id,
                raw_score: weights.co_occurrence * entry.weight,
            }));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alignment::test_fixtures::tiny_graph_bytes;
    use crate::skill_graph::SkillGraphRuntime;

    fn fixture_runtime() -> SkillGraphRuntime {
        let bytes = tiny_graph_bytes();
        SkillGraphRuntime::from_snapshot(&bytes).expect("runtime")
    }

    fn resume(ids: &[&str]) -> HashSet<String> {
        ids.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn direct_match_fires_when_resume_has_exact_skill() {
        let resume = resume(&["kubernetes"]);
        let weights = MatchWeights::default();
        let hit = direct_match("kubernetes", &resume, &weights).unwrap();
        assert_eq!(hit.match_type, MatchType::Direct);
        assert_eq!(hit.resume_skill_id, "kubernetes");
        assert_eq!(hit.raw_score, 1.0);
    }

    #[test]
    fn direct_match_misses_when_resume_lacks_skill() {
        let resume = resume(&["python"]);
        let weights = MatchWeights::default();
        assert!(direct_match("kubernetes", &resume, &weights).is_none());
    }

    #[test]
    fn alias_match_fires_when_resume_has_alias_node() {
        // tiny_graph has AliasOf edge between kubernetes and k8s.
        let runtime = fixture_runtime();
        let resume = resume(&["k8s"]);
        let weights = MatchWeights::default();
        let hit = alias_match("kubernetes", &resume, &runtime, &weights)
            .unwrap()
            .unwrap();
        assert_eq!(hit.match_type, MatchType::Alias);
        assert_eq!(hit.resume_skill_id, "k8s");
        assert_eq!(hit.raw_score, 1.0);
    }

    #[test]
    fn alias_match_is_bidirectional_per_trait_contract() {
        // Per SkillGraphTraversal::find_aliases, AliasOf is bidirectional:
        // querying with the alias side should also find the canonical side.
        let runtime = fixture_runtime();
        let resume = resume(&["kubernetes"]);
        let weights = MatchWeights::default();
        let hit = alias_match("k8s", &resume, &runtime, &weights)
            .unwrap()
            .unwrap();
        assert_eq!(hit.match_type, MatchType::Alias);
        assert_eq!(hit.resume_skill_id, "kubernetes");
    }

    #[test]
    fn alias_match_misses_when_no_alias_in_resume() {
        let runtime = fixture_runtime();
        let resume = resume(&["docker-swarm"]);
        let weights = MatchWeights::default();
        assert!(alias_match("kubernetes", &resume, &runtime, &weights)
            .unwrap()
            .is_none());
    }

    #[test]
    fn child_match_fires_when_resume_has_specific() {
        let runtime = fixture_runtime();
        let resume = resume(&["k3s"]);
        let weights = MatchWeights::default();
        let hit = child_match("kubernetes", &resume, &runtime, &weights)
            .unwrap()
            .unwrap();
        assert_eq!(hit.match_type, MatchType::Child);
        assert_eq!(hit.resume_skill_id, "k3s");
        assert_eq!(hit.raw_score, 0.9);
    }

    #[test]
    fn child_match_misses_when_resume_lacks_specific() {
        let runtime = fixture_runtime();
        let resume = resume(&["python"]);
        let weights = MatchWeights::default();
        assert!(child_match("kubernetes", &resume, &runtime, &weights)
            .unwrap()
            .is_none());
    }

    #[test]
    fn parent_match_fires_when_resume_has_broad() {
        let runtime = fixture_runtime();
        let resume = resume(&["container-orch"]);
        let weights = MatchWeights::default();
        let hit = parent_match("kubernetes", &resume, &runtime, &weights)
            .unwrap()
            .unwrap();
        assert_eq!(hit.match_type, MatchType::Parent);
        assert_eq!(hit.resume_skill_id, "container-orch");
        assert_eq!(hit.raw_score, 0.5);
    }

    #[test]
    fn sibling_match_fires_when_resume_has_related() {
        let runtime = fixture_runtime();
        let resume = resume(&["pulumi"]);
        let weights = MatchWeights::default();
        let hit = sibling_match("terraform", &resume, &runtime, &weights)
            .unwrap()
            .unwrap();
        assert_eq!(hit.match_type, MatchType::Sibling);
        assert_eq!(hit.resume_skill_id, "pulumi");
        assert_eq!(hit.raw_score, 0.4);
    }

    #[test]
    fn sibling_match_misses_when_no_related_in_resume() {
        let runtime = fixture_runtime();
        let resume = resume(&["python"]);
        let weights = MatchWeights::default();
        assert!(sibling_match("terraform", &resume, &runtime, &weights)
            .unwrap()
            .is_none());
    }

    #[test]
    fn cert_match_fires_when_resume_cert_validates_skill() {
        let validated: HashSet<String> = vec!["kubernetes".into()].into_iter().collect();
        let weights = MatchWeights::default();
        let hit = cert_match("kubernetes", &validated, &weights).unwrap();
        assert_eq!(hit.match_type, MatchType::Cert);
        assert_eq!(hit.resume_skill_id, "kubernetes");
        assert_eq!(hit.raw_score, 0.95);
    }

    #[test]
    fn cert_match_misses_when_resume_has_no_validating_cert() {
        let validated: HashSet<String> = HashSet::new();
        let weights = MatchWeights::default();
        assert!(cert_match("kubernetes", &validated, &weights).is_none());
    }

    // ---- Embedding ----

    #[test]
    fn embedding_match_fires_when_resume_skill_is_top_canned_hit() {
        use crate::alignment::test_fixtures::{find_node_by_id, tiny_graph, MockEmbeddingNN};
        let snap = tiny_graph();
        let kube = find_node_by_id(&snap, "kubernetes").unwrap();
        let mock = MockEmbeddingNN::with(vec![(kube, 0.85)]);
        let resume = resume(&["kubernetes"]);
        let weights = MatchWeights::default();
        let hit = embedding_match(&[0.0; 384], &resume, &mock, &weights, 0.7, 20)
            .unwrap()
            .unwrap();
        assert_eq!(hit.match_type, MatchType::Embedding);
        assert_eq!(hit.resume_skill_id, "kubernetes");
        assert!((hit.raw_score - 0.85).abs() < 1e-6);
    }

    #[test]
    fn embedding_match_misses_when_similarity_below_threshold() {
        use crate::alignment::test_fixtures::{find_node_by_id, tiny_graph, MockEmbeddingNN};
        let snap = tiny_graph();
        let kube = find_node_by_id(&snap, "kubernetes").unwrap();
        let mock = MockEmbeddingNN::with(vec![(kube, 0.5)]); // below 0.7 threshold
        let resume = resume(&["kubernetes"]);
        let weights = MatchWeights::default();
        assert!(
            embedding_match(&[0.0; 384], &resume, &mock, &weights, 0.7, 20)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn embedding_match_misses_when_top_hit_not_in_resume() {
        use crate::alignment::test_fixtures::{find_node_by_id, tiny_graph, MockEmbeddingNN};
        let snap = tiny_graph();
        let kube = find_node_by_id(&snap, "kubernetes").unwrap();
        let mock = MockEmbeddingNN::with(vec![(kube, 0.9)]);
        let resume = resume(&["python"]); // resume doesn't contain kubernetes
        let weights = MatchWeights::default();
        assert!(
            embedding_match(&[0.0; 384], &resume, &mock, &weights, 0.7, 20)
                .unwrap()
                .is_none()
        );
    }

    // ---- Co-occurrence ----

    #[test]
    fn co_occurrence_match_returns_none_when_runtime_returns_empty_stats() {
        let runtime = fixture_runtime();
        let resume = resume(&["pulumi"]);
        let weights = MatchWeights::default();
        let hit = co_occurrence_match("kubernetes", &resume, &runtime, &weights).unwrap();
        assert!(
            hit.is_none(),
            "co-occurrence must return None when stats are empty (c4i5 not yet shipped)"
        );
    }

    #[test]
    fn co_occurrence_match_returns_none_when_skill_unknown_to_graph() {
        let runtime = fixture_runtime();
        let resume = resume(&["kubernetes"]);
        let weights = MatchWeights::default();
        let result = co_occurrence_match("does-not-exist", &resume, &runtime, &weights);
        assert!(
            matches!(result, Ok(None)),
            "co-occurrence on unknown skill must yield Ok(None), got {:?}",
            result
        );
    }
}
