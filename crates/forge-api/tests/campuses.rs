//! Org locations (formerly campuses): the `/campuses` aliases and TS-compatible create, HQ flag
//! and list order (resu-mx/forge#30).

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
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn org(r: &Router, name: &str) -> String {
    let (_, o) = call(
        r,
        "POST",
        "/api/organizations",
        Some(json!({ "name": name })),
    )
    .await;
    o["data"]["id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn create_without_organization_id_on_both_paths() {
    let r = router();
    let o = org(&r, "Acme").await;
    for base in ["locations", "campuses"] {
        let (status, body) = call(
            &r,
            "POST",
            &format!("/api/organizations/{o}/{base}"),
            Some(json!({
                "name": format!("HQ {base}"), "modality": "in_person", "is_headquarters": true,
                "city": "Arlington", "state": "VA" // unknown fields are ignored
            })),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{base}: {body}");
        assert_eq!(body["data"]["organization_id"], json!(o));
        assert_eq!(body["data"]["is_headquarters"], json!(true)); // a boolean, not 1
        assert_eq!(body["data"]["address_id"], Value::Null);
    }
}

#[tokio::test]
async fn body_organization_id_is_ignored() {
    let r = router();
    let o = org(&r, "Acme").await;
    let other = org(&r, "Other").await;
    let (status, body) = call(
        &r,
        "POST",
        &format!("/api/organizations/{o}/locations"),
        Some(json!({ "name": "Main", "organization_id": other })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(body["data"]["organization_id"], json!(o));
    assert_eq!(body["data"]["is_headquarters"], json!(false));
}

#[tokio::test]
async fn list_orders_by_name_on_both_paths() {
    let r = router();
    let o = org(&r, "Order Co").await;
    for (name, hq) in [("Zeta HQ", true), ("Alpha", false)] {
        call(
            &r,
            "POST",
            &format!("/api/organizations/{o}/locations"),
            Some(json!({ "name": name, "is_headquarters": hq })),
        )
        .await;
    }
    for base in ["locations", "campuses"] {
        let (_, list) = call(&r, "GET", &format!("/api/organizations/{o}/{base}"), None).await;
        let names: Vec<_> = list["data"]
            .as_array()
            .unwrap()
            .iter()
            .map(|l| l["name"].clone())
            .collect();
        assert_eq!(names, [json!("Alpha"), json!("Zeta HQ")], "{base}");
    }
}

#[tokio::test]
async fn campus_alias_patch_and_delete() {
    let r = router();
    let o = org(&r, "Acme").await;
    let (_, created) = call(
        &r,
        "POST",
        &format!("/api/organizations/{o}/campuses"),
        Some(json!({ "name": "Temp", "is_headquarters": true })),
    )
    .await;
    let id = created["data"]["id"].as_str().unwrap().to_string();

    let (status, body) = call(
        &r,
        "PATCH",
        &format!("/api/campuses/{id}"),
        Some(json!({ "is_headquarters": false })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["data"]["is_headquarters"], json!(false));

    let (status, _) = call(&r, "DELETE", &format!("/api/campuses/{id}"), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, body) = call(&r, "DELETE", &format!("/api/campuses/{id}"), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    let (_, list) = call(&r, "GET", &format!("/api/organizations/{o}/campuses"), None).await;
    assert_eq!(list["data"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn patch_address_id_null_clears_it() {
    let r = router();
    let o = org(&r, "Acme").await;
    let (status, addr) = call(
        &r,
        "POST",
        "/api/addresses",
        Some(json!({ "name": "Main Campus", "city": "Arlington", "state": "VA" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{addr}");
    let addr_id = addr["data"]["id"].as_str().unwrap().to_string();

    let (status, loc) = call(
        &r,
        "POST",
        &format!("/api/organizations/{o}/locations"),
        Some(json!({ "name": "Main", "address_id": addr_id })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{loc}");
    assert_eq!(loc["data"]["address_id"], json!(addr_id));
    let id = loc["data"]["id"].as_str().unwrap().to_string();

    let (status, body) = call(
        &r,
        "PATCH",
        &format!("/api/locations/{id}"),
        Some(json!({ "address_id": null })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["data"]["address_id"], Value::Null);
}

#[tokio::test]
async fn blank_name_is_400() {
    let r = router();
    let o = org(&r, "Acme").await;
    let (status, body) = call(
        &r,
        "POST",
        &format!("/api/organizations/{o}/locations"),
        Some(json!({ "name": "   " })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["error"]["code"], "VALIDATION_ERROR");

    let (_, loc) = call(
        &r,
        "POST",
        &format!("/api/organizations/{o}/locations"),
        Some(json!({ "name": "  Padded  " })),
    )
    .await;
    assert_eq!(loc["data"]["name"], "Padded");
    let id = loc["data"]["id"].as_str().unwrap().to_string();
    let (status, body) = call(
        &r,
        "PATCH",
        &format!("/api/locations/{id}"),
        Some(json!({ "name": " " })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["error"]["code"], "VALIDATION_ERROR");
}

#[tokio::test]
async fn unknown_organization_is_400() {
    let r = router();
    let (status, body) = call(
        &r,
        "POST",
        "/api/organizations/00000000-0000-4000-8000-000000000000/locations",
        Some(json!({ "name": "Ghost" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["error"]["code"], "VALIDATION_ERROR");
}
