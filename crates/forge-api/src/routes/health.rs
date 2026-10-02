//! Health check route.

use axum::Json;
use serde::Serialize;

use crate::response::ApiData;

#[derive(Serialize)]
pub struct HealthInfo {
    server: &'static str,
    version: &'static str,
}

pub async fn health() -> Json<ApiData<HealthInfo>> {
    Json(ApiData {
        data: HealthInfo {
            server: "ok",
            version: env!("CARGO_PKG_VERSION"),
        },
    })
}
