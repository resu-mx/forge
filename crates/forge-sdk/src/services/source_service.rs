//! Business logic for source experience entries.
//!
//! Sources are polymorphic: the `sources` table stores common fields and
//! `source_type` discriminates among four extension tables (`source_roles`,
//! `source_projects`, `source_education`, `source_presentations`). A source
//! of type `general` has no extension row.
//!
//! Create and update coordinate writes across the base row and the matching
//! extension row atomically. `source_type` is immutable after creation.
//!
//! Embedding lifecycle is handled externally via entity-map hooks
//! (afterCreate on the `source` entity embeds the `description` field).

use forge_core::{
    CreateSource, ForgeError, PaginationParams, SourceFilter, SourceWithExtension, UpdateSource,
};

/// Service layer for source CRUD and business rules.
pub struct SourceService;

impl SourceService {
    /// Create a new `SourceService`.
    pub fn new() -> Self {
        todo!()
    }

    /// Create a new source with its type-specific extension data.
    ///
    /// Validation:
    /// - `title` must be non-empty after trimming.
    /// - `description` must be non-empty after trimming.
    /// - `source_type` defaults to `general` if not provided.
    ///
    /// The base row is inserted first, then the extension row (if the
    /// type has one). Returns the fully hydrated source with extension.
    pub fn create_source(&self, input: &CreateSource) -> Result<SourceWithExtension, ForgeError> {
        todo!()
    }

    /// Fetch a single source by id, hydrated with its extension data.
    ///
    /// Returns an error with code `NOT_FOUND` if the source does not exist.
    pub fn get_source(&self, id: &str) -> Result<SourceWithExtension, ForgeError> {
        todo!()
    }

    /// List sources with filtering and pagination.
    ///
    /// Filters:
    /// - `source_type`: exact match on the type discriminator.
    /// - `organization_id`: union match across `source_roles` and
    ///   `source_projects` extension tables.
    /// - `status`: exact match on source status.
    /// - `education_type`: match via `source_education` extension table.
    /// - `search`: case-insensitive substring match on title or description.
    ///
    /// When `search` is present, all matching rows are fetched first, then
    /// filtered in-memory, then paginated. This ensures accurate totals.
    ///
    /// Results are ordered by `created_at DESC`.
    pub fn list_sources(
        &self,
        filter: &SourceFilter,
        pagination: &PaginationParams,
    ) -> Result<(Vec<SourceWithExtension>, i64), ForgeError> {
        todo!()
    }

    /// Partially update a source and/or its extension data.
    ///
    /// Validation:
    /// - `title`, if provided, must be non-empty after trimming.
    /// - `description`, if provided, must be non-empty after trimming.
    ///
    /// The base row is updated first, then the extension row patch is
    /// applied based on the source's existing `source_type` (type is
    /// immutable post-create).
    pub fn update_source(
        &self,
        id: &str,
        input: &UpdateSource,
    ) -> Result<SourceWithExtension, ForgeError> {
        todo!()
    }

    /// Delete a source and its extension row.
    ///
    /// Extension rows and `bullet_sources` junction rows CASCADE on delete.
    /// Historically returned `CONFLICT` with message "Cannot delete source
    /// with existing bullets", but the current schema CASCADEs
    /// `bullet_sources` rather than RESTRICTing. The CONFLICT branch is
    /// preserved for any other FK constraint violations.
    pub fn delete_source(&self, id: &str) -> Result<(), ForgeError> {
        todo!()
    }
}
