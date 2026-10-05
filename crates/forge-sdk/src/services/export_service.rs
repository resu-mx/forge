//! Export service — data bundle export and database dump.
//!
//! Data export collects entity data into a `DataExportBundle` for backup
//! or migration. Database dump uses rusqlite's iterdump for a full SQL dump
//! without shelling out to external processes.
//!
//! Resume format exports (JSON/Markdown/LaTeX) are deferred to the
//! CompilerService (forge-9fwl), which owns IR compilation and rendering.

use rusqlite::{params, Connection};

use forge_core::{
    now_iso, Bullet, BulletStatus, DataExportBundle, ExportMetadata, ForgeError, Framing, OrgTag,
    Organization, Perspective, PerspectiveStatus, Skill, SkillCategory, Source, SourceStatus,
    SourceType, UpdatedBy,
};

/// Export operations: entity data bundles and raw database dumps.
pub struct ExportService;

impl ExportService {
    /// Export data for the specified entity types into a bundle.
    ///
    /// Supported entity names: `"sources"`, `"bullets"`, `"perspectives"`,
    /// `"skills"`, `"organizations"`.
    pub fn export_data(
        conn: &Connection,
        entities: &[String],
    ) -> Result<DataExportBundle, ForgeError> {
        let now = now_iso();
        let entity_names: Vec<String> = entities.iter().map(|e| e.to_lowercase()).collect();

        let mut bundle = DataExportBundle {
            forge_export: ExportMetadata {
                version: "1.0".into(),
                exported_at: now,
                entities: entity_names.clone(),
            },
            sources: None,
            bullets: None,
            perspectives: None,
            skills: None,
            organizations: None,
            summaries: None,
            job_descriptions: None,
        };

        for entity in &entity_names {
            match entity.as_str() {
                "sources" => {
                    bundle.sources = Some(Self::export_sources(conn)?);
                }
                "bullets" => {
                    bundle.bullets = Some(Self::export_bullets(conn)?);
                }
                "perspectives" => {
                    bundle.perspectives = Some(Self::export_perspectives(conn)?);
                }
                "skills" => {
                    bundle.skills = Some(Self::export_skills(conn)?);
                }
                "organizations" => {
                    bundle.organizations = Some(Self::export_organizations(conn)?);
                }
                // summaries, job_descriptions exported as raw JSON values
                _ => {} // silently ignore unknown entity types
            }
        }

        Ok(bundle)
    }

    /// Produce a full SQL dump of the SQLite database using iterdump.
    ///
    /// Uses rusqlite's native schema + data export without subprocess calls.
    pub fn dump_database(conn: &Connection) -> Result<String, ForgeError> {
        let mut dump = String::new();
        dump.push_str("BEGIN TRANSACTION;\n");

        // Get all table schemas
        let mut schema_stmt = conn.prepare(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name != '_migrations' ORDER BY name",
        )?;
        let schemas: Vec<String> = schema_stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<_, _>>()?;

        for schema in &schemas {
            dump.push_str(schema);
            dump.push_str(";\n");
        }

        // Get all table names and dump data
        let mut table_stmt = conn.prepare(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name != '_migrations' ORDER BY name",
        )?;
        let tables: Vec<String> = table_stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<_, _>>()?;

        for table in &tables {
            // Get column count
            let col_info = conn.prepare(&format!("SELECT * FROM \"{}\" LIMIT 0", table))?;
            let col_count = col_info.column_count();

            let mut data_stmt = conn.prepare(&format!("SELECT * FROM \"{}\"", table))?;
            let mut rows = data_stmt.query([])?;

            while let Some(row) = rows.next()? {
                let mut values = Vec::with_capacity(col_count);
                for i in 0..col_count {
                    let val: rusqlite::types::Value = row.get(i)?;
                    match val {
                        rusqlite::types::Value::Null => values.push("NULL".to_string()),
                        rusqlite::types::Value::Integer(i) => values.push(i.to_string()),
                        rusqlite::types::Value::Real(f) => values.push(f.to_string()),
                        rusqlite::types::Value::Text(s) => {
                            values.push(format!("'{}'", s.replace('\'', "''")));
                        }
                        rusqlite::types::Value::Blob(b) => {
                            let hex: String =
                                b.iter().map(|byte| format!("{:02x}", byte)).collect();
                            values.push(format!("X'{}'", hex));
                        }
                    }
                }
                dump.push_str(&format!(
                    "INSERT INTO \"{}\" VALUES ({});\n",
                    table,
                    values.join(", ")
                ));
            }
        }

        // Get indexes
        let mut idx_stmt = conn.prepare(
            "SELECT sql FROM sqlite_master WHERE type='index' AND sql IS NOT NULL ORDER BY name",
        )?;
        let indexes: Vec<String> = idx_stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<_, _>>()?;
        for idx in &indexes {
            dump.push_str(idx);
            dump.push_str(";\n");
        }

        dump.push_str("COMMIT;\n");
        Ok(dump)
    }

    // ── Entity exporters ────────────────────────────────────────────

    fn export_sources(conn: &Connection) -> Result<Vec<Source>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT id, title, description, source_type, start_date, end_date,
                    status, updated_by, last_derived_at, created_at, updated_at
             FROM sources ORDER BY created_at DESC",
        )?;
        let sources = stmt
            .query_map([], |row| {
                Ok(Source {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    description: row.get(2)?,
                    source_type: row
                        .get::<_, String>(3)?
                        .parse()
                        .unwrap_or(SourceType::General),
                    start_date: row.get(4)?,
                    end_date: row.get(5)?,
                    status: row
                        .get::<_, String>(6)?
                        .parse()
                        .unwrap_or(SourceStatus::Draft),
                    updated_by: row.get::<_, String>(7)?.parse().unwrap_or(UpdatedBy::Human),
                    last_derived_at: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        Ok(sources)
    }

    fn export_bullets(conn: &Connection) -> Result<Vec<Bullet>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT id, content, source_content_snapshot, metrics, domain,
                    status, rejection_reason, prompt_log_id, approved_at,
                    approved_by, created_at
             FROM bullets ORDER BY created_at DESC",
        )?;

        let bullets_raw: Vec<Bullet> = stmt
            .query_map([], |row| {
                Ok(Bullet {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    source_content_snapshot: row.get::<_, String>(2).unwrap_or_default(),
                    technologies: Vec::new(),
                    metrics: row.get(3)?,
                    domain: row.get(4)?,
                    status: row
                        .get::<_, String>(5)?
                        .parse()
                        .unwrap_or(BulletStatus::Draft),
                    rejection_reason: row.get(6)?,
                    prompt_log_id: row.get(7)?,
                    approved_at: row.get(8)?,
                    approved_by: row.get(9)?,
                    created_at: row.get(10)?,
                })
            })?
            .collect::<Result<_, _>>()?;

        // Hydrate technologies
        let mut result = Vec::with_capacity(bullets_raw.len());
        for mut bullet in bullets_raw {
            let mut tech_stmt = conn.prepare(
                "SELECT s.name FROM bullet_skills bsk
                 JOIN skills s ON s.id = bsk.skill_id
                 WHERE bsk.bullet_id = ?1
                 ORDER BY LOWER(s.name) ASC",
            )?;
            let techs: Vec<String> = tech_stmt
                .query_map(params![bullet.id], |row| row.get(0))?
                .collect::<Result<_, _>>()?;
            bullet.technologies = techs;
            result.push(bullet);
        }

        Ok(result)
    }

    fn export_perspectives(conn: &Connection) -> Result<Vec<Perspective>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT id, bullet_id, content, bullet_content_snapshot, target_archetype,
                    domain, framing, status, rejection_reason, prompt_log_id,
                    approved_at, approved_by, created_at
             FROM perspectives ORDER BY created_at DESC",
        )?;
        let perspectives = stmt
            .query_map([], |row| {
                Ok(Perspective {
                    id: row.get(0)?,
                    bullet_id: row.get(1)?,
                    content: row.get(2)?,
                    bullet_content_snapshot: row.get(3)?,
                    target_archetype: row.get(4)?,
                    domain: row.get(5)?,
                    framing: row
                        .get::<_, String>(6)?
                        .parse()
                        .unwrap_or(Framing::Responsibility),
                    status: row
                        .get::<_, String>(7)?
                        .parse()
                        .unwrap_or(PerspectiveStatus::Draft),
                    rejection_reason: row.get(8)?,
                    prompt_log_id: row.get(9)?,
                    approved_at: row.get(10)?,
                    approved_by: row.get(11)?,
                    created_at: row.get(12)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        Ok(perspectives)
    }

    fn export_skills(conn: &Connection) -> Result<Vec<Skill>, ForgeError> {
        let mut stmt =
            conn.prepare("SELECT id, name, category FROM skills ORDER BY LOWER(name) ASC")?;
        let skills = stmt
            .query_map([], |row| {
                Ok(Skill {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    category: row
                        .get::<_, String>(2)?
                        .parse()
                        .unwrap_or(SkillCategory::Other),
                })
            })?
            .collect::<Result<_, _>>()?;
        Ok(skills)
    }

    fn export_organizations(conn: &Connection) -> Result<Vec<Organization>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT id, name, org_type, industry, size, worked, employment_type,
                    website, linkedin_url, glassdoor_url, glassdoor_rating,
                    status, created_at, updated_at
             FROM organizations ORDER BY LOWER(name) ASC",
        )?;

        let orgs_raw: Vec<Organization> = stmt
            .query_map([], |row| {
                Ok(Organization {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    org_type: row.get(2)?,
                    tags: Vec::new(), // hydrated below
                    industry: row.get(3)?,
                    size: row.get(4)?,
                    worked: row.get(5)?,
                    employment_type: row.get(6)?,
                    website: row.get(7)?,
                    linkedin_url: row.get(8)?,
                    glassdoor_url: row.get(9)?,
                    glassdoor_rating: row.get(10)?,
                    status: row
                        .get::<_, Option<String>>(11)?
                        .and_then(|s| s.parse().ok()),
                    created_at: row.get(12)?,
                    updated_at: row.get(13)?,
                })
            })?
            .collect::<Result<_, _>>()?;

        // Hydrate tags
        let mut result = Vec::with_capacity(orgs_raw.len());
        for mut org in orgs_raw {
            let mut tag_stmt = conn
                .prepare("SELECT tag FROM org_tags WHERE organization_id = ?1 ORDER BY tag ASC")?;
            let tags: Vec<OrgTag> = tag_stmt
                .query_map(params![org.id], |row| {
                    let tag_str: String = row.get(0)?;
                    Ok(tag_str.parse().unwrap_or(OrgTag::Company))
                })?
                .collect::<Result<_, _>>()?;
            org.tags = tags;
            result.push(org);
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::stores::bullet::BulletStore;
    use crate::db::stores::skill::SkillStore;
    use crate::db::stores::source::SourceStore;
    use crate::forge::Forge;
    use forge_core::{CreateSource, SourceType};

    fn setup() -> Forge {
        Forge::open_memory().unwrap()
    }

    #[test]
    fn export_empty_bundle() {
        let forge = setup();
        let bundle = ExportService::export_data(forge.conn(), &["sources".into()]).unwrap();
        assert_eq!(bundle.forge_export.version, "1.0");
        assert_eq!(bundle.sources.unwrap().len(), 0);
        assert!(bundle.bullets.is_none());
    }

    #[test]
    fn export_sources() {
        let forge = setup();
        SourceStore::create(
            forge.conn(),
            &CreateSource {
                title: "Test Source".into(),
                description: "Desc".into(),
                source_type: Some(SourceType::General),
                ..Default::default()
            },
        )
        .unwrap();

        let bundle = ExportService::export_data(forge.conn(), &["sources".into()]).unwrap();
        let sources = bundle.sources.unwrap();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].title, "Test Source");
    }

    #[test]
    fn export_bullets_with_technologies() {
        let forge = setup();
        let src = SourceStore::create(
            forge.conn(),
            &CreateSource {
                title: "Source".into(),
                description: "Desc".into(),
                source_type: Some(SourceType::General),
                ..Default::default()
            },
        )
        .unwrap();

        BulletStore::create(
            forge.conn(),
            "Built APIs",
            None,
            None,
            None,
            &[(src.base.id, true)],
            &["rust".into(), "axum".into()],
        )
        .unwrap();

        let bundle = ExportService::export_data(forge.conn(), &["bullets".into()]).unwrap();
        let bullets = bundle.bullets.unwrap();
        assert_eq!(bullets.len(), 1);
        assert_eq!(bullets[0].technologies, vec!["axum", "rust"]);
    }

    #[test]
    fn export_skills() {
        let forge = setup();
        SkillStore::create(forge.conn(), "Rust", Some(SkillCategory::Language)).unwrap();
        SkillStore::create(forge.conn(), "Docker", Some(SkillCategory::Tool)).unwrap();

        let bundle = ExportService::export_data(forge.conn(), &["skills".into()]).unwrap();
        let skills = bundle.skills.unwrap();
        assert_eq!(skills.len(), 2);
    }

    #[test]
    fn export_multiple_entities() {
        let forge = setup();
        let bundle = ExportService::export_data(
            forge.conn(),
            &["sources".into(), "bullets".into(), "skills".into()],
        )
        .unwrap();

        assert!(bundle.sources.is_some());
        assert!(bundle.bullets.is_some());
        assert!(bundle.skills.is_some());
        assert!(bundle.perspectives.is_none());
    }

    #[test]
    fn unknown_entity_silently_ignored() {
        let forge = setup();
        let bundle =
            ExportService::export_data(forge.conn(), &["sources".into(), "nonexistent".into()])
                .unwrap();
        assert!(bundle.sources.is_some());
    }

    #[test]
    fn dump_database_contains_tables() {
        let forge = setup();
        let dump = ExportService::dump_database(forge.conn()).unwrap();
        assert!(dump.contains("BEGIN TRANSACTION"));
        assert!(dump.contains("COMMIT"));
        assert!(dump.contains("CREATE TABLE"));
        assert!(dump.contains("sources"));
        assert!(dump.contains("bullets"));
    }

    #[test]
    fn dump_database_contains_data() {
        let forge = setup();
        SourceStore::create(
            forge.conn(),
            &CreateSource {
                title: "Dump Test".into(),
                description: "Desc".into(),
                source_type: Some(SourceType::General),
                ..Default::default()
            },
        )
        .unwrap();

        let dump = ExportService::dump_database(forge.conn()).unwrap();
        assert!(dump.contains("Dump Test"));
        assert!(dump.contains("INSERT INTO"));
    }
}
