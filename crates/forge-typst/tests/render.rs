//! The generated Typst must compile and draw the resume, not merely look right as a string.

use forge_core::*;
use forge_sdk::services::render_typst;
use forge_typst::{compile_document, compile_pdf, document_text};

fn bullet(content: &str) -> ExperienceBullet {
    ExperienceBullet { content: content.into(), entry_id: None, source_chain: None, is_cloned: false }
}

/// One of everything, with hostile text where users can type freely.
fn full_document(hostile: &str) -> ResumeDocument {
    let section = |title: &str, kind: IRSectionType, items: Vec<IRSectionItem>| IRSection {
        id: "s".into(),
        section_type: kind,
        title: title.into(),
        display_order: 0,
        items,
    };
    ResumeDocument {
        resume_id: "r".into(),
        header: ResumeHeader {
            name: "Ada Lovelace".into(),
            tagline: Some("Analytical engineer".into()),
            location: Some("London, UK".into()),
            email: Some("ada@example.com".into()),
            phone: Some("+1 (555) 010-0100".into()),
            linkedin: Some("https://linkedin.com/in/ada".into()),
            github: Some("https://github.com/ada".into()),
            website: None,
            clearance: Some("TS/SCI".into()),
        },
        summary: Some(ResumeSummary { summary_id: None, title: None, content: "Builds engines.".into(), is_override: false }),
        sections: vec![
            section(
                "Experience",
                IRSectionType::Experience,
                vec![IRSectionItem::ExperienceGroup(ExperienceGroup {
                    id: "g".into(),
                    organization: "Acme Corp".into(),
                    subheadings: vec![
                        ExperienceSubheading {
                            id: "a".into(),
                            title: "Staff Engineer".into(),
                            location: Some("Remote".into()),
                            date_range: "2022 - present".into(),
                            source_id: None,
                            bullets: vec![bullet("Migrated 40 services to Kubernetes"), bullet(hostile)],
                        },
                        ExperienceSubheading {
                            id: "b".into(),
                            title: "Engineer".into(),
                            location: None,
                            date_range: "2019 - 2022".into(),
                            source_id: None,
                            bullets: vec![bullet("Shipped the analytical engine")],
                        },
                    ],
                })],
            ),
            section(
                "Technical Skills",
                IRSectionType::Skills,
                vec![IRSectionItem::SkillGroup(SkillGroup {
                    categories: vec![SkillCategoryGroup { label: "Languages".into(), skills: vec!["Rust".into(), "Go".into()] }],
                })],
            ),
            section(
                "Education",
                IRSectionType::Education,
                vec![IRSectionItem::Education(EducationItem {
                    institution: "University of London".into(),
                    degree: "Mathematics".into(),
                    date: "1835".into(),
                    entry_id: None,
                    source_id: None,
                    education_type: None,
                    degree_level: None,
                    degree_type: Some("B.Sc.".into()),
                    field: Some("Mathematics".into()),
                    gpa: None,
                    location: None,
                    credential_id: None,
                    issuing_body: None,
                    certificate_subtype: None,
                    edu_description: None,
                    campus_name: None,
                    campus_city: Some("London".into()),
                    campus_state: None,
                })],
            ),
            section(
                "Projects",
                IRSectionType::Projects,
                vec![IRSectionItem::Project(ProjectItem {
                    name: "Forge".into(),
                    description: None,
                    date: Some("2026".into()),
                    entry_id: None,
                    source_id: None,
                    bullets: vec![bullet("Resume builder")],
                })],
            ),
            section(
                "Certifications",
                IRSectionType::Certifications,
                vec![IRSectionItem::CertificationGroup(CertificationGroup {
                    categories: vec![CertificationCategoryGroup {
                        label: "Cloud".into(),
                        certs: vec![CertificationEntry { name: "CKA".into(), entry_id: None, source_id: None }],
                    }],
                })],
            ),
            section(
                "Presentations",
                IRSectionType::Presentations,
                vec![IRSectionItem::Presentation(PresentationItem {
                    title: "Rust in the browser".into(),
                    venue: Some("RustConf".into()),
                    date: Some("2026".into()),
                    entry_id: None,
                    source_id: None,
                    bullets: vec![bullet("Typst, in a Worker")],
                    description: None,
                    presentation_type: None,
                    url: None,
                    coauthors: None,
                })],
            ),
        ],
    }
}

fn all_text(source: &str) -> String {
    document_text(&compile_document(source).unwrap()).join(" ")
}

#[test]
fn a_full_resume_compiles_to_a_pdf_that_draws_its_content() {
    let source = render_typst(&full_document("plain bullet"));
    let document = compile_document(&source).unwrap_or_else(|e| panic!("did not compile: {e:?}\n{source}"));
    let text = document_text(&document).join(" ");

    for expected in [
        "Ada", "Lovelace", "Analytical", "TS/SCI", "Builds engines", "Acme", "Migrated 40 services",
        "Shipped the analytical engine", "Languages", "Rust", "University of London", "B.Sc.", "Forge",
        "CKA", "Typst, in a Worker",
    ] {
        assert!(text.contains(expected), "missing {expected:?} in drawn text: {text}");
    }
    assert!(!text.trim().is_empty(), "a PDF that draws nothing");
    // A bare "..." in Typst markup draws its quotation marks; summary and clearance text must not.
    assert!(!text.contains("\u{201c}Builds") && !text.contains("\"Builds"), "summary drawn with quotes: {text}");
    assert!(!text.contains("\u{201c}TS/SCI"), "clearance drawn with quotes: {text}");

    let pdf = compile_pdf(&source).unwrap();
    assert!(pdf.starts_with(b"%PDF-"));
}

#[test]
fn the_bundled_font_is_used_not_a_silent_fallback() {
    // If the family name did not resolve, Typst would fall back to another face (or none).
    // The PDF embeds the font names it used, so look for ours.
    let pdf = compile_pdf(&render_typst(&full_document("x"))).unwrap();
    let haystack = String::from_utf8_lossy(&pdf);
    assert!(haystack.contains("NewCM10"), "the PDF does not embed New Computer Modern");
}

#[test]
fn hostile_content_is_drawn_as_text_and_never_executed() {
    let hostile = r#"#read("/etc/passwd") *bold* $math$ _it_ @ref <tag> \ ] [ "quoted""#;
    let text = all_text(&render_typst(&full_document(hostile)));
    // Typst's text extraction may split on glyph runs, so check distinctive pieces.
    for piece in ["#read(", "/etc/passwd", "*bold*", "$math$", "@ref"] {
        assert!(text.contains(piece), "hostile text {piece:?} was not drawn literally: {text}");
    }
    assert!(!text.contains("root:"), "file contents leaked into the document");
}

#[test]
fn an_empty_resume_still_produces_a_one_page_pdf() {
    let doc = ResumeDocument {
        resume_id: "r".into(),
        header: ResumeHeader {
            name: "Nobody".into(),
            tagline: None,
            location: None,
            email: None,
            phone: None,
            linkedin: None,
            github: None,
            website: None,
            clearance: None,
        },
        summary: None,
        sections: vec![],
    };
    let document = compile_document(&render_typst(&doc)).unwrap();
    assert_eq!(document.pages().len(), 1);
}

#[test]
fn a_long_resume_flows_onto_more_pages() {
    let many: Vec<ExperienceBullet> = (0..120).map(|i| bullet(&format!("Accomplishment number {i} with a reasonably long description"))).collect();
    let mut doc = full_document("x");
    if let IRSectionItem::ExperienceGroup(g) = &mut doc.sections[0].items[0] {
        g.subheadings[0].bullets = many;
    }
    assert!(compile_document(&render_typst(&doc)).unwrap().pages().len() >= 2);
}

/// Dev tool, not a check: writes a full sample resume so the layout can be looked at.
/// `FORGE_SAMPLE_PDF=/tmp/sample.pdf cargo test -p forge-typst -- --ignored write_sample_pdf`
#[test]
#[ignore]
fn write_sample_pdf() {
    let path = std::env::var("FORGE_SAMPLE_PDF").expect("set FORGE_SAMPLE_PDF to an output path");
    let pdf = compile_pdf(&render_typst(&full_document("Cut deploy time 70% across 40 services"))).unwrap();
    std::fs::write(path, pdf).unwrap();
}

#[test]
fn broken_source_reports_diagnostics_instead_of_panicking() {
    let err = compile_pdf("#let x = ").unwrap_err();
    assert!(!err.message.is_empty());
    assert!(!err.details.is_empty());
}

#[test]
fn source_cannot_read_files() {
    // Even hand-written source (the POST /pdf escape hatch) gets no file system.
    assert!(compile_pdf(r#"#read("/etc/passwd")"#).is_err());
    assert!(compile_pdf(r#"#image("/etc/hosts")"#).is_err());
}
