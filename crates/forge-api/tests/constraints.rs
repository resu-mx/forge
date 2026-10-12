//! SQLite constraint failures answer with the TS codes, not `500 DATABASE_ERROR`
//! (resu-mx/forge#98). The generic mapping lives in `forge_core::sqlite_error`.

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

async fn call(r: &Router, method: &str, path: &str, body: Option<Value>) -> (StatusCode, Value) {
    let mut req = Request::builder().method(method).uri(path);
    let body = match body {
        Some(v) => {
            req = req.header("content-type", "application/json");
            Body::from(v.to_string())
        }
        None => Body::empty(),
    };
    let resp = r.clone().oneshot(req.body(body).unwrap()).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 1 << 20)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

fn code(v: &Value) -> &str {
    v["error"]["code"].as_str().unwrap_or("")
}

#[tokio::test]
async fn duplicate_note_reference_is_409_conflict() {
    let r = router();
    let (s, n) = call(&r, "POST", "/api/notes", Some(json!({ "content": "n" }))).await;
    assert_eq!(s, StatusCode::CREATED, "{n}");
    let path = format!("/api/notes/{}/references", n["data"]["id"].as_str().unwrap());
    let body = json!({ "entity_type": "skill", "entity_id": "11111111-1111-4111-8111-111111111111" });

    let (s, _) = call(&r, "POST", &path, Some(body.clone())).await;
    assert_eq!(s, StatusCode::CREATED);
    let (s, e) = call(&r, "POST", &path, Some(body)).await;
    assert_eq!(s, StatusCode::CONFLICT, "{e}");
    assert_eq!(code(&e), "CONFLICT");
    assert!(!e.to_string().contains("UNIQUE constraint"), "leaks SQL: {e}");
}

#[tokio::test]
async fn dangling_reference_is_400_validation_error() {
    // No route pre-check for the organization: the foreign key is the only guard.
    let r = router();
    let (s, e) = call(
        &r,
        "POST",
        "/api/organizations/22222222-2222-4222-8222-222222222222/locations",
        Some(json!({ "name": "HQ" })),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "{e}");
    assert_eq!(code(&e), "VALIDATION_ERROR");
}

#[tokio::test]
async fn foreign_key_error_never_serializes_as_fk_violation() {
    let err = forge_core::ForgeError::ForeignKey {
        message: "x".into(),
    };
    assert_eq!(err.code(), "VALIDATION_ERROR");
}
