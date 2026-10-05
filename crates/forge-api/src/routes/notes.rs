//! Note CRUD routes.
//!
//! Mirrors `packages/core/src/routes/notes.ts` — same URL paths,
//! same JSON shapes, so the webui and MCP server continue working.

use axum::extract::{Path, Query, State};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::Deserialize;

use forge_core::{NoteReferenceEntityType, UserNote};
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

/// A note with its entity references, as the TS API returns it.
async fn get_note(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<ApiData<serde_json::Value>>, ApiError> {
    let result = with_conn(&state, move |conn| {
        let note = NoteStore::get(conn, &id)?.ok_or_else(|| forge_core::ForgeError::NotFound {
            entity_type: "Note".into(),
            id: id.clone(),
        })?;
        let references = NoteStore::list_references(conn, &id)?;
        let mut value = serde_json::to_value(&note)
            .map_err(|e| forge_core::ForgeError::Internal(e.to_string()))?;
        if let Some(map) = value.as_object_mut() {
            map.insert(
                "references".into(),
                serde_json::to_value(&references)
                    .map_err(|e| forge_core::ForgeError::Internal(e.to_string()))?,
            );
        }
        Ok(value)
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

// ── References ──────────────────────────────────────────────────────

fn parse_entity_type(raw: &str) -> Result<NoteReferenceEntityType, ApiError> {
    raw.parse().map_err(|_| {
        ApiError(forge_core::ForgeError::Validation {
            message: format!("Invalid entity_type '{raw}'"),
            field: Some("entity_type".into()),
        })
    })
}

#[derive(Debug, Deserialize)]
pub struct AddReferenceBody {
    pub entity_type: String,
    pub entity_id: String,
}

/// `POST /notes/:id/references` — answers 201 with `{ "data": null }`, as TS does.
async fn add_reference(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(body): Json<AddReferenceBody>,
) -> Result<Created<Option<()>>, ApiError> {
    let entity_type = parse_entity_type(&body.entity_type)?;
    with_conn(&state, move |conn| {
        NoteStore::get(conn, &id)?.ok_or_else(|| forge_core::ForgeError::NotFound {
            entity_type: "Note".into(),
            id: id.clone(),
        })?;
        NoteStore::add_reference(conn, &id, entity_type, &body.entity_id)
    })
    .await?;
    Ok(Created(None))
}

async fn remove_reference(
    State(state): State<SharedState>,
    Path((id, entity_type, entity_id)): Path<(String, String, String)>,
) -> Result<NoContent, ApiError> {
    let entity_type = parse_entity_type(&entity_type)?;
    with_conn(&state, move |conn| {
        NoteStore::remove_reference(conn, &id, entity_type, &entity_id)
    })
    .await?;
    Ok(NoContent)
}

async fn notes_for_entity(
    State(state): State<SharedState>,
    Path((entity_type, entity_id)): Path<(String, String)>,
) -> Result<Json<ApiData<Vec<UserNote>>>, ApiError> {
    let entity_type = parse_entity_type(&entity_type)?;
    let data = with_conn(&state, move |conn| {
        NoteStore::find_by_entity(conn, entity_type, &entity_id)
    })
    .await?;
    Ok(Json(ApiData { data }))
}

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/notes", post(create_note).get(list_notes))
        .route(
            "/notes/{id}",
            get(get_note).patch(update_note).delete(delete_note),
        )
        .route(
            "/notes/by-entity/{entity_type}/{entity_id}",
            get(notes_for_entity),
        )
        .route("/notes/{id}/references", post(add_reference))
        .route(
            "/notes/{id}/references/{entity_type}/{entity_id}",
            delete(remove_reference),
        )
}
