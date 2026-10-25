//! The browser runtime: OPFS-backed SQLite, the real `forge-sdk`, and the
//! `forge-api` router, exposed to a Worker through four coarse-grained calls.
//!
//! - [`start`] installs the OPFS VFS, opens `forge.db` (running migrations through
//!   the unmodified `Forge::open`) and builds the router.
//! - [`request`] serves one HTTP-shaped request.
//! - [`export_database`] / [`import_database`] move the whole database as bytes.
//!
//! Must run in a **dedicated Worker**: the OPFS VFS needs
//! `FileSystemSyncAccessHandle`, which exists only there (ADR 0001/0002). Everything
//! here is single-threaded, so the thread-local state is the whole of the state.

use std::cell::RefCell;
use std::rc::Rc;

use axum::Router;
use forge_api::{app, AppState, SharedState};
use forge_sdk::db::migrate::schema_head;
use forge_sdk::Forge;
use serde_json::json;
use sqlite_wasm_vfs::sahpool::{install, OpfsSAHPoolCfg, OpfsSAHPoolUtil};
use wasm_bindgen::prelude::*;

use crate::dispatch::{dispatch, DispatchRequest};
use crate::snapshot::{check_import, head_of_bytes, schema_report};

/// The database file inside the OPFS pool.
const DB_NAME: &str = "forge.db";

struct Runtime {
    util: OpfsSAHPoolUtil,
    state: SharedState,
    router: Router,
}

thread_local! {
    static RUNTIME: RefCell<Option<Rc<Runtime>>> = const { RefCell::new(None) };
}

fn js_err<E: std::fmt::Display>(what: &str, e: E) -> JsValue {
    JsValue::from_str(&format!("{what}: {e}"))
}

fn runtime() -> Result<Rc<Runtime>, JsValue> {
    RUNTIME
        .with(|r| r.borrow().clone())
        .ok_or_else(|| JsValue::from_str("forge runtime not started"))
}

/// Count applied migrations, for the status report.
fn migrations_applied(forge: &Forge) -> i64 {
    forge
        .conn()
        .query_row("SELECT COUNT(*) FROM _migrations", [], |row| row.get(0))
        .unwrap_or(-1)
}

/// The newest migration applied to the open database, or `None` if it cannot be read.
fn live_schema_head(forge: &Forge) -> Option<String> {
    schema_head(forge.conn()).ok().flatten()
}

/// Install the OPFS VFS, open the database and build the router. Idempotent.
/// Resolves to a small JSON status report: `version` (the bundle version, also reported as
/// `bundle_version`), `migrations`, the schema heads, and timings. A second call reports
/// `already_started` and this bundle's own versions; the database's `schema_head` is only
/// read when the database is opened.
#[wasm_bindgen]
pub async fn start() -> Result<String, JsValue> {
    console_error_panic_hook::set_once();
    if RUNTIME.with(|r| r.borrow().is_some()) {
        let mut report = schema_report(None);
        report.remove("schema_head");
        report.insert("already_started".into(), json!(true));
        report.insert("version".into(), json!(crate::BUNDLE_VERSION));
        return Ok(serde_json::Value::Object(report).to_string());
    }

    let t0 = js_sys::Date::now();
    let util = install::<sqlite_wasm_rs::WasmOsCallback>(&OpfsSAHPoolCfg::default(), true)
        .await
        .map_err(|e| js_err("install OPFS storage", e))?;
    let t_vfs = js_sys::Date::now();

    let forge = Forge::open(DB_NAME).map_err(|e| js_err("open database", e))?;
    let migrations = migrations_applied(&forge);
    let head = live_schema_head(&forge);
    let t_open = js_sys::Date::now();

    let state = AppState::new(forge);
    let router = app(state.clone());
    RUNTIME.with(|r| {
        *r.borrow_mut() = Some(Rc::new(Runtime {
            util,
            state,
            router,
        }))
    });

    let mut report = schema_report(head.as_deref());
    report.insert("version".into(), json!(crate::BUNDLE_VERSION));
    report.insert("migrations".into(), json!(migrations));
    report.insert("ms_install_storage".into(), json!(t_vfs - t0));
    report.insert("ms_open_and_migrate".into(), json!(t_open - t_vfs));
    Ok(serde_json::Value::Object(report).to_string())
}

/// Serve one request. `headers_json` is `[["name","value"], …]`. Resolves to
/// `{ status, headers: [[name, value], …], body: Uint8Array }`.
///
/// Endpoints the Rust API does not serve yet answer 501 rather than 404.
#[wasm_bindgen]
pub async fn request(
    method: String,
    path: String,
    headers_json: String,
    body: Vec<u8>,
) -> Result<JsValue, JsValue> {
    let rt = runtime()?;
    let headers: Vec<(String, String)> = serde_json::from_str(&headers_json)
        .map_err(|e| js_err("headers must be [[name, value], …]", e))?;

    let response = dispatch(
        &rt.router,
        DispatchRequest {
            method,
            path,
            headers,
            body,
        },
        true,
    )
    .await
    .map_err(|e| JsValue::from_str(&e))?;

    let out = js_sys::Object::new();
    let set =
        |key: &str, value: JsValue| js_sys::Reflect::set(&out, &JsValue::from_str(key), &value);
    set("status", JsValue::from_f64(f64::from(response.status)))?;
    let headers = js_sys::Array::new();
    for (k, v) in response.headers {
        headers.push(&js_sys::Array::of2(
            &JsValue::from_str(&k),
            &JsValue::from_str(&v),
        ));
    }
    set("headers", headers.into())?;
    set(
        "body",
        js_sys::Uint8Array::from(response.body.as_slice()).into(),
    )?;
    Ok(out.into())
}

/// The whole database as a SQLite file, for backup, moving to another browser, or a cloud
/// snapshot. Resolves to `{ bytes, schema_head, known_head, bundle_version }`: the file plus
/// what a snapshot upload must declare about it. `schema_head` is read from the exported
/// bytes themselves and is `null` when they cannot be read as a database (the bytes are
/// returned regardless, so a damaged database can still be saved).
#[wasm_bindgen(js_name = exportDatabase)]
pub fn export_database() -> Result<JsValue, JsValue> {
    let rt = runtime()?;
    let bytes = rt
        .util
        .export_db(DB_NAME)
        .map_err(|e| js_err("export database", e))?;
    let head = head_of_bytes(&bytes).ok().flatten();

    let out = js_sys::Object::new();
    let set =
        |key: &str, value: JsValue| js_sys::Reflect::set(&out, &JsValue::from_str(key), &value);
    set("bytes", js_sys::Uint8Array::from(bytes.as_slice()).into())?;
    set(
        "schema_head",
        head.as_deref().map_or(JsValue::NULL, JsValue::from_str),
    )?;
    set(
        "known_head",
        JsValue::from_str(forge_sdk::db::migrate::known_head()),
    )?;
    set("bundle_version", JsValue::from_str(crate::BUNDLE_VERSION))?;
    Ok(out.into())
}

/// Put `previous` back after a failed import and report `cause`. If even that fails the
/// message says so, because then the data exists only in the caller's copy.
fn restore_previous(rt: &Runtime, previous: &[u8], cause: impl std::fmt::Display) -> JsValue {
    let restored = rt
        .util
        .delete_db(DB_NAME)
        .and_then(|_| rt.util.import_db(DB_NAME, previous));
    let reopened = Forge::open(DB_NAME);
    match (restored, reopened) {
        (Ok(_), Ok(forge)) => {
            let _ = rt.state.replace(forge);
            js_err("import rejected, current data kept", cause)
        }
        (restore, reopen) => JsValue::from_str(&format!(
            "import failed ({cause}) and the previous data could not be restored (restore: {:?}, reopen: {:?}); the exported copy in memory is lost on reload",
            restore.err().map(|e| e.to_string()),
            reopen.err().map(|e| e.to_string()),
        )),
    }
}

/// Replace the database with a SQLite file (for example a TS `forge.db`, checkpointed
/// first so no `-wal` file is needed). Migrations run on the imported data, so an
/// older database is brought up to date. On a bad file the current database is kept.
///
/// A file written by a newer bundle (its newest migration is past this bundle's) is
/// refused. That is decided from the file's bytes, in memory, before any storage is
/// touched. A refusal is thrown as JSON `{ code, message, known_head, schema_head? }` with
/// `code` one of `not_sqlite`, `invalid_database` or `schema_too_new`. Resolves to
/// `{ imported_bytes, migrations, schema_head, known_head, bundle_version }`, where
/// `schema_head` is the database's newest migration once it has been brought up to date.
#[wasm_bindgen(js_name = importDatabase)]
pub fn import_database(bytes: Vec<u8>) -> Result<String, JsValue> {
    let rt = runtime()?;

    // Refuse what this bundle must not open before touching any storage.
    check_import(&bytes).map_err(|refusal| JsValue::from_str(&refusal.to_json()))?;

    // The pool will not import over an existing file, so the old one is deleted first.
    // Keep its bytes so a failure after that point can put everything back.
    let previous = rt
        .util
        .export_db(DB_NAME)
        .map_err(|e| js_err("import (reading current data)", e))?;

    // Close the live connection before its file is touched.
    let placeholder = Forge::open_memory().map_err(|e| js_err("import", e))?;
    let old = rt
        .state
        .replace(placeholder)
        .map_err(|e| js_err("import", e))?;
    drop(old);

    let imported = rt
        .util
        .delete_db(DB_NAME)
        .and_then(|_| rt.util.import_db(DB_NAME, &bytes));
    if let Err(e) = imported {
        return Err(restore_previous(&rt, &previous, e));
    }

    // Opening runs the migrations; if they fail on this file, put the old data back too.
    let forge = match Forge::open(DB_NAME) {
        Ok(forge) => forge,
        Err(e) => {
            return Err(restore_previous(
                &rt,
                &previous,
                format!("could not open the imported database: {e}"),
            ))
        }
    };
    let migrations = migrations_applied(&forge);
    let head = live_schema_head(&forge);
    let _ = rt.state.replace(forge);

    let mut report = schema_report(head.as_deref());
    report.insert("imported_bytes".into(), json!(bytes.len()));
    report.insert("migrations".into(), json!(migrations));
    Ok(serde_json::Value::Object(report).to_string())
}
