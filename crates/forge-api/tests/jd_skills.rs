//! Router tests for job-description skill links (resu-mx/forge#23, #24).
//! Same setup as `api.rs`: in-process, in-memory database, the inline (no-tokio) `with_conn`.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use forge_api::{app, AppState};
use forge_sdk::Forge;
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use tower::ServiceExt;

// Ids must be 36 characters (CHECK constraints on both tables).
const JD: &str = "11111111-1111-4111-8111-111111111111";
const JD_EMPTY: &str = "11111111-1111-4111-8111-222222222222";
const TF: &str = "22222222-2222-4222-8222-222222222222";
const K8S: &str = "33333333-3333-4333-8333-333333333333";

/// A router over a fresh database that `seed` has filled first.
fn router_with(seed: impl FnOnce(&Connection)) -> Router {
    let forge = Forge::open_memory().unwrap();
    seed(forge.conn());
    app(AppState::new(forge))
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

fn names(list: &Value) -> Vec<String> {
    list["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap().to_string())
        .collect()
}

/// JD with Terraform and Kubernetes linked, plus a second JD with no links.
fn seed_jd_with_two_skills(conn: &Connection) {
    conn.execute(
        "INSERT INTO job_descriptions (id, title, raw_text) VALUES (?1, 'SRE', 'text'), (?2, 'Empty', 'text')",
        params![JD, JD_EMPTY],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO skills (id, name, category) VALUES (?1, 'Terraform', 'tool'), (?2, 'Kubernetes', 'platform')",
        params![TF, K8S],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO job_description_skills (job_description_id, skill_id) VALUES (?1, ?2), (?1, ?3)",
        params![JD, TF, K8S],
    )
    .unwrap();
}

#[tokio::test]
async fn get_skills_lists_links_by_name_with_created_at() {
    let r = router_with(seed_jd_with_two_skills);
    let (status, body) = call(
        &r,
        "GET",
        &format!("/api/job-descriptions/{JD}/skills"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(names(&body), ["Kubernetes", "Terraform"]);
    assert_eq!(body["data"][0]["id"], K8S);
    assert_eq!(body["data"][0]["category"], "platform");
    assert!(body["data"][0]["created_at"].is_string());
}

#[tokio::test]
async fn get_skills_is_empty_for_an_unlinked_or_unknown_jd() {
    let r = router_with(seed_jd_with_two_skills);
    for id in [JD_EMPTY, "99999999-9999-4999-8999-999999999999"] {
        let (status, body) = call(
            &r,
            "GET",
            &format!("/api/job-descriptions/{id}/skills"),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{id}");
        assert_eq!(body["data"], json!([]), "{id}");
    }
}
