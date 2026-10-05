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

const PY: &str = "44444444-4444-4444-8444-444444444444";
const UNKNOWN_JD: &str = "99999999-9999-4999-8999-999999999999";

fn seed_jd_and_python(conn: &Connection) {
    conn.execute(
        "INSERT INTO job_descriptions (id, title, raw_text) VALUES (?1, 'SRE', 'text')",
        params![JD],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO skills (id, name, category) VALUES (?1, 'Python', 'language')",
        params![PY],
    )
    .unwrap();
}

fn jd_skill(jd: &str, skill: &str) -> String {
    format!("/api/job-descriptions/{jd}/skills/{skill}")
}

fn jd_skills(jd: &str) -> String {
    format!("/api/job-descriptions/{jd}/skills")
}

#[tokio::test]
async fn post_skill_by_id_links_once_and_answers_201_with_the_row() {
    let r = router_with(seed_jd_and_python);
    for _ in 0..2 {
        let (status, body) =
            call(&r, "POST", &jd_skills(JD), Some(json!({ "skill_id": PY }))).await;
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(body["data"]["name"], "Python");
        assert!(body["data"]["created_at"].is_string());
    }
    let (_, list) = call(&r, "GET", &jd_skills(JD), None).await;
    assert_eq!(names(&list), ["Python"]);
}

#[tokio::test]
async fn post_skill_by_name_capitalises_reuses_and_applies_the_category_rule() {
    let r = router_with(seed_jd_and_python);
    let (status, body) = call(
        &r,
        "POST",
        &jd_skills(JD),
        Some(json!({ "name": "python", "category": "tool" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["data"]["id"], PY);
    assert_eq!(body["data"]["category"], "language");

    for (name, category, want) in [
        ("terraform", "tool", "tool"),
        ("vector stores", "ai_ml", "other"),
        ("cissp", "certification", "other"),
    ] {
        let (status, body) = call(
            &r,
            "POST",
            &jd_skills(JD),
            Some(json!({ "name": name, "category": category })),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{name}");
        assert_eq!(body["data"]["category"], want, "{name}");
    }
    let (_, list) = call(&r, "GET", &jd_skills(JD), None).await;
    assert_eq!(
        names(&list),
        ["Cissp", "Python", "Terraform", "Vector stores"]
    );
}

#[tokio::test]
async fn post_skill_rejects_an_empty_body_and_a_blank_name() {
    let r = router_with(seed_jd_and_python);
    for body in [
        json!({}),
        json!({ "name": "   " }),
        json!({ "skill_id": "" }),
    ] {
        let (status, err) = call(&r, "POST", &jd_skills(JD), Some(body.clone())).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
        assert_eq!(err["error"]["code"], "VALIDATION_ERROR");
        assert!(err["error"]["message"]
            .as_str()
            .unwrap()
            .contains("skill_id or name is required"));
    }
}

#[tokio::test]
async fn post_skill_answers_404_for_unknown_ids_and_creates_nothing() {
    let r = router_with(seed_jd_and_python);
    for body in [
        json!({ "skill_id": PY }),
        json!({ "name": "Zebra Mesh" }),
        json!({ "name": "python" }),
    ] {
        let (status, err) = call(&r, "POST", &jd_skills(UNKNOWN_JD), Some(body.clone())).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
        assert_eq!(err["error"]["code"], "NOT_FOUND");
        assert!(!err["error"]["message"]
            .as_str()
            .unwrap()
            .starts_with("Route not found"));
    }
    let (status, _) = call(
        &r,
        "POST",
        &jd_skills(JD),
        Some(json!({ "skill_id": "nope" })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (_, found) = call(&r, "GET", "/api/skills?search=Zebra", None).await;
    assert_eq!(found["data"], json!([]));
}

#[tokio::test]
async fn post_skill_id_wins_over_name() {
    let r = router_with(seed_jd_and_python);
    let (status, body) = call(
        &r,
        "POST",
        &jd_skills(JD),
        Some(json!({ "skill_id": PY, "name": "Zebra Mesh" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["data"]["name"], "Python");
    let (_, found) = call(&r, "GET", "/api/skills?search=Zebra", None).await;
    assert_eq!(found["data"], json!([]));
}

#[tokio::test]
async fn delete_skill_answers_204_and_keeps_the_skill() {
    let r = router_with(seed_jd_with_two_skills);
    let (status, body) = call(&r, "DELETE", &jd_skill(JD, TF), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(body, Value::Null);

    let (_, list) = call(&r, "GET", &jd_skills(JD), None).await;
    assert_eq!(names(&list), ["Kubernetes"]);
    let (status, _) = call(&r, "GET", &format!("/api/skills/{TF}"), None).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn delete_skill_is_204_for_a_missing_link_or_unknown_ids() {
    let r = router_with(seed_jd_with_two_skills);
    for path in [
        jd_skill(JD, TF),
        jd_skill(JD, TF),
        jd_skill(JD, "nope"),
        jd_skill("nope", K8S),
    ] {
        let (status, _) = call(&r, "DELETE", &path, None).await;
        assert_eq!(status, StatusCode::NO_CONTENT, "{path}");
    }
    let (_, list) = call(&r, "GET", &jd_skills(JD), None).await;
    assert_eq!(names(&list), ["Kubernetes"]);
}

#[tokio::test]
async fn delete_skill_leaves_other_jds_links() {
    let r = router_with(|conn| {
        seed_jd_with_two_skills(conn);
        conn.execute(
            "INSERT INTO job_description_skills (job_description_id, skill_id) VALUES (?1, ?2)",
            params![JD_EMPTY, TF],
        )
        .unwrap();
    });
    let (status, _) = call(&r, "DELETE", &jd_skill(JD, TF), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, list) = call(&r, "GET", &jd_skills(JD_EMPTY), None).await;
    assert_eq!(names(&list), ["Terraform"]);
}
