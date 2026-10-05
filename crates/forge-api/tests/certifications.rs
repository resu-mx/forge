//! Router tests for /api/certifications (resu-mx/forge#38). Same harness as tests/resources.rs.

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
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let value = if text.is_empty() {
        Value::Null
    } else {
        serde_json::from_str(&text).unwrap_or(Value::Null)
    };
    (status, value)
}

async fn create_cert(r: &Router, short: &str) -> String {
    let (status, body) = call(
        r,
        "POST",
        "/api/certifications",
        Some(json!({ "short_name": short, "long_name": format!("{short} long") })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    // POST stays a plain Certification (no skills key).
    assert!(body["data"].get("skills").is_none());
    body["data"]["id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn list_is_hydrated_unpaged_and_sorted_by_short_name() {
    let r = router();
    for short in ["PMP", "CISSP", "AWS"] {
        create_cert(&r, short).await;
    }
    let (status, body) = call(&r, "GET", "/api/certifications", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.get("pagination").is_none());
    let rows = body["data"].as_array().unwrap();
    let names: Vec<_> = rows
        .iter()
        .map(|c| c["short_name"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(names, ["AWS", "CISSP", "PMP"]);
    assert!(rows.iter().all(|c| c["skills"].is_array()));
}

#[tokio::test]
async fn list_is_not_capped_at_50() {
    let r = router();
    for i in 0..51 {
        create_cert(&r, &format!("C{i:02}")).await;
    }
    // offset/limit are ignored, not rejected.
    let (status, body) = call(&r, "GET", "/api/certifications?limit=10&offset=5", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"].as_array().unwrap().len(), 51);
}

#[tokio::test]
async fn get_returns_hydrated_certification_and_404_for_unknown() {
    let r = router();
    let id = create_cert(&r, "CKA").await;
    let (status, body) = call(&r, "GET", &format!("/api/certifications/{id}"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["short_name"], "CKA");
    assert_eq!(body["data"]["skills"], json!([]));

    let (status, body) = call(&r, "GET", "/api/certifications/nonexistent", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"]["code"], "NOT_FOUND");
}
