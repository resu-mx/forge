//! Stores, cross-entity services, and IR compiler for Forge.
//!
//! This crate contains the business logic layer: the per-entity stores in [`db::stores`]
//! (which also enforce each entity's rules), the cross-entity services in [`services`], and
//! the resume intermediate-representation compiler.
//! Depends on `forge-core` for type definitions.

pub mod db;
pub mod forge;
pub mod services;

pub use forge::Forge;
