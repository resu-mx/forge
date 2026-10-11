/**
 * Shared type definitions for the Forge resume builder.
 *
 * All entity interfaces, input types, status unions, result types, and
 * rich response types used across @forge/core and @forge/sdk.
 */

// ── Status Unions ─────────────────────────────────────────────────────

/** Unified 5-status model for kanban boards. Used by bullets, sources (excluding deriving), resumes, perspectives. */
export type UnifiedKanbanStatus = 'draft' | 'in_review' | 'approved' | 'rejected' | 'archived'

/** Valid statuses for a Source record. Includes transient 'deriving' lock status. */
export type SourceStatus = 'draft' | 'in_review' | 'approved' | 'rejected' | 'archived' | 'deriving'

/** Valid statuses for a Bullet record. */
export type BulletStatus = 'draft' | 'in_review' | 'approved' | 'rejected' | 'archived'

/** Valid statuses for a Perspective record. */
export type PerspectiveStatus = 'draft' | 'in_review' | 'approved' | 'rejected' | 'archived'

/** Valid statuses for a Resume record. */
export type ResumeStatus = 'draft' | 'in_review' | 'approved' | 'rejected' | 'archived'

/** Valid updated_by values. */
export type UpdatedBy = 'human' | 'ai'

/** Valid framing values for a Perspective. */
export type Framing = 'accomplishment' | 'responsibility' | 'context'

/** Valid section values for resume entries. */
export type ResumeSection =
  | 'summary'
  | 'experience'        // replaces work_history
  | 'work_history'      // kept for backward compatibility
  | 'projects'
  | 'education'
  | 'skills'
  | 'certifications'
  | 'clearance'
  | 'presentations'
  | 'awards'
  | 'custom'

/** Valid status values for organization tracking. */
export type OrganizationStatus = 'backlog' | 'researching' | 'exciting' | 'interested' | 'acceptable' | 'excluded'

/** Valid entity types for prompt logs. */
export type PromptLogEntityType = 'bullet' | 'perspective' | 'job_description'

/** A skill extracted from a JD by AI, pending human review. */
export interface ExtractedSkill {
  name: string
  category: string
  confidence: number
}

/** Result of AI skill extraction from a JD. */
export interface SkillExtractionResult {
  skills: ExtractedSkill[]
  warnings: string[]
}

/** Valid source type discriminator values. */
export type SourceType = 'role' | 'project' | 'education' | 'general' | 'presentation'

/** Valid entity types for embedding vectors. */
export type EmbeddingEntityType = 'bullet' | 'perspective' | 'jd_requirement' | 'source'

/** Valid clearance level values. */
export type ClearanceLevel = 'public' | 'confidential' | 'secret' | 'top_secret' | 'q' | 'l'

/** Valid clearance polygraph values. */
export type ClearancePolygraph = 'none' | 'ci' | 'full_scope'

/** Valid clearance status values. */
export type ClearanceStatus = 'active' | 'inactive'

/** Valid clearance type values. */
export type ClearanceType = 'personnel' | 'facility'

/** Valid clearance access program values. */
export type ClearanceAccessProgram = 'sci' | 'sap' | 'nato'

// ── Core Entities ─────────────────────────────────────────────────────

/** Location modality — how instruction/work is delivered at this location. */
export type LocationModality = 'in_person' | 'remote' | 'hybrid'

/** @deprecated Use LocationModality */
export type CampusModality = LocationModality

/** A named location belonging to an organization (office, campus, remote hub). */
export interface OrgLocation {
  id: string
  organization_id: string
  name: string
  modality: LocationModality
  address_id: string | null
  is_headquarters: number
  created_at: string
}

/** @deprecated Use OrgLocation */
export type OrgCampus = OrgLocation

/** An alias/shorthand name for an organization. */
export interface OrgAlias {
  id: string
  organization_id: string
  alias: string
}

/** Valid tags for an organization. */
export type OrgTag = 'company' | 'vendor' | 'platform' | 'university' | 'school'
  | 'nonprofit' | 'government' | 'military' | 'conference'
  | 'volunteer' | 'freelance' | 'other'

/** An organization (employer, school, etc.). */
export interface Organization {
  id: string
  name: string
  org_type: string
  tags: OrgTag[]
  industry: string | null
  size: string | null
  worked: number
  employment_type: string | null
  website: string | null
  linkedin_url: string | null
  glassdoor_url: string | null
  glassdoor_rating: number | null
  status: OrganizationStatus | null
  created_at: string
  updated_at: string
}

/** Valid statuses for a JobDescription record. */
export type JobDescriptionStatus =
  | 'discovered'
  | 'analyzing'
  | 'applying'
  | 'applied'
  | 'interviewing'
  | 'offered'
  | 'rejected'
  | 'withdrawn'
  | 'closed'

/** A stored job description for a target opportunity (base row). */
export interface JobDescription {
  id: string
  organization_id: string | null
  title: string
  url: string | null
  raw_text: string
  status: JobDescriptionStatus
  salary_range: string | null
  salary_min: number | null
  salary_max: number | null
  location: string | null
  parsed_sections: string | null
  work_posture: string | null
  parsed_locations: string | null
  salary_period: string | null
  created_at: string
  updated_at: string
}

/** JobDescription with computed organization_name from JOIN. Used in API responses. */
export interface JobDescriptionWithOrg extends JobDescription {
  organization_name: string | null
}

/**
 * A resume linked to a JD, with display fields JOINed from the resumes table.
 * `created_at` is from the junction table (when the link was created).
 * `resume_created_at` is from the resumes table (when the resume was created).
 */
export interface ResumeLink {
  resume_id: string
  resume_name: string
  target_role: string
  target_employer: string
  archetype: string
  status: ResumeStatus
  created_at: string
  resume_created_at: string
}

/**
 * A JD linked to a resume, with display fields JOINed from the
 * job_descriptions and organizations tables.
 * `created_at` is from the junction table (when the link was created).
 * `jd_created_at` is from the job_descriptions table.
 */
export interface JDLink {
  job_description_id: string
  title: string
  organization_name: string | null
  status: JobDescriptionStatus
  location: string | null
  salary_range: string | null
  created_at: string
  jd_created_at: string
}

/** Input for creating a new JobDescription. */
export interface CreateJobDescription {
  title: string
  organization_id?: string
  url?: string
  raw_text: string
  status?: JobDescriptionStatus
  salary_range?: string
  salary_min?: number
  salary_max?: number
  location?: string
  parsed_sections?: string
  work_posture?: string
  parsed_locations?: string
  salary_period?: string
}

/** Input for partially updating a JobDescription. */
export interface UpdateJobDescription {
  title?: string
  organization_id?: string | null
  url?: string | null
  raw_text?: string
  status?: JobDescriptionStatus
  salary_range?: string | null
  salary_min?: number | null
  salary_max?: number | null
  location?: string | null
  parsed_sections?: string | null
  work_posture?: string | null
  parsed_locations?: string | null
  salary_period?: string | null
}

// ── Contact Entity ─────────────────────────────────────────────────────

/** A contact person tracked in the job hunting process. */
export interface Contact {
  id: string
  name: string
  title: string | null
  email: string | null
  phone: string | null
  linkedin: string | null
  team: string | null
  dept: string | null
  notes: string | null
  organization_id: string | null
  created_at: string
  updated_at: string
}

/** Contact with computed organization_name from JOIN. Used in API responses. */
export interface ContactWithOrg extends Contact {
  organization_name: string | null
}

/** Input for creating a new Contact. */
export interface CreateContact {
  name: string
  title?: string
  email?: string
  phone?: string
  linkedin?: string
  team?: string
  dept?: string
  notes?: string
  organization_id?: string
}

/** Input for partially updating a Contact. */
export interface UpdateContact {
  name?: string
  title?: string | null
  email?: string | null
  phone?: string | null
  linkedin?: string | null
  team?: string | null
  dept?: string | null
  notes?: string | null
  organization_id?: string | null
}

/** Valid relationship types for contact-organization links. */
export type ContactOrgRelationship = 'recruiter' | 'hr' | 'referral' | 'peer' | 'manager' | 'other'

/** Valid relationship types for contact-job description links. */
export type ContactJDRelationship = 'hiring_manager' | 'recruiter' | 'interviewer' | 'referral' | 'other'

/** Valid relationship types for contact-resume links. */
export type ContactResumeRelationship = 'reference' | 'recommender' | 'other'

/** A contact linked to an entity with a typed relationship. */
export interface ContactLink {
  contact_id: string
  contact_name: string
  contact_title: string | null
  contact_email: string | null
  relationship: string
}

/** Filter parameters for listing contacts. */
export interface ContactFilter {
  organization_id?: string
  search?: string
}

/** A source experience entry — the root of the derivation chain. */
export interface Source {
  id: string
  title: string
  description: string
  source_type: SourceType
  start_date: string | null
  end_date: string | null
  status: SourceStatus
  updated_by: UpdatedBy
  last_derived_at: string | null
  created_at: string
  updated_at: string
}

/** Role-specific details for a source with source_type='role'. */
export interface SourceRole {
  source_id: string
  organization_id: string | null
  start_date: string | null
  end_date: string | null
  is_current: number
  work_arrangement: string | null
  base_salary: number | null
  total_comp_notes: string | null
}

/** Project-specific details for a source with source_type='project'. */
export interface SourceProject {
  source_id: string
  organization_id: string | null
  is_personal: number
  open_source: number
  url: string | null
  start_date: string | null
  end_date: string | null
}

// ── Education Sub-Type Unions ────────────────────────────────────────

export type DegreeLevelType = 'associate' | 'bachelors' | 'masters' | 'doctoral' | 'graduate_certificate'
export type CertificateSubtype = 'professional' | 'vendor' | 'completion'
export type EducationType = 'degree' | 'certificate' | 'course' | 'self_taught'

/** Valid presentation type discriminator values. */
export type PresentationType =
  | 'conference_talk'
  | 'workshop'
  | 'poster'
  | 'webinar'
  | 'lightning_talk'
  | 'panel'
  | 'internal'

/** Presentation-specific details for a source with source_type='presentation'. */
export interface SourcePresentation {
  source_id: string
  venue: string | null
  presentation_type: PresentationType
  url: string | null
  coauthors: string | null
}

/** Education-specific details for a source with source_type='education'. */
export interface SourceEducation {
  source_id: string
  education_type: EducationType
  // Shared
  organization_id: string | null
  campus_id: string | null
  edu_description: string | null
  location: string | null
  start_date: string | null
  end_date: string | null
  url: string | null
  // Degree-specific
  degree_level: DegreeLevelType | null
  degree_type: string | null
  field: string | null
  gpa: string | null
  is_in_progress: number
  // Certificate-specific
  certificate_subtype: CertificateSubtype | null
  credential_id: string | null
  expiration_date: string | null
}

/** A bullet point derived from a source. */
export interface Bullet {
  id: string
  content: string
  source_content_snapshot: string
  technologies: string[]
  metrics: string | null
  status: BulletStatus
  rejection_reason: string | null
  prompt_log_id: string | null
  approved_at: string | null
  approved_by: string | null
  domain: string | null
  created_at: string
}

/** Junction table linking bullets to sources. */
export interface BulletSource {
  bullet_id: string
  source_id: string
  is_primary: number
}

/** A perspective reframing of a bullet for a specific archetype/domain. */
export interface Perspective {
  id: string
  bullet_id: string
  content: string
  bullet_content_snapshot: string
  target_archetype: string | null
  domain: string | null
  framing: Framing
  status: PerspectiveStatus
  rejection_reason: string | null
  prompt_log_id: string | null
  approved_at: string | null
  approved_by: string | null
  created_at: string
}

/** A curated resume assembled from perspectives. */
export interface Resume {
  id: string
  name: string
  target_role: string
  target_employer: string
  archetype: string
  status: ResumeStatus
  header: string | null
  summary_id: string | null
  /**
   * TF-IDF tagline generated from linked JDs (Phase 92). Read-only for users;
   * auto-refreshed on JD link/unlink by the TaglineService. Display precedence
   * is `tagline_override ?? generated_tagline ?? ''`.
   */
  generated_tagline: string | null
  /**
   * User-authored tagline override (Phase 92). If set, takes precedence over
   * `generated_tagline` in all display contexts. Cleared with an explicit
   * reset to fall back to the generated value.
   */
  tagline_override: string | null
  markdown_override: string | null
  markdown_override_updated_at: string | null
  latex_override: string | null
  latex_override_updated_at: string | null
  summary_override: string | null
  summary_override_updated_at: string | null
  /** Per-resume toggle: show clearance one-liner in header (1=show, 0=hide). */
  show_clearance_in_header: number
  created_at: string
  updated_at: string
}

/** A resume section entity — first-class section with user-defined title and enforced entry type. */
export interface ResumeSectionEntity {
  id: string
  resume_id: string
  title: string
  entry_type: string
  position: number
  created_at: string
  updated_at: string
}

/** A skill pinned to a resume section (skills-type sections). */
export interface ResumeSkill {
  id: string
  section_id: string
  skill_id: string
  position: number
  created_at: string
}

// ── Certification Entities ──────────────────────────────────────────

/**
 * A certification entity — an earned credential that validates skills
 * (PMP, CISSP, AWS SA Pro, etc.).
 *
 * As of migration 041 the `name` and `issuer` (text) columns are replaced
 * by `short_name` + `long_name` and `issuer_id` (FK to organizations).
 * `education_source_id` is dropped.
 */
export interface Certification {
  id: string
  short_name: string
  long_name: string
  cert_id: string | null
  issuer_id: string | null
  date_earned: string | null
  expiry_date: string | null
  credential_id: string | null
  credential_url: string | null
  credly_url: string | null
  in_progress: boolean
  created_at: string
  updated_at: string
}

/** Input for creating a certification. `short_name` and `long_name` required. */
export interface CreateCertification {
  short_name: string
  long_name: string
  cert_id?: string
  issuer_id?: string
  date_earned?: string
  expiry_date?: string
  credential_id?: string
  credential_url?: string
  credly_url?: string
  in_progress?: boolean
}

/** Input for partially updating a certification. All fields optional with null-clearing. */
export interface UpdateCertification {
  short_name?: string
  long_name?: string
  cert_id?: string | null
  issuer_id?: string | null
  date_earned?: string | null
  expiry_date?: string | null
  credential_id?: string | null
  credential_url?: string | null
  credly_url?: string | null
  in_progress?: boolean
}

/** A certification with its linked skills populated. */
export interface CertificationWithSkills extends Certification {
  skills: Skill[]
}

/** A certification pinned to a resume (per-resume cert selection). */
export interface ResumeCertification {
  id: string
  resume_id: string
  certification_id: string
  section_id: string
  position: number
  created_at: string
}

/** Input for adding a certification to a resume. */
export interface AddResumeCertification {
  certification_id: string
  section_id: string
  position?: number
}

/** A resume template -- defines reusable section layouts without content. */
export interface ResumeTemplate {
  id: string
  name: string
  description: string | null
  sections: TemplateSectionDef[]
  is_builtin: number   // SQLite STRICT INTEGER: 0 | 1. Core keeps as number; SDK converts to boolean.
  created_at: string
  updated_at: string
}

/** A section definition within a template. */
export interface TemplateSectionDef {
  title: string
  entry_type: string
  position: number
}

/** Input for creating a resume template. */
export interface CreateResumeTemplate {
  name: string
  description?: string
  sections: TemplateSectionDef[]
}

/** Input for updating a resume template. */
export interface UpdateResumeTemplate {
  name?: string
  description?: string | null
  sections?: TemplateSectionDef[]
}

/** A resume entry linking a perspective to a resume section. */
export interface ResumeEntry {
  id: string
  resume_id: string
  section_id: string
  perspective_id: string | null
  /**
   * Optional direct link to a source. Populated when the entry was added
   * from a source that has no derived perspectives (e.g. an education
   * degree, a clearance, or a certification). An entry may have
   * `perspective_id` OR `source_id` OR both: when both are set the
   * perspective-chain source wins for IR compilation and the direct link
   * is the fallback for entries that skipped the bullet→perspective flow.
   */
  source_id: string | null
  content: string | null
  perspective_content_snapshot: string | null
  position: number
  created_at: string
  updated_at: string
}

/** A join-table entry linking a perspective to a resume section. */
export interface ResumePerspective {
  resume_id: string
  perspective_id: string
  section: ResumeSection
  position: number
}

/** Valid skill category enum (expanded in migration 031). */
export type SkillCategory =
  | 'language'
  | 'framework'
  | 'platform'
  | 'tool'
  | 'library'
  | 'methodology'
  | 'protocol'
  | 'concept'
  | 'soft_skill'
  | 'ai_ml'
  | 'infrastructure'
  | 'data_systems'
  | 'security'
  | 'other'

/** A named skill with structured category. */
export interface Skill {
  id: string
  name: string
  category: SkillCategory
}

/** A skill with its linked domains (many-to-many via skill_domains junction). */
export interface SkillWithDomains extends Skill {
  domains: Domain[]
}

/** An append-only log entry for AI prompt/response pairs. */
export interface PromptLog {
  id: string
  entity_type: PromptLogEntityType
  entity_id: string
  prompt_template: string
  prompt_input: string
  raw_response: string
  created_at: string
}

/** A user-created note. */
export interface UserNote {
  id: string
  title: string | null
  content: string
  created_at: string
  updated_at: string
}

/** Valid entity types for note_references. Must match the CHECK constraint in the database. */
export type NoteReferenceEntityType =
  | 'source'
  | 'bullet'
  | 'perspective'
  | 'resume_entry'
  | 'resume'
  | 'skill'
  | 'organization'
  | 'job_description'
  | 'contact'
  | 'credential'
  | 'certification'

/** A reference linking a note to an entity. */
export interface NoteReference {
  note_id: string
  entity_type: NoteReferenceEntityType
  entity_id: string
}

// ── Domain & Archetype Entities ───────────────────────────────────────

/** An editable experience domain. */
export interface Domain {
  id: string
  name: string
  description: string | null
  created_at: string
}

/** An editable industry (e.g. fintech, healthcare, defense). */
export interface Industry {
  id: string
  name: string
  description: string | null
  created_at: string
}

/** An editable role type (e.g. IC, lead, architect, manager). */
export interface RoleType {
  id: string
  name: string
  description: string | null
  created_at: string
}

/** An editable resume archetype. */
export interface Archetype {
  id: string
  name: string
  description: string | null
  created_at: string
}

/** Junction linking an archetype to an expected domain. */
export interface ArchetypeDomain {
  archetype_id: string
  domain_id: string
  created_at: string
}

// ── Summary Entities ──────────────────────────────────────────────────

/**
 * A reusable professional summary.
 *
 * As of Phase 92 (migration 034), tagline is NO LONGER on summaries.
 * It lives on `resume.generated_tagline` / `resume.tagline_override` with
 * automatic regeneration from linked JDs via the TaglineService.
 */
export interface Summary {
  id: string
  title: string
  role: string | null
  description: string | null
  is_template: number  // 0 or 1 (SQLite integer)
  /** Industry FK — added in migration 033 (Phase 91). Nullable. */
  industry_id: string | null
  /** Role type FK — added in migration 033 (Phase 91). Nullable. */
  role_type_id: string | null
  /** Computed via subquery — number of resumes with summary_id = this.id. */
  linked_resume_count: number
  notes: string | null
  created_at: string
  updated_at: string
}

/** A Summary plus its linked industry, role_type, and keyword skills. */
export interface SummaryWithRelations extends Summary {
  industry: Industry | null
  role_type: RoleType | null
  skills: Skill[]
}

/** Input for creating a new Summary. */
export interface CreateSummary {
  title: string
  role?: string
  description?: string
  is_template?: number
  industry_id?: string | null
  role_type_id?: string | null
  notes?: string
}

/** Input for partially updating a Summary. */
export interface UpdateSummary {
  title?: string
  role?: string | null
  description?: string | null
  is_template?: number
  industry_id?: string | null
  role_type_id?: string | null
  notes?: string | null
}

// ── Addresses ─────────────────────────────────────────────────────────

/** Shared address entity. Referenced by user_profile and org_locations. */
export interface Address {
  id: string
  name: string
  street_1: string | null
  street_2: string | null
  city: string | null
  state: string | null
  zip: string | null
  country_code: string
  created_at: string
  updated_at: string
}

export interface CreateAddress {
  name: string
  street_1?: string | null
  street_2?: string | null
  city?: string | null
  state?: string | null
  zip?: string | null
  country_code?: string
}

export interface UpdateAddress {
  name?: string
  street_1?: string | null
  street_2?: string | null
  city?: string | null
  state?: string | null
  zip?: string | null
  country_code?: string
}

// ── Profile URLs ──────────────────────────────────────────────────────

export interface ProfileUrl {
  id: string
  profile_id: string
  key: string
  url: string
  position: number
  created_at: string
}

export const WELL_KNOWN_URL_KEYS = ['linkedin', 'github', 'gitlab', 'indeed', 'blog', 'portfolio'] as const
export type WellKnownUrlKey = typeof WELL_KNOWN_URL_KEYS[number]

// ── User Profile ──────────────────────────────────────────────────────

/** Global user profile — single source of truth for contact information. */
export interface UserProfile {
  id: string
  name: string
  email: string | null
  phone: string | null
  address_id: string | null
  address: Address | null
  urls: ProfileUrl[]
  salary_minimum: number | null
  salary_target: number | null
  salary_stretch: number | null
  created_at: string
  updated_at: string
}

/** Input for partially updating the user profile. */
export interface UpdateProfile {
  name?: string | null
  email?: string | null
  phone?: string | null
  address_id?: string | null
  address?: CreateAddress | UpdateAddress
  urls?: Array<{ key: string; url: string }>
  salary_minimum?: number | null
  salary_target?: number | null
  salary_stretch?: number | null
}

// ── Input Types ───────────────────────────────────────────────────────

/** Input for creating a new Source (with optional extension fields). */
export interface CreateSource {
  title: string
  description: string
  source_type?: SourceType
  start_date?: string
  end_date?: string
  // Role extension fields
  organization_id?: string
  is_current?: number
  work_arrangement?: string
  base_salary?: number
  total_comp_notes?: string
  // Project extension fields
  is_personal?: number
  open_source?: number
  url?: string
  // Education extension fields
  education_type?: EducationType
  education_organization_id?: string
  campus_id?: string
  field?: string
  is_in_progress?: number
  credential_id?: string
  expiration_date?: string
  degree_level?: DegreeLevelType
  degree_type?: string
  certificate_subtype?: CertificateSubtype
  gpa?: string
  location?: string
  edu_description?: string
  // Presentation extension fields
  venue?: string
  presentation_type?: PresentationType
  coauthors?: string
}

/** Input for partially updating a Source. */
export interface UpdateSource {
  title?: string
  description?: string
  start_date?: string | null
  end_date?: string | null
  // Role extension fields
  organization_id?: string | null
  is_current?: number
  work_arrangement?: string | null
  base_salary?: number | null
  total_comp_notes?: string | null
  // Project extension fields
  is_personal?: number
  open_source?: number
  url?: string | null
  // Education extension fields
  education_type?: EducationType
  education_organization_id?: string | null
  campus_id?: string | null
  field?: string | null
  is_in_progress?: number
  credential_id?: string | null
  expiration_date?: string | null
  degree_level?: DegreeLevelType | null
  degree_type?: string | null
  certificate_subtype?: CertificateSubtype | null
  gpa?: string | null
  location?: string | null
  edu_description?: string | null
  // Presentation extension fields
  venue?: string
  presentation_type?: PresentationType
  coauthors?: string
}

/** Input for deriving a perspective from a bullet. */
export interface DerivePerspectiveInput {
  archetype: string
  domain: string
  framing: Framing
}

/** Input for creating a new Resume. */
export interface CreateResume {
  name: string
  target_role: string
  target_employer: string
  archetype: string
  summary_id?: string
}

/** Input for partially updating a Resume. */
export interface UpdateResume {
  name?: string
  target_role?: string
  target_employer?: string
  archetype?: string
  status?: ResumeStatus
  header?: string | null
  summary_id?: string | null
  summary_override?: string | null
  markdown_override?: string | null
  latex_override?: string | null
  show_clearance_in_header?: number
}

/** Input for adding a perspective to a resume. */
export interface AddResumePerspective {
  perspective_id: string
  section: string
  position: number
}

/** Input for adding a resume entry. */
export interface AddResumeEntry {
  section_id: string
  perspective_id?: string
  /**
   * Direct link to a source. Use for entries where the source has no
   * derived perspectives (e.g. education degrees, clearances, certain
   * certifications). Pairs naturally with `content` so the picker can
   * record the source provenance even when the entry text comes from
   * the source description rather than a perspective.
   */
  source_id?: string
  /**
   * Entry position within the section. Optional — when omitted the repository
   * appends the entry at the next available position (MAX(position) + 1) for
   * the given section, so callers that want "add to the end" don't need to
   * pre-compute the index.
   */
  position?: number
  content?: string | null
}

/** Input for reordering all perspectives in a resume. */
export interface ReorderPerspectives {
  perspectives: Array<{
    perspective_id: string
    section: string
    position: number
  }>
}

/** Input for rejecting a bullet or perspective. */
export interface RejectInput {
  rejection_reason: string
}

// ── Result Types ──────────────────────────────────────────────────────

/** A structured error returned by Forge operations. */
export interface ForgeError {
  code: string
  message: string
  details?: unknown
}

/** Discriminated union for operation results. */
export type Result<T> =
  | { ok: true; data: T }
  | { ok: false; error: ForgeError }

/** Pagination metadata. */
export interface Pagination {
  total: number
  offset: number
  limit: number
}

/** Discriminated union for paginated results. */
export type PaginatedResult<T> =
  | { ok: true; data: T[]; pagination: Pagination }
  | { ok: false; error: ForgeError }

/** Pagination parameters for list endpoints. */
export interface PaginationParams {
  offset?: number
  limit?: number
}

// ── Perspective Input Types ───────────────────────────────────────────

/** Input for creating a new Perspective (used by PerspectiveRepository). */
export interface CreatePerspectiveInput {
  bullet_id: string
  content: string
  bullet_content_snapshot: string
  target_archetype: string
  domain: string
  framing: Framing
  status?: PerspectiveStatus
  prompt_log_id?: string
}

/** Input for partially updating a Perspective. */
export interface UpdatePerspectiveInput {
  content?: string
  target_archetype?: string
  domain?: string
  framing?: Framing
}

/** Filter options for listing perspectives. */
export interface PerspectiveFilter {
  bullet_id?: string
  target_archetype?: string
  domain?: string
  framing?: Framing
  status?: PerspectiveStatus
  source_id?: string
  search?: string
}

// ── Audit Types ──────────────────────────────────────────────────────

/** Full derivation chain trace from perspective back to source. */
export interface ChainTrace {
  perspective: Perspective
  bullet: Bullet
  source: Source
}

/** Integrity check comparing content snapshots to current values. */
export interface IntegrityReport {
  perspective_id: string
  bullet_snapshot_matches: boolean
  source_snapshot_matches: boolean
  bullet_diff?: { snapshot: string; current: string }
  source_diff?: { snapshot: string; current: string }
}

// ── Gap Analysis Types ───────────────────────────────────────────────

export interface GapAnalysis {
  resume_id: string
  archetype: string
  target_role: string
  target_employer: string
  gaps: Gap[]
  coverage_summary: CoverageSummary
}

export type Gap = MissingDomainGap | ThinCoverageGap | UnusedBulletGap

export interface MissingDomainGap {
  type: 'missing_domain_coverage'
  domain: string
  description: string
  available_bullets: Array<{ id: string; content: string; source_title: string }>
  recommendation: string
}

export interface ThinCoverageGap {
  type: 'thin_coverage'
  domain: string
  current_count: number
  description: string
  recommendation: string
}

export interface UnusedBulletGap {
  type: 'unused_bullet'
  bullet_id: string
  bullet_content: string
  source_title: string
  description: string
  recommendation: string
}

export interface CoverageSummary {
  perspectives_included: number
  total_approved_perspectives_for_archetype: number
  domains_represented: string[]
  domains_missing: string[]
}

// ── Rich Response Types ───────────────────────────────────────────────

/** A source with its polymorphic extension data. */
export interface SourceWithExtension extends Source {
  extension: SourceRole | SourceProject | SourceEducation | SourcePresentation | null
}

/** A source with its associated bullets. */
export interface SourceWithBullets extends Source {
}

/** A bullet with its perspective count. */
export interface BulletWithRelations extends Bullet {
  perspective_count: number
}

/** A perspective with its full derivation chain. */
export interface PerspectiveWithChain extends Perspective {
  bullet: Bullet
  source: Source
}

/** A resume with entries grouped by section. */
export interface ResumeWithEntries extends Resume {
  sections: Array<{
    id: string
    title: string
    entry_type: string
    position: number
    entries: Array<ResumeEntry & { perspective_content: string | null }>
  }>
}

/** @deprecated Use ResumeWithEntries instead. */
export type ResumeWithPerspectives = ResumeWithEntries

// ── Resume IR (Intermediate Representation) ───────────────────────────

export interface ResumeDocument {
  resume_id: string
  header: ResumeHeader
  summary: ResumeSummary | null
  sections: IRSection[]
}

export interface ResumeHeader {
  name: string
  tagline: string | null
  location: string | null
  email: string | null
  phone: string | null
  linkedin: string | null
  github: string | null
  website: string | null
  /**
   * One-liner clearance string for the resume header, rendered between
   * tagline and contact info. Populated from the user's highest-level
   * active clearance credential. Format: "Active TS/SCI Clearance with
   * CI Poly". Null when no active clearance credential exists.
   */
  clearance: string | null
}

/** Summary shown at the top of the resume editor, backed by resumes.summary_id + summary_override. */
export interface ResumeSummary {
  /** FK to summaries row. Null when in freeform-only state. */
  summary_id: string | null
  /** Summary title for context on the editor card. Null when summary_id is null. */
  title: string | null
  /** Resolved text: override takes precedence over summaries.description. */
  content: string
  /** True when resumes.summary_override took precedence over the linked template. */
  is_override: boolean
}

export interface IRSection {
  id: string
  type: IRSectionType
  title: string
  display_order: number
  items: IRSectionItem[]
}

export type IRSectionType =
  | 'summary'
  | 'experience'
  | 'skills'
  | 'education'
  | 'projects'
  | 'certifications'
  | 'clearance'
  | 'presentations'
  | 'awards'
  | 'freeform'
  | 'custom'

export type IRSectionItem =
  | SummaryItem
  | ExperienceGroup
  | SkillGroup
  | EducationItem
  | ProjectItem
  | CertificationGroup
  | ClearanceItem
  | PresentationItem

export interface SummaryItem {
  kind: 'summary'
  content: string
  entry_id: string | null
}

export interface ExperienceGroup {
  kind: 'experience_group'
  id: string
  organization: string
  subheadings: ExperienceSubheading[]
}

export interface ExperienceSubheading {
  id: string
  title: string
  location: string | null
  date_range: string
  source_id: string | null
  bullets: ExperienceBullet[]
}

export interface ExperienceBullet {
  content: string
  entry_id: string | null
  source_chain?: {
    source_id: string
    source_title: string
    bullet_id: string
    bullet_preview: string
    perspective_id: string
    perspective_preview: string
  }
  is_cloned: boolean
}

export interface SkillGroup {
  kind: 'skill_group'
  categories: Array<{
    label: string
    skills: string[]
  }>
}

export interface EducationItem {
  kind: 'education'
  institution: string
  degree: string
  date: string
  entry_id: string | null
  source_id: string | null
  // New optional fields from education sub-types
  education_type?: string
  degree_level?: string | null
  degree_type?: string | null
  field?: string | null
  gpa?: string | null
  location?: string | null
  credential_id?: string | null
  issuing_body?: string | null
  certificate_subtype?: string | null
  edu_description?: string | null
  // Location fields from org_locations + addresses JOIN
  campus_name?: string | null
  campus_city?: string | null
  campus_state?: string | null
}

export interface ProjectItem {
  kind: 'project'
  name: string
  /**
   * Free-form description sourced from `sources.description` for the
   * linked project source. Useful for direct-source project entries
   * where there are no derived bullets/perspectives — the description
   * carries the context the bullets would otherwise provide.
   */
  description: string | null
  date: string | null
  entry_id: string | null
  source_id: string | null
  bullets: ExperienceBullet[]
}

export interface CertificationGroup {
  kind: 'certification_group'
  categories: Array<{
    label: string
    certs: Array<{
      name: string
      entry_id: string | null
      source_id: string | null
      /** Being worked toward, not earned yet. Every output marks it "(in progress)". */
      in_progress: boolean
    }>
  }>
}

export interface ClearanceItem {
  kind: 'clearance'
  content: string
  entry_id: string | null
  source_id: string | null
}

export interface PresentationItem {
  kind: 'presentation'
  title: string
  venue: string | null
  date: string | null
  entry_id: string | null
  source_id: string | null
  bullets: ExperienceBullet[]
  description: string | null
  presentation_type: string | null
  url: string | null
  coauthors: string | null
}

export interface LatexTemplate {
  preamble: string
  renderHeader: (header: ResumeHeader) => string
  renderSection: (section: IRSection) => string
  renderSectionFallback: (section: IRSection) => string
  footer: string
}

export type LintResult =
  | { ok: true }
  | { ok: false; errors: string[] }

// ── Alignment Constants ─────────────────────────────────────────────

export const STRONG_THRESHOLD_DEFAULT = 0.75
export const ADJACENT_THRESHOLD_DEFAULT = 0.50

// ── Alignment Types ──────────────────────────────────────────────────

export type MatchVerdict = 'strong' | 'adjacent' | 'gap'

export interface RequirementMatch {
  requirement_text: string
  requirement_index: number
  best_match: {
    entry_id: string
    perspective_id: string
    perspective_content: string
    similarity: number
  } | null
  verdict: MatchVerdict
}

export interface UnmatchedEntry {
  entry_id: string
  perspective_content: string
  best_requirement_similarity: number
}

export interface AlignmentReport {
  job_description_id: string
  resume_id: string
  overall_score: number
  requirement_matches: RequirementMatch[]
  unmatched_entries: UnmatchedEntry[]
  summary: {
    strong: number
    adjacent: number
    gaps: number
    total_requirements: number
    total_entries: number
  }
  computed_at: string
}

export interface RequirementMatchReport {
  job_description_id: string
  matches: Array<{
    requirement_text: string
    candidates: Array<{
      entity_id: string
      content: string
      similarity: number
    }>
  }>
  computed_at: string
}

export interface AlignmentScoreOptions {
  strong_threshold?: number
  adjacent_threshold?: number
}

export interface MatchRequirementsOptions {
  threshold?: number
  limit?: number
}

export interface StaleEmbedding {
  entity_type: EmbeddingEntityType
  entity_id: string
  stored_hash: string | null
  current_hash: string
}

// ── Export Types ────────────────────────────────────────────────────────

export interface DataExportBundle {
  forge_export: {
    version: string
    exported_at: string
    entities: string[]
  }
  sources?: Source[]
  bullets?: Bullet[]
  perspectives?: Perspective[]
  skills?: Skill[]
  organizations?: Organization[]
  summaries?: unknown[]       // typed as unknown[] until Spec 2 lands
  job_descriptions?: unknown[] // typed as unknown[] until Spec 4 lands
}

// ── Repository Input / Filter Types ──────────────────────────────────
// Extracted from db/repositories/ during Phase 1.8 cleanup.

/** Input for creating an Organization. */
export interface CreateOrganizationInput {
  name: string
  org_type?: string
  tags?: string[]
  industry?: string
  size?: string
  worked?: number
  employment_type?: string
  website?: string
  linkedin_url?: string
  glassdoor_url?: string
  glassdoor_rating?: number
  status?: OrganizationStatus | null
}

/** Filter options for listing Organizations. */
export interface OrganizationFilter {
  org_type?: string
  tag?: string
  worked?: number
  search?: string
  status?: string
}

/** Input for creating an Archetype. */
export interface CreateArchetypeInput {
  name: string
  description?: string
}

/** Archetype with its linked Domain array. */
export interface ArchetypeWithDomains extends Archetype {
  domains: Domain[]
}

/** Archetype with aggregated counts. */
export interface ArchetypeWithCounts extends Archetype {
  resume_count: number
  perspective_count: number
  domain_count: number
}

/** A UserNote with its NoteReference array. */
export interface UserNoteWithReferences extends UserNote {
  references: NoteReference[]
}

/** Filter options for listing Job Descriptions. */
export interface JobDescriptionFilter {
  status?: JobDescriptionStatus
  organization_id?: string
}

/** Sort-by column for summary lists. */
export type SummarySortBy = 'title' | 'created_at' | 'updated_at'

/** Sort direction. */
export type SortDirection = 'asc' | 'desc'

/** Sort configuration for summary lists. */
export interface SummarySort {
  sort_by?: SummarySortBy
  direction?: SortDirection
}

/** Filter options for listing Summaries. */
export interface SummaryFilter {
  is_template?: number
  industry_id?: string
  role_type_id?: string
  skill_id?: string
  search?: string
}

/** Alias — repositories used `CreateSummaryInput`; types uses `CreateSummary`. */
export type CreateSummaryInput = CreateSummary

/** Alias — repositories used `UpdateSummaryInput`; types uses `UpdateSummary`. */
export type UpdateSummaryInput = UpdateSummary

/** Input for creating an Industry. */
export interface CreateIndustryInput {
  name: string
  description?: string
}

/** Filter options for listing Bullets. */
export interface BulletFilter {
  source_id?: string
  status?: string
  technology?: string
  domain?: string
}

/** Input for partially updating a Bullet. */
export interface UpdateBulletInput {
  content?: string
  metrics?: string | null
  domain?: string | null
  technologies?: string[]
}

/** Filter options for listing Sources. */
export interface SourceFilter {
  source_type?: SourceType
  organization_id?: string
  status?: SourceStatus
  education_type?: string
  search?: string
}

/** Input for creating a Domain. */
export interface CreateDomainInput {
  name: string
  description?: string
}

/** Domain with perspective and archetype counts. */
export interface DomainWithUsage extends Domain {
  perspective_count: number
  archetype_count: number
}

/** Input for creating a RoleType. */
export interface CreateRoleTypeInput {
  name: string
  description?: string
}

/** A pending derivation lock row. */
export interface PendingDerivation {
  id: string
  entity_type: 'source' | 'bullet'
  entity_id: string
  client_id: string
  prompt: string
  snapshot: string
  derivation_params: string | null
  locked_at: string
  expires_at: string
  created_at: string
}

/** Input for creating a PendingDerivation. */
export interface CreatePendingDerivationInput {
  entity_type: 'source' | 'bullet'
  entity_id: string
  client_id: string
  prompt: string
  snapshot: string
  derivation_params: string | null
  expires_at: string
}

/** Database row shape for embeddings (vector is Uint8Array on read). */
export interface EmbeddingRow {
  id: string
  entity_type: EmbeddingEntityType
  entity_id: string
  content_hash: string
  vector: Uint8Array
  created_at: string
}

/** Input for upserting an embedding (vector is Float32Array on write). */
export interface UpsertEmbeddingInput {
  entity_type: EmbeddingEntityType
  entity_id: string
  content_hash: string
  vector: Float32Array
}

// ── Review Queue ──────────────────────────────────────────────────────

/** An item in the bullet review queue. */
export interface BulletReviewItem extends Bullet {
  source_title: string
}

/** An item in the perspective review queue. */
export interface PerspectiveReviewItem extends Perspective {
  bullet_content: string
  source_title: string
}

/** The combined review queue. */
export interface ReviewQueue {
  bullets: { count: number; items: BulletReviewItem[] }
  perspectives: { count: number; items: PerspectiveReviewItem[] }
}
