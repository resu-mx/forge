//! JS-facing wrapper for [`SkillGraphRuntime`].
//!
//! Exposes a coarse-grained API matching the bead's deliverables:
//!
//! - `SkillGraphRuntime.fromSnapshot(bytes)` — async, returns the wrapper
//! - `runtime.searchSkills(query, top_k)` — JSON-encoded `SkillNode[]`
//! - `runtime.searchByEmbedding(query, top_k)` — JSON-encoded
//!   `{node, score}[]` for callers that already hold an embedding vector.
//!
//! Results are returned as JSON strings so JS callers `JSON.parse(...)` once
//! per call. This avoids pulling in `serde-wasm-bindgen` for the prototype
//! and keeps the WASM↔JS boundary simple. forge-5x2h is responsible for the
//! polished JS API; this module is the minimum forge-afyg owes downstream
//! consumers.

use serde::Serialize;
use wasm_bindgen::prelude::*;

use super::SkillGraphRuntime;

/// JS-side handle to a loaded skill graph runtime.
#[wasm_bindgen(js_name = SkillGraphRuntime)]
pub struct SkillGraphRuntimeJs {
    inner: SkillGraphRuntime,
}

#[wasm_bindgen(js_class = SkillGraphRuntime)]
impl SkillGraphRuntimeJs {
    /// Decode a snapshot byte slice and load it into in-memory structures.
    /// Eager build — by the time this returns, the petgraph and HNSW index
    /// are fully constructed.
    #[wasm_bindgen(js_name = fromSnapshot)]
    pub fn from_snapshot(bytes: &[u8]) -> Result<SkillGraphRuntimeJs, JsValue> {
        SkillGraphRuntime::from_snapshot(bytes)
            .map(|inner| SkillGraphRuntimeJs { inner })
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Substring autocomplete. Returns a JSON-encoded `SkillNode[]`.
    #[wasm_bindgen(js_name = searchSkills)]
    pub fn search_skills(&self, query: &str, top_k: usize) -> Result<String, JsValue> {
        let hits = self.inner.search_skills(query, top_k);
        serde_json::to_string(&hits)
            .map_err(|e| JsValue::from_str(&format!("encode search_skills result: {e}")))
    }

    /// Vector search. The query MUST be `embedding_dim`-many f32s in native
    /// byte order, matching the snapshot's embedding model. Returns a
    /// JSON-encoded `{ skill, score }[]`.
    #[wasm_bindgen(js_name = searchByEmbedding)]
    pub fn search_by_embedding(&self, query: Vec<f32>, top_k: usize) -> Result<String, JsValue> {
        #[derive(Serialize)]
        struct Hit<'a> {
            skill: &'a forge_core::types::skill_graph::SkillNode,
            score: f32,
        }
        let raw = self.inner.search_by_embedding(&query, top_k);
        let hits: Vec<Hit> = raw
            .iter()
            .map(|(skill, score)| Hit {
                skill,
                score: *score,
            })
            .collect();
        serde_json::to_string(&hits)
            .map_err(|e| JsValue::from_str(&format!("encode search_by_embedding result: {e}")))
    }
}
