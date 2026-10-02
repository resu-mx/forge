//! Credential CRUD routes.
//!
//! Mirrors `packages/core/src/routes/credentials.ts` — full CRUD with
//! optional `?type=` filter on the list endpoint.

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use forge_core::{CreateCredential, Credential, UpdateCredential};
use forge_sdk::db::CredentialStore;

use crate::db::with_conn;
use crate::error::ApiError;
use crate::response::{ApiData, Created, NoContent};
use crate::state::SharedState;

// -- Query params ────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Default)]
pub struct CredentialListQuery {
    #[serde(rename = "type")]
    pub credential_type: Option<String>,
}

// -- Handlers ────────────────────────────────────────────────────────

async fn create_credential(
    State(state): State<SharedState>,
    Json(input): Json<CreateCredential>,
) -> Result<Created<Credential>, ApiError> {
    let result =
        with_conn(&state, move |conn| CredentialStore::create(conn, &input)).await?;
    Ok(Created(result))
}

async fn list_credentials(
    State(state): State<SharedState>,
    Query(q): Query<CredentialListQuery>,
) -> Result<Json<ApiData<Vec<Credential>>>, ApiError> {
    let data = match q.credential_type {
        Some(t) => with_conn(&state, move |conn| CredentialStore::find_by_type(conn, &t)).await?,
        None => with_conn(&state, move |conn| CredentialStore::list(conn)).await?,
    };
    Ok(Json(ApiData { data }))
}

async fn get_credential(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<ApiData<Credential>>, ApiError> {
    let result = with_conn(&state, move |conn| {
        CredentialStore::get(conn, &id)?.ok_or_else(|| forge_core::ForgeError::NotFound {
            entity_type: "Credential".into(),
            id: id.clone(),
        })
    })
    .await?;
    Ok(Json(ApiData { data: result }))
}

async fn update_credential(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(input): Json<UpdateCredential>,
) -> Result<Json<ApiData<Credential>>, ApiError> {
    let result =
        with_conn(&state, move |conn| CredentialStore::update(conn, &id, &input)).await?;
    Ok(Json(ApiData { data: result }))
}

async fn delete_credential(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<NoContent, ApiError> {
    with_conn(&state, move |conn| CredentialStore::delete(conn, &id)).await?;
    Ok(NoContent)
}

// -- Router ──────────────────────────────────────────────────────────

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/credentials", post(create_credential).get(list_credentials))
        .route(
            "/credentials/{id}",
            get(get_credential)
                .patch(update_credential)
                .delete(delete_credential),
        )
}
