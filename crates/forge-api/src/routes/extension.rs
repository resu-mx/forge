//! Extension routes: config and error logging for the browser extension.
//!
//! Mirrors `packages/core/src/routes/extension.ts`:
//! GET/PUT /extension/config, POST /extension/log, GET/DELETE /extension/logs.

use axum::extract::{Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::Value;

use forge_core::{
    CreateExtensionLog, ExtensionConfig, ExtensionLog, ExtensionLogFilter, ForgeError,
};
use forge_sdk::db::{ExtensionConfigStore, ExtensionLogStore};

use crate::db::with_conn;
use crate::error::ApiError;
use crate::response::{ApiData, Created, NoContent};
use crate::state::SharedState;

/// Strings, not `i64`: TS treats an empty `?limit=` as absent (extension.ts:49-50).
#[derive(Debug, Deserialize, Default)]
pub struct LogListQuery {
    pub limit: Option<String>,
    pub offset: Option<String>,
    pub error_code: Option<String>,
    pub layer: Option<String>,
}

fn validation(message: impl Into<String>, field: &str) -> ApiError {
    ForgeError::Validation {
        message: message.into(),
        field: Some(field.into()),
    }
    .into()
}

/// Absent or empty is `None` (the store default). Otherwise an integer, or 400.
fn int_param(raw: Option<&str>, name: &str) -> Result<Option<i64>, ApiError> {
    match raw {
        None | Some("") => Ok(None),
        Some(s) => s
            .parse::<i64>()
            .map(Some)
            .map_err(|_| validation(format!("{name} must be an integer"), name)),
    }
}

async fn get_config(
    State(state): State<SharedState>,
) -> Result<Json<ApiData<ExtensionConfig>>, ApiError> {
    let data = with_conn(&state, ExtensionConfigStore::get_all).await?;
    Ok(Json(ApiData { data }))
}

async fn update_config(
    State(state): State<SharedState>,
    Json(body): Json<Value>,
) -> Result<Json<ApiData<ExtensionConfig>>, ApiError> {
    // TS: `!body.updates || typeof body.updates !== 'object'` (extension.ts:28). Only
    // objects and arrays pass; `Object.entries` turns an array into index keys.
    let updates: Vec<(String, Value)> = match body.get("updates") {
        Some(Value::Object(map)) => map.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        Some(Value::Array(items)) => items
            .iter()
            .enumerate()
            .map(|(i, v)| (i.to_string(), v.clone()))
            .collect(),
        _ => {
            return Err(validation(
                r#"Body must contain an "updates" object"#,
                "updates",
            ))
        }
    };
    let data = with_conn(&state, move |conn| {
        ExtensionConfigStore::set_many(conn, &updates)
    })
    .await?;
    Ok(Json(ApiData { data }))
}

async fn append_log(
    State(state): State<SharedState>,
    Json(input): Json<CreateExtensionLog>,
) -> Result<Created<ExtensionLog>, ApiError> {
    let data = with_conn(&state, move |conn| ExtensionLogStore::append(conn, &input)).await?;
    Ok(Created(data))
}

async fn list_logs(
    State(state): State<SharedState>,
    Query(q): Query<LogListQuery>,
) -> Result<Json<ApiData<Vec<ExtensionLog>>>, ApiError> {
    let filter = ExtensionLogFilter {
        limit: int_param(q.limit.as_deref(), "limit")?,
        offset: int_param(q.offset.as_deref(), "offset")?,
        error_code: q.error_code,
        layer: q.layer,
    };
    // A bare array with no `pagination` object, as the TS route returns (extension.ts:56).
    let data = with_conn(&state, move |conn| ExtensionLogStore::list(conn, &filter)).await?;
    Ok(Json(ApiData { data }))
}

async fn clear_logs(State(state): State<SharedState>) -> Result<NoContent, ApiError> {
    with_conn(&state, |conn| ExtensionLogStore::clear(conn).map(|_| ())).await?;
    Ok(NoContent)
}

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/extension/config", get(get_config).put(update_config))
        .route("/extension/log", post(append_log))
        .route("/extension/logs", get(list_logs).delete(clear_logs))
}
