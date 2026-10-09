/**
 * @forge/sdk — shared types
 *
 * These are standalone definitions (no imports from @forge/core) so the SDK
 * can be consumed in both Bun (CLI) and browser (WebUI) contexts without
 * pulling in server-side dependencies.
 */

// ---------------------------------------------------------------------------
// Result types
// ---------------------------------------------------------------------------

export type Result<T> =
  | { ok: true; data: T }
  | { ok: false; error: ForgeError }

export interface ForgeError {
  code: string
  message: string
  details?: unknown
}

export interface Pagination {
  total: number
  offset: number
  limit: number
}

export type PaginatedResult<T> =
  | { ok: true; data: T[]; pagination: Pagination }
  | { ok: false; error: ForgeError }

export interface PaginationParams {
  offset?: number
  limit?: number
}

// ---------------------------------------------------------------------------
// Source type discriminator
// ---------------------------------------------------------------------------

export type SourceType = 'role' | 'project' | 'education' | 'general' | 'presentation'

// ── Clearance Unions ──────────────────────────────────────────────────

export type ClearanceLevel = 'public' | 'confidential' | 'secret' | 'top_secret' | 'q' | 'l'
export type ClearancePolygraph = 'none' | 'ci' | 'full_scope'
export type ClearanceStatus = 'active' | 'inactive'
export type ClearanceType = 'personnel' | 'facility'
export type ClearanceAccessProgram = 'sci' | 'sap' | 'nato'

// ── Clearance Constants ───────────────────────────────────────────────
// These mirror the CHECK constraints in migration 018 and the constants in @forge/core.
// Duplicated here so the SDK can be consumed without server-side dependencies.

/** All valid clearance levels (ordered: lowest to highest). */
export const CLEARANCE_LEVELS: readonly ClearanceLevel[] = [
  'public', 'l', 'confidential', 'secret', 'top_secret', 'q',
] as const

/** All valid polygraph types. */
export const CLEARANCE_POLYGRAPHS: readonly ClearancePolygraph[] = [
  'none', 'ci', 'full_scope',
] as const

/** All valid clearance statuses. */
export const CLEARANCE_STATUSES: readonly ClearanceStatus[] = [
  'active', 'inactive',
] as const

/** All valid clearance types. */
export const CLEARANCE_TYPES: readonly ClearanceType[] = [
  'personnel', 'facility',
] as const

/** All valid access programs. */
export const CLEARANCE_ACCESS_PROGRAMS: readonly ClearanceAccessProgram[] = [
  'sci', 'sap', 'nato',
] as const

/** Human-readable labels for clearance levels. */
export const CLEARANCE_LEVEL_LABELS: Record<ClearanceLevel, string> = {
  public: 'Public Trust',
  confidential: 'Confidential',
  secret: 'Secret',
  top_secret: 'Top Secret (TS)',
  q: 'DOE Q',
  l: 'DOE L',
}

/** Human-readable labels for polygraph types. */
export const CLEARANCE_POLYGRAPH_LABELS: Record<ClearancePolygraph, string> = {
  none: 'None',
  ci: 'CI Polygraph',
  full_scope: 'Full Scope (Lifestyle)',
}

/** Human-readable labels for access programs. */
export const CLEARANCE_ACCESS_PROGRAM_LABELS: Record<ClearanceAccessProgram, string> = {
  sci: 'SCI',
  sap: 'SAP',
  nato: 'NATO',
}

// ---------------------------------------------------------------------------
// Source extension types
// ---------------------------------------------------------------------------

export interface SourceRole {
  organization_id: string | null
  start_date: string | null
  end_date: string | null
  is_current: boolean
  work_arrangement: string | null
  base_salary: number | null
  total_comp_notes: string | null
}

export interface SourceProject {
  organization_id: string | null
  is_personal: boolean
  open_source: boolean
  url: string | null
  start_date: string | null
  end_date: string | null
}

// ── Education Sub-Type Unions ────────────────────────────────────────

export type DegreeLevelType = 'associate' | 'bachelors' | 'masters' | 'doctoral' | 'graduate_certificate'
export type CertificateSubtype = 'professional' | 'vendor' | 'completion'
export type EducationType = 'degree' | 'certificate' | 'course' | 'self_taught'

export type LocationModality = 'in_person' | 'remote' | 'hybrid'
/** @deprecated Use LocationModality */
export type CampusModality = LocationModality

export interface OrgLocation {
  id: string
  organization_id: string
  name: string
  modality: LocationModality
  address_id: string | null
  is_headquarters: boolean
  created_at: string
}

/** @deprecated Use OrgLocation */
export type OrgCampus = OrgLocation

export interface SourceEducation {
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
  is_in_progress: boolean
  // Certificate-specific
  certificate_subtype: CertificateSubtype | null
  credential_id: string | null
  expiration_date: string | null
}

// SourceClearance was removed in migration 037 (Phase 84, Qualifications
// track). Clearance is now a Credential, not a Source extension. The
// ClearanceLevel / ClearancePolygraph / ClearanceStatus / ClearanceType /
// ClearanceAccessProgram enums above remain exported — they're reused by
// the `credentials` entity's type-specific details JSON in Phase 85.

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

// ---------------------------------------------------------------------------
// Qualifications — credentials + certifications (Phase 84-85)
// ---------------------------------------------------------------------------

/** Valid credential type discriminator values. */
export type CredentialType =
  | 'clearance'
  | 'drivers_license'
  | 'bar_admission'
  | 'medical_license'

/** Valid credential status values. */
export type CredentialStatus = 'active' | 'inactive' | 'expired'

/** Clearance-specific details stored in credentials.details JSON. */
export interface ClearanceDetails {
  level: ClearanceLevel
  polygraph: ClearancePolygraph | null
  clearance_type: ClearanceType
  access_programs: ClearanceAccessProgram[]
}

/** Driver's license details stored in credentials.details JSON. */
export interface DriversLicenseDetails {
  class: string
  state: string
  endorsements: string[]
}

/** Bar admission details stored in credentials.details JSON. */
export interface BarAdmissionDetails {
  jurisdiction: string
  bar_number: string | null
}

/** Medical license details stored in credentials.details JSON. */
export interface MedicalLicenseDetails {
  license_type: string
  state: string
  license_number: string | null
}

/** Union of all credential detail types. */
export type CredentialDetails =
  | ClearanceDetails
  | DriversLicenseDetails
  | BarAdmissionDetails
  | MedicalLicenseDetails

/** A credential entity (clearance, license, admission). */
export interface Credential {
  id: string
  credential_type: CredentialType
  label: string
  status: CredentialStatus
  organization_id: string | null
  details: CredentialDetails
  issued_date: string | null
  expiry_date: string | null
  created_at: string
  updated_at: string
}

/** Input for creating a credential. */
export interface CreateCredential {
  credential_type: CredentialType
  label: string
  status?: CredentialStatus
  organization_id?: string
  details: CredentialDetails
  issued_date?: string
  expiry_date?: string
}

/** Input for updating a credential. */
export interface UpdateCredential {
  label?: string
  status?: CredentialStatus
  organization_id?: string | null
  details?: Partial<CredentialDetails>
  issued_date?: string | null
  expiry_date?: string | null
}

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

// ---------------------------------------------------------------------------
// Status types
// ---------------------------------------------------------------------------

/** Unified 5-status model for kanban boards. Used by bullets, sources (excluding deriving), resumes, perspectives. */
export type UnifiedKanbanStatus = 'draft' | 'in_review' | 'approved' | 'rejected' | 'archived'

// ---------------------------------------------------------------------------
// Core entities
// ---------------------------------------------------------------------------

export interface Source {
  id: string
  title: string
  description: string
  source_type: SourceType
  start_date: string | null
  end_date: string | null
  status: 'draft' | 'in_review' | 'approved' | 'rejected' | 'archived' | 'deriving'
  updated_by: 'human' | 'ai'
  last_derived_at: string | null
  created_at: string
  updated_at: string
  // Extension data — present when source_type matches
  role?: SourceRole
  project?: SourceProject
  education?: SourceEducation
  presentation?: SourcePresentation
}

export interface Bullet {
  id: string
  content: string
  source_content_snapshot: string
  technologies: string[]
  metrics: string | null
  domain: string | null
  status: 'draft' | 'in_review' | 'approved' | 'rejected' | 'archived'
  rejection_reason: string | null
  prompt_log_id: string | null
  approved_at: string | null
  approved_by: string | null
  created_at: string
  sources: Array<{ id: string; title: string; is_primary: boolean }>
}

export interface Perspective {
  id: string
  bullet_id: string
  content: string
  bullet_content_snapshot: string
  target_archetype: string | null
  domain: string | null
  framing: 'accomplishment' | 'responsibility' | 'context'
  status: 'draft' | 'in_review' | 'approved' | 'rejected' | 'archived'
  rejection_reason: string | null
  prompt_log_id: string | null
  approved_at: string | null
  approved_by: string | null
  created_at: string
}

export interface Resume {
  id: string
  name: string
  target_role: string
  target_employer: string
  archetype: string
  status: 'draft' | 'in_review' | 'approved' | 'rejected' | 'archived'
  header: string | null
  summary_id: string | null
  /** TF-IDF tagline generated from linked JDs (Phase 92). Read-only. */
  generated_tagline: string | null
  /** User-authored tagline override (Phase 92). Takes precedence over generated. */
  tagline_override: string | null
  markdown_override: string | null
  markdown_override_updated_at: string | null
  latex_override: string | null
  latex_override_updated_at: string | null
  summary_override: string | null
  summary_override_updated_at: string | null
  /** Per-resume toggle: show clearance one-liner in header (default true). */
  show_clearance_in_header: boolean
  created_at: string
  updated_at: string
}

/** Resume tagline state as returned by GET /resumes/:id/tagline (Phase 92). */
export interface ResumeTaglineState {
  generated_tagline: string | null
  tagline_override: string | null
  /** Resolved value: override takes precedence over generated; empty string if both null. */
  resolved: string
  /** True when tagline_override is set and non-empty. */
  has_override: boolean
}

/** A ranked keyword used to build a generated tagline (Phase 92). */
export interface RankedTaglineKeyword {
  term: string
  score: number
  matchedSkill: boolean
}

/** Response from POST /resumes/:id/tagline/regenerate (Phase 92). */
export interface ResumeTaglineRegenerationResult {
  generated_tagline: string
  has_override: boolean
  keywords: RankedTaglineKeyword[]
}

export type OrgTag = 'company' | 'vendor' | 'platform' | 'university' | 'school'
  | 'nonprofit' | 'government' | 'military' | 'conference'
  | 'volunteer' | 'freelance' | 'other'

export interface Organization {
  id: string
  name: string
  org_type: 'company' | 'nonprofit' | 'government' | 'military' | 'education' | 'volunteer' | 'freelance' | 'other'
  tags: OrgTag[]
  industry: string | null
  size: string | null
  worked: boolean
  employment_type: 'civilian' | 'contractor' | 'military_active' | 'military_reserve' | 'volunteer' | 'intern' | null
  website: string | null
  linkedin_url: string | null
  glassdoor_url: string | null
  glassdoor_rating: number | null
  /** Pipeline status for org vetting. Backlog->Researching->Targeting (exciting/interested/acceptable)->Excluded. Null means not in the pipeline. */
  status: 'backlog' | 'researching' | 'exciting' | 'interested' | 'acceptable' | 'excluded' | null
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

/** Filter options for listing job descriptions. */
export interface JobDescriptionFilter {
  status?: JobDescriptionStatus
  organization_id?: string
}

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

/** Valid statuses for a Resume record. */
export type ResumeStatus = 'draft' | 'in_review' | 'approved' | 'rejected' | 'archived'

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

/** A resume template -- defines reusable section layouts without content. */
export interface ResumeTemplate {
  id: string
  name: string
  description: string | null
  sections: TemplateSectionDef[]
  is_builtin: boolean   // SDK converts 0|1 to boolean for developer ergonomics
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

export interface ResumeEntry {
  id: string
  resume_id: string
  section_id: string
  perspective_id: string | null
  /**
   * Optional direct link to a source. Populated when the entry was added
   * from a source that has no derived perspectives (e.g. an education
   * degree, a clearance, a certification). Lets the IR compiler reach the
   * source's structured data (source_education, source_clearances, etc.)
   * without going through the perspective→bullet→bullet_sources chain.
   */
  source_id: string | null
  content: string | null
  perspective_content_snapshot: string | null
  position: number
  created_at: string
  updated_at: string
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

export interface UserNote {
  id: string
  title: string | null
  content: string
  references: NoteReference[]
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

export interface NoteReference {
  entity_type: NoteReferenceEntityType
  entity_id: string
}

export interface BulletSource {
  bullet_id: string
  source_id: string
  is_primary: boolean
}

// ---------------------------------------------------------------------------
// Domain & Archetype entities
// ---------------------------------------------------------------------------

/** An editable experience domain. */
export interface Domain {
  id: string
  name: string
  description: string | null
  created_at: string
}

/** A domain with its usage, as `GET /api/domains` returns it. */
export interface DomainWithUsage extends Domain {
  /** Perspectives whose `domain` equals this domain's name. */
  perspective_count: number
  /** Archetypes linked through `archetype_domains`. */
  archetype_count: number
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

// ---------------------------------------------------------------------------
// Domain & Archetype input types
// ---------------------------------------------------------------------------

export interface CreateDomain {
  name: string
  description?: string
}

export interface UpdateDomain {
  name?: string
  description?: string | null
}

export interface CreateArchetype {
  name: string
  description?: string
}

export interface UpdateArchetype {
  name?: string
  description?: string | null
}

export interface ArchetypeWithDomains extends Archetype {
  domains: Domain[]
}

// ---------------------------------------------------------------------------
// Summary entities
// ---------------------------------------------------------------------------

/** A reusable professional summary. */
export interface Summary {
  id: string
  title: string
  role: string | null
  /**
   * @deprecated Phase 92 (Tagline Engine) moves tagline to resume-level
   * `generated_tagline` / `tagline_override`. Do not add new tagline features
   * on summaries.
   */
  tagline: string | null
  description: string | null
  /** SQLite stores as 0|1 integer. JavaScript treats 0 as falsy and 1 as truthy,
   *  so `if (summary.is_template)` works. Strict `=== true` will fail. */
  is_template: boolean
  /** Industry FK (Phase 91). Nullable. */
  industry_id: string | null
  /** Role type FK (Phase 91). Nullable. */
  role_type_id: string | null
  /** Computed via subquery -- number of resumes with summary_id = this.id. */
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
  /** @deprecated see Summary.tagline */
  tagline?: string
  description?: string
  is_template?: boolean
  industry_id?: string | null
  role_type_id?: string | null
  notes?: string
}

/** Input for partially updating a Summary. */
export interface UpdateSummary {
  title?: string
  role?: string | null
  /** @deprecated see Summary.tagline */
  tagline?: string | null
  description?: string | null
  is_template?: boolean
  industry_id?: string | null
  role_type_id?: string | null
  notes?: string | null
}

/** Filter for listing summaries. */
export interface SummaryFilter {
  is_template?: boolean
  industry_id?: string
  role_type_id?: string
  skill_id?: string
  /** Case-insensitive substring search on title + description. */
  search?: string
}

/** Sort options for listing summaries. */
export type SummarySortBy = 'title' | 'created_at' | 'updated_at'
export type SummarySortDirection = 'asc' | 'desc'

export interface SummarySort {
  sort_by?: SummarySortBy
  direction?: SummarySortDirection
}

// ---------------------------------------------------------------------------
// Addresses
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Profile URLs
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// User Profile
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Rich response types (with nested data)
// ---------------------------------------------------------------------------

export interface SourceWithBullets extends Source {
  bullet_count: number
}

export interface BulletWithRelations extends Bullet {
  perspective_count: number
}

export interface PerspectiveWithChain extends Perspective {
  bullet: Bullet
  source: Source
}

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

// ---------------------------------------------------------------------------
// Resume IR (Intermediate Representation)
// ---------------------------------------------------------------------------

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
   * One-liner clearance string for the resume header. Populated from the
   * user's highest-level active clearance credential. Format: "Active
   * TS/SCI Clearance with CI Poly". Null when no active clearance exists.
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
   * Free-form description from the linked project source. Carries
   * context for direct-source project entries that have no derived
   * bullets/perspectives yet.
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

// ---------------------------------------------------------------------------
// LaTeX Template
// ---------------------------------------------------------------------------

export interface LatexTemplate {
  preamble: string
  renderHeader: (header: ResumeHeader) => string
  renderSection: (section: IRSection) => string
  renderSectionFallback: (section: IRSection) => string
  footer: string
}

// ---------------------------------------------------------------------------
// Lint Result
// ---------------------------------------------------------------------------

export type LintResult =
  | { ok: true }
  | { ok: false; errors: string[] }

// ---------------------------------------------------------------------------
// Review queue
// ---------------------------------------------------------------------------

export interface BulletReviewItem extends Bullet {
  source_title: string
}

export interface PerspectiveReviewItem extends Perspective {
  bullet_content: string
  source_title: string
}

export interface ReviewQueue {
  bullets: { count: number; items: BulletReviewItem[] }
  perspectives: { count: number; items: PerspectiveReviewItem[] }
}

// ---------------------------------------------------------------------------
// Gap analysis
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Drift / integrity types
// ---------------------------------------------------------------------------

export interface DriftReport {
  bullets: DriftedBullet[]
  perspectives: DriftedPerspective[]
  resume_entries: DriftedResumeEntry[]
}

export interface DriftedBullet {
  id: string
  content: string
  source_content_snapshot: string
  current_source_description: string
  source_id: string
  source_title: string
}

export interface DriftedPerspective {
  id: string
  content: string
  bullet_content_snapshot: string
  current_bullet_content: string
  bullet_id: string
}

export interface DriftedResumeEntry {
  id: string
  perspective_content_snapshot: string | null
  current_perspective_content: string
  perspective_id: string
  resume_id: string
}

// ---------------------------------------------------------------------------
// Input types
// ---------------------------------------------------------------------------

export interface CreateSource {
  title: string
  description: string
  source_type?: SourceType
  start_date?: string
  end_date?: string
  role?: Partial<SourceRole>
  project?: Partial<SourceProject>
  education?: Partial<SourceEducation>
  // Presentation extension fields
  venue?: string
  presentation_type?: PresentationType
  coauthors?: string
}

export interface UpdateSource {
  title?: string
  description?: string
  start_date?: string | null
  end_date?: string | null
  role?: Partial<SourceRole>
  project?: Partial<SourceProject>
  education?: Partial<SourceEducation>
  // Presentation extension fields
  venue?: string
  presentation_type?: PresentationType
  coauthors?: string
}

export interface UpdateBullet {
  content?: string
  technologies?: string[]
  metrics?: string | null
  domain?: string | null
}

export interface CreatePerspective {
  bullet_id: string
  content: string
  target_archetype?: string
  domain?: string
  framing?: 'accomplishment' | 'responsibility' | 'context'
  auto_approve?: boolean
}

export interface UpdatePerspective {
  content?: string
  target_archetype?: string | null
  domain?: string | null
  framing?: 'accomplishment' | 'responsibility' | 'context'
}

export interface RejectInput {
  rejection_reason: string
}

export interface DerivePerspectiveInput {
  archetype: string
  domain: string
  framing: 'accomplishment' | 'responsibility' | 'context'
}

export interface CreateResume {
  name: string
  target_role: string
  target_employer: string
  archetype: string
  summary_id?: string
  /** Pre-populate sections from this resume template. */
  template_id?: string
}

export interface UpdateResume {
  name?: string
  target_role?: string
  target_employer?: string
  archetype?: string
  status?: 'draft' | 'in_review' | 'approved' | 'rejected' | 'archived'
  header?: string | null
  summary_id?: string | null
  summary_override?: string | null
  markdown_override?: string | null
  latex_override?: string | null
}

export interface AddResumeEntry {
  section_id: string
  perspective_id?: string
  /**
   * Direct link to a source. Use for entries where the source has no
   * derived perspectives (education degrees, clearances, certain
   * certifications). Pairs with `content` so the picker can record the
   * source provenance even when the entry text comes from the source
   * description rather than a perspective.
   */
  source_id?: string
  /**
   * Entry position within the section. Optional — when omitted the server
   * appends the entry at the next available position for the section. Most
   * callers can skip this and get "add to the end" behavior.
   */
  position?: number
  content?: string | null
}

export interface UpdateResumeEntry {
  content?: string | null
  section_id?: string
  position?: number
}

export interface CreateOrganization {
  name: string
  org_type?: string
  industry?: string
  size?: string
  worked?: boolean
  employment_type?: string
  website?: string
  linkedin_url?: string
  glassdoor_url?: string
  glassdoor_rating?: number
  status?: 'backlog' | 'researching' | 'exciting' | 'interested' | 'acceptable' | 'excluded' | null
}

export interface UpdateOrganization {
  name?: string
  org_type?: string
  industry?: string
  size?: string
  worked?: boolean
  employment_type?: string | null
  website?: string | null
  linkedin_url?: string | null
  glassdoor_url?: string | null
  glassdoor_rating?: number | null
  status?: 'backlog' | 'researching' | 'exciting' | 'interested' | 'acceptable' | 'excluded' | null
}

export interface CreateNote {
  title?: string
  content: string
}

export interface UpdateNote {
  title?: string | null
  content?: string
}

// ---------------------------------------------------------------------------
// Filter types
// ---------------------------------------------------------------------------

export interface SourceFilter {
  status?: string
  source_type?: string
  /**
   * Narrow education-type sources by their subtype. Only meaningful when
   * `source_type: 'education'` is also set. Values match
   * `source_education.education_type` (e.g. `'degree'`, `'certificate'`,
   * `'course'`).
   */
  education_type?: string
  /** Case-insensitive substring search on title + description. */
  search?: string
}

export interface BulletFilter {
  source_id?: string
  status?: string
  technology?: string
}

export interface PerspectiveFilter {
  bullet_id?: string
  archetype?: string
  domain?: string
  framing?: string
  status?: string
  source_id?: string
  /** Case-insensitive substring search on perspective content. */
  search?: string
}

export interface OrganizationFilter {
  org_type?: string
  tag?: string
  worked?: string
  status?: string
  search?: string
}

// ---------------------------------------------------------------------------
// Export Types
// ---------------------------------------------------------------------------

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
  summaries?: unknown[]
  job_descriptions?: unknown[]
}

// ---------------------------------------------------------------------------
// Request function types (used by resource classes)
// ---------------------------------------------------------------------------

export type RequestFn = <T>(
  method: string,
  path: string,
  body?: unknown,
) => Promise<Result<T>>

export type RequestListFn = <T>(
  method: string,
  path: string,
  params?: Record<string, string>,
) => Promise<PaginatedResult<T>>

// ---------------------------------------------------------------------------
// Alignment Constants
// ---------------------------------------------------------------------------

export const STRONG_THRESHOLD_DEFAULT = 0.75
export const ADJACENT_THRESHOLD_DEFAULT = 0.50

// ---------------------------------------------------------------------------
// Alignment Types
// ---------------------------------------------------------------------------

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

// ── Split-Handshake Derivation ─────────────────────────────────────

export interface PrepareResult {
  derivation_id: string
  prompt: string
  snapshot: string
  instructions: string
  expires_at: string
}

export interface BulletCommitInput {
  bullets: Array<{ content: string; technologies: string[]; metrics: string | null }>
}

export interface PerspectiveCommitInput {
  content: string
  reasoning: string
}

// ---------------------------------------------------------------------------
// Answer Bank (extension M6)
// ---------------------------------------------------------------------------

/** A stored answer for EEO/work-auth form fields. */
export interface AnswerBankEntry {
  id: string
  field_kind: string
  label: string
  value: string
  created_at: string
  updated_at: string
}

/** Input for creating or updating an answer bank entry. */
export interface UpsertAnswer {
  field_kind: string
  label: string
  value: string
}

// ---------------------------------------------------------------------------
// JD Skill Extraction
// ---------------------------------------------------------------------------

export interface JDSkillExtractionContext {
  jd_raw_text: string
  existing_skills: Array<{ id: string; name: string; category: string }>
  prompt_template: string
  instructions: string
}

// ---------------------------------------------------------------------------
// Extension Infrastructure (M7)
// ---------------------------------------------------------------------------

/** Full extension config — all keys merged over defaults. */
export interface ExtensionConfig {
  baseUrl: string
  devMode: boolean
  enabledPlugins: string[]
  enableServerLogging: boolean
}

/** A stored extension error log entry. */
export interface ExtensionLog {
  id: string
  error_code: string
  message: string
  layer: string
  plugin: string | null
  url: string | null
  context: Record<string, unknown> | null
  created_at: string
}

/** Input for appending an extension log entry. */
export interface CreateExtensionLog {
  error_code: string
  message: string
  layer: string
  plugin?: string
  url?: string
  context?: Record<string, unknown>
}

/** Filter options for listing extension logs. */
export interface ExtensionLogFilter {
  limit?: number
  offset?: number
  error_code?: string
  layer?: string
}
