//! Campus (org location & alias) routes — sub-resources for organizations.
//!
//! Mirrors `packages/core/src/routes/campuses.ts`. Locations and aliases
//! are scoped under `/organizations/:orgId/`.

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};

use forge_core::{CreateOrgAlias, CreateOrgLocation, OrgAlias, OrgLocation, UpdateOrgLocation};
use forge_sdk::db::CampusStore;

use crate::db::with_conn;
use crate::error::ApiError;
use crate::response::{ApiData, Created, NoContent};
use crate::state::SharedState;

// -- Location handlers ───────────────────────────────────────────────

async fn list_locations(
    State(state): State<SharedState>,
    Path(org_id): Path<String>,
) -> Result<Json<ApiData<Vec<OrgLocation>>>, ApiError> {
    let data =
        with_conn(&state, move |conn| CampusStore::list_by_org(conn, &org_id)).await?;
    Ok(Json(ApiData { data }))
}

async fn create_location(
    State(state): State<SharedState>,
    Path(org_id): Path<String>,
    Json(mut input): Json<CreateOrgLocation>,
) -> Result<Created<OrgLocation>, ApiError> {
    input.organization_id = org_id;
    let result =
        with_conn(&state, move |conn| CampusStore::create_location(conn, &input)).await?;
    Ok(Created(result))
}

async fn update_location(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(input): Json<UpdateOrgLocation>,
) -> Result<Json<ApiData<OrgLocation>>, ApiError> {
    let result =
        with_conn(&state, move |conn| CampusStore::update_location(conn, &id, &input)).await?;
    Ok(Json(ApiData { data: result }))
}

async fn delete_location(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<NoContent, ApiError> {
    with_conn(&state, move |conn| CampusStore::delete_location(conn, &id)).await?;
    Ok(NoContent)
}

// -- Alias handlers ──────────────────────────────────────────────────

async fn list_aliases(
    State(state): State<SharedState>,
    Path(org_id): Path<String>,
) -> Result<Json<ApiData<Vec<OrgAlias>>>, ApiError> {
    let data =
        with_conn(&state, move |conn| CampusStore::list_aliases(conn, &org_id)).await?;
    Ok(Json(ApiData { data }))
}

async fn create_alias(
    State(state): State<SharedState>,
    Path(org_id): Path<String>,
    Json(input): Json<CreateOrgAlias>,
) -> Result<Created<OrgAlias>, ApiError> {
    let result =
        with_conn(&state, move |conn| CampusStore::create_alias(conn, &org_id, &input)).await?;
    Ok(Created(result))
}

async fn delete_alias(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<NoContent, ApiError> {
    with_conn(&state, move |conn| CampusStore::delete_alias(conn, &id)).await?;
    Ok(NoContent)
}

// -- Router ──────────────────────────────────────────────────────────

pub fn router() -> Router<SharedState> {
    Router::new()
        // Locations (primary paths)
        .route(
            "/organizations/{org_id}/locations",
            get(list_locations).post(create_location),
        )
        .route("/locations/{id}", axum::routing::patch(update_location).delete(delete_location))
        // Aliases
        .route(
            "/organizations/{org_id}/aliases",
            get(list_aliases).post(create_alias),
        )
        .route(
            "/aliases/{id}",
            axum::routing::delete(delete_alias),
        )
}
