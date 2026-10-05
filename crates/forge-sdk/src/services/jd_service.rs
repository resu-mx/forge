//! Business logic service for job descriptions.
//!
//! Validates input, delegates to `JdStore` for persistence, and hydrates
//! `JobDescriptionWithOrg` by joining organization names. Owns the
//! embedding lifecycle for JD requirement parsing.
//!
//! All method bodies are `todo!()` stubs.

use forge_core::{
    CreateJobDescription, ForgeError, JobDescriptionFilter, JobDescriptionWithOrg, Pagination,
    UpdateJobDescription,
};

/// Service layer for job description business logic.
pub struct JdService;

impl JdService {
    /// Create a new `JdService` instance.
    pub fn new() -> Self {
        todo!()
    }

    /// Create a new job description.
    ///
    /// Validates that title and raw_text are non-empty, status is valid,
    /// and salary_min <= salary_max. Returns the created JD with hydrated
    /// organization name. Fires an async embedding hook on success.
    pub fn create(&self, input: CreateJobDescription) -> Result<JobDescriptionWithOrg, ForgeError> {
        todo!()
    }

    /// Fetch a single job description by ID, hydrated with organization name.
    pub fn get(&self, id: &str) -> Result<JobDescriptionWithOrg, ForgeError> {
        todo!()
    }

    /// List job descriptions with optional status/organization filter and pagination.
    ///
    /// Results are ordered by `updated_at DESC`.
    pub fn list(
        &self,
        filter: Option<&JobDescriptionFilter>,
        offset: i64,
        limit: i64,
    ) -> Result<(Vec<JobDescriptionWithOrg>, Pagination), ForgeError> {
        todo!()
    }

    /// Partially update a job description.
    ///
    /// Validates non-empty title/raw_text when provided, valid status, and
    /// salary_min <= salary_max. Re-embeds requirements if raw_text changed.
    pub fn update(
        &self,
        id: &str,
        input: UpdateJobDescription,
    ) -> Result<JobDescriptionWithOrg, ForgeError> {
        todo!()
    }

    /// Delete a job description by ID.
    pub fn delete(&self, id: &str) -> Result<(), ForgeError> {
        todo!()
    }

    /// Look up a job description by its URL (exact match).
    ///
    /// Returns `NOT_FOUND` if no JD matches the given URL.
    pub fn lookup_by_url(&self, url: &str) -> Result<JobDescriptionWithOrg, ForgeError> {
        todo!()
    }
}
