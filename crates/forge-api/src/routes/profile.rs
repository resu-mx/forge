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
    Json(input): Json<UpdateProfile>,
) -> Result<Json<ApiData<forge_core::UserProfile>>, ApiError> {
    let result =
        with_conn(&state, move |conn| ProfileStore::update_profile(conn, &input)).await?;
    Ok(Json(ApiData { data: result }))
}

// ── Router ──────────────────────────────────────────────────────────

pub fn router() -> Router<SharedState> {
    Router::new().route("/profile", get(get_profile).patch(update_profile))
}
