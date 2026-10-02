//! Review queue route — pending bullets and perspectives.
//!
//! Mirrors `packages/core/src/routes/review.ts`.

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};

use forge_core::ReviewQueue;
use forge_sdk::services::ReviewService;

use crate::db::with_conn;
use crate::error::ApiError;
use crate::response::ApiData;
use crate::state::SharedState;

// ── Handlers ────────────────────────────────────────────────────────

async fn get_pending_review(
    State(state): State<SharedState>,
) -> Result<Json<ApiData<ReviewQueue>>, ApiError> {
    let result = with_conn(&state, move |conn| {
        ReviewService::get_pending_review(conn)
    })
    .await?;
    Ok(Json(ApiData { data: result }))
}

// ── Router ──────────────────────────────────────────────────────────

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/review/pending", get(get_pending_review))
}
