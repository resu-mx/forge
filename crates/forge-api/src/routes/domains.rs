//! Domain CRUD routes.
//!
//! Simple lookup table — no pagination, no update.

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};

use forge_core::{CreateDomainInput, Domain};
use forge_sdk::db::DomainStore;

use crate::db::with_conn;
use crate::error::ApiError;
use crate::response::{ApiData, Created, NoContent};
use crate::state::SharedState;

// -- Handlers ────────────────────────────────────────────────────────

async fn create_domain(
    State(state): State<SharedState>,
    Json(input): Json<CreateDomainInput>,
) -> Result<Created<Domain>, ApiError> {
    let result = with_conn(&state, move |conn| DomainStore::create(conn, &input)).await?;
    Ok(Created(result))
}

async fn list_domains(
    State(state): State<SharedState>,
) -> Result<Json<ApiData<Vec<Domain>>>, ApiError> {
    let data = with_conn(&state, move |conn| DomainStore::list(conn)).await?;
    Ok(Json(ApiData { data }))
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
            get(get_domain).delete(delete_domain),
        )
}
