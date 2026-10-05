//! Job Description CRUD routes.
//!
//! Mirrors the TS job-description routes — same URL paths,
//! same JSON shapes, so the webui and MCP server continue working.

use std::collections::BTreeSet;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use forge_ai::prompts::jd_skill_extraction;
use forge_core::{
    ContactLink, CreateJobDescription, ForgeError, JobDescriptionFilter, JobDescriptionStatus,
    JobDescriptionWithOrg, ResumeLink, Skill, SkillRow, UpdateJobDescription,
};
use forge_sdk::db::{ContactStore, JdResumeStore, JdStore, ResumeStore, SkillStore};

use super::bullets::LinkSkillBody;
use crate::db::with_conn;
use crate::error::ApiError;
use crate::response::{ApiData, ApiList, Created, NoContent};
use crate::state::SharedState;

// ── Query params ────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Default)]
pub struct JdListQuery {
    pub offset: Option<i64>,
    pub limit: Option<i64>,
    pub status: Option<String>,
    pub organization_id: Option<String>,
    pub search: Option<String>,
}

// ── Handlers ────────────────────────────────────────────────────────

async fn create_job_description(
    State(state): State<SharedState>,
    Json(input): Json<CreateJobDescription>,
) -> Result<Created<JobDescriptionWithOrg>, ApiError> {
    let result = with_conn(&state, move |conn| {
        let jd = JdStore::create(conn, &input)?;
        JdStore::get_with_org(conn, &jd.id)?.ok_or_else(|| {
            forge_core::ForgeError::Internal("Job description created but not found".into())
        })
    })
    .await?;
    Ok(Created(result))
}

async fn list_job_descriptions(
    State(state): State<SharedState>,
    Query(q): Query<JdListQuery>,
) -> Result<Json<ApiList<JobDescriptionWithOrg>>, ApiError> {
    let filter = JobDescriptionFilter {
        status: q
            .status
            .and_then(|s| s.parse::<JobDescriptionStatus>().ok()),
        organization_id: q.organization_id,
    };
    let offset = q.offset.unwrap_or(0).max(0);
    let limit = q.limit.unwrap_or(50).clamp(1, 200);

    let (data, pagination) = with_conn(&state, move |conn| {
        JdStore::list_with_org(conn, &filter, offset, limit)
    })
    .await?;

    Ok(Json(ApiList { data, pagination }))
}

async fn get_job_description(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<ApiData<JobDescriptionWithOrg>>, ApiError> {
    let result = with_conn(&state, move |conn| {
        JdStore::get_with_org(conn, &id)?.ok_or_else(|| forge_core::ForgeError::NotFound {
            entity_type: "JobDescription".into(),
            id: id.clone(),
        })
    })
    .await?;
    Ok(Json(ApiData { data: result }))
}

async fn update_job_description(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(input): Json<UpdateJobDescription>,
) -> Result<Json<ApiData<JobDescriptionWithOrg>>, ApiError> {
    let result = with_conn(&state, move |conn| {
        JdStore::update(conn, &id, &input)?;
        JdStore::get_with_org(conn, &id)?.ok_or_else(|| {
            forge_core::ForgeError::Internal("Job description updated but not found".into())
        })
    })
    .await?;
    Ok(Json(ApiData { data: result }))
}

async fn delete_job_description(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<NoContent, ApiError> {
    with_conn(&state, move |conn| JdStore::delete(conn, &id)).await?;
    Ok(NoContent)
}

// ── Lookup by URL ───────────────────────────────────────────────────

/// `POST /job-descriptions/lookup-by-url` (TS job-descriptions.ts:43-58). The static segment
/// outranks `{id}` in axum 0.8 / matchit 0.8, so registration order doesn't matter.
async fn lookup_by_url(
    State(state): State<SharedState>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<ApiData<JobDescriptionWithOrg>>, ApiError> {
    let url = body
        .get("url")
        .and_then(serde_json::Value::as_str)
        .filter(|u| !u.trim().is_empty())
        .map(str::to_owned)
        .ok_or_else(|| ForgeError::Validation {
            message: "URL must be a non-empty string".into(),
            field: Some("url".into()),
        })?;
    let data = with_conn(&state, move |conn| {
        let jd = JdStore::find_by_url(conn, &url)?.ok_or_else(|| ForgeError::NotFound {
            entity_type: "JobDescription".into(),
            id: url.clone(), // TS: "No job description found for URL: <url>"
        })?;
        JdStore::get_with_org(conn, &jd.id)?
            .ok_or_else(|| ForgeError::Internal("JD found by URL but not by id".into()))
    })
    .await?;
    Ok(Json(ApiData { data }))
}

// ── Extract skills (context only) ───────────────────────────────────

/// Verbatim from TS job-descriptions.ts:276-284. `domain` and `certification` aren't skill
/// categories, so they never select a skill; kept for parity.
const CATEGORY_KEYWORDS: &[(&str, &[&str])] = &[
    (
        "language",
        &[
            "python",
            "java",
            "go",
            "rust",
            "typescript",
            "javascript",
            "c++",
            "scala",
            "ruby",
            "kotlin",
        ],
    ),
    (
        "framework",
        &[
            "fastapi",
            "django",
            "flask",
            "react",
            "next.js",
            "express",
            "spring",
            "pytorch",
            "tensorflow",
        ],
    ),
    (
        "tool",
        &[
            "docker",
            "kubernetes",
            "terraform",
            "helm",
            "git",
            "jenkins",
            "grafana",
            "prometheus",
        ],
    ),
    (
        "platform",
        &["aws", "gcp", "azure", "vercel", "heroku", "cloudflare"],
    ),
    (
        "methodology",
        &[
            "agile",
            "scrum",
            "kanban",
            "devops",
            "devsecops",
            "ci/cd",
            "tdd",
            "sre",
        ],
    ),
    (
        "domain",
        &[
            "machine learning",
            "deep learning",
            "nlp",
            "computer vision",
            "distributed systems",
            "security",
        ],
    ),
    (
        "certification",
        &["cka", "ckad", "aws certified", "security+", "cissp"],
    ),
];

/// Verbatim from TS job-descriptions.ts:309.
const EXTRACT_INSTRUCTIONS: &str = "Execute the prompt_template to extract skills from the JD text. For each extracted skill, check existing_skills for a match by name before creating new ones. Call forge_tag_jd_skill (or POST /api/job-descriptions/:id/skills) for each accepted skill.";

/// Categories whose keywords appear in `lower` (already lowercased), plus `other` and
/// `soft_skill`.
fn matched_categories(lower: &str) -> BTreeSet<&'static str> {
    let mut cats: BTreeSet<&'static str> = CATEGORY_KEYWORDS
        .iter()
        .filter(|(_, keywords)| keywords.iter().any(|kw| lower.contains(kw)))
        .map(|(cat, _)| *cat)
        .collect();
    cats.insert("other");
    cats.insert("soft_skill");
    cats
}

/// The SDK's `JDSkillExtractionContext` (packages/sdk/src/types.ts:1741-1746).
#[derive(Debug, Serialize)]
pub struct JdSkillExtractionContext {
    pub jd_raw_text: String,
    pub existing_skills: Vec<Skill>,
    pub prompt_template: String,
    pub instructions: &'static str,
}

/// `POST /job-descriptions/:id/extract-skills` (TS job-descriptions.ts:249-312).
/// Context for client-side extraction: no AI call, no writes.
async fn extract_skills(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<ApiData<JdSkillExtractionContext>>, ApiError> {
    let data = with_conn(&state, move |conn| {
        let jd = JdStore::get(conn, &id)?.ok_or_else(|| ForgeError::NotFound {
            entity_type: "JobDescription".into(),
            id: id.clone(),
        })?;
        if jd.raw_text.trim().is_empty() {
            return Err(ForgeError::Validation {
                message: "Job description has no text to extract skills from".into(),
                field: Some("raw_text".into()),
            });
        }
        let cats = matched_categories(&jd.raw_text.to_lowercase());
        let existing_skills = SkillStore::list(conn, None, None, None)?
            .into_iter()
            .filter(|s| cats.contains(s.category.as_ref()))
            .collect();
        let p = jd_skill_extraction::render(&jd.raw_text);
        // Same join as derivations.rs; byte-equal to TS renderJDSkillExtractionPrompt.
        let prompt_template = format!("{}\n\n{}", p.system, p.user);
        Ok(JdSkillExtractionContext {
            jd_raw_text: jd.raw_text,
            existing_skills,
            prompt_template,
            instructions: EXTRACT_INSTRUCTIONS,
        })
    })
    .await?;
    Ok(Json(ApiData { data }))
}

/// Contacts linked to a job description (TS job-descriptions.ts:241-247). No parent check: an unknown id gives [].
async fn list_job_description_contacts(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<ApiData<Vec<ContactLink>>>, ApiError> {
    let data = with_conn(&state, move |conn| {
        ContactStore::list_by_job_description(conn, &id)
    })
    .await?;
    Ok(Json(ApiData { data }))
}

/// Skills linked to a JD (TS job-descriptions.ts:84-92). No JD check: an unknown id gives [].
async fn list_job_description_skills(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<ApiData<Vec<SkillRow>>>, ApiError> {
    let data = with_conn(&state, move |conn| JdStore::list_skills(conn, &id)).await?;
    Ok(Json(ApiData { data }))
}

/// `POST /job-descriptions/:id/skills` (TS job-descriptions.ts:94-134). `{ skill_id }` links an
/// existing skill; `{ name, category? }` finds or creates one. `skill_id` wins when both are
/// present. Both paths answer 201 with the full skill row, also when the link already existed.
async fn add_job_description_skill(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(body): Json<LinkSkillBody>,
) -> Result<Created<SkillRow>, ApiError> {
    let skill = with_conn(&state, move |conn| {
        if let Some(skill_id) = body.skill_id.as_deref().filter(|s| !s.is_empty()) {
            JdStore::add_skill(conn, &id, skill_id)
        } else if let Some(name) = body.name.as_deref().filter(|n| !n.trim().is_empty()) {
            JdStore::add_skill_by_name(conn, &id, name, body.category.as_deref())
        } else {
            Err(ForgeError::Validation {
                message: "skill_id or name is required".into(),
                field: None,
            })
        }
    })
    .await?;
    Ok(Created(skill))
}

/// `DELETE /job-descriptions/:jd_id/skills/:skill_id` (TS job-descriptions.ts:136-142). Always
/// 204, also when nothing was linked or the JD or skill is unknown. Never deletes the skill.
async fn remove_job_description_skill(
    State(state): State<SharedState>,
    Path((jd_id, skill_id)): Path<(String, String)>,
) -> Result<NoContent, ApiError> {
    with_conn(&state, move |conn| {
        JdStore::remove_skill(conn, &jd_id, &skill_id)
    })
    .await?;
    Ok(NoContent)
}

// ── JD <-> resume links ─────────────────────────────────────────────

/// Body of `POST /job-descriptions/:id/resumes` (TS job-descriptions.ts:221).
#[derive(Serialize)]
struct LinkResumeResponse {
    data: ResumeLink,
    /// TS `RegenerateResult`. Always `null` until resu-mx/forge#71 regenerates the tagline on
    /// link. Never omitted: TS always sends the key.
    tagline: Option<serde_json::Value>,
}

/// `GET /job-descriptions/:id/resumes` (TS job-descriptions.ts:146-166). 404 for an unknown JD
/// (checked first, as in TS); `[]` for a JD with no links.
async fn list_jd_resumes(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<ApiData<Vec<ResumeLink>>>, ApiError> {
    let data = with_conn(&state, move |conn| {
        JdStore::get(conn, &id)?.ok_or_else(|| ForgeError::NotFound {
            entity_type: "JobDescription".into(),
            id: id.clone(),
        })?;
        JdResumeStore::list_by_jd(conn, &id)
    })
    .await?;
    Ok(Json(ApiData { data }))
}

/// `POST /job-descriptions/:id/resumes` (TS job-descriptions.ts:168-222). 201 for a new link,
/// 200 if it already existed.
async fn link_resume(
    State(state): State<SharedState>,
    Path(jd_id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<(StatusCode, Json<LinkResumeResponse>), ApiError> {
    // 1. Body first, as TS does (:173-178).
    let resume_id = body
        .get("resume_id")
        .and_then(serde_json::Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| ForgeError::Validation {
            message: "resume_id is required".into(),
            field: Some("resume_id".into()),
        })?;

    let (created, link) = with_conn(&state, move |conn| {
        // 2. JD, then 3. resume (:180-191).
        JdStore::get(conn, &jd_id)?.ok_or_else(|| ForgeError::NotFound {
            entity_type: "JobDescription".into(),
            id: jd_id.clone(),
        })?;
        ResumeStore::get(conn, &resume_id)?.ok_or_else(|| ForgeError::NotFound {
            entity_type: "Resume".into(),
            id: resume_id.clone(),
        })?;
        // 4. Idempotent insert; `created` picks 201 or 200 (:193-199, :218-219).
        let created = JdResumeStore::link(conn, &jd_id, &resume_id)?;
        // TODO(resu-mx/forge#71): regenerate the resume's generated_tagline from all linked
        // JDs here, for new links and re-links alike, and return it as `tagline`.
        let link = JdResumeStore::get_link(conn, &jd_id, &resume_id)?
            .ok_or_else(|| ForgeError::Internal("JD-resume link written but not found".into()))?;
        Ok((created, link))
    })
    .await?;

    let status = if created {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    Ok((
        status,
        Json(LinkResumeResponse {
            data: link,
            tagline: None,
        }),
    ))
}

/// `DELETE /job-descriptions/:jdId/resumes/:resumeId` (TS job-descriptions.ts:224-239).
/// Always 204: neither id is checked, and a missing link is not an error.
async fn unlink_resume(
    State(state): State<SharedState>,
    Path((jd_id, resume_id)): Path<(String, String)>,
) -> Result<NoContent, ApiError> {
    with_conn(&state, move |conn| {
        JdResumeStore::unlink(conn, &jd_id, &resume_id)?;
        // TODO(resu-mx/forge#71): regenerate generated_tagline from the remaining links
        // (NULL when none remain). Never touch tagline_override.
        Ok(())
    })
    .await?;
    Ok(NoContent)
}

// ── Router ──────────────────────────────────────────────────────────

pub fn router() -> Router<SharedState> {
    Router::new()
        .route(
            "/job-descriptions",
            post(create_job_description).get(list_job_descriptions),
        )
        .route(
            "/job-descriptions/{id}",
            get(get_job_description)
                .patch(update_job_description)
                .delete(delete_job_description),
        )
        .route("/job-descriptions/lookup-by-url", post(lookup_by_url))
        .route(
            "/job-descriptions/{id}/extract-skills",
            post(extract_skills),
        )
        .route(
            "/job-descriptions/{id}/skills",
            get(list_job_description_skills).post(add_job_description_skill),
        )
        .route(
            "/job-descriptions/{jd_id}/skills/{skill_id}",
            delete(remove_job_description_skill),
        )
        .route(
            "/job-descriptions/{id}/resumes",
            get(list_jd_resumes).post(link_resume),
        )
        .route(
            "/job-descriptions/{jd_id}/resumes/{resume_id}",
            axum::routing::delete(unlink_resume),
        )
        .route(
            "/job-descriptions/{id}/contacts",
            get(list_job_description_contacts),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matched_categories_follows_the_ts_table() {
        let set = |s: &str| matched_categories(s).into_iter().collect::<Vec<_>>();
        assert_eq!(
            set("we write rust and ship with docker."),
            ["language", "other", "soft_skill", "tool"]
        );
        assert_eq!(set("machine learning"), ["domain", "other", "soft_skill"]);
        assert_eq!(set(""), ["other", "soft_skill"]);
        // "go" is a substring match, as in TS.
        assert!(matched_categories("a good fit").contains("language"));
    }
}
