//! Serve one HTTP-shaped request through the `forge-api` router, with no socket.
//!
//! This is the whole "transport" of the browser runtime: the TypeScript SDK builds a
//! normal request, a Worker hands it here, and the same axum router that
//! `forge-server` runs answers it. It is host-independent (no wasm-bindgen, no
//! OPFS), so it is tested natively.

use axum::body::Body;
use axum::http::{header, HeaderName, HeaderValue, Method, Request};
use axum::Router;
use tower::ServiceExt;

/// Largest request or response body accepted (a database import is the big one).
const MAX_BODY: usize = 256 * 1024 * 1024;

/// A request as the JS side describes it.
#[derive(Debug, Clone, Default)]
pub struct DispatchRequest {
    pub method: String,
    /// Path and query, e.g. `/api/sources?limit=5`.
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// The router's answer, ready to become a `Response` on the JS side.
#[derive(Debug, Clone)]
pub struct DispatchResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// Run `req` through `router`.
///
/// With `unported_as_501`, a "Route not found" 404 is rewritten to
/// `501 NOT_IMPLEMENTED`: in the browser, an endpoint that the Rust API does not
/// serve yet is "not available here", not a typo, and the UI should say so.
pub async fn dispatch(
    router: &Router,
    req: DispatchRequest,
    unported_as_501: bool,
) -> Result<DispatchResponse, String> {
    let method = Method::from_bytes(req.method.to_uppercase().as_bytes())
        .map_err(|e| format!("invalid method {:?}: {e}", req.method))?;

    let mut builder = Request::builder().method(method.clone()).uri(&req.path);
    for (name, value) in &req.headers {
        // The Fetch API owns these; forwarding them would only confuse axum.
        if matches!(
            name.to_ascii_lowercase().as_str(),
            "host" | "content-length" | "connection"
        ) {
            continue;
        }
        let name = HeaderName::from_bytes(name.as_bytes())
            .map_err(|e| format!("invalid header {name:?}: {e}"))?;
        let value = HeaderValue::from_str(value)
            .map_err(|e| format!("invalid header value for {name}: {e}"))?;
        builder = builder.header(name, value);
    }
    let request = builder
        .body(Body::from(req.body))
        .map_err(|e| format!("invalid request: {e}"))?;

    // The router's error type is Infallible, so this pattern is irrefutable.
    let Ok(response) = router.clone().oneshot(request).await;
    let status = response.status().as_u16();
    let headers: Vec<(String, String)> = response
        .headers()
        .iter()
        .filter_map(|(k, v)| {
            v.to_str()
                .ok()
                .map(|v| (k.as_str().to_string(), v.to_string()))
        })
        .collect();
    let body = axum::body::to_bytes(response.into_body(), MAX_BODY)
        .await
        .map_err(|e| format!("reading response body: {e}"))?
        .to_vec();

    if unported_as_501 && status == 404 && is_route_not_found(&body) {
        let message = format!(
            "{method} {} is not available in the browser runtime yet",
            req.path
        );
        let body =
            serde_json::json!({ "error": { "code": "NOT_IMPLEMENTED", "message": message } })
                .to_string()
                .into_bytes();
        return Ok(DispatchResponse {
            status: 501,
            headers: vec![(
                header::CONTENT_TYPE.as_str().to_string(),
                "application/json".to_string(),
            )],
            body,
        });
    }

    Ok(DispatchResponse {
        status,
        headers,
        body,
    })
}

/// The router's fallback is `{"error":{"code":"NOT_FOUND","message":"Route not found: …"}}`.
/// A handler's own 404 (a missing row) has a different message and must pass through.
fn is_route_not_found(body: &[u8]) -> bool {
    serde_json::from_slice::<serde_json::Value>(body)
        .ok()
        .and_then(|v| {
            v["error"]["message"]
                .as_str()
                .map(|m| m.starts_with("Route not found"))
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_api::{app, AppState};
    use forge_sdk::Forge;

    fn router() -> Router {
        app(AppState::new(Forge::open_memory().unwrap()))
    }

    fn req(method: &str, path: &str, body: &str) -> DispatchRequest {
        DispatchRequest {
            method: method.into(),
            path: path.into(),
            headers: if body.is_empty() {
                vec![]
            } else {
                vec![("content-type".into(), "application/json".into())]
            },
            body: body.as_bytes().to_vec(),
        }
    }

    fn json(resp: &DispatchResponse) -> serde_json::Value {
        serde_json::from_slice(&resp.body).unwrap()
    }

    #[tokio::test]
    async fn serves_health() {
        let resp = dispatch(&router(), req("GET", "/api/health", ""), false)
            .await
            .unwrap();
        assert_eq!(resp.status, 200);
        assert_eq!(json(&resp)["data"]["server"], "ok");
    }

    #[tokio::test]
    async fn round_trips_a_json_body_and_query_string() {
        let r = router();
        let created = dispatch(
            &r,
            req("POST", "/api/sources", r#"{"title":"T","description":"D"}"#),
            false,
        )
        .await
        .unwrap();
        assert_eq!(created.status, 201);

        let listed = dispatch(&r, req("GET", "/api/sources?limit=1&offset=0", ""), false)
            .await
            .unwrap();
        assert_eq!(listed.status, 200);
        assert_eq!(json(&listed)["pagination"]["total"], 1);
        assert_eq!(json(&listed)["pagination"]["limit"], 1);
    }

    #[tokio::test]
    async fn response_headers_are_returned() {
        let resp = dispatch(&router(), req("GET", "/api/health", ""), false)
            .await
            .unwrap();
        assert!(resp
            .headers
            .iter()
            .any(|(k, v)| k == "content-type" && v.starts_with("application/json")));
    }

    #[tokio::test]
    async fn method_is_case_insensitive_and_bad_input_is_an_error() {
        let ok = dispatch(&router(), req("get", "/api/health", ""), false)
            .await
            .unwrap();
        assert_eq!(ok.status, 200);
        assert!(dispatch(&router(), req("GE T", "/api/health", ""), false)
            .await
            .is_err());
        assert!(dispatch(&router(), req("GET", "not a uri", ""), false)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn fetch_owned_headers_are_not_forwarded() {
        let mut r = req("GET", "/api/health", "");
        r.headers = vec![
            ("Host".into(), "evil".into()),
            ("Content-Length".into(), "999".into()),
        ];
        assert_eq!(dispatch(&router(), r, false).await.unwrap().status, 200);
    }

    #[tokio::test]
    async fn unported_routes_are_501_only_when_asked() {
        let r = router();
        let plain = dispatch(&r, req("GET", "/api/definitely-not-a-route", ""), false)
            .await
            .unwrap();
        assert_eq!(plain.status, 404);

        let rewritten = dispatch(&r, req("GET", "/api/definitely-not-a-route", ""), true)
            .await
            .unwrap();
        assert_eq!(rewritten.status, 501);
        assert_eq!(json(&rewritten)["error"]["code"], "NOT_IMPLEMENTED");
        assert!(json(&rewritten)["error"]["message"]
            .as_str()
            .unwrap()
            .contains("/api/definitely-not-a-route"));
    }

    #[tokio::test]
    async fn a_handlers_own_404_is_not_rewritten() {
        let resp = dispatch(&router(), req("GET", "/api/sources/missing", ""), true)
            .await
            .unwrap();
        assert_eq!(
            resp.status, 404,
            "a missing row is a real 404, not 'unported'"
        );
        assert_eq!(json(&resp)["error"]["code"], "NOT_FOUND");
    }

    #[tokio::test]
    async fn tagline_get_is_ported_not_501() {
        let resp = dispatch(
            &router(),
            req("GET", "/api/resumes/missing/tagline", ""),
            true,
        )
        .await
        .unwrap();
        assert_eq!(resp.status, 404, "a missing resume, not an unported route");
        assert_eq!(json(&resp)["error"]["message"], "Resume not found");
    }
}
