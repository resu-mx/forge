//! Router tests for /api/domains (resu-mx/forge#37). In-process, in-memory database,
//! the inline (no-tokio) `with_conn` path, same as tests/resources.rs.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use forge_api::{app, AppState};
use forge_sdk::Forge;
use serde_json::{json, Value};
use tower::ServiceExt;

const SECURITY: &str = "d0000001-0000-4000-8000-000000000003";
const SECURITY_ENGINEER: &str = "a0000001-0000-4000-8000-000000000003";

fn router() -> Router {
    app(AppState::new(Forge::open_memory().unwrap()))
}

async fn call(router: &Router, method: &str, path: &str, body: Option<Value>) -> (StatusCode, Value) {
    let mut req = Request::builder().method(method).uri(path);
    let body = match body {
        Some(v) => {
            req = req.header("content-type", "application/json");
            Body::from(v.to_string())
        }
        None => Body::empty(),
    };
    let resp = router.clone().oneshot(req.body(body).unwrap()).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 1 << 20).await.unwrap();
    let value = if bytes.is_empty() { Value::Null } else { serde_json::from_slice(&bytes).unwrap_or(Value::Null) };
    (status, value)
}

#[tokio::test]
async fn deleting_a_domain_linked_to_archetypes_is_a_409_and_changes_nothing() {
    let r = router();
    let (status, err) = call(&r, "DELETE", &format!("/api/domains/{SECURITY}"), None).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(err["error"]["code"], "CONFLICT");
    assert!(err["error"]["message"]
        .as_str()
        .unwrap()
        .contains("Cannot delete domain 'security': associated with 2 archetype(s)"));

    let (status, _) = call(&r, "GET", &format!("/api/domains/{SECURITY}"), None).await;
    assert_eq!(status, StatusCode::OK);
    let (_, links) = call(&r, "GET", &format!("/api/archetypes/{SECURITY_ENGINEER}/domains"), None).await;
    assert!(links["data"].as_array().unwrap().iter().any(|d| d["name"] == "security"));
}

#[tokio::test]
async fn deleting_a_domain_named_by_a_perspective_is_a_409() {
    let r = router();
    let (_, d) = call(&r, "POST", "/api/domains", Some(json!({ "name": "block_delete" }))).await;
    let (_, b) = call(&r, "POST", "/api/bullets", Some(json!({ "content": "Built APIs" }))).await;
    let (status, _) = call(
        &r,
        "POST",
        "/api/perspectives",
        Some(json!({
            "bullet_id": b["data"]["id"], "content": "Designed APIs", "domain": "block_delete"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let id = d["data"]["id"].as_str().unwrap();
    let (status, err) = call(&r, "DELETE", &format!("/api/domains/{id}"), None).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert!(err["error"]["message"].as_str().unwrap().contains("referenced by 1 perspective(s)"));
}

#[tokio::test]
async fn list_has_usage_counts_and_pagination() {
    let r = router();
    let (status, body) = call(&r, "GET", "/api/domains", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["pagination"]["offset"], 0);
    assert_eq!(body["pagination"]["limit"], 50);
    let items = body["data"].as_array().unwrap();
    assert_eq!(body["pagination"]["total"].as_u64().unwrap() as usize, items.len());
    let security = items.iter().find(|d| d["name"] == "security").unwrap();
    assert_eq!(security["archetype_count"], 2);
    assert!(items.iter().all(|d| d["perspective_count"].is_i64() && d["archetype_count"].is_i64()));

    let (_, page) = call(&r, "GET", "/api/domains?offset=1&limit=2", None).await;
    assert_eq!(page["data"].as_array().unwrap().len(), 2);
    assert_eq!(page["pagination"]["offset"], 1);
    assert_eq!(page["pagination"]["limit"], 2);

    let (_, big) = call(&r, "GET", "/api/domains?limit=500&offset=-3", None).await;
    assert_eq!(big["pagination"]["limit"], 200);
    assert_eq!(big["pagination"]["offset"], 0);
}

#[tokio::test]
async fn deleting_an_unused_domain_is_204_and_unknown_is_404() {
    let r = router();
    let (_, d) = call(&r, "POST", "/api/domains", Some(json!({ "name": "unused_dom" }))).await;
    let id = d["data"]["id"].as_str().unwrap();

    let (status, _) = call(&r, "DELETE", &format!("/api/domains/{id}"), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call(&r, "GET", &format!("/api/domains/{id}"), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, err) = call(&r, "DELETE", "/api/domains/does-not-exist", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(err["error"]["code"], "NOT_FOUND");
}
