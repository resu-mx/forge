//! Render the resume IR to Typst source.
//!
//! A port of the `sb2nov` LaTeX layout (`packages/core/src/templates/sb2nov.ts`): the
//! same header, the same section rules, the same experience / education / project /
//! skills / presentation line layouts, in Computer Modern.
//!
//! **User text never becomes Typst markup.** Every field is emitted as an escaped Typst
//! *string literal* passed to a helper function (`#sub("Acme", ...)`), so a bullet reading
//! `#read("/etc/passwd")` or `*bold*` is just those characters. There is nothing to escape
//! for markup, and no way for content to call a Typst function.

use forge_core::{
    ClearanceItem, EducationItem, ExperienceBullet, ExperienceGroup, IRSection, IRSectionItem,
    PresentationItem, ProjectItem, ResumeDocument, ResumeHeader,
};

/// The font family the template asks for (bundled by `forge-typst`).
const FONT: &str = "New Computer Modern";

/// A Typst string literal: `"..."` with `\`, `"` and control characters escaped.
fn lit(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{{{:x}}}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// `lit` for an optional field, `""` when absent.
fn opt(text: &Option<String>) -> String {
    lit(text.as_deref().unwrap_or(""))
}

const PREAMBLE: &str = r#"#set page(paper: "us-letter", margin: (x: 0.5in, y: 0.5in))
#set text(font: "__FONT__", size: 10pt, lang: "en")
#set par(justify: false, leading: 0.5em, spacing: 0.5em)
#show link: it => it

// A section heading: small caps, a rule underneath, kept with the content that follows.
#let sec(title) = {
  v(0.7em, weak: true)
  block(sticky: true, below: 0.45em, stack(spacing: 3pt,
    text(size: 12pt, smallcaps(title)),
    line(length: 100%, stroke: 0.5pt)))
}
// Two columns, the second right-aligned.
// (Not named `right`: that would shadow Typst's `right` alignment.)
#let row(lhs, rhs) = grid(columns: (1fr, auto), lhs, align(right, rhs))
#let small(body) = text(size: 9pt, body)
// resumeSubheading: bold org + right, italic title + date.
#let sub(a, b, c, d) = {
  row(strong(a), b)
  row(emph(small(c)), emph(small(d)))
  v(2pt)
}
// resumeSubSubheading: a second role at the same organisation.
#let subsub(a, b) = {
  v(5pt)
  row(emph(small(a)), emph(small(b)))
  v(2pt)
}
// resumeProjectHeading
#let projhead(lhs, rhs) = {
  row(small(lhs), rhs)
  v(1pt)
}
// resumeItemList: compact bullets in small type.
#let items(..bullets) = {
  list(marker: [•], indent: 0.15in, body-indent: 0.4em, spacing: 0.4em,
    ..bullets.pos().map(b => small(b)))
  v(3pt)
}
// Skills / certifications: "Label: a, b, c" lines.
#let labelled(..rows) = {
  for r in rows.pos() [
    #small[#strong(r.at(0)): #r.at(1)] \
  ]
}
"#;

/// Render a compiled resume as Typst source.
pub fn render_typst(doc: &ResumeDocument) -> String {
    let mut out = String::new();
    out.push_str(&PREAMBLE.replace("__FONT__", FONT));
    out.push_str(&format!(
        "\n#set document(title: {}, author: {})\n\n",
        lit(&doc.header.name),
        lit(&doc.header.name)
    ));

    render_header(&doc.header, &mut out);

    if let Some(summary) = &doc.summary {
        if !summary.content.trim().is_empty() {
            out.push_str(&format!("#sec({})\n", lit(summary.title.as_deref().unwrap_or("Summary"))));
            out.push_str(&format!("#{}\n\n", lit(&summary.content)));
        }
    }

    for section in &doc.sections {
        render_section(section, &mut out);
    }
    out
}

fn render_header(h: &ResumeHeader, out: &mut String) {
    out.push_str("#align(center)[\n");
    out.push_str(&format!("  #text(size: 22pt, weight: \"bold\", smallcaps({}))\n", lit(&h.name)));
    if let Some(tagline) = h.tagline.as_deref().filter(|s| !s.is_empty()) {
        out.push_str(&format!("  #linebreak() #small({})\n", lit(tagline)));
    }
    if let Some(clearance) = h.clearance.as_deref().filter(|s| !s.is_empty()) {
        out.push_str(&format!("  #linebreak() #small(strong({}))\n", lit(clearance)));
    }

    // Contact line, in the same order as the LaTeX template.
    let mut parts: Vec<String> = Vec::new();
    let text_part = |s: &str| format!("small({})", lit(s));
    let link_part = |url: String, label: &str| format!("small(link({}, underline({})))", lit(&url), lit(label));
    if let Some(location) = h.location.as_deref().filter(|s| !s.is_empty()) {
        parts.push(text_part(location));
    }
    if let Some(email) = h.email.as_deref().filter(|s| !s.is_empty()) {
        parts.push(link_part(format!("mailto:{email}"), email));
    }
    if let Some(phone) = h.phone.as_deref().filter(|s| !s.is_empty()) {
        let digits: String = phone.chars().filter(char::is_ascii_digit).collect();
        parts.push(link_part(format!("tel:+{digits}"), phone));
    }
    for (url, label) in [(&h.linkedin, "LinkedIn"), (&h.github, "GitHub"), (&h.website, "Website")] {
        if let Some(url) = url.as_deref().filter(|s| !s.is_empty()) {
            parts.push(link_part(url.to_string(), label));
        }
    }
    if !parts.is_empty() {
        out.push_str(&format!(
            "  #linebreak() #({}).join(small[ #sym.bar.v ])\n",
            parts.iter().map(|p| format!("[#{p}]")).collect::<Vec<_>>().join(", ") + ","
        ));
    }
    out.push_str("]\n\n");
}

fn render_section(section: &IRSection, out: &mut String) {
    out.push_str(&format!("#sec({})\n", lit(&section.title)));

    for item in &section.items {
        match item {
            IRSectionItem::Summary(s) => out.push_str(&format!("#{}\n\n", lit(&s.content))),
            IRSectionItem::ExperienceGroup(group) => render_experience(group, out),
            IRSectionItem::SkillGroup(group) => {
                let rows: Vec<String> = group
                    .categories
                    .iter()
                    .map(|c| format!("({}, {})", lit(&c.label), lit(&c.skills.join(", "))))
                    .collect();
                if !rows.is_empty() {
                    out.push_str(&format!("#labelled({})\n", rows.join(", ")));
                }
            }
            IRSectionItem::Education(edu) => render_education(edu, out),
            IRSectionItem::Project(project) => render_project(project, out),
            IRSectionItem::CertificationGroup(group) => {
                let rows: Vec<String> = group
                    .categories
                    .iter()
                    .map(|c| {
                        let names: Vec<&str> = c.certs.iter().map(|e| e.name.as_str()).collect();
                        format!("({}, {})", lit(&c.label), lit(&names.join(", ")))
                    })
                    .collect();
                if !rows.is_empty() {
                    out.push_str(&format!("#labelled({})\n", rows.join(", ")));
                }
            }
            IRSectionItem::Clearance(ClearanceItem { content, .. }) => {
                out.push_str(&format!("#{}\n\n", lit(content)));
            }
            IRSectionItem::Presentation(p) => render_presentation(p, out),
        }
    }
    out.push('\n');
}

fn bullets(list: &[ExperienceBullet], out: &mut String) {
    if list.is_empty() {
        return;
    }
    let args: Vec<String> = list.iter().map(|b| lit(&b.content)).collect();
    out.push_str(&format!("#items({})\n", args.join(", ")));
}

fn render_experience(group: &ExperienceGroup, out: &mut String) {
    out.push_str("#v(5pt)\n");
    for (i, sub) in group.subheadings.iter().enumerate() {
        let title = match sub.location.as_deref().filter(|l| !l.is_empty()) {
            Some(location) => format!("{} ({location})", sub.title),
            None => sub.title.clone(),
        };
        if i == 0 {
            out.push_str(&format!("#sub({}, \"\", {}, {})\n", lit(&group.organization), lit(&title), lit(&sub.date_range)));
        } else {
            out.push_str(&format!("#subsub({}, {})\n", lit(&title), lit(&sub.date_range)));
        }
        bullets(&sub.bullets, out);
    }
}

/// `City, ST`, preferring the campus over the deprecated free-text location.
fn education_location(edu: &EducationItem) -> String {
    match (&edu.campus_city, &edu.campus_state) {
        (Some(city), Some(state)) => format!("{city}, {state}"),
        (city, state) => city
            .clone()
            .or_else(|| state.clone())
            .or_else(|| edu.location.clone())
            .unwrap_or_default(),
    }
}

fn render_education(edu: &EducationItem, out: &mut String) {
    let kind = edu.education_type.as_deref().unwrap_or("degree");
    let date = edu.date.as_str();
    match kind {
        "self_taught" => {
            let content = edu.edu_description.as_deref().unwrap_or(&edu.degree);
            out.push_str(&format!("#items({})\n", lit(content)));
        }
        "certificate" => {
            let expires = if date.is_empty() { String::new() } else { format!("Exp. {date}") };
            let issuer = edu.issuing_body.as_deref().unwrap_or(&edu.institution);
            let credential = edu.credential_id.as_deref().map(|c| format!(" -- Credential ID: {c}")).unwrap_or_default();
            let line2 = if issuer.is_empty() {
                credential.trim_start_matches(" -- ").to_string()
            } else {
                format!("{issuer}{credential}")
            };
            out.push_str(&format!("#sub({}, {}, {}, \"\")\n", lit(&edu.degree), lit(&expires), lit(&line2)));
        }
        "course" => {
            let location = education_location(edu);
            let line2 = if location.is_empty() { edu.institution.clone() } else { format!("{}, {location}", edu.institution) };
            out.push_str(&format!("#sub({}, {}, {}, \"\")\n", lit(&edu.degree), lit(date), lit(&line2)));
        }
        _ => {
            let degree_type = edu.degree_type.as_deref().unwrap_or("");
            let mut degree_line = match (degree_type.is_empty(), edu.field.as_deref().filter(|f| !f.is_empty())) {
                (false, Some(field)) => format!("{degree_type} in {field}"),
                (false, None) => degree_type.to_string(),
                (true, _) => edu.degree.clone(),
            };
            if let Some(gpa) = edu.gpa.as_deref().filter(|g| !g.is_empty()) {
                degree_line.push_str(&format!(", GPA: {gpa}"));
            }
            out.push_str(&format!(
                "#sub({}, {}, {}, {})\n",
                lit(&edu.institution),
                lit(&education_location(edu)),
                lit(&degree_line),
                lit(date)
            ));
        }
    }
}

fn render_project(project: &ProjectItem, out: &mut String) {
    out.push_str(&format!("#projhead(strong({}), {})\n", lit(&project.name), opt(&project.date)));
    bullets(&project.bullets, out);
}

fn render_presentation(p: &PresentationItem, out: &mut String) {
    let venue = match p.venue.as_deref().filter(|v| !v.is_empty()) {
        Some(venue) => {
            let date = p.date.as_deref().filter(|d| !d.is_empty()).map(|d| format!(", {d}")).unwrap_or_default();
            format!(" | {venue}{date}")
        }
        None => String::new(),
    };
    out.push_str(&format!(
        "#projhead([#strong({})#emph({})], \"\")\n",
        lit(&format!("\u{201c}{}\u{201d}", p.title)),
        lit(&venue)
    ));
    bullets(&p.bullets, out);
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_core::{
        CertificationCategoryGroup, CertificationEntry, CertificationGroup, ExperienceSubheading,
        IRSectionType, ResumeSummary, SkillCategoryGroup, SkillGroup,
    };

    fn bullet(content: &str) -> ExperienceBullet {
        ExperienceBullet { content: content.into(), entry_id: None, source_chain: None, is_cloned: false }
    }

    fn header() -> ResumeHeader {
        ResumeHeader {
            name: "Ada Lovelace".into(),
            tagline: Some("Analytical engineer".into()),
            location: Some("London, UK".into()),
            email: Some("ada@example.com".into()),
            phone: Some("+1 (555) 010-0100".into()),
            linkedin: Some("https://linkedin.com/in/ada".into()),
            github: None,
            website: None,
            clearance: None,
        }
    }

    fn doc(sections: Vec<IRSection>) -> ResumeDocument {
        ResumeDocument { resume_id: "r".into(), header: header(), summary: None, sections }
    }

    fn section(title: &str, kind: IRSectionType, items: Vec<IRSectionItem>) -> IRSection {
        IRSection { id: "s".into(), section_type: kind, title: title.into(), display_order: 0, items }
    }

    #[test]
    fn literals_escape_backslash_quote_and_control_characters() {
        assert_eq!(lit(r#"a\b"c"#), r#""a\\b\"c""#);
        assert_eq!(lit("line1\nline2\ttab"), r#""line1\nline2\ttab""#);
        assert_eq!(lit("\u{7}"), "\"\\u{7}\"");
        assert_eq!(lit("naïve — 日本"), "\"naïve — 日本\"");
    }

    #[test]
    fn markup_and_code_in_content_stay_inside_string_literals() {
        let hostile = r#"#read("/etc/passwd") *x* $y$ _z_ @ref <tag> \ ] [ "quoted""#;
        let out = render_typst(&doc(vec![section(
            "Experience",
            IRSectionType::Experience,
            vec![IRSectionItem::ExperienceGroup(ExperienceGroup {
                id: "g".into(),
                organization: hostile.into(),
                subheadings: vec![ExperienceSubheading {
                    id: "s".into(),
                    title: "Role".into(),
                    location: None,
                    date_range: "2020".into(),
                    source_id: None,
                    bullets: vec![bullet(hostile)],
                }],
            })],
        )]));
        // The hostile text appears only escaped inside a literal, never bare.
        let escaped = lit(hostile);
        assert!(out.contains(&format!("#sub({escaped}, ")), "organization is a literal");
        assert!(out.contains(&format!("#items({escaped})")), "bullet is a literal");
        let without_literals = out.replace(&escaped, "");
        assert!(!without_literals.contains("/etc/passwd"), "no unescaped copy: {without_literals}");
    }

    #[test]
    fn header_has_name_tagline_and_contact_links() {
        let out = render_typst(&doc(vec![]));
        assert!(out.contains("smallcaps(\"Ada Lovelace\")"));
        assert!(out.contains("small(\"Analytical engineer\")"));
        assert!(out.contains("link(\"mailto:ada@example.com\", underline(\"ada@example.com\"))"));
        assert!(out.contains("link(\"tel:+15550100100\", underline(\"+1 (555) 010-0100\"))"), "digits only in the tel: link");
        assert!(out.contains("underline(\"LinkedIn\")"));
        assert!(!out.contains("GitHub"), "absent fields are omitted");
        assert!(out.contains("#set document(title: \"Ada Lovelace\""));
    }

    #[test]
    fn summary_and_every_section_kind_render() {
        let mut d = doc(vec![
            section(
                "Technical Skills",
                IRSectionType::Skills,
                vec![IRSectionItem::SkillGroup(SkillGroup {
                    categories: vec![SkillCategoryGroup { label: "Languages".into(), skills: vec!["Rust".into(), "Go".into()] }],
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
        ]);
        d.summary = Some(ResumeSummary { summary_id: None, title: None, content: "Builds things.".into(), is_override: false });
        let out = render_typst(&d);
        assert!(out.contains("#sec(\"Summary\")\n#\"Builds things.\""));
        assert!(out.contains("#labelled((\"Languages\", \"Rust, Go\"))"));
        assert!(out.contains("#labelled((\"Cloud\", \"CKA\"))"));
    }

    #[test]
    fn a_second_role_at_the_same_organization_uses_the_compact_subheading() {
        let sub = |title: &str| ExperienceSubheading {
            id: "s".into(),
            title: title.into(),
            location: Some("Remote".into()),
            date_range: "2020 - 2022".into(),
            source_id: None,
            bullets: vec![bullet("Did a thing")],
        };
        let out = render_typst(&doc(vec![section(
            "Experience",
            IRSectionType::Experience,
            vec![IRSectionItem::ExperienceGroup(ExperienceGroup {
                id: "g".into(),
                organization: "Acme".into(),
                subheadings: vec![sub("Senior"), sub("Junior")],
            })],
        )]));
        assert!(out.contains("#sub(\"Acme\", \"\", \"Senior (Remote)\", \"2020 - 2022\")"));
        assert!(out.contains("#subsub(\"Junior (Remote)\", \"2020 - 2022\")"));
        assert_eq!(out.matches("#items(").count(), 2);
    }

    #[test]
    fn education_lines_follow_the_degree_certificate_course_rules() {
        let base = || EducationItem {
            institution: "Caltech".into(),
            degree: "Physics".into(),
            date: "2018".into(),
            entry_id: None,
            source_id: None,
            education_type: None,
            degree_level: None,
            degree_type: Some("Ph.D.".into()),
            field: Some("Physics".into()),
            gpa: Some("4.0".into()),
            location: Some("Old Town".into()),
            credential_id: None,
            issuing_body: None,
            certificate_subtype: None,
            edu_description: None,
            campus_name: None,
            campus_city: Some("Pasadena".into()),
            campus_state: Some("CA".into()),
        };
        let render = |e: EducationItem| {
            render_typst(&doc(vec![section("Education", IRSectionType::Education, vec![IRSectionItem::Education(e)])]))
        };

        let degree = render(base());
        assert!(degree.contains("#sub(\"Caltech\", \"Pasadena, CA\", \"Ph.D. in Physics, GPA: 4.0\", \"2018\")"), "{degree}");

        let mut cert = base();
        cert.education_type = Some("certificate".into());
        cert.degree = "CKA".into();
        cert.issuing_body = Some("CNCF".into());
        cert.credential_id = Some("ABC-123".into());
        assert!(render(cert).contains("#sub(\"CKA\", \"Exp. 2018\", \"CNCF -- Credential ID: ABC-123\", \"\")"));

        let mut course = base();
        course.education_type = Some("course".into());
        course.degree = "Distributed Systems".into();
        assert!(render(course).contains("#sub(\"Distributed Systems\", \"2018\", \"Caltech, Pasadena, CA\", \"\")"));

        let mut self_taught = base();
        self_taught.education_type = Some("self_taught".into());
        self_taught.edu_description = Some("Self-directed study".into());
        assert!(render(self_taught).contains("#items(\"Self-directed study\")"));
    }

    #[test]
    fn projects_and_presentations() {
        let out = render_typst(&doc(vec![
            section(
                "Projects",
                IRSectionType::Projects,
                vec![IRSectionItem::Project(ProjectItem {
                    name: "Forge".into(),
                    description: None,
                    date: Some("2026".into()),
                    entry_id: None,
                    source_id: None,
                    bullets: vec![bullet("Built it")],
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
                    bullets: vec![],
                    description: None,
                    presentation_type: None,
                    url: None,
                    coauthors: None,
                })],
            ),
        ]));
        assert!(out.contains("#projhead(strong(\"Forge\"), \"2026\")"));
        assert!(out.contains("\u{201c}Rust in the browser\u{201d}"));
        assert!(out.contains(" | RustConf, 2026"));
    }
}
