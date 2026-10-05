//! Native host for the Forge API: configuration, middleware and the listener.
//!
//! The routes themselves live in `forge-api` so the same router can run in a
//! browser Worker. Everything here is native-only (tokio, tower-http).

use std::any::Any;
use std::net::SocketAddr;
use std::path::PathBuf;

use axum::body::Body;
use axum::http::{header, HeaderValue, Method, Request, Response, StatusCode};
use axum::middleware::{from_fn, Next};
use axum::Router;
use forge_api::AppState;
use forge_sdk::db::DerivationStore;
use forge_sdk::Forge;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, Any as AnyOrigin, CorsLayer};
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::trace::TraceLayer;

/// Runtime configuration, read from the same environment variables as the TS server.
#[derive(Debug, Clone)]
pub struct Config {
    /// `FORGE_PORT`, default 3000.
    pub port: u16,
    /// `FORGE_HOST`, default 127.0.0.1. The TS server listens on all interfaces;
    /// this one is loopback-only until told otherwise because the API has no auth.
    pub host: String,
    /// `FORGE_DB_PATH`, required.
    pub db_path: PathBuf,
    /// `FORGE_ENV=production` (or `NODE_ENV=production`): CORS allows any origin.
    pub production: bool,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        Self::from_lookup(|k| std::env::var(k).ok())
    }

    /// Testable core of [`Config::from_env`].
    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Result<Self, String> {
        let db_path = get("FORGE_DB_PATH")
            .filter(|v| !v.is_empty())
            .ok_or("FORGE_DB_PATH is required. Set it to a path for the SQLite database file.")?;
        let port = match get("FORGE_PORT") {
            None => 3000,
            Some(raw) => raw.parse::<u16>().ok().filter(|p| *p >= 1).ok_or_else(|| {
                format!("FORGE_PORT must be a valid port number (1-65535), got: {raw}")
            })?,
        };
        let production =
            get("FORGE_ENV").or_else(|| get("NODE_ENV")).as_deref() == Some("production");
        Ok(Self {
            port,
            host: get("FORGE_HOST").unwrap_or_else(|| "127.0.0.1".into()),
            db_path: PathBuf::from(db_path),
            production,
        })
    }

    pub fn bind_addr(&self) -> Result<SocketAddr, String> {
        format!("{}:{}", self.host, self.port)
            .parse()
            .map_err(|e| format!("invalid FORGE_HOST/FORGE_PORT: {e}"))
    }
}

/// The API plus the middleware the TS server has: CORS, `X-Request-Id`, request
/// logging, and a JSON 500 for panics.
pub fn build_app(forge: Forge, production: bool) -> Router {
    forge_api::app(AppState::new(forge))
        .layer(cors(production))
        .layer(from_fn(preflight_no_content))
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(TraceLayer::new_for_http())
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(CatchPanicLayer::custom(panic_response))
}

/// tower-http answers a CORS preflight with 200; Hono (and so the TS server) with 204.
/// Clients should not care, but the contract tests do, so match it.
async fn preflight_no_content(req: Request<Body>, next: Next) -> Response<Body> {
    let is_preflight = req.method() == Method::OPTIONS
        && req
            .headers()
            .contains_key(header::ACCESS_CONTROL_REQUEST_METHOD);
    let mut resp = next.run(req).await;
    if is_preflight && resp.status() == StatusCode::OK {
        *resp.status_mut() = StatusCode::NO_CONTENT;
    }
    resp
}

fn cors(production: bool) -> CorsLayer {
    let origin = if production {
        AllowOrigin::from(AnyOrigin)
    } else {
        AllowOrigin::predicate(|origin: &HeaderValue, _| dev_origin_allowed(origin))
    };
    CorsLayer::new()
        .allow_origin(origin)
        .allow_methods(AllowMethods::list([
            Method::GET,
            Method::HEAD,
            Method::PUT,
            Method::POST,
            Method::DELETE,
            Method::PATCH,
        ]))
        .allow_headers(AllowHeaders::mirror_request())
}

/// Development: the web UI and browser-extension origins only.
fn dev_origin_allowed(origin: &HeaderValue) -> bool {
    let Ok(origin) = origin.to_str() else {
        return false;
    };
    matches!(origin, "http://localhost:5173" | "http://127.0.0.1:5173")
        || origin.starts_with("chrome-extension://")
        || origin.starts_with("moz-extension://")
}

fn panic_response(_: Box<dyn Any + Send + 'static>) -> Response<Body> {
    tracing::error!("handler panicked");
    let body = serde_json::json!({
        "error": { "code": "INTERNAL_ERROR", "message": "Internal server error" }
    });
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("static response parts are valid")
}

/// Open the database (running migrations), clear expired derivation locks, and serve.
pub async fn run(config: Config) -> anyhow::Result<()> {
    if let Some(dir) = config
        .db_path
        .parent()
        .filter(|d| !d.as_os_str().is_empty())
    {
        std::fs::create_dir_all(dir)?;
    }
    let path = config
        .db_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("FORGE_DB_PATH is not valid UTF-8"))?;
    let forge = Forge::open(path)?;
    tracing::info!(path, "database ready");

    let expired = DerivationStore::cleanup_expired(forge.conn())?;
    if expired > 0 {
        tracing::warn!(count = expired, "removed expired derivation locks");
    }

    let addr = config.bind_addr().map_err(anyhow::Error::msg)?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, production = config.production, "forge-server listening");

    axum::serve(listener, build_app(forge, config.production))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn cfg(vars: &[(&str, &str)]) -> Result<Config, String> {
        let map: HashMap<String, String> = vars
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Config::from_lookup(|k| map.get(k).cloned())
    }

    #[test]
    fn db_path_is_required() {
        assert!(cfg(&[]).unwrap_err().contains("FORGE_DB_PATH is required"));
        assert!(cfg(&[("FORGE_DB_PATH", "")]).is_err());
    }

    #[test]
    fn defaults() {
        let c = cfg(&[("FORGE_DB_PATH", "/tmp/x.db")]).unwrap();
        assert_eq!(
            (c.port, c.host.as_str(), c.production),
            (3000, "127.0.0.1", false)
        );
    }

    #[test]
    fn port_is_validated() {
        for bad in ["0", "70000", "abc", "-1"] {
            assert!(
                cfg(&[("FORGE_DB_PATH", "x"), ("FORGE_PORT", bad)]).is_err(),
                "{bad}"
            );
        }
        assert_eq!(
            cfg(&[("FORGE_DB_PATH", "x"), ("FORGE_PORT", "8080")])
                .unwrap()
                .port,
            8080
        );
    }

    #[test]
    fn production_flag_from_either_variable() {
        assert!(
            cfg(&[("FORGE_DB_PATH", "x"), ("FORGE_ENV", "production")])
                .unwrap()
                .production
        );
        assert!(
            cfg(&[("FORGE_DB_PATH", "x"), ("NODE_ENV", "production")])
                .unwrap()
                .production
        );
        assert!(
            !cfg(&[("FORGE_DB_PATH", "x"), ("NODE_ENV", "development")])
                .unwrap()
                .production
        );
    }

    #[test]
    fn dev_cors_origins() {
        let ok = |s: &str| dev_origin_allowed(&HeaderValue::from_str(s).unwrap());
        assert!(ok("http://localhost:5173"));
        assert!(ok("http://127.0.0.1:5173"));
        assert!(ok("chrome-extension://abcdef"));
        assert!(ok("moz-extension://abcdef"));
        assert!(!ok("http://evil.example"));
        assert!(!ok("http://localhost:5174"));
    }
}
