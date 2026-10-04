//! Summary linked-resumes and keyword-skill routes (resu-mx/forge#30, endpoints 13-16).
//!
//! Same setup as `api.rs`: the real router over an in-memory database, on the inline
//! `with_conn` path the browser host uses.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use forge_api::{app, AppState};
use forge_sdk::Forge;
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use tower::ServiceExt;

fn router() -> Router {
    app(AppState::new(Forge::open_memory().unwrap()))
}

/// Seed rows with explicit `updated_at` values before the router takes the database:
/// `updated_at` has one-second precision, so API-created rows can tie.
fn router_with(seed: impl FnOnce(&Connection)) -> Router {
    let forge = Forge::open_memory().unwrap();
    seed(forge.conn());
    app(AppState::new(forge))
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

const S: &str = "11111111-1111-1111-1111-111111111111";

async fn summary_and_skill(r: &Router) -> (String, String) {
    let (_, s) = call(r, "POST", "/api/summaries", Some(json!({ "title": "Infra" }))).await;
    let (_, k) = call(r, "POST", "/api/skills", Some(json!({ "name": "Zzkube", "category": "tool" }))).await;
    (s["data"]["id"].as_str().unwrap().to_string(), k["data"]["id"].as_str().unwrap().to_string())
}

#[tokio::test]
async fn linked_resumes_lists_newest_first_with_pagination() {
    let r = router_with(|c| {
        c.execute("INSERT INTO summaries (id, title) VALUES (?1, 'Shared')", params![S]).unwrap();
        for (id, name, at) in [
            ("aaaaaaaa-0000-0000-0000-000000000001", "Older", "2026-01-01T00:00:00Z"),
            ("aaaaaaaa-0000-0000-0000-000000000002", "Newer", "2026-02-01T00:00:00Z"),
        ] {
            c.execute(
                "INSERT INTO resumes (id, name, target_role, target_employer, archetype, summary_id, updated_at)
                 VALUES (?1, ?2, 'SRE', 'Acme', 'backend', ?3, ?4)",
                params![id, name, S, at],
            )
            .unwrap();
        }
    });
    let (status, body) = call(&r, "GET", &format!("/api/summaries/{S}/linked-resumes?limit=1"), None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["data"][0]["name"], "Newer");
    assert_eq!(body["data"].as_array().unwrap().len(), 1);
    assert_eq!(body["pagination"], json!({ "total": 2, "offset": 0, "limit": 1 }));
}

#[tokio::test]
async fn linked_resumes_of_unknown_summary_is_404() {
    let (status, body) = call(&router(), "GET", "/api/summaries/nope/linked-resumes", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"]["code"], "NOT_FOUND");
}

#[tokio::test]
async fn skills_of_unknown_summary_is_404() {
    let (status, body) = call(&router(), "GET", "/api/summaries/nope/skills", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"]["code"], "NOT_FOUND");
}

#[tokio::test]
async fn add_skill_answers_204_empty_and_is_idempotent() {
    let r = router();
    let (sid, kid) = summary_and_skill(&r).await;
    for _ in 0..2 {
        let (status, body) =
            call(&r, "POST", &format!("/api/summaries/{sid}/skills"), Some(json!({ "skill_id": kid }))).await;
        assert_eq!(status, StatusCode::NO_CONTENT);
        assert_eq!(body, Value::Null, "204 must have an empty body");
    }
    let (_, list) = call(&r, "GET", &format!("/api/summaries/{sid}/skills"), None).await;
    assert_eq!(list["data"].as_array().unwrap().len(), 1);
    assert_eq!(list["data"][0]["id"], kid.as_str());
    assert_eq!(list["data"][0]["name"], "Zzkube");
    assert_eq!(list["data"][0]["category"], "tool");
}

#[tokio::test]
async fn add_skill_without_skill_id_is_400() {
    let r = router();
    let (sid, _) = summary_and_skill(&r).await;
    for (id, body) in [(sid.as_str(), json!({})), (sid.as_str(), json!({ "skill_id": "" })), ("nope", json!({}))] {
        let (status, resp) = call(&r, "POST", &format!("/api/summaries/{id}/skills"), Some(body)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{id}");
        assert_eq!(resp["error"]["code"], "VALIDATION_ERROR");
    }
}

#[tokio::test]
async fn add_skill_to_unknown_summary_or_unknown_skill_is_404() {
    let r = router();
    let (sid, kid) = summary_and_skill(&r).await;
    let (status, body) =
        call(&r, "POST", "/api/summaries/nope/skills", Some(json!({ "skill_id": kid }))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"]["code"], "NOT_FOUND");
    let (status, body) =
        call(&r, "POST", &format!("/api/summaries/{sid}/skills"), Some(json!({ "skill_id": "nope" }))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"]["code"], "NOT_FOUND");
}

#[tokio::test]
async fn remove_skill_is_idempotent() {
    let r = router();
    let (sid, kid) = summary_and_skill(&r).await;
    call(&r, "POST", &format!("/api/summaries/{sid}/skills"), Some(json!({ "skill_id": kid }))).await;
    for _ in 0..2 {
        let (status, body) = call(&r, "DELETE", &format!("/api/summaries/{sid}/skills/{kid}"), None).await;
        assert_eq!(status, StatusCode::NO_CONTENT);
        assert_eq!(body, Value::Null);
    }
    let (_, list) = call(&r, "GET", &format!("/api/summaries/{sid}/skills"), None).await;
    assert_eq!(list["data"], json!([]));
}
