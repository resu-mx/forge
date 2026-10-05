//! Transport-agnostic Forge HTTP API.
//!
//! One axum [`Router`] serving the same JSON contract as the TypeScript API
//! (`packages/core/src/routes`). It is hosted natively by `forge-server` (tokio +
//! hyper) and, in roadmap M4, inside a browser Worker by `forge-wasm`, so this
//! crate must not depend on tokio or any other native-only crate unless behind a
//! feature.

pub mod db;
pub mod error;
pub mod response;
pub mod routes;
pub mod state;

use axum::body::Body;
use axum::http::{header, Response, StatusCode};
use axum::middleware::map_response;
use axum::Router;

pub use error::ApiError;
pub use state::{AppState, SharedState};

/// The full API, with state attached and error envelopes normalised.
pub fn app(state: SharedState) -> Router {
    routes::api_router()
        .with_state(state)
        .layer(map_response(envelope_plain_errors))
}

/// axum's own rejections (malformed JSON, wrong content type, bad path or query)
/// are plain text. The TS API always answers errors as `{"error":{"code","message"}}`,
/// so wrap any 4xx/5xx that is not already JSON.
async fn envelope_plain_errors(resp: Response<Body>) -> Response<Body> {
    let status = resp.status();
    if !(status.is_client_error() || status.is_server_error()) {
        return resp;
    }
    let is_json = resp
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("application/json"));
    if is_json {
        return resp;
    }

    let (code, status) = match status {
        StatusCode::BAD_REQUEST | StatusCode::UNPROCESSABLE_ENTITY => {
            ("VALIDATION_ERROR", StatusCode::BAD_REQUEST)
        }
        StatusCode::NOT_FOUND => ("NOT_FOUND", status),
        StatusCode::METHOD_NOT_ALLOWED => ("METHOD_NOT_ALLOWED", status),
        StatusCode::UNSUPPORTED_MEDIA_TYPE => ("UNSUPPORTED_MEDIA_TYPE", status),
        s if s.is_server_error() => ("INTERNAL_ERROR", status),
        _ => ("BAD_REQUEST", status),
    };

    let bytes = axum::body::to_bytes(resp.into_body(), 64 * 1024)
        .await
        .unwrap_or_default();
    let mut message = String::from_utf8_lossy(&bytes).trim().to_string();
    if message.is_empty() {
        message = status.canonical_reason().unwrap_or("error").to_string();
    }

    let body = serde_json::json!({ "error": { "code": code, "message": message } });
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("static response parts are valid")
}
