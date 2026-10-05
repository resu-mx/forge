//! Skill service — business logic for skill entities and the `skill_domains`
//! junction.
//!
//! Skills have a structured `category` enum validated both at the service
//! layer (for friendly error messages) and at the DB layer (FK to
//! `skill_categories.slug`).
//!
//! Skill names are capitalized on the first character while preserving the
//! rest (e.g. "SAFe" stays "SAFe", "typescript" becomes "Typescript").

use forge_core::{Domain, ForgeError, Skill, SkillCategory, SkillWithDomains};

/// Filter options for listing skills.
#[derive(Debug, Clone, Default)]
pub struct SkillFilter {
    pub category: Option<SkillCategory>,
    pub domain_id: Option<String>,
    pub search: Option<String>,
}

/// Business logic for skill management and domain linkage.
pub struct SkillService;

impl SkillService {
    /// Create a new service instance.
    pub fn new() -> Self {
        todo!()
    }

    // ── Skill CRUD ──────────────────────────────────────────────────

    /// Create a new skill.
    ///
    /// Validates that `name` is non-empty and `category` (if provided) is
    /// a valid `SkillCategory` enum value. The name is capitalized on the
    /// first character. Defaults `category` to `other` when omitted.
    pub fn create(&self, name: &str, category: Option<SkillCategory>) -> Result<Skill, ForgeError> {
        todo!()
    }

    /// Fetch a single skill by ID.
    pub fn get(&self, id: &str) -> Result<Skill, ForgeError> {
        todo!()
    }

    /// Fetch a skill with its linked domains populated.
    pub fn get_with_domains(&self, id: &str) -> Result<SkillWithDomains, ForgeError> {
        todo!()
    }

    /// List skills with optional filters.
    ///
    /// - `category`: exact match on the `SkillCategory` enum
    /// - `domain_id`: walks the `skill_domains` junction to find skills
    ///   linked to a domain
    /// - `search`: case-insensitive substring match on `name`
    ///
    /// Results are sorted by `name ASC`.
    pub fn list(&self, filter: Option<SkillFilter>) -> Result<Vec<Skill>, ForgeError> {
        todo!()
    }

    /// Partially update a skill's `name` and/or `category`.
    ///
    /// Validates that `name` (if provided) is non-empty, and `category`
    /// (if provided) is a valid enum value. Name is capitalized on the
    /// first character.
    pub fn update(
        &self,
        id: &str,
        name: Option<&str>,
        category: Option<SkillCategory>,
    ) -> Result<Skill, ForgeError> {
        todo!()
    }

    /// Delete a skill by ID. Cascades through all junction tables
    /// (`bullet_skills`, `resume_skills`, `certification_skills`,
    /// `job_description_skills`, `perspective_skills`, `source_skills`,
    /// `skill_domains`, `summary_skills`).
    pub fn delete(&self, id: &str) -> Result<(), ForgeError> {
        todo!()
    }

    // ── Get or Create ───────────────────────────────────────────────

    /// Find a skill by name (case-insensitive), creating it if it does
    /// not exist. Supports the combobox "select existing or create new"
    /// UI pattern.
    ///
    /// If `category` is provided and invalid, returns `VALIDATION_ERROR`.
    pub fn get_or_create(
        &self,
        name: &str,
        category: Option<SkillCategory>,
    ) -> Result<Skill, ForgeError> {
        todo!()
    }

    // ── Skill <-> Domain junction ───────────────────────────────────

    /// Link a skill to a domain. Idempotent — duplicate pairs are a no-op.
    ///
    /// Returns `NOT_FOUND` if the skill or domain does not exist.
    pub fn add_domain(&self, skill_id: &str, domain_id: &str) -> Result<(), ForgeError> {
        todo!()
    }

    /// Unlink a skill from a domain. Silent if the pair does not exist.
    pub fn remove_domain(&self, skill_id: &str, domain_id: &str) -> Result<(), ForgeError> {
        todo!()
    }

    /// Get all domains linked to a skill.
    ///
    /// Returns `NOT_FOUND` if the skill does not exist.
    pub fn get_domains(&self, skill_id: &str) -> Result<Vec<Domain>, ForgeError> {
        todo!()
    }

    // ── Merge ───────────────────────────────────────────────────────

    /// Atomically merge `source_id` into `target_id`.
    ///
    /// Re-points all junction rows from the source skill to the target,
    /// resolves UNIQUE constraint conflicts by deleting conflicting source
    /// rows, then deletes the source skill.
    ///
    /// Returns `VALIDATION_ERROR` if `source_id == target_id`.
    /// Returns `NOT_FOUND` if either skill does not exist.
    ///
    /// Junction tables migrated: `bullet_skills`, `resume_skills`,
    /// `certification_skills`, `job_description_skills`, `perspective_skills`,
    /// `source_skills`, `skill_domains`, `summary_skills`.
    pub fn merge(&self, source_id: &str, target_id: &str) -> Result<Skill, ForgeError> {
        todo!()
    }
}
