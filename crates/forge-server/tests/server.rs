//! The native server end to end: a real socket, the real middleware stack, and
//! the `tokio` (blocking-pool) flavour of `with_conn`.

use forge_sdk::Forge;
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

/// Minimal HTTP/1.1 client: returns (status, lowercased headers block, body).
async fn request(port: u16, method: &str, path: &str, extra_headers: &str, body: &str) -> (u16, String, String) {
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
    let body = body.lines().find(|l| l.starts_with('{') || l.starts_with('[')).unwrap_or("").to_string();
    (status, head.to_lowercase(), body)
}

#[tokio::test]
async fn serves_the_api_over_a_real_socket() {
    let port = spawn(false).await;

    let (status, head, body) = request(port, "GET", "/api/health", "", "").await;
    assert_eq!(status, 200);
    assert!(body.contains(r#""server":"ok""#), "{body}");
    assert!(head.contains("x-request-id:"), "request id header present: {head}");

    let json_hdr = "Content-Type: application/json\r\n";
    let (status, _, body) =
        request(port, "POST", "/api/sources", json_hdr, r#"{"title":"T","description":"D"}"#).await;
    assert_eq!(status, 201, "{body}");

    let (status, _, body) = request(port, "GET", "/api/sources", "", "").await;
    assert_eq!(status, 200);
    assert!(body.contains(r#""total":1"#), "{body}");
}

#[tokio::test]
async fn request_ids_are_unique_per_request() {
    let port = spawn(false).await;
    let id = |head: &str| head.lines().find(|l| l.starts_with("x-request-id:")).unwrap().to_string();
    let (_, a, _) = request(port, "GET", "/api/health", "", "").await;
    let (_, b, _) = request(port, "GET", "/api/health", "", "").await;
    assert_ne!(id(&a), id(&b));
}

#[tokio::test]
async fn dev_cors_allows_the_webui_and_extensions_only() {
    let port = spawn(false).await;
    let acao = |head: &str| head.lines().find(|l| l.starts_with("access-control-allow-origin:")).map(str::to_string);

    let (_, head, _) = request(port, "GET", "/api/health", "Origin: http://localhost:5173\r\n", "").await;
    assert_eq!(acao(&head).as_deref(), Some("access-control-allow-origin: http://localhost:5173"));

    let (_, head, _) = request(port, "GET", "/api/health", "Origin: chrome-extension://abc\r\n", "").await;
    assert!(acao(&head).is_some());

    let (_, head, _) = request(port, "GET", "/api/health", "Origin: http://evil.example\r\n", "").await;
    assert_eq!(acao(&head), None, "foreign origins get no CORS header");
}

#[tokio::test]
async fn production_cors_allows_any_origin() {
    let port = spawn(true).await;
    let (_, head, _) = request(port, "GET", "/api/health", "Origin: http://anything.example\r\n", "").await;
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
