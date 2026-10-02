//! Answer bank routes — list, upsert, and delete profile answers.
//!
//! Mirrors `packages/core/src/routes/answer-bank.ts`. All routes are
//! nested under `/profile/answers`.

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};

use forge_core::{AnswerBankEntry, UpsertAnswerInput};
use forge_sdk::db::AnswerBankStore;

use crate::db::with_conn;
use crate::error::ApiError;
use crate::response::{ApiData, NoContent};
use crate::state::SharedState;

// -- Handlers ────────────────────────────────────────────────────────

async fn list_answers(
    State(state): State<SharedState>,
) -> Result<Json<ApiData<Vec<AnswerBankEntry>>>, ApiError> {
    let data = with_conn(&state, move |conn| AnswerBankStore::list(conn)).await?;
    Ok(Json(ApiData { data }))
}

async fn upsert_answer(
    State(state): State<SharedState>,
    Json(input): Json<UpsertAnswerInput>,
) -> Result<Json<ApiData<AnswerBankEntry>>, ApiError> {
    let data =
        with_conn(&state, move |conn| AnswerBankStore::upsert(conn, &input)).await?;
    Ok(Json(ApiData { data }))
}

async fn delete_answer(
    State(state): State<SharedState>,
    Path(field_kind): Path<String>,
) -> Result<NoContent, ApiError> {
    with_conn(&state, move |conn| {
        AnswerBankStore::delete_by_field_kind(conn, &field_kind)
    })
    .await?;
    Ok(NoContent)
}

// -- Router ──────────────────────────────────────────────────────────

pub fn router() -> Router<SharedState> {
    Router::new()
        .route(
            "/profile/answers",
            get(list_answers).put(upsert_answer),
        )
        .route(
            "/profile/answers/{field_kind}",
            axum::routing::delete(delete_answer),
        )
}
