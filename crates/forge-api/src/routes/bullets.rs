//! Bullet CRUD + status transition routes.
//!
//! Mirrors `packages/core/src/routes/bullets.ts`.

use axum::extract::{Path, Query, State};
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use serde::Deserialize;

use forge_core::{BulletFilter, BulletStatus, Bullet, PaginationParams, UpdateBulletInput};
use forge_sdk::db::BulletStore;

use crate::db::with_conn;
use crate::error::ApiError;
use crate::response::{ApiData, ApiList, Created, NoContent};
use crate::state::SharedState;

// ── Query params ────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Default)]
pub struct BulletListQuery {
    pub offset: Option<i64>,
    pub limit: Option<i64>,
    pub source_id: Option<String>,
    pub status: Option<String>,
    pub technology: Option<String>,
}

// ── Request bodies ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateBulletBody {
    pub content: String,
    pub source_content_snapshot: Option<String>,
    pub metrics: Option<String>,
    pub domain: Option<String>,
    pub source_ids: Option<Vec<SourceLink>>,
    pub technologies: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct SourceLink {
    pub source_id: String,
    pub is_primary: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct RejectBody {
    pub rejection_reason: Option<String>,
}

// ── Handlers ────────────────────────────────────────────────────────

async fn create_bullet(
    State(state): State<SharedState>,
    Json(body): Json<CreateBulletBody>,
) -> Result<Created<Bullet>, ApiError> {
    let source_ids: Vec<(String, bool)> = body
        .source_ids
        .unwrap_or_default()
        .into_iter()
        .map(|s| (s.source_id, s.is_primary.unwrap_or(false)))
        .collect();
    let technologies = body.technologies.unwrap_or_default();

    let result = with_conn(&state, move |conn| {
        BulletStore::create(
            conn,
            &body.content,
            body.source_content_snapshot.as_deref(),
            body.metrics.as_deref(),
            body.domain.as_deref(),
            &source_ids,
            &technologies,
        )
    })
    .await?;
    Ok(Created(result))
}

async fn list_bullets(
    State(state): State<SharedState>,
    Query(q): Query<BulletListQuery>,
) -> Result<Json<ApiList<Bullet>>, ApiError> {
    let filter = BulletFilter {
        source_id: q.source_id,
        status: q.status,
        technology: q.technology,
        domain: None,
    };
    let pg = PaginationParams {
        offset: Some(q.offset.unwrap_or(0).max(0)),
        limit: Some(q.limit.unwrap_or(50).clamp(1, 200)),
    };

    let (data, pagination) =
        with_conn(&state, move |conn| BulletStore::list(conn, &filter, &pg)).await?;

    Ok(Json(ApiList { data, pagination }))
}

async fn get_bullet(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<ApiData<Bullet>>, ApiError> {
    let result = with_conn(&state, move |conn| {
        BulletStore::get_hydrated(conn, &id)?.ok_or_else(|| forge_core::ForgeError::NotFound {
            entity_type: "Bullet".into(),
            id: id.clone(),
        })
    })
    .await?;
    Ok(Json(ApiData { data: result }))
}

async fn update_bullet(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(input): Json<UpdateBulletInput>,
) -> Result<Json<ApiData<Bullet>>, ApiError> {
    let result =
        with_conn(&state, move |conn| BulletStore::update(conn, &id, &input)).await?;
    Ok(Json(ApiData { data: result }))
}

async fn delete_bullet(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<NoContent, ApiError> {
    with_conn(&state, move |conn| BulletStore::delete(conn, &id)).await?;
    Ok(NoContent)
}

async fn approve_bullet(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<ApiData<Bullet>>, ApiError> {
    let result = with_conn(&state, move |conn| {
        BulletStore::transition_status(conn, &id, BulletStatus::Approved, None)
    })
    .await?;
    Ok(Json(ApiData { data: result }))
}

async fn reject_bullet(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(body): Json<RejectBody>,
) -> Result<Json<ApiData<Bullet>>, ApiError> {
    let reason = body.rejection_reason.unwrap_or_default();
    let result = with_conn(&state, move |conn| {
        BulletStore::transition_status(conn, &id, BulletStatus::Rejected, Some(&reason))
    })
    .await?;
    Ok(Json(ApiData { data: result }))
}

async fn reopen_bullet(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<ApiData<Bullet>>, ApiError> {
    let result = with_conn(&state, move |conn| {
        BulletStore::transition_status(conn, &id, BulletStatus::Draft, None)
    })
    .await?;
    Ok(Json(ApiData { data: result }))
}

async fn submit_bullet(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<ApiData<Bullet>>, ApiError> {
    let result = with_conn(&state, move |conn| {
        BulletStore::transition_status(conn, &id, BulletStatus::InReview, None)
    })
    .await?;
    Ok(Json(ApiData { data: result }))
}

// ── Router ──────────────────────────────────────────────────────────

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/bullets", post(create_bullet).get(list_bullets))
        .route(
            "/bullets/{id}",
            get(get_bullet).patch(update_bullet).delete(delete_bullet),
        )
        .route("/bullets/{id}/approve", patch(approve_bullet))
        .route("/bullets/{id}/reject", patch(reject_bullet))
        .route("/bullets/{id}/reopen", patch(reopen_bullet))
        .route("/bullets/{id}/submit", patch(submit_bullet))
}
