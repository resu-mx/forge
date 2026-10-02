//! Audit routes — chain tracing and integrity checking.
//!
//! Mirrors `packages/core/src/routes/audit.ts`.

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};

use forge_core::{ChainTrace, IntegrityReport};
use forge_sdk::services::AuditService;

use crate::db::with_conn;
use crate::error::ApiError;
use crate::response::ApiData;
use crate::state::SharedState;

// ── Handlers ────────────────────────────────────────────────────────

async fn trace_chain(
    State(state): State<SharedState>,
    Path(perspective_id): Path<String>,
) -> Result<Json<ApiData<ChainTrace>>, ApiError> {
    let result = with_conn(&state, move |conn| {
        AuditService::trace_chain(conn, &perspective_id)
    })
    .await?;
    Ok(Json(ApiData { data: result }))
}

async fn check_integrity(
    State(state): State<SharedState>,
    Path(perspective_id): Path<String>,
) -> Result<Json<ApiData<IntegrityReport>>, ApiError> {
    let result = with_conn(&state, move |conn| {
        AuditService::check_integrity(conn, &perspective_id)
    })
    .await?;
    Ok(Json(ApiData { data: result }))
}

// ── Router ──────────────────────────────────────────────────────────

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/audit/chain/{perspectiveId}", get(trace_chain))
        .route("/audit/integrity/{perspectiveId}", get(check_integrity))
}
