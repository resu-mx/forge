//! Job Description CRUD routes.
//!
//! Mirrors the TS job-description routes — same URL paths,
//! same JSON shapes, so the webui and MCP server continue working.

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use forge_core::{
    CreateJobDescription, JobDescriptionFilter, JobDescriptionStatus, JobDescriptionWithOrg,
    UpdateJobDescription,
};
use forge_sdk::db::JdStore;

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
        JdStore::get_with_org(conn, &jd.id)?
            .ok_or_else(|| forge_core::ForgeError::Internal("Job description created but not found".into()))
    })
    .await?;
    Ok(Created(result))
}

async fn list_job_descriptions(
    State(state): State<SharedState>,
    Query(q): Query<JdListQuery>,
) -> Result<Json<ApiList<JobDescriptionWithOrg>>, ApiError> {
    let filter = JobDescriptionFilter {
        status: q.status.and_then(|s| s.parse::<JobDescriptionStatus>().ok()),
        organization_id: q.organization_id,
    };
    let offset = q.offset.unwrap_or(0).max(0);
    let limit = q.limit.unwrap_or(50).clamp(1, 200);

    let (data, pagination) =
        with_conn(&state, move |conn| JdStore::list_with_org(conn, &filter, offset, limit)).await?;

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
}
