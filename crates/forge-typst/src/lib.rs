//! Compile Typst source to PDF, in-process.
//!
//! No subprocess, no system fonts, no files: the compiler runs inside this process with
//! New Computer Modern bundled, and it is given no way to read a file, so the source it
//! compiles cannot reach the host's file system or the network.
//!
//! Natively this replaces the `tectonic` subprocess the TypeScript server used. For the
//! browser the same crate builds as its own wasm module (`just typst-bundle`), loaded only
//! when a PDF is first asked for, because it is large (about 25 MB raw, 9 MB gzipped, most
//! of it Typst's own embedded data) and the data layer's wasm should not carry it.

use serde::Serialize;
use typst_as_lib::{TypstAsLibError, TypstEngine};
use typst_layout::PagedDocument;

/// New Computer Modern 10, the Computer Modern look of the old LaTeX template.
/// Licences: `fonts/NOTICE`.
const FONTS: [&[u8]; 4] = [
    include_bytes!("../fonts/NewCM10-Regular.otf"),
    include_bytes!("../fonts/NewCM10-Bold.otf"),
    include_bytes!("../fonts/NewCM10-Italic.otf"),
    include_bytes!("../fonts/NewCM10-BoldItalic.otf"),
];

/// The font family the generated source asks for.
pub const FONT_FAMILY: &str = "New Computer Modern";

/// Why a document did not compile.
#[derive(Debug, Clone, Serialize)]
pub struct CompileError {
    /// One line, for a status message.
    pub message: String,
    /// Each diagnostic Typst reported, for the `details` of an error response.
    pub details: Vec<String>,
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for CompileError {}

impl From<TypstAsLibError> for CompileError {
    fn from(error: TypstAsLibError) -> Self {
        let details: Vec<String> = match &error {
            TypstAsLibError::TypstSource(diagnostics) => {
                diagnostics.iter().map(|d| d.message.to_string()).collect()
            }
            other => vec![other.to_string()],
        };
        let message = details.first().cloned().unwrap_or_else(|| "Typst compilation failed".into());
        CompileError { message, details }
    }
}

/// Compile Typst source to a laid-out document.
pub fn compile_document(source: &str) -> Result<PagedDocument, CompileError> {
    let engine = TypstEngine::builder().main_file(source).fonts(FONTS).build();
    Ok(engine.compile::<PagedDocument>().output?)
}

/// Compile Typst source to PDF bytes.
pub fn compile_pdf(source: &str) -> Result<Vec<u8>, CompileError> {
    let document = compile_document(source)?;
    typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default()).map_err(|errors| {
        let details: Vec<String> = errors.iter().map(|d| d.message.to_string()).collect();
        CompileError {
            message: details.first().cloned().unwrap_or_else(|| "PDF export failed".into()),
            details,
        }
    })
}

/// The text a document draws, page by page, for tests and previews. A PDF that "compiles"
/// but draws nothing (a missing font, say) is the failure this exists to catch.
pub fn document_text(document: &PagedDocument) -> Vec<String> {
    use typst_library::layout::{Frame, FrameItem};

    fn walk(frame: &Frame, out: &mut String) {
        for (_, item) in frame.items() {
            match item {
                FrameItem::Text(text) => {
                    out.push_str(text.text.as_str());
                    out.push(' ');
                }
                FrameItem::Group(group) => walk(&group.frame, out),
                _ => {}
            }
        }
    }

    document
        .pages()
        .iter()
        .map(|page| {
            let mut text = String::new();
            walk(&page.frame, &mut text);
            text
        })
        .collect()
}

// ── Browser module ──────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
mod browser {
    use wasm_bindgen::prelude::*;

    /// Compile Typst source to PDF bytes. On failure throws a JSON string:
    /// `{"message": "...", "details": ["..."]}`.
    #[wasm_bindgen(js_name = compilePdf)]
    pub fn compile_pdf(source: &str) -> Result<Vec<u8>, JsValue> {
        console_error_panic_hook::set_once();
        super::compile_pdf(source).map_err(|e| JsValue::from_str(&serde_json::to_string(&e).unwrap_or(e.message)))
    }
}
