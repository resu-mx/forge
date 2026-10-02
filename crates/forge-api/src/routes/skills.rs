//! Skill CRUD routes.

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use forge_core::{Skill, SkillCategory};
use forge_sdk::db::SkillStore;

use crate::db::with_conn;
use crate::error::ApiError;
use crate::response::{ApiData, Created, NoContent};
use crate::state::SharedState;

// ── Query params ────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Default)]
pub struct SkillListQuery {
    pub category: Option<String>,
    pub domain_id: Option<String>,
    pub search: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSkillBody {
    pub name: String,
    pub category: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSkillBody {
    pub name: Option<String>,
    pub category: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MergeSkillBody {
    pub target_id: String,
}

// ── Handlers ────────────────────────────────────────────────────────

async fn create_skill(
    State(state): State<SharedState>,
    Json(body): Json<CreateSkillBody>,
) -> Result<Created<Skill>, ApiError> {
    let category = body
        .category
        .and_then(|c| c.parse::<SkillCategory>().ok());
    let result =
        with_conn(&state, move |conn| SkillStore::create(conn, &body.name, category)).await?;
    Ok(Created(result))
}

async fn list_skills(
    State(state): State<SharedState>,
    Query(q): Query<SkillListQuery>,
) -> Result<Json<ApiData<Vec<Skill>>>, ApiError> {
    let category = q.category.and_then(|c| c.parse::<SkillCategory>().ok());
    let domain_id = q.domain_id;
    let search = q.search;
    let data = with_conn(&state, move |conn| {
        SkillStore::list(conn, category, domain_id.as_deref(), search.as_deref())
    })
    .await?;
    Ok(Json(ApiData { data }))
}

async fn get_skill(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<ApiData<Skill>>, ApiError> {
    let result = with_conn(&state, move |conn| {
        SkillStore::get(conn, &id)?.ok_or_else(|| forge_core::ForgeError::NotFound {
            entity_type: "Skill".into(),
            id: id.clone(),
        })
    })
    .await?;
    Ok(Json(ApiData { data: result }))
}

async fn update_skill(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateSkillBody>,
) -> Result<Json<ApiData<Skill>>, ApiError> {
    let category = body
        .category
        .and_then(|c| c.parse::<SkillCategory>().ok());
    let result = with_conn(&state, move |conn| {
        SkillStore::update(conn, &id, body.name.as_deref(), category)
    })
    .await?;
    Ok(Json(ApiData { data: result }))
}

async fn delete_skill(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<NoContent, ApiError> {
    with_conn(&state, move |conn| SkillStore::delete(conn, &id)).await?;
    Ok(NoContent)
}

async fn merge_skills(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(body): Json<MergeSkillBody>,
) -> Result<Json<ApiData<Skill>>, ApiError> {
    let result =
        with_conn(&state, move |conn| SkillStore::merge(conn, &id, &body.target_id)).await?;
    Ok(Json(ApiData { data: result }))
}

// ── Router ──────────────────────────────────────────────────────────

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/skills", post(create_skill).get(list_skills))
        .route(
            "/skills/{id}",
            get(get_skill).patch(update_skill).delete(delete_skill),
        )
        .route("/skills/{id}/merge", post(merge_skills))
}
