//! M1 spike: prove forge-sdk (rusqlite) + an axum Router run inside a browser Worker.
//!
//! Gates exercised (see .agents/plans/rust-mvp-roadmap.md, M1):
//!   2. forge-sdk compiles for wasm32 and runs.
//!   3. sqlite-wasm-vfs sahpool (OPFS) persists a database across reloads.
//!   4. an axum `Router` (no tokio) answers a request through `oneshot`.

use std::cell::RefCell;

use axum::body::Body;
use axum::http::Request;
use axum::routing::get;
use axum::{Json, Router};
use forge_core::CreateSource;
use forge_sdk::db::SourceStore;
use forge_sdk::Forge;
use serde_json::{json, Value};
use sqlite_wasm_vfs::sahpool::{install, OpfsSAHPoolCfg};
use tower::ServiceExt;
use wasm_bindgen::prelude::*;

thread_local! {
    static FORGE: RefCell<Option<Forge>> = const { RefCell::new(None) };
}

fn js_err<E: std::fmt::Display>(what: &str, e: E) -> JsValue {
    JsValue::from_str(&format!("{what}: {e}"))
}

fn with_forge<T>(f: impl FnOnce(&Forge) -> T) -> Result<T, String> {
    FORGE.with(|cell| match cell.borrow().as_ref() {
        Some(forge) => Ok(f(forge)),
        None => Err("forge not initialised".to_string()),
    })
}

/// Install the OPFS VFS as the default, open forge.db through the unmodified
/// `Forge::open` (which runs all migrations), and report what happened.
#[wasm_bindgen]
pub async fn init() -> Result<String, JsValue> {
    console_error_panic_hook::set_once();
    let t0 = js_sys::Date::now();

    install::<sqlite_wasm_rs::WasmOsCallback>(&OpfsSAHPoolCfg::default(), true)
        .await
        .map_err(|e| js_err("install sahpool vfs", e))?;
    let t_vfs = js_sys::Date::now();

    let forge = Forge::open("forge.db").map_err(|e| js_err("Forge::open", e))?;
    let t_open = js_sys::Date::now();

    let conn = forge.conn();
    let migrations: i64 = conn
        .query_row("SELECT count(*) FROM _migrations", [], |r| r.get(0))
        .map_err(|e| js_err("count migrations", e))?;
    let journal_mode: String = conn
        .query_row("PRAGMA journal_mode", [], |r| r.get(0))
        .map_err(|e| js_err("journal_mode", e))?;
    let sqlite_version: String = conn
        .query_row("SELECT sqlite_version()", [], |r| r.get(0))
        .map_err(|e| js_err("sqlite_version", e))?;
    let foreign_keys: i64 = conn
        .query_row("PRAGMA foreign_keys", [], |r| r.get(0))
        .map_err(|e| js_err("foreign_keys", e))?;
    let sources: i64 = conn
        .query_row("SELECT count(*) FROM sources", [], |r| r.get(0))
        .map_err(|e| js_err("count sources", e))?;

    FORGE.with(|cell| *cell.borrow_mut() = Some(forge));

    Ok(json!({
        "sqlite_version": sqlite_version,
        "journal_mode": journal_mode,
        "foreign_keys": foreign_keys,
        "migrations": migrations,
        "sources_at_open": sources,
        "ms_install_vfs": t_vfs - t0,
        "ms_open_and_migrate": t_open - t_vfs,
    })
    .to_string())
}

fn api_router() -> Router {
    Router::new()
        .route(
            "/api/health",
            get(|| async { Json(json!({ "data": { "server": "ok", "runtime": "wasm-worker" } })) }),
        )
        .route(
            "/api/sources",
            get(list_sources).post(create_source),
        )
}

async fn list_sources() -> Json<Value> {
    let rows = with_forge(|f| {
        let mut stmt = f
            .conn()
            .prepare("SELECT id, title FROM sources ORDER BY created_at, id")
            .unwrap();
        stmt.query_map([], |r| Ok(json!({ "id": r.get::<_, String>(0)?, "title": r.get::<_, String>(1)? })))
            .unwrap()
            .map(|r| r.unwrap())
            .collect::<Vec<_>>()
    })
    .unwrap_or_default();
    Json(json!({ "data": rows, "pagination": { "total": rows.len() } }))
}

async fn create_source(Json(input): Json<CreateSource>) -> (axum::http::StatusCode, Json<Value>) {
    match with_forge(|f| SourceStore::create(f.conn(), &input)) {
        Ok(Ok(created)) => (
            axum::http::StatusCode::CREATED,
            Json(json!({ "data": created })),
        ),
        Ok(Err(e)) => (
            axum::http::StatusCode::BAD_REQUEST,
            Json(json!({ "error": { "code": "STORE_ERROR", "message": e.to_string() } })),
        ),
        Err(msg) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": { "code": "NOT_READY", "message": msg } })),
        ),
    }
}

/// Dispatch one request through the axum Router. Returns `{status, body}` as JSON.
#[wasm_bindgen]
pub async fn dispatch(method: String, path: String, body: String) -> Result<String, JsValue> {
    let req = Request::builder()
        .method(method.as_str())
        .uri(path.as_str())
        .header("content-type", "application/json")
        .body(Body::from(body))
        .map_err(|e| js_err("build request", e))?;

    let t0 = js_sys::Date::now();
    // The router's error type is Infallible, so this pattern is irrefutable.
    let Ok(resp) = api_router().oneshot(req).await;
    let status = resp.status().as_u16();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .map_err(|e| js_err("read body", e))?;
    let text = String::from_utf8_lossy(&bytes).into_owned();

    Ok(json!({ "status": status, "ms": js_sys::Date::now() - t0, "body": text }).to_string())
}
