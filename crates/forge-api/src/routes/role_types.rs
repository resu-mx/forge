//! RoleType CRUD routes.
//!
//! Simple lookup table — no pagination (resu-mx/forge#141).

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};

use forge_core::{CreateRoleTypeInput, RoleType, UpdateRoleTypeInput};
use forge_sdk::db::RoleTypeStore;

use crate::db::with_conn;
use crate::error::ApiError;
use crate::response::{ApiData, Created, NoContent};
use crate::state::SharedState;

// -- Handlers ────────────────────────────────────────────────────────

async fn create_role_type(
    State(state): State<SharedState>,
    Json(input): Json<CreateRoleTypeInput>,
) -> Result<Created<RoleType>, ApiError> {
    let result = with_conn(&state, move |conn| RoleTypeStore::create(conn, &input)).await?;
    Ok(Created(result))
}

async fn list_role_types(
    State(state): State<SharedState>,
) -> Result<Json<ApiData<Vec<RoleType>>>, ApiError> {
    let data = with_conn(&state, move |conn| RoleTypeStore::list(conn)).await?;
    Ok(Json(ApiData { data }))
}

async fn get_role_type(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<ApiData<RoleType>>, ApiError> {
    let result = with_conn(&state, move |conn| {
        RoleTypeStore::get(conn, &id)?.ok_or_else(|| forge_core::ForgeError::NotFound {
            entity_type: "RoleType".into(),
            id: id.clone(),
        })
    })
    .await?;
    Ok(Json(ApiData { data: result }))
}

/// `PATCH /role-types/:id` (TS role-types.ts:33-38). 200 with the row, 400 for a blank name,
/// 404 for an unknown id, 409 for a name another role type has.
async fn update_role_type(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(input): Json<UpdateRoleTypeInput>,
) -> Result<Json<ApiData<RoleType>>, ApiError> {
    let result = with_conn(&state, move |conn| RoleTypeStore::update(conn, &id, &input)).await?;
    Ok(Json(ApiData { data: result }))
}

async fn delete_role_type(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<NoContent, ApiError> {
    with_conn(&state, move |conn| RoleTypeStore::delete(conn, &id)).await?;
    Ok(NoContent)
}

// -- Router ──────────────────────────────────────────────────────────

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/role-types", post(create_role_type).get(list_role_types))
        .route(
            "/role-types/{id}",
            get(get_role_type)
                .patch(update_role_type)
                .delete(delete_role_type),
        )
}
