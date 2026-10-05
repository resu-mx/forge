//! Typst source and PDF output (roadmap M5). The Typst-source tests run in every build;
//! the PDF tests need the `pdf` feature (the native server's build).

use axum::body::Body;
use axum::http::{HeaderMap, Request, StatusCode};
use axum::Router;
use forge_api::{app, AppState};
use forge_sdk::Forge;
use serde_json::{json, Value};
use tower::ServiceExt;

fn router() -> Router {
    app(AppState::new(Forge::open_memory().unwrap()))
}

async fn send(router: &Router, req: Request<Body>) -> (StatusCode, HeaderMap, Vec<u8>) {
    let resp = router.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let headers = resp.headers().clone();
    let bytes = axum::body::to_bytes(resp.into_body(), 16 << 20)
        .await
        .unwrap()
        .to_vec();
    (status, headers, bytes)
}

async fn json_call(
    r: &Router,
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
    let (status, _, bytes) = send(r, req.body(body).unwrap()).await;
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// A resume from the first template, with one experience entry carrying hostile text.
async fn resume_with_content(r: &Router) -> String {
    let (_, templates) = json_call(r, "GET", "/api/templates", None).await;
    let template = templates["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "Standard Tech Resume")
        .unwrap();
    let (_, resume) = json_call(
        r,
        "POST",
        "/api/resumes",
        Some(json!({
            "name": "Ada Résumé", "target_role": "Engineer", "target_employer": "Acme",
            "archetype": "agentic-ai", "template_id": template["id"]
        })),
    )
    .await;
    let id = resume["data"]["id"].as_str().unwrap().to_string();

    let (_, full) = json_call(r, "GET", &format!("/api/resumes/{id}"), None).await;
    let section = full["data"]["sections"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["entry_type"] == "experience")
        .unwrap();
    // Build the chain source -> bullet -> perspective -> entry by hand.
    let (_, src) = json_call(
        r,
        "POST",
        "/api/sources",
        Some(json!({"title": "Eng", "description": "d", "source_type": "role"})),
    )
    .await;
    let (_, bullet) = json_call(
        r,
        "POST",
        "/api/bullets",
        Some(json!({"content": "b", "source_content_snapshot": "d", "source_ids": [{"source_id": src["data"]["id"], "is_primary": true}], "technologies": []})),
    )
    .await;
    let bullet_id = bullet["data"]["id"].as_str().unwrap();
    for step in ["submit", "approve"] {
        json_call(
            r,
            "PATCH",
            &format!("/api/bullets/{bullet_id}/{step}"),
            None,
        )
        .await;
    }
    let (_, persp) = json_call(
        r,
        "POST",
        "/api/perspectives",
        Some(json!({"bullet_id": bullet_id, "content": "Shipped #read(\"/etc/passwd\") *fast*", "target_archetype": "agentic-ai", "domain": "ai_ml"})),
    )
    .await;
    let (status, _) = json_call(
        r,
        "POST",
        &format!("/api/resumes/{id}/entries"),
        Some(json!({"perspective_id": persp["data"]["id"], "section_id": section["id"]})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    id
}

fn get(path: &str) -> Request<Body> {
    Request::builder().uri(path).body(Body::empty()).unwrap()
}

#[tokio::test]
async fn typst_export_is_source_with_the_right_headers() {
    let r = router();
    let id = resume_with_content(&r).await;
    let (status, headers, body) =
        send(&r, get(&format!("/api/export/resume/{id}?format=typst"))).await;
    assert_eq!(status, StatusCode::OK);
    assert!(headers["content-type"]
        .to_str()
        .unwrap()
        .starts_with("text/x-typst"));
    let disposition = headers["content-disposition"].to_str().unwrap();
    assert!(
        disposition.starts_with("attachment; filename=\"ada-r-sum-")
            && disposition.ends_with(".typ\""),
        "{disposition}"
    );

    let source = String::from_utf8(body).unwrap();
    assert!(source.contains("#set page("), "has the preamble");
    // The hostile perspective text is present only as an escaped literal.
    assert!(
        source.contains(r#""Shipped #read(\"/etc/passwd\") *fast*""#),
        "{source}"
    );
    assert!(!source
        .replace(r#""Shipped #read(\"/etc/passwd\") *fast*""#, "")
        .contains("/etc/passwd"));
}

#[tokio::test]
async fn a_latex_override_is_flagged_not_compiled() {
    let r = router();
    let id = resume_with_content(&r).await;
    let (status, _) = json_call(
        &r,
        "PATCH",
        &format!("/api/resumes/{id}/latex-override"),
        Some(json!({"content": "\\documentclass{article}"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (_, headers, body) = send(&r, get(&format!("/api/export/resume/{id}?format=typst"))).await;
    assert!(
        headers["x-forge-pdf-notice"]
            .to_str()
            .unwrap()
            .contains("latex_override"),
        "notice header present"
    );
    assert!(
        !String::from_utf8_lossy(&body).contains("documentclass"),
        "the LaTeX is not used"
    );
}

#[tokio::test]
async fn no_notice_without_an_override() {
    let r = router();
    let id = resume_with_content(&r).await;
    let (_, headers, _) = send(&r, get(&format!("/api/export/resume/{id}?format=typst"))).await;
    assert!(headers.get("x-forge-pdf-notice").is_none());
}

#[tokio::test]
async fn unknown_resume_is_404_for_typst_and_pdf() {
    let r = router();
    for format in ["typst", "pdf"] {
        let (status, _, _) =
            send(&r, get(&format!("/api/export/resume/nope?format={format}"))).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "format={format}");
    }
}

#[cfg(feature = "pdf")]
mod with_pdf {
    use super::*;

    #[tokio::test]
    async fn export_pdf_returns_a_pdf_attachment() {
        let r = router();
        let id = resume_with_content(&r).await;
        let (status, headers, body) =
            send(&r, get(&format!("/api/export/resume/{id}?format=pdf"))).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(headers["content-type"], "application/pdf");
        assert!(headers["content-disposition"]
            .to_str()
            .unwrap()
            .ends_with(".pdf\""));
        assert!(body.starts_with(b"%PDF-"));
    }

    #[tokio::test]
    async fn post_pdf_returns_an_inline_pdf() {
        let r = router();
        let id = resume_with_content(&r).await;
        let req = Request::builder()
            .method("POST")
            .uri(format!("/api/resumes/{id}/pdf"))
            .body(Body::empty())
            .unwrap();
        let (status, headers, body) = send(&r, req).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(headers["content-type"], "application/pdf");
        assert_eq!(
            headers["content-disposition"],
            "inline; filename=\"resume.pdf\""
        );
        assert_eq!(headers["x-forge-pdf-cache"], "miss");
        assert!(body.starts_with(b"%PDF-"));
    }

    #[tokio::test]
    async fn post_pdf_carries_the_latex_override_notice() {
        let r = router();
        let id = resume_with_content(&r).await;
        json_call(
            &r,
            "PATCH",
            &format!("/api/resumes/{id}/latex-override"),
            Some(json!({"content": "x"})),
        )
        .await;
        let req = Request::builder()
            .method("POST")
            .uri(format!("/api/resumes/{id}/pdf"))
            .body(Body::empty())
            .unwrap();
        let (status, headers, _) = send(&r, req).await;
        assert_eq!(status, StatusCode::OK);
        assert!(headers["x-forge-pdf-notice"]
            .to_str()
            .unwrap()
            .contains("latex_override"));
    }

    #[tokio::test]
    async fn post_pdf_compiles_supplied_typst_and_ignores_the_old_latex_field() {
        let r = router();
        let id = resume_with_content(&r).await;
        let body =
            json!({"typst": "= Hello\nSupplied source", "latex": "\\documentclass{article}"});
        let (status, _, bytes) = send(
            &r,
            Request::builder()
                .method("POST")
                .uri(format!("/api/resumes/{id}/pdf"))
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert!(bytes.starts_with(b"%PDF-"));
    }

    #[tokio::test]
    async fn invalid_typst_is_a_422_with_details() {
        let r = router();
        let id = resume_with_content(&r).await;
        let (status, _, bytes) = send(
            &r,
            Request::builder()
                .method("POST")
                .uri(format!("/api/resumes/{id}/pdf"))
                .header("content-type", "application/json")
                .body(Body::from(json!({"typst": "#let x = "}).to_string()))
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        let v: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["error"]["code"], "TYPST_COMPILE_ERROR");
        assert!(v["error"]["details"]
            .as_array()
            .is_some_and(|d| !d.is_empty()));
    }

    #[tokio::test]
    async fn supplied_typst_cannot_read_files() {
        let r = router();
        let id = resume_with_content(&r).await;
        let (status, _, _) = send(
            &r,
            Request::builder()
                .method("POST")
                .uri(format!("/api/resumes/{id}/pdf"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"typst": "#read(\"/etc/passwd\")"}).to_string(),
                ))
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    }
}

#[cfg(not(feature = "pdf"))]
#[tokio::test]
async fn without_the_pdf_feature_pdf_is_501_and_points_at_typst() {
    let r = router();
    let id = resume_with_content(&r).await;
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/resumes/{id}/pdf"))
        .body(Body::empty())
        .unwrap();
    let (status, _, bytes) = send(&r, req).await;
    assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
    assert!(String::from_utf8_lossy(&bytes).contains("format=typst"));
}
