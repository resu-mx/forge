//! Profile routes — singleton user profile.
//!
//! Mirrors `packages/core/src/routes/profile.ts` — same URL paths,
//! same JSON shapes, so the webui and MCP server continue working.

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};

use forge_core::UpdateProfile;
use forge_sdk::db::ProfileStore;

use crate::db::with_conn;
use crate::error::ApiError;
use crate::response::ApiData;
use crate::state::SharedState;

// ── Handlers ────────────────────────────────────────────────────────

async fn get_profile(
    State(state): State<SharedState>,
) -> Result<Json<ApiData<forge_core::UserProfile>>, ApiError> {
    let result = with_conn(&state, move |conn| {
        ProfileStore::get_profile(conn)?.ok_or_else(|| forge_core::ForgeError::NotFound {
            entity_type: "Profile".into(),
            id: "singleton".into(),
        })
    })
    .await?;
    Ok(Json(ApiData { data: result }))
}

async fn update_profile(
    State(state): State<SharedState>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<ApiData<forge_core::UserProfile>>, ApiError> {
    // `UpdateProfile.name` cannot tell `null` from absent, but the TS API rejects
    // an explicit null, so look at the raw body.
    if body.get("name").is_some_and(serde_json::Value::is_null) {
        return Err(ApiError(forge_core::ForgeError::Validation {
            message: "Name cannot be null".into(),
            field: Some("name".into()),
        }));
    }
    let input: UpdateProfile = serde_json::from_value(body).map_err(|e| {
        ApiError(forge_core::ForgeError::Validation {
            message: e.to_string(),
            field: None,
        })
    })?;
    let result = with_conn(&state, move |conn| {
        ProfileStore::update_profile(conn, &input)
    })
    .await?;
    Ok(Json(ApiData { data: result }))
}

// ── Router ──────────────────────────────────────────────────────────

pub fn router() -> Router<SharedState> {
    Router::new().route("/profile", get(get_profile).patch(update_profile))
}
