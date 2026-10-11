/**
 * Persona corpus types.
 *
 * A corpus is hand-authored, typed data. Entities refer to each other by stable local
 * keys (unique per entity kind within a persona); the generator's ledger maps those keys
 * to the ids the server assigns. Times are `daysAgo` offsets from `--as-of`; calendar
 * dates (start/end, earned, expiry) are `monthsAgo` offsets, negative for the future.
 */

// ── Enumerations (mirror the schema CHECK constraints and the seeded rows) ──

export const ARCHETYPES = ['agentic-ai', 'hft', 'infrastructure', 'public-sector', 'security-engineer', 'solutions-architect'] as const
export const DOMAINS = ['ai_ml', 'devops', 'leadership', 'security', 'software_engineering', 'systems_engineering'] as const
export const SKILL_CATEGORIES = [
  'ai_ml', 'concept', 'data_systems', 'framework', 'infrastructure', 'language', 'library',
  'methodology', 'other', 'platform', 'protocol', 'security', 'soft_skill', 'tool',
] as const
export const TEMPLATES = ['Standard Tech Resume', 'Academic CV', 'Federal Resume'] as const
export const ORG_TYPES = ['company', 'nonprofit', 'government', 'military', 'education', 'volunteer', 'freelance', 'other'] as const
export const ORG_TAGS = [
  'company', 'vendor', 'platform', 'university', 'school', 'nonprofit', 'government',
  'military', 'conference', 'volunteer', 'freelance', 'other',
] as const
export const ORG_STATUSES = ['backlog', 'researching', 'exciting', 'interested', 'acceptable', 'excluded'] as const
export const EMPLOYMENT_TYPES = ['civilian', 'contractor', 'military_active', 'military_reserve', 'volunteer', 'intern'] as const
export const LOCATION_MODALITIES = ['in_person', 'remote', 'hybrid'] as const
export const SOURCE_TYPES = ['role', 'project', 'education', 'general', 'presentation'] as const
export const EDUCATION_TYPES = ['degree', 'certificate', 'course', 'self_taught'] as const
export const DEGREE_LEVELS = ['associate', 'bachelors', 'masters', 'doctoral', 'graduate_certificate'] as const
export const CERTIFICATE_SUBTYPES = ['professional', 'vendor', 'completion'] as const
export const PRESENTATION_TYPES = ['conference_talk', 'workshop', 'poster', 'webinar', 'lightning_talk', 'panel', 'internal'] as const
export const REVIEW_STATUSES = ['draft', 'in_review', 'approved', 'rejected', 'archived'] as const
export const FRAMINGS = ['accomplishment', 'responsibility', 'context'] as const
export const JD_STATUSES = [
  'discovered', 'analyzing', 'applying', 'applied', 'interviewing', 'offered', 'rejected', 'withdrawn', 'closed',
] as const
export const CREDENTIAL_TYPES = ['clearance', 'drivers_license', 'bar_admission', 'medical_license'] as const
export const CREDENTIAL_STATUSES = ['active', 'inactive', 'expired'] as const
export const CONTACT_ORG_RELATIONSHIPS = ['recruiter', 'hr', 'referral', 'peer', 'manager', 'other'] as const
export const CONTACT_JD_RELATIONSHIPS = ['hiring_manager', 'recruiter', 'interviewer', 'referral', 'other'] as const
export const CONTACT_RESUME_RELATIONSHIPS = ['reference', 'recommender', 'other'] as const
export const NOTE_ENTITY_TYPES = [
  'source', 'bullet', 'perspective', 'resume_entry', 'resume', 'skill', 'organization',
  'job_description', 'contact', 'credential', 'certification',
] as const
export const SECTION_ENTRY_TYPES = [
  'experience', 'skills', 'education', 'projects', 'clearance', 'presentations', 'certifications', 'awards', 'freeform',
] as const

/** Answer-bank keys and their allowed values (from the web UI's settings pages). */
export const ANSWER_OPTIONS: Record<string, { label: string; values: readonly string[] }> = {
  'work_auth.us': { label: 'Are you authorized to work in the United States?', values: ['Yes', 'No'] },
  'work_auth.sponsorship': { label: 'Will you now or in the future require sponsorship?', values: ['Yes', 'No'] },
  'eeo.gender': {
    label: 'Gender',
    values: ['Male', 'Female', 'Non-binary', 'Prefer not to say', 'Decline to self-identify'],
  },
  'eeo.race': {
    label: 'Race / Ethnicity',
    values: [
      'American Indian or Alaska Native', 'Asian', 'Black or African American', 'Hispanic or Latino',
      'Native Hawaiian or Other Pacific Islander', 'White', 'Two or More Races', 'Decline to self-identify',
    ],
  },
  'eeo.veteran': { label: 'Veteran Status', values: ['Protected Veteran', 'Not a Veteran', 'Prefer not to say'] },
  'eeo.disability': {
    label: 'Disability Status',
    values: [
      'Yes, I have a disability (or previously had a disability)',
      'No, I do not have a disability',
      'Prefer not to say',
    ],
  },
}

export type Archetype = (typeof ARCHETYPES)[number]
export type Domain = (typeof DOMAINS)[number]
export type SkillCategory = (typeof SKILL_CATEGORIES)[number]
export type TemplateName = (typeof TEMPLATES)[number]
export type OrgType = (typeof ORG_TYPES)[number]
export type OrgTag = (typeof ORG_TAGS)[number]
export type OrgStatus = (typeof ORG_STATUSES)[number]
export type EmploymentType = (typeof EMPLOYMENT_TYPES)[number]
export type LocationModality = (typeof LOCATION_MODALITIES)[number]
export type SourceType = (typeof SOURCE_TYPES)[number]
export type EducationType = (typeof EDUCATION_TYPES)[number]
export type DegreeLevel = (typeof DEGREE_LEVELS)[number]
export type CertificateSubtype = (typeof CERTIFICATE_SUBTYPES)[number]
export type PresentationType = (typeof PRESENTATION_TYPES)[number]
export type ReviewStatus = (typeof REVIEW_STATUSES)[number]
export type Framing = (typeof FRAMINGS)[number]
export type JdStatus = (typeof JD_STATUSES)[number]
export type CredentialType = (typeof CREDENTIAL_TYPES)[number]
export type CredentialStatus = (typeof CREDENTIAL_STATUSES)[number]
export type ContactOrgRelationship = (typeof CONTACT_ORG_RELATIONSHIPS)[number]
export type ContactJdRelationship = (typeof CONTACT_JD_RELATIONSHIPS)[number]
export type ContactResumeRelationship = (typeof CONTACT_RESUME_RELATIONSHIPS)[number]
export type NoteEntityType = (typeof NOTE_ENTITY_TYPES)[number]
export type SectionEntryType = (typeof SECTION_ENTRY_TYPES)[number]

// ── Corpus entities ─────────────────────────────────────────────────

/** A calendar date `monthsAgo` months before `--as-of` (negative: in the future). */
export interface RelDate {
  monthsAgo: number
  /** Day of the month, default 1. */
  day?: number
}

export interface AddressSpec {
  /**
   * The display location (the profile page's "Display Location"; resume headers print it),
   * e.g. "Columbus, OH". `name / street_1` is the address's natural key: unique per persona.
   */
  name: string
  street_1?: string
  street_2?: string
  city: string
  state: string
  zip: string
}

export interface ProfileSpec {
  name: string
  email: string
  phone: string
  address: AddressSpec
  urls: { key: string; url: string }[]
  salary_minimum: number
  salary_target: number
  salary_stretch: number
}

export interface NamedSpec {
  key: string
  name: string
  description?: string
}

export interface SkillSpec {
  key: string
  /** Stored as given: start with an upper-case letter (the API capitalises the first one). */
  name: string
  category: SkillCategory
  domains?: Domain[]
}

export interface LocationSpec {
  key: string
  name: string
  modality: LocationModality
  is_headquarters?: boolean
  address?: AddressSpec
}

export interface OrgSpec {
  key: string
  name: string
  org_type: OrgType
  tags: OrgTag[]
  /** Free-text industry label. */
  industry?: string
  size?: string
  worked: boolean
  employment_type?: EmploymentType
  website?: string
  linkedin_url?: string
  glassdoor_url?: string
  glassdoor_rating?: number
  status: OrgStatus | null
  aliases?: string[]
  locations?: LocationSpec[]
  daysAgo: number
}

export interface CredentialSpec {
  key: string
  credential_type: CredentialType
  label: string
  status: CredentialStatus
  orgKey?: string
  details: Record<string, unknown>
  issued?: RelDate
  expires?: RelDate
  daysAgo: number
}

export interface CertificationSpec {
  key: string
  short_name: string
  long_name: string
  cert_id?: string
  issuerKey?: string
  earned?: RelDate
  expires?: RelDate
  credential_id?: string
  credential_url?: string
  credly_url?: string
  in_progress: boolean
  skills: string[]
  daysAgo: number
}

export interface SourceSpec {
  key: string
  title: string
  description: string
  source_type: SourceType
  /** Final status. Anything but `draft` is set by the post-pass overlay (no API sets it). */
  status: ReviewStatus
  start?: RelDate
  end?: RelDate
  role?: {
    orgKey: string
    is_current?: boolean
    work_arrangement?: string
    total_comp_notes?: string
  }
  project?: {
    orgKey?: string
    is_personal?: boolean
    open_source?: boolean
    url?: string
  }
  education?: {
    education_type: EducationType
    orgKey?: string
    campusKey?: string
    field?: string
    is_in_progress?: boolean
    credential_id?: string
    url?: string
    degree_level?: DegreeLevel
    degree_type?: string
    certificate_subtype?: CertificateSubtype
    gpa?: string
    location?: string
    edu_description?: string
  }
  presentation?: {
    venue: string
    presentation_type: PresentationType
    url?: string
    coauthors?: string
  }
  skills: string[]
  daysAgo: number
}

export interface BulletSpec {
  key: string
  sourceKey: string
  content: string
  /** Skill keys; the derivation links them by (case-insensitive) name. */
  technologies: string[]
  metrics: string | null
  domain?: Domain
  /** `derive`: prepare/commit from the source. `draft`: `POST /bullets`. */
  via: 'derive' | 'draft'
  status: ReviewStatus
  rejection_reason?: string
  /** For an `in_review` bullet: rejected with this reason first, then reopened. */
  reopenedAfter?: string
  daysAgo: number
  /** When it was approved or rejected; defaults to `daysAgo`. */
  reviewedDaysAgo?: number
}

export interface PerspectiveSpec {
  key: string
  bulletKey: string
  content: string
  reasoning?: string
  archetype: Archetype
  domain: Domain
  framing: Framing
  /** `derive`: prepare/commit from an approved bullet. `draft`: `POST /perspectives`. */
  via: 'derive' | 'draft'
  status: ReviewStatus
  rejection_reason?: string
  daysAgo: number
  reviewedDaysAgo?: number
}

export interface SummarySpec {
  key: string
  title: string
  role?: string
  description?: string
  is_template?: boolean
  industryKey?: string
  roleTypeKey?: string
  notes?: string
  skills: string[]
  daysAgo: number
}

export interface JdSpec {
  key: string
  title: string
  orgKey?: string
  url?: string
  /** File name under the persona's `jds/` directory. */
  file: string
  status: JdStatus
  salary_min?: number
  salary_max?: number
  salary_range?: string
  location?: string
  skills: string[]
  daysAgo: number
  /** When the last status change happened; defaults to `daysAgo`. */
  updatedDaysAgo?: number
}

export interface ContactSpec {
  key: string
  name: string
  title?: string
  email?: string
  phone?: string
  linkedin?: string
  team?: string
  dept?: string
  notes?: string
  orgKey?: string
  orgLinks: { orgKey: string; relationship: ContactOrgRelationship }[]
  jdLinks: { jdKey: string; relationship: ContactJdRelationship }[]
  resumeLinks: { resumeKey: string; relationship: ContactResumeRelationship }[]
  daysAgo: number
}

/** One resume entry. Exactly one of `perspectiveKey`, `sourceKey` or `content` drives it. */
export interface EntrySpec {
  /** Optional key, so notes can reference the entry. */
  key?: string
  section: string
  perspectiveKey?: string
  sourceKey?: string
  content?: string
}

export interface ResumeSpec {
  key: string
  name: string
  target_role: string
  target_employer: string
  archetype: Archetype
  template: TemplateName
  summaryKey?: string
  status: ReviewStatus
  /** Sections added on top of the template's, e.g. a certifications section. */
  extraSections: { title: string; entry_type: SectionEntryType }[]
  entries: EntrySpec[]
  /** Skill keys per skills-section title. */
  skills: Record<string, string[]>
  /** Certification keys per certifications-section title. */
  certifications: Record<string, string[]>
  jdKeys: string[]
  daysAgo: number
  /** When the last status change happened; defaults to `daysAgo`. */
  updatedDaysAgo?: number
}

export interface NoteRef {
  type: NoteEntityType
  /** The referenced entity's local key (for `resume_entry`, the entry's `key`). */
  key: string
}

export interface NoteSpec {
  key: string
  title?: string
  content: string
  refs: NoteRef[]
  daysAgo: number
}

export interface AnswerSpec {
  field_kind: string
  value: string
}

export interface PersonaCorpus {
  slug: string
  /** One line for the index and the "About" note. */
  summary: string
  /** How long before `--as-of` the account started; seeded and system rows get this time. */
  accountAgeDays: number
  profile: ProfileSpec
  answers: AnswerSpec[]
  industries: NamedSpec[]
  roleTypes: NamedSpec[]
  skills: SkillSpec[]
  orgs: OrgSpec[]
  credentials: CredentialSpec[]
  certifications: CertificationSpec[]
  sources: SourceSpec[]
  bullets: BulletSpec[]
  perspectives: PerspectiveSpec[]
  summaries: SummarySpec[]
  jds: JdSpec[]
  contacts: ContactSpec[]
  resumes: ResumeSpec[]
  notes: NoteSpec[]
  /** Directory holding `jds/*.md`. */
  dir: string
}
