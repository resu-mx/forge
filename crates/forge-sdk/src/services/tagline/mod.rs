//! Resume tagline engine. TS: `packages/core/src/services/tagline-service.ts` and the tagline
//! routes in `packages/core/src/routes/resumes.ts:222-307`.
//!
//! resu-mx/forge#67 adds `generator` (tokenize, TF-IDF, rank, format) and `regenerate`.

use rusqlite::Connection;

use forge_core::{ForgeError, ResumeTaglineState};

use crate::db::ResumeStore;

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

/// TS `!!(override && override.trim().length > 0)`
/// (`routes/resumes.ts:239`, `:296`; `services/tagline-service.ts:356`).
pub fn has_override(tagline_override: Option<&str>) -> bool {
    tagline_override.is_some_and(|s| !js_trim(s).is_empty())
}

/// The wire state for the two stored columns (`routes/resumes.ts:239-249`).
pub fn tagline_state(
    generated_tagline: Option<String>,
    tagline_override: Option<String>,
) -> ResumeTaglineState {
    let has_override = has_override(tagline_override.as_deref());
    let resolved = tagline_override
        .as_deref()
        .or(generated_tagline.as_deref())
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
        // TS quirk, kept on purpose (resu-mx/forge#72): a blank override still wins `resolved`.
        let s = tagline_state(Some("gen".into()), Some("   ".into()));
        assert_eq!((s.resolved.as_str(), s.has_override), ("   ", false));
        // U+FEFF-only is blank in JS; U+0085-only is not.
        assert!(!has_override(Some("\u{FEFF}")));
        assert!(has_override(Some("\u{0085}")));
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
