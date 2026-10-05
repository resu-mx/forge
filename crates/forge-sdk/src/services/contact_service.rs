//! Business logic service for contacts.
//!
//! Validates input (name, email format, LinkedIn URL), delegates to
//! `ContactStore` for persistence, and manages three many-to-many junction
//! tables: `contact_organizations`, `contact_job_descriptions`, and
//! `contact_resumes`. Each junction uses a composite PK
//! `(contact_id, other_id, relationship)`.
//!
//! All method bodies are `todo!()` stubs.

use forge_core::{
    ContactFilter, ContactJDRelationship, ContactLink, ContactOrgRelationship,
    ContactResumeRelationship, ContactWithOrg, CreateContact, ForgeError, Pagination,
    UpdateContact,
};

/// Service layer for contact business logic.
pub struct ContactService;

impl ContactService {
    /// Create a new `ContactService` instance.
    pub fn new() -> Self {
        todo!()
    }

    // ── Core CRUD ───────────────────────────────────────────────────

    /// Create a new contact.
    ///
    /// Validates non-empty name, email format, and LinkedIn URL format.
    /// Returns the created contact with hydrated organization name.
    pub fn create(&self, input: CreateContact) -> Result<ContactWithOrg, ForgeError> {
        todo!()
    }

    /// Fetch a single contact by ID, hydrated with organization name.
    pub fn get(&self, id: &str) -> Result<ContactWithOrg, ForgeError> {
        todo!()
    }

    /// List contacts with optional organization/search filter and pagination.
    ///
    /// Search matches across name, title, and email (case-insensitive).
    pub fn list(
        &self,
        filter: Option<&ContactFilter>,
        offset: i64,
        limit: i64,
    ) -> Result<(Vec<ContactWithOrg>, Pagination), ForgeError> {
        todo!()
    }

    /// Partially update a contact.
    ///
    /// Validates non-empty name, email format, and LinkedIn URL when provided.
    pub fn update(&self, id: &str, input: UpdateContact) -> Result<ContactWithOrg, ForgeError> {
        todo!()
    }

    /// Delete a contact by ID.
    pub fn delete(&self, id: &str) -> Result<(), ForgeError> {
        todo!()
    }

    // ── Organization relationships ──────────────────────────────────

    /// Link a contact to an organization with a typed relationship.
    ///
    /// Validates the relationship type. Idempotent: existing links are a no-op.
    pub fn link_organization(
        &self,
        contact_id: &str,
        org_id: &str,
        relationship: &str,
    ) -> Result<(), ForgeError> {
        todo!()
    }

    /// Remove a contact-organization link.
    pub fn unlink_organization(
        &self,
        contact_id: &str,
        org_id: &str,
        relationship: &str,
    ) -> Result<(), ForgeError> {
        todo!()
    }

    /// List organizations linked to a contact, with IDs, names, and relationships.
    pub fn list_organizations(&self, contact_id: &str) -> Result<Vec<OrgRelation>, ForgeError> {
        todo!()
    }

    // ── Job description relationships ───────────────────────────────

    /// Link a contact to a job description with a typed relationship.
    ///
    /// Validates the relationship type. Idempotent.
    pub fn link_job_description(
        &self,
        contact_id: &str,
        jd_id: &str,
        relationship: &str,
    ) -> Result<(), ForgeError> {
        todo!()
    }

    /// Remove a contact-job description link.
    pub fn unlink_job_description(
        &self,
        contact_id: &str,
        jd_id: &str,
        relationship: &str,
    ) -> Result<(), ForgeError> {
        todo!()
    }

    /// List job descriptions linked to a contact, with titles and org names.
    pub fn list_job_descriptions(&self, contact_id: &str) -> Result<Vec<JdRelation>, ForgeError> {
        todo!()
    }

    // ── Resume relationships ────────────────────────────────────────

    /// Link a contact to a resume with a typed relationship.
    ///
    /// Validates the relationship type. Idempotent.
    pub fn link_resume(
        &self,
        contact_id: &str,
        resume_id: &str,
        relationship: &str,
    ) -> Result<(), ForgeError> {
        todo!()
    }

    /// Remove a contact-resume link.
    pub fn unlink_resume(
        &self,
        contact_id: &str,
        resume_id: &str,
        relationship: &str,
    ) -> Result<(), ForgeError> {
        todo!()
    }

    /// List resumes linked to a contact, with IDs, names, and relationships.
    pub fn list_resumes(&self, contact_id: &str) -> Result<Vec<ResumeRelation>, ForgeError> {
        todo!()
    }

    // ── Reverse lookups ─────────────────────────────────────────────

    /// List contacts linked to a given organization (reverse lookup).
    pub fn list_by_organization(&self, org_id: &str) -> Result<Vec<ContactLink>, ForgeError> {
        todo!()
    }

    /// List contacts linked to a given job description (reverse lookup).
    pub fn list_by_job_description(&self, jd_id: &str) -> Result<Vec<ContactLink>, ForgeError> {
        todo!()
    }

    /// List contacts linked to a given resume (reverse lookup).
    pub fn list_by_resume(&self, resume_id: &str) -> Result<Vec<ContactLink>, ForgeError> {
        todo!()
    }
}

// ── Relation helper structs ─────────────────────────────────────────

/// An organization linked to a contact with a typed relationship.
pub struct OrgRelation {
    pub id: String,
    pub name: String,
    pub relationship: ContactOrgRelationship,
}

/// A job description linked to a contact with a typed relationship.
pub struct JdRelation {
    pub id: String,
    pub title: String,
    pub organization_name: Option<String>,
    pub relationship: ContactJDRelationship,
}

/// A resume linked to a contact with a typed relationship.
pub struct ResumeRelation {
    pub id: String,
    pub name: String,
    pub relationship: ContactResumeRelationship,
}
