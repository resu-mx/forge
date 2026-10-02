//! Create/Update input types for all entities.
//!
//! TS source: `packages/core/src/types/index.ts` (input types section)

use serde::{Deserialize, Serialize};

use super::enums::*;

// ── Job Description ──────────────────────────────────────────────────

/// Input for creating a new JobDescription.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateJobDescription {
    pub title: String,
    pub organization_id: Option<String>,
    pub url: Option<String>,
    pub raw_text: String,
    pub status: Option<JobDescriptionStatus>,
    pub salary_range: Option<String>,
    pub salary_min: Option<f64>,
    pub salary_max: Option<f64>,
    pub location: Option<String>,
    pub parsed_sections: Option<String>,
    pub work_posture: Option<String>,
    pub parsed_locations: Option<String>,
    pub salary_period: Option<String>,
}

/// Input for partially updating a JobDescription.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateJobDescription {
    pub title: Option<String>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub url: Option<Option<String>>,
    pub raw_text: Option<String>,
    pub status: Option<JobDescriptionStatus>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub salary_range: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub salary_min: Option<Option<f64>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub salary_max: Option<Option<f64>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub location: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub parsed_sections: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub work_posture: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub parsed_locations: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub salary_period: Option<Option<String>>,
}

// ── Contact ──────────────────────────────────────────────────────────

/// Input for creating a new Contact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateContact {
    pub name: String,
    pub title: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub linkedin: Option<String>,
    pub team: Option<String>,
    pub dept: Option<String>,
    pub notes: Option<String>,
    pub organization_id: Option<String>,
}

/// Input for partially updating a Contact.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateContact {
    pub name: Option<String>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub title: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub email: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub phone: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub linkedin: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub team: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub dept: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub notes: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<Option<String>>,
}

// ── Source ────────────────────────────────────────────────────────────

/// Input for creating a new Source (with optional extension fields).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateSource {
    pub title: String,
    pub description: String,
    pub source_type: Option<SourceType>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    // Role extension
    pub organization_id: Option<String>,
    pub is_current: Option<i32>,
    pub work_arrangement: Option<String>,
    pub base_salary: Option<f64>,
    pub total_comp_notes: Option<String>,
    // Project extension
    pub is_personal: Option<i32>,
    pub open_source: Option<i32>,
    pub url: Option<String>,
    // Education extension
    pub education_type: Option<EducationType>,
    pub education_organization_id: Option<String>,
    pub campus_id: Option<String>,
    pub field: Option<String>,
    pub is_in_progress: Option<i32>,
    pub credential_id: Option<String>,
    pub expiration_date: Option<String>,
    pub degree_level: Option<DegreeLevelType>,
    pub degree_type: Option<String>,
    pub certificate_subtype: Option<CertificateSubtype>,
    pub gpa: Option<String>,
    pub location: Option<String>,
    pub edu_description: Option<String>,
    // Presentation extension
    pub venue: Option<String>,
    pub presentation_type: Option<PresentationType>,
    pub coauthors: Option<String>,
}

/// Input for partially updating a Source.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateSource {
    pub title: Option<String>,
    pub description: Option<String>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub start_date: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub end_date: Option<Option<String>>,
    // Role extension
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<Option<String>>,
    pub is_current: Option<i32>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub work_arrangement: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub base_salary: Option<Option<f64>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub total_comp_notes: Option<Option<String>>,
    // Project extension
    pub is_personal: Option<i32>,
    pub open_source: Option<i32>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub url: Option<Option<String>>,
    // Education extension
    pub education_type: Option<EducationType>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub education_organization_id: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub campus_id: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub field: Option<Option<String>>,
    pub is_in_progress: Option<i32>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub credential_id: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub expiration_date: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub degree_level: Option<Option<DegreeLevelType>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub degree_type: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub certificate_subtype: Option<Option<CertificateSubtype>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub gpa: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub location: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub edu_description: Option<Option<String>>,
    // Presentation extension
    pub venue: Option<String>,
    pub presentation_type: Option<PresentationType>,
    pub coauthors: Option<String>,
}

// ── Bullet ───────────────────────────────────────────────────────────

/// Input for partially updating a Bullet.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateBulletInput {
    pub content: Option<String>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub metrics: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub domain: Option<Option<String>>,
    pub technologies: Option<Vec<String>>,
}

// ── Perspective ──────────────────────────────────────────────────────

/// Input for deriving a perspective from a bullet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DerivePerspectiveInput {
    pub archetype: String,
    pub domain: String,
    pub framing: Framing,
}

/// Input for creating a new Perspective.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePerspectiveInput {
    pub bullet_id: String,
    pub content: String,
    pub bullet_content_snapshot: String,
    pub target_archetype: String,
    pub domain: String,
    pub framing: Framing,
    pub status: Option<PerspectiveStatus>,
    pub prompt_log_id: Option<String>,
}

/// Input for partially updating a Perspective.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdatePerspectiveInput {
    pub content: Option<String>,
    pub target_archetype: Option<String>,
    pub domain: Option<String>,
    pub framing: Option<Framing>,
}

// ── Resume ───────────────────────────────────────────────────────────

/// Input for creating a new Resume.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateResume {
    pub name: String,
    pub target_role: String,
    pub target_employer: String,
    pub archetype: String,
    pub summary_id: Option<String>,
}

/// Input for partially updating a Resume.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateResume {
    pub name: Option<String>,
    pub target_role: Option<String>,
    pub target_employer: Option<String>,
    pub archetype: Option<String>,
    pub status: Option<ResumeStatus>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub header: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub summary_id: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub summary_override: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub markdown_override: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub latex_override: Option<Option<String>>,
    pub show_clearance_in_header: Option<i32>,
}

/// Input for adding a perspective to a resume.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddResumePerspective {
    pub perspective_id: String,
    pub section: String,
    pub position: i32,
}

/// Input for adding a resume entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddResumeEntry {
    pub section_id: String,
    pub perspective_id: Option<String>,
    pub source_id: Option<String>,
    pub position: Option<i32>,
    pub content: Option<String>,
}

/// Input for reordering perspectives in a resume.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReorderPerspectives {
    pub perspectives: Vec<ReorderPerspectiveItem>,
}

/// A single item in a perspective reorder request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReorderPerspectiveItem {
    pub perspective_id: String,
    pub section: String,
    pub position: i32,
}

/// Input for rejecting a bullet or perspective.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RejectInput {
    pub rejection_reason: String,
}

// ── Certification ────────────────────────────────────────────────────

/// Input for creating a certification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCertification {
    pub short_name: String,
    pub long_name: String,
    pub cert_id: Option<String>,
    pub issuer_id: Option<String>,
    pub date_earned: Option<String>,
    pub expiry_date: Option<String>,
    pub credential_id: Option<String>,
    pub credential_url: Option<String>,
    pub credly_url: Option<String>,
    pub in_progress: Option<bool>,
}

/// Input for partially updating a certification.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateCertification {
    pub short_name: Option<String>,
    pub long_name: Option<String>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub cert_id: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub issuer_id: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub date_earned: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub expiry_date: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub credential_id: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub credential_url: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub credly_url: Option<Option<String>>,
    pub in_progress: Option<bool>,
}

/// Input for adding a certification to a resume.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddResumeCertification {
    pub certification_id: String,
    pub section_id: String,
    pub position: Option<i32>,
}

// ── Template ─────────────────────────────────────────────────────────

/// Input for creating a resume template.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateResumeTemplate {
    pub name: String,
    pub description: Option<String>,
    pub sections: Vec<super::entities::TemplateSectionDef>,
}

/// Input for updating a resume template.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateResumeTemplate {
    pub name: Option<String>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub description: Option<Option<String>>,
    pub sections: Option<Vec<super::entities::TemplateSectionDef>>,
}

// ── Organization ─────────────────────────────────────────────────────

/// Input for creating an Organization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOrganizationInput {
    pub name: String,
    pub org_type: Option<String>,
    pub tags: Option<Vec<String>>,
    pub industry: Option<String>,
    pub size: Option<String>,
    pub worked: Option<i32>,
    pub employment_type: Option<String>,
    pub website: Option<String>,
    pub linkedin_url: Option<String>,
    pub glassdoor_url: Option<String>,
    pub glassdoor_rating: Option<f64>,
    pub status: Option<OrganizationStatus>,
}

// ── Archetype ────────────────────────────────────────────────────────

/// Input for creating an Archetype.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateArchetypeInput {
    pub name: String,
    pub description: Option<String>,
}

/// Input for partially updating an Archetype.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateArchetypeInput {
    pub name: Option<String>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub description: Option<Option<String>>,
}

// ── Org Location ────────────────────────────────────────────────────

/// Input for creating an OrgLocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOrgLocation {
    pub organization_id: String,
    pub name: String,
    pub modality: Option<LocationModality>,
    pub address_id: Option<String>,
    pub is_headquarters: Option<bool>,
}

/// Input for partially updating an OrgLocation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateOrgLocation {
    pub name: Option<String>,
    pub modality: Option<LocationModality>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub address_id: Option<Option<String>>,
    pub is_headquarters: Option<bool>,
}

/// Input for creating an OrgAlias.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOrgAlias {
    pub alias: String,
}

// ── Summary ──────────────────────────────────────────────────────────

/// Input for creating a new Summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSummary {
    pub title: String,
    pub role: Option<String>,
    pub description: Option<String>,
    pub is_template: Option<i32>,
    pub industry_id: Option<String>,
    pub role_type_id: Option<String>,
    pub notes: Option<String>,
}

/// Input for partially updating a Summary.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateSummary {
    pub title: Option<String>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub role: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub description: Option<Option<String>>,
    pub is_template: Option<i32>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub industry_id: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub role_type_id: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub notes: Option<Option<String>>,
}

// ── Address ──────────────────────────────────────────────────────────

/// Input for creating an Address.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAddress {
    pub name: String,
    pub street_1: Option<String>,
    pub street_2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip: Option<String>,
    pub country_code: Option<String>,
}

/// Input for updating an Address.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateAddress {
    pub name: Option<String>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub street_1: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub street_2: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub city: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub state: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub zip: Option<Option<String>>,
    pub country_code: Option<String>,
}

// ── Profile ──────────────────────────────────────────────────────────

/// Input for partially updating the user profile.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateProfile {
    pub name: Option<String>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub email: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub phone: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub address_id: Option<Option<String>>,
    pub address: Option<CreateAddress>,
    pub urls: Option<Vec<ProfileUrlInput>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub salary_minimum: Option<Option<f64>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub salary_target: Option<Option<f64>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub salary_stretch: Option<Option<f64>>,
}

/// A URL entry in a profile update request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileUrlInput {
    pub key: String,
    pub url: String,
}

// ── Domain / Industry / RoleType ─────────────────────────────────────

/// Input for creating a Domain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDomainInput {
    pub name: String,
    pub description: Option<String>,
}

/// Input for creating an Industry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateIndustryInput {
    pub name: String,
    pub description: Option<String>,
}

/// Input for creating a RoleType.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRoleTypeInput {
    pub name: String,
    pub description: Option<String>,
}

// ── Credential ──────────────────────────────────────────────────────

/// Input for creating a Credential.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCredential {
    pub credential_type: CredentialType,
    pub label: String,
    pub status: Option<CredentialStatus>,
    pub organization_id: Option<String>,
    pub details: Option<String>,
    pub issued_date: Option<String>,
    pub expiry_date: Option<String>,
}

/// Input for partially updating a Credential.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateCredential {
    pub label: Option<String>,
    pub status: Option<CredentialStatus>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<Option<String>>,
    pub details: Option<String>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub issued_date: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::serde_util::double_option", skip_serializing_if = "Option::is_none")]
    pub expiry_date: Option<Option<String>>,
}

// ── Answer Bank ─────────────────────────────────────────────────────

/// Input for upserting an AnswerBankEntry (keyed by field_kind).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertAnswerInput {
    pub field_kind: String,
    pub label: String,
    pub value: String,
}

// ── Derivation ───────────────────────────────────────────────────────

/// Input for creating a PendingDerivation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePendingDerivationInput {
    pub entity_type: String,
    pub entity_id: String,
    pub client_id: String,
    pub prompt: String,
    pub snapshot: String,
    pub derivation_params: Option<String>,
    pub expires_at: String,
}

/// Input for upserting an embedding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertEmbeddingInput {
    pub entity_type: EmbeddingEntityType,
    pub entity_id: String,
    pub content_hash: String,
    pub vector: Vec<f32>,
}
