//! Source repository — data access for Source entities.
//!
//! TS implementation: packages/core/src/db/repositories/source-repository.ts
//! Storage: SQLite via bun:sqlite (TS) / rusqlite (Rust)
//!
//! Design notes:
//! - Sources use UUID primary keys (TEXT in SQLite)
//! - `status` is an enum: draft | approved | deriving
//! - `deriving` status acts as a lock during AI derivation
//! - FK to employers and projects (both optional)
//! - `source_content_snapshot` on derived bullets captures
//!   this source's description at derivation time

use crate::types::{Source, SourceFilter, CreateSource, UpdateSource};

pub struct SourceRepository {
    // In Rust: holds a rusqlite::Connection or pool reference
    // In TS: holds a bun:sqlite Database instance
}

impl SourceRepository {
    /// Create a new source. Generates UUID, sets status to 'draft'.
    pub fn create(&self, input: CreateSource) -> Result<Source, RepoError> {
        todo!()
    }

    /// Get source by ID. Returns None if not found.
    pub fn get(&self, id: &str) -> Result<Option<Source>, RepoError> {
        todo!()
    }

    /// List sources with optional filters and pagination.
    /// Filters: employer_id, project_id, status
    /// Sort: created_at DESC (default)
    pub fn list(&self, filter: SourceFilter, offset: u32, limit: u32) -> Result<(Vec<Source>, u32), RepoError> {
        // Returns (data, total_count) for pagination
        todo!()
    }

    /// Update source fields. Returns error if source not found.
    ///
    /// Note: If source has derived bullets and description changes,
    /// existing bullets' source_content_snapshot will diverge from
    /// current content — this is intentional and the chain view
    /// highlights the divergence.
    pub fn update(&self, id: &str, input: UpdateSource) -> Result<Source, RepoError> {
        todo!()
    }

    /// Delete source. Fails if source has any bullets (ON DELETE RESTRICT).
    pub fn delete(&self, id: &str) -> Result<(), RepoError> {
        todo!()
    }

    /// Atomically set status to 'deriving' if not already deriving.
    /// Returns the source if lock acquired, None if already locked.
    ///
    /// Implementation:
    ///   UPDATE sources SET status = 'deriving'
    ///   WHERE id = ? AND status != 'deriving'
    ///   RETURNING *;
    pub fn acquire_deriving_lock(&self, id: &str) -> Result<Option<Source>, RepoError> {
        todo!()
    }

    /// Release the deriving lock, resetting to the given status.
    /// Also sets last_derived_at if derivation succeeded.
    pub fn release_deriving_lock(&self, id: &str, restore_status: &str, derived: bool) -> Result<(), RepoError> {
        todo!()
    }
}
