export { ForgeClient } from './client'
export type { FetchFn, ForgeClientOptions } from './client'

// Debug store + utilities
export { DebugStore, isDevMode } from './debug'
export type { SDKLogEntry, DebugOptions } from './debug'

// Result & error types
export type {
  ForgeError,
  Pagination,
  PaginatedResult,
  PaginationParams,
  Result,
} from './types'

// Core entity types
export type {
  Source,
  Bullet,
  Perspective,
  Resume,
  Organization,
  ResumeEntry,
  ResumeSectionEntity,
  ResumeSkill,
  ResumeTemplate,
  TemplateSectionDef,
  Skill,
  SkillCategory,
  SkillWithDomains,
  UserNote,
  UserProfile,
  Summary,
} from './types'

// Source type discriminator
export type { SourceType } from './types'

// Extension types
export type {
  SourceRole,
  SourceProject,
  SourceEducation,
} from './types'

// Education sub-type unions
export type { DegreeLevelType, CertificateSubtype, EducationType } from './types'

// Clearance unions
export type {
  ClearanceLevel,
  ClearancePolygraph,
  ClearanceStatus,
  ClearanceType,
  ClearanceAccessProgram,
} from './types'

// Clearance constants
export {
  CLEARANCE_LEVELS,
  CLEARANCE_POLYGRAPHS,
  CLEARANCE_STATUSES,
  CLEARANCE_TYPES,
  CLEARANCE_ACCESS_PROGRAMS,
  CLEARANCE_LEVEL_LABELS,
  CLEARANCE_POLYGRAPH_LABELS,
  CLEARANCE_ACCESS_PROGRAM_LABELS,
} from './types'

// Organization tags, locations, aliases
export type { OrgTag, OrgLocation, OrgCampus, OrgAlias, LocationModality, CampusModality } from './types'

// Rich response types
export type {
  SourceWithBullets,
  BulletWithRelations,
  PerspectiveWithChain,
  ResumeWithEntries,
} from './types'

// Review queue types
export type {
  BulletReviewItem,
  PerspectiveReviewItem,
  ReviewQueue,
} from './types'

// Gap analysis types
export type {
  GapAnalysis,
  Gap,
  MissingDomainGap,
  ThinCoverageGap,
  UnusedBulletGap,
  CoverageSummary,
} from './types'

// Drift / integrity types
export type {
  DriftReport,
  DriftedBullet,
  DriftedPerspective,
  DriftedResumeEntry,
} from './types'

// Note types
export type {
  NoteReference,
  CreateNote,
  UpdateNote,
} from './types'

// Profile types
export type {
  UpdateProfile,
} from './types'

// Bullet source junction
export type { BulletSource } from './types'

// Domain/Archetype entity types
export type {
  Domain,
  Industry,
  RoleType,
  Archetype,
  ArchetypeDomain,
} from './types'

// Qualifications — credentials + certifications (Phase 84-86)
export type {
  CredentialType,
  CredentialStatus,
  ClearanceDetails,
  DriversLicenseDetails,
  BarAdmissionDetails,
  MedicalLicenseDetails,
  CredentialDetails,
  Credential,
  CreateCredential,
  UpdateCredential,
  Certification,
  CreateCertification,
  UpdateCertification,
  CertificationWithSkills,
  ResumeCertification,
  AddResumeCertification,
} from './types'

// Domain/Archetype input + rich response types
export type {
  CreateDomain,
  UpdateDomain,
  CreateArchetype,
  UpdateArchetype,
  ArchetypeWithDomains,
} from './types'

// Resume IR types
export type {
  ResumeDocument,
  ResumeHeader,
  IRSection,
  IRSectionType,
  IRSectionItem,
  SummaryItem,
  ExperienceGroup,
  ExperienceSubheading,
  ExperienceBullet,
  SkillGroup,
  EducationItem,
  ProjectItem,
  CertificationGroup,
  ClearanceItem,
  PresentationItem,
  LatexTemplate,
  LintResult,
} from './types'

// Input types
export type {
  CreateSource,
  UpdateSource,
  UpdateBullet,
  UpdatePerspective,
  RejectInput,
  DerivePerspectiveInput,
  CreateResume,
  UpdateResume,
  AddResumeEntry,
  UpdateResumeEntry,
  CreateOrganization,
  UpdateOrganization,
  CreateResumeTemplate,
  UpdateResumeTemplate,
} from './types'

// Job description types
export type {
  JobDescription,
  JobDescriptionWithOrg,
  JobDescriptionStatus,
  JobDescriptionFilter,
  CreateJobDescription,
  UpdateJobDescription,
  ResumeLink,
  JDLink,
  ExtractedSkill,
  SkillExtractionResult,
} from './types'

// Resume status type (used in ResumeLink)
export type { ResumeStatus } from './types'

// Phase 92: Resume tagline state + regeneration response
export type {
  ResumeTaglineState,
  ResumeTaglineRegenerationResult,
  RankedTaglineKeyword,
} from './types'

// Contact types
export type {
  Contact,
  ContactWithOrg,
  ContactLink,
  ContactFilter,
  ContactOrgRelationship,
  ContactJDRelationship,
  ContactResumeRelationship,
  CreateContact,
  UpdateContact,
} from './types'

// Note reference entity type (shared)
export type {
  NoteReferenceEntityType,
} from './types'

// Filter types
export type {
  SourceFilter,
  BulletFilter,
  PerspectiveFilter,
  OrganizationFilter,
} from './types'

// Summary types
export type {
  CreateSummary,
  UpdateSummary,
  SummaryFilter,
  SummarySort,
  SummarySortBy,
  SummarySortDirection,
  SummaryWithRelations,
} from './types'

// Export types
export type { DataExportBundle } from './types'

// Alignment constants
export {
  STRONG_THRESHOLD_DEFAULT,
  ADJACENT_THRESHOLD_DEFAULT,
} from './types'

// Alignment types
export type {
  MatchVerdict,
  RequirementMatch,
  UnmatchedEntry,
  AlignmentReport,
  RequirementMatchReport,
  AlignmentScoreOptions,
  MatchRequirementsOptions,
} from './types'

// Split-handshake derivation types
export type {
  PrepareResult,
  BulletCommitInput,
  PerspectiveCommitInput,
  JDSkillExtractionContext,
} from './types'

// Resource classes (for advanced use / testing)
export { SourcesResource } from './resources/sources'
export { BulletsResource } from './resources/bullets'
export { PerspectivesResource } from './resources/perspectives'
export { ResumesResource } from './resources/resumes'
export { ReviewResource } from './resources/review'
export { OrganizationsResource } from './resources/organizations'
export { NotesResource } from './resources/notes'
export { IntegrityResource } from './resources/integrity'
export { DomainsResource } from './resources/domains'
export { IndustriesResource, type CreateIndustry, type UpdateIndustry } from './resources/industries'
export { RoleTypesResource, type CreateRoleType, type UpdateRoleType } from './resources/role-types'
export { CredentialsResource } from './resources/credentials'
export { CertificationsResource } from './resources/certifications'
export { ArchetypesResource } from './resources/archetypes'
export { SkillsResource } from './resources/skills'
export { ProfileResource } from './resources/profile'
export { JobDescriptionsResource } from './resources/job-descriptions'
export { TemplatesResource } from './resources/templates'
export { SummariesResource } from './resources/summaries'
export { ExportResource } from './resources/export'
export { ContactsResource } from './resources/contacts'
export { AlignmentResource } from './resources/alignment'
export { DerivationsResource } from './resources/derivations'
export { AnswerBankResource } from './resources/answer-bank'
export { ExtensionConfigResource } from './resources/extension-config'
export { ExtensionLogsResource } from './resources/extension-logs'

// Answer bank types (extension M6)
export type { AnswerBankEntry, UpsertAnswer } from './types'

// Extension infrastructure types (M7)
export type { ExtensionConfig, ExtensionLog, CreateExtensionLog, ExtensionLogFilter } from './types'
