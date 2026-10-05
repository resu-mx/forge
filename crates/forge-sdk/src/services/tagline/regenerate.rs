//! DB-backed regeneration. TS: `regenerateResumeTagline`
//! (`packages/core/src/services/tagline-service.ts:301-359`).

use rusqlite::Connection;

use forge_core::{ForgeError, ResumeTaglineRegenerationResult};

use super::{generate_tagline, has_override, DEFAULT_TOP_K};
use crate::db::{ResumeStore, SkillStore};

/// Recompute `resumes.generated_tagline` from the JDs linked to the resume and every skill.
///
/// - `Ok(None)`: no resume has this id, and nothing is written.
/// - Writes `generated_tagline` (NULL for an empty result) and bumps `updated_at`. It never
///   writes `tagline_override`.
/// - It opens no transaction, so callers (resu-mx/forge#71's link/unlink) may run it inside theirs.
pub fn regenerate(
    conn: &Connection,
    resume_id: &str,
) -> Result<Option<ResumeTaglineRegenerationResult>, ForgeError> {
    let Some(resume) = ResumeStore::get(conn, resume_id)? else {
        return Ok(None);
    };
    let jd_texts = ResumeStore::linked_jd_texts(conn, resume_id)?;
    let skill_names: Vec<String> = SkillStore::list(conn, None, None, None)?
        .into_iter()
        .map(|s| s.name)
        .collect();

    let generated = generate_tagline(
        &jd_texts,
        &skill_names,
        DEFAULT_TOP_K,
        Some(resume.target_role.as_str()),
    );

    let stored = (!generated.tagline.is_empty()).then_some(generated.tagline.as_str());
    ResumeStore::set_generated_tagline(conn, resume_id, stored)?;

    Ok(Some(ResumeTaglineRegenerationResult {
        has_override: has_override(resume.tagline_override.as_deref()),
        generated_tagline: generated.tagline,
        keywords: generated.keywords,
    }))
}

#[cfg(test)]
mod tests {
    use rusqlite::params;

    use super::*;
    use crate::forge::Forge;
    use forge_core::{new_id, CreateResume};

    fn resume(forge: &Forge, target_role: &str) -> String {
        ResumeStore::create(
            forge.conn(),
            &CreateResume {
                name: "R".into(),
                target_role: target_role.into(),
                target_employer: "Acme".into(),
                archetype: "sre".into(),
                summary_id: None,
            },
        )
        .unwrap()
        .id
    }

    /// Insert a JD with `raw_text` and link it to the resume (mirrors the TS route-test seeds).
    fn link_jd(forge: &Forge, resume_id: &str, raw_text: &str) {
        let jd_id = new_id();
        forge
            .conn()
            .execute(
                "INSERT INTO job_descriptions (id, title, raw_text, status) VALUES (?1, 'JD', ?2, 'discovered')",
                params![jd_id, raw_text],
            )
            .unwrap();
        forge
            .conn()
            .execute(
                "INSERT INTO job_description_resumes (job_description_id, resume_id) VALUES (?1, ?2)",
                params![jd_id, resume_id],
            )
            .unwrap();
    }

    fn stored(forge: &Forge, id: &str) -> forge_core::Resume {
        ResumeStore::get(forge.conn(), id).unwrap().unwrap()
    }

    #[test]
    fn regenerate_unknown_resume_is_none() {
        let forge = Forge::open_memory().unwrap();
        let id = resume(&forge, "SRE");
        forge
            .conn()
            .execute(
                "UPDATE resumes SET generated_tagline = 'keep' WHERE id = ?1",
                params![id],
            )
            .unwrap();
        assert!(regenerate(forge.conn(), "missing").unwrap().is_none());
        assert_eq!(
            stored(&forge, &id).generated_tagline.as_deref(),
            Some("keep")
        );
    }

    #[test]
    fn regenerate_uses_target_role_prefix_and_skill_boost() {
        let forge = Forge::open_memory().unwrap();
        let id = resume(&forge, "Senior Platform Engineer");
        SkillStore::create(forge.conn(), "Terraform", None).unwrap();
        link_jd(&forge, &id, "Terraform Kubernetes Python Ansible");

        let out = regenerate(forge.conn(), &id).unwrap().unwrap();
        // Values produced by TS regenerateResumeTagline on the same seed.
        assert_eq!(
            out.generated_tagline,
            "Senior Platform Engineer -- terraform + ansible + kubernetes"
        );
        let got: Vec<(&str, f64, bool)> = out
            .keywords
            .iter()
            .map(|k| (k.term.as_str(), k.score, k.matched_skill))
            .collect();
        assert_eq!(
            got,
            [
                ("terraform", 2.0, true),
                ("ansible", 1.0, false),
                ("kubernetes", 1.0, false)
            ]
        );
        assert!(!out.has_override);
        assert_eq!(
            stored(&forge, &id).generated_tagline.as_deref(),
            Some(out.generated_tagline.as_str())
        );
    }

    #[test]
    fn regenerate_aggregates_jds_in_link_order() {
        let forge = Forge::open_memory().unwrap();
        let id = resume(&forge, "SRE");
        link_jd(&forge, &id, "kubernetes terraform python");
        link_jd(&forge, &id, "kubernetes prometheus grafana");

        let out = regenerate(forge.conn(), &id).unwrap().unwrap();
        assert_eq!(
            out.generated_tagline,
            "SRE -- kubernetes + grafana + prometheus"
        );
        let terms: Vec<&str> = out.keywords.iter().map(|k| k.term.as_str()).collect();
        assert_eq!(terms, ["kubernetes", "grafana", "prometheus"]);
        let tail = 1.5f64.ln() + 1.0;
        assert!((out.keywords[0].score - 2.0).abs() < 1e-9);
        assert!((out.keywords[1].score - tail).abs() < 1e-9);
        assert!((out.keywords[2].score - tail).abs() < 1e-9);
    }

    #[test]
    fn regenerate_without_links_stores_null_and_returns_empty() {
        let forge = Forge::open_memory().unwrap();
        let id = resume(&forge, "SRE");
        forge
            .conn()
            .execute(
                "UPDATE resumes SET generated_tagline = 'stale' WHERE id = ?1",
                params![id],
            )
            .unwrap();
        let out = regenerate(forge.conn(), &id).unwrap().unwrap();
        assert_eq!(
            (out.generated_tagline.as_str(), out.keywords.len()),
            ("", 0)
        );
        assert!(!out.has_override);
        assert_eq!(stored(&forge, &id).generated_tagline, None);
    }

    #[test]
    fn regenerate_skips_empty_raw_text() {
        let forge = Forge::open_memory().unwrap();
        let id = resume(&forge, "SRE");
        link_jd(&forge, &id, "");
        let out = regenerate(forge.conn(), &id).unwrap().unwrap();
        assert_eq!(out.generated_tagline, "");
        assert!(out.keywords.is_empty());
        assert_eq!(stored(&forge, &id).generated_tagline, None);
    }

    #[test]
    fn regenerate_empty_target_role_has_no_prefix() {
        let forge = Forge::open_memory().unwrap();
        let id = resume(&forge, "");
        link_jd(&forge, &id, "kubernetes terraform");
        let out = regenerate(forge.conn(), &id).unwrap().unwrap();
        assert_eq!(out.generated_tagline, "kubernetes + terraform");
    }

    #[test]
    fn regenerate_keeps_override_and_reports_has_override() {
        let forge = Forge::open_memory().unwrap();
        let id = resume(&forge, "SRE");
        forge
            .conn()
            .execute(
                "UPDATE resumes SET tagline_override = ' Mine ' WHERE id = ?1",
                params![id],
            )
            .unwrap();
        link_jd(&forge, &id, "Kafka Kubernetes Python");
        let out = regenerate(forge.conn(), &id).unwrap().unwrap();
        assert!(out.has_override);
        assert!(!out.generated_tagline.is_empty());
        let row = stored(&forge, &id);
        assert_eq!(row.tagline_override.as_deref(), Some(" Mine "));
        assert_eq!(
            row.generated_tagline.as_deref(),
            Some(out.generated_tagline.as_str())
        );
    }

    #[test]
    fn regenerate_bumps_updated_at() {
        let forge = Forge::open_memory().unwrap();
        let id = resume(&forge, "SRE");
        forge
            .conn()
            .execute(
                "UPDATE resumes SET updated_at = '2000-01-01T00:00:00Z' WHERE id = ?1",
                params![id],
            )
            .unwrap();
        regenerate(forge.conn(), &id).unwrap().unwrap();
        assert_ne!(stored(&forge, &id).updated_at, "2000-01-01T00:00:00Z");
    }

    #[test]
    fn regenerate_inside_a_rolled_back_transaction_writes_nothing() {
        let forge = Forge::open_memory().unwrap();
        let id = resume(&forge, "SRE");
        link_jd(&forge, &id, "kafka kubernetes terraform");
        {
            let tx = forge.conn().unchecked_transaction().unwrap();
            assert!(regenerate(&tx, &id).unwrap().is_some());
            // dropped without commit, so it rolls back
        }
        assert_eq!(stored(&forge, &id).generated_tagline, None);
    }
}
