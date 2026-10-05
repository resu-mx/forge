//! Resume IR compiler service — transforms resume data into `ResumeDocument`.
//!
//! Reads resume sections from the database and dispatches to section-type
//! builders. Each builder queries entries via `section_id` and assembles
//! the appropriate IR items.

use std::collections::BTreeMap;

use rusqlite::{params, Connection, OptionalExtension};

use forge_core::{
    CertificationCategoryGroup, CertificationEntry, CertificationGroup, ClearanceItem,
    EducationItem, ExperienceBullet, ExperienceGroup, ExperienceSubheading, ForgeError, IRSection,
    IRSectionItem, IRSectionType, PresentationItem, ProjectItem, ResumeDocument, ResumeHeader,
    ResumeSummary, SkillCategoryGroup, SkillGroup, SourceChain, SummaryItem,
};

/// Resume IR compiler.
pub struct CompilerService;

impl CompilerService {
    /// Compile a resume into the intermediate representation.
    pub fn compile(
        conn: &Connection,
        resume_id: &str,
    ) -> Result<Option<ResumeDocument>, ForgeError> {
        // Fetch resume base data
        let resume = match conn
            .query_row(
                "SELECT id, name, target_role, header, summary_id,
                    generated_tagline, tagline_override,
                    summary_override, show_clearance_in_header
             FROM resumes WHERE id = ?1",
                params![resume_id],
                |row| {
                    Ok(ResumeRow {
                        _id: row.get(0)?,
                        name: row.get(1)?,
                        target_role: row.get(2)?,
                        header: row.get(3)?,
                        summary_id: row.get(4)?,
                        generated_tagline: row.get(5)?,
                        tagline_override: row.get(6)?,
                        summary_override: row.get(7)?,
                        show_clearance_in_header: row.get::<_, i32>(8).unwrap_or(1),
                    })
                },
            )
            .optional()?
        {
            Some(r) => r,
            None => return Ok(None),
        };

        // Build header from profile + resume
        let header = Self::build_header(conn, &resume)?;

        // Build summary
        let summary = Self::build_summary(conn, &resume)?;

        // Fetch sections
        let mut section_stmt = conn.prepare(
            "SELECT id, title, entry_type, position
             FROM resume_sections
             WHERE resume_id = ?1
             ORDER BY position ASC",
        )?;
        let sections_raw: Vec<SectionRow> = section_stmt
            .query_map(params![resume_id], |row| {
                Ok(SectionRow {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    entry_type: row.get(2)?,
                    position: row.get(3)?,
                })
            })?
            .collect::<Result<_, _>>()?;

        // Build IR sections via section-type dispatch
        let mut ir_sections = Vec::new();

        // Prepend synthetic summary section if summary exists
        if let Some(ref sum) = summary {
            ir_sections.push(IRSection {
                id: format!("summary-{}", resume_id),
                section_type: IRSectionType::Summary,
                title: "Summary".into(),
                display_order: -1,
                items: vec![IRSectionItem::Summary(SummaryItem {
                    content: sum.content.clone(),
                    entry_id: None,
                })],
            });
        }

        for section in &sections_raw {
            let items = Self::build_section_items(conn, section)?;
            let section_type = match section.entry_type.as_str() {
                "experience" => IRSectionType::Experience,
                "skills" => IRSectionType::Skills,
                "education" => IRSectionType::Education,
                "projects" => IRSectionType::Projects,
                "certifications" => IRSectionType::Certifications,
                "clearance" => IRSectionType::Clearance,
                "presentations" => IRSectionType::Presentations,
                "awards" => IRSectionType::Awards,
                "freeform" => IRSectionType::Freeform,
                _ => IRSectionType::Custom,
            };

            ir_sections.push(IRSection {
                id: section.id.clone(),
                section_type,
                title: section.title.clone(),
                display_order: section.position,
                items,
            });
        }

        Ok(Some(ResumeDocument {
            resume_id: resume_id.into(),
            header,
            summary,
            sections: ir_sections,
        }))
    }

    /// Render a compiled `ResumeDocument` to Markdown.
    pub fn render_markdown(doc: &ResumeDocument) -> String {
        let mut md = String::new();

        // Header
        md.push_str(&format!("# {}\n\n", doc.header.name));
        if let Some(ref tagline) = doc.header.tagline {
            md.push_str(&format!("*{}*\n\n", tagline));
        }

        let mut contact_parts = Vec::new();
        if let Some(ref email) = doc.header.email {
            contact_parts.push(email.clone());
        }
        if let Some(ref phone) = doc.header.phone {
            contact_parts.push(phone.clone());
        }
        if let Some(ref loc) = doc.header.location {
            contact_parts.push(loc.clone());
        }
        if !contact_parts.is_empty() {
            md.push_str(&contact_parts.join(" | "));
            md.push_str("\n\n");
        }

        let mut link_parts = Vec::new();
        if let Some(ref linkedin) = doc.header.linkedin {
            link_parts.push(linkedin.clone());
        }
        if let Some(ref github) = doc.header.github {
            link_parts.push(github.clone());
        }
        if let Some(ref website) = doc.header.website {
            link_parts.push(website.clone());
        }
        if !link_parts.is_empty() {
            md.push_str(&link_parts.join(" | "));
            md.push_str("\n\n");
        }

        if let Some(ref clearance) = doc.header.clearance {
            md.push_str(&format!("{}\n\n", clearance));
        }

        // Sections
        for section in &doc.sections {
            md.push_str(&format!("## {}\n\n", section.title));

            for item in &section.items {
                match item {
                    IRSectionItem::Summary(s) => {
                        md.push_str(&s.content);
                        md.push_str("\n\n");
                    }
                    IRSectionItem::ExperienceGroup(group) => {
                        md.push_str(&format!("### {}\n\n", group.organization));
                        for sub in &group.subheadings {
                            md.push_str(&format!("**{}** | {}\n\n", sub.title, sub.date_range));
                            if let Some(ref loc) = sub.location {
                                md.push_str(&format!("_{}_\n\n", loc));
                            }
                            for bullet in &sub.bullets {
                                md.push_str(&format!("- {}\n", bullet.content));
                            }
                            md.push('\n');
                        }
                    }
                    IRSectionItem::SkillGroup(group) => {
                        for cat in &group.categories {
                            md.push_str(&format!(
                                "**{}**: {}\n\n",
                                cat.label,
                                cat.skills.join(", ")
                            ));
                        }
                    }
                    IRSectionItem::Education(edu) => {
                        md.push_str(&format!(
                            "**{}** — {} ({})\n\n",
                            edu.institution, edu.degree, edu.date
                        ));
                    }
                    IRSectionItem::Project(proj) => {
                        md.push_str(&format!("### {}\n\n", proj.name));
                        if let Some(ref desc) = proj.description {
                            md.push_str(&format!("{}\n\n", desc));
                        }
                        for bullet in &proj.bullets {
                            md.push_str(&format!("- {}\n", bullet.content));
                        }
                        md.push('\n');
                    }
                    IRSectionItem::CertificationGroup(group) => {
                        for cat in &group.categories {
                            md.push_str(&format!("**{}**: ", cat.label));
                            let names: Vec<&str> =
                                cat.certs.iter().map(|c| c.name.as_str()).collect();
                            md.push_str(&names.join(", "));
                            md.push_str("\n\n");
                        }
                    }
                    IRSectionItem::Clearance(item) => {
                        md.push_str(&format!("- {}\n", item.content));
                    }
                    IRSectionItem::Presentation(pres) => {
                        md.push_str(&format!("### {}\n\n", pres.title));
                        if let Some(ref venue) = pres.venue {
                            md.push_str(&format!("_{}_\n\n", venue));
                        }
                        if let Some(ref desc) = pres.description {
                            md.push_str(&format!("{}\n\n", desc));
                        }
                        for bullet in &pres.bullets {
                            md.push_str(&format!("- {}\n", bullet.content));
                        }
                        md.push('\n');
                    }
                }
            }
        }

        md
    }

    /// Render a compiled `ResumeDocument` to LaTeX source.
    /// Render the document as Typst source (see [`super::typst_render`]).
    pub fn render_typst(doc: &ResumeDocument) -> String {
        super::typst_render::render_typst(doc)
    }

    pub fn render_latex(doc: &ResumeDocument) -> String {
        let mut tex = String::new();

        // Preamble
        tex.push_str("\\documentclass[letterpaper,11pt]{article}\n\n");
        tex.push_str("\\usepackage[empty]{fullpage}\n");
        tex.push_str("\\usepackage{titlesec}\n");
        tex.push_str("\\usepackage[usenames,dvipsnames]{color}\n");
        tex.push_str("\\usepackage{enumitem}\n");
        tex.push_str("\\usepackage[hidelinks]{hyperref}\n");
        tex.push_str("\\usepackage[margin=0.5in]{geometry}\n\n");
        tex.push_str(
            "\\titleformat{\\section}{\\scshape\\raggedright\\large}{}{0em}{}[\\titlerule]\n",
        );
        tex.push_str("\\titlespacing*{\\section}{0pt}{6pt}{4pt}\n\n");
        tex.push_str("\\begin{document}\n\n");

        // Header
        tex.push_str("\\begin{center}\n");
        tex.push_str(&format!(
            "  {{\\Huge \\scshape {}}}\n",
            latex_escape(&doc.header.name)
        ));
        tex.push_str("  \\vspace{2pt}\n");

        if let Some(ref tagline) = doc.header.tagline {
            tex.push_str(&format!("  \\textit{{{}}}\n", latex_escape(tagline)));
            tex.push_str("  \\vspace{2pt}\n");
        }

        let mut contact_parts = Vec::new();
        if let Some(ref email) = doc.header.email {
            contact_parts.push(format!(
                "\\href{{mailto:{}}}{{\\underline{{{}}}}}",
                email,
                latex_escape(email)
            ));
        }
        if let Some(ref phone) = doc.header.phone {
            contact_parts.push(latex_escape(phone));
        }
        if let Some(ref loc) = doc.header.location {
            contact_parts.push(latex_escape(loc));
        }
        if let Some(ref linkedin) = doc.header.linkedin {
            contact_parts.push(format!("\\href{{{}}}{{\\underline{{LinkedIn}}}}", linkedin));
        }
        if let Some(ref github) = doc.header.github {
            contact_parts.push(format!("\\href{{{}}}{{\\underline{{GitHub}}}}", github));
        }
        if let Some(ref website) = doc.header.website {
            contact_parts.push(format!("\\href{{{}}}{{\\underline{{Website}}}}", website));
        }
        if !contact_parts.is_empty() {
            tex.push_str(&format!("  {}\n", contact_parts.join(" $|$ ")));
        }

        if let Some(ref clearance) = doc.header.clearance {
            tex.push_str(&format!(
                "  \\vspace{{2pt}}\n  \\textit{{{}}}\n",
                latex_escape(clearance)
            ));
        }

        tex.push_str("\\end{center}\n\n");

        // Sections
        for section in &doc.sections {
            tex.push_str(&format!("\\section{{{}}}\n", latex_escape(&section.title)));

            for item in &section.items {
                match item {
                    IRSectionItem::Summary(s) => {
                        tex.push_str(&latex_escape(&s.content));
                        tex.push_str("\n\n");
                    }
                    IRSectionItem::ExperienceGroup(group) => {
                        for sub in &group.subheadings {
                            tex.push_str(&format!(
                                "\\textbf{{{}}} \\hfill {} \\\\\n",
                                latex_escape(&group.organization),
                                latex_escape(&sub.date_range)
                            ));
                            let loc_str = sub.location.as_deref().unwrap_or("");
                            tex.push_str(&format!(
                                "\\textit{{{}}} \\hfill \\textit{{{}}}\n",
                                latex_escape(&sub.title),
                                latex_escape(loc_str)
                            ));
                            if !sub.bullets.is_empty() {
                                tex.push_str(
                                    "\\begin{itemize}[leftmargin=0.15in, label={\\textbullet}]\n",
                                );
                                for bullet in &sub.bullets {
                                    tex.push_str(&format!(
                                        "  \\item {}\n",
                                        latex_escape(&bullet.content)
                                    ));
                                }
                                tex.push_str("\\end{itemize}\n");
                            }
                        }
                    }
                    IRSectionItem::SkillGroup(group) => {
                        tex.push_str("\\begin{itemize}[leftmargin=0.15in, label={}]\n");
                        for cat in &group.categories {
                            tex.push_str(&format!(
                                "  \\item \\textbf{{{}}}: {}\n",
                                latex_escape(&cat.label),
                                latex_escape(&cat.skills.join(", "))
                            ));
                        }
                        tex.push_str("\\end{itemize}\n");
                    }
                    IRSectionItem::Education(edu) => {
                        tex.push_str(&format!(
                            "\\textbf{{{}}} \\hfill {} \\\\\n",
                            latex_escape(&edu.institution),
                            latex_escape(&edu.date)
                        ));
                        tex.push_str(&format!("\\textit{{{}}}\n\n", latex_escape(&edu.degree)));
                    }
                    IRSectionItem::Project(proj) => {
                        tex.push_str(&format!("\\textbf{{{}}}\n", latex_escape(&proj.name)));
                        if !proj.bullets.is_empty() {
                            tex.push_str(
                                "\\begin{itemize}[leftmargin=0.15in, label={\\textbullet}]\n",
                            );
                            for bullet in &proj.bullets {
                                tex.push_str(&format!(
                                    "  \\item {}\n",
                                    latex_escape(&bullet.content)
                                ));
                            }
                            tex.push_str("\\end{itemize}\n");
                        }
                    }
                    IRSectionItem::CertificationGroup(group) => {
                        tex.push_str("\\begin{itemize}[leftmargin=0.15in, label={}]\n");
                        for cat in &group.categories {
                            let names: Vec<String> =
                                cat.certs.iter().map(|c| latex_escape(&c.name)).collect();
                            tex.push_str(&format!(
                                "  \\item \\textbf{{{}}}: {}\n",
                                latex_escape(&cat.label),
                                names.join(", ")
                            ));
                        }
                        tex.push_str("\\end{itemize}\n");
                    }
                    IRSectionItem::Clearance(item) => {
                        tex.push_str(&format!("{}\n\n", latex_escape(&item.content)));
                    }
                    IRSectionItem::Presentation(pres) => {
                        tex.push_str(&format!("\\textbf{{{}}}\n", latex_escape(&pres.title)));
                        if let Some(ref venue) = pres.venue {
                            tex.push_str(&format!("\\textit{{{}}}\n", latex_escape(venue)));
                        }
                        if !pres.bullets.is_empty() {
                            tex.push_str(
                                "\\begin{itemize}[leftmargin=0.15in, label={\\textbullet}]\n",
                            );
                            for bullet in &pres.bullets {
                                tex.push_str(&format!(
                                    "  \\item {}\n",
                                    latex_escape(&bullet.content)
                                ));
                            }
                            tex.push_str("\\end{itemize}\n");
                        }
                    }
                }
            }
        }

        tex.push_str("\\end{document}\n");
        tex
    }

    // ── Internal: header, summary ───────────────────────────────────

    fn build_header(conn: &Connection, resume: &ResumeRow) -> Result<ResumeHeader, ForgeError> {
        // Fetch profile
        let profile = conn
            .query_row(
                "SELECT up.name, up.email, up.phone, a.name AS location
             FROM user_profile up
             LEFT JOIN addresses a ON a.id = up.address_id
             LIMIT 1",
                [],
                |row| {
                    Ok(ProfileRow {
                        name: row.get(0)?,
                        email: row.get(1)?,
                        phone: row.get(2)?,
                        location: row.get(3)?,
                    })
                },
            )
            .optional()?;

        // Fetch URLs from profile_urls
        let (linkedin, github, website) = if let Some(ref _profile) = profile {
            let profile_id: Option<String> = conn
                .query_row("SELECT id FROM user_profile LIMIT 1", [], |row| row.get(0))
                .optional()?;

            if let Some(pid) = profile_id {
                let mut url_stmt =
                    conn.prepare("SELECT key, url FROM profile_urls WHERE profile_id = ?1")?;
                let urls: Vec<(String, String)> = url_stmt
                    .query_map(params![pid], |row| Ok((row.get(0)?, row.get(1)?)))?
                    .collect::<Result<_, _>>()?;

                let linkedin = urls
                    .iter()
                    .find(|(k, _)| k == "linkedin")
                    .map(|(_, v)| v.clone());
                let github = urls
                    .iter()
                    .find(|(k, _)| k == "github")
                    .map(|(_, v)| v.clone());
                let website = urls
                    .iter()
                    .find(|(k, _)| k == "blog" || k == "portfolio" || k == "website")
                    .map(|(_, v)| v.clone());
                (linkedin, github, website)
            } else {
                (None, None, None)
            }
        } else {
            (None, None, None)
        };

        // Tagline resolution: tagline_override > generated_tagline > header.tagline > target_role
        let header_tagline = resume.header.as_ref().and_then(|h| {
            serde_json::from_str::<serde_json::Value>(h)
                .ok()
                .and_then(|v| {
                    v.get("tagline")
                        .and_then(|t| t.as_str().map(|s| s.to_string()))
                })
        });
        let tagline = resume
            .tagline_override
            .clone()
            .or_else(|| resume.generated_tagline.clone())
            .or(header_tagline)
            .or_else(|| Some(resume.target_role.clone()));

        // Clearance in header
        let clearance = if resume.show_clearance_in_header != 0 {
            Self::build_header_clearance_line(conn)?
        } else {
            None
        };

        let p = profile.as_ref();
        Ok(ResumeHeader {
            name: p
                .map(|p| p.name.clone())
                .unwrap_or_else(|| resume.name.clone()),
            tagline,
            location: p.and_then(|p| p.location.clone()),
            email: p.and_then(|p| p.email.clone()),
            phone: p.and_then(|p| p.phone.clone()),
            linkedin,
            github,
            website,
            clearance,
        })
    }

    fn build_header_clearance_line(conn: &Connection) -> Result<Option<String>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT details FROM credentials
             WHERE credential_type = 'clearance' AND status = 'active'",
        )?;
        let details: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<_, _>>()?;

        if details.is_empty() {
            return Ok(None);
        }

        // Find highest level clearance
        let level_order = |level: &str| -> i32 {
            match level {
                "top_secret_sci" | "ts_sci" => 0,
                "top_secret" => 1,
                "secret" => 2,
                "confidential" => 3,
                _ => 99,
            }
        };

        let mut best_level: Option<String> = None;
        let mut best_poly: Option<String> = None;
        let mut best_rank = 99;

        for detail_json in &details {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(detail_json) {
                if let Some(level) = val.get("level").and_then(|v| v.as_str()) {
                    let rank = level_order(level);
                    if rank < best_rank {
                        best_rank = rank;
                        best_level = Some(level.to_string());
                        best_poly = val
                            .get("polygraph")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                    }
                }
            }
        }

        let line = best_level.map(|level| {
            let display_level = match level.as_str() {
                "top_secret_sci" | "ts_sci" => "TS/SCI",
                "top_secret" => "Top Secret",
                "secret" => "Secret",
                "confidential" => "Confidential",
                _ => &level,
            };
            let poly_str = best_poly
                .as_ref()
                .map(|p| match p.as_str() {
                    "full_scope" => " with Full-Scope Poly",
                    "ci" => " with CI Poly",
                    "lifestyle" => " with Lifestyle Poly",
                    _ => "",
                })
                .unwrap_or("");

            format!("Active {} Clearance{}", display_level, poly_str)
        });

        Ok(line)
    }

    fn build_summary(
        conn: &Connection,
        resume: &ResumeRow,
    ) -> Result<Option<ResumeSummary>, ForgeError> {
        // summary_override takes priority
        if let Some(ref content) = resume.summary_override {
            return Ok(Some(ResumeSummary {
                summary_id: resume.summary_id.clone(),
                title: None,
                content: content.clone(),
                is_override: true,
            }));
        }

        // Try summary via FK
        if let Some(ref sid) = resume.summary_id {
            let summary = conn
                .query_row(
                    "SELECT title, description FROM summaries WHERE id = ?1",
                    params![sid],
                    |row| {
                        Ok((
                            row.get::<_, Option<String>>(0)?,
                            row.get::<_, Option<String>>(1)?,
                        ))
                    },
                )
                .optional()?;

            if let Some((title, description)) = summary {
                if let Some(desc) = description {
                    return Ok(Some(ResumeSummary {
                        summary_id: Some(sid.clone()),
                        title,
                        content: desc,
                        is_override: false,
                    }));
                }
            }
        }

        Ok(None)
    }

    // ── Section dispatch ────────────────────────────────────────────

    fn build_section_items(
        conn: &Connection,
        section: &SectionRow,
    ) -> Result<Vec<IRSectionItem>, ForgeError> {
        match section.entry_type.as_str() {
            "experience" => Self::build_experience_items(conn, &section.id),
            "skills" => Self::build_skill_items(conn, &section.id),
            "education" => Self::build_education_items(conn, &section.id),
            "projects" => Self::build_project_items(conn, &section.id),
            "certifications" => Self::build_certification_items(conn, &section.id),
            "clearance" => Self::build_clearance_items(conn),
            "presentations" => Self::build_presentation_items(conn, &section.id),
            "freeform" | "awards" => Self::build_freeform_items(conn, &section.id),
            _ => Ok(Vec::new()),
        }
    }

    // ── Experience ──────────────────────────────────────────────────

    fn build_experience_items(
        conn: &Connection,
        section_id: &str,
    ) -> Result<Vec<IRSectionItem>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT
               re.id AS entry_id,
               re.content AS entry_content,
               re.perspective_id,
               re.position,
               p.content AS perspective_content,
               p.bullet_id,
               b.content AS bullet_content,
               COALESCE(re.source_id, bs.source_id) AS source_id,
               s.title AS source_title,
               sr.organization_id,
               sr.start_date,
               sr.end_date,
               sr.is_current,
               sr.work_arrangement,
               o.name AS org_name,
               o.employment_type,
               addr.city AS org_city,
               addr.state AS org_state
             FROM resume_entries re
             LEFT JOIN perspectives p ON p.id = re.perspective_id
             LEFT JOIN bullets b ON b.id = p.bullet_id
             LEFT JOIN bullet_sources bs ON bs.bullet_id = p.bullet_id AND bs.is_primary = 1
             LEFT JOIN sources s ON s.id = COALESCE(re.source_id, bs.source_id)
             LEFT JOIN source_roles sr ON sr.source_id = s.id
             LEFT JOIN organizations o ON o.id = sr.organization_id
             LEFT JOIN org_locations ol ON ol.id = (
               SELECT id FROM org_locations
               WHERE organization_id = o.id
               ORDER BY is_headquarters DESC, id ASC
               LIMIT 1
             )
             LEFT JOIN addresses addr ON addr.id = ol.address_id
             WHERE re.section_id = ?1
             ORDER BY sr.is_current DESC, sr.start_date DESC, re.position ASC",
        )?;

        let rows: Vec<ExperienceRow> = stmt
            .query_map(params![section_id], |row| {
                Ok(ExperienceRow {
                    entry_id: row.get(0)?,
                    entry_content: row.get(1)?,
                    perspective_id: row.get(2)?,
                    _position: row.get(3)?,
                    perspective_content: row.get(4)?,
                    bullet_id: row.get(5)?,
                    bullet_content: row.get(6)?,
                    source_id: row.get(7)?,
                    source_title: row.get(8)?,
                    organization_id: row.get(9)?,
                    start_date: row.get(10)?,
                    end_date: row.get(11)?,
                    is_current: row.get(12)?,
                    work_arrangement: row.get(13)?,
                    org_name: row.get(14)?,
                    _employment_type: row.get(15)?,
                    org_city: row.get(16)?,
                    org_state: row.get(17)?,
                })
            })?
            .collect::<Result<_, _>>()?;

        // Group by organization, then by role (source_title)
        let mut org_map: BTreeMap<String, Vec<&ExperienceRow>> = BTreeMap::new();
        for row in &rows {
            let org_key = row
                .organization_id
                .clone()
                .or_else(|| row.org_name.clone())
                .unwrap_or_else(|| "Other".into());
            org_map.entry(org_key).or_default().push(row);
        }

        let mut items = Vec::new();
        for (org_key, org_rows) in &org_map {
            let first = org_rows[0];
            let org_display = build_org_display_string(
                first.org_name.as_deref(),
                first.org_city.as_deref(),
                first.org_state.as_deref(),
                first.work_arrangement.as_deref(),
            );

            // Group by role (source_title)
            let mut role_map: BTreeMap<String, Vec<&&ExperienceRow>> = BTreeMap::new();
            for row in org_rows {
                let role_key = row.source_title.clone().unwrap_or_else(|| {
                    format!("untitled:{}", row.source_id.as_deref().unwrap_or("unknown"))
                });
                role_map.entry(role_key).or_default().push(row);
            }

            let mut subheadings = Vec::new();
            for (role_key, role_rows) in &role_map {
                let role_first = role_rows[0];
                let title = if role_key.starts_with("untitled:") {
                    "Untitled Role".into()
                } else {
                    role_key.clone()
                };

                let location = build_location_string(
                    role_first.org_city.as_deref(),
                    role_first.org_state.as_deref(),
                    role_first.work_arrangement.as_deref(),
                );

                let date_range = format_date_range(
                    role_first.start_date.as_deref(),
                    role_first.end_date.as_deref(),
                    role_first.is_current.unwrap_or(0),
                );

                let bullets: Vec<ExperienceBullet> = role_rows
                    .iter()
                    .map(|r| {
                        let content = r
                            .entry_content
                            .clone()
                            .or_else(|| r.perspective_content.clone())
                            .or_else(|| r.bullet_content.clone())
                            .unwrap_or_default();

                        let source_chain = if r.perspective_id.is_some()
                            && r.bullet_id.is_some()
                            && r.source_id.is_some()
                            && r.source_title.is_some()
                        {
                            Some(SourceChain {
                                source_id: r.source_id.clone().unwrap(),
                                source_title: truncate(r.source_title.as_deref().unwrap(), 60),
                                bullet_id: r.bullet_id.clone().unwrap(),
                                bullet_preview: truncate(
                                    r.bullet_content.as_deref().unwrap_or(""),
                                    60,
                                ),
                                perspective_id: r.perspective_id.clone().unwrap(),
                                perspective_preview: truncate(
                                    r.perspective_content.as_deref().unwrap_or(""),
                                    60,
                                ),
                            })
                        } else {
                            None
                        };

                        ExperienceBullet {
                            content,
                            entry_id: Some(r.entry_id.clone()),
                            source_chain,
                            is_cloned: false,
                        }
                    })
                    .collect();

                subheadings.push(ExperienceSubheading {
                    id: format!("sub-{}", role_first.entry_id),
                    title,
                    location,
                    date_range,
                    source_id: role_first.source_id.clone(),
                    bullets,
                });
            }

            items.push(IRSectionItem::ExperienceGroup(ExperienceGroup {
                id: format!("org-{}", org_key),
                organization: org_display,
                subheadings,
            }));
        }

        Ok(items)
    }

    // ── Skills ──────────────────────────────────────────────────────

    fn build_skill_items(
        conn: &Connection,
        section_id: &str,
    ) -> Result<Vec<IRSectionItem>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT s.name AS skill_name, COALESCE(sc.display_name, s.category) AS category_display
             FROM resume_skills rs
             JOIN skills s ON s.id = rs.skill_id
             LEFT JOIN skill_categories sc ON sc.slug = s.category
             WHERE rs.section_id = ?1
             ORDER BY COALESCE(sc.position, 999) ASC, rs.position ASC",
        )?;

        let rows: Vec<(String, String)> = stmt
            .query_map(params![section_id], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<_, _>>()?;

        // Group by category, preserving order
        let mut categories: Vec<SkillCategoryGroup> = Vec::new();
        let mut seen: Vec<String> = Vec::new();
        for (skill_name, category) in &rows {
            if let Some(cat) = categories.iter_mut().find(|c| c.label == *category) {
                cat.skills.push(skill_name.clone());
            } else {
                seen.push(category.clone());
                categories.push(SkillCategoryGroup {
                    label: category.clone(),
                    skills: vec![skill_name.clone()],
                });
            }
        }

        if categories.is_empty() {
            return Ok(Vec::new());
        }

        Ok(vec![IRSectionItem::SkillGroup(SkillGroup { categories })])
    }

    // ── Education ───────────────────────────────────────────────────

    fn build_education_items(
        conn: &Connection,
        section_id: &str,
    ) -> Result<Vec<IRSectionItem>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT
               re.id AS entry_id,
               re.content AS entry_content,
               p.content AS perspective_content,
               s.title AS source_title,
               COALESCE(re.source_id, bs.source_id) AS source_id,
               se.education_type,
               o.name AS institution,
               se.field,
               se.end_date,
               se.degree_level,
               se.degree_type,
               se.gpa,
               COALESCE(se.location,
                 CASE
                   WHEN addr.city IS NOT NULL AND addr.state IS NOT NULL THEN addr.city || ', ' || addr.state
                   WHEN addr.city IS NOT NULL THEN addr.city
                   WHEN addr.state IS NOT NULL THEN addr.state
                   ELSE NULL
                 END
               ) AS location,
               se.credential_id,
               o.name AS issuing_body,
               se.certificate_subtype,
               se.edu_description,
               se.organization_id,
               ol.name AS campus_name,
               addr.city AS campus_city,
               addr.state AS campus_state
             FROM resume_entries re
             LEFT JOIN perspectives p ON p.id = re.perspective_id
             LEFT JOIN bullet_sources bs ON bs.bullet_id = p.bullet_id AND bs.is_primary = 1
             LEFT JOIN sources s ON s.id = COALESCE(re.source_id, bs.source_id)
             LEFT JOIN source_education se ON se.source_id = s.id
             LEFT JOIN organizations o ON o.id = se.organization_id
             LEFT JOIN org_locations ol ON ol.id = se.campus_id
             LEFT JOIN addresses addr ON addr.id = ol.address_id
             WHERE re.section_id = ?1
             ORDER BY CASE WHEN se.end_date IS NULL THEN 1 ELSE 0 END DESC,
                      se.end_date DESC,
                      re.position ASC",
        )?;

        let items: Vec<IRSectionItem> = stmt
            .query_map(params![section_id], |row| {
                let entry_content: Option<String> = row.get(1)?;
                let perspective_content: Option<String> = row.get(2)?;
                let source_title: Option<String> = row.get(3)?;
                let end_date: Option<String> = row.get(8)?;

                let degree = entry_content
                    .or(perspective_content)
                    .or(source_title)
                    .unwrap_or_default();

                let date = end_date
                    .as_ref()
                    .and_then(|d| d.get(..4).map(|s| s.to_string()))
                    .unwrap_or_default();

                Ok(IRSectionItem::Education(EducationItem {
                    institution: row
                        .get::<_, Option<String>>(6)?
                        .unwrap_or_else(|| "Unknown".into()),
                    degree,
                    date,
                    entry_id: row.get(0)?,
                    source_id: row.get(4)?,
                    education_type: row.get(5)?,
                    degree_level: row.get(9)?,
                    degree_type: row.get(10)?,
                    field: row.get(7)?,
                    gpa: row.get::<_, Option<String>>(11)?,
                    location: row.get(12)?,
                    credential_id: row.get(13)?,
                    issuing_body: row.get(14)?,
                    certificate_subtype: row.get(15)?,
                    edu_description: row.get(16)?,
                    campus_name: row.get(18)?,
                    campus_city: row.get(19)?,
                    campus_state: row.get(20)?,
                }))
            })?
            .collect::<Result<_, _>>()?;

        Ok(items)
    }

    // ── Projects ────────────────────────────────────────────────────

    fn build_project_items(
        conn: &Connection,
        section_id: &str,
    ) -> Result<Vec<IRSectionItem>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT
               re.id AS entry_id,
               re.content AS entry_content,
               re.perspective_id,
               p.content AS perspective_content,
               p.bullet_id,
               b.content AS bullet_content,
               COALESCE(re.source_id, bs.source_id) AS source_id,
               s.title AS source_title,
               s.description AS source_description,
               sp.end_date
             FROM resume_entries re
             LEFT JOIN perspectives p ON p.id = re.perspective_id
             LEFT JOIN bullets b ON b.id = p.bullet_id
             LEFT JOIN bullet_sources bs ON bs.bullet_id = p.bullet_id AND bs.is_primary = 1
             LEFT JOIN sources s ON s.id = COALESCE(re.source_id, bs.source_id)
             LEFT JOIN source_projects sp ON sp.source_id = s.id
             WHERE re.section_id = ?1
             ORDER BY re.position ASC",
        )?;

        #[derive(Debug)]
        struct ProjectRow {
            entry_id: String,
            entry_content: Option<String>,
            perspective_id: Option<String>,
            perspective_content: Option<String>,
            bullet_id: Option<String>,
            bullet_content: Option<String>,
            source_id: Option<String>,
            source_title: Option<String>,
            source_description: Option<String>,
            end_date: Option<String>,
        }

        let rows: Vec<ProjectRow> = stmt
            .query_map(params![section_id], |row| {
                Ok(ProjectRow {
                    entry_id: row.get(0)?,
                    entry_content: row.get(1)?,
                    perspective_id: row.get(2)?,
                    perspective_content: row.get(3)?,
                    bullet_id: row.get(4)?,
                    bullet_content: row.get(5)?,
                    source_id: row.get(6)?,
                    source_title: row.get(7)?,
                    source_description: row.get(8)?,
                    end_date: row.get(9)?,
                })
            })?
            .collect::<Result<_, _>>()?;

        // Group by source_title (project name)
        let mut project_map: Vec<(String, Vec<&ProjectRow>)> = Vec::new();
        for row in &rows {
            let key = row.source_title.clone().unwrap_or_else(|| {
                format!(
                    "untitled:{}",
                    row.source_id.as_deref().unwrap_or(&row.entry_id)
                )
            });
            if let Some(entry) = project_map.iter_mut().find(|(k, _)| k == &key) {
                entry.1.push(row);
            } else {
                project_map.push((key, vec![row]));
            }
        }

        let mut items = Vec::new();
        for (key, proj_rows) in &project_map {
            let first = proj_rows[0];
            let name = if key.starts_with("untitled:") {
                "Untitled Project".into()
            } else {
                key.clone()
            };

            let date = first
                .end_date
                .as_ref()
                .and_then(|d| d.get(..4).map(|s| s.to_string()));

            let bullets: Vec<ExperienceBullet> = proj_rows
                .iter()
                .map(|r| {
                    let content = r
                        .entry_content
                        .clone()
                        .or_else(|| r.perspective_content.clone())
                        .or_else(|| r.bullet_content.clone())
                        .unwrap_or_default();

                    let source_chain = if r.perspective_id.is_some()
                        && r.bullet_id.is_some()
                        && r.source_id.is_some()
                        && r.source_title.is_some()
                    {
                        Some(SourceChain {
                            source_id: r.source_id.clone().unwrap(),
                            source_title: truncate(r.source_title.as_deref().unwrap(), 60),
                            bullet_id: r.bullet_id.clone().unwrap(),
                            bullet_preview: truncate(r.bullet_content.as_deref().unwrap_or(""), 60),
                            perspective_id: r.perspective_id.clone().unwrap(),
                            perspective_preview: truncate(
                                r.perspective_content.as_deref().unwrap_or(""),
                                60,
                            ),
                        })
                    } else {
                        None
                    };

                    ExperienceBullet {
                        content,
                        entry_id: Some(r.entry_id.clone()),
                        source_chain,
                        is_cloned: false,
                    }
                })
                .collect();

            let has_real_bullets = bullets.iter().any(|b| !b.content.is_empty());
            let description = if has_real_bullets {
                None
            } else {
                first.source_description.clone()
            };

            items.push(IRSectionItem::Project(ProjectItem {
                name,
                description,
                date,
                entry_id: Some(first.entry_id.clone()),
                source_id: first.source_id.clone(),
                bullets,
            }));
        }

        Ok(items)
    }

    // ── Certifications ──────────────────────────────────────────────

    fn build_certification_items(
        conn: &Connection,
        section_id: &str,
    ) -> Result<Vec<IRSectionItem>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT rc.id AS entry_id, c.short_name,
                    o.name AS issuer_name
             FROM resume_certifications rc
             JOIN certifications c ON c.id = rc.certification_id
             LEFT JOIN organizations o ON o.id = c.issuer_id
             WHERE rc.section_id = ?1
             ORDER BY rc.position ASC",
        )?;

        let rows: Vec<(String, String, Option<String>)> = stmt
            .query_map(params![section_id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })?
            .collect::<Result<_, _>>()?;

        // Group by issuer
        let mut cat_map: Vec<(String, Vec<CertificationEntry>)> = Vec::new();
        for (entry_id, short_name, issuer) in &rows {
            let issuer_key = issuer.clone().unwrap_or_else(|| "Other".into());
            let entry = CertificationEntry {
                name: short_name.clone(),
                entry_id: Some(entry_id.clone()),
                source_id: None,
            };
            if let Some(cat) = cat_map.iter_mut().find(|(k, _)| k == &issuer_key) {
                cat.1.push(entry);
            } else {
                cat_map.push((issuer_key, vec![entry]));
            }
        }

        if cat_map.is_empty() {
            return Ok(Vec::new());
        }

        let categories: Vec<CertificationCategoryGroup> = cat_map
            .into_iter()
            .map(|(label, certs)| CertificationCategoryGroup { label, certs })
            .collect();

        Ok(vec![IRSectionItem::CertificationGroup(
            CertificationGroup { categories },
        )])
    }

    // ── Clearance ───────────────────────────────────────────────────

    fn build_clearance_items(conn: &Connection) -> Result<Vec<IRSectionItem>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT id, label, status, details
             FROM credentials
             WHERE credential_type = 'clearance'
             ORDER BY
               CASE status WHEN 'active' THEN 0 WHEN 'inactive' THEN 1 ELSE 2 END,
               label ASC",
        )?;

        let items: Vec<IRSectionItem> = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let label: String = row.get(1)?;
                let status: String = row.get(2)?;
                let details_json: String = row.get::<_, String>(3).unwrap_or_else(|_| "{}".into());

                let content = if !label.is_empty() {
                    let mut c = label;
                    if status == "inactive" {
                        c.push_str(" (Inactive)");
                    }
                    c
                } else {
                    let val: serde_json::Value =
                        serde_json::from_str(&details_json).unwrap_or_default();
                    let level = val
                        .get("level")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown");
                    let display_level = match level {
                        "top_secret_sci" | "ts_sci" => "TS/SCI",
                        "top_secret" => "Top Secret",
                        "secret" => "Secret",
                        "confidential" => "Confidential",
                        _ => level,
                    };
                    let poly = val.get("polygraph").and_then(|v| v.as_str());
                    let poly_str = match poly {
                        Some("full_scope") => " with Full-Scope Poly",
                        Some("ci") => " with CI Poly",
                        Some("lifestyle") => " with Lifestyle Poly",
                        _ => "",
                    };
                    let mut c = format!("{}{}", display_level, poly_str);
                    if status == "inactive" {
                        c.push_str(" (Inactive)");
                    }
                    c
                };

                Ok(IRSectionItem::Clearance(ClearanceItem {
                    content,
                    entry_id: Some(id),
                    source_id: None,
                }))
            })?
            .collect::<Result<_, _>>()?;

        Ok(items)
    }

    // ── Presentations ───────────────────────────────────────────────

    fn build_presentation_items(
        conn: &Connection,
        section_id: &str,
    ) -> Result<Vec<IRSectionItem>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT
               re.id AS entry_id,
               re.content AS entry_content,
               re.perspective_id,
               p.content AS perspective_content,
               p.bullet_id,
               b.content AS bullet_content,
               COALESCE(re.source_id, bs.source_id) AS source_id,
               s.title AS source_title,
               s.description AS source_description,
               s.end_date,
               sp.venue,
               sp.presentation_type,
               sp.url AS presentation_url,
               sp.coauthors
             FROM resume_entries re
             LEFT JOIN perspectives p ON p.id = re.perspective_id
             LEFT JOIN bullets b ON b.id = p.bullet_id
             LEFT JOIN bullet_sources bs ON bs.bullet_id = p.bullet_id AND bs.is_primary = 1
             LEFT JOIN sources s ON s.id = COALESCE(re.source_id, bs.source_id)
             LEFT JOIN source_presentations sp ON sp.source_id = s.id
             WHERE re.section_id = ?1
             ORDER BY re.position ASC",
        )?;

        #[derive(Debug)]
        struct PresRow {
            entry_id: String,
            entry_content: Option<String>,
            _perspective_id: Option<String>,
            perspective_content: Option<String>,
            _bullet_id: Option<String>,
            bullet_content: Option<String>,
            source_id: Option<String>,
            source_title: Option<String>,
            source_description: Option<String>,
            end_date: Option<String>,
            venue: Option<String>,
            presentation_type: Option<String>,
            url: Option<String>,
            coauthors: Option<String>,
        }

        let rows: Vec<PresRow> = stmt
            .query_map(params![section_id], |row| {
                Ok(PresRow {
                    entry_id: row.get(0)?,
                    entry_content: row.get(1)?,
                    _perspective_id: row.get(2)?,
                    perspective_content: row.get(3)?,
                    _bullet_id: row.get(4)?,
                    bullet_content: row.get(5)?,
                    source_id: row.get(6)?,
                    source_title: row.get(7)?,
                    source_description: row.get(8)?,
                    end_date: row.get(9)?,
                    venue: row.get(10)?,
                    presentation_type: row.get(11)?,
                    url: row.get(12)?,
                    coauthors: row.get(13)?,
                })
            })?
            .collect::<Result<_, _>>()?;

        // Group by source_title
        let mut pres_map: Vec<(String, Vec<&PresRow>)> = Vec::new();
        for row in &rows {
            let key = row.source_title.clone().unwrap_or_else(|| {
                format!(
                    "untitled:{}",
                    row.source_id.as_deref().unwrap_or(&row.entry_id)
                )
            });
            if let Some(entry) = pres_map.iter_mut().find(|(k, _)| k == &key) {
                entry.1.push(row);
            } else {
                pres_map.push((key, vec![row]));
            }
        }

        let mut items = Vec::new();
        for (key, pres_rows) in &pres_map {
            let first = pres_rows[0];
            let title = if key.starts_with("untitled:") {
                "Untitled Presentation".into()
            } else {
                key.clone()
            };

            let date = first
                .end_date
                .as_ref()
                .and_then(|d| d.get(..4).map(|s| s.to_string()));

            let bullets: Vec<ExperienceBullet> = pres_rows
                .iter()
                .map(|r| {
                    let content = r
                        .entry_content
                        .clone()
                        .or_else(|| r.perspective_content.clone())
                        .or_else(|| r.bullet_content.clone())
                        .unwrap_or_default();

                    ExperienceBullet {
                        content,
                        entry_id: Some(r.entry_id.clone()),
                        source_chain: None,
                        is_cloned: false,
                    }
                })
                .collect();

            let has_real_bullets = bullets.iter().any(|b| !b.content.is_empty());
            let description = if has_real_bullets {
                None
            } else {
                first.source_description.clone()
            };

            items.push(IRSectionItem::Presentation(PresentationItem {
                title,
                venue: first.venue.clone(),
                date,
                entry_id: Some(first.entry_id.clone()),
                source_id: first.source_id.clone(),
                bullets,
                description,
                presentation_type: first.presentation_type.clone(),
                url: first.url.clone(),
                coauthors: first.coauthors.clone(),
            }));
        }

        Ok(items)
    }

    // ── Freeform / Awards ───────────────────────────────────────────

    fn build_freeform_items(
        conn: &Connection,
        section_id: &str,
    ) -> Result<Vec<IRSectionItem>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT re.id, re.content, p.content AS perspective_content
             FROM resume_entries re
             LEFT JOIN perspectives p ON p.id = re.perspective_id
             WHERE re.section_id = ?1
             ORDER BY re.position ASC",
        )?;

        let items: Vec<IRSectionItem> = stmt
            .query_map(params![section_id], |row| {
                let entry_id: String = row.get(0)?;
                let content: Option<String> = row.get(1)?;
                let perspective: Option<String> = row.get(2)?;

                Ok(IRSectionItem::Summary(SummaryItem {
                    content: content.or(perspective).unwrap_or_default(),
                    entry_id: Some(entry_id),
                }))
            })?
            .collect::<Result<_, _>>()?;

        Ok(items)
    }
}

// ── Helper types ────────────────────────────────────────────────────

struct ResumeRow {
    _id: String,
    name: String,
    target_role: String,
    header: Option<String>,
    summary_id: Option<String>,
    generated_tagline: Option<String>,
    tagline_override: Option<String>,
    summary_override: Option<String>,
    show_clearance_in_header: i32,
}

struct ProfileRow {
    name: String,
    email: Option<String>,
    phone: Option<String>,
    location: Option<String>,
}

struct SectionRow {
    id: String,
    title: String,
    entry_type: String,
    position: i32,
}

struct ExperienceRow {
    entry_id: String,
    entry_content: Option<String>,
    perspective_id: Option<String>,
    _position: i32,
    perspective_content: Option<String>,
    bullet_id: Option<String>,
    bullet_content: Option<String>,
    source_id: Option<String>,
    source_title: Option<String>,
    organization_id: Option<String>,
    start_date: Option<String>,
    end_date: Option<String>,
    is_current: Option<i32>,
    work_arrangement: Option<String>,
    org_name: Option<String>,
    _employment_type: Option<String>,
    org_city: Option<String>,
    org_state: Option<String>,
}

// ── Free functions ──────────────────────────────────────────────────

fn format_date_range(start: Option<&str>, end: Option<&str>, is_current: i32) -> String {
    let fmt = |date: &str| -> String {
        // Parse ISO date and format as "Mon YYYY"
        if date.len() >= 7 {
            let month = &date[5..7];
            let year = &date[..4];
            let month_name = match month {
                "01" => "Jan",
                "02" => "Feb",
                "03" => "Mar",
                "04" => "Apr",
                "05" => "May",
                "06" => "Jun",
                "07" => "Jul",
                "08" => "Aug",
                "09" => "Sep",
                "10" => "Oct",
                "11" => "Nov",
                "12" => "Dec",
                _ => month,
            };
            format!("{} {}", month_name, year)
        } else if date.len() >= 4 {
            date[..4].to_string()
        } else {
            date.to_string()
        }
    };

    match (start, end, is_current) {
        (Some(s), _, 1) => format!("{} - Present", fmt(s)),
        (Some(s), Some(e), _) => format!("{} - {}", fmt(s), fmt(e)),
        (Some(s), None, _) => fmt(s),
        (None, Some(e), _) => fmt(e),
        (None, None, _) => String::new(),
    }
}

/// Build the organization display string for experience sections.
/// Prioritizes location over work arrangement.
fn build_org_display_string(
    org_name: Option<&str>,
    city: Option<&str>,
    state: Option<&str>,
    work_arrangement: Option<&str>,
) -> String {
    let name = org_name.unwrap_or("Other");
    match (city, state) {
        (Some(c), Some(s)) => format!("{name} ({c}, {s})"),
        (Some(c), None) => format!("{name} ({c})"),
        (None, Some(s)) => format!("{name} ({s})"),
        (None, None) => match work_arrangement {
            Some(w) => {
                let mut chars = w.chars();
                let label: String = match chars.next() {
                    Some(first) => first.to_uppercase().chain(chars).collect(),
                    None => return name.to_string(),
                };
                format!("{name} ({label})")
            }
            None => name.to_string(),
        },
    }
}

fn build_location_string(
    city: Option<&str>,
    state: Option<&str>,
    work_arrangement: Option<&str>,
) -> Option<String> {
    match (city, state) {
        (Some(c), Some(s)) => Some(format!("{}, {}", c, s)),
        (Some(c), None) => Some(c.to_string()),
        (None, Some(s)) => Some(s.to_string()),
        (None, None) => work_arrangement.map(|w| w.to_string()),
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

fn latex_escape(s: &str) -> String {
    s.replace('\\', "\\textbackslash{}")
        .replace('&', "\\&")
        .replace('%', "\\%")
        .replace('$', "\\$")
        .replace('#', "\\#")
        .replace('_', "\\_")
        .replace('{', "\\{")
        .replace('}', "\\}")
        .replace('~', "\\textasciitilde{}")
        .replace('^', "\\textasciicircum{}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::stores::resume::ResumeStore;
    use crate::db::stores::skill::SkillStore;
    use crate::forge::Forge;
    use forge_core::{AddResumeEntry, CreateResume, SkillCategory};

    fn setup() -> Forge {
        Forge::open_memory().unwrap()
    }

    fn create_profile(conn: &Connection) {
        // Update the migration-seeded profile instead of inserting a new one
        conn.execute(
            "UPDATE user_profile SET name = 'Adam Smith', email = 'adam@example.com', phone = '555-0100'",
            [],
        ).unwrap();
    }

    fn create_resume(conn: &Connection) -> String {
        let resume = ResumeStore::create(
            conn,
            &CreateResume {
                name: "SRE Resume".into(),
                target_role: "Site Reliability Engineer".into(),
                target_employer: "Acme Corp".into(),
                archetype: "sre".into(),
                summary_id: None,
            },
        )
        .unwrap();
        resume.id
    }

    #[test]
    fn compile_nonexistent_resume_returns_none() {
        let forge = setup();
        let result = CompilerService::compile(forge.conn(), "nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn compile_empty_resume() {
        let forge = setup();
        create_profile(forge.conn());
        let resume_id = create_resume(forge.conn());

        let doc = CompilerService::compile(forge.conn(), &resume_id)
            .unwrap()
            .unwrap();
        assert_eq!(doc.resume_id, resume_id);
        assert_eq!(doc.header.name, "Adam Smith");
        assert_eq!(doc.header.email, Some("adam@example.com".into()));
        assert!(doc.summary.is_none());
        assert!(doc.sections.is_empty());
    }

    #[test]
    fn compile_with_tagline_priority() {
        let forge = setup();
        create_profile(forge.conn());
        let resume_id = create_resume(forge.conn());

        // Default: falls back to target_role
        let doc = CompilerService::compile(forge.conn(), &resume_id)
            .unwrap()
            .unwrap();
        assert_eq!(doc.header.tagline, Some("Site Reliability Engineer".into()));

        // Set tagline_override — should take priority
        forge
            .conn()
            .execute(
                "UPDATE resumes SET tagline_override = 'Custom Tagline' WHERE id = ?1",
                params![resume_id],
            )
            .unwrap();
        let doc = CompilerService::compile(forge.conn(), &resume_id)
            .unwrap()
            .unwrap();
        assert_eq!(doc.header.tagline, Some("Custom Tagline".into()));
    }

    #[test]
    fn compile_with_summary_override() {
        let forge = setup();
        create_profile(forge.conn());
        let resume_id = create_resume(forge.conn());

        forge
            .conn()
            .execute(
                "UPDATE resumes SET summary_override = 'Custom summary text' WHERE id = ?1",
                params![resume_id],
            )
            .unwrap();

        let doc = CompilerService::compile(forge.conn(), &resume_id)
            .unwrap()
            .unwrap();
        assert!(doc.summary.is_some());
        let summary = doc.summary.unwrap();
        assert_eq!(summary.content, "Custom summary text");
        assert!(summary.is_override);

        // Synthetic summary section should be prepended
        assert_eq!(doc.sections.len(), 1);
        assert_eq!(doc.sections[0].section_type, IRSectionType::Summary);
    }

    #[test]
    fn compile_experience_section() {
        let forge = setup();
        create_profile(forge.conn());
        let resume_id = create_resume(forge.conn());

        // Create experience section with a simple entry
        let section = ResumeStore::create_section(
            forge.conn(),
            &resume_id,
            "Experience",
            "experience",
            Some(0),
        )
        .unwrap();

        ResumeStore::add_entry(
            forge.conn(),
            &resume_id,
            &AddResumeEntry {
                section_id: section.id.clone(),
                perspective_id: None,
                source_id: None,
                position: None,
                content: Some("Led infrastructure migration".into()),
            },
        )
        .unwrap();

        let doc = CompilerService::compile(forge.conn(), &resume_id)
            .unwrap()
            .unwrap();
        // Find the experience section (not the summary)
        let exp_section = doc
            .sections
            .iter()
            .find(|s| s.section_type == IRSectionType::Experience)
            .unwrap();
        assert_eq!(exp_section.title, "Experience");
        assert!(!exp_section.items.is_empty());
    }

    #[test]
    fn compile_skills_section() {
        let forge = setup();
        create_profile(forge.conn());
        let resume_id = create_resume(forge.conn());

        let section =
            ResumeStore::create_section(forge.conn(), &resume_id, "Skills", "skills", Some(0))
                .unwrap();

        let rust = SkillStore::create(forge.conn(), "Rust", Some(SkillCategory::Language)).unwrap();
        let go = SkillStore::create(forge.conn(), "Go", Some(SkillCategory::Language)).unwrap();
        let docker = SkillStore::create(forge.conn(), "Docker", Some(SkillCategory::Tool)).unwrap();

        ResumeStore::add_skill(forge.conn(), &resume_id, &section.id, &rust.id).unwrap();
        ResumeStore::add_skill(forge.conn(), &resume_id, &section.id, &go.id).unwrap();
        ResumeStore::add_skill(forge.conn(), &resume_id, &section.id, &docker.id).unwrap();

        let doc = CompilerService::compile(forge.conn(), &resume_id)
            .unwrap()
            .unwrap();
        let skills_section = doc
            .sections
            .iter()
            .find(|s| s.section_type == IRSectionType::Skills)
            .unwrap();
        assert_eq!(skills_section.items.len(), 1);

        if let IRSectionItem::SkillGroup(group) = &skills_section.items[0] {
            assert!(group.categories.len() >= 1);
            // Check that skills are grouped by category
            let total_skills: usize = group.categories.iter().map(|c| c.skills.len()).sum();
            assert_eq!(total_skills, 3);
        } else {
            panic!("Expected SkillGroup");
        }
    }

    #[test]
    fn compile_certifications_section() {
        let forge = setup();
        create_profile(forge.conn());
        let resume_id = create_resume(forge.conn());

        let section = ResumeStore::create_section(
            forge.conn(),
            &resume_id,
            "Certifications",
            "certifications",
            Some(0),
        )
        .unwrap();

        // Create org + cert
        let org_id = forge_core::new_id();
        let now = forge_core::now_iso();
        forge.conn().execute(
            "INSERT INTO organizations (id, name, org_type, worked, created_at, updated_at) VALUES (?1, 'AWS', 'company', 0, ?2, ?2)",
            params![org_id, now],
        ).unwrap();

        let cert_id = forge_core::new_id();
        forge.conn().execute(
            "INSERT INTO certifications (id, short_name, long_name, issuer_id, created_at, updated_at) VALUES (?1, 'SAA', 'Solutions Architect Associate', ?2, ?3, ?3)",
            params![cert_id, org_id, now],
        ).unwrap();

        ResumeStore::add_certification(
            forge.conn(),
            &resume_id,
            &forge_core::AddResumeCertification {
                certification_id: cert_id,
                section_id: section.id.clone(),
                position: None,
            },
        )
        .unwrap();

        let doc = CompilerService::compile(forge.conn(), &resume_id)
            .unwrap()
            .unwrap();
        let cert_section = doc
            .sections
            .iter()
            .find(|s| s.section_type == IRSectionType::Certifications)
            .unwrap();

        if let IRSectionItem::CertificationGroup(group) = &cert_section.items[0] {
            assert_eq!(group.categories.len(), 1);
            assert_eq!(group.categories[0].label, "AWS");
            assert_eq!(group.categories[0].certs[0].name, "SAA");
        } else {
            panic!("Expected CertificationGroup");
        }
    }

    #[test]
    fn render_markdown_basic() {
        let doc = ResumeDocument {
            resume_id: "test".into(),
            header: ResumeHeader {
                name: "Adam Smith".into(),
                tagline: Some("SRE".into()),
                location: Some("NYC".into()),
                email: Some("adam@example.com".into()),
                phone: None,
                linkedin: None,
                github: None,
                website: None,
                clearance: None,
            },
            summary: None,
            sections: vec![],
        };

        let md = CompilerService::render_markdown(&doc);
        assert!(md.contains("# Adam Smith"));
        assert!(md.contains("*SRE*"));
        assert!(md.contains("adam@example.com"));
    }

    #[test]
    fn render_latex_basic() {
        let doc = ResumeDocument {
            resume_id: "test".into(),
            header: ResumeHeader {
                name: "Adam Smith".into(),
                tagline: Some("SRE".into()),
                location: None,
                email: Some("adam@example.com".into()),
                phone: None,
                linkedin: None,
                github: None,
                website: None,
                clearance: None,
            },
            summary: None,
            sections: vec![],
        };

        let tex = CompilerService::render_latex(&doc);
        assert!(tex.contains("\\documentclass"));
        assert!(tex.contains("Adam Smith"));
        assert!(tex.contains("\\end{document}"));
    }

    #[test]
    fn latex_escape_special_chars() {
        assert_eq!(latex_escape("foo & bar"), "foo \\& bar");
        assert_eq!(latex_escape("100%"), "100\\%");
        assert_eq!(latex_escape("$10"), "\\$10");
        assert_eq!(latex_escape("C#"), "C\\#");
    }

    #[test]
    fn format_date_range_variants() {
        assert_eq!(
            format_date_range(Some("2024-03-01"), Some("2025-07-15"), 0),
            "Mar 2024 - Jul 2025"
        );
        assert_eq!(
            format_date_range(Some("2024-03-01"), None, 1),
            "Mar 2024 - Present"
        );
        assert_eq!(format_date_range(None, None, 0), "");
    }

    #[test]
    fn build_org_display_city_and_state() {
        assert_eq!(
            build_org_display_string(
                Some("Raytheon"),
                Some("Arlington"),
                Some("VA"),
                Some("hybrid")
            ),
            "Raytheon (Arlington, VA)"
        );
    }

    #[test]
    fn build_org_display_city_only() {
        assert_eq!(
            build_org_display_string(Some("Acme"), Some("Austin"), None, None),
            "Acme (Austin)"
        );
    }

    #[test]
    fn build_org_display_state_only() {
        assert_eq!(
            build_org_display_string(Some("Acme"), None, Some("TX"), None),
            "Acme (TX)"
        );
    }

    #[test]
    fn build_org_display_work_arrangement_fallback() {
        assert_eq!(
            build_org_display_string(Some("Cisco"), None, None, Some("remote")),
            "Cisco (Remote)"
        );
    }

    #[test]
    fn build_org_display_no_location_or_arrangement() {
        assert_eq!(
            build_org_display_string(Some("SomeCo"), None, None, None),
            "SomeCo"
        );
    }

    #[test]
    fn build_org_display_null_org_name() {
        assert_eq!(
            build_org_display_string(None, None, None, Some("hybrid")),
            "Other (Hybrid)"
        );
    }

    #[test]
    fn build_org_display_location_wins_over_arrangement() {
        assert_eq!(
            build_org_display_string(
                Some("Cisco"),
                Some("San Jose"),
                Some("CA"),
                Some("contract")
            ),
            "Cisco (San Jose, CA)"
        );
    }
}
