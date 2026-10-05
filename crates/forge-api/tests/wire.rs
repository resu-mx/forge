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
    let bytes = axum::body::to_bytes(resp.into_body(), 1 << 20)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// The UI's "I have worked here" checkbox sends a JSON boolean; older clients send 0/1. Both
/// are stored as 0/1 and read back as a number.
#[tokio::test]
async fn organization_worked_accepts_a_boolean_or_a_number() {
    let r = router();

    for (sent, expected) in [
        (json!(true), 1),
        (json!(false), 0),
        (json!(1), 1),
        (json!(0), 0),
    ] {
        let (status, created) = call(
            &r,
            "POST",
            "/api/organizations",
            Some(json!({ "name": format!("Org {sent}"), "worked": sent })),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::CREATED,
            "create with worked={sent}: {created}"
        );
        assert_eq!(
            created["data"]["worked"], expected,
            "create with worked={sent}"
        );
    }

    // PATCH takes the same shapes.
    let (_, org) = call(
        &r,
        "POST",
        "/api/organizations",
        Some(json!({ "name": "Patched" })),
    )
    .await;
    let id = org["data"]["id"].as_str().unwrap().to_string();
    let (status, updated) = call(
        &r,
        "PATCH",
        &format!("/api/organizations/{id}"),
        Some(json!({ "worked": true })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{updated}");
    assert_eq!(updated["data"]["worked"], 1);

    // Anything else is still refused.
    let (status, _) = call(
        &r,
        "POST",
        "/api/organizations",
        Some(json!({ "name": "Bad", "worked": "yes" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

/// PATCH is partial, as in TS: the Kanban board sends `{status}` alone, and `{status: null}`
/// takes an organization off the board (resu-mx/forge#32).
#[tokio::test]
async fn organization_patch_is_partial() {
    let r = router();
    let (_, org) = call(
        &r,
        "POST",
        "/api/organizations",
        Some(json!({ "name": "Acme", "status": "backlog", "website": "https://acme.test" })),
    )
    .await;
    let id = org["data"]["id"].as_str().unwrap().to_string();
    let path = format!("/api/organizations/{id}");

    let (status, body) = call(&r, "PATCH", &path, Some(json!({ "status": "researching" }))).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["data"]["status"], "researching");
    assert_eq!(body["data"]["name"], "Acme");
    assert_eq!(body["data"]["website"], "https://acme.test");

    let (status, body) = call(&r, "PATCH", &path, Some(json!({ "status": null }))).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(body["data"]["status"].is_null());

    for bad in [
        json!({ "name": "  " }),
        json!({ "status": "bogus" }),
        json!({ "org_type": "guild" }),
    ] {
        let (status, body) = call(&r, "PATCH", &path, Some(bad.clone())).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{bad}: {body}");
        assert_eq!(body["error"]["code"], "VALIDATION_ERROR");
    }

    let (status, body) = call(
        &r,
        "PATCH",
        "/api/organizations/00000000-0000-4000-8000-000000000000",
        Some(json!({ "status": "backlog" })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    assert_eq!(body["error"]["code"], "NOT_FOUND");
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
