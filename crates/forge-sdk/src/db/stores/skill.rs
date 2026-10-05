//! Skill repository — data access layer for skills and the `skill_domains`
//! junction table.

use rusqlite::{params, Connection, OptionalExtension};

use forge_core::{new_id, Domain, ForgeError, Skill, SkillCategory, SkillRow, SkillWithDomains};

/// Capitalize first character only, preserving the rest (e.g. "SAFe" stays "SAFe").
fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
    }
}

/// Data access for the `skills` and `skill_domains` tables.
///
/// Skills have a `UNIQUE(name)` constraint (case-insensitive at the app layer)
/// and a `category` column that is an FK to `skill_categories.slug`.
pub struct SkillStore;

impl SkillStore {
    // ── Skill CRUD ──────────────────────────────────────────────────

    /// Insert a new skill row. The `name` is capitalized on the first
    /// character (preserving the rest, e.g. "SAFe" stays "SAFe").
    /// Returns `CONFLICT` if a skill with the same name already exists.
    pub fn create(
        conn: &Connection,
        name: &str,
        category: Option<SkillCategory>,
    ) -> Result<Skill, ForgeError> {
        let id = new_id();
        let cat = category.unwrap_or(SkillCategory::Other);
        let capitalized = capitalize_first(name.trim());

        conn.execute(
            "INSERT INTO skills (id, name, category) VALUES (?1, ?2, ?3)",
            params![id, capitalized, cat.as_ref()],
        )
        .map_err(|e| {
            if let rusqlite::Error::SqliteFailure(ref err, ref msg) = e {
                if err.code == rusqlite::ErrorCode::ConstraintViolation {
                    // UNIQUE constraint on skills.name
                    if msg.as_deref().unwrap_or("").contains("UNIQUE") {
                        return ForgeError::Conflict {
                            message: format!("Skill with name '{}' already exists", capitalized),
                        };
                    }
                }
            }
            ForgeError::Database { source: e }
        })?;

        Self::get(conn, &id)?
            .ok_or_else(|| ForgeError::Internal("Skill created but not found".into()))
    }

    /// Fetch a single skill by ID.
    pub fn get(conn: &Connection, id: &str) -> Result<Option<Skill>, ForgeError> {
        let mut stmt = conn.prepare("SELECT id, name, category FROM skills WHERE id = ?1")?;
        let result = stmt.query_row(params![id], Self::map_skill).optional()?;
        Ok(result)
    }

    /// Fetch a skill with its linked domains populated.
    pub fn get_with_domains(
        conn: &Connection,
        id: &str,
    ) -> Result<Option<SkillWithDomains>, ForgeError> {
        let skill = match Self::get(conn, id)? {
            Some(s) => s,
            None => return Ok(None),
        };
        let domains = Self::fetch_domains_for(conn, &skill.id)?;
        Ok(Some(SkillWithDomains {
            base: skill,
            domains,
        }))
    }

    /// List all skills, optionally filtered by category and/or domain_id.
    /// When `domain_id` is provided, walks the `skill_domains` junction.
    /// Supports text search (case-insensitive substring on `name`).
    /// Results are sorted by `name ASC`.
    pub fn list(
        conn: &Connection,
        category: Option<SkillCategory>,
        domain_id: Option<&str>,
        search: Option<&str>,
    ) -> Result<Vec<Skill>, ForgeError> {
        // If filtering by domain, get skill IDs from the junction first
        let domain_skill_ids: Option<Vec<String>> = if let Some(did) = domain_id {
            let mut stmt =
                conn.prepare("SELECT skill_id FROM skill_domains WHERE domain_id = ?1")?;
            let ids: Vec<String> = stmt
                .query_map(params![did], |row| row.get(0))?
                .collect::<Result<_, _>>()?;
            Some(ids)
        } else {
            None
        };

        let mut conditions = Vec::new();
        let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(ref cat) = category {
            conditions.push(format!("category = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(cat.to_string()));
        }

        if let Some(ref s) = search {
            let param_idx = bind_values.len() + 1;
            conditions.push(format!("name LIKE ?{param_idx}"));
            bind_values.push(Box::new(format!("%{s}%")));
        }

        // If domain filter yielded IDs, add an IN clause
        if let Some(ref ids) = domain_skill_ids {
            if ids.is_empty() {
                return Ok(Vec::new());
            }
            let placeholders: Vec<String> = ids
                .iter()
                .enumerate()
                .map(|(i, _)| format!("?{}", bind_values.len() + 1 + i))
                .collect();
            conditions.push(format!("id IN ({})", placeholders.join(", ")));
            for id in ids {
                bind_values.push(Box::new(id.clone()));
            }
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let sql = format!("SELECT id, name, category FROM skills {where_clause} ORDER BY name ASC");

        let mut stmt = conn.prepare(&sql)?;
        let skills: Vec<Skill> = stmt
            .query_map(
                rusqlite::params_from_iter(bind_values.iter().map(|b| b.as_ref())),
                Self::map_skill,
            )?
            .collect::<Result<_, _>>()?;

        Ok(skills)
    }

    /// Partially update a skill's `name` and/or `category`.
    pub fn update(
        conn: &Connection,
        id: &str,
        name: Option<&str>,
        category: Option<SkillCategory>,
    ) -> Result<Skill, ForgeError> {
        // Verify exists
        Self::get(conn, id)?.ok_or_else(|| ForgeError::NotFound {
            entity_type: "skill".into(),
            id: id.into(),
        })?;

        let mut sets = Vec::new();
        let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(n) = name {
            sets.push(format!("name = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(capitalize_first(n.trim())));
        }
        if let Some(ref cat) = category {
            sets.push(format!("category = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(cat.to_string()));
        }

        if !sets.is_empty() {
            let sql = format!(
                "UPDATE skills SET {} WHERE id = ?{}",
                sets.join(", "),
                bind_values.len() + 1
            );
            bind_values.push(Box::new(id.to_string()));

            conn.execute(
                &sql,
                rusqlite::params_from_iter(bind_values.iter().map(|b| b.as_ref())),
            )?;
        }

        Self::get(conn, id)?
            .ok_or_else(|| ForgeError::Internal("Skill updated but not found".into()))
    }

    /// Delete a skill by ID. This cascades through all junction tables.
    pub fn delete(conn: &Connection, id: &str) -> Result<(), ForgeError> {
        let deleted = conn.execute("DELETE FROM skills WHERE id = ?1", params![id])?;
        if deleted == 0 {
            return Err(ForgeError::NotFound {
                entity_type: "skill".into(),
                id: id.into(),
            });
        }
        Ok(())
    }

    // ── Case-insensitive lookup ─────────────────────────────────────

    /// Find a skill by exact name match (case-insensitive). Returns `None`
    /// if no match exists.
    pub fn find_by_name(conn: &Connection, name: &str) -> Result<Option<Skill>, ForgeError> {
        let mut stmt =
            conn.prepare("SELECT id, name, category FROM skills WHERE LOWER(name) = LOWER(?1)")?;
        let result = stmt
            .query_row(params![name.trim()], Self::map_skill)
            .optional()?;
        Ok(result)
    }

    /// Find a skill by name (case-insensitive), creating it if it doesn't
    /// exist. Supports the combobox "select existing or create new" pattern.
    pub fn get_or_create(
        conn: &Connection,
        name: &str,
        category: Option<SkillCategory>,
    ) -> Result<Skill, ForgeError> {
        if let Some(existing) = Self::find_by_name(conn, name)? {
            return Ok(existing);
        }
        Self::create(conn, name, category)
    }

    // ── Full rows and link helpers (source_skills / JD skills) ──────

    /// Categories the TS link routes accept (`sources.ts` valid list). Narrower
    /// than the full [`SkillCategory`]; anything else falls back to `Other`.
    pub const LINK_CATEGORIES: [SkillCategory; 10] = [
        SkillCategory::Language,
        SkillCategory::Framework,
        SkillCategory::Platform,
        SkillCategory::Tool,
        SkillCategory::Library,
        SkillCategory::Methodology,
        SkillCategory::Protocol,
        SkillCategory::Concept,
        SkillCategory::SoftSkill,
        SkillCategory::Other,
    ];

    /// Fetch a full `skills` row (including `created_at`) by id.
    pub fn get_row(conn: &Connection, id: &str) -> Result<Option<SkillRow>, ForgeError> {
        let row = conn
            .query_row(
                "SELECT id, name, category, created_at FROM skills WHERE id = ?1",
                params![id],
                Self::map_skill_row,
            )
            .optional()?;
        Ok(row)
    }

    /// Resolve a skill for a junction link using the TS name rules: trim and
    /// capitalise the first character, reuse a case-insensitive match (its
    /// stored category wins), otherwise create it. `category` must be in
    /// [`Self::LINK_CATEGORIES`]; unknown, missing or out-of-list values become
    /// `Other`. An empty name is a validation error.
    pub fn get_or_create_for_link(
        conn: &Connection,
        name: &str,
        category: Option<&str>,
    ) -> Result<SkillRow, ForgeError> {
        let name = capitalize_first(name.trim());
        if name.is_empty() {
            return Err(ForgeError::Validation {
                message: "skill_id or name is required".into(),
                field: None,
            });
        }
        let category = category
            .and_then(|c| c.parse::<SkillCategory>().ok())
            .filter(|c| Self::LINK_CATEGORIES.contains(c))
            .unwrap_or(SkillCategory::Other);
        let skill = Self::get_or_create(conn, &name, Some(category))?;
        Self::get_row(conn, &skill.id)?
            .ok_or_else(|| ForgeError::Internal("Skill resolved but not found".into()))
    }

    // ── Skill <-> Domain junction ───────────────────────────────────

    /// Link a skill to a domain. Idempotent — if the pair already exists,
    /// returns `Ok(())` without error. Returns `NOT_FOUND` if the skill
    /// or domain does not exist.
    pub fn add_domain(
        conn: &Connection,
        skill_id: &str,
        domain_id: &str,
    ) -> Result<(), ForgeError> {
        // Verify skill exists
        Self::get(conn, skill_id)?.ok_or_else(|| ForgeError::NotFound {
            entity_type: "skill".into(),
            id: skill_id.into(),
        })?;

        conn.execute(
            "INSERT OR IGNORE INTO skill_domains (skill_id, domain_id) VALUES (?1, ?2)",
            params![skill_id, domain_id],
        )
        .map_err(|e| {
            if let rusqlite::Error::SqliteFailure(ref err, ref msg) = e {
                if err.code == rusqlite::ErrorCode::ConstraintViolation {
                    if msg.as_deref().unwrap_or("").contains("FOREIGN KEY") {
                        return ForgeError::NotFound {
                            entity_type: "domain".into(),
                            id: domain_id.into(),
                        };
                    }
                }
            }
            ForgeError::Database { source: e }
        })?;

        Ok(())
    }

    /// Unlink a skill from a domain. Silent if the pair does not exist.
    pub fn remove_domain(
        conn: &Connection,
        skill_id: &str,
        domain_id: &str,
    ) -> Result<(), ForgeError> {
        conn.execute(
            "DELETE FROM skill_domains WHERE skill_id = ?1 AND domain_id = ?2",
            params![skill_id, domain_id],
        )?;
        Ok(())
    }

    /// Get all domains linked to a skill. Returns `NOT_FOUND` if the
    /// skill does not exist.
    pub fn get_domains(conn: &Connection, skill_id: &str) -> Result<Vec<Domain>, ForgeError> {
        Self::get(conn, skill_id)?.ok_or_else(|| ForgeError::NotFound {
            entity_type: "skill".into(),
            id: skill_id.into(),
        })?;

        Self::fetch_domains_for(conn, skill_id)
    }

    // ── Merge ───────────────────────────────────────────────────────

    /// Atomically merge `source_id` into `target_id`: re-point all junction
    /// rows from the source skill to the target, handle UNIQUE constraint
    /// conflicts by deleting the conflicting source rows, then delete the
    /// source skill.
    ///
    /// Returns the surviving target skill after merge.
    pub fn merge(conn: &Connection, source_id: &str, target_id: &str) -> Result<Skill, ForgeError> {
        if source_id == target_id {
            return Err(ForgeError::Validation {
                message: "Cannot merge a skill into itself".into(),
                field: None,
            });
        }

        // Verify both exist
        Self::get(conn, source_id)?.ok_or_else(|| ForgeError::NotFound {
            entity_type: "skill".into(),
            id: source_id.into(),
        })?;
        Self::get(conn, target_id)?.ok_or_else(|| ForgeError::NotFound {
            entity_type: "skill".into(),
            id: target_id.into(),
        })?;

        // Junction tables to migrate: (table_name, parent_field)
        let junctions = [
            ("bullet_skills", "bullet_id"),
            ("resume_skills", "section_id"),
            ("certification_skills", "certification_id"),
            ("job_description_skills", "job_description_id"),
            ("perspective_skills", "perspective_id"),
            ("source_skills", "source_id"),
            ("skill_domains", "domain_id"),
            ("summary_skills", "summary_id"),
        ];

        for (table, parent_field) in &junctions {
            // Delete conflicting rows (where target already has a link to the same parent)
            let delete_sql = format!(
                "DELETE FROM {table} WHERE skill_id = ?1 AND {parent_field} IN (
                    SELECT {parent_field} FROM {table} WHERE skill_id = ?2
                )"
            );
            conn.execute(&delete_sql, params![source_id, target_id])?;

            // Remap remaining source rows to target
            let update_sql = format!("UPDATE {table} SET skill_id = ?1 WHERE skill_id = ?2");
            conn.execute(&update_sql, params![target_id, source_id])?;
        }

        // Delete the source skill
        conn.execute("DELETE FROM skills WHERE id = ?1", params![source_id])?;

        Self::get(conn, target_id)?
            .ok_or_else(|| ForgeError::Internal("Target skill not found after merge".into()))
    }

    // ── Internal helpers ────────────────────────────────────────────

    fn fetch_domains_for(conn: &Connection, skill_id: &str) -> Result<Vec<Domain>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT d.id, d.name, d.description, d.created_at
             FROM domains d
             INNER JOIN skill_domains sd ON sd.domain_id = d.id
             WHERE sd.skill_id = ?1
             ORDER BY d.name ASC",
        )?;
        let domains: Vec<Domain> = stmt
            .query_map(params![skill_id], |row| {
                Ok(Domain {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        Ok(domains)
    }

    // ── Row mapping ─────────────────────────────────────────────────

    /// Map `(id, name, category, created_at)` to a [`SkillRow`].
    pub(crate) fn map_skill_row(row: &rusqlite::Row) -> rusqlite::Result<SkillRow> {
        Ok(SkillRow {
            base: Self::map_skill(row)?,
            created_at: row.get(3)?,
        })
    }

    fn map_skill(row: &rusqlite::Row) -> rusqlite::Result<Skill> {
        Ok(Skill {
            id: row.get(0)?,
            name: row.get(1)?,
            category: row
                .get::<_, String>(2)?
                .parse()
                .unwrap_or(SkillCategory::Other),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::Forge;

    fn setup() -> Forge {
        Forge::open_memory().unwrap()
    }

    #[test]
    fn get_or_create_for_link_applies_ts_rules() {
        let forge = setup();
        let conn = forge.conn();

        let k = SkillStore::get_or_create_for_link(conn, "  kubernetes ", None).unwrap();
        assert_eq!(k.base.name, "Kubernetes");
        assert_eq!(k.base.category, SkillCategory::Other);
        assert!(!k.created_at.is_empty());

        assert_eq!(
            SkillStore::get_or_create_for_link(conn, "sAFe", None)
                .unwrap()
                .base
                .name,
            "SAFe"
        );

        let t = SkillStore::get_or_create_for_link(conn, "Terraform", Some("tool")).unwrap();
        assert_eq!(t.base.category, SkillCategory::Tool);

        let v = SkillStore::get_or_create_for_link(conn, "Vector search", Some("ai_ml")).unwrap();
        assert_eq!(v.base.category, SkillCategory::Other);

        let b = SkillStore::get_or_create_for_link(conn, "Bogus cat", Some("bogus")).unwrap();
        assert_eq!(b.base.category, SkillCategory::Other);

        assert!(matches!(
            SkillStore::get_or_create_for_link(conn, "   ", None),
            Err(ForgeError::Validation { .. })
        ));

        let py = SkillStore::create(conn, "Python", Some(SkillCategory::Language)).unwrap();
        let reused = SkillStore::get_or_create_for_link(conn, "python", Some("tool")).unwrap();
        assert_eq!(reused.base.id, py.id);
        assert_eq!(reused.base.category, SkillCategory::Language);
    }

    #[test]
    fn skill_row_serialises_to_four_keys() {
        let forge = setup();
        let s = SkillStore::get_or_create_for_link(forge.conn(), "Rust", Some("language")).unwrap();
        let v = serde_json::to_value(&s).unwrap();
        let mut keys: Vec<_> = v.as_object().unwrap().keys().cloned().collect();
        keys.sort();
        assert_eq!(keys, ["category", "created_at", "id", "name"]);
    }

    #[test]
    fn create_and_get_skill() {
        let forge = setup();
        let skill =
            SkillStore::create(forge.conn(), "typescript", Some(SkillCategory::Language)).unwrap();
        assert_eq!(skill.name, "Typescript"); // first char capitalized
        assert_eq!(skill.category, SkillCategory::Language);

        let fetched = SkillStore::get(forge.conn(), &skill.id).unwrap().unwrap();
        assert_eq!(fetched.id, skill.id);
        assert_eq!(fetched.name, "Typescript");
    }

    #[test]
    fn create_preserves_case_after_first_char() {
        let forge = setup();
        let skill = SkillStore::create(forge.conn(), "SAFe", None).unwrap();
        assert_eq!(skill.name, "SAFe");
    }

    #[test]
    fn get_returns_none_for_missing() {
        let forge = setup();
        let result = SkillStore::get(forge.conn(), "nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn list_empty() {
        let forge = setup();
        let skills = SkillStore::list(forge.conn(), None, None, None).unwrap();
        // May contain seed data from migrations, but at minimum should not error
        let _ = skills;
    }

    #[test]
    fn list_with_category_filter() {
        let forge = setup();
        SkillStore::create(forge.conn(), "Rust", Some(SkillCategory::Language)).unwrap();
        SkillStore::create(forge.conn(), "Docker", Some(SkillCategory::Tool)).unwrap();

        let langs =
            SkillStore::list(forge.conn(), Some(SkillCategory::Language), None, None).unwrap();
        assert!(langs.iter().any(|s| s.name == "Rust"));
        assert!(!langs.iter().any(|s| s.name == "Docker"));
    }

    #[test]
    fn list_with_search() {
        let forge = setup();
        SkillStore::create(forge.conn(), "Kubernetes", Some(SkillCategory::Tool)).unwrap();
        SkillStore::create(forge.conn(), "Python", Some(SkillCategory::Language)).unwrap();

        let results = SkillStore::list(forge.conn(), None, None, Some("kube")).unwrap();
        assert!(results.iter().any(|s| s.name == "Kubernetes"));
        assert!(!results.iter().any(|s| s.name == "Python"));
    }

    #[test]
    fn find_by_name_case_insensitive() {
        let forge = setup();
        SkillStore::create(forge.conn(), "TypeScript", Some(SkillCategory::Language)).unwrap();

        let found = SkillStore::find_by_name(forge.conn(), "typescript").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "TypeScript");
    }

    #[test]
    fn get_or_create_returns_existing() {
        let forge = setup();
        let created =
            SkillStore::create(forge.conn(), "Go", Some(SkillCategory::Language)).unwrap();
        let got = SkillStore::get_or_create(forge.conn(), "go", Some(SkillCategory::Tool)).unwrap();
        assert_eq!(got.id, created.id);
        assert_eq!(got.category, SkillCategory::Language); // keeps original category
    }

    #[test]
    fn get_or_create_creates_new() {
        let forge = setup();
        let skill =
            SkillStore::get_or_create(forge.conn(), "Elixir", Some(SkillCategory::Language))
                .unwrap();
        assert_eq!(skill.name, "Elixir");
        assert_eq!(skill.category, SkillCategory::Language);
    }

    #[test]
    fn update_skill() {
        let forge = setup();
        let skill =
            SkillStore::create(forge.conn(), "Pythn", Some(SkillCategory::Language)).unwrap();
        let updated = SkillStore::update(forge.conn(), &skill.id, Some("Python"), None).unwrap();
        assert_eq!(updated.name, "Python");
        assert_eq!(updated.category, SkillCategory::Language);
    }

    #[test]
    fn delete_skill() {
        let forge = setup();
        let skill = SkillStore::create(forge.conn(), "ToDelete", None).unwrap();
        SkillStore::delete(forge.conn(), &skill.id).unwrap();
        assert!(SkillStore::get(forge.conn(), &skill.id).unwrap().is_none());
    }

    #[test]
    fn delete_missing_returns_not_found() {
        let forge = setup();
        let result = SkillStore::delete(forge.conn(), "nonexistent");
        assert!(matches!(result, Err(ForgeError::NotFound { .. })));
    }

    #[test]
    fn duplicate_name_returns_conflict() {
        let forge = setup();
        SkillStore::create(forge.conn(), "React", Some(SkillCategory::Framework)).unwrap();
        let result = SkillStore::create(forge.conn(), "React", Some(SkillCategory::Library));
        assert!(matches!(result, Err(ForgeError::Conflict { .. })));
    }
}
