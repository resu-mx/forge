//! Domain routes: create, paginated list with usage counts, get, partial update, delete.

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use forge_core::{CreateDomainInput, Domain, DomainWithUsage, UpdateDomainInput};
use forge_sdk::db::DomainStore;

use crate::db::with_conn;
use crate::error::ApiError;
use crate::response::{ApiData, ApiList, Created, NoContent};
use crate::state::SharedState;

// -- Handlers ────────────────────────────────────────────────────────

async fn create_domain(
    State(state): State<SharedState>,
    Json(input): Json<CreateDomainInput>,
) -> Result<Created<Domain>, ApiError> {
    let result = with_conn(&state, move |conn| DomainStore::create(conn, &input)).await?;
    Ok(Created(result))
}

#[derive(Debug, Deserialize, Default)]
pub struct DomainListQuery {
    pub offset: Option<i64>,
    pub limit: Option<i64>,
}

async fn list_domains(
    State(state): State<SharedState>,
    Query(q): Query<DomainListQuery>,
) -> Result<Json<ApiList<DomainWithUsage>>, ApiError> {
    // TS defaults and bounds: packages/core/src/routes/domains.ts:20-21.
    // Known edge difference, left as in archetypes.rs: TS reads a non-numeric
    // limit or limit=0 as 50, while Rust answers 400 for non-numeric input and
    // clamps 0 to 1.
    let offset = q.offset.unwrap_or(0).max(0);
    let limit = q.limit.unwrap_or(50).clamp(1, 200);

    let (data, pagination) = with_conn(&state, move |conn| {
        DomainStore::list_with_usage(conn, offset, limit)
    })
    .await?;
    Ok(Json(ApiList { data, pagination }))
}

async fn get_domain(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<ApiData<Domain>>, ApiError> {
    let result = with_conn(&state, move |conn| {
        DomainStore::get(conn, &id)?.ok_or_else(|| forge_core::ForgeError::NotFound {
            entity_type: "Domain".into(),
            id: id.clone(),
        })
    })
    .await?;
    Ok(Json(ApiData { data: result }))
}

async fn update_domain(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(input): Json<UpdateDomainInput>,
) -> Result<Json<ApiData<Domain>>, ApiError> {
    let result = with_conn(&state, move |conn| DomainStore::update(conn, &id, &input)).await?;
    Ok(Json(ApiData { data: result }))
}

async fn delete_domain(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<NoContent, ApiError> {
    with_conn(&state, move |conn| DomainStore::delete(conn, &id)).await?;
    Ok(NoContent)
}

// -- Router ──────────────────────────────────────────────────────────

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/domains", post(create_domain).get(list_domains))
        .route(
            "/domains/{id}",
            get(get_domain).patch(update_domain).delete(delete_domain),
        )
}
