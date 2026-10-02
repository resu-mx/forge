//! Note CRUD routes.
//!
//! Mirrors `packages/core/src/routes/notes.ts` — same URL paths,
//! same JSON shapes, so the webui and MCP server continue working.

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use forge_core::UserNote;
use forge_sdk::db::NoteStore;

use crate::db::with_conn;
use crate::error::ApiError;
use crate::response::{ApiData, ApiList, Created, NoContent};
use crate::state::SharedState;

// ── Query params ────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Default)]
pub struct NoteListQuery {
    pub offset: Option<i64>,
    pub limit: Option<i64>,
    pub search: Option<String>,
}

// ── Request bodies ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateNoteBody {
    pub title: Option<String>,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateNoteBody {
    pub title: Option<String>,
    pub content: Option<String>,
}

// ── Handlers ────────────────────────────────────────────────────────

async fn create_note(
    State(state): State<SharedState>,
    Json(body): Json<CreateNoteBody>,
) -> Result<Created<UserNote>, ApiError> {
    let result = with_conn(&state, move |conn| {
        NoteStore::create(conn, body.title.as_deref(), &body.content)
    })
    .await?;
    Ok(Created(result))
}

async fn list_notes(
    State(state): State<SharedState>,
    Query(q): Query<NoteListQuery>,
) -> Result<Json<ApiList<UserNote>>, ApiError> {
    let offset = q.offset.unwrap_or(0).max(0);
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let search = q.search;

    let (data, pagination) = with_conn(&state, move |conn| {
        NoteStore::list(conn, search.as_deref(), offset, limit)
    })
    .await?;

    Ok(Json(ApiList { data, pagination }))
}

async fn get_note(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<ApiData<UserNote>>, ApiError> {
    let result = with_conn(&state, move |conn| {
        NoteStore::get(conn, &id)?.ok_or_else(|| forge_core::ForgeError::NotFound {
            entity_type: "Note".into(),
            id: id.clone(),
        })
    })
    .await?;
    Ok(Json(ApiData { data: result }))
}

async fn update_note(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateNoteBody>,
) -> Result<Json<ApiData<UserNote>>, ApiError> {
    let result = with_conn(&state, move |conn| {
        NoteStore::update(conn, &id, body.title.as_deref(), body.content.as_deref())
    })
    .await?;
    Ok(Json(ApiData { data: result }))
}

async fn delete_note(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<NoContent, ApiError> {
    with_conn(&state, move |conn| NoteStore::delete(conn, &id)).await?;
    Ok(NoContent)
}

// ── Router ──────────────────────────────────────────────────────────

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/notes", post(create_note).get(list_notes))
        .route(
            "/notes/{id}",
            get(get_note).patch(update_note).delete(delete_note),
        )
}
