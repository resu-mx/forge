//! Contact relationship routes against the TS contract (packages/core/src/routes/contacts.ts).

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

async fn raw(r: &Router, method: &str, path: &str, body: Option<Value>) -> (StatusCode, String) {
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
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

async fn call(r: &Router, method: &str, path: &str, body: Option<Value>) -> (StatusCode, Value) {
    let (status, text) = raw(r, method, path, body).await;
    (status, serde_json::from_str(&text).unwrap_or(Value::Null))
}

fn id(v: &Value) -> String {
    v["data"]["id"].as_str().unwrap().to_string()
}

/// A contact, an organization, a JD at that organization, and a resume.
async fn seed(r: &Router) -> (String, String, String, String) {
    let (s, c) = call(
        r,
        "POST",
        "/api/contacts",
        Some(json!({ "name": "Ada Recruiter" })),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED, "{c}");
    let (s, o) = call(
        r,
        "POST",
        "/api/organizations",
        Some(json!({ "name": "Acme Corp" })),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED, "{o}");
    let (s, j) = call(
        r,
        "POST",
        "/api/job-descriptions",
        Some(json!({ "title": "Platform SRE", "raw_text": "Run the platform.", "organization_id": id(&o) })),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED, "{j}");
    let (s, rs) = call(
        r,
        "POST",
        "/api/resumes",
        Some(json!({
            "name": "Platform resume", "target_role": "SRE",
            "target_employer": "Acme", "archetype": "backend",
        })),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED, "{rs}");
    (id(&c), id(&o), id(&j), id(&rs))
}

#[tokio::test]
async fn organization_link_list_unlink_round_trip() {
    let r = router();
    let (c, o, _, _) = seed(&r).await;

    let (status, body) = raw(
        &r,
        "POST",
        &format!("/api/contacts/{c}/organizations"),
        Some(json!({ "organization_id": o, "relationship": "recruiter" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert!(body.is_empty(), "201 must have an empty body, got {body:?}");

    let (_, list) = call(&r, "GET", &format!("/api/contacts/{c}/organizations"), None).await;
    assert_eq!(
        list["data"],
        json!([{ "id": o, "name": "Acme Corp", "relationship": "recruiter" }])
    );

    let (_, rev) = call(&r, "GET", &format!("/api/organizations/{o}/contacts"), None).await;
    assert_eq!(rev["data"][0]["contact_id"], json!(c));
    assert_eq!(rev["data"][0]["relationship"], "recruiter");

    for _ in 0..2 {
        // The second DELETE matches nothing and is still 204.
        let (status, text) = raw(
            &r,
            "DELETE",
            &format!("/api/contacts/{c}/organizations/{o}/recruiter"),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::NO_CONTENT);
        assert!(text.is_empty());
    }
    let (_, list) = call(&r, "GET", &format!("/api/contacts/{c}/organizations"), None).await;
    assert_eq!(list["data"], json!([]));
}

#[tokio::test]
async fn job_description_link_lists_organization_name() {
    let r = router();
    let (c, o, j, _) = seed(&r).await;

    let (status, body) = raw(
        &r,
        "POST",
        &format!("/api/contacts/{c}/job-descriptions"),
        Some(json!({ "job_description_id": j, "relationship": "hiring_manager" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert!(body.is_empty());

    // A JD with no organization lists organization_name null.
    let (_, bare) = call(
        &r,
        "POST",
        "/api/job-descriptions",
        Some(json!({ "title": "Another role", "raw_text": "text" })),
    )
    .await;
    let (status, _) = raw(
        &r,
        "POST",
        &format!("/api/contacts/{c}/job-descriptions"),
        Some(json!({ "job_description_id": id(&bare), "relationship": "other" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (_, list) = call(
        &r,
        "GET",
        &format!("/api/contacts/{c}/job-descriptions"),
        None,
    )
    .await;
    assert_eq!(
        list["data"],
        json!([
            { "id": id(&bare), "title": "Another role", "organization_name": null, "relationship": "other" },
            { "id": j, "title": "Platform SRE", "organization_name": "Acme Corp", "relationship": "hiring_manager" },
        ])
    );
    let _ = o;

    let (status, _) = raw(
        &r,
        "DELETE",
        &format!("/api/contacts/{c}/job-descriptions/{j}/hiring_manager"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, list) = call(
        &r,
        "GET",
        &format!("/api/contacts/{c}/job-descriptions"),
        None,
    )
    .await;
    assert_eq!(list["data"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn resume_link_round_trip() {
    let r = router();
    let (c, _, _, rs) = seed(&r).await;

    let (status, body) = raw(
        &r,
        "POST",
        &format!("/api/contacts/{c}/resumes"),
        Some(json!({ "resume_id": rs, "relationship": "reference" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert!(body.is_empty());

    let (_, list) = call(&r, "GET", &format!("/api/contacts/{c}/resumes"), None).await;
    assert_eq!(
        list["data"],
        json!([{ "id": rs, "name": "Platform resume", "relationship": "reference" }])
    );

    let (status, _) = raw(
        &r,
        "DELETE",
        &format!("/api/contacts/{c}/resumes/{rs}/reference"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, list) = call(&r, "GET", &format!("/api/contacts/{c}/resumes"), None).await;
    assert_eq!(list["data"], json!([]));
}

#[tokio::test]
async fn link_is_idempotent_and_keeps_distinct_relationships() {
    let r = router();
    let (c, o, _, _) = seed(&r).await;
    let path = format!("/api/contacts/{c}/organizations");

    for _ in 0..2 {
        let (status, _) = raw(
            &r,
            "POST",
            &path,
            Some(json!({ "organization_id": o, "relationship": "recruiter" })),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
    }
    let (_, list) = call(&r, "GET", &path, None).await;
    assert_eq!(list["data"].as_array().unwrap().len(), 1);

    let (status, _) = raw(
        &r,
        "POST",
        &path,
        Some(json!({ "organization_id": o, "relationship": "peer" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let (_, list) = call(&r, "GET", &path, None).await;
    let mut rels: Vec<String> = list["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["relationship"].as_str().unwrap().to_string())
        .collect();
    rels.sort();
    assert_eq!(rels, ["peer", "recruiter"]);
}

#[tokio::test]
async fn invalid_relationship_is_400_in_body_and_path() {
    let r = router();
    let (c, o, j, rs) = seed(&r).await;

    let (status, body) = call(
        &r,
        "POST",
        &format!("/api/contacts/{c}/organizations"),
        Some(json!({ "organization_id": o, "relationship": "boss" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "VALIDATION_ERROR");
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("recruiter, hr, referral, peer, manager, other"));

    // A missing relationship is invalid too.
    let (status, body) = call(
        &r,
        "POST",
        &format!("/api/contacts/{c}/job-descriptions"),
        Some(json!({ "job_description_id": j })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("hiring_manager, recruiter, interviewer, referral, other"));

    let (status, body) = call(
        &r,
        "POST",
        &format!("/api/contacts/{c}/resumes"),
        Some(json!({ "resume_id": rs, "relationship": "hr" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("reference, recommender, other"));

    for p in [
        format!("/api/contacts/{c}/organizations/{o}/boss"),
        format!("/api/contacts/{c}/job-descriptions/{j}/boss"),
        format!("/api/contacts/{c}/resumes/{rs}/boss"),
    ] {
        let (status, _) = raw(&r, "DELETE", &p, None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{p}");
    }
}

#[tokio::test]
async fn missing_target_id_is_400() {
    let r = router();
    let (c, _, _, _) = seed(&r).await;
    for (kind, field, rel) in [
        ("organizations", "organization_id", "recruiter"),
        ("job-descriptions", "job_description_id", "recruiter"),
        ("resumes", "resume_id", "reference"),
    ] {
        let (status, body) = call(
            &r,
            "POST",
            &format!("/api/contacts/{c}/{kind}"),
            Some(json!({ "relationship": rel })),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{kind}: {body}");
        assert_eq!(body["error"]["code"], "VALIDATION_ERROR");

        let (status, _) = call(
            &r,
            "POST",
            &format!("/api/contacts/{c}/{kind}"),
            Some(json!({ field: "  ", "relationship": rel })),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{kind} blank");
    }
}

#[tokio::test]
async fn missing_contact_or_target_is_400_not_500() {
    let r = router();
    let (c, o, _, _) = seed(&r).await;
    let nil = "00000000-0000-0000-0000-000000000000";

    let (status, body) = call(
        &r,
        "POST",
        &format!("/api/contacts/{c}/organizations"),
        Some(json!({ "organization_id": nil, "relationship": "hr" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["error"]["code"], "VALIDATION_ERROR");

    let (status, body) = call(
        &r,
        "POST",
        &format!("/api/contacts/{nil}/organizations"),
        Some(json!({ "organization_id": o, "relationship": "hr" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["error"]["code"], "VALIDATION_ERROR");

    let (status, _) = call(
        &r,
        "POST",
        &format!("/api/contacts/{c}/job-descriptions"),
        Some(json!({ "job_description_id": nil, "relationship": "other" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = call(
        &r,
        "POST",
        &format!("/api/contacts/{c}/resumes"),
        Some(json!({ "resume_id": nil, "relationship": "other" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn lists_of_unknown_parents_are_empty() {
    let r = router();
    let nil = "00000000-0000-0000-0000-000000000000";
    for p in [
        format!("/api/contacts/{nil}/organizations"),
        format!("/api/contacts/{nil}/job-descriptions"),
        format!("/api/contacts/{nil}/resumes"),
        format!("/api/organizations/{nil}/contacts"),
        format!("/api/job-descriptions/{nil}/contacts"),
        format!("/api/resumes/{nil}/contacts"),
    ] {
        let (status, body) = call(&r, "GET", &p, None).await;
        assert_eq!(status, StatusCode::OK, "{p}");
        assert_eq!(body, json!({ "data": [] }), "{p}");
    }
    // Unlinking an unknown row is idempotent.
    let (status, _) = raw(
        &r,
        "DELETE",
        &format!("/api/contacts/{nil}/organizations/{nil}/hr"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn reverse_lookups_list_contact_links() {
    let r = router();
    let (c, o, j, rs) = seed(&r).await;
    let (_, g) = call(
        &r,
        "POST",
        "/api/contacts",
        Some(json!({ "name": "Grace Manager", "title": "VP", "email": "grace@example.com" })),
    )
    .await;
    let g = id(&g);

    for (who, rel) in [(&c, "recruiter"), (&g, "manager")] {
        raw(
            &r,
            "POST",
            &format!("/api/contacts/{who}/organizations"),
            Some(json!({ "organization_id": o, "relationship": rel })),
        )
        .await;
        raw(
            &r,
            "POST",
            &format!("/api/contacts/{who}/job-descriptions"),
            Some(json!({ "job_description_id": j, "relationship": "other" })),
        )
        .await;
        raw(
            &r,
            "POST",
            &format!("/api/contacts/{who}/resumes"),
            Some(json!({ "resume_id": rs, "relationship": "other" })),
        )
        .await;
    }

    let (_, rev) = call(&r, "GET", &format!("/api/organizations/{o}/contacts"), None).await;
    assert_eq!(
        rev["data"],
        json!([
            { "contact_id": c, "contact_name": "Ada Recruiter", "contact_title": null,
              "contact_email": null, "relationship": "recruiter" },
            { "contact_id": g, "contact_name": "Grace Manager", "contact_title": "VP",
              "contact_email": "grace@example.com", "relationship": "manager" },
        ])
    );
    for p in [
        format!("/api/job-descriptions/{j}/contacts"),
        format!("/api/resumes/{rs}/contacts"),
    ] {
        let (status, rev) = call(&r, "GET", &p, None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(rev["data"].as_array().unwrap().len(), 2, "{p}");
        assert_eq!(rev["data"][0]["contact_name"], "Ada Recruiter");
        assert_eq!(rev["data"][0]["relationship"], "other");
    }
}
