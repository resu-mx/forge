//! Extension stores: the browser extension's config (`extension_config`) and its error
//! log (`extension_logs`), both from migration 051.
//!
//! Mirrors `packages/core/src/services/extension-config-service.ts` (raw SQL: the table is
//! keyed by a TEXT `key`, not a UUID) and `extension-log-service.ts`.

use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;

use forge_core::{
    new_id, now_iso, CreateExtensionLog, ExtensionConfig, ExtensionLog, ExtensionLogFilter,
    ForgeError, EXTENSION_CONFIG_KEYS,
};

const UPSERT_CONFIG: &str =
    "INSERT INTO extension_config (key, value, updated_at) VALUES (?1, ?2, datetime('now'))
     ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at";

const LOG_COLUMNS: &str = "id, error_code, message, layer, plugin, url, context, created_at";

/// Data access for `extension_config` (TEXT `key` -> JSON text `value`).
pub struct ExtensionConfigStore;

impl ExtensionConfigStore {
    /// All four keys, stored values merged over the defaults (TS `getAll`).
    pub fn get_all(conn: &Connection) -> Result<ExtensionConfig, ForgeError> {
        let mut config = ExtensionConfig::default();
        let mut stmt = conn.prepare("SELECT key, value FROM extension_config")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (key, value) = row?;
            if let Some(slot) = slot(&mut config, &key) {
                *slot = serde_json::from_str(&value).map_err(|e| {
                    ForgeError::Internal(format!("extension_config.{key} is not valid JSON: {e}"))
                })?;
            }
        }
        Ok(config)
    }

    /// Check every key, then upsert them all in one transaction (TS `setMany`).
    pub fn set_many(
        conn: &Connection,
        updates: &[(String, Value)],
    ) -> Result<ExtensionConfig, ForgeError> {
        if let Some((key, _)) = updates
            .iter()
            .find(|(k, _)| !EXTENSION_CONFIG_KEYS.contains(&k.as_str()))
        {
            return Err(ForgeError::Validation {
                message: format!("Unknown config key: {key}"),
                field: Some(key.clone()),
            });
        }
        let tx = conn.unchecked_transaction()?;
        for (key, value) in updates {
            let text =
                serde_json::to_string(value).map_err(|e| ForgeError::Internal(e.to_string()))?;
            tx.execute(UPSERT_CONFIG, params![key, text])?;
        }
        tx.commit()?;
        Self::get_all(conn)
    }
}

/// The field for a config key, or `None` for any other key.
fn slot<'a>(config: &'a mut ExtensionConfig, key: &str) -> Option<&'a mut Value> {
    match key {
        "baseUrl" => Some(&mut config.base_url),
        "devMode" => Some(&mut config.dev_mode),
        "enabledPlugins" => Some(&mut config.enabled_plugins),
        "enableServerLogging" => Some(&mut config.enable_server_logging),
        _ => None,
    }
}

/// JS truthiness of a JSON value (TS `input.context ? ... : null`).
fn js_truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0 && !f.is_nan()),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

/// A required string: present and non-empty after trimming. The value is returned untrimmed.
fn required<'a>(value: Option<&'a str>, field: &str) -> Result<&'a str, ForgeError> {
    match value {
        Some(s) if !s.trim().is_empty() => Ok(s),
        _ => Err(ForgeError::Validation {
            message: format!("{field} is required"),
            field: Some(field.to_string()),
        }),
    }
}

/// Data access for `extension_logs` (append-only error reports).
pub struct ExtensionLogStore;

impl ExtensionLogStore {
    pub fn append(
        conn: &Connection,
        input: &CreateExtensionLog,
    ) -> Result<ExtensionLog, ForgeError> {
        let error_code = required(input.error_code.as_deref(), "error_code")?;
        let message = required(input.message.as_deref(), "message")?;
        let layer = required(input.layer.as_deref(), "layer")?;
        let context = match &input.context {
            Some(v) if js_truthy(v) => {
                Some(serde_json::to_string(v).map_err(|e| ForgeError::Internal(e.to_string()))?)
            }
            _ => None,
        };
        let id = new_id();
        conn.execute(
            "INSERT INTO extension_logs (id, error_code, message, layer, plugin, url, context, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                id,
                error_code,
                message,
                layer,
                input.plugin,
                input.url,
                context,
                now_iso()
            ],
        )?;
        Self::get(conn, &id)?
            .ok_or_else(|| ForgeError::Internal("Extension log created but not found".into()))
    }

    pub fn get(conn: &Connection, id: &str) -> Result<Option<ExtensionLog>, ForgeError> {
        let raw = conn
            .query_row(
                &format!("SELECT {LOG_COLUMNS} FROM extension_logs WHERE id = ?1"),
                params![id],
                RawLog::from_row,
            )
            .optional()?;
        raw.map(RawLog::parse).transpose()
    }

    /// Newest first; `rowid` breaks ties within one second (TS `list`).
    pub fn list(
        conn: &Connection,
        filter: &ExtensionLogFilter,
    ) -> Result<Vec<ExtensionLog>, ForgeError> {
        let error_code = filter.error_code.as_deref().filter(|s| !s.is_empty());
        let layer = filter.layer.as_deref().filter(|s| !s.is_empty());
        let mut stmt = conn.prepare(&format!(
            "SELECT {LOG_COLUMNS} FROM extension_logs
             WHERE (?1 IS NULL OR error_code = ?1) AND (?2 IS NULL OR layer = ?2)
             ORDER BY created_at DESC, rowid DESC
             LIMIT ?3 OFFSET ?4"
        ))?;
        let raws = stmt
            .query_map(
                params![
                    error_code,
                    layer,
                    filter.limit.unwrap_or(50),
                    filter.offset.unwrap_or(0)
                ],
                RawLog::from_row,
            )?
            .collect::<Result<Vec<_>, _>>()?;
        raws.into_iter().map(RawLog::parse).collect()
    }

    /// Delete every log row. TS caps this at 10,000 rows; Rust doesn't (resu-mx/forge#114).
    pub fn clear(conn: &Connection) -> Result<usize, ForgeError> {
        Ok(conn.execute("DELETE FROM extension_logs", [])?)
    }
}

/// A row before `context` is parsed: a JSON error is a `ForgeError`, not a `rusqlite::Error`.
struct RawLog {
    log: ExtensionLog,
    context: Option<String>,
}

impl RawLog {
    fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            log: ExtensionLog {
                id: row.get(0)?,
                error_code: row.get(1)?,
                message: row.get(2)?,
                layer: row.get(3)?,
                plugin: row.get(4)?,
                url: row.get(5)?,
                context: None,
                created_at: row.get(7)?,
            },
            context: row.get(6)?,
        })
    }

    /// TS `parseRow`: `r.context ? JSON.parse(r.context) : null`.
    fn parse(self) -> Result<ExtensionLog, ForgeError> {
        let context = match self.context.filter(|s| !s.is_empty()) {
            Some(text) => Some(serde_json::from_str(&text).map_err(|e| {
                ForgeError::Internal(format!("extension_logs.context is not valid JSON: {e}"))
            })?),
            None => None,
        };
        Ok(ExtensionLog {
            context,
            ..self.log
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::Forge;
    use serde_json::json;

    fn setup() -> Forge {
        Forge::open_memory().unwrap()
    }

    fn input(code: &str, msg: &str, layer: &str) -> CreateExtensionLog {
        CreateExtensionLog {
            error_code: Some(code.into()),
            message: Some(msg.into()),
            layer: Some(layer.into()),
            ..Default::default()
        }
    }

    fn count(conn: &Connection, table: &str) -> i64 {
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .unwrap()
    }

    // ── config ──

    #[test]
    fn get_all_returns_defaults_on_fresh_db() {
        let f = setup();
        assert_eq!(
            ExtensionConfigStore::get_all(f.conn()).unwrap(),
            ExtensionConfig::default()
        );
    }

    #[test]
    fn get_all_defaults_for_deleted_row_and_ignores_other_keys() {
        let f = setup();
        f.conn()
            .execute("DELETE FROM extension_config WHERE key = 'devMode'", [])
            .unwrap();
        f.conn()
            .execute(
                "INSERT INTO extension_config (key, value) VALUES ('other', '123')",
                [],
            )
            .unwrap();
        assert_eq!(
            ExtensionConfigStore::get_all(f.conn()).unwrap(),
            ExtensionConfig::default()
        );
    }

    #[test]
    fn get_all_returns_stored_values_of_any_json_type() {
        let f = setup();
        f.conn()
            .execute(
                "UPDATE extension_config SET value = '42' WHERE key = 'baseUrl'",
                [],
            )
            .unwrap();
        let c = ExtensionConfigStore::get_all(f.conn()).unwrap();
        assert_eq!(c.base_url, json!(42));
    }

    #[test]
    fn get_all_invalid_json_is_internal() {
        let f = setup();
        f.conn()
            .execute(
                "UPDATE extension_config SET value = 'not json' WHERE key = 'baseUrl'",
                [],
            )
            .unwrap();
        assert!(matches!(
            ExtensionConfigStore::get_all(f.conn()),
            Err(ForgeError::Internal(_))
        ));
    }

    #[test]
    fn set_many_upserts_and_stores_json_text() {
        let f = setup();
        let c = ExtensionConfigStore::set_many(
            f.conn(),
            &[
                ("baseUrl".into(), json!("http://x")),
                ("devMode".into(), json!(true)),
                ("enabledPlugins".into(), json!(["a"])),
            ],
        )
        .unwrap();
        assert_eq!(c.base_url, json!("http://x"));
        assert_eq!(c.dev_mode, json!(true));
        assert_eq!(c.enabled_plugins, json!(["a"]));
        let raw: String = f
            .conn()
            .query_row(
                "SELECT value FROM extension_config WHERE key = 'baseUrl'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(raw, "\"http://x\"");
    }

    #[test]
    fn set_many_unknown_key_writes_nothing() {
        let f = setup();
        let err = ExtensionConfigStore::set_many(
            f.conn(),
            &[("devMode".into(), json!(true)), ("badKey".into(), json!(1))],
        )
        .unwrap_err();
        match err {
            ForgeError::Validation { message, field } => {
                assert_eq!(message, "Unknown config key: badKey");
                assert_eq!(field.as_deref(), Some("badKey"));
            }
            e => panic!("unexpected {e:?}"),
        }
        assert_eq!(
            ExtensionConfigStore::get_all(f.conn()).unwrap().dev_mode,
            json!(false)
        );
    }

    // ── logs ──

    #[test]
    fn append_validates_in_order_and_writes_nothing() {
        let f = setup();
        let cases = [
            (CreateExtensionLog::default(), "error_code is required"),
            (input("  ", "m", "l"), "error_code is required"),
            (input("c", "", "l"), "message is required"),
            (input("c", "m", " "), "layer is required"),
        ];
        for (inp, want) in cases {
            match ExtensionLogStore::append(f.conn(), &inp).unwrap_err() {
                ForgeError::Validation { message, .. } => assert_eq!(message, want),
                e => panic!("unexpected {e:?}"),
            }
        }
        assert_eq!(count(f.conn(), "extension_logs"), 0);
    }

    #[test]
    fn append_stores_untrimmed_iso_timestamp_and_round_trips() {
        let f = setup();
        let mut inp = input(" E1 ", "boom", "content");
        inp.plugin = Some(String::new());
        inp.context = Some(json!({"a": 1}));
        let log = ExtensionLogStore::append(f.conn(), &inp).unwrap();
        assert_eq!(log.error_code, " E1 ");
        assert_eq!(log.plugin.as_deref(), Some(""));
        assert_eq!(log.url, None);
        assert_eq!(log.context, Some(json!({"a": 1})));
        let b = log.created_at.as_bytes();
        assert_eq!(log.created_at.len(), 20);
        assert!(b[10] == b'T' && b[19] == b'Z');
        assert_eq!(
            ExtensionLogStore::get(f.conn(), &log.id)
                .unwrap()
                .unwrap()
                .id,
            log.id
        );
    }

    #[test]
    fn append_falsy_context_stores_null() {
        let f = setup();
        for v in [json!(null), json!(false), json!(0), json!("")] {
            let mut inp = input("c", "m", "l");
            inp.context = Some(v);
            let log = ExtensionLogStore::append(f.conn(), &inp).unwrap();
            assert_eq!(log.context, None);
        }
        let n: i64 = f
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM extension_logs WHERE context IS NULL",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 4);
    }

    #[test]
    fn list_defaults_newest_first_with_rowid_tiebreak() {
        let f = setup();
        for i in 0..3 {
            f.conn()
                .execute(
                    "INSERT INTO extension_logs (id, error_code, message, layer, created_at)
                     VALUES (?1, 'c', 'm', 'l', '2026-01-01T00:00:00Z')",
                    params![format!("id{i}")],
                )
                .unwrap();
        }
        f.conn()
            .execute(
                "INSERT INTO extension_logs (id, error_code, message, layer, created_at)
                 VALUES ('newer', 'c', 'm', 'l', '2026-01-02T00:00:00Z')",
                [],
            )
            .unwrap();
        let ids: Vec<_> = ExtensionLogStore::list(f.conn(), &Default::default())
            .unwrap()
            .into_iter()
            .map(|l| l.id)
            .collect();
        assert_eq!(ids, ["newer", "id2", "id1", "id0"]);
    }

    #[test]
    fn list_filters_and_pages() {
        let f = setup();
        for (c, l) in [("a", "x"), ("a", "y"), ("b", "x"), ("a", "x")] {
            ExtensionLogStore::append(f.conn(), &input(c, "m", l)).unwrap();
        }
        let filt = ExtensionLogFilter {
            error_code: Some("a".into()),
            layer: Some("x".into()),
            ..Default::default()
        };
        assert_eq!(ExtensionLogStore::list(f.conn(), &filt).unwrap().len(), 2);
        // empty filters are ignored
        let filt = ExtensionLogFilter {
            error_code: Some(String::new()),
            layer: Some(String::new()),
            ..Default::default()
        };
        assert_eq!(ExtensionLogStore::list(f.conn(), &filt).unwrap().len(), 4);
        let filt = ExtensionLogFilter {
            limit: Some(2),
            offset: Some(3),
            ..Default::default()
        };
        assert_eq!(ExtensionLogStore::list(f.conn(), &filt).unwrap().len(), 1);
        // negative limit means no limit
        let filt = ExtensionLogFilter {
            limit: Some(-1),
            ..Default::default()
        };
        assert_eq!(ExtensionLogStore::list(f.conn(), &filt).unwrap().len(), 4);
    }

    #[test]
    fn list_reads_ts_written_rows_and_rejects_bad_context() {
        let f = setup();
        f.conn()
            .execute(
                "INSERT INTO extension_logs (id, error_code, message, layer, context)
                 VALUES ('ts', 'c', 'm', 'l', '{\"k\":[1]}')",
                [],
            )
            .unwrap();
        let logs = ExtensionLogStore::list(f.conn(), &Default::default()).unwrap();
        assert_eq!(logs[0].context, Some(json!({"k": [1]})));
        f.conn()
            .execute("UPDATE extension_logs SET context = '{bad'", [])
            .unwrap();
        assert!(matches!(
            ExtensionLogStore::list(f.conn(), &Default::default()),
            Err(ForgeError::Internal(_))
        ));
    }

    #[test]
    fn clear_deletes_all_logs_and_only_logs() {
        let f = setup();
        for _ in 0..3 {
            ExtensionLogStore::append(f.conn(), &input("c", "m", "l")).unwrap();
        }
        let cfg_before = count(f.conn(), "extension_config");
        assert_eq!(ExtensionLogStore::clear(f.conn()).unwrap(), 3);
        assert_eq!(count(f.conn(), "extension_logs"), 0);
        assert_eq!(count(f.conn(), "extension_config"), cfg_before);
    }
}
