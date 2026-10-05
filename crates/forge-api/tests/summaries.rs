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

const S: &str = "11111111-1111-1111-1111-111111111111";

async fn summary_and_skill(r: &Router) -> (String, String) {
    let (_, s) = call(
        r,
        "POST",
        "/api/summaries",
        Some(json!({ "title": "Infra" })),
    )
    .await;
    let (_, k) = call(
        r,
        "POST",
        "/api/skills",
        Some(json!({ "name": "Zzkube", "category": "tool" })),
    )
    .await;
    (
        s["data"]["id"].as_str().unwrap().to_string(),
        k["data"]["id"].as_str().unwrap().to_string(),
    )
}

#[tokio::test]
async fn linked_resumes_lists_newest_first_with_pagination() {
    let r = router_with(|c| {
        c.execute(
            "INSERT INTO summaries (id, title) VALUES (?1, 'Shared')",
            params![S],
        )
        .unwrap();
        for (id, name, at) in [
            (
                "aaaaaaaa-0000-0000-0000-000000000001",
                "Older",
                "2026-01-01T00:00:00Z",
            ),
            (
                "aaaaaaaa-0000-0000-0000-000000000002",
                "Newer",
                "2026-02-01T00:00:00Z",
            ),
        ] {
            c.execute(
                "INSERT INTO resumes (id, name, target_role, target_employer, archetype, summary_id, updated_at)
                 VALUES (?1, ?2, 'SRE', 'Acme', 'backend', ?3, ?4)",
                params![id, name, S, at],
            )
            .unwrap();
        }
    });
    let (status, body) = call(
        &r,
        "GET",
        &format!("/api/summaries/{S}/linked-resumes?limit=1"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["data"][0]["name"], "Newer");
    assert_eq!(body["data"].as_array().unwrap().len(), 1);
    assert_eq!(
        body["pagination"],
        json!({ "total": 2, "offset": 0, "limit": 1 })
    );
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
        let (status, body) = call(
            &r,
            "POST",
            &format!("/api/summaries/{sid}/skills"),
            Some(json!({ "skill_id": kid })),
        )
        .await;
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
    for (id, body) in [
        (sid.as_str(), json!({})),
        (sid.as_str(), json!({ "skill_id": "" })),
        ("nope", json!({})),
    ] {
        let (status, resp) = call(
            &r,
            "POST",
            &format!("/api/summaries/{id}/skills"),
            Some(body),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{id}");
        assert_eq!(resp["error"]["code"], "VALIDATION_ERROR");
    }
}

#[tokio::test]
async fn add_skill_to_unknown_summary_or_unknown_skill_is_404() {
    let r = router();
    let (sid, kid) = summary_and_skill(&r).await;
    let (status, body) = call(
        &r,
        "POST",
        "/api/summaries/nope/skills",
        Some(json!({ "skill_id": kid })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"]["code"], "NOT_FOUND");
    let (status, body) = call(
        &r,
        "POST",
        &format!("/api/summaries/{sid}/skills"),
        Some(json!({ "skill_id": "nope" })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"]["code"], "NOT_FOUND");
}

#[tokio::test]
async fn remove_skill_is_idempotent() {
    let r = router();
    let (sid, kid) = summary_and_skill(&r).await;
    call(
        &r,
        "POST",
        &format!("/api/summaries/{sid}/skills"),
        Some(json!({ "skill_id": kid })),
    )
    .await;
    for _ in 0..2 {
        let (status, body) = call(
            &r,
            "DELETE",
            &format!("/api/summaries/{sid}/skills/{kid}"),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::NO_CONTENT);
        assert_eq!(body, Value::Null);
    }
    let (_, list) = call(&r, "GET", &format!("/api/summaries/{sid}/skills"), None).await;
    assert_eq!(list["data"], json!([]));
}

// ── GET /summaries/:id?include=relations (resu-mx/forge#35) ───────────

use forge_core::{CreateIndustryInput, CreateRoleTypeInput, CreateSummary, SkillCategory};
use forge_sdk::db::{IndustryStore, RoleTypeStore, SkillStore, SummaryStore};

struct Seeded {
    router: Router,
    /// Industry "Aero", role type "Tech Lead", keywords RustLang + Kubernetes.
    full: String,
    /// No industry, role type or keywords.
    bare: String,
}

fn new_summary(
    title: &str,
    industry_id: Option<String>,
    role_type_id: Option<String>,
) -> CreateSummary {
    CreateSummary {
        title: title.into(),
        role: None,
        description: None,
        is_template: None,
        industry_id,
        role_type_id,
        notes: None,
    }
}

fn seeded() -> Seeded {
    let forge = Forge::open_memory().unwrap();
    let (full, bare) = {
        let conn = forge.conn();
        let industry = IndustryStore::create(
            conn,
            &CreateIndustryInput {
                name: "Aero".into(),
                description: None,
            },
        )
        .unwrap();
        let role_type = RoleTypeStore::create(
            conn,
            &CreateRoleTypeInput {
                name: "Tech Lead".into(),
                description: None,
            },
        )
        .unwrap();
        let rust = SkillStore::create(conn, "RustLang", Some(SkillCategory::Language)).unwrap();
        let k8s = SkillStore::create(conn, "Kubernetes", Some(SkillCategory::Tool)).unwrap();
        let full = SummaryStore::create(
            conn,
            &new_summary("Hydrated", Some(industry.id), Some(role_type.id)),
        )
        .unwrap();
        SummaryStore::add_skill(conn, &full.id, &rust.id).unwrap();
        SummaryStore::add_skill(conn, &full.id, &k8s.id).unwrap();
        let bare = SummaryStore::create(conn, &new_summary("Bare", None, None)).unwrap();
        (full.id, bare.id)
    };
    Seeded {
        router: app(AppState::new(forge)),
        full,
        bare,
    }
}

#[tokio::test]
async fn include_relations_hydrates_industry_role_type_and_skills() {
    let s = seeded();
    let (status, body) = call(
        &s.router,
        "GET",
        &format!("/api/summaries/{}?include=relations", s.full),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let d = &body["data"];
    assert_eq!(d["title"], "Hydrated"); // flattened base
    assert_eq!(d["linked_resume_count"], 0);
    assert_eq!(d["industry"]["name"], "Aero");
    assert_eq!(d["role_type"]["name"], "Tech Lead");
    let names: Vec<&str> = d["skills"]
        .as_array()
        .unwrap()
        .iter()
        .map(|k| k["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Kubernetes", "RustLang"]); // ordered by name
    let keys: Vec<&String> = d["skills"][0].as_object().unwrap().keys().collect();
    assert_eq!(keys.len(), 3); // id, name, category
}

#[tokio::test]
async fn include_relations_missing_relations_are_null() {
    let s = seeded();
    let (status, body) = call(
        &s.router,
        "GET",
        &format!("/api/summaries/{}?include=relations", s.bare),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["data"]["industry"].is_null() && body["data"].get("industry").is_some());
    assert!(body["data"]["role_type"].is_null() && body["data"].get("role_type").is_some());
    assert_eq!(body["data"]["skills"], json!([]));
}

#[tokio::test]
async fn plain_get_and_other_include_values_do_not_hydrate() {
    let s = seeded();
    for q in ["", "?include=", "?include=foo"] {
        let (status, body) = call(
            &s.router,
            "GET",
            &format!("/api/summaries/{}{q}", s.full),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{q}");
        for key in ["industry", "role_type", "skills"] {
            assert!(body["data"].get(key).is_none(), "{q}: unexpected {key}");
        }
    }
}

#[tokio::test]
async fn unknown_id_is_404_with_and_without_include() {
    let s = seeded();
    for p in [
        "/api/summaries/nope",
        "/api/summaries/nope?include=relations",
    ] {
        let (status, body) = call(&s.router, "GET", p, None).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{p}");
        assert_eq!(body["error"]["code"], "NOT_FOUND");
    }
}

#[tokio::test]
async fn create_with_blank_title_is_400() {
    let r = router();
    let (status, body) = call(
        &r,
        "POST",
        "/api/summaries",
        Some(json!({ "title": "   " })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["error"]["code"], "VALIDATION_ERROR");
}

#[tokio::test]
async fn update_with_blank_title_is_400() {
    let r = router();
    let (_, created) = call(&r, "POST", "/api/summaries", Some(json!({ "title": "A" }))).await;
    let id = created["data"]["id"].as_str().unwrap();
    let (status, body) = call(
        &r,
        "PATCH",
        &format!("/api/summaries/{id}"),
        Some(json!({ "title": "  " })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["error"]["code"], "VALIDATION_ERROR");
}

#[tokio::test]
async fn list_ignores_a_non_numeric_is_template() {
    let r = router();
    call(
        &r,
        "POST",
        "/api/summaries",
        Some(json!({ "title": "A", "is_template": 1 })),
    )
    .await;
    call(&r, "POST", "/api/summaries", Some(json!({ "title": "B" }))).await;
    let (status, body) = call(&r, "GET", "/api/summaries?is_template=invalid", None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["data"].as_array().unwrap().len(), 2);
    let (_, body) = call(&r, "GET", "/api/summaries?is_template=1", None).await;
    assert_eq!(body["data"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn list_rejects_unknown_sort_by_and_direction() {
    let r = router();
    for q in ["sort_by=bogus", "direction=sideways"] {
        let (status, body) = call(&r, "GET", &format!("/api/summaries?{q}"), None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{q}");
        assert_eq!(body["error"]["code"], "VALIDATION_ERROR");
    }
    let (status, _) = call(&r, "GET", "/api/summaries?sort_by=&direction=", None).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = call(
        &r,
        "GET",
        "/api/summaries?sort_by=title&direction=asc",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn clone_of_unknown_summary_is_summary_not_found() {
    let r = router();
    let (status, body) = call(&r, "POST", "/api/summaries/nope/clone", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"]["code"], "SUMMARY_NOT_FOUND");
}
