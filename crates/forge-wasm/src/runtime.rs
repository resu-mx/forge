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
use forge_sdk::Forge;
use serde_json::json;
use sqlite_wasm_vfs::sahpool::{install, OpfsSAHPoolCfg, OpfsSAHPoolUtil};
use wasm_bindgen::prelude::*;

use crate::dispatch::{dispatch, DispatchRequest};

/// The database file inside the OPFS pool.
const DB_NAME: &str = "forge.db";

/// First 16 bytes of every SQLite database file.
const SQLITE_HEADER: &[u8] = b"SQLite format 3\0";

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
    RUNTIME.with(|r| r.borrow().clone()).ok_or_else(|| JsValue::from_str("forge runtime not started"))
}

/// Count applied migrations, for the status report.
fn migrations_applied(forge: &Forge) -> i64 {
    forge
        .conn()
        .query_row("SELECT COUNT(*) FROM _migrations", [], |row| row.get(0))
        .unwrap_or(-1)
}

/// Install the OPFS VFS, open the database and build the router. Idempotent.
/// Resolves to a small JSON status report.
#[wasm_bindgen]
pub async fn start() -> Result<String, JsValue> {
    console_error_panic_hook::set_once();
    if RUNTIME.with(|r| r.borrow().is_some()) {
        return Ok(json!({ "already_started": true, "version": crate::BUNDLE_VERSION }).to_string());
    }

    let t0 = js_sys::Date::now();
    let util = install::<sqlite_wasm_rs::WasmOsCallback>(&OpfsSAHPoolCfg::default(), true)
        .await
        .map_err(|e| js_err("install OPFS storage", e))?;
    let t_vfs = js_sys::Date::now();

    let forge = Forge::open(DB_NAME).map_err(|e| js_err("open database", e))?;
    let migrations = migrations_applied(&forge);
    let t_open = js_sys::Date::now();

    let state = AppState::new(forge);
    let router = app(state.clone());
    RUNTIME.with(|r| *r.borrow_mut() = Some(Rc::new(Runtime { util, state, router })));

    Ok(json!({
        "version": crate::BUNDLE_VERSION,
        "migrations": migrations,
        "ms_install_storage": t_vfs - t0,
        "ms_open_and_migrate": t_open - t_vfs,
    })
    .to_string())
}

/// Serve one request. `headers_json` is `[["name","value"], …]`. Resolves to
/// `{ status, headers: [[name, value], …], body: Uint8Array }`.
///
/// Endpoints the Rust API does not serve yet answer 501 rather than 404.
#[wasm_bindgen]
pub async fn request(method: String, path: String, headers_json: String, body: Vec<u8>) -> Result<JsValue, JsValue> {
    let rt = runtime()?;
    let headers: Vec<(String, String)> =
        serde_json::from_str(&headers_json).map_err(|e| js_err("headers must be [[name, value], …]", e))?;

    let response = dispatch(&rt.router, DispatchRequest { method, path, headers, body }, true)
        .await
        .map_err(|e| JsValue::from_str(&e))?;

    let out = js_sys::Object::new();
    let set = |key: &str, value: JsValue| js_sys::Reflect::set(&out, &JsValue::from_str(key), &value);
    set("status", JsValue::from_f64(f64::from(response.status)))?;
    let headers = js_sys::Array::new();
    for (k, v) in response.headers {
        headers.push(&js_sys::Array::of2(&JsValue::from_str(&k), &JsValue::from_str(&v)));
    }
    set("headers", headers.into())?;
    set("body", js_sys::Uint8Array::from(response.body.as_slice()).into())?;
    Ok(out.into())
}

/// The whole database as a SQLite file, for backup or moving to another browser.
#[wasm_bindgen(js_name = exportDatabase)]
pub fn export_database() -> Result<Vec<u8>, JsValue> {
    let rt = runtime()?;
    rt.util.export_db(DB_NAME).map_err(|e| js_err("export database", e))
}

/// Replace the database with a SQLite file (for example a TS `forge.db`, checkpointed
/// first so no `-wal` file is needed). Migrations run on the imported data, so an
/// older database is brought up to date. On a bad file the current database is kept.
#[wasm_bindgen(js_name = importDatabase)]
pub fn import_database(bytes: Vec<u8>) -> Result<String, JsValue> {
    let rt = runtime()?;

    // Reject the obvious non-databases before touching any storage.
    if bytes.len() < SQLITE_HEADER.len() || &bytes[..SQLITE_HEADER.len()] != SQLITE_HEADER {
        return Err(JsValue::from_str("import rejected, current data kept: not a SQLite database file"));
    }

    // The pool will not import over an existing file, so the old one is deleted first.
    // Keep its bytes so a failure after that point can put everything back.
    let previous = rt.util.export_db(DB_NAME).map_err(|e| js_err("import (reading current data)", e))?;

    // Close the live connection before its file is touched.
    let placeholder = Forge::open_memory().map_err(|e| js_err("import", e))?;
    let old = rt.state.replace(placeholder).map_err(|e| js_err("import", e))?;
    drop(old);

    let imported = rt
        .util
        .delete_db(DB_NAME)
        .and_then(|_| rt.util.import_db(DB_NAME, &bytes));
    if let Err(e) = imported {
        // Restore what was there, then report the failure.
        let restored = rt.util.delete_db(DB_NAME).and_then(|_| rt.util.import_db(DB_NAME, &previous));
        let reopened = Forge::open(DB_NAME);
        return match (restored, reopened) {
            (Ok(_), Ok(forge)) => {
                let _ = rt.state.replace(forge);
                Err(js_err("import rejected, current data kept", e))
            }
            (restore, reopen) => Err(JsValue::from_str(&format!(
                "import failed ({e}) and the previous data could not be restored (restore: {:?}, reopen: {:?}); the exported copy in memory is lost on reload",
                restore.err().map(|e| e.to_string()),
                reopen.err().map(|e| e.to_string()),
            ))),
        };
    }

    let forge = Forge::open(DB_NAME).map_err(|e| js_err("could not open the imported database", e))?;
    let migrations = migrations_applied(&forge);
    let _ = rt.state.replace(forge);
    Ok(json!({ "imported_bytes": bytes.len(), "migrations": migrations }).to_string())
}
