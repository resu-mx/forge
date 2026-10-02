//! Organization CRUD routes.
//!
//! Mirrors the TS organization routes.

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use forge_core::{CreateOrganizationInput, Organization, OrganizationFilter};
use forge_sdk::db::OrganizationStore;

use crate::db::with_conn;
use crate::error::ApiError;
use crate::response::{ApiData, ApiList, Created, NoContent};
use crate::state::SharedState;

// ── Query params ────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Default)]
pub struct OrgListQuery {
    pub offset: Option<i64>,
    pub limit: Option<i64>,
    pub org_type: Option<String>,
    pub tag: Option<String>,
    pub worked: Option<i32>,
    pub search: Option<String>,
    pub status: Option<String>,
}

// ── Handlers ────────────────────────────────────────────────────────

async fn create_organization(
    State(state): State<SharedState>,
    Json(input): Json<CreateOrganizationInput>,
) -> Result<Created<Organization>, ApiError> {
    let result =
        with_conn(&state, move |conn| OrganizationStore::create(conn, &input)).await?;
    Ok(Created(result))
}

async fn list_organizations(
    State(state): State<SharedState>,
    Query(q): Query<OrgListQuery>,
) -> Result<Json<ApiList<Organization>>, ApiError> {
    let filter = OrganizationFilter {
        org_type: q.org_type,
        tag: q.tag,
        worked: q.worked,
        search: q.search,
        status: q.status,
    };
    let offset = Some(q.offset.unwrap_or(0).max(0));
    let limit = Some(q.limit.unwrap_or(50).clamp(1, 200));

    let (data, pagination) = with_conn(&state, move |conn| {
        OrganizationStore::list(conn, Some(&filter), offset, limit)
    })
    .await?;

    Ok(Json(ApiList { data, pagination }))
}

async fn get_organization(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<ApiData<Organization>>, ApiError> {
    let result = with_conn(&state, move |conn| {
        OrganizationStore::get(conn, &id)?.ok_or_else(|| forge_core::ForgeError::NotFound {
            entity_type: "Organization".into(),
            id: id.clone(),
        })
    })
    .await?;
    Ok(Json(ApiData { data: result }))
}

async fn update_organization(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(input): Json<CreateOrganizationInput>,
) -> Result<Json<ApiData<Organization>>, ApiError> {
    let result =
        with_conn(&state, move |conn| OrganizationStore::update(conn, &id, &input)).await?;
    Ok(Json(ApiData { data: result }))
}

async fn delete_organization(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<NoContent, ApiError> {
    with_conn(&state, move |conn| OrganizationStore::delete(conn, &id)).await?;
    Ok(NoContent)
}

// ── Router ──────────────────────────────────────────────────────────

pub fn router() -> Router<SharedState> {
    Router::new()
        .route(
            "/organizations",
            post(create_organization).get(list_organizations),
        )
        .route(
            "/organizations/{id}",
            get(get_organization)
                .patch(update_organization)
                .delete(delete_organization),
        )
}
