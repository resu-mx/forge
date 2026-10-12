//! The native server end to end: a real socket, the real middleware stack, and
//! the `tokio` (blocking-pool) flavour of `with_conn`.

use forge_sdk::Forge;
use forge_server::CorsOrigins;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// Serve on an ephemeral port; returns the port.
async fn spawn(production: bool) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let app = forge_server::build_app(Forge::open_memory().unwrap(), production);
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    port
}

/// [`spawn`] with an explicit `FORGE_CORS_ORIGINS` policy.
async fn spawn_with_cors(production: bool, cors_origins: Option<CorsOrigins>) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let app =
        forge_server::build_app_with_cors(Forge::open_memory().unwrap(), production, cors_origins);
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    port
}

/// The `access-control-allow-origin` value in a lowercased header block, if any.
fn allow_origin(head: &str) -> Option<String> {
    head.lines()
        .find_map(|l| l.strip_prefix("access-control-allow-origin: "))
        .map(str::to_string)
}

/// Minimal HTTP/1.1 client: returns (status, lowercased headers block, body).
async fn request(
    port: u16,
    method: &str,
    path: &str,
    extra_headers: &str,
    body: &str,
) -> (u16, String, String) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    let req = format!(
        "{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n{extra_headers}Content-Length: {}\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(req.as_bytes()).await.unwrap();
    let mut raw = String::new();
    stream.read_to_string(&mut raw).await.unwrap();
    let (head, body) = raw.split_once("\r\n\r\n").unwrap_or((&raw, ""));
    let status = head.split_whitespace().nth(1).unwrap().parse().unwrap();
    // Chunked bodies carry size lines; the JSON payload is the line starting with '{' or '['.
    let body = body
        .lines()
        .find(|l| l.starts_with('{') || l.starts_with('['))
        .unwrap_or("")
        .to_string();
    (status, head.to_lowercase(), body)
}

#[tokio::test]
async fn serves_the_api_over_a_real_socket() {
    let port = spawn(false).await;

    let (status, head, body) = request(port, "GET", "/api/health", "", "").await;
    assert_eq!(status, 200);
    assert!(body.contains(r#""server":"ok""#), "{body}");
    assert!(
        head.contains("x-request-id:"),
        "request id header present: {head}"
    );

    let json_hdr = "Content-Type: application/json\r\n";
    let (status, _, body) = request(
        port,
        "POST",
        "/api/sources",
        json_hdr,
        r#"{"title":"T","description":"D"}"#,
    )
    .await;
    assert_eq!(status, 201, "{body}");

    let (status, _, body) = request(port, "GET", "/api/sources", "", "").await;
    assert_eq!(status, 200);
    assert!(body.contains(r#""total":1"#), "{body}");
}

#[tokio::test]
async fn request_ids_are_unique_per_request() {
    let port = spawn(false).await;
    let id = |head: &str| {
        head.lines()
            .find(|l| l.starts_with("x-request-id:"))
            .unwrap()
            .to_string()
    };
    let (_, a, _) = request(port, "GET", "/api/health", "", "").await;
    let (_, b, _) = request(port, "GET", "/api/health", "", "").await;
    assert_ne!(id(&a), id(&b));
}

#[tokio::test]
async fn dev_cors_allows_the_webui_and_extensions_only() {
    let port = spawn(false).await;
    let acao = |head: &str| {
        head.lines()
            .find(|l| l.starts_with("access-control-allow-origin:"))
            .map(str::to_string)
    };

    let (_, head, _) = request(
        port,
        "GET",
        "/api/health",
        "Origin: http://localhost:5173\r\n",
        "",
    )
    .await;
    assert_eq!(
        acao(&head).as_deref(),
        Some("access-control-allow-origin: http://localhost:5173")
    );

    let (_, head, _) = request(
        port,
        "GET",
        "/api/health",
        "Origin: chrome-extension://abc\r\n",
        "",
    )
    .await;
    assert!(acao(&head).is_some());

    let (_, head, _) = request(
        port,
        "GET",
        "/api/health",
        "Origin: http://evil.example\r\n",
        "",
    )
    .await;
    assert_eq!(acao(&head), None, "foreign origins get no CORS header");
}

#[tokio::test]
async fn production_cors_allows_any_origin() {
    let port = spawn(true).await;
    let (_, head, _) = request(
        port,
        "GET",
        "/api/health",
        "Origin: http://anything.example\r\n",
        "",
    )
    .await;
    assert!(head.contains("access-control-allow-origin: *"), "{head}");
}

#[tokio::test]
async fn preflight_is_answered() {
    let port = spawn(false).await;
    let hdrs = "Origin: http://localhost:5173\r\nAccess-Control-Request-Method: PATCH\r\nAccess-Control-Request-Headers: content-type\r\n";
    let (status, head, _) = request(port, "OPTIONS", "/api/sources/x", hdrs, "").await;
    assert!(status == 200 || status == 204, "preflight status {status}");
    assert!(head.contains("access-control-allow-methods:"), "{head}");
    assert!(head.contains("patch"), "{head}");
}

// ── FORGE_CORS_ORIGINS (resu-mx/forge#126) ──────────────────────────────────────────────────

/// `GET /api/health` from `origin`: the `access-control-allow-origin` it gets back.
async fn acao_for(port: u16, origin: &str) -> Option<String> {
    let hdrs = format!("Origin: {origin}\r\n");
    let (status, head, _) = request(port, "GET", "/api/health", &hdrs, "").await;
    assert_eq!(status, 200, "the request itself is served either way");
    allow_origin(&head)
}

#[tokio::test]
async fn configured_origins_replace_the_built_in_list() {
    let list = CorsOrigins::parse(Some("http://localhost:4173,chrome-extension://abc"));
    let port = spawn_with_cors(false, list).await;

    assert_eq!(
        acao_for(port, "http://localhost:4173").await.as_deref(),
        Some("http://localhost:4173")
    );
    assert_eq!(
        acao_for(port, "chrome-extension://abc").await.as_deref(),
        Some("chrome-extension://abc")
    );
    // The built-in dev origins are gone: the operator's list replaced them.
    assert_eq!(acao_for(port, "http://localhost:5173").await, None);
    assert_eq!(acao_for(port, "chrome-extension://other").await, None);
    assert_eq!(acao_for(port, "https://evil.example").await, None);

    // A preflight from an allowed origin is answered with a 204 and the allow header.
    let hdrs = "Origin: http://localhost:4173\r\nAccess-Control-Request-Method: POST\r\nAccess-Control-Request-Headers: content-type\r\n";
    let (status, head, _) = request(port, "OPTIONS", "/api/sources", hdrs, "").await;
    assert_eq!(status, 204);
    assert_eq!(
        allow_origin(&head).as_deref(),
        Some("http://localhost:4173")
    );
    // ...and from a refused one, with no allow header, so the browser blocks it.
    let hdrs = "Origin: https://evil.example\r\nAccess-Control-Request-Method: POST\r\n";
    let (_, head, _) = request(port, "OPTIONS", "/api/sources", hdrs, "").await;
    assert_eq!(allow_origin(&head), None);
}

#[tokio::test]
async fn a_configured_star_allows_every_origin() {
    for production in [false, true] {
        let port = spawn_with_cors(production, CorsOrigins::parse(Some("*"))).await;
        assert_eq!(
            acao_for(port, "https://anything.example").await.as_deref(),
            Some("*"),
            "production={production}"
        );
    }
}

#[tokio::test]
async fn a_configured_list_narrows_production_too() {
    // Production without the variable is `*` (see production_cors_allows_any_origin); with it,
    // only the listed origins.
    let port = spawn_with_cors(true, CorsOrigins::parse(Some("https://app.example"))).await;
    assert_eq!(
        acao_for(port, "https://app.example").await.as_deref(),
        Some("https://app.example")
    );
    assert_eq!(acao_for(port, "https://other.example").await, None);
}

#[tokio::test]
async fn a_blank_variable_keeps_the_defaults() {
    // Blank parses to "not configured", so the dev defaults are untouched.
    let port = spawn_with_cors(false, CorsOrigins::parse(Some(" , "))).await;
    assert_eq!(
        acao_for(port, "http://localhost:5173").await.as_deref(),
        Some("http://localhost:5173")
    );
    assert_eq!(acao_for(port, "https://evil.example").await, None);
}
