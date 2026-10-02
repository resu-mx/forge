//! Summary CRUD routes.
//!
//! Mirrors `packages/core/src/routes/summaries.ts` — same URL paths,
//! same JSON shapes, so the webui and MCP server continue working.

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use forge_core::{
    CreateSummary, SortDirection, Summary, SummaryFilter, SummarySort, SummarySortBy, UpdateSummary,
};
use forge_sdk::db::SummaryStore;

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

async fn get_summary(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<ApiData<Summary>>, ApiError> {
    let result = with_conn(&state, move |conn| {
        SummaryStore::get(conn, &id)?.ok_or_else(|| forge_core::ForgeError::NotFound {
            entity_type: "Summary".into(),
            id: id.clone(),
        })
    })
    .await?;
    Ok(Json(ApiData { data: result }))
}

async fn update_summary(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(input): Json<UpdateSummary>,
) -> Result<Json<ApiData<Summary>>, ApiError> {
    let result =
        with_conn(&state, move |conn| SummaryStore::update(conn, &id, &input)).await?;
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
    let result =
        with_conn(&state, move |conn| SummaryStore::toggle_template(conn, &id)).await?;
    Ok(Json(ApiData { data: result }))
}

async fn clone_summary(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Created<Summary>, ApiError> {
    let result =
        with_conn(&state, move |conn| SummaryStore::clone_summary(conn, &id)).await?;
    Ok(Created(result))
}

// ── Router ──────────────────────────────────────────────────────────

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/summaries", post(create_summary).get(list_summaries))
        .route("/summaries/{id}/toggle-template", post(toggle_template))
        .route("/summaries/{id}/clone", post(clone_summary))
        .route(
            "/summaries/{id}",
            get(get_summary).patch(update_summary).delete(delete_summary),
        )
}
