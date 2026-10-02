//! PDF output, shared by `POST /resumes/:id/pdf` and `GET /export/resume/:id?format=pdf`.
//!
//! A PDF is Typst source compiled by `forge-typst`, in-process. That is behind the `pdf`
//! feature: the native server turns it on; the browser build leaves it off (Typst is a
//! 25 MB module the browser loads separately, on first use) and instead serves the Typst
//! *source* (`?format=typst`) for the browser's own Typst module to compile.

use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use forge_core::ForgeError;
use forge_sdk::db::ResumeStore;
use forge_sdk::services::CompilerService;
use rusqlite::Connection;

/// Sent when a resume has a saved LaTeX override. LaTeX is no longer compiled, so the
/// override cannot affect the PDF; the client should say so.
pub const LATEX_OVERRIDE_NOTICE: &str =
    "latex_override is not compiled: this PDF is generated from the resume content";

/// A resume rendered to Typst, plus anything the caller should be told about it.
pub struct ResumeTypst {
    pub name: String,
    pub source: String,
    pub notice: Option<&'static str>,
}

/// Compile the resume's IR and render it as Typst. 404 for an unknown resume.
pub fn resume_typst(conn: &Connection, id: &str) -> Result<ResumeTypst, ForgeError> {
    let resume = ResumeStore::get(conn, id)?
        .ok_or_else(|| ForgeError::NotFound { entity_type: "resume".into(), id: id.into() })?;
    let doc = CompilerService::compile(conn, id)?
        .ok_or_else(|| ForgeError::NotFound { entity_type: "resume".into(), id: id.into() })?;
    let notice = resume
        .latex_override
        .as_deref()
        .is_some_and(|s| !s.trim().is_empty())
        .then_some(LATEX_OVERRIDE_NOTICE);
    Ok(ResumeTypst { name: resume.name, source: CompilerService::render_typst(&doc), notice })
}

/// Attach the notice header, if any.
pub fn with_notice(mut response: Response, notice: Option<&str>) -> Response {
    if let Some(value) = notice.and_then(|n| HeaderValue::from_str(n).ok()) {
        response.headers_mut().insert("x-forge-pdf-notice", value);
    }
    response
}

/// Compile `source` and answer with the PDF (`disposition` is the Content-Disposition value).
#[cfg(feature = "pdf")]
pub fn pdf_response(source: &str, disposition: &str, notice: Option<&str>) -> Response {
    match forge_typst::compile_pdf(source) {
        Ok(bytes) => with_notice(
            (
                [
                    (header::CONTENT_TYPE, "application/pdf".to_string()),
                    (header::CONTENT_DISPOSITION, disposition.to_string()),
                    // Compiling takes milliseconds, so nothing is cached; kept for clients that read it.
                    (header::HeaderName::from_static("x-forge-pdf-cache"), "miss".to_string()),
                ],
                bytes,
            )
                .into_response(),
            notice,
        ),
        Err(error) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({
                "error": { "code": "TYPST_COMPILE_ERROR", "message": error.message, "details": error.details }
            })),
        )
            .into_response(),
    }
}

/// Without the `pdf` feature this build does not compile PDFs.
#[cfg(not(feature = "pdf"))]
pub fn pdf_response(_source: &str, _disposition: &str, _notice: Option<&str>) -> Response {
    crate::response::not_implemented(
        "PDF compilation is not part of this build. Fetch the Typst source with ?format=typst and compile it with forge-typst",
    )
}
