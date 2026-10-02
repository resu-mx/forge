//! In-process tests of the real router over an in-memory database.
//!
//! Runs without the `tokio` feature, i.e. the same inline `with_conn` path the
//! browser host uses.

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

async fn raw(router: &Router, req: Request<Body>) -> (StatusCode, Value) {
    let resp = router.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 1 << 20).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

#[tokio::test]
async fn health() {
    let (status, body) = call(&router(), "GET", "/api/health", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["server"], "ok");
}

#[tokio::test]
async fn source_crud_uses_the_ts_envelopes() {
    let r = router();

    let (status, created) = call(&r, "POST", "/api/sources", Some(json!({"title": "T", "description": "D"}))).await;
    assert_eq!(status, StatusCode::CREATED);
    let id = created["data"]["id"].as_str().unwrap().to_string();

    let (status, got) = call(&r, "GET", &format!("/api/sources/{id}"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(got["data"]["title"], "T");

    let (status, list) = call(&r, "GET", "/api/sources", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list["pagination"]["total"], 1);
    assert_eq!(list["pagination"]["offset"], 0);
    assert_eq!(list["data"].as_array().unwrap().len(), 1);

    let (status, body) = call(&r, "DELETE", &format!("/api/sources/{id}"), None).await;
    assert_eq!((status, body), (StatusCode::NO_CONTENT, Value::Null));

    let (status, missing) = call(&r, "GET", &format!("/api/sources/{id}"), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(missing["error"]["code"], "NOT_FOUND");
}

#[tokio::test]
async fn unknown_route_is_a_json_404() {
    let (status, body) = call(&router(), "GET", "/api/nope", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"]["code"], "NOT_FOUND");
}

#[tokio::test]
async fn extractor_rejections_are_wrapped_in_the_error_envelope() {
    let r = router();

    // Malformed JSON.
    let req = Request::builder()
        .method("POST")
        .uri("/api/sources")
        .header("content-type", "application/json")
        .body(Body::from("{not json"))
        .unwrap();
    let (status, body) = raw(&r, req).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "VALIDATION_ERROR");

    // Missing required field (axum answers 422; the TS API answers 400).
    let (status, body) = call(&r, "POST", "/api/sources", Some(json!({"title": "only"}))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "VALIDATION_ERROR");

    // Wrong content type.
    let req = Request::builder()
        .method("POST")
        .uri("/api/sources")
        .header("content-type", "text/plain")
        .body(Body::from("x"))
        .unwrap();
    let (status, body) = raw(&r, req).await;
    assert_eq!(status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    assert_eq!(body["error"]["code"], "UNSUPPORTED_MEDIA_TYPE");
}

#[tokio::test]
async fn templates_crud_and_builtin_protection() {
    let r = router();

    let (status, list) = call(&r, "GET", "/api/templates", None).await;
    assert_eq!(status, StatusCode::OK);
    let all = list["data"].as_array().unwrap();
    assert!(all.len() >= 3);
    assert_eq!(all[0]["is_builtin"], 1, "built-ins sort first");
    let builtin_id = all[0]["id"].as_str().unwrap().to_string();

    let (status, created) = call(
        &r,
        "POST",
        "/api/templates",
        Some(json!({"name": "Mine", "sections": [{"title": "Work", "entry_type": "experience", "position": 5}]})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(created["data"]["sections"][0]["position"], 0);
    let id = created["data"]["id"].as_str().unwrap().to_string();

    let (status, patched) = call(&r, "PATCH", &format!("/api/templates/{id}"), Some(json!({"name": "Renamed"}))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(patched["data"]["name"], "Renamed");

    let (status, err) = call(&r, "DELETE", &format!("/api/templates/{builtin_id}"), None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(err["error"]["code"], "VALIDATION_ERROR");

    let (status, _) = call(&r, "DELETE", &format!("/api/templates/{id}"), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call(&r, "GET", &format!("/api/templates/{id}"), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, err) = call(
        &r,
        "POST",
        "/api/templates",
        Some(json!({"name": "Bad", "sections": [{"title": "X", "entry_type": "nonsense", "position": 0}]})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(err["error"]["code"], "VALIDATION_ERROR");
}

#[tokio::test]
async fn post_resumes_with_template_id_creates_the_sections() {
    let r = router();
    let (_, list) = call(&r, "GET", "/api/templates", None).await;
    let template = &list["data"][0];
    let template_id = template["id"].as_str().unwrap();
    let expected = template["sections"].as_array().unwrap().len();

    let body = json!({
        "name": "R", "target_role": "Engineer", "target_employer": "Acme",
        "archetype": "backend", "template_id": template_id
    });
    let (status, created) = call(&r, "POST", "/api/resumes", Some(body)).await;
    assert_eq!(status, StatusCode::CREATED);
    let id = created["data"]["id"].as_str().unwrap();

    let (status, full) = call(&r, "GET", &format!("/api/resumes/{id}"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(full["data"]["sections"].as_array().unwrap().len(), expected);

    // Unknown template: 404 and no resume left behind.
    let body = json!({
        "name": "R2", "target_role": "E", "target_employer": "A",
        "archetype": "x", "template_id": "does-not-exist"
    });
    let (status, err) = call(&r, "POST", "/api/resumes", Some(body)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(err["error"]["code"], "NOT_FOUND");
    let (_, resumes) = call(&r, "GET", "/api/resumes", None).await;
    assert_eq!(resumes["pagination"]["total"], 1);

    // Plain create (no template_id) still works.
    let body = json!({"name": "Plain", "target_role": "E", "target_employer": "A", "archetype": "x"});
    let (status, _) = call(&r, "POST", "/api/resumes", Some(body)).await;
    assert_eq!(status, StatusCode::CREATED);
}
