//! Core entity structs — database row shapes.
//!
//! TS source: `packages/core/src/types/index.ts` (core entities section)

use serde::{Deserialize, Serialize};

use super::enums::*;

// ── Organization & Location ──────────────────────────────────────────

/// A named location belonging to an organization (office, campus, remote hub).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgLocation {
    pub id: String,
    pub organization_id: String,
    pub name: String,
    pub modality: LocationModality,
    pub address_id: Option<String>,
    pub is_headquarters: i32,
    pub created_at: String,
}

/// An alias/shorthand name for an organization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgAlias {
    pub id: String,
    pub organization_id: String,
    pub alias: String,
}

/// An organization (employer, school, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: String,
    pub name: String,
    pub org_type: String,
    pub tags: Vec<OrgTag>,
    pub industry: Option<String>,
    pub size: Option<String>,
    pub worked: i32,
    pub employment_type: Option<String>,
    pub website: Option<String>,
    pub linkedin_url: Option<String>,
    pub glassdoor_url: Option<String>,
    pub glassdoor_rating: Option<f64>,
    pub status: Option<OrganizationStatus>,
    pub created_at: String,
    pub updated_at: String,
}

// ── Job Description ──────────────────────────────────────────────────

/// A stored job description for a target opportunity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobDescription {
    pub id: String,
    pub organization_id: Option<String>,
    pub title: String,
    pub url: Option<String>,
    pub raw_text: String,
    pub status: JobDescriptionStatus,
    pub salary_range: Option<String>,
    pub salary_min: Option<f64>,
    pub salary_max: Option<f64>,
    pub location: Option<String>,
    pub parsed_sections: Option<String>,
    pub work_posture: Option<String>,
    pub parsed_locations: Option<String>,
    pub salary_period: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// JobDescription with computed organization_name from JOIN.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobDescriptionWithOrg {
    #[serde(flatten)]
    pub base: JobDescription,
    pub organization_name: Option<String>,
}

/// A resume linked to a JD, with display fields JOINed from the resumes table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumeLink {
    pub resume_id: String,
    pub resume_name: String,
    pub target_role: String,
    pub target_employer: String,
    pub archetype: String,
    pub status: ResumeStatus,
    pub created_at: String,
    pub resume_created_at: String,
}

/// A JD linked to a resume, with display fields JOINed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JDLink {
    pub job_description_id: String,
    pub title: String,
    pub organization_name: Option<String>,
    pub status: JobDescriptionStatus,
    pub location: Option<String>,
    pub salary_range: Option<String>,
    pub created_at: String,
    pub jd_created_at: String,
}

// ── Contact ──────────────────────────────────────────────────────────

/// A contact person tracked in the job hunting process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub id: String,
    pub name: String,
    pub title: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub linkedin: Option<String>,
    pub team: Option<String>,
    pub dept: Option<String>,
    pub notes: Option<String>,
    pub organization_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Contact with computed organization_name from JOIN.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactWithOrg {
    #[serde(flatten)]
    pub base: Contact,
    pub organization_name: Option<String>,
}

/// A contact linked to an entity with a typed relationship.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactLink {
    pub contact_id: String,
    pub contact_name: String,
    pub contact_title: Option<String>,
    pub contact_email: Option<String>,
    pub relationship: String,
}

// ── Source & Extensions ──────────────────────────────────────────────

/// A source experience entry — the root of the derivation chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub id: String,
    pub title: String,
    pub description: String,
    pub source_type: SourceType,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub status: SourceStatus,
    pub updated_by: UpdatedBy,
    pub last_derived_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Role-specific details for a source with source_type='role'.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceRole {
    pub source_id: String,
    pub organization_id: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub is_current: i32,
    pub work_arrangement: Option<String>,
    pub base_salary: Option<f64>,
    pub total_comp_notes: Option<String>,
}

/// Project-specific details for a source with source_type='project'.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceProject {
    pub source_id: String,
    pub organization_id: Option<String>,
    pub is_personal: i32,
    pub open_source: i32,
    pub url: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

/// Presentation-specific details for a source with source_type='presentation'.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourcePresentation {
    pub source_id: String,
    pub venue: Option<String>,
    pub presentation_type: PresentationType,
    pub url: Option<String>,
    pub coauthors: Option<String>,
}

/// Education-specific details for a source with source_type='education'.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceEducation {
    pub source_id: String,
    pub education_type: EducationType,
    pub organization_id: Option<String>,
    pub campus_id: Option<String>,
    pub edu_description: Option<String>,
    pub location: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub url: Option<String>,
    pub degree_level: Option<DegreeLevelType>,
    pub degree_type: Option<String>,
    pub field: Option<String>,
    pub gpa: Option<String>,
    pub is_in_progress: i32,
    pub certificate_subtype: Option<CertificateSubtype>,
    pub credential_id: Option<String>,
    pub expiration_date: Option<String>,
}

/// A source with its polymorphic extension data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceWithExtension {
    #[serde(flatten)]
    pub base: Source,
    pub extension: Option<SourceExtension>,
}

/// Polymorphic extension for source sub-types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SourceExtension {
    Role(SourceRole),
    Project(SourceProject),
    Education(SourceEducation),
    Presentation(SourcePresentation),
}

// ── Bullet ───────────────────────────────────────────────────────────

/// A bullet point derived from a source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bullet {
    pub id: String,
    pub content: String,
    pub source_content_snapshot: String,
    pub technologies: Vec<String>,
    pub metrics: Option<String>,
    pub status: BulletStatus,
    pub rejection_reason: Option<String>,
    pub prompt_log_id: Option<String>,
    pub approved_at: Option<String>,
    pub approved_by: Option<String>,
    pub domain: Option<String>,
    pub created_at: String,
}

/// Junction table linking bullets to sources.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulletSource {
    pub bullet_id: String,
    pub source_id: String,
    pub is_primary: i32,
}

/// A bullet with its perspective count.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulletWithRelations {
    #[serde(flatten)]
    pub base: Bullet,
    pub perspective_count: i64,
}

// ── Perspective ──────────────────────────────────────────────────────

/// A perspective reframing of a bullet for a specific archetype/domain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Perspective {
    pub id: String,
    pub bullet_id: String,
    pub content: String,
    pub bullet_content_snapshot: String,
    pub target_archetype: Option<String>,
    pub domain: Option<String>,
    pub framing: Framing,
    pub status: PerspectiveStatus,
    pub rejection_reason: Option<String>,
    pub prompt_log_id: Option<String>,
    pub approved_at: Option<String>,
    pub approved_by: Option<String>,
    pub created_at: String,
}

/// A perspective with its full derivation chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerspectiveWithChain {
    #[serde(flatten)]
    pub base: Perspective,
    pub bullet: Bullet,
    pub source: Source,
}

// ── Resume ───────────────────────────────────────────────────────────

/// A curated resume assembled from perspectives.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resume {
    pub id: String,
    pub name: String,
    pub target_role: String,
    pub target_employer: String,
    pub archetype: String,
    pub status: ResumeStatus,
    pub header: Option<String>,
    pub summary_id: Option<String>,
    pub generated_tagline: Option<String>,
    pub tagline_override: Option<String>,
    pub markdown_override: Option<String>,
    pub markdown_override_updated_at: Option<String>,
    pub latex_override: Option<String>,
    pub latex_override_updated_at: Option<String>,
    pub summary_override: Option<String>,
    pub summary_override_updated_at: Option<String>,
    pub show_clearance_in_header: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// A resume section entity — first-class section with user-defined title.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumeSectionEntity {
    pub id: String,
    pub resume_id: String,
    pub title: String,
    pub entry_type: String,
    pub position: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// A skill pinned to a resume section (skills-type sections).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumeSkill {
    pub id: String,
    pub section_id: String,
    pub skill_id: String,
    pub position: i32,
    pub created_at: String,
}

/// A resume entry linking a perspective to a resume section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumeEntry {
    pub id: String,
    pub resume_id: String,
    pub section_id: String,
    pub perspective_id: Option<String>,
    pub source_id: Option<String>,
    pub content: Option<String>,
    pub perspective_content_snapshot: Option<String>,
    pub position: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// A join-table entry linking a perspective to a resume section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumePerspective {
    pub resume_id: String,
    pub perspective_id: String,
    pub section: ResumeSection,
    pub position: i32,
}

/// A resume with entries grouped by section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumeWithEntries {
    #[serde(flatten)]
    pub base: Resume,
    pub sections: Vec<ResumeWithEntriesSection>,
}

/// A section within a ResumeWithEntries response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumeWithEntriesSection {
    pub id: String,
    pub title: String,
    pub entry_type: String,
    pub position: i32,
    pub entries: Vec<ResumeEntryWithContent>,
}

/// A resume entry with resolved perspective content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumeEntryWithContent {
    #[serde(flatten)]
    pub base: ResumeEntry,
    pub perspective_content: Option<String>,
}

// ── Certification ────────────────────────────────────────────────────

/// A certification entity — an earned credential that validates skills.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Certification {
    pub id: String,
    pub short_name: String,
    pub long_name: String,
    pub cert_id: Option<String>,
    pub issuer_id: Option<String>,
    pub date_earned: Option<String>,
    pub expiry_date: Option<String>,
    pub credential_id: Option<String>,
    pub credential_url: Option<String>,
    pub credly_url: Option<String>,
    pub in_progress: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// A certification with its linked skills populated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificationWithSkills {
    #[serde(flatten)]
    pub base: Certification,
    pub skills: Vec<Skill>,
}

/// A certification pinned to a resume.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumeCertification {
    pub id: String,
    pub resume_id: String,
    pub certification_id: String,
    pub section_id: String,
    pub position: i32,
    pub created_at: String,
}

// ── Template ─────────────────────────────────────────────────────────

/// A resume template — defines reusable section layouts without content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumeTemplate {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub sections: Vec<TemplateSectionDef>,
    pub is_builtin: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// A section definition within a template.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateSectionDef {
    pub title: String,
    pub entry_type: String,
    pub position: i32,
}

// ── Skill ────────────────────────────────────────────────────────────

/// A named skill with structured category.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub category: SkillCategory,
}

/// A skill with its linked domains.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillWithDomains {
    #[serde(flatten)]
    pub base: Skill,
    pub domains: Vec<Domain>,
}

/// A full `skills` row, as the TS junction routes return it (`SELECT s.*`).
/// `Skill` leaves out `created_at`. Serialises to exactly
/// `{id, name, category, created_at}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRow {
    #[serde(flatten)]
    pub base: Skill,
    pub created_at: String,
}

/// A skill extracted from a JD by AI, pending human review.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedSkill {
    pub name: String,
    pub category: String,
    pub confidence: f64,
}

/// Result of AI skill extraction from a JD.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillExtractionResult {
    pub skills: Vec<ExtractedSkill>,
    pub warnings: Vec<String>,
}

// ── Prompt Log ───────────────────────────────────────────────────────

/// An append-only log entry for AI prompt/response pairs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptLog {
    pub id: String,
    pub entity_type: PromptLogEntityType,
    pub entity_id: String,
    pub prompt_template: String,
    pub prompt_input: String,
    pub raw_response: String,
    pub created_at: String,
}

// ── Note ─────────────────────────────────────────────────────────────

/// A user-created note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserNote {
    pub id: String,
    pub title: Option<String>,
    pub content: String,
    pub created_at: String,
    pub updated_at: String,
}

/// A reference linking a note to an entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteReference {
    pub note_id: String,
    pub entity_type: NoteReferenceEntityType,
    pub entity_id: String,
}

/// A UserNote with its NoteReference array.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserNoteWithReferences {
    #[serde(flatten)]
    pub base: UserNote,
    pub references: Vec<NoteReference>,
}

// ── Domain, Industry, RoleType, Archetype ────────────────────────────

/// An editable experience domain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Domain {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
}

/// Domain with perspective and archetype counts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainWithUsage {
    #[serde(flatten)]
    pub base: Domain,
    pub perspective_count: i64,
    pub archetype_count: i64,
}

/// An editable industry (e.g. fintech, healthcare, defense).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Industry {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
}

/// An editable role type (e.g. IC, lead, architect, manager).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleType {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
}

/// An editable resume archetype.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Archetype {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
}

/// Archetype with its linked Domain array.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchetypeWithDomains {
    #[serde(flatten)]
    pub base: Archetype,
    pub domains: Vec<Domain>,
}

/// Archetype with aggregated counts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchetypeWithCounts {
    #[serde(flatten)]
    pub base: Archetype,
    pub resume_count: i64,
    pub perspective_count: i64,
    pub domain_count: i64,
}

/// Junction linking an archetype to an expected domain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchetypeDomain {
    pub archetype_id: String,
    pub domain_id: String,
    pub created_at: String,
}

// ── Summary ──────────────────────────────────────────────────────────

/// A reusable professional summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    pub id: String,
    pub title: String,
    pub role: Option<String>,
    pub description: Option<String>,
    pub is_template: i32,
    pub industry_id: Option<String>,
    pub role_type_id: Option<String>,
    pub linked_resume_count: i64,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// A Summary plus its linked industry, role_type, and keyword skills.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryWithRelations {
    #[serde(flatten)]
    pub base: Summary,
    pub industry: Option<Industry>,
    pub role_type: Option<RoleType>,
    pub skills: Vec<Skill>,
}

// ── Address ──────────────────────────────────────────────────────────

/// Shared address entity. Referenced by user_profile and org_locations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Address {
    pub id: String,
    pub name: String,
    pub street_1: Option<String>,
    pub street_2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip: Option<String>,
    pub country_code: String,
    pub created_at: String,
    pub updated_at: String,
}

// ── Profile ──────────────────────────────────────────────────────────

/// A profile URL entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileUrl {
    pub id: String,
    pub profile_id: String,
    pub key: String,
    pub url: String,
    pub position: i32,
    pub created_at: String,
}

/// Global user profile — single source of truth for contact information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub id: String,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub address_id: Option<String>,
    pub address: Option<Address>,
    pub urls: Vec<ProfileUrl>,
    pub salary_minimum: Option<f64>,
    pub salary_target: Option<f64>,
    pub salary_stretch: Option<f64>,
    pub created_at: String,
    pub updated_at: String,
}

// ── Embedding ────────────────────────────────────────────────────────

/// Database row shape for embeddings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingRow {
    pub id: String,
    pub entity_type: EmbeddingEntityType,
    pub entity_id: String,
    pub content_hash: String,
    pub vector: Vec<u8>,
    pub created_at: String,
}

// ── Derivation ───────────────────────────────────────────────────────

/// A pending derivation lock row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingDerivation {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub client_id: String,
    pub prompt: String,
    pub snapshot: String,
    pub derivation_params: Option<String>,
    pub locked_at: String,
    pub expires_at: String,
    pub created_at: String,
}

// ── Audit ────────────────────────────────────────────────────────────

/// Full derivation chain trace from perspective back to source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainTrace {
    pub perspective: Perspective,
    pub bullet: Bullet,
    pub source: Source,
}

/// Integrity check comparing content snapshots to current values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityReport {
    pub perspective_id: String,
    pub bullet_snapshot_matches: bool,
    pub source_snapshot_matches: bool,
    pub bullet_diff: Option<SnapshotDiff>,
    pub source_diff: Option<SnapshotDiff>,
}

/// A snapshot/current content diff pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotDiff {
    pub snapshot: String,
    pub current: String,
}

// ── Review Queue ─────────────────────────────────────────────────────

/// An item in the bullet review queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulletReviewItem {
    #[serde(flatten)]
    pub base: Bullet,
    pub source_title: String,
}

/// An item in the perspective review queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerspectiveReviewItem {
    #[serde(flatten)]
    pub base: Perspective,
    pub bullet_content: String,
    pub source_title: String,
}

/// The combined review queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewQueue {
    pub bullets: ReviewQueueSection<BulletReviewItem>,
    pub perspectives: ReviewQueueSection<PerspectiveReviewItem>,
}

/// A section of the review queue with count + items.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewQueueSection<T> {
    pub count: i64,
    pub items: Vec<T>,
}

// ── Credential ──────────────────────────────────────────────────────

/// A non-certification credential (clearance, drivers license, bar admission, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credential {
    pub id: String,
    pub credential_type: CredentialType,
    pub label: String,
    pub status: CredentialStatus,
    pub organization_id: Option<String>,
    pub details: String,
    pub issued_date: Option<String>,
    pub expiry_date: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// ── Answer Bank ─────────────────────────────────────────────────────

/// A reusable answer for form fields (EEO, work-auth, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnswerBankEntry {
    pub id: String,
    pub field_kind: String,
    pub label: String,
    pub value: String,
    pub created_at: String,
    pub updated_at: String,
}
