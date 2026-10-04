//! Router tests for the resume tagline routes (resu-mx/forge#68, #69, #70). In-process,
//! in-memory database, the inline (no-tokio) `with_conn` path, as in `resources.rs`.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use forge_api::{app, AppState};
use forge_core::CreateResume;
use forge_sdk::db::ResumeStore;
use forge_sdk::Forge;
use serde_json::{json, Value};
use tower::ServiceExt;

const OLD: &str = "2000-01-01T00:00:00Z";

/// A fresh database with one resume whose tagline columns are set and whose `updated_at` is old.
fn forge_with_resume(generated: Option<&str>, override_: Option<&str>) -> (Forge, String) {
    let forge = Forge::open_memory().unwrap();
    let id = ResumeStore::create(
        forge.conn(),
        &CreateResume {
            name: "Tagline".into(),
            target_role: "Cloud Engineer".into(),
            target_employer: "Acme".into(),
            archetype: "sre".into(),
            summary_id: None,
        },
    )
    .unwrap()
    .id;
    forge
        .conn()
        .execute(
            "UPDATE resumes SET generated_tagline = ?1, tagline_override = ?2, updated_at = ?3 WHERE id = ?4",
            rusqlite::params![generated, override_, OLD, id],
        )
        .unwrap();
    (forge, id)
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

const GENERATED: &str = "Cloud Engineer -- kubernetes + aws + docker";

#[tokio::test]
async fn get_tagline_returns_ts_shape() {
    let (forge, id) = forge_with_resume(Some(GENERATED), None);
    let r = app(AppState::new(forge));
    let (status, body) = call(&r, "GET", &format!("/api/resumes/{id}/tagline"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        json!({"data": {
            "generated_tagline": GENERATED,
            "tagline_override": null,
            "resolved": GENERATED,
            "has_override": false
        }})
    );
}

#[tokio::test]
async fn get_tagline_override_wins() {
    let (forge, id) = forge_with_resume(Some(GENERATED), Some("Platform engineer"));
    let r = app(AppState::new(forge));
    let (status, body) = call(&r, "GET", &format!("/api/resumes/{id}/tagline"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        json!({"data": {
            "generated_tagline": GENERATED,
            "tagline_override": "Platform engineer",
            "resolved": "Platform engineer",
            "has_override": true
        }})
    );
}

#[tokio::test]
async fn get_tagline_whitespace_override_resolves_as_is_but_is_not_an_override() {
    let (forge, id) = forge_with_resume(Some(GENERATED), Some("   "));
    let r = app(AppState::new(forge));
    let (_, body) = call(&r, "GET", &format!("/api/resumes/{id}/tagline"), None).await;
    assert_eq!(body["data"]["resolved"], "   ");
    assert_eq!(body["data"]["has_override"], false);
}

#[tokio::test]
async fn get_tagline_unset_resolves_empty() {
    let (forge, id) = forge_with_resume(None, None);
    let r = app(AppState::new(forge));
    let (status, body) = call(&r, "GET", &format!("/api/resumes/{id}/tagline"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        json!({"data": {
            "generated_tagline": null,
            "tagline_override": null,
            "resolved": "",
            "has_override": false
        }})
    );
}

#[tokio::test]
async fn get_tagline_unknown_resume_is_404_with_ts_message() {
    let r = app(AppState::new(Forge::open_memory().unwrap()));
    let (status, body) = call(&r, "GET", "/api/resumes/missing/tagline", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(
        body,
        json!({"error": {"code": "NOT_FOUND", "message": "Resume not found"}})
    );
}

#[tokio::test]
async fn get_tagline_does_not_bump_updated_at() {
    let (forge, id) = forge_with_resume(Some(GENERATED), None);
    let r = app(AppState::new(forge));
    let (status, _) = call(&r, "GET", &format!("/api/resumes/{id}/tagline"), None).await;
    assert_eq!(status, StatusCode::OK);
    let (_, resume) = call(&r, "GET", &format!("/api/resumes/{id}"), None).await;
    assert_eq!(resume["data"]["updated_at"], OLD);
}
