//! Contact CRUD routes.
//!
//! Mirrors the TS contact routes — same URL paths,
//! same JSON shapes, so the webui and MCP server continue working.

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use forge_core::{Contact, ContactFilter, ContactWithOrg, CreateContact, UpdateContact};
use forge_sdk::db::ContactStore;

use crate::db::with_conn;
use crate::error::ApiError;
use crate::response::{ApiData, ApiList, Created, NoContent};
use crate::state::SharedState;

// ── Query params ────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Default)]
pub struct ContactListQuery {
    pub offset: Option<i64>,
    pub limit: Option<i64>,
    pub search: Option<String>,
    pub organization_id: Option<String>,
}

// ── Handlers ────────────────────────────────────────────────────────

async fn create_contact(
    State(state): State<SharedState>,
    Json(input): Json<CreateContact>,
) -> Result<Created<Contact>, ApiError> {
    let result = with_conn(&state, move |conn| ContactStore::create(conn, &input)).await?;
    Ok(Created(result))
}

async fn list_contacts(
    State(state): State<SharedState>,
    Query(q): Query<ContactListQuery>,
) -> Result<Json<ApiList<ContactWithOrg>>, ApiError> {
    let filter = ContactFilter {
        organization_id: q.organization_id,
        search: q.search,
    };
    let offset = q.offset.unwrap_or(0).max(0);
    let limit = q.limit.unwrap_or(50).clamp(1, 200);

    let (data, pagination) =
        with_conn(&state, move |conn| ContactStore::list(conn, &filter, offset, limit)).await?;

    Ok(Json(ApiList { data, pagination }))
}

async fn get_contact(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<ApiData<ContactWithOrg>>, ApiError> {
    let result = with_conn(&state, move |conn| {
        ContactStore::get_with_org(conn, &id)?.ok_or_else(|| forge_core::ForgeError::NotFound {
            entity_type: "Contact".into(),
            id: id.clone(),
        })
    })
    .await?;
    Ok(Json(ApiData { data: result }))
}

async fn update_contact(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(input): Json<UpdateContact>,
) -> Result<Json<ApiData<Contact>>, ApiError> {
    let result =
        with_conn(&state, move |conn| ContactStore::update(conn, &id, &input)).await?;
    Ok(Json(ApiData { data: result }))
}

async fn delete_contact(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<NoContent, ApiError> {
    with_conn(&state, move |conn| ContactStore::delete(conn, &id)).await?;
    Ok(NoContent)
}

// ── Router ──────────────────────────────────────────────────────────

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/contacts", post(create_contact).get(list_contacts))
        .route(
            "/contacts/{id}",
            get(get_contact).patch(update_contact).delete(delete_contact),
        )
}
