//! Certification CRUD routes with skill sub-resource.
//!
//! Mirrors the TS certification routes — full CRUD plus
//! certification_skills junction management.

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use forge_core::{
    Certification, CertificationWithSkills, CreateCertification, Skill, UpdateCertification,
};
use forge_sdk::db::CertificationStore;

use crate::db::with_conn;
use crate::error::ApiError;
use crate::response::{ApiData, Created, NoContent};
use crate::state::SharedState;

// -- Query params ────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AddSkillBody {
    pub skill_id: String,
}

// -- Handlers ────────────────────────────────────────────────────────

async fn create_certification(
    State(state): State<SharedState>,
    Json(input): Json<CreateCertification>,
) -> Result<Created<Certification>, ApiError> {
    let result = with_conn(&state, move |conn| CertificationStore::create(conn, &input)).await?;
    Ok(Created(result))
}

async fn list_certifications(
    State(state): State<SharedState>,
) -> Result<Json<ApiData<Vec<CertificationWithSkills>>>, ApiError> {
    // TS returns every row as {data}; offset/limit are not part of the contract.
    let data = with_conn(&state, move |conn| CertificationStore::list_with_skills(conn)).await?;
    Ok(Json(ApiData { data }))
}

async fn get_certification(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<ApiData<CertificationWithSkills>>, ApiError> {
    let data = with_conn(&state, move |conn| {
        CertificationStore::get_with_skills(conn, &id)?.ok_or_else(|| {
            forge_core::ForgeError::NotFound {
                entity_type: "Certification".into(),
                id: id.clone(),
            }
        })
    })
    .await?;
    Ok(Json(ApiData { data }))
}

async fn update_certification(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(input): Json<UpdateCertification>,
) -> Result<Json<ApiData<Certification>>, ApiError> {
    let result = with_conn(&state, move |conn| {
        CertificationStore::update(conn, &id, &input)
    })
    .await?;
    Ok(Json(ApiData { data: result }))
}

async fn delete_certification(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<NoContent, ApiError> {
    with_conn(&state, move |conn| CertificationStore::delete(conn, &id)).await?;
    Ok(NoContent)
}

// -- Skill sub-resource handlers ────────────────────────────────────

async fn get_certification_skills(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<ApiData<Vec<Skill>>>, ApiError> {
    let data = with_conn(&state, move |conn| {
        CertificationStore::get_skills(conn, &id)
    })
    .await?;
    Ok(Json(ApiData { data }))
}

async fn add_certification_skill(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(body): Json<AddSkillBody>,
) -> Result<NoContent, ApiError> {
    with_conn(&state, move |conn| {
        CertificationStore::add_skill(conn, &id, &body.skill_id)
    })
    .await?;
    Ok(NoContent)
}

async fn remove_certification_skill(
    State(state): State<SharedState>,
    Path((cert_id, skill_id)): Path<(String, String)>,
) -> Result<NoContent, ApiError> {
    with_conn(&state, move |conn| {
        CertificationStore::remove_skill(conn, &cert_id, &skill_id)
    })
    .await?;
    Ok(NoContent)
}

// -- Router ──────────────────────────────────────────────────────────

pub fn router() -> Router<SharedState> {
    Router::new()
        .route(
            "/certifications",
            post(create_certification).get(list_certifications),
        )
        .route(
            "/certifications/{id}",
            get(get_certification)
                .patch(update_certification)
                .delete(delete_certification),
        )
        .route(
            "/certifications/{id}/skills",
            get(get_certification_skills).post(add_certification_skill),
        )
        .route(
            "/certifications/{cert_id}/skills/{skill_id}",
            axum::routing::delete(remove_certification_skill),
        )
}
