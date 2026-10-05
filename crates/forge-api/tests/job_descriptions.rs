//! `POST /job-descriptions/lookup-by-url` and `POST /job-descriptions/:id/extract-skills`.

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

fn router_with(seed: impl FnOnce(&Connection)) -> Router {
    let forge = Forge::open_memory().unwrap();
    seed(forge.conn());
    app(AppState::new(forge))
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

#[tokio::test]
async fn lookup_by_url_finds_the_jd_with_its_org_name() {
    let r = router();
    let (_, org) = call(
        &r,
        "POST",
        "/api/organizations",
        Some(json!({ "name": "Anthropic" })),
    )
    .await;
    let (_, jd) = call(
        &r,
        "POST",
        "/api/job-descriptions",
        Some(json!({
            "title": "SWE", "raw_text": "Build things.", "url": "https://example.com/jobs/swe",
            "organization_id": org["data"]["id"],
        })),
    )
    .await;
    let (status, body) = call(
        &r,
        "POST",
        "/api/job-descriptions/lookup-by-url",
        Some(json!({ "url": "https://example.com/jobs/swe" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["data"]["id"], jd["data"]["id"]);
    assert_eq!(body["data"]["organization_name"], "Anthropic");
}

#[tokio::test]
async fn lookup_by_url_unknown_is_404_not_405() {
    let r = router();
    let (status, body) = call(
        &r,
        "POST",
        "/api/job-descriptions/lookup-by-url",
        Some(json!({ "url": "https://example.com/none" })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    assert_eq!(body["error"]["code"], "NOT_FOUND");
}

#[tokio::test]
async fn lookup_by_url_rejects_a_missing_blank_or_non_string_url() {
    let r = router();
    for bad in [
        json!({}),
        json!({ "url": "" }),
        json!({ "url": "  " }),
        json!({ "url": 42 }),
    ] {
        let (status, body) = call(
            &r,
            "POST",
            "/api/job-descriptions/lookup-by-url",
            Some(bad.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{bad}: {body}");
        assert_eq!(body["error"]["code"], "VALIDATION_ERROR", "{bad}");
    }
}

#[tokio::test]
async fn extract_skills_payload_matches_the_ts_contract() {
    let r = router();
    let mut names = Vec::new();
    for (name, category) in [
        ("Zzlang", "language"),
        ("Zzframe", "framework"),
        ("Zzsoft", "soft_skill"),
    ] {
        let (status, created) = call(
            &r,
            "POST",
            "/api/skills",
            Some(json!({ "name": name, "category": category })),
        )
        .await;
        assert!(status.is_success(), "{created}");
        // POST /skills may normalise the name; compare against what it returned.
        names.push(created["data"]["name"].clone());
    }
    let raw = "We write Rust and ship with Docker.";
    let (_, jd) = call(
        &r,
        "POST",
        "/api/job-descriptions",
        Some(json!({ "title": "SRE", "raw_text": raw })),
    )
    .await;
    let id = jd["data"]["id"].as_str().unwrap();

    let (status, body) = call(
        &r,
        "POST",
        &format!("/api/job-descriptions/{id}/extract-skills"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let data = &body["data"];
    assert_eq!(data["jd_raw_text"], raw);
    let p = forge_ai::prompts::jd_skill_extraction::render(raw);
    assert_eq!(
        data["prompt_template"],
        format!("{}\n\n{}", p.system, p.user)
    );
    assert!(data["instructions"]
        .as_str()
        .unwrap()
        .starts_with("Execute the prompt_template"));
    let got: Vec<_> = data["existing_skills"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].clone())
        .collect();
    assert!(got.contains(&names[0]) && got.contains(&names[2]));
    assert!(
        !got.contains(&names[1]),
        "framework isn't matched by this text"
    );
    // Each row is exactly {id, name, category}.
    for s in data["existing_skills"].as_array().unwrap() {
        let mut keys: Vec<_> = s.as_object().unwrap().keys().cloned().collect();
        keys.sort();
        assert_eq!(keys, ["category", "id", "name"]);
    }
}

#[tokio::test]
async fn extract_skills_blank_text_is_400() {
    let id = forge_core::new_id();
    let seeded = id.clone();
    let r = router_with(move |c| {
        c.execute(
            "INSERT INTO job_descriptions (id, title, raw_text) VALUES (?1, 'Blank', '   ')",
            params![seeded],
        )
        .unwrap();
    });
    let (status, body) = call(
        &r,
        "POST",
        &format!("/api/job-descriptions/{id}/extract-skills"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["error"]["code"], "VALIDATION_ERROR");
}

#[tokio::test]
async fn extract_skills_unknown_jd_is_404() {
    let r = router();
    let (status, body) = call(
        &r,
        "POST",
        "/api/job-descriptions/does-not-exist/extract-skills",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    assert_eq!(body["error"]["code"], "NOT_FOUND");
}

#[tokio::test]
async fn extract_skills_writes_nothing() {
    let r = router();
    let (_, jd) = call(
        &r,
        "POST",
        "/api/job-descriptions",
        Some(json!({ "title": "SRE", "raw_text": "Rust and Docker on AWS." })),
    )
    .await;
    let id = jd["data"]["id"].as_str().unwrap();

    let (_, skills_before) = call(&r, "GET", "/api/skills", None).await;
    let (_, jd_before) = call(&r, "GET", &format!("/api/job-descriptions/{id}"), None).await;
    let (status, _) = call(
        &r,
        "POST",
        &format!("/api/job-descriptions/{id}/extract-skills"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (_, skills_after) = call(&r, "GET", "/api/skills", None).await;
    let (_, jd_after) = call(&r, "GET", &format!("/api/job-descriptions/{id}"), None).await;
    assert_eq!(skills_before, skills_after);
    assert_eq!(jd_before, jd_after);
}
