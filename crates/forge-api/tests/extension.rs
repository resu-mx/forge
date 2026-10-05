//! Router tests for the browser-extension endpoints (resu-mx/forge#110 to #114).
//! Same setup as `resources.rs`: in-process router, in-memory database.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use forge_api::{app, AppState};
use forge_sdk::Forge;
use serde_json::{json, Value};
use tower::ServiceExt;

fn router() -> Router {
    app(AppState::new(Forge::open_memory().unwrap()))
}

async fn call(r: &Router, method: &str, path: &str, body: Option<Value>) -> (StatusCode, Value) {
    let mut req = Request::builder().method(method).uri(path);
    let body = match body {
        Some(v) => {
            req = req.header("content-type", "application/json");
            Body::from(v.to_string())
        }
        None => Body::empty(),
    };
    let resp = r.clone().oneshot(req.body(body).unwrap()).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 1 << 20)
        .await
        .unwrap();
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, value)
}

fn defaults() -> Value {
    json!({"baseUrl": "http://localhost:3000", "devMode": false,
           "enabledPlugins": ["linkedin"], "enableServerLogging": true})
}

fn msg(err: &Value) -> &str {
    err["error"]["message"].as_str().unwrap()
}

#[tokio::test]
async fn config_get_and_put_round_trip() {
    let r = router();
    let (status, body) = call(&r, "GET", "/api/extension/config", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"], defaults());

    let (status, body) = call(
        &r,
        "PUT",
        "/api/extension/config",
        Some(json!({"updates": {"devMode": true, "enableServerLogging": false}})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["devMode"], true);
    assert_eq!(body["data"]["enableServerLogging"], false);
    assert_eq!(body["data"]["baseUrl"], "http://localhost:3000");

    let (_, again) = call(&r, "GET", "/api/extension/config", None).await;
    assert_eq!(again["data"], body["data"]);
}

#[tokio::test]
async fn config_put_rejections_write_nothing() {
    let r = router();
    for bad in [
        json!({}),
        json!({"updates": null}),
        json!({"updates": "x"}),
        json!({"updates": 0}),
        json!({"updates": false}),
        json!(null),
        json!([]),
        json!("x"),
    ] {
        let (status, err) = call(&r, "PUT", "/api/extension/config", Some(bad)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(err["error"]["code"], "VALIDATION_ERROR");
        assert!(msg(&err).contains(r#"Body must contain an "updates" object"#));
    }
    let (status, err) = call(
        &r,
        "PUT",
        "/api/extension/config",
        Some(json!({"updates": {"devMode": true, "badKey": "value"}})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(msg(&err).contains("Unknown config key: badKey"));

    let (_, cfg) = call(&r, "GET", "/api/extension/config", None).await;
    assert_eq!(cfg["data"], defaults());

    // An empty array is an empty update; a non-empty one yields index keys.
    let (status, ok) = call(
        &r,
        "PUT",
        "/api/extension/config",
        Some(json!({"updates": []})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(ok["data"], defaults());
    let (status, err) = call(
        &r,
        "PUT",
        "/api/extension/config",
        Some(json!({"updates": ["x"]})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(msg(&err).contains("Unknown config key: 0"));
}

#[tokio::test]
async fn log_post_list_and_clear() {
    let r = router();
    let (status, created) = call(
        &r,
        "POST",
        "/api/extension/log",
        Some(
            json!({"error_code": "API_UNREACHABLE", "message": "Failed to fetch", "layer": "sdk"}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let row = &created["data"];
    assert!(row["id"].is_string() && row["created_at"].is_string());
    assert!(row["plugin"].is_null() && row["url"].is_null() && row["context"].is_null());

    for code in ["B", "C"] {
        call(
            &r,
            "POST",
            "/api/extension/log",
            Some(
                json!({"error_code": code, "message": "m", "layer": "plugin",
                        "context": {"step": code}}),
            ),
        )
        .await;
    }
    let (status, page) = call(&r, "GET", "/api/extension/logs?limit=2", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(page.get("pagination").is_none());
    let codes: Vec<Value> = page["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["error_code"].clone())
        .collect();
    assert_eq!(codes, [json!("C"), json!("B")]);
    assert_eq!(page["data"][0]["context"], json!({"step": "C"}));

    let (_, only) = call(&r, "GET", "/api/extension/logs?layer=sdk", None).await;
    assert_eq!(only["data"].as_array().unwrap().len(), 1);

    let (status, cleared) = call(&r, "DELETE", "/api/extension/logs", None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(cleared.is_null());
    let (_, empty) = call(&r, "GET", "/api/extension/logs", None).await;
    assert_eq!(empty["data"], json!([]));
}

#[tokio::test]
async fn log_post_validation() {
    let r = router();
    let cases = [
        (json!({}), "error_code is required"),
        (json!({"error_code": "E"}), "message is required"),
        (
            json!({"error_code": "E", "message": "m", "layer": "  "}),
            "layer is required",
        ),
    ];
    for (body, expected) in cases {
        let (status, err) = call(&r, "POST", "/api/extension/log", Some(body)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(err["error"]["code"], "VALIDATION_ERROR");
        assert!(msg(&err).contains(expected), "{}", msg(&err));
    }
    let (status, err) = call(
        &r,
        "POST",
        "/api/extension/log",
        Some(json!({"error_code": 1, "message": "m", "layer": "sdk"})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(err["error"]["code"], "VALIDATION_ERROR");
}

#[tokio::test]
async fn log_list_params() {
    let r = router();
    for code in ["A", "B", "C"] {
        call(
            &r,
            "POST",
            "/api/extension/log",
            Some(json!({"error_code": code, "message": "m", "layer": "sdk"})),
        )
        .await;
    }
    let len = |v: &Value| v["data"].as_array().unwrap().len();
    let (_, a) = call(&r, "GET", "/api/extension/logs?error_code=A", None).await;
    assert_eq!(len(&a), 1);
    let (_, o) = call(&r, "GET", "/api/extension/logs?offset=1", None).await;
    assert_eq!(len(&o), 2);
    let (_, z) = call(&r, "GET", "/api/extension/logs?limit=0", None).await;
    assert_eq!(len(&z), 0);
    let (status, d) = call(&r, "GET", "/api/extension/logs?limit=", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(len(&d), 3);
    for q in ["limit=abc", "offset=1.5", "limit=10abc"] {
        let (status, err) = call(&r, "GET", &format!("/api/extension/logs?{q}"), None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{q}");
        assert_eq!(err["error"]["code"], "VALIDATION_ERROR");
    }
}

#[tokio::test]
async fn clear_on_empty_table_and_config_untouched() {
    let r = router();
    let (status, _) = call(&r, "DELETE", "/api/extension/logs", None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    call(
        &r,
        "PUT",
        "/api/extension/config",
        Some(json!({"updates": {"devMode": true}})),
    )
    .await;
    call(&r, "DELETE", "/api/extension/logs", None).await;
    let (_, cfg) = call(&r, "GET", "/api/extension/config", None).await;
    assert_eq!(cfg["data"]["devMode"], true);
}
