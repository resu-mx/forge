//! Export routes — data bundle export and database dump.
//!
//! Mirrors the data-export and dump endpoints from
//! `packages/core/src/routes/export.ts`. Resume format exports are
//! deferred to the compiler/resume route modules.

use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;

use forge_core::DataExportBundle;
use forge_sdk::db::ResumeStore;
use forge_sdk::services::{CompilerService, ExportService};

use crate::db::with_conn;
use crate::error::ApiError;
use crate::response::{not_implemented, ApiData};
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

// ── Resume export ───────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ExportResumeQuery {
    pub format: Option<String>,
}

/// Lowercase, runs of non-alphanumerics become `-`, edges trimmed (TS `slugify`).
fn slugify(name: &str) -> String {
    let mut slug = String::new();
    let mut pending_dash = false;
    for c in name.to_lowercase().chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            if pending_dash && !slug.is_empty() {
                slug.push('-');
            }
            pending_dash = false;
            slug.push(c);
        } else {
            pending_dash = true;
        }
    }
    slug
}

/// `GET /export/resume/:id?format=pdf|markdown|latex|json`. Markdown and LaTeX use
/// the resume's saved override when there is one, otherwise the generated output.
async fn export_resume(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Query(q): Query<ExportResumeQuery>,
) -> Result<Response, ApiError> {
    let format = match q.format.as_deref() {
        Some(f @ ("pdf" | "markdown" | "latex" | "json")) => f.to_string(),
        _ => {
            return Err(ApiError(forge_core::ForgeError::Validation {
                message: "format query parameter is required. Valid values: pdf, markdown, latex, json".into(),
                field: Some("format".into()),
            }))
        }
    };

    let format_for_db = format.clone();
    let (name, body) = with_conn(&state, move |conn| {
        let resume = ResumeStore::get(conn, &id)?.ok_or_else(|| forge_core::ForgeError::NotFound {
            entity_type: "resume".into(),
            id: id.clone(),
        })?;
        let body = match format_for_db.as_str() {
            "markdown" if resume.markdown_override.as_deref().is_some_and(|s| !s.is_empty()) => {
                resume.markdown_override.clone().unwrap_or_default()
            }
            "latex" if resume.latex_override.as_deref().is_some_and(|s| !s.is_empty()) => {
                resume.latex_override.clone().unwrap_or_default()
            }
            "pdf" => String::new(),
            other => {
                let doc = CompilerService::compile(conn, &id)?.ok_or_else(|| {
                    forge_core::ForgeError::NotFound { entity_type: "resume".into(), id: id.clone() }
                })?;
                match other {
                    "markdown" => CompilerService::render_markdown(&doc),
                    "latex" => CompilerService::render_latex(&doc),
                    _ => serde_json::to_string(&serde_json::json!({ "data": doc }))
                        .map_err(|e| forge_core::ForgeError::Internal(e.to_string()))?,
                }
            }
        };
        Ok((resume.name, body))
    })
    .await?;

    let filename = |ext: &str| {
        format!(
            "attachment; filename=\"{}-{}.{ext}\"",
            slugify(&name),
            chrono::Utc::now().format("%Y-%m-%d")
        )
    };
    let response = match format.as_str() {
        "json" => (
            [(header::CONTENT_TYPE, "application/json".to_string()), (header::CONTENT_DISPOSITION, filename("json"))],
            body,
        )
            .into_response(),
        "markdown" => (
            [
                (header::CONTENT_TYPE, "text/markdown; charset=utf-8".to_string()),
                (header::CONTENT_DISPOSITION, filename("md")),
            ],
            body,
        )
            .into_response(),
        "latex" => (
            [
                (header::CONTENT_TYPE, "application/x-latex; charset=utf-8".to_string()),
                (header::CONTENT_DISPOSITION, filename("tex")),
            ],
            body,
        )
            .into_response(),
        _ => not_implemented("PDF rendering is not available in the Rust server yet"),
    };
    debug_assert!(response.status() != StatusCode::INTERNAL_SERVER_ERROR);
    Ok(response)
}

// ── Router ──────────────────────────────────────────────────────────

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/export/resume/{id}", get(export_resume))
        .route("/export/data", get(export_data))
        .route("/export/dump", get(dump_database))
}
