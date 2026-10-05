//! Business logic for bullet points.
//!
//! Enforces status transition rules:
//! - `draft` -> `in_review`
//! - `in_review` -> `approved` | `rejected`
//! - `rejected` -> `in_review` (reopen)
//! - `approved` -> `archived`
//! - `archived` -> `draft`
//!
//! Multi-table writes:
//! - `bullet_sources` junction for source associations (composite PK).
//! - `bullet_skills` junction for technology tagging (composite PK).
//!
//! The `technologies` field on `Bullet` is a projection of linked skill
//! names (lowercased/trimmed), computed per read via hydration.
//! Creating or updating technologies does a find-or-create on the
//! `skills` table (case-insensitive) then inserts the junction row.
//!
//! Embedding lifecycle is handled externally via entity-map hooks
//! (afterCreate on the `bullet` entity).

use forge_core::{Bullet, BulletFilter, ForgeError, PaginationParams, UpdateBulletInput};

/// Input for creating a bullet through the service layer.
pub struct CreateBulletInput {
    pub content: String,
    pub source_content_snapshot: Option<String>,
    pub metrics: Option<String>,
    pub domain: Option<String>,
    pub technologies: Option<Vec<String>>,
    pub source_ids: Option<Vec<BulletSourceLink>>,
}

/// A source to link to a bullet at creation time.
pub struct BulletSourceLink {
    pub id: String,
    pub is_primary: Option<bool>,
}

/// Service layer for bullet CRUD, status transitions, and technology management.
pub struct BulletService;

impl BulletService {
    /// Create a new `BulletService`.
    pub fn new() -> Self {
        todo!()
    }

    // ── CRUD ────────────────────────────────────────────────────────

    /// Create a bullet manually (without AI derivation).
    ///
    /// Defaults to `draft` status so the user can review before submitting.
    ///
    /// Validation:
    /// - `content` must be non-empty after trimming.
    ///
    /// After inserting the base row:
    /// 1. Inserts `bullet_sources` junction rows for each `source_ids`
    ///    entry (default `is_primary = true`).
    /// 2. Inserts `bullet_skills` junction rows for each `technologies`
    ///    entry (find-or-create skill by name, case-insensitive).
    pub fn create_bullet(&self, input: &CreateBulletInput) -> Result<Bullet, ForgeError> {
        todo!()
    }

    /// Fetch a single bullet by id, hydrated with technologies.
    ///
    /// Returns an error with code `NOT_FOUND` if the bullet does not exist.
    pub fn get_bullet(&self, id: &str) -> Result<Bullet, ForgeError> {
        todo!()
    }

    /// List bullets with filtering and pagination.
    ///
    /// Filters:
    /// - `source_id`: matches via `bullet_sources` junction.
    /// - `status`: exact match on bullet status.
    /// - `technology`: case-insensitive match on linked skill name.
    /// - `domain`: exact match on the `domain` column.
    ///
    /// Results are ordered by `created_at DESC`.
    pub fn list_bullets(
        &self,
        filter: &BulletFilter,
        pagination: &PaginationParams,
    ) -> Result<(Vec<Bullet>, i64), ForgeError> {
        todo!()
    }

    /// Partially update a bullet's content, metrics, domain, and/or
    /// technologies.
    ///
    /// Validation:
    /// - `content`, if provided, must be non-empty after trimming.
    ///
    /// If `technologies` is provided, all existing technology links are
    /// replaced (delete-then-insert pattern).
    pub fn update_bullet(&self, id: &str, input: &UpdateBulletInput) -> Result<Bullet, ForgeError> {
        todo!()
    }

    /// Delete a bullet by id.
    ///
    /// Junction rows in `bullet_sources` and `bullet_skills` CASCADE.
    /// Returns an error with code `CONFLICT` and message "Cannot delete
    /// bullet with existing perspectives" if the bullet is referenced by
    /// any perspective row (RESTRICT on `perspectives.bullet_id`).
    pub fn delete_bullet(&self, id: &str) -> Result<(), ForgeError> {
        todo!()
    }

    // ── Status transitions ──────────────────────────────────────────

    /// Approve a bullet (transition: `in_review` -> `approved`).
    ///
    /// Sets `approved_at` to now and `approved_by` to `"human"`.
    pub fn approve_bullet(&self, id: &str) -> Result<Bullet, ForgeError> {
        todo!()
    }

    /// Reject a bullet (transition: `in_review` -> `rejected`).
    ///
    /// Validation:
    /// - `reason` must be non-empty after trimming.
    ///
    /// Sets `rejection_reason` on the bullet row.
    pub fn reject_bullet(&self, id: &str, reason: &str) -> Result<Bullet, ForgeError> {
        todo!()
    }

    /// Reopen a rejected bullet for re-review (transition: `rejected` -> `in_review`).
    pub fn reopen_bullet(&self, id: &str) -> Result<Bullet, ForgeError> {
        todo!()
    }

    /// Submit a draft bullet for review (transition: `draft` -> `in_review`).
    ///
    /// Only draft bullets can be submitted. Use `reopen_bullet` for
    /// rejected bullets.
    pub fn submit_bullet(&self, id: &str) -> Result<Bullet, ForgeError> {
        todo!()
    }
}
