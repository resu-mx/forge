//! Resume template routes.
//!
//! Mirrors `packages/core/src/routes/templates.ts` — same paths, same JSON shapes.
//! Validation lives in `TemplateStore` (port of the TS `TemplateService`).

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};

use forge_core::{CreateResumeTemplate, ResumeTemplate, UpdateResumeTemplate};
use forge_sdk::db::TemplateStore;

use crate::db::with_conn;
use crate::error::ApiError;
use crate::response::{ApiData, Created, NoContent};
use crate::state::SharedState;

async fn list_templates(
    State(state): State<SharedState>,
) -> Result<Json<ApiData<Vec<ResumeTemplate>>>, ApiError> {
    let data = with_conn(&state, TemplateStore::list).await?;
    Ok(Json(ApiData { data }))
}

async fn get_template(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<ApiData<ResumeTemplate>>, ApiError> {
    let data = with_conn(&state, move |conn| {
        TemplateStore::get(conn, &id)?.ok_or_else(|| forge_core::ForgeError::NotFound {
            entity_type: "template".into(),
            id: id.clone(),
        })
    })
    .await?;
    Ok(Json(ApiData { data }))
}

async fn create_template(
    State(state): State<SharedState>,
    Json(input): Json<CreateResumeTemplate>,
) -> Result<Created<ResumeTemplate>, ApiError> {
    let data = with_conn(&state, move |conn| TemplateStore::create(conn, &input)).await?;
    Ok(Created(data))
}

async fn update_template(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(patch): Json<UpdateResumeTemplate>,
) -> Result<Json<ApiData<ResumeTemplate>>, ApiError> {
    let data = with_conn(&state, move |conn| TemplateStore::update(conn, &id, &patch)).await?;
    Ok(Json(ApiData { data }))
}

async fn delete_template(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<NoContent, ApiError> {
    with_conn(&state, move |conn| TemplateStore::delete(conn, &id)).await?;
    Ok(NoContent)
}

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/templates", get(list_templates).post(create_template))
        .route(
            "/templates/{id}",
            get(get_template).patch(update_template).delete(delete_template),
        )
}
