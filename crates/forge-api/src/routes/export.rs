//! Export routes — data bundle export and database dump.
//!
//! Mirrors the data-export and dump endpoints from
//! `packages/core/src/routes/export.ts`. Resume format exports are
//! deferred to the compiler/resume route modules.

use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;

use forge_core::DataExportBundle;
use forge_sdk::services::ExportService;

use crate::db::with_conn;
use crate::error::ApiError;
use crate::response::ApiData;
use crate::state::SharedState;

// ── Query params ────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ExportDataQuery {
    /// Comma-separated list of entity types to export
    /// (e.g. `sources,bullets,skills`).
    pub entities: String,
}

// ── Handlers ────────────────────────────────────────────────────────

async fn export_data(
    State(state): State<SharedState>,
    Query(q): Query<ExportDataQuery>,
) -> Result<Json<ApiData<DataExportBundle>>, ApiError> {
    let entities: Vec<String> = q
        .entities
        .split(',')
        .map(|e| e.trim().to_string())
        .filter(|e| !e.is_empty())
        .collect();

    if entities.is_empty() {
        return Err(ApiError(forge_core::ForgeError::Validation {
            field: Some("entities".into()),
            message: "At least one entity type is required".into(),
        }));
    }

    let result = with_conn(&state, move |conn| {
        ExportService::export_data(conn, &entities)
    })
    .await?;
    Ok(Json(ApiData { data: result }))
}

async fn dump_database(
    State(state): State<SharedState>,
) -> Result<Json<ApiData<String>>, ApiError> {
    let result = with_conn(&state, move |conn| {
        ExportService::dump_database(conn)
    })
    .await?;
    Ok(Json(ApiData { data: result }))
}

// ── Router ──────────────────────────────────────────────────────────

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/export/data", get(export_data))
        .route("/export/dump", get(dump_database))
}
