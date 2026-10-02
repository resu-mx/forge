//! Perspective CRUD + status transition routes.
//!
//! Mirrors `packages/core/src/routes/perspectives.ts`.

use axum::extract::{Path, Query, State};
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use serde::Deserialize;

use forge_core::{
    CreatePerspectiveInput, Framing, PaginationParams, Perspective, PerspectiveFilter,
    PerspectiveStatus, PerspectiveWithChain, UpdatePerspectiveInput,
};
use forge_sdk::db::PerspectiveStore;

use crate::db::with_conn;
use crate::error::ApiError;
use crate::response::{ApiData, ApiList, Created, NoContent};
use crate::state::SharedState;

// ── Query params ────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Default)]
pub struct PerspectiveListQuery {
    pub offset: Option<i64>,
    pub limit: Option<i64>,
    pub bullet_id: Option<String>,
    pub target_archetype: Option<String>,
    pub archetype: Option<String>,
    pub domain: Option<String>,
    pub framing: Option<String>,
    pub status: Option<String>,
    pub source_id: Option<String>,
    pub search: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RejectBody {
    pub rejection_reason: Option<String>,
}

// ── Handlers ────────────────────────────────────────────────────────

async fn create_perspective(
    State(state): State<SharedState>,
    Json(input): Json<CreatePerspectiveInput>,
) -> Result<Created<Perspective>, ApiError> {
    let result =
        with_conn(&state, move |conn| PerspectiveStore::create(conn, &input)).await?;
    Ok(Created(result))
}

async fn list_perspectives(
    State(state): State<SharedState>,
    Query(q): Query<PerspectiveListQuery>,
) -> Result<Json<ApiList<Perspective>>, ApiError> {
    let filter = PerspectiveFilter {
        bullet_id: q.bullet_id,
        target_archetype: q.target_archetype.or(q.archetype),
        domain: q.domain,
        framing: q.framing.and_then(|s| s.parse::<Framing>().ok()),
        status: q.status.and_then(|s| s.parse::<PerspectiveStatus>().ok()),
        source_id: q.source_id,
        search: q.search,
    };
    let pg = PaginationParams {
        offset: Some(q.offset.unwrap_or(0).max(0)),
        limit: Some(q.limit.unwrap_or(50).clamp(1, 200)),
    };

    let (data, pagination) =
        with_conn(&state, move |conn| PerspectiveStore::list(conn, &filter, &pg)).await?;

    Ok(Json(ApiList { data, pagination }))
}

async fn get_perspective(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<ApiData<PerspectiveWithChain>>, ApiError> {
    let result = with_conn(&state, move |conn| {
        PerspectiveStore::get_with_chain(conn, &id)?.ok_or_else(|| {
            forge_core::ForgeError::NotFound {
                entity_type: "Perspective".into(),
                id: id.clone(),
            }
        })
    })
    .await?;
    Ok(Json(ApiData { data: result }))
}

async fn update_perspective(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(input): Json<UpdatePerspectiveInput>,
) -> Result<Json<ApiData<Perspective>>, ApiError> {
    let result =
        with_conn(&state, move |conn| PerspectiveStore::update(conn, &id, &input)).await?;
    Ok(Json(ApiData { data: result }))
}

async fn delete_perspective(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<NoContent, ApiError> {
    with_conn(&state, move |conn| PerspectiveStore::delete(conn, &id)).await?;
    Ok(NoContent)
}

async fn approve_perspective(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<ApiData<Perspective>>, ApiError> {
    let result = with_conn(&state, move |conn| {
        PerspectiveStore::transition_status(conn, &id, PerspectiveStatus::Approved, None)
    })
    .await?;
    Ok(Json(ApiData { data: result }))
}

async fn reject_perspective(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(body): Json<RejectBody>,
) -> Result<Json<ApiData<Perspective>>, ApiError> {
    let reason = body.rejection_reason.unwrap_or_default();
    let result = with_conn(&state, move |conn| {
        PerspectiveStore::transition_status(
            conn,
            &id,
            PerspectiveStatus::Rejected,
            Some(&reason),
        )
    })
    .await?;
    Ok(Json(ApiData { data: result }))
}

async fn reopen_perspective(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<ApiData<Perspective>>, ApiError> {
    let result = with_conn(&state, move |conn| {
        PerspectiveStore::transition_status(conn, &id, PerspectiveStatus::Draft, None)
    })
    .await?;
    Ok(Json(ApiData { data: result }))
}

// ── Router ──────────────────────────────────────────────────────────

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/perspectives", post(create_perspective).get(list_perspectives))
        .route(
            "/perspectives/{id}",
            get(get_perspective)
                .patch(update_perspective)
                .delete(delete_perspective),
        )
        .route("/perspectives/{id}/approve", patch(approve_perspective))
        .route("/perspectives/{id}/reject", patch(reject_perspective))
        .route("/perspectives/{id}/reopen", patch(reopen_perspective))
}
