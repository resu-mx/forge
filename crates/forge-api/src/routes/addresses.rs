//! Address CRUD routes.
//!
//! Mirrors `packages/core/src/routes/addresses.ts` — same URL paths,
//! same JSON shapes.

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use forge_core::{Address, CreateAddress, UpdateAddress};
use forge_sdk::db::AddressStore;

use crate::db::with_conn;
use crate::error::ApiError;
use crate::response::{ApiData, ApiList, Created, NoContent};
use crate::state::SharedState;

// -- Query params ────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Default)]
pub struct AddressListQuery {
    pub offset: Option<i64>,
    pub limit: Option<i64>,
}

// -- Handlers ────────────────────────────────────────────────────────

async fn create_address(
    State(state): State<SharedState>,
    Json(input): Json<CreateAddress>,
) -> Result<Created<Address>, ApiError> {
    let result = with_conn(&state, move |conn| AddressStore::create(conn, &input)).await?;
    Ok(Created(result))
}

async fn list_addresses(
    State(state): State<SharedState>,
    Query(q): Query<AddressListQuery>,
) -> Result<Json<ApiList<Address>>, ApiError> {
    let offset = q.offset.unwrap_or(0).max(0);
    let limit = q.limit.unwrap_or(50).clamp(1, 200);

    let (data, pagination) =
        with_conn(&state, move |conn| AddressStore::list(conn, offset, limit)).await?;

    Ok(Json(ApiList { data, pagination }))
}

async fn get_address(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<ApiData<Address>>, ApiError> {
    let result = with_conn(&state, move |conn| {
        AddressStore::get(conn, &id)?.ok_or_else(|| forge_core::ForgeError::NotFound {
            entity_type: "Address".into(),
            id: id.clone(),
        })
    })
    .await?;
    Ok(Json(ApiData { data: result }))
}

async fn update_address(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(input): Json<UpdateAddress>,
) -> Result<Json<ApiData<Address>>, ApiError> {
    let result =
        with_conn(&state, move |conn| AddressStore::update(conn, &id, &input)).await?;
    Ok(Json(ApiData { data: result }))
}

async fn delete_address(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<NoContent, ApiError> {
    with_conn(&state, move |conn| AddressStore::delete(conn, &id)).await?;
    Ok(NoContent)
}

// -- Router ──────────────────────────────────────────────────────────

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/addresses", post(create_address).get(list_addresses))
        .route(
            "/addresses/{id}",
            get(get_address).patch(update_address).delete(delete_address),
        )
}
