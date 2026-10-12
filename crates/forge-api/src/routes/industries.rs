//! Industry CRUD routes.
//!
//! Simple lookup table. The list is paginated like TS (`offset` default 0, `limit` default 50
//! clamped to 1..=200; resu-mx/forge#141).

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use forge_core::{CreateIndustryInput, Industry, UpdateIndustryInput};
use forge_sdk::db::IndustryStore;

use crate::db::with_conn;
use crate::error::ApiError;
use crate::response::{ApiData, ApiList, Created, NoContent};
use crate::state::SharedState;

// -- Query params ────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Default)]
pub struct IndustryListQuery {
    pub offset: Option<i64>,
    pub limit: Option<i64>,
}

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
    Query(q): Query<IndustryListQuery>,
) -> Result<Json<ApiList<Industry>>, ApiError> {
    let offset = q.offset.unwrap_or(0).max(0);
    let limit = q.limit.unwrap_or(50).clamp(1, 200);

    let (data, pagination) =
        with_conn(&state, move |conn| IndustryStore::list(conn, offset, limit)).await?;

    Ok(Json(ApiList { data, pagination }))
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

/// `PATCH /industries/:id` (TS industries.ts:33-38). 200 with the row, 400 for a blank name,
/// 404 for an unknown id, 409 for a name another industry has.
async fn update_industry(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(input): Json<UpdateIndustryInput>,
) -> Result<Json<ApiData<Industry>>, ApiError> {
    let result = with_conn(&state, move |conn| IndustryStore::update(conn, &id, &input)).await?;
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
            get(get_industry)
                .patch(update_industry)
                .delete(delete_industry),
        )
}
