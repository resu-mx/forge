//! Result types, pagination, filter types, and shared constants.
//!
//! TS source: `packages/core/src/types/index.ts` (result types + filter sections)

use serde::{Deserialize, Serialize};

use super::enums::*;

// ── Error ────────────────────────────────────────────────────────────

/// Structured error type for all Forge operations.
///
/// Uses `thiserror` for Display/Error derives. Serializes to the same
/// `{ code, message, details }` JSON shape as the TS `ForgeError` for
/// API wire compatibility.
#[derive(Debug, thiserror::Error)]
pub enum ForgeError {
    /// Entity not found by ID.
    #[error("{entity_type} not found: {id}")]
    NotFound { entity_type: String, id: String },

    /// Input validation failure.
    #[error("validation error: {message}")]
    Validation {
        message: String,
        field: Option<String>,
    },

    /// Unique constraint or duplicate conflict.
    #[error("conflict: {message}")]
    Conflict { message: String },

    /// Foreign key reference to a missing entity.
    #[error("foreign key violation: {message}")]
    ForeignKey { message: String },

    /// Underlying database error (rusqlite). Only present when the `rusqlite`
    /// feature is enabled (default for native targets; disabled for wasm32
    /// builds via `default-features = false` in dependents like forge-wasm).
    #[cfg(feature = "rusqlite")]
    #[error("database error: {source}")]
    Database {
        #[from]
        source: rusqlite::Error,
    },

    /// Database error sourced from the wa-sqlite JS binding in the WASM
    /// browser runtime. Carries the JS-side message as a `String` because
    /// `JsValue` is not portable across the JS↔Rust boundary in error types.
    /// Shares the `DATABASE_ERROR` wire code with the native variant so API
    /// consumers can't distinguish backends.
    ///
    /// Always-present (no feature gate). Native consumers will never
    /// construct this variant — but they would otherwise need a feature
    /// passthrough to handle it in exhaustive matches once cargo workspace
    /// feature unification activates `forge-core/wasm` via forge-wasm.
    /// Cheaper to keep the variant always available than coordinate
    /// per-consumer cfg gating.
    #[error("database error: {0}")]
    WasmDatabase(String),

    /// Resource expired (HTTP 410 Gone).
    #[error("gone: {message}")]
    Gone { message: String },

    /// Catch-all for internal/unexpected errors.
    #[error("{0}")]
    Internal(String),
}

impl ForgeError {
    /// Error code string for API responses.
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotFound { .. } => "NOT_FOUND",
            Self::Validation { .. } => "VALIDATION_ERROR",
            Self::Conflict { .. } => "CONFLICT",
            Self::ForeignKey { .. } => "FK_VIOLATION",
            #[cfg(feature = "rusqlite")]
            Self::Database { .. } => "DATABASE_ERROR",
            Self::WasmDatabase(_) => "DATABASE_ERROR",
            Self::Gone { .. } => "GONE",
            Self::Internal(_) => "INTERNAL_ERROR",
        }
    }
}

impl Serialize for ForgeError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("ForgeError", 2)?;
        s.serialize_field("code", self.code())?;
        s.serialize_field("message", &self.to_string())?;
        s.end()
    }
}

// ── Pagination ───────────────────────────────────────────────────────

/// Pagination metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pagination {
    pub total: i64,
    pub offset: i64,
    pub limit: i64,
}

/// Pagination parameters for list endpoints.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PaginationParams {
    pub offset: Option<i64>,
    pub limit: Option<i64>,
}

/// Lint result — ok or errors.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LintResult {
    Ok { ok: bool },
    Err { ok: bool, errors: Vec<String> },
}

// ── Filter Types ─────────────────────────────────────────────────────

/// Filter options for listing contacts.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContactFilter {
    pub organization_id: Option<String>,
    pub search: Option<String>,
}

/// Filter options for listing perspectives.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PerspectiveFilter {
    pub bullet_id: Option<String>,
    pub target_archetype: Option<String>,
    pub domain: Option<String>,
    pub framing: Option<Framing>,
    pub status: Option<PerspectiveStatus>,
    pub source_id: Option<String>,
    pub search: Option<String>,
}

/// Filter options for listing Organizations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OrganizationFilter {
    pub org_type: Option<String>,
    pub tag: Option<String>,
    pub worked: Option<i32>,
    pub search: Option<String>,
    pub status: Option<String>,
}

/// Filter options for listing Job Descriptions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JobDescriptionFilter {
    pub status: Option<JobDescriptionStatus>,
    pub organization_id: Option<String>,
}

/// Sort configuration for summary lists.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SummarySort {
    pub sort_by: Option<SummarySortBy>,
    pub direction: Option<SortDirection>,
}

/// Filter options for listing Summaries.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SummaryFilter {
    pub is_template: Option<i32>,
    pub industry_id: Option<String>,
    pub role_type_id: Option<String>,
    pub skill_id: Option<String>,
    pub search: Option<String>,
}

/// Filter options for listing Bullets.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BulletFilter {
    pub source_id: Option<String>,
    pub status: Option<String>,
    pub technology: Option<String>,
    pub domain: Option<String>,
}

/// Filter options for listing Sources.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SourceFilter {
    pub source_type: Option<SourceType>,
    pub organization_id: Option<String>,
    pub status: Option<SourceStatus>,
    pub education_type: Option<String>,
    pub search: Option<String>,
}

// ── Gap Analysis ─────────────────────────────────────────────────────

/// Gap analysis report for a resume.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapAnalysis {
    pub resume_id: String,
    pub archetype: String,
    pub target_role: String,
    pub target_employer: String,
    pub gaps: Vec<Gap>,
    pub coverage_summary: CoverageSummary,
}

/// A gap found in resume coverage.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Gap {
    #[serde(rename = "missing_domain_coverage")]
    MissingDomain {
        domain: String,
        description: String,
        available_bullets: Vec<GapBulletCandidate>,
        recommendation: String,
    },
    #[serde(rename = "thin_coverage")]
    ThinCoverage {
        domain: String,
        current_count: i64,
        description: String,
        recommendation: String,
    },
    #[serde(rename = "unused_bullet")]
    UnusedBullet {
        bullet_id: String,
        bullet_content: String,
        source_title: String,
        description: String,
        recommendation: String,
    },
}

/// A bullet candidate in a gap analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapBulletCandidate {
    pub id: String,
    pub content: String,
    pub source_title: String,
}

/// Coverage summary within a gap analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageSummary {
    pub perspectives_included: i64,
    pub total_approved_perspectives_for_archetype: i64,
    pub domains_represented: Vec<String>,
    pub domains_missing: Vec<String>,
}

// ── Tagline ──────────────────────────────────────────────────────────

/// Body of `GET /resumes/:id/tagline` and `PATCH /resumes/:id/tagline-override`
/// (`packages/sdk/src/types.ts:443-451`). Field order is the TS key order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResumeTaglineState {
    pub generated_tagline: Option<String>,
    pub tagline_override: Option<String>,
    /// `tagline_override ?? generated_tagline ?? ""`, with no trim.
    pub resolved: String,
    /// True only for a non-blank override.
    pub has_override: bool,
}

// ── Alignment ────────────────────────────────────────────────────────

/// A requirement-to-entry match result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequirementMatch {
    pub requirement_text: String,
    pub requirement_index: i64,
    pub best_match: Option<RequirementBestMatch>,
    pub verdict: MatchVerdict,
}

/// Best matching entry for a requirement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequirementBestMatch {
    pub entry_id: String,
    pub perspective_id: String,
    pub perspective_content: String,
    pub similarity: f64,
}

/// An entry not matched to any requirement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnmatchedEntry {
    pub entry_id: String,
    pub perspective_content: String,
    pub best_requirement_similarity: f64,
}

/// Full alignment report between a resume and a JD.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentReport {
    pub job_description_id: String,
    pub resume_id: String,
    pub overall_score: f64,
    pub requirement_matches: Vec<RequirementMatch>,
    pub unmatched_entries: Vec<UnmatchedEntry>,
    pub summary: AlignmentSummary,
    pub computed_at: String,
}

/// Summary counts within an alignment report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentSummary {
    pub strong: i64,
    pub adjacent: i64,
    pub gaps: i64,
    pub total_requirements: i64,
    pub total_entries: i64,
}

/// Requirement match report (bulk matching).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequirementMatchReport {
    pub job_description_id: String,
    pub matches: Vec<RequirementMatchGroup>,
    pub computed_at: String,
}

/// A group of candidate matches for one requirement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequirementMatchGroup {
    pub requirement_text: String,
    pub candidates: Vec<MatchCandidate>,
}

/// A candidate match within a requirement match group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchCandidate {
    pub entity_id: String,
    pub content: String,
    pub similarity: f64,
}

/// Options for alignment score computation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AlignmentScoreOptions {
    pub strong_threshold: Option<f64>,
    pub adjacent_threshold: Option<f64>,
}

/// Options for requirement matching.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MatchRequirementsOptions {
    pub threshold: Option<f64>,
    pub limit: Option<i64>,
}

/// A stale embedding needing refresh.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaleEmbedding {
    pub entity_type: EmbeddingEntityType,
    pub entity_id: String,
    pub stored_hash: Option<String>,
    pub current_hash: String,
}

// ── Export ────────────────────────────────────────────────────────────

/// Data export bundle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataExportBundle {
    pub forge_export: ExportMetadata,
    pub sources: Option<Vec<super::entities::Source>>,
    pub bullets: Option<Vec<super::entities::Bullet>>,
    pub perspectives: Option<Vec<super::entities::Perspective>>,
    pub skills: Option<Vec<super::entities::Skill>>,
    pub organizations: Option<Vec<super::entities::Organization>>,
    pub summaries: Option<Vec<serde_json::Value>>,
    pub job_descriptions: Option<Vec<serde_json::Value>>,
}

/// Metadata header for a data export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportMetadata {
    pub version: String,
    pub exported_at: String,
    pub entities: Vec<String>,
}

// ── Constants ────────────────────────────────────────────────────────

/// Default threshold for a "strong" alignment match.
pub const STRONG_THRESHOLD_DEFAULT: f64 = 0.75;

/// Default threshold for an "adjacent" alignment match.
pub const ADJACENT_THRESHOLD_DEFAULT: f64 = 0.50;

/// Well-known URL keys for profile URLs.
pub const WELL_KNOWN_URL_KEYS: &[&str] = &[
    "linkedin",
    "github",
    "gitlab",
    "indeed",
    "blog",
    "portfolio",
];
