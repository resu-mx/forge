//! Business logic for perspective reframings.
//!
//! Same status transition rules as BulletService:
//! - `draft` -> `in_review`
//! - `in_review` -> `approved` | `rejected`
//! - `rejected` -> `in_review` (reopen)
//! - `approved` -> `archived`
//! - `archived` -> `draft`
//!
//! Delete is blocked if the perspective is referenced by a resume entry
//! (`resume_entries.perspective_id` uses RESTRICT).
//!
//! `bullet_content_snapshot` is automatically populated at create time
//! by the `captureBulletSnapshotHook` wired into the entity map.
//!
//! `getPerspectiveWithChain` resolves the full derivation chain:
//! perspective -> bullet -> primary source (3-step fetch).
//!
//! Embedding on create is wired via `createEmbedHook(.., 'perspective')`.

use forge_core::{
    ForgeError, Framing, PaginationParams, Perspective, PerspectiveFilter, PerspectiveWithChain,
    UpdatePerspectiveInput,
};

/// Input for creating a perspective through the service layer.
pub struct CreatePerspectiveParams {
    pub bullet_id: String,
    pub content: String,
    pub target_archetype: Option<String>,
    pub domain: Option<String>,
    pub framing: Option<Framing>,
    /// When `true` (the default), the perspective is created with status
    /// `approved` so it can be immediately added to a resume. When `false`,
    /// the perspective starts as `draft`.
    pub auto_approve: Option<bool>,
}

/// Service layer for perspective CRUD, status transitions, and chain queries.
pub struct PerspectiveService;

impl PerspectiveService {
    /// Create a new `PerspectiveService`.
    pub fn new() -> Self {
        todo!()
    }

    // ── CRUD ────────────────────────────────────────────────────────

    /// Create a perspective directly from a bullet.
    ///
    /// Intended for filler bullets whose text is already resume-ready,
    /// bypassing the derivation flow.
    ///
    /// Validation:
    /// - `content` must be non-empty after trimming.
    /// - The referenced `bullet_id` must exist.
    ///
    /// When `auto_approve` is `true` (default), the perspective is created
    /// with status `approved`, `approved_at` set to now, and `approved_by`
    /// set to `"direct"`. Otherwise it starts as `draft`.
    ///
    /// `bullet_content_snapshot` is auto-populated by the entity lifecycle
    /// hook.
    pub fn create_perspective(
        &self,
        input: &CreatePerspectiveParams,
    ) -> Result<Perspective, ForgeError> {
        todo!()
    }

    /// Fetch a single perspective by id.
    ///
    /// Returns an error with code `NOT_FOUND` if the perspective does not
    /// exist.
    pub fn get_perspective(&self, id: &str) -> Result<Perspective, ForgeError> {
        todo!()
    }

    /// Fetch a perspective with its full derivation chain.
    ///
    /// Resolves: perspective -> bullet -> primary source.
    ///
    /// The primary source is found by walking `bullet_sources` for the row
    /// where `is_primary = true`. Returns an error with code `NOT_FOUND`
    /// if the chain is incomplete (missing bullet, or no primary source
    /// link).
    pub fn get_perspective_with_chain(&self, id: &str) -> Result<PerspectiveWithChain, ForgeError> {
        todo!()
    }

    /// List perspectives with filtering and pagination.
    ///
    /// Filters:
    /// - `bullet_id`: exact match on the parent bullet.
    /// - `target_archetype`: exact match.
    /// - `domain`: exact match.
    /// - `framing`: exact match on framing enum value.
    /// - `status`: exact match on perspective status.
    /// - `source_id`: matches perspectives whose bullet is linked to
    ///   the given source via `bullet_sources`.
    /// - `search`: case-insensitive substring match on `content`
    ///   (applied in-memory after the database query).
    ///
    /// When `search` is present, all matching rows are fetched first,
    /// filtered in-memory, then paginated. This ensures accurate totals.
    ///
    /// Results are ordered by `created_at DESC`.
    pub fn list_perspectives(
        &self,
        filter: &PerspectiveFilter,
        pagination: &PaginationParams,
    ) -> Result<(Vec<Perspective>, i64), ForgeError> {
        todo!()
    }

    /// Partially update a perspective's content, target_archetype,
    /// domain, and/or framing.
    ///
    /// Validation:
    /// - `content`, if provided, must be non-empty after trimming.
    ///
    /// If no fields are provided, the current perspective is returned
    /// unchanged.
    pub fn update_perspective(
        &self,
        id: &str,
        input: &UpdatePerspectiveInput,
    ) -> Result<Perspective, ForgeError> {
        todo!()
    }

    /// Delete a perspective by id.
    ///
    /// Returns an error with code `CONFLICT` and message "Cannot delete
    /// perspective that is in a resume" if the perspective is referenced
    /// by any resume entry (RESTRICT on `resume_entries.perspective_id`).
    pub fn delete_perspective(&self, id: &str) -> Result<(), ForgeError> {
        todo!()
    }

    // ── Status transitions ──────────────────────────────────────────

    /// Approve a perspective (transition: `in_review` -> `approved`).
    ///
    /// Sets `approved_at` to now, `approved_by` to `"human"`, and clears
    /// `rejection_reason`.
    pub fn approve_perspective(&self, id: &str) -> Result<Perspective, ForgeError> {
        todo!()
    }

    /// Reject a perspective (transition: `in_review` -> `rejected`).
    ///
    /// Validation:
    /// - `reason` must be non-empty after trimming.
    ///
    /// Sets `rejection_reason` on the perspective row. Leaves `approved_at`
    /// and `approved_by` untouched.
    pub fn reject_perspective(&self, id: &str, reason: &str) -> Result<Perspective, ForgeError> {
        todo!()
    }

    /// Reopen a rejected perspective for re-review (transition:
    /// `rejected` -> `in_review`).
    ///
    /// Clears `rejection_reason`. Leaves `approved_at` and `approved_by`
    /// untouched.
    pub fn reopen_perspective(&self, id: &str) -> Result<Perspective, ForgeError> {
        todo!()
    }
}
