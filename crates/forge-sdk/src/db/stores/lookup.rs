//! Shared rules for the three name/description lookup tables: `domains`,
//! `industries` and `role_types` (resu-mx/forge#17, #18, #19, #140).
//!
//! The tables have the same columns and a `UNIQUE` name (migrations 003 and 032).
//! TS enforces these rules in its services and the ELM; Rust enforces them here,
//! in the store layer, so forge-server and the browser runtime behave the same.

use rusqlite::{params, Connection, OptionalExtension};

use forge_core::ForgeError;

/// The lookup tables these helpers may touch. A closed set, so a table name
/// that ends up in SQL never comes from input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LookupTable {
    Domains,
    Industries,
    RoleTypes,
}

impl LookupTable {
    pub fn table(self) -> &'static str {
        match self {
            Self::Domains => "domains",
            Self::Industries => "industries",
            Self::RoleTypes => "role_types",
        }
    }
}

/// domain-service.ts:23, industry-service.ts:26, role-type-service.ts:23, and their update paths.
pub const NAME_EMPTY: &str = "Name must not be empty";
/// The create wording (domain-service.ts:31).
pub const DOMAIN_NAME_FORMAT_ON_CREATE: &str =
    "Domain name must be lowercase, start with a letter, and contain only letters, digits, and underscores";
/// The update wording (domain-service.ts:97). TS really does use two different messages.
pub const DOMAIN_NAME_FORMAT_ON_UPDATE: &str =
    "Domain name must be lowercase with underscores only";

fn invalid_name(message: &str) -> ForgeError {
    ForgeError::Validation {
        message: message.into(),
        field: Some("name".into()),
    }
}

/// The trimmed name, or `Validation(NAME_EMPTY)` when it is empty or whitespace-only.
pub fn require_name(name: &str) -> Result<&str, ForgeError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(invalid_name(NAME_EMPTY));
    }
    Ok(trimmed)
}

/// `^[a-z][a-z0-9_]*$`, without adding `regex` to forge-sdk.
pub fn is_domain_slug(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some('a'..='z'))
        && chars.all(|c| matches!(c, 'a'..='z' | '0'..='9' | '_'))
}

/// Domain name rules in TS order: empty first, then format. Domain names are not
/// trimmed (TS doesn't trim them), so `" security"` fails the format check.
pub fn validate_domain_name(name: &str, format_message: &str) -> Result<(), ForgeError> {
    require_name(name)?;
    if !is_domain_slug(name) {
        return Err(invalid_name(format_message));
    }
    Ok(())
}

/// `Conflict` when another row of `table` already has `name`.
///
/// Mirrors the ELM's uniqueness check (lifecycle-manager.ts:863-891), which skips
/// the row being updated, and its message (errors.ts:96-108).
pub fn ensure_name_free(
    conn: &Connection,
    table: LookupTable,
    name: &str,
    exclude_id: Option<&str>,
) -> Result<(), ForgeError> {
    let sql = format!(
        "SELECT 1 FROM {} WHERE name = ?1 AND (?2 IS NULL OR id <> ?2) LIMIT 1",
        table.table()
    );
    let taken = conn
        .query_row(&sql, params![name, exclude_id], |_| Ok(()))
        .optional()?
        .is_some();
    if taken {
        return Err(ForgeError::Conflict {
            message: format!(
                "{}.name must be unique: \"{}\" already exists",
                table.table(),
                name
            ),
        });
    }
    Ok(())
}

/// Write the fields of a partial update that are present.
///
/// `description`: `None` leaves the column, `Some(None)` clears it, `Some(Some(v))`
/// sets it. Callers validate the name and check that the row exists first. With no
/// fields present nothing runs (the TS adapter does the same, sqlite-adapter.ts:134-135).
pub fn update_name_description(
    conn: &Connection,
    table: LookupTable,
    id: &str,
    name: Option<&str>,
    description: Option<Option<&str>>,
) -> Result<usize, ForgeError> {
    let mut sets = Vec::new();
    let mut values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(v) = name {
        sets.push(format!("name = ?{}", values.len() + 1));
        values.push(Box::new(v.to_string()));
    }
    if let Some(v) = description {
        sets.push(format!("description = ?{}", values.len() + 1));
        values.push(Box::new(v.map(str::to_string)));
    }
    if sets.is_empty() {
        return Ok(0);
    }

    let sql = format!(
        "UPDATE {} SET {} WHERE id = ?{}",
        table.table(),
        sets.join(", "),
        values.len() + 1
    );
    values.push(Box::new(id.to_string()));
    Ok(conn.execute(
        &sql,
        rusqlite::params_from_iter(values.iter().map(|b| b.as_ref())),
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::Forge;
    use forge_core::new_id;

    /// Seeded by migration 003.
    const SECURITY: &str = "d0000001-0000-4000-8000-000000000003";

    fn insert_lookup(forge: &Forge, table: &str, name: &str, description: Option<&str>) -> String {
        let id = new_id();
        forge
            .conn()
            .execute(
                &format!("INSERT INTO {table} (id, name, description) VALUES (?1, ?2, ?3)"),
                params![id, name, description],
            )
            .unwrap();
        id
    }

    fn row(conn: &Connection, table: &str, id: &str) -> (String, Option<String>) {
        conn.query_row(
            &format!("SELECT name, description FROM {table} WHERE id = ?1"),
            params![id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap()
    }

    #[test]
    fn require_name_rejects_blank_and_trims() {
        for blank in ["", " ", "\t\n "] {
            assert!(matches!(require_name(blank),
                Err(ForgeError::Validation { ref message, .. }) if message == NAME_EMPTY));
        }
        assert_eq!(require_name("  FinTech  ").unwrap(), "FinTech");
    }

    #[test]
    fn domain_slug_matches_the_ts_regex() {
        for ok in ["a", "security", "ai_ml", "cloud2", "a_"] {
            assert!(is_domain_slug(ok), "{ok}");
        }
        for bad in [
            "",
            "Security",
            "cloud security",
            "2fa",
            "_x",
            "a-b",
            "é",
            " security",
        ] {
            assert!(!is_domain_slug(bad), "{bad}");
        }
    }

    #[test]
    fn validate_domain_name_checks_empty_before_format() {
        let err = validate_domain_name("   ", DOMAIN_NAME_FORMAT_ON_CREATE).unwrap_err();
        assert!(matches!(&err, ForgeError::Validation { message, .. } if message == NAME_EMPTY));
        let err = validate_domain_name("Bad Name", DOMAIN_NAME_FORMAT_ON_UPDATE).unwrap_err();
        assert!(matches!(&err, ForgeError::Validation { message, field }
            if message == DOMAIN_NAME_FORMAT_ON_UPDATE && field.as_deref() == Some("name")));
        validate_domain_name("cloud_security", DOMAIN_NAME_FORMAT_ON_CREATE).unwrap();
    }

    #[test]
    fn ensure_name_free_conflicts_only_on_another_row() {
        let forge = Forge::open_memory().unwrap();
        let err =
            ensure_name_free(forge.conn(), LookupTable::Domains, "security", None).unwrap_err();
        assert!(matches!(&err, ForgeError::Conflict { message }
            if message == "domains.name must be unique: \"security\" already exists"));
        // The row itself is excluded, so re-sending its own name is fine.
        ensure_name_free(
            forge.conn(),
            LookupTable::Domains,
            "security",
            Some(SECURITY),
        )
        .unwrap();
        // BINARY collation, like the UNIQUE index.
        ensure_name_free(forge.conn(), LookupTable::Domains, "Security", None).unwrap();

        insert_lookup(&forge, "industries", "Aero", None);
        assert!(matches!(
            ensure_name_free(forge.conn(), LookupTable::Industries, "Aero", None),
            Err(ForgeError::Conflict { .. })
        ));

        insert_lookup(&forge, "role_types", "Pilot", None);
        let err =
            ensure_name_free(forge.conn(), LookupTable::RoleTypes, "Pilot", None).unwrap_err();
        assert!(matches!(&err, ForgeError::Conflict { message }
            if message == "role_types.name must be unique: \"Pilot\" already exists"));
    }

    #[test]
    fn update_name_description_writes_only_present_fields() {
        let forge = Forge::open_memory().unwrap();
        let conn = forge.conn();
        let id = insert_lookup(&forge, "industries", "Aero", Some("Planes"));

        assert_eq!(
            update_name_description(conn, LookupTable::Industries, &id, None, None).unwrap(),
            0
        );
        assert_eq!(
            row(conn, "industries", &id),
            ("Aero".into(), Some("Planes".into()))
        );

        update_name_description(conn, LookupTable::Industries, &id, None, Some(None)).unwrap();
        assert_eq!(row(conn, "industries", &id), ("Aero".into(), None));

        update_name_description(
            conn,
            LookupTable::Industries,
            &id,
            Some("Aerospace"),
            Some(Some("Air and space")),
        )
        .unwrap();
        assert_eq!(
            row(conn, "industries", &id),
            ("Aerospace".into(), Some("Air and space".into()))
        );

        assert_eq!(
            update_name_description(conn, LookupTable::Industries, "missing", Some("X"), None)
                .unwrap(),
            0
        );

        let did = insert_lookup(&forge, "domains", "temp_dom", None);
        assert_eq!(
            update_name_description(conn, LookupTable::Domains, &did, Some("renamed"), None)
                .unwrap(),
            1
        );
        assert_eq!(row(conn, "domains", &did).0, "renamed");

        let rid = insert_lookup(&forge, "role_types", "Pilot", None);
        assert_eq!(
            update_name_description(
                conn,
                LookupTable::RoleTypes,
                &rid,
                None,
                Some(Some("Flies"))
            )
            .unwrap(),
            1
        );
        assert_eq!(
            row(conn, "role_types", &rid),
            ("Pilot".into(), Some("Flies".into()))
        );
    }
}
