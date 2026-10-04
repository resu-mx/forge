//! Summary CRUD routes.
//!
//! Mirrors `packages/core/src/routes/summaries.ts` — same URL paths,
//! same JSON shapes, so the webui and MCP server continue working.

use axum::extract::{Path, Query, State};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use forge_core::{
    CreateSummary, ForgeError, Resume, Skill, SortDirection, Summary, SummaryFilter, SummarySort,
    SummarySortBy, SummaryWithRelations, UpdateSummary,
};
use forge_sdk::db::{IndustryStore, RoleTypeStore, SummaryStore};

use crate::db::with_conn;
use crate::error::ApiError;
use crate::response::{ApiData, ApiList, Created, NoContent};
use crate::state::SharedState;

// ── Query params ────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Default)]
pub struct SummaryListQuery {
    pub offset: Option<i64>,
    pub limit: Option<i64>,
    pub is_template: Option<i32>,
    pub industry_id: Option<String>,
    pub role_type_id: Option<String>,
    pub skill_id: Option<String>,
    pub search: Option<String>,
    pub sort_by: Option<String>,
    pub direction: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct LinkedResumesQuery {
    pub offset: Option<i64>,
    pub limit: Option<i64>,
}

/// `GET /summaries/:id` query. Only `include=relations` means anything, as in TS.
#[derive(Debug, Deserialize, Default)]
pub struct SummaryGetQuery {
    pub include: Option<String>,
}

/// `GET /summaries/:id` answers one of two shapes; `untagged` serializes the inner value as-is.
#[derive(Serialize)]
#[serde(untagged)]
enum SummaryBody {
    Plain(Summary),
    Hydrated(SummaryWithRelations),
}

#[derive(Debug, Deserialize)]
pub struct AddSummarySkillBody {
    pub skill_id: Option<String>,
}

// ── Handlers ────────────────────────────────────────────────────────

async fn create_summary(
    State(state): State<SharedState>,
    Json(input): Json<CreateSummary>,
) -> Result<Created<Summary>, ApiError> {
    let result = with_conn(&state, move |conn| SummaryStore::create(conn, &input)).await?;
    Ok(Created(result))
}

async fn list_summaries(
    State(state): State<SharedState>,
    Query(q): Query<SummaryListQuery>,
) -> Result<Json<ApiList<Summary>>, ApiError> {
    let filter = SummaryFilter {
        is_template: q.is_template,
        industry_id: q.industry_id,
        role_type_id: q.role_type_id,
        skill_id: q.skill_id,
        search: q.search,
    };
    let sort = SummarySort {
        sort_by: q.sort_by.and_then(|s| s.parse::<SummarySortBy>().ok()),
        direction: q.direction.and_then(|s| s.parse::<SortDirection>().ok()),
    };
    let offset = q.offset.unwrap_or(0).max(0);
    let limit = q.limit.unwrap_or(50).clamp(1, 200);

    let (data, pagination) = with_conn(&state, move |conn| {
        SummaryStore::list(conn, Some(&filter), Some(&sort), offset, limit)
    })
    .await?;

    Ok(Json(ApiList { data, pagination }))
}

fn summary_not_found(id: &str) -> ForgeError {
    ForgeError::NotFound { entity_type: "Summary".into(), id: id.to_string() }
}

/// Mirrors `SummaryService.getWithRelations` (packages/core/src/services/summary-service.ts:78-99):
/// a missing industry or role type is `null`, not an error.
fn get_with_relations(conn: &Connection, id: &str) -> Result<SummaryWithRelations, ForgeError> {
    let base = SummaryStore::get(conn, id)?.ok_or_else(|| summary_not_found(id))?;
    let industry = match base.industry_id.as_deref() {
        Some(industry_id) => IndustryStore::get(conn, industry_id)?,
        None => None,
    };
    let role_type = match base.role_type_id.as_deref() {
        Some(role_type_id) => RoleTypeStore::get(conn, role_type_id)?,
        None => None,
    };
    let skills = SummaryStore::get_skills(conn, id)?;
    Ok(SummaryWithRelations { base, industry, role_type, skills })
}

async fn get_summary(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Query(q): Query<SummaryGetQuery>,
) -> Result<Json<ApiData<SummaryBody>>, ApiError> {
    let hydrate = q.include.as_deref() == Some("relations");
    let data = with_conn(&state, move |conn| {
        if hydrate {
            get_with_relations(conn, &id).map(SummaryBody::Hydrated)
        } else {
            SummaryStore::get(conn, &id)?
                .map(SummaryBody::Plain)
                .ok_or_else(|| summary_not_found(&id))
        }
    })
    .await?;
    Ok(Json(ApiData { data }))
}

async fn update_summary(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(input): Json<UpdateSummary>,
) -> Result<Json<ApiData<Summary>>, ApiError> {
    let result = with_conn(&state, move |conn| SummaryStore::update(conn, &id, &input)).await?;
    Ok(Json(ApiData { data: result }))
}

async fn delete_summary(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<NoContent, ApiError> {
    with_conn(&state, move |conn| SummaryStore::delete(conn, &id)).await?;
    Ok(NoContent)
}

async fn toggle_template(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<ApiData<Summary>>, ApiError> {
    let result = with_conn(&state, move |conn| SummaryStore::toggle_template(conn, &id)).await?;
    Ok(Json(ApiData { data: result }))
}

async fn clone_summary(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Created<Summary>, ApiError> {
    let result = with_conn(&state, move |conn| SummaryStore::clone_summary(conn, &id)).await?;
    Ok(Created(result))
}

/// The store methods don't check the summary; TS does, first
/// (summary-service.ts:329-332, :359-365, :408-414).
fn require_summary(conn: &Connection, id: &str) -> Result<(), ForgeError> {
    match SummaryStore::get(conn, id)? {
        Some(_) => Ok(()),
        None => Err(ForgeError::NotFound { entity_type: "Summary".into(), id: id.into() }),
    }
}

/// `GET /summaries/:id/linked-resumes` (TS summaries.ts:64-71): newest `updated_at` first.
async fn list_linked_resumes(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Query(q): Query<LinkedResumesQuery>,
) -> Result<Json<ApiList<Resume>>, ApiError> {
    let offset = q.offset.unwrap_or(0).max(0);
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let (data, pagination) = with_conn(&state, move |conn| {
        require_summary(conn, &id)?;
        SummaryStore::list_linked_resumes(conn, &id, offset, limit)
    })
    .await?;
    Ok(Json(ApiList { data, pagination }))
}

/// `GET /summaries/:id/skills` (TS summaries.ts:76-80).
async fn list_summary_skills(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<ApiData<Vec<Skill>>>, ApiError> {
    let data = with_conn(&state, move |conn| {
        require_summary(conn, &id)?;
        SummaryStore::get_skills(conn, &id)
    })
    .await?;
    Ok(Json(ApiData { data }))
}

/// `POST /summaries/:id/skills` (TS summaries.ts:82-90). 204, not 201, as TS answers.
async fn add_summary_skill(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(body): Json<AddSummarySkillBody>,
) -> Result<NoContent, ApiError> {
    // Checked before the summary, as in TS.
    let skill_id = body
        .skill_id
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ForgeError::Validation {
            message: "skill_id is required".into(),
            field: Some("skill_id".into()),
        })?;
    with_conn(&state, move |conn| {
        require_summary(conn, &id)?;
        // An unknown skill fails the FK; the store maps that to NotFound.
        SummaryStore::add_skill(conn, &id, &skill_id)
    })
    .await?;
    Ok(NoContent)
}

/// `DELETE /summaries/:id/skills/:skillId` (TS summaries.ts:92-96). Idempotent, no summary check.
async fn remove_summary_skill(
    State(state): State<SharedState>,
    Path((id, skill_id)): Path<(String, String)>,
) -> Result<NoContent, ApiError> {
    with_conn(&state, move |conn| SummaryStore::remove_skill(conn, &id, &skill_id)).await?;
    Ok(NoContent)
}

// ── Router ──────────────────────────────────────────────────────────

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/summaries", post(create_summary).get(list_summaries))
        .route("/summaries/{id}/toggle-template", post(toggle_template))
        .route("/summaries/{id}/clone", post(clone_summary))
        .route("/summaries/{id}/linked-resumes", get(list_linked_resumes))
        .route("/summaries/{id}/skills", get(list_summary_skills).post(add_summary_skill))
        .route("/summaries/{id}/skills/{skill_id}", delete(remove_summary_skill))
        .route(
            "/summaries/{id}",
            get(get_summary)
                .patch(update_summary)
                .delete(delete_summary),
        )
}
