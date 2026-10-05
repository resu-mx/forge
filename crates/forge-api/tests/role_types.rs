//! Router tests for POST /api/role-types name rules (resu-mx/forge#140). In-process,
//! in-memory database, the inline (no-tokio) `with_conn` path.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use forge_api::{app, AppState};
use forge_sdk::Forge;
use serde_json::{json, Value};
use tower::ServiceExt;

fn router() -> Router {
    app(AppState::new(Forge::open_memory().unwrap()))
}

async fn call(
    router: &Router,
    method: &str,
    path: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut req = Request::builder().method(method).uri(path);
    let body = match body {
        Some(v) => {
            req = req.header("content-type", "application/json");
            Body::from(v.to_string())
        }
        None => Body::empty(),
    };
    let resp = router
        .clone()
        .oneshot(req.body(body).unwrap())
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 1 << 20)
        .await
        .unwrap();
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, value)
}

#[tokio::test]
async fn post_trims_rejects_blank_and_duplicate_names() {
    let r = router();
    let (status, body) = call(
        &r,
        "POST",
        "/api/role-types",
        Some(json!({ "name": "  Brand New  " })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["data"]["name"], "Brand New");

    let (status, err) = call(
        &r,
        "POST",
        "/api/role-types",
        Some(json!({ "name": "   " })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(err["error"]["code"], "VALIDATION_ERROR");
    assert!(err["error"]["message"]
        .as_str()
        .unwrap()
        .contains("Name must not be empty"));

    let (status, err) = call(
        &r,
        "POST",
        "/api/role-types",
        Some(json!({ "name": "Brand New" })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(err["error"]["code"], "CONFLICT");

    let (status, err) = call(&r, "POST", "/api/role-types", Some(json!({}))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(err["error"]["code"], "VALIDATION_ERROR");
}
