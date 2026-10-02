//! Industry CRUD routes.
//!
//! Simple lookup table — no pagination, no update.

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};

use forge_core::{CreateIndustryInput, Industry};
use forge_sdk::db::IndustryStore;

use crate::db::with_conn;
use crate::error::ApiError;
use crate::response::{ApiData, Created, NoContent};
use crate::state::SharedState;

// -- Handlers ────────────────────────────────────────────────────────

async fn create_industry(
    State(state): State<SharedState>,
    Json(input): Json<CreateIndustryInput>,
) -> Result<Created<Industry>, ApiError> {
    let result = with_conn(&state, move |conn| IndustryStore::create(conn, &input)).await?;
    Ok(Created(result))
}

async fn list_industries(
    State(state): State<SharedState>,
) -> Result<Json<ApiData<Vec<Industry>>>, ApiError> {
    let data = with_conn(&state, move |conn| IndustryStore::list(conn)).await?;
    Ok(Json(ApiData { data }))
}

async fn get_industry(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<ApiData<Industry>>, ApiError> {
    let result = with_conn(&state, move |conn| {
        IndustryStore::get(conn, &id)?.ok_or_else(|| forge_core::ForgeError::NotFound {
            entity_type: "Industry".into(),
            id: id.clone(),
        })
    })
    .await?;
    Ok(Json(ApiData { data: result }))
}

async fn delete_industry(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<NoContent, ApiError> {
    with_conn(&state, move |conn| IndustryStore::delete(conn, &id)).await?;
    Ok(NoContent)
}

// -- Router ──────────────────────────────────────────────────────────

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/industries", post(create_industry).get(list_industries))
        .route(
            "/industries/{id}",
            get(get_industry).delete(delete_industry),
        )
}
