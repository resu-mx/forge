//! Resume CRUD routes with entry, section, and reorder sub-resources.
//!
//! Mirrors `packages/core/src/routes/resumes.ts`.

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use forge_core::{
    AddResumeEntry, CreateResume, Resume, ResumeEntry, ResumeSectionEntity, ResumeWithEntries,
    UpdateResume,
};
use forge_sdk::db::ResumeStore;

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
    pub entry_id: String,
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

async fn create_resume(
    State(state): State<SharedState>,
    Json(input): Json<CreateResume>,
) -> Result<Created<Resume>, ApiError> {
    let result = with_conn(&state, move |conn| ResumeStore::create(conn, &input)).await?;
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
        ResumeStore::get_with_entries(conn, &id)?.ok_or_else(|| {
            forge_core::ForgeError::NotFound {
                entity_type: "Resume".into(),
                id: id.clone(),
            }
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
    let result =
        with_conn(&state, move |conn| ResumeStore::update(conn, &id, &input)).await?;
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
    let result =
        with_conn(&state, move |conn| ResumeStore::add_entry(conn, &resume_id, &input))
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
) -> Result<NoContent, ApiError> {
    let entries: Vec<(String, String, i32)> = body
        .entries
        .into_iter()
        .map(|e| (e.entry_id, e.section_id, e.position))
        .collect();

    with_conn(&state, move |conn| {
        ResumeStore::reorder_entries(conn, &resume_id, &entries)
    })
    .await?;
    Ok(NoContent)
}

// ── Section sub-resource handlers ──────────────────────────────────

async fn create_section(
    State(state): State<SharedState>,
    Path(resume_id): Path<String>,
    Json(body): Json<CreateSectionBody>,
) -> Result<Created<ResumeSectionEntity>, ApiError> {
    let result = with_conn(&state, move |conn| {
        ResumeStore::create_section(conn, &resume_id, &body.title, &body.entry_type, body.position)
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

// ── Router ──────────────────────────────────────────────────────────

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/resumes", post(create_resume).get(list_resumes))
        .route(
            "/resumes/{id}",
            get(get_resume).patch(update_resume).delete(delete_resume),
        )
        .route("/resumes/{id}/entries", post(add_entry))
        .route(
            "/resumes/{resume_id}/entries/{entry_id}",
            axum::routing::patch(update_entry).delete(remove_entry),
        )
        .route("/resumes/{id}/entries/reorder", post(reorder_entries))
        .route("/resumes/{id}/sections", post(create_section))
        .route(
            "/resumes/{resume_id}/sections/{section_id}",
            axum::routing::delete(delete_section),
        )
}
