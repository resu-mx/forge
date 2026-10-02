//! Archetype CRUD routes with domain sub-resource.
//!
//! Mirrors `packages/core/src/routes/archetypes.ts` — full CRUD plus
//! archetype_domains junction management.

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use forge_core::{
    Archetype, ArchetypeWithDomains, CreateArchetypeInput, Domain, UpdateArchetypeInput,
};
use forge_sdk::db::ArchetypeStore;

use crate::db::with_conn;
use crate::error::ApiError;
use crate::response::{ApiData, ApiList, Created, NoContent};
use crate::state::SharedState;

// -- Query params ────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Default)]
pub struct ArchetypeListQuery {
    pub offset: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct AddDomainBody {
    pub domain_id: String,
}

// -- Handlers ────────────────────────────────────────────────────────

async fn create_archetype(
    State(state): State<SharedState>,
    Json(input): Json<CreateArchetypeInput>,
) -> Result<Created<Archetype>, ApiError> {
    let result =
        with_conn(&state, move |conn| ArchetypeStore::create(conn, &input)).await?;
    Ok(Created(result))
}

async fn list_archetypes(
    State(state): State<SharedState>,
    Query(q): Query<ArchetypeListQuery>,
) -> Result<Json<ApiList<Archetype>>, ApiError> {
    let offset = q.offset.unwrap_or(0).max(0);
    let limit = q.limit.unwrap_or(50).clamp(1, 200);

    let (data, pagination) =
        with_conn(&state, move |conn| ArchetypeStore::list(conn, offset, limit)).await?;

    Ok(Json(ApiList { data, pagination }))
}

async fn get_archetype(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<ApiData<ArchetypeWithDomains>>, ApiError> {
    let result = with_conn(&state, move |conn| {
        ArchetypeStore::get_with_domains(conn, &id)?.ok_or_else(|| {
            forge_core::ForgeError::NotFound {
                entity_type: "Archetype".into(),
                id: id.clone(),
            }
        })
    })
    .await?;
    Ok(Json(ApiData { data: result }))
}

async fn update_archetype(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(input): Json<UpdateArchetypeInput>,
) -> Result<Json<ApiData<Archetype>>, ApiError> {
    let result =
        with_conn(&state, move |conn| ArchetypeStore::update(conn, &id, &input)).await?;
    Ok(Json(ApiData { data: result }))
}

async fn delete_archetype(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<NoContent, ApiError> {
    with_conn(&state, move |conn| ArchetypeStore::delete(conn, &id)).await?;
    Ok(NoContent)
}

// -- Domain sub-resource handlers ──────────────────────────────────

async fn get_archetype_domains(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<ApiData<Vec<Domain>>>, ApiError> {
    let data =
        with_conn(&state, move |conn| ArchetypeStore::list_domains(conn, &id)).await?;
    Ok(Json(ApiData { data }))
}

async fn add_archetype_domain(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(body): Json<AddDomainBody>,
) -> Result<Created<()>, ApiError> {
    with_conn(&state, move |conn| {
        ArchetypeStore::add_domain(conn, &id, &body.domain_id)
    })
    .await?;
    Ok(Created(()))
}

async fn remove_archetype_domain(
    State(state): State<SharedState>,
    Path((archetype_id, domain_id)): Path<(String, String)>,
) -> Result<NoContent, ApiError> {
    with_conn(&state, move |conn| {
        ArchetypeStore::remove_domain(conn, &archetype_id, &domain_id)
    })
    .await?;
    Ok(NoContent)
}

// -- Router ──────────────────────────────────────────────────────────

pub fn router() -> Router<SharedState> {
    Router::new()
        .route(
            "/archetypes",
            post(create_archetype).get(list_archetypes),
        )
        .route(
            "/archetypes/{id}",
            get(get_archetype)
                .patch(update_archetype)
                .delete(delete_archetype),
        )
        .route(
            "/archetypes/{id}/domains",
            get(get_archetype_domains).post(add_archetype_domain),
        )
        .route(
            "/archetypes/{archetype_id}/domains/{domain_id}",
            axum::routing::delete(remove_archetype_domain),
        )
}
