//! Core types, enums, and shapes for Forge.
//!
//! This crate defines the shared data model used across all Forge crates.
//! It contains no business logic and performs no I/O — only type definitions,
//! enums, and serialization derives.
//!
//! TS source: `packages/core/src/types/index.ts`

pub mod serde_util;
pub mod types;
pub mod util;

pub use types::*;
pub use util::{new_id, now_iso};
