//! Wire-format leniency the TypeScript server had and the web UI depends on.

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
    let bytes = axum::body::to_bytes(resp.into_body(), 1 << 20).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

/// The UI's "I have worked here" checkbox sends a JSON boolean; older clients send 0/1. Both
/// are stored as 0/1 and read back as a number.
#[tokio::test]
async fn organization_worked_accepts_a_boolean_or_a_number() {
    let r = router();

    for (sent, expected) in [(json!(true), 1), (json!(false), 0), (json!(1), 1), (json!(0), 0)] {
        let (status, created) =
            call(&r, "POST", "/api/organizations", Some(json!({ "name": format!("Org {sent}"), "worked": sent }))).await;
        assert_eq!(status, StatusCode::CREATED, "create with worked={sent}: {created}");
        assert_eq!(created["data"]["worked"], expected, "create with worked={sent}");
    }

    // PATCH takes the same shapes. (It still requires `name`: update reuses the create input.)
    let (_, org) = call(&r, "POST", "/api/organizations", Some(json!({ "name": "Patched" }))).await;
    let id = org["data"]["id"].as_str().unwrap().to_string();
    let (status, updated) = call(&r, "PATCH", &format!("/api/organizations/{id}"), Some(json!({ "name": "Patched", "worked": true }))).await;
    assert_eq!(status, StatusCode::OK, "{updated}");
    assert_eq!(updated["data"]["worked"], 1);

    // Anything else is still refused.
    let (status, _) = call(&r, "POST", "/api/organizations", Some(json!({ "name": "Bad", "worked": "yes" }))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

/// The same leniency for the other 0/1 flags a client sets: `role.is_current` here.
#[tokio::test]
async fn source_role_is_current_accepts_a_boolean() {
    let r = router();
    let (status, created) = call(
        &r,
        "POST",
        "/api/sources",
        Some(json!({ "title": "Engineer", "description": "Did things", "source_type": "role", "role": { "is_current": true } })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_eq!(created["data"]["role"]["is_current"], 1);
}
