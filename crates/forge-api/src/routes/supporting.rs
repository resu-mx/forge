//! Supporting routes — skill-domain sub-resource management.
//!
//! The main /skills CRUD lives in `skills.rs`. This module adds only
//! the domain junction sub-routes that mirror the TS supporting.ts.

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use forge_core::Domain;
use forge_sdk::db::SkillStore;

use crate::db::with_conn;
use crate::error::ApiError;
use crate::response::{ApiData, NoContent};
use crate::state::SharedState;

// -- Body types ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AddDomainBody {
    pub domain_id: String,
}

// -- Handlers ────────────────────────────────────────────────────────

async fn get_skill_domains(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<ApiData<Vec<Domain>>>, ApiError> {
    let data = with_conn(&state, move |conn| SkillStore::get_domains(conn, &id)).await?;
    Ok(Json(ApiData { data }))
}

async fn add_skill_domain(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(body): Json<AddDomainBody>,
) -> Result<NoContent, ApiError> {
    with_conn(&state, move |conn| {
        SkillStore::add_domain(conn, &id, &body.domain_id)
    })
    .await?;
    Ok(NoContent)
}

async fn remove_skill_domain(
    State(state): State<SharedState>,
    Path((skill_id, domain_id)): Path<(String, String)>,
) -> Result<NoContent, ApiError> {
    with_conn(&state, move |conn| {
        SkillStore::remove_domain(conn, &skill_id, &domain_id)
    })
    .await?;
    Ok(NoContent)
}

// -- Router ──────────────────────────────────────────────────────────

pub fn router() -> Router<SharedState> {
    Router::new()
        .route(
            "/skills/{id}/domains",
            get(get_skill_domains).post(add_skill_domain),
        )
        .route(
            "/skills/{skill_id}/domains/{domain_id}",
            axum::routing::delete(remove_skill_domain),
        )
}
