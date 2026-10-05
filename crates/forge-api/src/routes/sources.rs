//! Source CRUD routes.
//!
//! Mirrors `packages/core/src/routes/sources.ts` — same URL paths,
//! same JSON shapes, so the webui and MCP server continue working.

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::Value;

use forge_core::{
    CreateSource, PaginationParams, SkillRow, SourceFilter, SourceType, SourceWithExtension,
    UpdateSource,
};
use forge_sdk::db::SourceStore;

use crate::db::with_conn;
use crate::error::ApiError;
use crate::response::{not_implemented, ApiData, ApiList, Created, NoContent};
use crate::state::SharedState;

// ── Query params ────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Default)]
pub struct SourceListQuery {
    pub offset: Option<i64>,
    pub limit: Option<i64>,
    pub source_type: Option<String>,
    pub organization_id: Option<String>,
    pub status: Option<String>,
    pub education_type: Option<String>,
    pub search: Option<String>,
}

// ── Wire shape ──────────────────────────────────────────────────────
//
// The TS API takes extension fields nested under `role` / `project` /
// `education` / `presentation` and returns them under the same key; internally
// they are flat (`CreateSource`) and the response carries `extension`.

const EXTENSION_KEYS: [&str; 4] = ["role", "project", "education", "presentation"];

/// Merge nested extension objects into the top level (nested values win, as in TS).
fn flatten_extensions(body: Value) -> Value {
    let Value::Object(mut map) = body else {
        return body;
    };
    for key in EXTENSION_KEYS {
        if let Some(Value::Object(nested)) = map.remove(key) {
            map.extend(nested);
        }
    }
    Value::Object(map)
}

/// Decode a flattened body, reporting problems in the standard error envelope.
fn decode<T: serde::de::DeserializeOwned>(body: Value) -> Result<T, ApiError> {
    serde_json::from_value(flatten_extensions(body)).map_err(|e| {
        ApiError(forge_core::ForgeError::Validation {
            message: e.to_string(),
            field: None,
        })
    })
}

/// Serialize a source, renaming `extension` to the typed key for its source type.
fn to_wire(source: SourceWithExtension) -> Value {
    let mut value = serde_json::to_value(&source).unwrap_or(Value::Null);
    if let Value::Object(map) = &mut value {
        let extension = map.remove("extension").unwrap_or(Value::Null);
        let key = map
            .get("source_type")
            .and_then(Value::as_str)
            .filter(|k| EXTENSION_KEYS.contains(k))
            .map(str::to_string);
        if let (Some(key), false) = (key, extension.is_null()) {
            map.insert(key, extension);
        }
    }
    value
}

// ── Handlers ────────────────────────────────────────────────────────

async fn create_source(
    State(state): State<SharedState>,
    Json(body): Json<Value>,
) -> Result<Created<Value>, ApiError> {
    let input: CreateSource = decode(body)?;
    let result = with_conn(&state, move |conn| SourceStore::create(conn, &input)).await?;
    Ok(Created(to_wire(result)))
}

async fn list_sources(
    State(state): State<SharedState>,
    Query(q): Query<SourceListQuery>,
) -> Result<Json<ApiList<Value>>, ApiError> {
    let filter = SourceFilter {
        source_type: q.source_type.and_then(|s| s.parse::<SourceType>().ok()),
        organization_id: q.organization_id,
        status: q.status.and_then(|s| s.parse().ok()),
        education_type: q.education_type,
        search: q.search,
    };
    let pg = PaginationParams {
        offset: Some(q.offset.unwrap_or(0).max(0)),
        limit: Some(q.limit.unwrap_or(50).clamp(1, 200)),
    };

    let (data, pagination) =
        with_conn(&state, move |conn| SourceStore::list(conn, &filter, &pg)).await?;

    Ok(Json(ApiList {
        data: data.into_iter().map(to_wire).collect(),
        pagination,
    }))
}

async fn get_source(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<ApiData<Value>>, ApiError> {
    let result = with_conn(&state, move |conn| {
        SourceStore::get_hydrated(conn, &id)?.ok_or_else(|| forge_core::ForgeError::NotFound {
            entity_type: "Source".into(),
            id: id.clone(),
        })
    })
    .await?;
    Ok(Json(ApiData {
        data: to_wire(result),
    }))
}

async fn update_source(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> Result<Json<ApiData<Value>>, ApiError> {
    let input: UpdateSource = decode(body)?;
    let result = with_conn(&state, move |conn| SourceStore::update(conn, &id, &input)).await?;
    Ok(Json(ApiData {
        data: to_wire(result),
    }))
}

async fn delete_source(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<NoContent, ApiError> {
    with_conn(&state, move |conn| SourceStore::delete(conn, &id)).await?;
    Ok(NoContent)
}

async fn derive_bullets_replaced() -> axum::response::Response {
    not_implemented(
        "This endpoint has been replaced. Use POST /api/derivations/prepare with entity_type \"source\".",
    )
}

// ── Skills (source_skills) ──────────────────────────────────────────

/// Skills linked to a source, by name. An unknown source answers `[]`, as in TS.
async fn list_source_skills(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<ApiData<Vec<SkillRow>>>, ApiError> {
    let data = with_conn(&state, move |conn| SourceStore::list_skills(conn, &id)).await?;
    Ok(Json(ApiData { data }))
}

/// `{ skill_id }` links an existing skill; `{ name, category? }` finds or creates one.
/// A non-empty `skill_id` wins (`sources.ts:133`).
#[derive(Debug, Deserialize)]
pub struct AddSourceSkillBody {
    pub skill_id: Option<String>,
    pub name: Option<String>,
    pub category: Option<String>,
}

async fn add_source_skill(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(body): Json<AddSourceSkillBody>,
) -> Result<Created<SkillRow>, ApiError> {
    let skill = with_conn(&state, move |conn| {
        if let Some(skill_id) = body.skill_id.as_deref().filter(|s| !s.is_empty()) {
            SourceStore::add_skill(conn, &id, skill_id)
        } else if let Some(name) = body.name.as_deref().filter(|n| !n.trim().is_empty()) {
            // Checks the source before creating anything (no orphan skill).
            SourceStore::add_skill_by_name(conn, &id, name, body.category.as_deref())
        } else {
            Err(forge_core::ForgeError::Validation {
                message: "skill_id or name is required".into(),
                field: None,
            })
        }
    })
    .await?;
    Ok(Created(skill))
}

// ── Router ──────────────────────────────────────────────────────────

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/sources", post(create_source).get(list_sources))
        .route(
            "/sources/{id}",
            get(get_source).patch(update_source).delete(delete_source),
        )
        .route(
            "/sources/{id}/derive-bullets",
            post(derive_bullets_replaced),
        )
        .route(
            "/sources/{id}/skills",
            get(list_source_skills).post(add_source_skill),
        )
}
