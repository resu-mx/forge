//! Source CRUD routes.
//!
//! Mirrors `packages/core/src/routes/sources.ts` — same URL paths,
//! same JSON shapes, so the webui and MCP server continue working.

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use forge_core::{
    CreateSource, PaginationParams, SourceFilter, SourceType, SourceWithExtension, UpdateSource,
};
use forge_sdk::db::SourceStore;

use crate::db::with_conn;
use crate::error::ApiError;
use crate::response::{ApiData, ApiList, Created, NoContent};
use crate::state::SharedState;

// ── Query params ────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Default)]
pub struct SourceListQuery {
    pub offset: Option<i64>,
    pub limit: Option<i64>,
    pub source_type: Option<String>,
    pub organization_id: Option<String>,
    pub status: Option<String>,
    pub education_type: Option<String>,
    pub search: Option<String>,
}

// ── Handlers ────────────────────────────────────────────────────────

async fn create_source(
    State(state): State<SharedState>,
    Json(input): Json<CreateSource>,
) -> Result<Created<SourceWithExtension>, ApiError> {
    let result = with_conn(&state, move |conn| SourceStore::create(conn, &input)).await?;
    Ok(Created(result))
}

async fn list_sources(
    State(state): State<SharedState>,
    Query(q): Query<SourceListQuery>,
) -> Result<Json<ApiList<SourceWithExtension>>, ApiError> {
    let filter = SourceFilter {
        source_type: q.source_type.and_then(|s| s.parse::<SourceType>().ok()),
        organization_id: q.organization_id,
        status: q.status.and_then(|s| s.parse().ok()),
        education_type: q.education_type,
        search: q.search,
    };
    let pg = PaginationParams {
        offset: Some(q.offset.unwrap_or(0).max(0)),
        limit: Some(q.limit.unwrap_or(50).clamp(1, 200)),
    };

    let (data, pagination) =
        with_conn(&state, move |conn| SourceStore::list(conn, &filter, &pg)).await?;

    Ok(Json(ApiList { data, pagination }))
}

async fn get_source(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<ApiData<SourceWithExtension>>, ApiError> {
    let result = with_conn(&state, move |conn| {
        SourceStore::get_hydrated(conn, &id)?
            .ok_or_else(|| forge_core::ForgeError::NotFound {
                entity_type: "Source".into(),
                id: id.clone(),
            })
    })
    .await?;
    Ok(Json(ApiData { data: result }))
}

async fn update_source(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(input): Json<UpdateSource>,
) -> Result<Json<ApiData<SourceWithExtension>>, ApiError> {
    let result =
        with_conn(&state, move |conn| SourceStore::update(conn, &id, &input)).await?;
    Ok(Json(ApiData { data: result }))
}

async fn delete_source(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<NoContent, ApiError> {
    with_conn(&state, move |conn| SourceStore::delete(conn, &id)).await?;
    Ok(NoContent)
}

// ── Router ──────────────────────────────────────────────────────────

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/sources", post(create_source).get(list_sources))
        .route(
            "/sources/{id}",
            get(get_source).patch(update_source).delete(delete_source),
        )
}
