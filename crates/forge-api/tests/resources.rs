//! Router tests for resume sub-resources, export, note references and the typed
//! source wire format (roadmap M3). Same setup as `api.rs`: in-process, in-memory
//! database, the inline (no-tokio) `with_conn` path.

use axum::body::Body;
use axum::http::{HeaderMap, Request, StatusCode};
use axum::Router;
use forge_api::{app, AppState};
use forge_core::{CreateSource, SkillCategory};
use forge_sdk::db::{SkillStore, SourceStore};
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
    let (status, _, text) = raw_full(router, req.body(body).unwrap()).await;
    let value = if text.is_empty() {
        Value::Null
    } else {
        serde_json::from_str(&text).unwrap_or(Value::Null)
    };
    (status, value)
}

async fn raw_full(router: &Router, req: Request<Body>) -> (StatusCode, HeaderMap, String) {
    let resp = router.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let headers = resp.headers().clone();
    let bytes = axum::body::to_bytes(resp.into_body(), 1 << 20)
        .await
        .unwrap();
    (
        status,
        headers,
        String::from_utf8_lossy(&bytes).into_owned(),
    )
}

/// A resume created from the first built-in template; returns (resume id, section count).
async fn resume_from_template(r: &Router) -> (String, usize) {
    let (_, list) = call(r, "GET", "/api/templates", None).await;
    let template = &list["data"][0];
    let body = json!({
        "name": "My Résumé!", "target_role": "Engineer", "target_employer": "Acme",
        "archetype": "backend", "template_id": template["id"]
    });
    let (status, created) = call(r, "POST", "/api/resumes", Some(body)).await;
    assert_eq!(status, StatusCode::CREATED);
    (
        created["data"]["id"].as_str().unwrap().to_string(),
        template["sections"].as_array().unwrap().len(),
    )
}

#[tokio::test]
async fn resume_sections_list_and_update() {
    let r = router();
    let (id, count) = resume_from_template(&r).await;

    let (status, sections) = call(&r, "GET", &format!("/api/resumes/{id}/sections"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(sections["data"].as_array().unwrap().len(), count);
    let section_id = sections["data"][0]["id"].as_str().unwrap().to_string();

    let (status, patched) = call(
        &r,
        "PATCH",
        &format!("/api/resumes/{id}/sections/{section_id}"),
        Some(json!({"title": "Renamed"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(patched["data"]["title"], "Renamed");

    let (status, err) = call(&r, "GET", "/api/resumes/nope/sections", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(err["error"]["code"], "NOT_FOUND");
}

#[tokio::test]
async fn resume_ir_and_gaps() {
    let r = router();
    let (id, _) = resume_from_template(&r).await;

    let (status, ir) = call(&r, "GET", &format!("/api/resumes/{id}/ir"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(ir["data"]["resume_id"], id.as_str());

    let (status, gaps) = call(&r, "GET", &format!("/api/resumes/{id}/gaps"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(gaps["data"]["resume_id"], id.as_str());
    assert!(gaps["data"]["coverage_summary"]["perspectives_included"].is_number());

    let (status, _) = call(&r, "GET", "/api/resumes/nope/ir", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn resume_export_formats_and_errors() {
    let r = router();
    let (id, _) = resume_from_template(&r).await;
    let get = |format: &str| {
        let uri = if format.is_empty() {
            format!("/api/export/resume/{id}")
        } else {
            format!("/api/export/resume/{id}?format={format}")
        };
        Request::builder().uri(uri).body(Body::empty()).unwrap()
    };

    let (status, headers, body) = raw_full(&r, get("markdown")).await;
    assert_eq!(status, StatusCode::OK);
    assert!(headers["content-type"]
        .to_str()
        .unwrap()
        .starts_with("text/markdown"));
    let disposition = headers["content-disposition"].to_str().unwrap();
    assert!(
        disposition.starts_with("attachment; filename=\"my-r-sum-"),
        "{disposition}"
    );
    assert!(disposition.ends_with(".md\""), "{disposition}");
    assert!(!body.is_empty());

    let (status, headers, _) = raw_full(&r, get("latex")).await;
    assert_eq!(status, StatusCode::OK);
    assert!(headers["content-type"]
        .to_str()
        .unwrap()
        .starts_with("application/x-latex"));

    let (status, headers, body) = raw_full(&r, get("json")).await;
    assert_eq!(status, StatusCode::OK);
    assert!(headers["content-disposition"]
        .to_str()
        .unwrap()
        .contains(".json"));
    let doc: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(doc["data"]["resume_id"], id.as_str());

    // A saved override wins over generated output.
    let (status, _) = call(
        &r,
        "PATCH",
        &format!("/api/resumes/{id}/markdown-override"),
        Some(json!({"content": "# Hand written"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (_, _, body) = raw_full(&r, get("markdown")).await;
    assert_eq!(body, "# Hand written");

    for bad in ["", "invalid"] {
        let (status, _, body) = raw_full(&r, get(bad)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "format={bad:?}");
        assert!(body.contains("VALIDATION_ERROR"));
    }
    // PDFs compile in-process only when the `pdf` feature is on (the native server); otherwise
    // the build answers 501 and points at ?format=typst.
    let (status, _, _) = raw_full(&r, get("pdf")).await;
    assert_eq!(
        status,
        if cfg!(feature = "pdf") {
            StatusCode::OK
        } else {
            StatusCode::NOT_IMPLEMENTED
        }
    );

    let missing = Request::builder()
        .uri("/api/export/resume/nope?format=json")
        .body(Body::empty())
        .unwrap();
    let (status, _, _) = raw_full(&r, missing).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn entry_reorder_is_a_patch_that_answers_data_null() {
    let r = router();
    let (id, _) = resume_from_template(&r).await;
    let (status, body) = call(
        &r,
        "PATCH",
        &format!("/api/resumes/{id}/entries/reorder"),
        Some(json!({"entries": []})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!({"data": null}));
    // The old method is gone.
    let (status, _) = call(
        &r,
        "POST",
        &format!("/api/resumes/{id}/entries/reorder"),
        Some(json!({"entries": []})),
    )
    .await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn note_references_round_trip() {
    let r = router();
    let (_, note) = call(
        &r,
        "POST",
        "/api/notes",
        Some(json!({"title": "N", "content": "c"})),
    )
    .await;
    let note_id = note["data"]["id"].as_str().unwrap().to_string();
    let (_, src) = call(
        &r,
        "POST",
        "/api/sources",
        Some(json!({"title": "T", "description": "D"})),
    )
    .await;
    let source_id = src["data"]["id"].as_str().unwrap().to_string();

    let body = json!({"entity_type": "source", "entity_id": source_id});
    let (status, created) = call(
        &r,
        "POST",
        &format!("/api/notes/{note_id}/references"),
        Some(body),
    )
    .await;
    assert_eq!(
        (status, created),
        (StatusCode::CREATED, json!({"data": null}))
    );

    let (_, got) = call(&r, "GET", &format!("/api/notes/{note_id}"), None).await;
    let refs = got["data"]["references"].as_array().unwrap();
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0]["entity_type"], "source");

    let (status, by_entity) = call(
        &r,
        "GET",
        &format!("/api/notes/by-entity/source/{source_id}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(by_entity["data"].as_array().unwrap().len(), 1);

    let bad = json!({"entity_type": "nonsense", "entity_id": "x"});
    let (status, err) = call(
        &r,
        "POST",
        &format!("/api/notes/{note_id}/references"),
        Some(bad),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(err["error"]["code"], "VALIDATION_ERROR");

    let (status, _) = call(
        &r,
        "DELETE",
        &format!("/api/notes/{note_id}/references/source/{source_id}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn source_extension_uses_the_typed_wire_key() {
    let r = router();
    let body = json!({
        "title": "PhD", "description": "D", "source_type": "education",
        "education": {"education_type": "degree", "degree_level": "doctoral", "location": "Pasadena"}
    });
    let (status, created) = call(&r, "POST", "/api/sources", Some(body)).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(created["data"]["education"]["degree_level"], "doctoral");
    assert!(
        created["data"].get("extension").is_none(),
        "renamed to the typed key"
    );

    let id = created["data"]["id"].as_str().unwrap();
    let (_, patched) = call(
        &r,
        "PATCH",
        &format!("/api/sources/{id}"),
        Some(json!({"education": {"location": null, "gpa": "3.9"}})),
    )
    .await;
    assert_eq!(
        patched["data"]["education"]["location"],
        Value::Null,
        "null clears the column"
    );
    assert_eq!(patched["data"]["education"]["gpa"], "3.9");
    assert_eq!(
        patched["data"]["education"]["degree_level"], "doctoral",
        "absent fields are untouched"
    );
}

// ── Source skills ───────────────────────────────────────────────────

/// A router over a database with one source linked to `names`, seeded through
/// the store before the app is built.
fn source_with_skills(names: &[&str]) -> (Router, String, Vec<String>) {
    let forge = Forge::open_memory().unwrap();
    let source = SourceStore::create(
        forge.conn(),
        &CreateSource {
            title: "T".into(),
            description: "D".into(),
            ..Default::default()
        },
    )
    .unwrap()
    .base
    .id;
    let ids = names
        .iter()
        .map(|n| {
            let skill = SkillStore::create(forge.conn(), n, Some(SkillCategory::Tool)).unwrap();
            SourceStore::add_skill(forge.conn(), &source, &skill.id).unwrap();
            skill.id
        })
        .collect();
    (app(AppState::new(forge)), source, ids)
}

#[tokio::test]
async fn source_skills_list_returns_full_rows_by_name() {
    let (r, source, _) = source_with_skills(&["Zig", "Ada"]);
    let (status, body) = call(&r, "GET", &format!("/api/sources/{source}/skills"), None).await;
    assert_eq!(status, StatusCode::OK);
    let names: Vec<&str> = body["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Ada", "Zig"]);
    let mut keys: Vec<String> = body["data"][0]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    keys.sort();
    assert_eq!(keys, ["category", "created_at", "id", "name"]);
}

#[tokio::test]
async fn source_skills_list_unknown_source_is_empty() {
    let (status, body) = call(&router(), "GET", "/api/sources/nope/skills", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"], json!([]));
}

#[tokio::test]
async fn source_skills_list_no_links_is_empty() {
    let (r, source, _) = source_with_skills(&[]);
    let (status, body) = call(&r, "GET", &format!("/api/sources/{source}/skills"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"], json!([]));
}

const UNKNOWN: &str = "00000000-0000-4000-8000-000000000000";

#[tokio::test]
async fn source_skills_add_by_id_and_by_name() {
    let (r, source, _) = source_with_skills(&[]);
    let path = format!("/api/sources/{source}/skills");
    let (_, go) = call(
        &r,
        "POST",
        "/api/skills",
        Some(json!({"name": "Go", "category": "language"})),
    )
    .await;
    let go = go["data"]["id"].as_str().unwrap().to_string();

    for _ in 0..2 {
        let (status, body) = call(&r, "POST", &path, Some(json!({"skill_id": go}))).await;
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(body["data"]["id"], go.as_str());
        assert!(body["data"]["created_at"].is_string());
    }
    let (_, list) = call(&r, "GET", &path, None).await;
    assert_eq!(list["data"].as_array().unwrap().len(), 1, "one link");

    let (status, body) = call(&r, "POST", &path, Some(json!({"name": "kubernetes"}))).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["data"]["name"], "Kubernetes");
    assert_eq!(body["data"]["category"], "other");

    let (_, body) = call(&r, "POST", &path, Some(json!({"name": "sAFe"}))).await;
    assert_eq!(body["data"]["name"], "SAFe");

    let (_, body) = call(
        &r,
        "POST",
        &path,
        Some(json!({"name": "GO", "category": "tool"})),
    )
    .await;
    assert_eq!(body["data"]["id"], go.as_str(), "case-insensitive reuse");
    assert_eq!(
        body["data"]["category"], "language",
        "category ignored on reuse"
    );

    let (_, body) = call(
        &r,
        "POST",
        &path,
        Some(json!({"name": "vector search", "category": "ai_ml"})),
    )
    .await;
    assert_eq!(
        body["data"]["category"], "other",
        "only TS's 10 categories are kept"
    );

    let (status, body) = call(
        &r,
        "POST",
        &path,
        Some(json!({"skill_id": go, "name": "Ignored"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["data"]["id"], go.as_str(), "skill_id wins");
    let (_, found) = call(&r, "GET", "/api/skills?search=Ignored", None).await;
    assert_eq!(found["data"], json!([]), "no Ignored skill");
}

#[tokio::test]
async fn source_skills_add_errors_match_ts() {
    let (r, source, ids) = source_with_skills(&["Rust"]);
    let path = format!("/api/sources/{source}/skills");
    for body in [json!({}), json!({"name": "   "}), json!({"skill_id": ""})] {
        let (status, err) = call(&r, "POST", &path, Some(body)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(err["error"]["code"], "VALIDATION_ERROR");
    }

    let (status, _) = call(&r, "POST", &path, Some(json!({"skill_id": UNKNOWN}))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Validation is checked before the source, as in TS.
    let unknown_path = format!("/api/sources/{UNKNOWN}/skills");
    let (status, _) = call(&r, "POST", &unknown_path, Some(json!({}))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, _) = call(&r, "POST", &unknown_path, Some(json!({"skill_id": ids[0]}))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, err) = call(
        &r,
        "POST",
        &unknown_path,
        Some(json!({"name": "Orphan check"})),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(err["error"]["code"], "NOT_FOUND");
    let (_, found) = call(&r, "GET", "/api/skills?search=Orphan", None).await;
    assert_eq!(found["data"], json!([]), "no orphan skill");
}
