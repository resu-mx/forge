//! Resume tagline engine. TS: `packages/core/src/services/tagline-service.ts` and the tagline
//! routes in `packages/core/src/routes/resumes.ts:222-307`.
//!
//! resu-mx/forge#67 adds `generator` (tokenize, TF-IDF, rank, format) and `regenerate`.

use rusqlite::Connection;

use forge_core::{ForgeError, ResumeTaglineState};

use crate::db::ResumeStore;

mod generator;
#[cfg(test)]
mod golden;
mod regenerate;

pub use regenerate::regenerate;

pub use generator::{
    compute_tf_idf, generate_tagline, rank_keywords, tokenize, GeneratedTagline, DEFAULT_TOP_K,
    SKILL_MATCH_BOOST,
};

/// ECMAScript WhiteSpace + LineTerminator: the set that JS `\s` and `String.prototype.trim`
/// use. Rust's `char::is_whitespace` (Unicode White_Space) differs in two code points:
/// it includes U+0085, which JS does not, and it excludes U+FEFF, which JS includes.
pub fn is_js_whitespace(c: char) -> bool {
    c == '\u{FEFF}' || (c != '\u{0085}' && c.is_whitespace())
}

/// JS `s.trim()`.
pub fn js_trim(s: &str) -> &str {
    s.trim_matches(is_js_whitespace)
}

/// The one rule for a tagline column that may hold text: an empty or whitespace-only string is
/// absent (resu-mx/forge#72). TS `nonBlank` (`services/tagline-service.ts`). The writers already
/// store blank as NULL, so this only matters for a row written another way. The text is returned
/// as stored, never trimmed. `resolved`, `has_override` and the resume header all use it.
pub fn non_blank(value: Option<&str>) -> Option<&str> {
    value.filter(|s| !js_trim(s).is_empty())
}

/// TS `nonBlank(override) !== null` (`services/tagline-service.ts`, and the routes in
/// `routes/resumes.ts`).
pub fn has_override(tagline_override: Option<&str>) -> bool {
    non_blank(tagline_override).is_some()
}

/// The wire state for the two stored columns (TS `resolveTagline`): a real override, else the
/// generated tagline, else `""`. `has_override` agrees with `resolved` by construction.
pub fn tagline_state(
    generated_tagline: Option<String>,
    tagline_override: Option<String>,
) -> ResumeTaglineState {
    let real_override = non_blank(tagline_override.as_deref());
    let has_override = real_override.is_some();
    let resolved = real_override
        .or(non_blank(generated_tagline.as_deref()))
        .unwrap_or_default()
        .to_string();
    ResumeTaglineState {
        generated_tagline,
        tagline_override,
        resolved,
        has_override,
    }
}

/// `Ok(None)` when no resume has this id.
pub fn get_state(
    conn: &Connection,
    resume_id: &str,
) -> Result<Option<ResumeTaglineState>, ForgeError> {
    Ok(ResumeStore::get(conn, resume_id)?
        .map(|r| tagline_state(r.generated_tagline, r.tagline_override)))
}

/// `PATCH /resumes/:id/tagline-override` (TS `routes/resumes.ts:269-307`). `None`, `""` and
/// JS-blank strings clear the override (NULL); anything else is stored verbatim, untrimmed.
/// `Ok(None)` when no resume has this id, in which case nothing is written.
pub fn set_override(
    conn: &Connection,
    resume_id: &str,
    content: Option<&str>,
) -> Result<Option<ResumeTaglineState>, ForgeError> {
    let normalized = content.filter(|c| !js_trim(c).is_empty());
    match ResumeStore::update_tagline_override(conn, resume_id, normalized) {
        Ok(resume) => Ok(Some(tagline_state(
            resume.generated_tagline,
            resume.tagline_override,
        ))),
        Err(ForgeError::NotFound { .. }) => Ok(None),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::Forge;
    use forge_core::CreateResume;

    #[test]
    fn js_whitespace_matches_ecmascript() {
        assert!(is_js_whitespace('\u{FEFF}'));
        assert!(!is_js_whitespace('\u{0085}'));
        for c in [
            ' ', '\t', '\n', '\u{0B}', '\u{0C}', '\r', '\u{A0}', '\u{2028}', '\u{3000}',
        ] {
            assert!(is_js_whitespace(c), "{c:?}");
        }
        assert_eq!(js_trim("\u{FEFF} x \u{A0}"), "x");
        assert_eq!(js_trim("\u{0085}"), "\u{0085}");
    }

    #[test]
    fn state_rules_match_ts() {
        let s = tagline_state(Some("gen".into()), None);
        assert_eq!((s.resolved.as_str(), s.has_override), ("gen", false));
        let s = tagline_state(Some("gen".into()), Some("mine".into()));
        assert_eq!((s.resolved.as_str(), s.has_override), ("mine", true));
        let s = tagline_state(None, None);
        assert_eq!((s.resolved.as_str(), s.has_override), ("", false));
        // resu-mx/forge#72: a blank override is no override, so `resolved` and `has_override`
        // agree. The stored text is not changed (the state still reports it as stored).
        let s = tagline_state(Some("gen".into()), Some("   ".into()));
        assert_eq!((s.resolved.as_str(), s.has_override), ("gen", false));
        assert_eq!(s.tagline_override.as_deref(), Some("   "));
        let s = tagline_state(Some("gen".into()), Some(String::new()));
        assert_eq!((s.resolved.as_str(), s.has_override), ("gen", false));
        let s = tagline_state(None, Some("   ".into()));
        assert_eq!((s.resolved.as_str(), s.has_override), ("", false));
        // A blank generated tagline is absent too.
        let s = tagline_state(Some("  ".into()), None);
        assert_eq!(s.resolved, "");
        // U+FEFF-only is blank in JS; U+0085-only is not.
        assert!(!has_override(Some("\u{FEFF}")));
        assert!(has_override(Some("\u{0085}")));
    }

    #[test]
    fn non_blank_returns_the_text_as_stored() {
        assert_eq!(non_blank(None), None);
        assert_eq!(non_blank(Some("")), None);
        assert_eq!(non_blank(Some(" \t\n")), None);
        assert_eq!(non_blank(Some("\u{FEFF}")), None);
        // Never trimmed: only the decision is.
        assert_eq!(non_blank(Some("  x ")), Some("  x "));
    }

    #[test]
    fn get_state_reads_the_columns_and_misses_cleanly() {
        let forge = Forge::open_memory().unwrap();
        assert!(get_state(forge.conn(), "missing").unwrap().is_none());
        let id = ResumeStore::create(
            forge.conn(),
            &CreateResume {
                name: "R".into(),
                target_role: "Cloud Engineer".into(),
                target_employer: "Acme".into(),
                archetype: "sre".into(),
                summary_id: None,
            },
        )
        .unwrap()
        .id;
        let s = get_state(forge.conn(), &id).unwrap().unwrap();
        assert_eq!(
            (
                s.generated_tagline,
                s.tagline_override,
                s.resolved.as_str(),
                s.has_override
            ),
            (None, None, "", false)
        );

        forge
            .conn()
            .execute(
                "UPDATE resumes SET generated_tagline = ?1, tagline_override = ?2, updated_at = ?3 WHERE id = ?4",
                rusqlite::params!["gen", "mine", "2000-01-01T00:00:00Z", id],
            )
            .unwrap();
        let s = get_state(forge.conn(), &id).unwrap().unwrap();
        assert_eq!(s.generated_tagline.as_deref(), Some("gen"));
        assert_eq!(s.tagline_override.as_deref(), Some("mine"));
        assert_eq!((s.resolved.as_str(), s.has_override), ("mine", true));

        // Read-only: updated_at is untouched.
        let updated_at: String = forge
            .conn()
            .query_row("SELECT updated_at FROM resumes WHERE id = ?1", [&id], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(updated_at, "2000-01-01T00:00:00Z");
    }

    fn resume_with_taglines(forge: &Forge, generated: Option<&str>, ov: Option<&str>) -> String {
        let id = ResumeStore::create(
            forge.conn(),
            &CreateResume {
                name: "R".into(),
                target_role: "Cloud Engineer".into(),
                target_employer: "Acme".into(),
                archetype: "sre".into(),
                summary_id: None,
            },
        )
        .unwrap()
        .id;
        forge
            .conn()
            .execute(
                "UPDATE resumes SET generated_tagline = ?1, tagline_override = ?2, updated_at = ?3 WHERE id = ?4",
                rusqlite::params![generated, ov, "2000-01-01T00:00:00Z", id],
            )
            .unwrap();
        id
    }

    fn stored(forge: &Forge, id: &str) -> (Option<String>, Option<String>, String) {
        forge
            .conn()
            .query_row(
                "SELECT generated_tagline, tagline_override, updated_at FROM resumes WHERE id = ?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap()
    }

    #[test]
    fn set_override_stores_verbatim_and_bumps_updated_at() {
        let forge = Forge::open_memory().unwrap();
        let id = resume_with_taglines(&forge, Some("gen"), None);
        let s = set_override(forge.conn(), &id, Some("  Padded  "))
            .unwrap()
            .unwrap();
        assert_eq!(s.tagline_override.as_deref(), Some("  Padded  "));
        assert_eq!((s.resolved.as_str(), s.has_override), ("  Padded  ", true));
        let (generated, ov, updated_at) = stored(&forge, &id);
        assert_eq!(generated.as_deref(), Some("gen"));
        assert_eq!(ov.as_deref(), Some("  Padded  "));
        assert_ne!(updated_at, "2000-01-01T00:00:00Z");
    }

    #[test]
    fn set_override_clears_on_blank_and_keeps_u0085() {
        let forge = Forge::open_memory().unwrap();
        let id = resume_with_taglines(&forge, Some("gen"), Some("old"));
        for blank in [None, Some(""), Some("   "), Some("\u{FEFF}")] {
            set_override(forge.conn(), &id, Some("old")).unwrap();
            let s = set_override(forge.conn(), &id, blank).unwrap().unwrap();
            assert_eq!(s.tagline_override, None, "{blank:?}");
            assert_eq!((s.resolved.as_str(), s.has_override), ("gen", false));
            assert_eq!(stored(&forge, &id).1, None);
        }
        // JS keeps U+0085, so it is a real (non-blank) override.
        let s = set_override(forge.conn(), &id, Some("\u{0085}"))
            .unwrap()
            .unwrap();
        assert_eq!(s.tagline_override.as_deref(), Some("\u{0085}"));
        assert!(s.has_override);
    }

    #[test]
    fn set_override_unknown_id_is_none() {
        let forge = Forge::open_memory().unwrap();
        assert!(set_override(forge.conn(), "missing", Some("x"))
            .unwrap()
            .is_none());
    }
}
