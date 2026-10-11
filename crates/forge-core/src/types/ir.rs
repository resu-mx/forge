//! Resume Intermediate Representation (IR) types.
//!
//! These types define the compiled resume document structure used for
//! rendering to Markdown, LaTeX, and the web UI.
//!
//! TS source: `packages/core/src/types/index.ts` (Resume IR section)

use serde::{Deserialize, Serialize};

use super::enums::IRSectionType;

/// A fully compiled resume document ready for rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumeDocument {
    pub resume_id: String,
    pub header: ResumeHeader,
    pub summary: Option<ResumeSummary>,
    pub sections: Vec<IRSection>,
}

/// Resume header with contact information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumeHeader {
    pub name: String,
    pub tagline: Option<String>,
    pub location: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub linkedin: Option<String>,
    pub github: Option<String>,
    pub website: Option<String>,
    /// One-liner clearance string for the resume header.
    pub clearance: Option<String>,
}

/// Summary shown at the top of the resume, backed by summary_id + summary_override.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumeSummary {
    pub summary_id: Option<String>,
    pub title: Option<String>,
    pub content: String,
    pub is_override: bool,
}

/// A section in the compiled resume IR.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IRSection {
    pub id: String,
    #[serde(rename = "type")]
    pub section_type: IRSectionType,
    pub title: String,
    pub display_order: i32,
    pub items: Vec<IRSectionItem>,
}

/// Polymorphic section item, tagged by `kind`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum IRSectionItem {
    #[serde(rename = "summary")]
    Summary(SummaryItem),
    #[serde(rename = "experience_group")]
    ExperienceGroup(ExperienceGroup),
    #[serde(rename = "skill_group")]
    SkillGroup(SkillGroup),
    #[serde(rename = "education")]
    Education(EducationItem),
    #[serde(rename = "project")]
    Project(ProjectItem),
    #[serde(rename = "certification_group")]
    CertificationGroup(CertificationGroup),
    #[serde(rename = "clearance")]
    Clearance(ClearanceItem),
    #[serde(rename = "presentation")]
    Presentation(PresentationItem),
}

/// A summary text item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryItem {
    pub content: String,
    pub entry_id: Option<String>,
}

/// A group of experience entries under one organization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperienceGroup {
    pub id: String,
    pub organization: String,
    pub subheadings: Vec<ExperienceSubheading>,
}

/// A role/position subheading within an experience group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperienceSubheading {
    pub id: String,
    pub title: String,
    pub location: Option<String>,
    pub date_range: String,
    pub source_id: Option<String>,
    pub bullets: Vec<ExperienceBullet>,
}

/// A bullet point within an experience subheading.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperienceBullet {
    pub content: String,
    pub entry_id: Option<String>,
    pub source_chain: Option<SourceChain>,
    pub is_cloned: bool,
}

/// Provenance chain linking a bullet back to source → bullet → perspective.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceChain {
    pub source_id: String,
    pub source_title: String,
    pub bullet_id: String,
    pub bullet_preview: String,
    pub perspective_id: String,
    pub perspective_preview: String,
}

/// A group of skills organized by category.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillGroup {
    pub categories: Vec<SkillCategoryGroup>,
}

/// A skill category with its skill names.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillCategoryGroup {
    pub label: String,
    pub skills: Vec<String>,
}

/// An education entry in the IR.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EducationItem {
    pub institution: String,
    pub degree: String,
    pub date: String,
    pub entry_id: Option<String>,
    pub source_id: Option<String>,
    pub education_type: Option<String>,
    pub degree_level: Option<String>,
    pub degree_type: Option<String>,
    pub field: Option<String>,
    pub gpa: Option<String>,
    pub location: Option<String>,
    pub credential_id: Option<String>,
    pub issuing_body: Option<String>,
    pub certificate_subtype: Option<String>,
    pub edu_description: Option<String>,
    pub campus_name: Option<String>,
    pub campus_city: Option<String>,
    pub campus_state: Option<String>,
}

/// A project entry in the IR.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectItem {
    pub name: String,
    pub description: Option<String>,
    pub date: Option<String>,
    pub entry_id: Option<String>,
    pub source_id: Option<String>,
    pub bullets: Vec<ExperienceBullet>,
}

/// A group of certifications organized by category.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificationGroup {
    pub categories: Vec<CertificationCategoryGroup>,
}

/// A certification category with its cert entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificationCategoryGroup {
    pub label: String,
    pub certs: Vec<CertificationEntry>,
}

/// A single certification entry within a category group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificationEntry {
    pub name: String,
    pub entry_id: Option<String>,
    pub source_id: Option<String>,
    /// The certification is being worked toward, not earned yet.
    #[serde(default)]
    pub in_progress: bool,
}

impl CertificationEntry {
    /// The text every output prints: the short name, marked while the certification is in
    /// progress. One rule for Markdown, LaTeX and Typst (the TS compiler applies the same one).
    pub fn display_name(&self) -> String {
        if self.in_progress {
            format!("{} (in progress)", self.name)
        } else {
            self.name.clone()
        }
    }
}

/// A clearance entry in the IR.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClearanceItem {
    pub content: String,
    pub entry_id: Option<String>,
    pub source_id: Option<String>,
}

/// A presentation entry in the IR.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresentationItem {
    pub title: String,
    pub venue: Option<String>,
    pub date: Option<String>,
    pub entry_id: Option<String>,
    pub source_id: Option<String>,
    pub bullets: Vec<ExperienceBullet>,
    pub description: Option<String>,
    pub presentation_type: Option<String>,
    pub url: Option<String>,
    pub coauthors: Option<String>,
}
