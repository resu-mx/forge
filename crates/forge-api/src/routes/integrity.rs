//! Integrity routes — content drift detection.
//!
//! Mirrors `packages/core/src/routes/integrity.ts`.

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};

use forge_sdk::services::{DriftedEntity, IntegrityService};

use crate::db::with_conn;
use crate::error::ApiError;
use crate::response::ApiData;
use crate::state::SharedState;

// ── Handlers ────────────────────────────────────────────────────────

async fn get_drifted_entities(
    State(state): State<SharedState>,
) -> Result<Json<ApiData<Vec<DriftedEntity>>>, ApiError> {
    let result = with_conn(&state, move |conn| {
        IntegrityService::get_drifted_entities(conn)
    })
    .await?;
    Ok(Json(ApiData { data: result }))
}

// ── Router ──────────────────────────────────────────────────────────

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/integrity/drift", get(get_drifted_entities))
}
