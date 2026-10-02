//! Resume template store — CRUD for the `resume_templates` table, plus
//! creating a resume pre-populated from a template.
//!
//! Port of `packages/core/src/services/template-service.ts`: the validation
//! rules live here because the Rust service layer for templates does not exist.
//! `sections` is stored as a JSON array in a TEXT column.

use rusqlite::{params, Connection, OptionalExtension};

use forge_core::{
    new_id, now_iso, CreateResume, CreateResumeTemplate, ForgeError, Resume, ResumeTemplate,
    TemplateSectionDef, UpdateResumeTemplate,
};

use super::resume::ResumeStore;

/// Valid `entry_type` values — mirrors the `resume_sections` CHECK constraint.
const VALID_ENTRY_TYPES: &[&str] = &[
    "experience",
    "skills",
    "education",
    "projects",
    "clearance",
    "presentations",
    "certifications",
    "awards",
    "freeform",
];

/// Data-access store for the `resume_templates` table.
pub struct TemplateStore;

fn validation(message: impl Into<String>, field: Option<&str>) -> ForgeError {
    ForgeError::Validation { message: message.into(), field: field.map(str::to_string) }
}

fn not_found(id: &str) -> ForgeError {
    ForgeError::NotFound { entity_type: "template".into(), id: id.into() }
}

impl TemplateStore {
    // ── Read ─────────────────────────────────────────────────────────

    /// All templates: built-ins first, then by name.
    pub fn list(conn: &Connection) -> Result<Vec<ResumeTemplate>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT id, name, description, sections, is_builtin, created_at, updated_at
             FROM resume_templates
             ORDER BY is_builtin DESC, name ASC",
        )?;
        let rows = stmt
            .query_map([], Self::map_template)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Fetch one template by id.
    pub fn get(conn: &Connection, id: &str) -> Result<Option<ResumeTemplate>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT id, name, description, sections, is_builtin, created_at, updated_at
             FROM resume_templates WHERE id = ?1",
        )?;
        Ok(stmt.query_row(params![id], Self::map_template).optional()?)
    }

    // ── Create ───────────────────────────────────────────────────────

    /// Create a user template. Positions are normalised to 0..n in input order.
    pub fn create(
        conn: &Connection,
        input: &CreateResumeTemplate,
    ) -> Result<ResumeTemplate, ForgeError> {
        Self::validate(&input.name, &input.sections)?;

        let id = new_id();
        let now = now_iso();
        let sections = Self::normalize(&input.sections);
        conn.execute(
            "INSERT INTO resume_templates (id, name, description, sections, is_builtin, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 0, ?5, ?5)",
            params![
                id,
                input.name.trim(),
                input.description,
                serde_json::to_string(&sections).map_err(|e| ForgeError::Internal(e.to_string()))?,
                now
            ],
        )?;

        Self::get(conn, &id)?
            .ok_or_else(|| ForgeError::Internal("Template created but not found".into()))
    }

    /// Create a resume and one section per template section, atomically.
    pub fn create_resume_from_template(
        conn: &Connection,
        template_id: &str,
        input: &CreateResume,
    ) -> Result<Resume, ForgeError> {
        let template = Self::get(conn, template_id)?.ok_or_else(|| not_found(template_id))?;

        for (value, label, field) in [
            (&input.name, "Name", "name"),
            (&input.target_role, "Target role", "target_role"),
            (&input.target_employer, "Target employer", "target_employer"),
            (&input.archetype, "Archetype", "archetype"),
        ] {
            if value.trim().is_empty() {
                return Err(validation(format!("{label} must not be empty"), Some(field)));
            }
        }

        let tx = conn.unchecked_transaction()?;
        let resume = ResumeStore::create(&tx, input)?;
        for section in &template.sections {
            ResumeStore::create_section(
                &tx,
                &resume.id,
                &section.title,
                &section.entry_type,
                Some(section.position),
            )?;
        }
        tx.commit()?;

        ResumeStore::get(conn, &resume.id)?
            .ok_or_else(|| ForgeError::Internal("Resume created but not found".into()))
    }

    // ── Update ───────────────────────────────────────────────────────

    /// Patch a template's name, description and/or sections.
    pub fn update(
        conn: &Connection,
        id: &str,
        patch: &UpdateResumeTemplate,
    ) -> Result<ResumeTemplate, ForgeError> {
        let existing = Self::get(conn, id)?.ok_or_else(|| not_found(id))?;

        if let Some(name) = &patch.name {
            if name.trim().is_empty() {
                return Err(validation("Name must not be empty", Some("name")));
            }
        }
        let sections = match &patch.sections {
            Some(sections) => {
                let name = patch.name.as_deref().unwrap_or(&existing.name);
                Self::validate(name, sections)?;
                Some(Self::normalize(sections))
            }
            None => None,
        };

        let name = patch.name.as_deref().map(str::trim).unwrap_or(&existing.name);
        let description = match &patch.description {
            Some(d) => d.clone(),
            None => existing.description.clone(),
        };
        let sections = sections.unwrap_or_else(|| existing.sections.clone());

        conn.execute(
            "UPDATE resume_templates
                SET name = ?1, description = ?2, sections = ?3, updated_at = ?4
              WHERE id = ?5",
            params![
                name,
                description,
                serde_json::to_string(&sections).map_err(|e| ForgeError::Internal(e.to_string()))?,
                now_iso(),
                id
            ],
        )?;

        Self::get(conn, id)?.ok_or_else(|| not_found(id))
    }

    // ── Delete ───────────────────────────────────────────────────────

    /// Delete a user template. Built-in templates cannot be deleted.
    pub fn delete(conn: &Connection, id: &str) -> Result<(), ForgeError> {
        let existing = Self::get(conn, id)?.ok_or_else(|| not_found(id))?;
        if existing.is_builtin == 1 {
            return Err(validation("Built-in templates cannot be deleted", None));
        }
        conn.execute("DELETE FROM resume_templates WHERE id = ?1", params![id])?;
        Ok(())
    }

    // ── Helpers ──────────────────────────────────────────────────────

    fn validate(name: &str, sections: &[TemplateSectionDef]) -> Result<(), ForgeError> {
        if name.trim().is_empty() {
            return Err(validation("Name must not be empty", Some("name")));
        }
        if sections.is_empty() {
            return Err(validation("Sections must be a non-empty array", Some("sections")));
        }
        for (i, s) in sections.iter().enumerate() {
            if s.title.trim().is_empty() {
                return Err(validation(format!("Section {i}: title must not be empty"), Some("sections")));
            }
            if !VALID_ENTRY_TYPES.contains(&s.entry_type.as_str()) {
                return Err(validation(
                    format!(
                        "Section {i}: invalid entry_type '{}'. Must be one of: {}",
                        s.entry_type,
                        VALID_ENTRY_TYPES.join(", ")
                    ),
                    Some("sections"),
                ));
            }
        }
        Ok(())
    }

    /// Titles trimmed, positions sequential from 0 in input order.
    fn normalize(sections: &[TemplateSectionDef]) -> Vec<TemplateSectionDef> {
        sections
            .iter()
            .enumerate()
            .map(|(i, s)| TemplateSectionDef {
                title: s.title.trim().to_string(),
                entry_type: s.entry_type.clone(),
                position: i as i32,
            })
            .collect()
    }

    fn map_template(row: &rusqlite::Row) -> rusqlite::Result<ResumeTemplate> {
        let sections_json: String = row.get(3)?;
        let sections: Vec<TemplateSectionDef> = serde_json::from_str(&sections_json).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, Box::new(e))
        })?;
        Ok(ResumeTemplate {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            sections,
            is_builtin: row.get(4)?,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::Forge;

    fn section(title: &str, entry_type: &str) -> TemplateSectionDef {
        TemplateSectionDef { title: title.into(), entry_type: entry_type.into(), position: 99 }
    }

    fn new_template(name: &str) -> CreateResumeTemplate {
        CreateResumeTemplate {
            name: name.into(),
            description: Some("d".into()),
            sections: vec![section(" Summary ", "freeform"), section("Work", "experience")],
        }
    }

    fn resume_input() -> CreateResume {
        CreateResume {
            name: "R".into(),
            target_role: "Engineer".into(),
            target_employer: "Acme".into(),
            archetype: "backend".into(),
            summary_id: None,
        }
    }

    #[test]
    fn list_returns_seeded_builtins_first() {
        let forge = Forge::open_memory().unwrap();
        TemplateStore::create(forge.conn(), &new_template("AAA user")).unwrap();
        let all = TemplateStore::list(forge.conn()).unwrap();
        assert!(all.iter().filter(|t| t.is_builtin == 1).count() >= 3);
        let first_user = all.iter().position(|t| t.is_builtin == 0).unwrap();
        assert!(all[..first_user].iter().all(|t| t.is_builtin == 1), "built-ins sort first");
    }

    #[test]
    fn create_normalizes_titles_and_positions() {
        let forge = Forge::open_memory().unwrap();
        let t = TemplateStore::create(forge.conn(), &new_template("  Mine ")).unwrap();
        assert_eq!(t.name, "Mine");
        assert_eq!(t.is_builtin, 0);
        assert_eq!(t.sections[0].title, "Summary");
        assert_eq!(t.sections.iter().map(|s| s.position).collect::<Vec<_>>(), vec![0, 1]);
    }

    #[test]
    fn create_rejects_bad_input() {
        let forge = Forge::open_memory().unwrap();
        let mut empty_name = new_template(" ");
        assert!(matches!(
            TemplateStore::create(forge.conn(), &empty_name),
            Err(ForgeError::Validation { .. })
        ));
        empty_name.name = "ok".into();
        empty_name.sections = vec![];
        assert!(matches!(
            TemplateStore::create(forge.conn(), &empty_name),
            Err(ForgeError::Validation { .. })
        ));
        let bad_type = CreateResumeTemplate {
            name: "ok".into(),
            description: None,
            sections: vec![section("X", "nonsense")],
        };
        assert!(matches!(
            TemplateStore::create(forge.conn(), &bad_type),
            Err(ForgeError::Validation { .. })
        ));
    }

    #[test]
    fn update_patches_only_given_fields() {
        let forge = Forge::open_memory().unwrap();
        let t = TemplateStore::create(forge.conn(), &new_template("Orig")).unwrap();
        let patched = TemplateStore::update(
            forge.conn(),
            &t.id,
            &UpdateResumeTemplate { name: Some("New".into()), ..Default::default() },
        )
        .unwrap();
        assert_eq!(patched.name, "New");
        assert_eq!(patched.description, t.description);
        assert_eq!(patched.sections.len(), t.sections.len());
    }

    #[test]
    fn update_missing_is_not_found() {
        let forge = Forge::open_memory().unwrap();
        let r = TemplateStore::update(forge.conn(), "nope", &UpdateResumeTemplate::default());
        assert!(matches!(r, Err(ForgeError::NotFound { .. })));
    }

    #[test]
    fn builtin_cannot_be_deleted_but_user_template_can() {
        let forge = Forge::open_memory().unwrap();
        let builtin = TemplateStore::list(forge.conn()).unwrap().into_iter().find(|t| t.is_builtin == 1).unwrap();
        assert!(matches!(
            TemplateStore::delete(forge.conn(), &builtin.id),
            Err(ForgeError::Validation { .. })
        ));
        let mine = TemplateStore::create(forge.conn(), &new_template("Mine")).unwrap();
        TemplateStore::delete(forge.conn(), &mine.id).unwrap();
        assert!(TemplateStore::get(forge.conn(), &mine.id).unwrap().is_none());
    }

    #[test]
    fn create_resume_from_template_makes_sections() {
        let forge = Forge::open_memory().unwrap();
        let t = TemplateStore::create(forge.conn(), &new_template("T")).unwrap();
        let resume = TemplateStore::create_resume_from_template(forge.conn(), &t.id, &resume_input()).unwrap();
        let sections = ResumeStore::list_sections(forge.conn(), &resume.id).unwrap();
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].title, "Summary");
        assert_eq!(sections[1].entry_type, "experience");
    }

    #[test]
    fn create_resume_from_template_validates_and_rolls_back() {
        let forge = Forge::open_memory().unwrap();
        let t = TemplateStore::create(forge.conn(), &new_template("T")).unwrap();
        let mut bad = resume_input();
        bad.archetype = "  ".into();
        assert!(matches!(
            TemplateStore::create_resume_from_template(forge.conn(), &t.id, &bad),
            Err(ForgeError::Validation { .. })
        ));
        assert!(matches!(
            TemplateStore::create_resume_from_template(forge.conn(), "missing", &resume_input()),
            Err(ForgeError::NotFound { .. })
        ));
        let count: i64 = forge.conn().query_row("SELECT count(*) FROM resumes", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 0, "no resume left behind by a failed create");
    }
}
