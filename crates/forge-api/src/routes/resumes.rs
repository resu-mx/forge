//! Resume CRUD routes with entry, section, and reorder sub-resources.
//!
//! Mirrors `packages/core/src/routes/resumes.ts`.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use serde::Deserialize;

use forge_core::{
    AddResumeCertification, AddResumeEntry, ContactLink, CreateResume, GapAnalysis, Resume,
    ResumeCertification, ResumeEntry, ResumeSectionEntity, ResumeSkill, ResumeTemplate,
    ResumeWithEntries, UpdateResume,
};
use forge_sdk::db::{ContactStore, ResumeStore, TemplateStore};
use forge_sdk::services::tagline;

use crate::db::with_conn;
use crate::error::ApiError;
use crate::response::{ApiData, ApiList, Created, NoContent};
use crate::state::SharedState;

// ── Query params ────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Default)]
pub struct ResumeListQuery {
    pub offset: Option<i64>,
    pub limit: Option<i64>,
}

// ── Request bodies ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct UpdateEntryBody {
    #[serde(default, deserialize_with = "forge_core::serde_util::double_option")]
    pub content: Option<Option<String>>,
    pub section_id: Option<String>,
    pub position: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct ReorderEntriesBody {
    pub entries: Vec<ReorderEntryItem>,
}

#[derive(Debug, Deserialize)]
pub struct ReorderEntryItem {
    pub id: String,
    pub section_id: String,
    pub position: i32,
}

#[derive(Debug, Deserialize)]
pub struct CreateSectionBody {
    pub title: String,
    pub entry_type: String,
    pub position: Option<i32>,
}

// ── Handlers ────────────────────────────────────────────────────────

/// `POST /resumes` body: a plain `CreateResume`, optionally with `template_id` to
/// pre-populate sections from a template (matches the TS route).
#[derive(Debug, serde::Deserialize)]
struct CreateResumeBody {
    #[serde(flatten)]
    input: CreateResume,
    template_id: Option<String>,
}

async fn create_resume(
    State(state): State<SharedState>,
    Json(body): Json<CreateResumeBody>,
) -> Result<Created<Resume>, ApiError> {
    let result = with_conn(&state, move |conn| match &body.template_id {
        Some(template_id) => forge_sdk::db::TemplateStore::create_resume_from_template(
            conn,
            template_id,
            &body.input,
        ),
        None => ResumeStore::create(conn, &body.input),
    })
    .await?;
    Ok(Created(result))
}

async fn list_resumes(
    State(state): State<SharedState>,
    Query(q): Query<ResumeListQuery>,
) -> Result<Json<ApiList<Resume>>, ApiError> {
    let offset = q.offset.unwrap_or(0).max(0);
    let limit = q.limit.unwrap_or(50).clamp(1, 200);

    let (data, pagination) =
        with_conn(&state, move |conn| ResumeStore::list(conn, offset, limit)).await?;

    Ok(Json(ApiList { data, pagination }))
}

async fn get_resume(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<ApiData<ResumeWithEntries>>, ApiError> {
    let result = with_conn(&state, move |conn| {
        ResumeStore::get_with_entries(conn, &id)?.ok_or_else(|| forge_core::ForgeError::NotFound {
            entity_type: "Resume".into(),
            id: id.clone(),
        })
    })
    .await?;
    Ok(Json(ApiData { data: result }))
}

async fn update_resume(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(input): Json<UpdateResume>,
) -> Result<Json<ApiData<Resume>>, ApiError> {
    let result = with_conn(&state, move |conn| ResumeStore::update(conn, &id, &input)).await?;
    Ok(Json(ApiData { data: result }))
}

async fn delete_resume(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<NoContent, ApiError> {
    with_conn(&state, move |conn| ResumeStore::delete(conn, &id)).await?;
    Ok(NoContent)
}

// ── Entry sub-resource handlers ────────────────────────────────────

async fn add_entry(
    State(state): State<SharedState>,
    Path(resume_id): Path<String>,
    Json(input): Json<AddResumeEntry>,
) -> Result<Created<ResumeEntry>, ApiError> {
    let result = with_conn(&state, move |conn| {
        ResumeStore::add_entry(conn, &resume_id, &input)
    })
    .await?;
    Ok(Created(result))
}

async fn update_entry(
    State(state): State<SharedState>,
    Path((resume_id, entry_id)): Path<(String, String)>,
    Json(body): Json<UpdateEntryBody>,
) -> Result<Json<ApiData<ResumeEntry>>, ApiError> {
    let result = with_conn(&state, move |conn| {
        ResumeStore::update_entry(
            conn,
            &resume_id,
            &entry_id,
            body.content,
            body.section_id,
            body.position,
        )
    })
    .await?;
    Ok(Json(ApiData { data: result }))
}

async fn remove_entry(
    State(state): State<SharedState>,
    Path((resume_id, entry_id)): Path<(String, String)>,
) -> Result<NoContent, ApiError> {
    with_conn(&state, move |conn| {
        ResumeStore::remove_entry(conn, &resume_id, &entry_id)
    })
    .await?;
    Ok(NoContent)
}

async fn reorder_entries(
    State(state): State<SharedState>,
    Path(resume_id): Path<String>,
    Json(body): Json<ReorderEntriesBody>,
) -> Result<Json<ApiData<Option<()>>>, ApiError> {
    let entries: Vec<(String, String, i32)> = body
        .entries
        .into_iter()
        .map(|e| (e.id, e.section_id, e.position))
        .collect();

    with_conn(&state, move |conn| {
        ResumeStore::reorder_entries(conn, &resume_id, &entries)
    })
    .await?;
    Ok(Json(ApiData { data: None }))
}

// ── Section sub-resource handlers ──────────────────────────────────

async fn create_section(
    State(state): State<SharedState>,
    Path(resume_id): Path<String>,
    Json(body): Json<CreateSectionBody>,
) -> Result<Created<ResumeSectionEntity>, ApiError> {
    let result = with_conn(&state, move |conn| {
        ResumeStore::create_section(
            conn,
            &resume_id,
            &body.title,
            &body.entry_type,
            body.position,
        )
    })
    .await?;
    Ok(Created(result))
}

async fn delete_section(
    State(state): State<SharedState>,
    Path((resume_id, section_id)): Path<(String, String)>,
) -> Result<NoContent, ApiError> {
    with_conn(&state, move |conn| {
        ResumeStore::delete_section(conn, &resume_id, &section_id)
    })
    .await?;
    Ok(NoContent)
}

async fn list_sections(
    State(state): State<SharedState>,
    Path(resume_id): Path<String>,
) -> Result<Json<ApiData<Vec<ResumeSectionEntity>>>, ApiError> {
    let data = with_conn(&state, move |conn| {
        // 404 for an unknown resume rather than an empty list.
        ResumeStore::get(conn, &resume_id)?.ok_or_else(|| forge_core::ForgeError::NotFound {
            entity_type: "resume".into(),
            id: resume_id.clone(),
        })?;
        ResumeStore::list_sections(conn, &resume_id)
    })
    .await?;
    Ok(Json(ApiData { data }))
}

#[derive(Debug, Deserialize)]
pub struct UpdateSectionBody {
    pub title: Option<String>,
    pub position: Option<i32>,
}

async fn update_section(
    State(state): State<SharedState>,
    Path((resume_id, section_id)): Path<(String, String)>,
    Json(body): Json<UpdateSectionBody>,
) -> Result<Json<ApiData<ResumeSectionEntity>>, ApiError> {
    let data = with_conn(&state, move |conn| {
        ResumeStore::update_section(
            conn,
            &resume_id,
            &section_id,
            body.title.as_deref(),
            body.position,
        )
    })
    .await?;
    Ok(Json(ApiData { data }))
}

// ── Section skills ─────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AddSkillBody {
    pub skill_id: String,
}

async fn add_section_skill(
    State(state): State<SharedState>,
    Path((resume_id, section_id)): Path<(String, String)>,
    Json(body): Json<AddSkillBody>,
) -> Result<Created<ResumeSkill>, ApiError> {
    let data = with_conn(&state, move |conn| {
        ResumeStore::add_skill(conn, &resume_id, &section_id, &body.skill_id)
    })
    .await?;
    Ok(Created(data))
}

async fn list_section_skills(
    State(state): State<SharedState>,
    Path((resume_id, section_id)): Path<(String, String)>,
) -> Result<Json<ApiData<Vec<ResumeSkill>>>, ApiError> {
    let data = with_conn(&state, move |conn| {
        ResumeStore::list_skills_for_section(conn, &resume_id, &section_id)
    })
    .await?;
    Ok(Json(ApiData { data }))
}

async fn remove_section_skill(
    State(state): State<SharedState>,
    Path((resume_id, section_id, skill_id)): Path<(String, String, String)>,
) -> Result<NoContent, ApiError> {
    with_conn(&state, move |conn| {
        ResumeStore::remove_skill(conn, &resume_id, &section_id, &skill_id)
    })
    .await?;
    Ok(NoContent)
}

#[derive(Debug, Deserialize)]
pub struct ReorderSkillsBody {
    pub skills: Vec<ReorderSkillItem>,
}

#[derive(Debug, Deserialize)]
pub struct ReorderSkillItem {
    pub skill_id: String,
    pub position: i32,
}

async fn reorder_section_skills(
    State(state): State<SharedState>,
    Path((resume_id, section_id)): Path<(String, String)>,
    Json(body): Json<ReorderSkillsBody>,
) -> Result<Json<ApiData<Option<()>>>, ApiError> {
    let skills: Vec<(String, i32)> = body
        .skills
        .into_iter()
        .map(|s| (s.skill_id, s.position))
        .collect();
    with_conn(&state, move |conn| {
        ResumeStore::reorder_skills(conn, &resume_id, &section_id, &skills)
    })
    .await?;
    Ok(Json(ApiData { data: None }))
}

// ── Certifications ─────────────────────────────────────────────────

async fn add_resume_certification(
    State(state): State<SharedState>,
    Path(resume_id): Path<String>,
    Json(input): Json<AddResumeCertification>,
) -> Result<Created<ResumeCertification>, ApiError> {
    let data = with_conn(&state, move |conn| {
        ResumeStore::add_certification(conn, &resume_id, &input)
    })
    .await?;
    Ok(Created(data))
}

async fn list_resume_certifications(
    State(state): State<SharedState>,
    Path(resume_id): Path<String>,
) -> Result<Json<ApiData<Vec<ResumeCertification>>>, ApiError> {
    let data = with_conn(&state, move |conn| {
        ResumeStore::list_certifications(conn, &resume_id)
    })
    .await?;
    Ok(Json(ApiData { data }))
}

async fn remove_resume_certification(
    State(state): State<SharedState>,
    Path((resume_id, rc_id)): Path<(String, String)>,
) -> Result<NoContent, ApiError> {
    with_conn(&state, move |conn| {
        ResumeStore::remove_certification(conn, &resume_id, &rc_id)
    })
    .await?;
    Ok(NoContent)
}

// ── Templates, gaps, IR, header, overrides, PDF ────────────────────

#[derive(Debug, Deserialize)]
pub struct SaveAsTemplateBody {
    pub name: String,
    pub description: Option<String>,
}

async fn save_as_template(
    State(state): State<SharedState>,
    Path(resume_id): Path<String>,
    Json(body): Json<SaveAsTemplateBody>,
) -> Result<Created<ResumeTemplate>, ApiError> {
    let data = with_conn(&state, move |conn| {
        TemplateStore::save_as_template(conn, &resume_id, &body.name, body.description.as_deref())
    })
    .await?;
    Ok(Created(data))
}

async fn analyze_gaps(
    State(state): State<SharedState>,
    Path(resume_id): Path<String>,
) -> Result<Json<ApiData<GapAnalysis>>, ApiError> {
    let data = with_conn(&state, move |conn| {
        ResumeStore::analyze_gaps(conn, &resume_id)
    })
    .await?;
    Ok(Json(ApiData { data }))
}

/// The resume's intermediate representation (what every renderer consumes).
async fn get_ir(
    State(state): State<SharedState>,
    Path(resume_id): Path<String>,
) -> Result<Json<ApiData<forge_core::ResumeDocument>>, ApiError> {
    let data = with_conn(&state, move |conn| {
        forge_sdk::services::CompilerService::compile(conn, &resume_id)?.ok_or_else(|| {
            forge_core::ForgeError::NotFound {
                entity_type: "resume".into(),
                id: resume_id.clone(),
            }
        })
    })
    .await?;
    Ok(Json(ApiData { data }))
}

async fn update_header(
    State(state): State<SharedState>,
    Path(resume_id): Path<String>,
    Json(header): Json<serde_json::Value>,
) -> Result<Json<ApiData<Resume>>, ApiError> {
    let data = with_conn(&state, move |conn| {
        ResumeStore::update_header(conn, &resume_id, &header)
    })
    .await?;
    Ok(Json(ApiData { data }))
}

/// `{ "content": string | null }` — null clears the override.
#[derive(Debug, Deserialize)]
pub struct OverrideBody {
    pub content: Option<String>,
}

async fn update_markdown_override(
    State(state): State<SharedState>,
    Path(resume_id): Path<String>,
    Json(body): Json<OverrideBody>,
) -> Result<Json<ApiData<Resume>>, ApiError> {
    let data = with_conn(&state, move |conn| {
        ResumeStore::update_markdown_override(conn, &resume_id, body.content.as_deref())
    })
    .await?;
    Ok(Json(ApiData { data }))
}

async fn update_latex_override(
    State(state): State<SharedState>,
    Path(resume_id): Path<String>,
    Json(body): Json<OverrideBody>,
) -> Result<Json<ApiData<Resume>>, ApiError> {
    let data = with_conn(&state, move |conn| {
        ResumeStore::update_latex_override(conn, &resume_id, body.content.as_deref())
    })
    .await?;
    Ok(Json(ApiData { data }))
}

/// `POST /resumes/:id/pdf`. The body is optional; `{ "typst": "..." }` compiles that source
/// instead of the generated one (the old `{ "latex": ... }` is ignored: LaTeX is not compiled).
async fn resume_pdf(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    body: axum::body::Bytes,
) -> Result<axum::response::Response, ApiError> {
    let supplied: Option<String> = serde_json::from_slice::<serde_json::Value>(&body)
        .ok()
        .and_then(|v| v.get("typst").and_then(|s| s.as_str().map(str::to_string)));

    let rendered = with_conn(&state, move |conn| super::pdf::resume_typst(conn, &id)).await?;
    let source = supplied.as_deref().unwrap_or(&rendered.source);
    // A hand-supplied source replaces the generated one, so the LaTeX notice no longer applies.
    let notice = if supplied.is_some() {
        None
    } else {
        rendered.notice
    };
    Ok(super::pdf::pdf_response(
        source,
        "inline; filename=\"resume.pdf\"",
        notice,
    ))
}

/// Contacts linked to a resume (TS resumes.ts:366-372). No parent check: an unknown id gives [].
async fn list_resume_contacts(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<ApiData<Vec<ContactLink>>>, ApiError> {
    let data = with_conn(&state, move |conn| ContactStore::list_by_resume(conn, &id)).await?;
    Ok(Json(ApiData { data }))
}

/// The tagline routes answer a missing resume with TS's literal envelope
/// (`packages/core/src/routes/resumes.ts:232-237`, `:256-261`, `:276-281`), not
/// `ForgeError::NotFound`'s "resume not found: <id>". Not "Route not found", so the browser
/// runtime does not rewrite it to 501.
pub(crate) fn resume_not_found() -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(
            serde_json::json!({ "error": { "code": "NOT_FOUND", "message": "Resume not found" } }),
        ),
    )
        .into_response()
}

/// `GET /resumes/:id/tagline`. Read-only.
async fn get_tagline(
    State(state): State<SharedState>,
    Path(resume_id): Path<String>,
) -> Result<Response, ApiError> {
    let found = with_conn(&state, move |conn| tagline::get_state(conn, &resume_id)).await?;
    Ok(match found {
        Some(data) => Json(ApiData { data }).into_response(),
        None => resume_not_found(),
    })
}

/// `PATCH /resumes/:id/tagline-override` with `{ "content": string | null }`. It answers with
/// the tagline state (the GET shape), not the resume row that the other override routes return.
async fn update_tagline_override(
    State(state): State<SharedState>,
    Path(resume_id): Path<String>,
    Json(body): Json<OverrideBody>,
) -> Result<Response, ApiError> {
    let found = with_conn(&state, move |conn| {
        tagline::set_override(conn, &resume_id, body.content.as_deref())
    })
    .await?;
    Ok(match found {
        Some(data) => Json(ApiData { data }).into_response(),
        None => resume_not_found(),
    })
}

// ── Router ──────────────────────────────────────────────────────────

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/resumes", post(create_resume).get(list_resumes))
        .route(
            "/resumes/{id}",
            get(get_resume).patch(update_resume).delete(delete_resume),
        )
        .route("/resumes/{id}/entries", post(add_entry))
        // Static `reorder` wins over the `{entry_id}` parameter below.
        .route("/resumes/{id}/entries/reorder", patch(reorder_entries))
        .route(
            "/resumes/{resume_id}/entries/{entry_id}",
            patch(update_entry).delete(remove_entry),
        )
        .route(
            "/resumes/{id}/sections",
            post(create_section).get(list_sections),
        )
        .route(
            "/resumes/{resume_id}/sections/{section_id}",
            patch(update_section).delete(delete_section),
        )
        .route(
            "/resumes/{resume_id}/sections/{section_id}/skills",
            post(add_section_skill).get(list_section_skills),
        )
        .route(
            "/resumes/{resume_id}/sections/{section_id}/skills/reorder",
            patch(reorder_section_skills),
        )
        .route(
            "/resumes/{resume_id}/sections/{section_id}/skills/{skill_id}",
            axum::routing::delete(remove_section_skill),
        )
        .route(
            "/resumes/{id}/certifications",
            post(add_resume_certification).get(list_resume_certifications),
        )
        .route(
            "/resumes/{resume_id}/certifications/{rc_id}",
            axum::routing::delete(remove_resume_certification),
        )
        .route("/resumes/{id}/save-as-template", post(save_as_template))
        .route("/resumes/{id}/gaps", get(analyze_gaps))
        .route("/resumes/{id}/ir", get(get_ir))
        .route("/resumes/{id}/header", patch(update_header))
        .route(
            "/resumes/{id}/markdown-override",
            patch(update_markdown_override),
        )
        .route("/resumes/{id}/latex-override", patch(update_latex_override))
        .route("/resumes/{id}/pdf", post(resume_pdf))
        .route("/resumes/{id}/contacts", get(list_resume_contacts))
        .route("/resumes/{id}/tagline", get(get_tagline))
        .route(
            "/resumes/{id}/tagline-override",
            patch(update_tagline_override),
        )
}
