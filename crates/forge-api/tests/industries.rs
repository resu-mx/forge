//! Router tests for POST /api/industries name rules (resu-mx/forge#140). In-process,
//! in-memory database, the inline (no-tokio) `with_conn` path.

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

async fn call(
    router: &Router,
    method: &str,
    path: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut req = Request::builder().method(method).uri(path);
    let body = match body {
        Some(v) => {
            req = req.header("content-type", "application/json");
            Body::from(v.to_string())
        }
        None => Body::empty(),
    };
    let resp = router
        .clone()
        .oneshot(req.body(body).unwrap())
        .await
        .unwrap();
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

#[tokio::test]
async fn post_trims_rejects_blank_and_duplicate_names() {
    let r = router();
    let (status, body) = call(
        &r,
        "POST",
        "/api/industries",
        Some(json!({ "name": "  Brand New  " })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["data"]["name"], "Brand New");

    let (status, err) = call(
        &r,
        "POST",
        "/api/industries",
        Some(json!({ "name": "   " })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(err["error"]["code"], "VALIDATION_ERROR");
    assert!(err["error"]["message"]
        .as_str()
        .unwrap()
        .contains("Name must not be empty"));

    let (status, err) = call(
        &r,
        "POST",
        "/api/industries",
        Some(json!({ "name": "Brand New" })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(err["error"]["code"], "CONFLICT");

    let (status, err) = call(&r, "POST", "/api/industries", Some(json!({}))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(err["error"]["code"], "VALIDATION_ERROR");
}

// -- GET /api/industries pagination (resu-mx/forge#141) ---------------

/// Names sort in numeric order; rows are created in reverse so the order is the sort, not the
/// insert order.
async fn seed(r: &Router, count: usize) {
    for i in (0..count).rev() {
        let (status, _) = call(
            r,
            "POST",
            "/api/industries",
            Some(json!({ "name": format!("Industry {i:03}") })),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
    }
}

fn names(body: &Value) -> Vec<String> {
    body["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["name"].as_str().unwrap().to_string())
        .collect()
}

#[tokio::test]
async fn list_empty_has_the_pagination_shape() {
    let r = router();
    let (status, body) = call(&r, "GET", "/api/industries", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"], json!([]));
    assert_eq!(
        body["pagination"],
        json!({ "total": 0, "offset": 0, "limit": 50 })
    );
}

#[tokio::test]
async fn list_defaults_to_a_page_of_50_ordered_by_name() {
    let r = router();
    seed(&r, 60).await;
    let (status, body) = call(&r, "GET", "/api/industries", None).await;
    assert_eq!(status, StatusCode::OK);
    let expected: Vec<String> = (0..50).map(|i| format!("Industry {i:03}")).collect();
    assert_eq!(names(&body), expected);
    assert_eq!(
        body["pagination"],
        json!({ "total": 60, "offset": 0, "limit": 50 })
    );
}

#[tokio::test]
async fn list_honours_offset_and_limit_and_echoes_them() {
    let r = router();
    seed(&r, 10).await;
    let (_, body) = call(&r, "GET", "/api/industries?offset=3&limit=4", None).await;
    assert_eq!(
        names(&body),
        [
            "Industry 003",
            "Industry 004",
            "Industry 005",
            "Industry 006"
        ]
    );
    assert_eq!(
        body["pagination"],
        json!({ "total": 10, "offset": 3, "limit": 4 })
    );

    let (_, body) = call(&r, "GET", "/api/industries?offset=20", None).await;
    assert!(names(&body).is_empty());
    assert_eq!(
        body["pagination"],
        json!({ "total": 10, "offset": 20, "limit": 50 })
    );
}

#[tokio::test]
async fn list_clamps_a_negative_offset_to_zero() {
    let r = router();
    seed(&r, 3).await;
    let (status, body) = call(&r, "GET", "/api/industries?offset=-3", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(names(&body).len(), 3);
    assert_eq!(
        body["pagination"],
        json!({ "total": 3, "offset": 0, "limit": 50 })
    );
}

#[tokio::test]
async fn list_clamps_limit_to_200_and_pages_to_the_total() {
    let r = router();
    seed(&r, 250).await;

    let (status, first) = call(&r, "GET", "/api/industries?limit=500", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(names(&first).len(), 200);
    assert_eq!(
        first["pagination"],
        json!({ "total": 250, "offset": 0, "limit": 200 })
    );

    let (_, second) = call(&r, "GET", "/api/industries?limit=200&offset=200", None).await;
    assert_eq!(names(&second).len(), 50);
    assert_eq!(
        second["pagination"],
        json!({ "total": 250, "offset": 200, "limit": 200 })
    );

    let mut all = names(&first);
    all.extend(names(&second));
    let expected: Vec<String> = (0..250).map(|i| format!("Industry {i:03}")).collect();
    assert_eq!(all, expected);
}
