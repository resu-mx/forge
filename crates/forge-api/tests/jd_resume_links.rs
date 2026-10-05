//! Router tests for the JD <-> resume link endpoints (resu-mx/forge#26-#29).
//! Same in-process setup as `tests/resources.rs`. Rows are seeded through the stores before
//! `AppState` takes ownership of the database.

use std::collections::BTreeSet;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use forge_api::{app, AppState};
use forge_core::{CreateJobDescription, CreateResume};
use forge_sdk::db::{JdStore, ResumeStore};
use forge_sdk::Forge;
use serde_json::{json, Value};
use tower::ServiceExt;

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

fn resume(f: &Forge, name: &str) -> String {
    ResumeStore::create(
        f.conn(),
        &CreateResume {
            name: name.into(),
            target_role: "Security Engineer".into(),
            target_employer: "Acme".into(),
            archetype: "devsecops".into(),
            summary_id: None,
        },
    )
    .unwrap()
    .id
}

/// `v` is a `CreateJobDescription` as JSON; omitted optional fields are `None`.
fn jd(f: &Forge, v: Value) -> String {
    let input: CreateJobDescription = serde_json::from_value(v).unwrap();
    JdStore::create(f.conn(), &input).unwrap().id
}

fn keys(v: &Value) -> BTreeSet<String> {
    v.as_object().unwrap().keys().cloned().collect()
}

fn set(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|s| s.to_string()).collect()
}

const RESUME_LINK_KEYS: [&str; 8] = [
    "archetype",
    "created_at",
    "resume_created_at",
    "resume_id",
    "resume_name",
    "status",
    "target_employer",
    "target_role",
];

// ── POST /job-descriptions/:id/resumes (forge#27) ────────────────────

#[tokio::test]
async fn link_resume_201_then_200_with_null_tagline() {
    let f = Forge::open_memory().unwrap();
    let j = jd(
        &f,
        json!({"title": "SRE", "raw_text": "Kubernetes Terraform"}),
    );
    let res = resume(&f, "Linked Resume");
    let r = app(AppState::new(f));
    let path = format!("/api/job-descriptions/{j}/resumes");

    let (status, first) = call(&r, "POST", &path, Some(json!({"resume_id": res}))).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(keys(&first), set(&["data", "tagline"]));
    assert!(first["tagline"].is_null());
    assert_eq!(first["data"]["resume_id"], res.as_str());
    assert_eq!(first["data"]["resume_name"], "Linked Resume");
    assert_eq!(keys(&first["data"]), set(&RESUME_LINK_KEYS));

    let (status, again) = call(&r, "POST", &path, Some(json!({"resume_id": res}))).await;
    assert_eq!(status, StatusCode::OK);
    assert!(again["tagline"].is_null());
    // The original row is kept.
    assert_eq!(again["data"]["created_at"], first["data"]["created_at"]);
}

#[tokio::test]
async fn link_resume_validates_body_then_jd_then_resume() {
    let f = Forge::open_memory().unwrap();
    let j = jd(&f, json!({"title": "SRE", "raw_text": ""}));
    let res = resume(&f, "A");
    let r = app(AppState::new(f));
    let path = format!("/api/job-descriptions/{j}/resumes");

    for bad in [
        json!({}),
        json!({"resume_id": 42}),
        json!({"resume_id": ""}),
    ] {
        let (status, body) = call(&r, "POST", &path, Some(bad)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["error"]["code"], "VALIDATION_ERROR");
    }
    // Unknown JD: 404 naming the JD, even though the resume is unknown too.
    let (status, body) = call(
        &r,
        "POST",
        "/api/job-descriptions/nope/resumes",
        Some(json!({"resume_id": "also-nope"})),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("JobDescription"));
    // Unknown resume: 404.
    let (status, body) = call(&r, "POST", &path, Some(json!({"resume_id": "missing"}))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"]["code"], "NOT_FOUND");
    // Failed attempts leave nothing behind and a valid link still works.
    let (status, _) = call(&r, "POST", &path, Some(json!({"resume_id": res}))).await;
    assert_eq!(status, StatusCode::CREATED);
}
