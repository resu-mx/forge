//! Forge browser runtime.
//!
//! A Rust library compiled to WebAssembly that runs the **same code as the native
//! server** inside a browser Worker: `forge-sdk`'s stores over rusqlite, behind
//! `forge-api`'s axum router, with SQLite persisted in the Origin Private File
//! System. The web UI keeps talking HTTP through the TypeScript SDK; only the
//! transport changes (a Worker message instead of a socket).
//!
//! ## Layout
//!
//! - [`dispatch`] — run one HTTP-shaped request through the router. Host-independent,
//!   tested natively.
//! - `runtime` (wasm32 only) — OPFS storage, the database, and the wasm-bindgen
//!   exports a Worker calls: `start`, `request`, `exportDatabase`, `importDatabase`.
//! - [`skill_graph`], [`alignment`] — the browser-side compute modules (graph traversal,
//!   HNSW search, alignment scoring).
//!
//! ## Architectural rules
//!
//! - `forge-server` MUST NOT depend on this crate. The workspace omits `forge-wasm` from
//!   `[workspace.dependencies]` as a passive guard.
//! - The wasm-bindgen surface is **coarse-grained**: each export does substantial work
//!   and returns a complete result. No chatty per-field calls across the boundary.
//! - Must run in a **dedicated Worker** (OPFS sync access handles); one database owner
//!   per origin. See `docs/src/dev/adrs/rust-wasm/`.

use wasm_bindgen::prelude::*;

pub mod alignment;
pub mod dispatch;
pub mod error;
pub mod skill_graph;

#[cfg(target_arch = "wasm32")]
pub mod runtime;

/// Format version of the WASM bundle. Bumped on any breaking change to the
/// JS-facing API. Read from JS via [`bundle_version`].
pub const BUNDLE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Install a panic hook that forwards Rust panics to the browser console.
/// Idempotent — safe to call multiple times.
#[wasm_bindgen(start)]
pub fn _wasm_init() {
    console_error_panic_hook::set_once();
}

/// Bundle version exposed to JS.
#[wasm_bindgen(js_name = bundleVersion)]
pub fn bundle_version() -> String {
    BUNDLE_VERSION.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundle_version_matches_cargo_pkg() {
        assert_eq!(bundle_version(), env!("CARGO_PKG_VERSION"));
    }
}
